//! Part D4, `open_for_resume` (SDD 3.7.3, 3.9.2, 3.11.3): the row with no journal (CT-ST-080,
//! CT-ST-089 no-journal path), an intact journal, a torn tail, the rebase (damaged middle, store
//! ahead D-66, `UndecodableTail` CH-5 B, unreadable header RR-A-1 (a)), a final or voided game
//! that is moved aside and not rebased (UT-ST-091), the whole journal in one call (D3-10, D-87),
//! `GameParked` and the documented unpark repair (D3RL-01), B-R05 on the resume path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d4_support;

use std::fs;

use chess_domain::{GameStatus, Matchup, Mode, WallMs};
use chess_store::journal::format::{
    EndReason, GameEndPayload, HEADER_LEN, JournalRecord, Payload, crc32,
};
use chess_store::journal::{DamageKind, RunSource, fold, scan_bytes, scan_file};
use chess_store::{DataDir, JournalError, Store, StoreError};
use common::{block_on, gid, jb, new_game, pid, raw};
use d4_support::{
    apply, corrupt_record, damaged_files, dump, game_end, jdir, jpath, live, playing, rebase_path,
    rec, voided, world, write_journal,
};
use uuid::Uuid;

const STORE_NOTE: &str =
    "the store id of a store opened by `open_in` is read from the journal header";

fn store_uuid(dir: &std::path::Path) -> u128 {
    DataDir::new(dir)
        .read_store_id()
        .expect(STORE_NOTE)
        .unwrap()
        .store_uuid
        .as_u128()
}

fn resume(
    s: &Store,
    n: u128,
) -> Result<
    (
        Option<chess_store::journal::JournalWriter>,
        chess_store::journal::Recovered,
    ),
    chess_store::journal::ResumeError,
> {
    let journal = s.journal().unwrap();
    block_on(journal.open_for_resume(s, &gid(n)))
}

fn rows(s: &Store, n: u128) -> Vec<(u32, String)> {
    block_on(s.game_moves(gid(n)))
        .unwrap()
        .into_iter()
        .map(|m| (m.ply, m.uci))
        .collect()
}

// ------------------------------------------------------------------------- the row with no journal

/// SDD 3.9.2: a row with no journal resumes as a game with zero moves; `open_for_resume`
/// creates a header-only file (the app then appends `Start` as record 1).
#[test]
fn a_row_with_no_journal_resumes_with_zero_moves_and_a_header_only_file() {
    let (d, s) = live();
    assert!(!jpath(d.path(), 1).exists());
    let (writer, recovered) = resume(&s, 1).unwrap();
    let writer = writer.expect("a writer for an in-progress game");
    assert!(recovered.records.is_empty() && recovered.moves.is_empty());
    assert!(recovered.report.created_empty);
    assert_eq!(recovered.report.damaged, None);
    assert_eq!(
        fs::metadata(jpath(d.path(), 1)).unwrap().len(),
        HEADER_LEN as u64
    );
    // The app appends `Start` with `wall_ms = GAME.started_at` read from the row.
    let started = block_on(s.get_game(gid(1))).unwrap().started_at.unwrap();
    assert_eq!(started, WallMs(5_001));
    assert_eq!(block_on(writer.append(started.0, jb::start(1))).unwrap(), 1);
    // A second resume of the same game while the writer is held is refused.
    assert!(matches!(
        resume(&s, 1),
        Err(chess_store::journal::ResumeError::Journal(
            JournalError::Locked { .. }
        ))
    ));
    drop(writer);
    // And once released, the file is resumed as an ordinary journal (Start only).
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert_eq!(recovered.records.len(), 1);
    assert!(!recovered.report.created_empty);
    assert_eq!(writer.unwrap().next_seq(), 2);
}

/// A resume of a game with no row is `NotFound`.
#[test]
fn a_game_with_no_row_cannot_be_resumed() {
    let (_d, s) = world();
    assert!(matches!(
        resume(&s, 77),
        Err(chess_store::journal::ResumeError::Store(
            StoreError::NotFound { .. }
        ))
    ));
}

