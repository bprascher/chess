//! Part D4 verification: tests added after the hand-mutant round (docs/engineering/
//! step2-store-D4-evidence.md, section 3). Each test names the mutant it was written against.
//! They pin cases the first test set left open: which stored state counts as "nothing stored",
//! a cut or header-only journal against a row with moves, a final row against a journal that is
//! shorter than the store, a rebased file that ends in a terminal record, and the completeness
//! check of a `.rebase` (B-R05).
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
use std::path::Path;

use chess_domain::GameStatus;
use chess_store::journal::format::{
    ClockPayload, HEADER_LEN, Payload, Side, encode_header, encode_record,
};
use chess_store::journal::{DamageKind, RecoveryOutcome, Settled, settle_rebase_files};
use chess_store::{DataDir, Store};
use common::{block_on, gid, jb, raw};
use d4_support::{
    ENDED, apply, corrupt_record, damaged_files, game_end, jpath, live, playing, rebase_path, rec,
    record_offset, reopen, voided, write_journal,
};
use uuid::Uuid;

fn store_uuid(dir: &Path) -> Uuid {
    DataDir::new(dir)
        .read_store_id()
        .unwrap()
        .unwrap()
        .store_uuid
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
    block_on(s.journal().unwrap().open_for_resume(s, &gid(n)))
}

fn set_journal_seq(dir: &Path, n: u128, seq: i64) {
    let changed = raw(dir)
        .execute(
            "UPDATE game SET journal_seq = ?1 WHERE uuid = ?2",
            rusqlite::params![seq, Uuid::from_u128(n).hyphenated().to_string()],
        )
        .unwrap();
    assert_eq!(changed, 1);
}

// -------------------------------------------------------- "nothing stored" needs both halves

/// Mutants 3 and 4 (`nothing_stored`): a row with `journal_seq` 1 (the `Start` was applied) and
/// no `MOVE` rows, and no journal file, is not "nothing stored": the game is rebuilt from the
/// store (D-66), not given an empty header.
#[test]
fn a_row_with_a_journal_seq_and_no_moves_and_no_journal_is_rebuilt_not_created_empty() {
    let (d, s) = live();
    let records = write_journal(&s, 1, vec![jb::start(1)]);
    apply(&s, 1, records);
    assert_eq!(block_on(s.get_game(gid(1))).unwrap().journal_seq, 1);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(!recovered.report.created_empty, "{:?}", recovered.report);
    assert!(recovered.report.rebased, "{:?}", recovered.report);
    assert!(writer.is_some());
}

/// Mutant 3: `MOVE` rows and `journal_seq` 0 (no `Start` counted), no journal: also rebuilt.
#[test]
fn a_row_with_moves_and_a_zero_journal_seq_and_no_journal_is_rebuilt() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 2));
    apply(&s, 1, records);
    set_journal_seq(d.path(), 1, 0);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(!recovered.report.created_empty, "{:?}", recovered.report);
    assert!(recovered.report.rebased, "{:?}", recovered.report);
    // (Not asserted: with `journal_seq` 0 the journal wins and the two rows are dropped; the
    // state cannot arise through the API, where the rows and `journal_seq` commit together.)
    assert!(writer.is_some());
}

// ------------------------------------------------------- a cut or header-only journal vs moves

/// Mutant 18 (`short && !final_row` without `nothing_stored`): a journal cut inside its header,
/// for a row that holds moves, is damage; the game is rebuilt from the stored rows and keeps
/// them. It is never a "killed create" that gets a fresh empty header.
#[test]
fn a_journal_cut_inside_its_header_with_stored_moves_is_rebased_and_keeps_the_moves() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    apply(&s, 1, records);
    let bytes = fs::read(jpath(d.path(), 1)).unwrap();
    fs::write(jpath(d.path(), 1), &bytes[..5]).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased, "{:?}", recovered.report);
    assert!(!recovered.report.created_empty);
    assert!(recovered.report.damaged.is_some());
    assert_eq!(recovered.moves.len(), 3);
    assert!(writer.is_some());
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
}

/// Mutant 16 (`short` for `len <= HEADER_LEN`): a journal that is exactly a header, for a row
/// with stored moves, is the **store ahead of the journal** (D-66), not a cut header.
#[test]
fn a_header_only_journal_with_stored_moves_is_the_store_ahead() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    apply(&s, 1, records);
    let bytes = fs::read(jpath(d.path(), 1)).unwrap();
    fs::write(jpath(d.path(), 1), &bytes[..HEADER_LEN]).unwrap();
    let (_writer, recovered) = resume(&s, 1).unwrap();
    let damaged = recovered.report.damaged.expect("damage is reported");
    assert!(
        matches!(damaged.kind, DamageKind::StoreAhead),
        "{damaged:?}"
    );
    assert!(recovered.report.rebased);
    assert_eq!(recovered.moves.len(), 3);
}