/// CT-ST-080 (the store half): a slot conflict at `start_game` leaves no journal; the kill
/// between `start_game` and `MoveJournal::create` is the case above (a row with no journal).
#[test]
fn ct_st_080_a_slot_conflict_leaves_no_journal() {
    let (d, s) = live();
    let err = block_on(s.start_game(new_game(
        2,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap_err();
    assert!(matches!(err, StoreError::SlotTaken { .. }), "{err:?}");
    assert!(!jpath(d.path(), 2).exists());
    assert!(
        fs::read_dir(jdir(d.path())).unwrap().flatten().all(|e| e
            .file_name()
            .to_string_lossy()
            .contains("done")
            || e.file_name().to_string_lossy().contains("damaged")
            || e.file_name().to_string_lossy().contains("unrestored")),
        "the journal directory holds no journal"
    );
}

/// A file cut inside its header (a create that was killed) for a row that holds nothing: the
/// header is written again, in place.
#[test]
fn a_cut_header_for_an_empty_row_is_written_again() {
    let (d, s) = live();
    fs::write(jpath(d.path(), 1), [0x43, 0x48, 0x53]).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.created_empty && recovered.moves.is_empty());
    assert_eq!(
        fs::metadata(jpath(d.path(), 1)).unwrap().len(),
        HEADER_LEN as u64
    );
    assert_eq!(writer.unwrap().next_seq(), 1);
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        Uuid::from_u128(store_uuid(d.path())),
    )
    .unwrap();
    assert!(scan.records.is_empty() && scan.damaged.is_none());
}

// ------------------------------------------------------------------------------- an intact journal

/// An intact journal that the database has not seen: every record is applied (the whole journal
/// in one call), the rows and clocks follow the journal, `journal_seq` is the last `seq`, and the
/// writer continues with the next `seq`.
#[test]
fn an_intact_journal_is_applied_and_the_writer_continues() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(jb::clock(299_000, 301_000));
    write_journal(&s, 1, payloads);
    assert!(rows(&s, 1).is_empty());
    let (writer, recovered) = resume(&s, 1).unwrap();
    let writer = writer.unwrap();
    assert_eq!(recovered.records.len(), 4);
    assert_eq!(recovered.moves.len(), 2);
    assert_eq!(recovered.clocks, Some((299_000, 301_000)));
    assert_eq!(recovered.report.damaged, None);
    assert!(!recovered.report.rebased);
    assert_eq!(rows(&s, 1), vec![(1, "u0".into()), (2, "u1".into())]);
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_eq!(g.journal_seq, 4);
    assert_eq!(
        (g.white_clock_ms, g.black_clock_ms),
        (Some(299_000), Some(301_000))
    );
    assert_eq!(writer.next_seq(), 5);
    assert_eq!(block_on(writer.append(2_000, jb::mv(2))).unwrap(), 5);
    drop(writer);
    // Nothing is damaged and nothing was moved aside.
    assert!(damaged_files(d.path(), 1, ".jnl").is_empty());
}

/// A resume of a game that the database already holds changes nothing (idempotent).
#[test]
fn a_second_resume_changes_nothing() {
    let (d, s) = live();
    write_journal(&s, 1, playing(1, 3));
    drop(resume(&s, 1).unwrap());
    let before = (dump(d.path()), fs::read(jpath(d.path(), 1)).unwrap());
    let (_writer, recovered) = resume(&s, 1).unwrap();
    assert_eq!(recovered.moves.len(), 3);
    drop(_writer);
    assert_eq!(
        before,
        (dump(d.path()), fs::read(jpath(d.path(), 1)).unwrap())
    );
}