// --------------------------------------------------------------- a final row against its journal

/// Mutant 34 (`StoreAhead` never set from the view): a final game whose store holds more than
/// the journal proves (the journal was cut after the `GameEnd` was applied) is damage: the file
/// is moved aside and the rows are unchanged, as for any damaged journal of a final game.
#[test]
fn a_final_game_whose_store_is_ahead_of_its_journal_is_moved_aside() {
    let (d, s) = live();
    let mut payloads = playing(1, 3);
    payloads.push(game_end(ENDED));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    let path = jpath(d.path(), 1);
    let bytes = fs::read(&path).unwrap();
    let cut = record_offset(&bytes, 4, store_uuid(d.path()).as_u128(), 1);
    fs::write(&path, &bytes[..cut]).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(writer.is_none());
    let damaged = recovered.report.damaged.expect("damage is reported");
    assert!(
        matches!(damaged.kind, DamageKind::StoreAhead),
        "{damaged:?}"
    );
    assert!(recovered.report.moved_aside.is_some());
    assert!(!path.exists());
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
}

/// Mutant 1 (`<` for `>`): a final game whose journal holds MORE than the store (a record after
/// the applied `GameEnd`) is not damage: the writer comes back so that the caller can close it.
#[test]
fn a_final_game_whose_journal_is_ahead_of_the_store_is_not_damage() {
    let (d, s) = live();
    let mut payloads = playing(1, 3);
    payloads.push(game_end(ENDED));
    payloads.push(voided(None));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records[..5].to_vec());
    assert_eq!(block_on(s.get_game(gid(1))).unwrap().journal_seq, 5);
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.damaged.is_none(), "{:?}", recovered.report);
    assert!(writer.is_some());
    assert!(jpath(d.path(), 1).exists());
    assert!(damaged_files(d.path(), 1, "").is_empty());
}

// ------------------------------------------------------- a rebased file that ends in a terminal

/// Mutant 40 (`terminal = false` after the swap): the intact run before a damaged middle ends in
/// a `GameEnd` the store has not applied. The rebased file ends in it, so the whole file goes to
/// the applier and the game is finalized (and its journal closed), not left in progress.
#[test]
fn a_rebased_journal_that_ends_in_a_game_end_finalizes_the_game_at_startup() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    payloads.push(voided(None));
    payloads.push(voided(None));
    write_journal(&s, 1, payloads);
    corrupt_record(&jpath(d.path(), 1), 4, store_uuid(d.path()).as_u128(), 1);
    let opened = reopen(d.path(), s);
    let outcome = opened.report.recovery.outcome_of(gid(1)).cloned();
    assert_eq!(
        block_on(opened.store.get_game(gid(1))).unwrap().status,
        GameStatus::Completed,
        "{outcome:?}"
    );
    assert!(
        matches!(outcome, Some(RecoveryOutcome::Finalized)),
        "{outcome:?}"
    );
}

// ------------------------------------------------------ the completeness check of a `.rebase`

/// The bytes of a `.rebase` for game `n`: header, then the records.
fn rebase_bytes(d: &Path, n: u128, records: &[Payload]) -> Vec<u8> {
    let mut out = encode_header(Uuid::from_u128(n), store_uuid(d)).to_vec();
    for (i, payload) in records.iter().enumerate() {
        let record = rec(u64::try_from(i).unwrap() + 1, payload.clone());
        out.extend(encode_record(&record).unwrap());
    }
    out
}

fn settle(d: &Path, n: u128, bytes: &[u8]) -> Settled {
    let jdir = d.join("journal");
    let _ = fs::remove_file(jpath(d, n));
    fs::write(rebase_path(d, n), bytes).unwrap();
    settle_rebase_files(&jdir, Uuid::from_u128(n), store_uuid(d)).unwrap()
}

fn running_clock() -> Payload {
    Payload::Clock(ClockPayload {
        white_ms: Some(1),
        black_ms: Some(2),
        running_side: Some(Side::White),
    })
}