/// The journal wins where the database differs at a ply (the cache wins, Q54): the stored row
/// from that ply on is replaced, and a stored row beyond the journal's end goes.
#[test]
fn the_journal_wins_where_the_rows_differ() {
    let (_d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    // The database holds another second move and a fourth one that the journal never had.
    let Payload::Move(mut other) = jb::mv(1) else {
        panic!()
    };
    other.uci = "zz".into();
    apply(
        &s,
        1,
        vec![
            records[1].clone(),
            rec(3, Payload::Move(other)),
            rec(4, jb::mv(2)),
            rec(5, jb::mv(3)),
        ],
    );
    assert_eq!(rows(&s, 1).len(), 4);
    // journal_seq is now 5, beyond the journal's last seq 4: that is the D-66 case (damage), so
    // the journal does not win here; the next test is that case. Make the journal the later one.
    raw(_d.path())
        .execute("UPDATE game SET journal_seq = 2", [])
        .unwrap();
    let (_w, recovered) = resume(&s, 1).unwrap();
    assert_eq!(
        rows(&s, 1),
        vec![(1, "u0".into()), (2, "u1".into()), (3, "u2".into())],
        "ply 2 is the journal's again and ply 4 is gone"
    );
    assert_eq!(recovered.moves.len(), 3);
    assert_eq!(recovered.report.source, RunSource::Journal);
}

/// A torn tail is cut before anything is appended after it; it is never reported as damage.
#[test]
fn a_torn_tail_is_cut_before_the_next_append() {
    let (d, s) = live();
    write_journal(&s, 1, playing(1, 2));
    let intact = fs::read(jpath(d.path(), 1)).unwrap();
    let mut torn = intact.clone();
    torn.extend_from_slice(&[0xA5, 0x5A, 0x40, 0x00, 0x00]);
    fs::write(jpath(d.path(), 1), &torn).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    let writer = writer.unwrap();
    assert!(recovered.report.torn_tail_dropped);
    assert_eq!(recovered.report.damaged, None);
    assert_eq!(
        fs::read(jpath(d.path(), 1)).unwrap(),
        intact,
        "cut to the intact end"
    );
    assert_eq!(block_on(writer.append(9, jb::mv(2))).unwrap(), 4);
    drop(writer);
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        Uuid::from_u128(store_uuid(d.path())),
    )
    .unwrap();
    assert!(scan.damaged.is_none() && !scan.torn_tail_dropped);
    assert_eq!(scan.records.len(), 4);
}

// -------------------------------------------------------------------------------------- the rebase

fn assert_rebased_clean(d: &std::path::Path, n: u128, last_seq_of_db: u64) {
    let scan = scan_file(
        &jpath(d, n),
        Uuid::from_u128(n),
        Uuid::from_u128(store_uuid(d)),
    )
    .unwrap();
    assert!(
        scan.damaged.is_none() && !scan.torn_tail_dropped,
        "the new journal is intact"
    );
    assert_eq!(
        scan.last_seq(),
        last_seq_of_db,
        "journal_seq equals the last seq"
    );
    assert!(!rebase_path(d, n).exists(), "no .rebase remains");
}

/// A damaged middle while the store holds more: the game continues on a rebased journal built
/// from the intact run and the stored rows; the old file is in `damaged/`, byte for byte.
#[test]
fn a_damaged_middle_with_a_longer_store_is_rebased() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 4));
    apply(&s, 1, records);
    let original = corrupt_record(&jpath(d.path(), 1), 3, store_uuid(d.path()), 1);
    let (writer, recovered) = resume(&s, 1).unwrap();
    let writer = writer.unwrap();
    assert!(recovered.report.rebased);
    assert!(matches!(
        recovered.report.damaged.map(|x| x.kind),
        Some(DamageKind::Middle { .. })
    ));
    assert_eq!(recovered.report.source, RunSource::Store);
    assert_eq!(
        recovered.moves.len(),
        4,
        "the longer run (the store) resumes"
    );
    let aside = damaged_files(d.path(), 1, ".jnl");
    assert_eq!(aside.len(), 1);
    assert_eq!(fs::read(&aside[0]).unwrap(), original);
    assert_eq!(
        recovered.report.moved_aside.as_deref(),
        Some(aside[0].as_path())
    );
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_rebased_clean(d.path(), 1, g.journal_seq);
    assert_eq!(writer.next_seq(), g.journal_seq + 1);
    assert_eq!(rows(&s, 1).len(), 4);
    // The new file replays to the same moves.
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        Uuid::from_u128(store_uuid(d.path())),
    )
    .unwrap();
    assert_eq!(fold(&scan.records).moves.len(), 4);
    drop(writer);
    // A second resume changes nothing and makes no second damaged file (D-141, idempotence).
    let bytes = fs::read(jpath(d.path(), 1)).unwrap();
    let (_w, again) = resume(&s, 1).unwrap();
    assert!(!again.report.rebased && again.report.damaged.is_none());
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), bytes);
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
}