/// Mutants 48 and 49 (`ends_a_rebase`): a `.rebase` that ends in an `Abort` or a `Voided` is
/// complete; one that ends in a `Clock` with a running side is not (the closing `Clock` of a
/// rebase stops the clock).
#[test]
fn a_rebase_is_complete_when_it_ends_in_a_terminal_or_a_stopped_clock_only() {
    let (d, _s) = live();
    let abort = Payload::Abort(chess_store::journal::format::AbortPayload {
        ended_at: ENDED,
        content_hash: "sha256:v1:ab".into(),
    });
    let ends = |tail: Payload| {
        let mut r = playing(1, 1);
        r.push(tail);
        r
    };
    let cases: [(&str, Vec<Payload>, bool); 5] = [
        ("abort", ends(abort), true),
        ("voided", ends(voided(None)), true),
        ("game end", ends(game_end(ENDED)), true),
        ("stopped clock", ends(jb::clock(1, 2)), true),
        ("running clock", ends(running_clock()), false),
    ];
    for (name, records, complete) in cases {
        let bytes = rebase_bytes(d.path(), 1, &records);
        let settled = settle(d.path(), 1, &bytes);
        let got = matches!(settled, Settled::RenamedIntoPlace);
        assert_eq!(got, complete, "{name}: {settled:?}");
        // Leave nothing behind for the next case.
        let _ = fs::remove_file(jpath(d.path(), 1));
        for aside in damaged_files(d.path(), 1, "") {
            fs::remove_file(aside).unwrap();
        }
    }
}

/// Mutants 44 to 47: a `.rebase` that is only a header, one with bytes after its last record,
/// and one with a CRC-valid frame the decoder refuses after it, are all incomplete.
#[test]
fn a_rebase_with_no_record_or_with_a_bad_tail_is_incomplete() {
    let (d, _s) = live();
    let good = rebase_bytes(d.path(), 1, &{
        let mut r = playing(1, 1);
        r.push(jb::clock(1, 2));
        r
    });
    let mut garbage = good.clone();
    garbage.extend_from_slice(&[0xA5, 0x5A, 0x01]);
    let mut undecodable = good.clone();
    undecodable.extend(common_undecodable_frame(5));
    let header_only = rebase_bytes(d.path(), 1, &[]);
    for (name, bytes) in [
        ("header only", header_only),
        ("garbage tail", garbage),
        ("undecodable tail", undecodable),
    ] {
        let settled = settle(d.path(), 1, &bytes);
        assert!(
            matches!(settled, Settled::MovedAsideIncomplete { .. }),
            "{name}: {settled:?}"
        );
        let _ = fs::remove_file(jpath(d.path(), 1));
        for aside in damaged_files(d.path(), 1, "") {
            fs::remove_file(aside).unwrap();
        }
    }
}

/// A CRC-valid frame whose payload the decoder refuses (a `GameEnd` with `ended_at` 0).
fn common_undecodable_frame(seq: u64) -> Vec<u8> {
    use chess_store::journal::format::{EndReason, GameEndPayload, crc32};
    let payload = serde_json::to_vec(&GameEndPayload {
        result: "1-0".into(),
        reason: EndReason::Checkmate,
        ended_at: 0,
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
    body.push(12);
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

// ------------------------------------------------------------------ parked games at startup

/// A game the store parked is parked again by the next start (the parked set is not persisted;
/// the scan passes the whole journal again and the store refuses it again). The scan reports
/// `Parked` with the cause the store gives, one entry, no retry loop. Observed behavior; the
/// `GameParked` branch of the scan's error classification (mutant 28) is not reached by this
/// path (evidence file, finding FIND-D4V-05).
#[test]
fn a_game_parked_at_one_start_is_parked_again_at_the_next_with_its_cause() {
    use chess_store::journal::format::GameEndPayload;
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(Payload::GameEnd(GameEndPayload {
        content_hash: String::new(),
        ..match game_end(ENDED) {
            Payload::GameEnd(e) => e,
            _ => panic!(),
        }
    }));
    write_journal(&s, 1, payloads);
    let first = reopen(d.path(), s);
    let one = first.report.recovery.outcome_of(gid(1)).cloned();
    let second = reopen(d.path(), first.store);
    let two = second.report.recovery.outcome_of(gid(1)).cloned();
    for outcome in [&one, &two] {
        assert!(
            matches!(outcome, Some(RecoveryOutcome::Parked { cause: Some(_) })),
            "{outcome:?}"
        );
    }
    assert_eq!(second.report.recovery.games.len(), 1);
    assert_eq!(block_on(second.store.parked_games()).unwrap().len(), 1);
}

// ----------------------------------------------------- an incomplete `.rebase` found at startup

/// B-R05 at startup (the resume path has its own test): no journal, an in-progress row with
/// moves, and a `.rebase` that is cut short. The scan moves the half file to `damaged/` with its
/// bytes, rebuilds the journal from the store, and reports it.
#[test]
fn b_r05_an_incomplete_rebase_is_moved_aside_and_the_game_rebuilt_at_startup() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 3));
    apply(&s, 1, records);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let mut cut = rebase_bytes(d.path(), 1, &playing(1, 2));
    cut.truncate(cut.len() - 4);
    fs::write(rebase_path(d.path(), 1), &cut).unwrap();
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::IncompleteRebaseMovedAside)
    );
    let aside = damaged_files(d.path(), 1, ".rebase");
    assert_eq!(aside.len(), 1);
    assert_eq!(fs::read(&aside[0]).unwrap(), cut);
    assert!(!rebase_path(d.path(), 1).exists());
    assert!(jpath(d.path(), 1).exists(), "rebuilt from the store");
    assert_eq!(block_on(opened.store.game_moves(gid(1))).unwrap().len(), 3);
}

// ------------------------------------- the `Start` rebuilt from a row with a computer and a clock

/// The rebase of a game whose journal is gone builds its `Start` from the row (3.9.2, RR-A-1
/// (a)): the seat kinds (a computer among them), the matchup code and the clock come from the
/// `GAME` row and the `PLAYER` rows. Coverage found `SeatKind::Computer` and the clock branch
/// of `start_from_row` unexercised.
#[test]
fn the_start_rebuilt_from_a_row_carries_a_computer_seat_and_the_clock() {
    use chess_domain::{Matchup, Mode};
    use chess_store::journal::format::{SeatKind, StartPayload, UuidText};
    let (d, s) = d4_support::world();
    let mut new = common::new_game(
        1,
        common::pid(1),
        common::pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    );
    new.clock = Some(common::blitz());
    block_on(s.start_game(new)).unwrap();
    let Payload::Start(base) = jb::start(1) else {
        panic!()
    };
    let start = Payload::Start(StartPayload {
        black: UuidText(Uuid::from_u128(10)),
        black_kind: SeatKind::Computer,
        matchup: "hc".into(),
        ..base
    });
    let records = write_journal(&s, 1, vec![start, jb::mv(0), jb::mv(1)]);
    apply(&s, 1, records);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased, "{:?}", recovered.report);
    let Some(Payload::Start(built)) = recovered.records.first().map(|r| r.payload.clone()) else {
        panic!("{:?}", recovered.records.first());
    };
    assert_eq!(built.white_kind, SeatKind::Human);
    assert_eq!(built.black_kind, SeatKind::Computer);
    assert_eq!(built.matchup, "hc");
    assert_eq!(built.black, UuidText(Uuid::from_u128(10)));
    assert_eq!(
        built.clock_spec,
        chess_store::journal::format::ClockText::parse("300+2")
    );
    assert!(writer.is_some());
}

// ------------------------------------------------ observed: a `Voided` after an applied `GameEnd`

/// OBSERVED, not intended (FIND-D4V-03 in the evidence file): the store holds the `GameEnd`
/// (the row is `completed`) and the journal holds one more record, a `Voided`, that the store
/// does not. The scan closes the journal to `done/` and does not apply the `Voided`; the row
/// stays `completed`. The test pins what the code does today so that a change shows; whether
/// the `Voided` should win is a question for the Systems Engineer (D-87 speaks of a `GameEnd`
/// followed by a `Voided` **before** the finalize commit, not after it).
#[test]
fn observed_a_voided_after_an_applied_game_end_is_closed_without_being_applied() {
    let (d, s) = live();
    let mut payloads = playing(1, 3);
    payloads.push(game_end(ENDED));
    payloads.push(voided(None));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records[..5].to_vec());
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::ClosedLeftover)
    );
    assert_eq!(
        block_on(opened.store.get_game(gid(1))).unwrap().status,
        GameStatus::Completed
    );
}

// --------------------------------- an incomplete `.rebase` at startup for a final or empty row

/// B-R05 at startup for the two rows that need no rebuild: a final game (nothing to rebuild; the
/// half file is kept in `damaged/`) and an in-progress row with nothing stored (the next resume
/// creates the header). Neither gets a journal from the scan.
#[test]
fn b_r05_an_incomplete_rebase_for_a_final_or_an_empty_row_is_moved_aside_and_nothing_is_built() {
    for final_game in [true, false] {
        let (d, s) = live();
        if final_game {
            let mut payloads = playing(1, 2);
            payloads.push(game_end(ENDED));
            let records = write_journal(&s, 1, payloads);
            apply(&s, 1, records);
            fs::remove_file(jpath(d.path(), 1)).unwrap();
        }
        let mut cut = rebase_bytes(d.path(), 1, &playing(1, 2));
        cut.truncate(cut.len() - 4);
        fs::write(rebase_path(d.path(), 1), &cut).unwrap();
        let opened = reopen(d.path(), s);
        let outcome = opened.report.recovery.outcome_of(gid(1)).cloned();
        assert!(
            matches!(outcome, Some(RecoveryOutcome::Reconciled { .. })),
            "final {final_game}: {outcome:?}"
        );
        let aside = damaged_files(d.path(), 1, ".rebase");
        assert_eq!(aside.len(), 1, "final {final_game}");
        assert_eq!(fs::read(&aside[0]).unwrap(), cut);
        assert!(!rebase_path(d.path(), 1).exists());
        assert!(!jpath(d.path(), 1).exists(), "the scan builds no journal");
    }
}