/// D-66: the store applied a record that the journal no longer proves (the journal ends clean
/// but short). It is damage; no stored row is deleted; the rebase builds from the store.
#[test]
fn a_store_ahead_of_the_journal_is_damage_and_keeps_every_stored_row() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 4));
    apply(&s, 1, records);
    // The journal loses its last two records at a record boundary (a clean, short file).
    let path = jpath(d.path(), 1);
    let bytes = fs::read(&path).unwrap();
    let cut = d4_support::record_offset(&bytes, 3, store_uuid(d.path()), 1);
    fs::write(&path, &bytes[..cut]).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased);
    assert_eq!(
        recovered.report.damaged.map(|x| x.kind),
        Some(DamageKind::StoreAhead)
    );
    assert_eq!(
        recovered.moves.len(),
        4,
        "no stored row beyond the journal's end is deleted"
    );
    assert_eq!(rows(&s, 1).len(), 4);
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_rebased_clean(d.path(), 1, g.journal_seq);
    assert_eq!(writer.unwrap().next_seq(), g.journal_seq + 1);
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
}

/// A torn tail while the store is ahead is damage too (D-66 holds with or without a torn tail).
#[test]
fn a_torn_tail_with_the_store_ahead_is_damage() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    apply(&s, 1, records);
    let path = jpath(d.path(), 1);
    let mut bytes = fs::read(&path).unwrap();
    let cut = d4_support::record_offset(&bytes, 3, store_uuid(d.path()), 1);
    bytes.truncate(cut + 7);
    fs::write(&path, &bytes).unwrap();
    let (_w, recovered) = resume(&s, 1).unwrap();
    assert_eq!(
        recovered.report.damaged.map(|x| x.kind),
        Some(DamageKind::StoreAhead)
    );
    assert_eq!(recovered.moves.len(), 3);
}

/// CH-5 B and D-144: a CRC-valid frame that the decoder refuses (here a `GameEnd` whose
/// `ended_at` is 0) at the tail is damage (`UndecodableTail`), takes the same path as a damaged
/// middle, and its bytes stay in `damaged/`.
fn crafted_game_end_frame(seq: u64, ended_at: i64) -> Vec<u8> {
    let payload = serde_json::to_vec(&GameEndPayload {
        result: "1-0".into(),
        reason: EndReason::Checkmate,
        ended_at,
        eco: None,
        opening_name: None,
        analysis_used: false,
        content_hash: "sha256:v1:aa".into(),
        white_time_used_ms: 1,
        black_time_used_ms: 2,
        total_active_ms: 3,
    })
    .unwrap();
    let mut body = Vec::new();
    body.extend_from_slice(&seq.to_le_bytes());
    body.push(12); // GameEnd
    body.extend_from_slice(&50i64.to_le_bytes());
    body.extend_from_slice(&payload);
    let len = u32::try_from(body.len()).unwrap();
    let mut covered = len.to_le_bytes().to_vec();
    covered.extend_from_slice(&body);
    let mut frame = vec![0xA5, 0x5A];
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&crc32(&covered).to_le_bytes());
    frame.extend_from_slice(&body);
    frame
}

#[test]
fn an_undecodable_tail_takes_the_path_of_a_damaged_middle() {
    let (d, s) = live();
    write_journal(&s, 1, playing(1, 3));
    let path = jpath(d.path(), 1);
    let mut original = fs::read(&path).unwrap();
    original.extend(crafted_game_end_frame(5, 0));
    fs::write(&path, &original).unwrap();
    // The scan itself reports it (CH-5 B, D-144).
    let scan = scan_bytes(
        &original,
        Uuid::from_u128(1),
        Uuid::from_u128(store_uuid(d.path())),
    )
    .unwrap();
    assert!(matches!(
        scan.damaged.map(|x| x.kind),
        Some(DamageKind::UndecodableTail { .. })
    ));
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased);
    assert!(matches!(
        recovered.report.damaged.map(|x| x.kind),
        Some(DamageKind::UndecodableTail { .. })
    ));
    let aside = damaged_files(d.path(), 1, ".jnl");
    assert_eq!(aside.len(), 1);
    assert_eq!(
        fs::read(&aside[0]).unwrap(),
        original,
        "the refused bytes are kept"
    );
    assert_eq!(recovered.moves.len(), 3);
    // The store never applied the refused record: the game is still in progress.
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::InProgress
    );
    assert!(writer.is_some());
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_rebased_clean(d.path(), 1, g.journal_seq);
}

/// UT-ST-090 through the resume: the header is the damaged part, so the intact run is empty;
/// the store holds the moves. The rebased file starts with a `Start` synthesized from the row
/// (`wall_ms = GAME.started_at`), then one `Move` per stored ply, then the closing `Clock`.
#[test]
fn an_unreadable_header_is_rebuilt_from_the_row_and_the_stored_moves() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    apply(&s, 1, records);
    let path = jpath(d.path(), 1);
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] ^= 0xFF; // the magic
    fs::write(&path, &bytes).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased && writer.is_some());
    assert_eq!(recovered.moves.len(), 3);
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        Uuid::from_u128(store_uuid(d.path())),
    )
    .unwrap();
    let kinds: Vec<_> = scan.records.iter().map(JournalRecord::kind).collect();
    use chess_store::journal::format::Kind;
    assert_eq!(
        kinds,
        vec![Kind::Start, Kind::Move, Kind::Move, Kind::Move, Kind::Clock]
    );
    assert_eq!(scan.records[0].wall_ms, 5_001, "wall_ms = GAME.started_at");
    let Payload::Start(start) = &scan.records[0].payload else {
        panic!()
    };
    assert_eq!(
        (
            start.white.0,
            start.black.0,
            start.rated,
            start.self_play,
            start.custom_start
        ),
        (Uuid::from_u128(1), Uuid::from_u128(2), true, false, false)
    );
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
}

/// Neither the journal nor the store holds a readable game: no intact run of even the start
/// record and no `MOVE` rows. The caller offers `VoidUnreadableGame`; the file is untouched.
#[test]
fn nothing_readable_in_either_is_unreadable_and_touches_nothing() {
    let (d, s) = live();
    write_journal(&s, 1, playing(1, 2));
    let path = jpath(d.path(), 1);
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] ^= 0xFF;
    fs::write(&path, &bytes).unwrap();
    let before = dump(d.path());
    let err = resume(&s, 1).unwrap_err();
    assert!(
        matches!(
            err,
            chess_store::journal::ResumeError::Journal(JournalError::Unreadable)
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(dump(d.path()), before);
    assert!(damaged_files(d.path(), 1, "").is_empty());
}

// ----------------------------------------------------------------- a final or voided game: UT-ST-091

/// UT-ST-091: a game whose row is `completed`, `aborted` or `voided` and whose journal is
/// damaged is not rebased: no `.rebase` is built, the damaged file is moved to
/// `damaged/<uuid>.<k>.jnl`, no `<uuid>.jnl` exists afterwards, the `GAME` and `MOVE` rows are
/// unchanged, and the report says the journal was damaged.
#[test]
fn ut_st_091_a_final_or_voided_game_is_moved_aside_and_not_rebased() {
    type Make = fn() -> Payload;
    let cases: [(&str, Make, GameStatus); 3] = [
        ("completed", || game_end(ENDED_AT), GameStatus::Completed),
        (
            "aborted",
            || {
                Payload::Abort(chess_store::journal::format::AbortPayload {
                    ended_at: ENDED_AT,
                    content_hash: "sha256:v1:ab".into(),
                })
            },
            GameStatus::Aborted,
        ),
        ("voided", || voided(Some(1)), GameStatus::Voided),
    ];
    for (name, end, status) in cases {
        let (d, s) = live();
        let mut payloads = playing(1, 3);
        payloads.push(end());
        let records = write_journal(&s, 1, payloads);
        apply(&s, 1, records);
        assert_eq!(
            block_on(s.get_game(gid(1))).unwrap().status,
            status,
            "{name}"
        );
        let original = corrupt_record(&jpath(d.path(), 1), 2, store_uuid(d.path()), 1);
        let before = dump(d.path());
        let (writer, recovered) = resume(&s, 1).unwrap();
        assert!(writer.is_none(), "{name}: nothing will be appended");
        assert!(
            recovered.report.damaged.is_some(),
            "{name}: JournalUnreadable"
        );
        assert!(!recovered.report.rebased, "{name}");
        assert!(!jpath(d.path(), 1).exists(), "{name}: no <uuid>.jnl");
        assert!(!rebase_path(d.path(), 1).exists(), "{name}: no .rebase");
        let aside = damaged_files(d.path(), 1, ".jnl");
        assert_eq!(aside.len(), 1, "{name}");
        assert_eq!(fs::read(&aside[0]).unwrap(), original, "{name}");
        assert_eq!(
            dump(d.path()),
            before,
            "{name}: GAME and MOVE rows unchanged"
        );
    }
}

const ENDED_AT: i64 = 9_000_000;

/// An intact journal of a final game: the writer is returned so that the caller can close it;
/// nothing is applied and nothing moves.
#[test]
fn an_intact_journal_of_a_final_game_is_returned_for_closing() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED_AT));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    let before = dump(d.path());
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert_eq!(recovered.report.damaged, None);
    assert_eq!(recovered.records.len(), 4);
    writer.unwrap().close_final().unwrap();
    assert_eq!(dump(d.path()), before);
    assert!(d4_support::done_path(d.path(), 1).exists());
}

// ------------------------------------------------------------- the whole journal in one call (D-87)

/// D3-10 / D-87: a journal that holds `GameEnd` and then `Voided` is a voided journal. A resume
/// passes the whole journal in one call, so the `GameEnd` is never finalized and no rating
/// changes; the game is voided.
#[test]
fn a_game_end_followed_by_voided_is_voided_and_settles_no_rating() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED_AT));
    payloads.push(voided(Some(1)));
    write_journal(&s, 1, payloads);
    let ratings_before: i64 = raw(d.path())
        .query_row("SELECT sum(strict_elo) FROM player", [], |r| r.get(0))
        .unwrap();
    let (_w, recovered) = resume(&s, 1).unwrap();
    assert_eq!(recovered.records.len(), 5);
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_eq!(g.status, GameStatus::Voided);
    let ratings_after: i64 = raw(d.path())
        .query_row("SELECT sum(strict_elo) FROM player", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ratings_before, ratings_after);
    let history: i64 = raw(d.path())
        .query_row(
            "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(history, 0);
}

// -------------------------------------------------------------- GameParked and the unpark repair

/// A game parked by a refused batch: a resume answers `GameParked` (it does not loop and does
/// not touch the files). `unpark_game` drops the held queue; the next resume passes the whole
/// journal and the rows equal the journal (D3RL-01, D-145).
#[test]
fn a_parked_game_is_refused_until_unparked_and_then_the_whole_journal_repairs_it() {
    let (d, s) = live();
    write_journal(&s, 1, playing(1, 3));
    // A batch the store refuses (a half-null clock pair built in memory) parks the game.
    let Payload::Move(mut half) = jb::mv(0) else {
        panic!()
    };
    half.black_ms = None;
    assert!(block_on(s.apply_records(gid(1), vec![rec(2, Payload::Move(half))])).is_err());
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    let err = resume(&s, 1).unwrap_err();
    assert!(
        matches!(
            err,
            chess_store::journal::ResumeError::Store(StoreError::GameParked { .. })
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
    assert_eq!(block_on(s.parked_games()).unwrap().len(), 1);
    // The documented repair.
    assert_eq!(block_on(s.unpark_game(gid(1))).unwrap(), Some(1));
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert_eq!(recovered.moves.len(), 3);
    assert_eq!(rows(&s, 1).len(), 3);
    assert_eq!(block_on(s.get_game(gid(1))).unwrap().journal_seq, 4);
    assert!(block_on(s.parked_games()).unwrap().is_empty());
    assert_eq!(writer.unwrap().next_seq(), 5);
}

/// A journal whose records the store refuses (an empty `content_hash` in a `GameEnd`) parks the
/// game on the resume; the error is the store's, the journal is untouched and still the truth.
#[test]
fn a_journal_that_the_store_refuses_parks_the_game_on_resume() {
    let (d, s) = live();
    let mut payloads = playing(1, 1);
    payloads.push(Payload::GameEnd(GameEndPayload {
        result: "1-0".into(),
        reason: EndReason::Checkmate,
        ended_at: ENDED_AT,
        eco: None,
        opening_name: None,
        analysis_used: false,
        content_hash: String::new(),
        white_time_used_ms: 1,
        black_time_used_ms: 2,
        total_active_ms: 3,
    }));
    write_journal(&s, 1, payloads);
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    let err = resume(&s, 1).unwrap_err();
    assert!(
        matches!(
            err,
            chess_store::journal::ResumeError::Store(StoreError::InvariantViolated)
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
    let parked = block_on(s.parked_games()).unwrap();
    assert_eq!(parked.len(), 1);
    assert_eq!(block_on(s.backlog_status()).unwrap().pending_batches, 1);
    // Parked: a second resume answers GameParked at once, not a second attempt of the batch.
    assert!(matches!(
        resume(&s, 1).unwrap_err(),
        chess_store::journal::ResumeError::Store(StoreError::GameParked { .. })
    ));
}

// ------------------------------------------------------------------------ the .rebase on resume

fn complete_rebase_bytes(s: &Store, d: &std::path::Path, n: u128) -> Vec<u8> {
    // The bytes the rebase builder writes for a game with `Start`, one move and the closing
    // `Clock`: made by a real journal, then copied.
    let w = s.journal().unwrap().create(&gid(n + 1000)).unwrap();
    block_on(w.append(5_001, jb::start(n))).unwrap();
    block_on(w.append(5_002, jb::mv(0))).unwrap();
    block_on(w.append(5_002, jb::clock(299_999, 300_001))).unwrap();
    drop(w);
    // The header names game `n + 1000`; rebuild it with the right header.
    let raw = fs::read(jpath(d, n + 1000)).unwrap();
    fs::remove_file(jpath(d, n + 1000)).unwrap();
    let mut out = chess_store::journal::format::encode_header(
        Uuid::from_u128(n),
        Uuid::from_u128(store_uuid(d)),
    )
    .to_vec();
    out.extend_from_slice(&raw[HEADER_LEN..]);
    out
}

/// CT-ST-089 (no-journal path): a complete `<uuid>.jnl.rebase` and no `<uuid>.jnl`: the resume
/// renames it into place and does not create a new `Start` journal.
#[test]
fn ct_st_089_a_complete_rebase_is_renamed_into_place_by_the_no_journal_path() {
    let (d, s) = live();
    let bytes = complete_rebase_bytes(&s, d.path(), 1);
    fs::write(rebase_path(d.path(), 1), &bytes).unwrap();
    assert!(!jpath(d.path(), 1).exists());
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert_eq!(
        fs::read(jpath(d.path(), 1)).unwrap(),
        bytes,
        "renamed, not created"
    );
    assert!(!rebase_path(d.path(), 1).exists());
    assert!(!recovered.report.created_empty);
    assert_eq!(recovered.records.len(), 3);
    assert_eq!(recovered.moves.len(), 1);
    assert_eq!(writer.unwrap().next_seq(), 4);
    assert!(damaged_files(d.path(), 1, "").is_empty());
}

/// B-R05 on the resume path: an incomplete `.rebase` with no journal is not installed. It goes
/// to `damaged/<uuid>.<k>.rebase` with its bytes, and the game is rebuilt from the store.
#[test]
fn b_r05_an_incomplete_rebase_is_moved_aside_and_the_game_is_rebuilt_from_the_store() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    apply(&s, 1, records);
    // The journal is gone (the swap's first move happened) and the `.rebase` is cut short.
    let full = complete_rebase_bytes(&s, d.path(), 1);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let cut = &full[..full.len() - 5];
    fs::write(rebase_path(d.path(), 1), cut).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    let aside = damaged_files(d.path(), 1, ".rebase");
    assert_eq!(aside.len(), 1);
    assert_eq!(
        fs::read(&aside[0]).unwrap(),
        cut,
        "the half rebase is kept as it was"
    );
    assert!(!rebase_path(d.path(), 1).exists());
    assert!(recovered.report.rebased, "rebuilt from the store");
    assert_eq!(recovered.moves.len(), 3);
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_rebased_clean(d.path(), 1, g.journal_seq);
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        Uuid::from_u128(store_uuid(d.path())),
    )
    .unwrap();
    assert_eq!(
        scan.records
            .iter()
            .filter(|r| matches!(r.payload, Payload::Move(_)))
            .count(),
        3
    );
    assert!(writer.is_some());
}

/// A game opened for resume is the same game for a second `Store` of the same process only when
/// the lock is free: a live writer makes the resume `Locked` and touches nothing.
#[test]
fn a_held_journal_is_locked_and_untouched() {
    let (d, s) = live();
    let w = s.journal().unwrap().create(&gid(1)).unwrap();
    block_on(w.append(5_001, jb::start(1))).unwrap();
    let before = (dump(d.path()), fs::read(jpath(d.path(), 1)).unwrap());
    assert!(matches!(
        resume(&s, 1),
        Err(chess_store::journal::ResumeError::Journal(
            JournalError::Locked { .. }
        ))
    ));
    assert_eq!(
        before,
        (dump(d.path()), fs::read(jpath(d.path(), 1)).unwrap())
    );
    drop(w);
}
