//! Part B kill tests (feature `test-support`): a child process is killed at a journal failpoint
//! and the parent inspects what is on disk: CT-ST-003 (every `journal.*` point, and the kill
//! between the last `Move` and the `GameEnd` for each terminal reason), CT-ST-087 (a failed
//! append that was cleaned up, then a kill), CT-ST-088 (six kill points of the rebase, file
//! half) and the CT-ST-089 restart. What is **deferred to part D4**: the resume of a killed
//! journal by a new writer (`open_for_resume`, which truncates the torn tail), the database
//! half of the rebase (step 2) and the startup scan that drives the repair; here the parent
//! runs a small stand-in routine in the same shape (settle, scan, reconcile, build, write, swap).
#![cfg(feature = "test-support")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chess_store::fault::FaultPlan;
use chess_store::journal::format::{EndReason, GameEndPayload, HEADER_LEN, Payload, encode_record};
use chess_store::journal::{
    MoveJournal, RebaseInput, ReconcileInput, StoredMove, build_rebased, fold, reconcile,
    reconciled_clocks, scan_bytes, scan_file, settle_rebase_files, swap_in_rebased,
    write_rebase_file,
};
use chess_store::{DataDir, JournalError};
use common::child::{self, Spawned};
use common::jb;
use common::{block_on, gid};
use tempfile::TempDir;
use uuid::Uuid;

const STORE: u128 = 0x5701;
const GAME: u128 = 1;
const ENV_SCENARIO: &str = "CHESS_STORE_JC_SCENARIO";
const ENV_DIR: &str = "CHESS_STORE_JC_DIR";
const ENV_PLAN: &str = "CHESS_STORE_JC_PLAN";
const ENV_REASON: &str = "CHESS_STORE_JC_REASON";

const REASONS: [(&str, EndReason); 9] = [
    ("checkmate", EndReason::Checkmate),
    ("stalemate", EndReason::Stalemate),
    ("insufficient_material", EndReason::InsufficientMaterial),
    ("threefold_repetition", EndReason::ThreefoldRepetition),
    ("fifty_move_rule", EndReason::FiftyMoveRule),
    ("time_forfeit", EndReason::TimeForfeit),
    (
        "timeout_opponent_cannot_mate",
        EndReason::TimeoutOpponentCannotMate,
    ),
    ("resignation", EndReason::Resignation),
    ("draw_agreed", EndReason::DrawAgreed),
];

fn jdir(dir: &Path) -> PathBuf {
    dir.join("journal")
}

fn jpath(dir: &Path, game: u128) -> PathBuf {
    jdir(dir).join(format!("{}.jnl", Uuid::from_u128(game).hyphenated()))
}

fn journal(dir: &Path) -> MoveJournal {
    let data = DataDir::new(dir);
    data.ensure().unwrap();
    MoveJournal::new(data, Uuid::from_u128(STORE))
}

fn game_end(reason: EndReason) -> Payload {
    Payload::GameEnd(GameEndPayload {
        result: "1-0".into(),
        reason,
        ended_at: 99,
        eco: None,
        opening_name: None,
        analysis_used: false,
        content_hash: "sha256:v1:00".into(),
        white_time_used_ms: 1,
        black_time_used_ms: 2,
        total_active_ms: 3,
    })
}

// ------------------------------------------------------------------ the rebase stand-in

/// The state the "database" holds: `db_seq` in a file (the real one is `GAME.journal_seq`).
fn db_seq(dir: &Path) -> u64 {
    fs::read_to_string(dir.join("db_seq")).map_or(7, |t| t.trim().parse().unwrap())
}

fn stored_rows() -> Vec<StoredMove> {
    (1..=5u32)
        .map(|p| {
            let j = p - 1;
            StoredMove {
                ply: p,
                uci: format!("u{j}"),
                san: format!("s{j}"),
                fen_after: format!("fen{j}"),
                think_ms: Some(10 + u64::from(j)),
                clocks: Some((300_000 - u64::from(j), 300_000 + u64::from(j))),
                played_at: Some(10 + i64::from(j)),
            }
        })
        .collect()
}

/// A journal of Start and four moves with record 4 (journal ply 2) damaged: the intact run is
/// `Start, Move 0, Move 1`. Returns the damaged bytes.
fn make_damaged(dir: &Path) -> Vec<u8> {
    let w = journal(dir).create(&gid(GAME)).unwrap();
    block_on(w.append(1, jb::start(GAME))).unwrap();
    for ply in 0..4 {
        block_on(w.append(10 + i64::from(ply), jb::mv(ply))).unwrap();
    }
    drop(w);
    let mut bytes = fs::read(jpath(dir, GAME)).unwrap();
    let s = scan_bytes(&bytes, Uuid::from_u128(GAME), Uuid::from_u128(STORE)).unwrap();
    let at = HEADER_LEN
        + s.records[..3]
            .iter()
            .map(|r| encode_record(r).unwrap().len())
            .sum::<usize>();
    bytes[at + 14] ^= 0xFF;
    fs::write(jpath(dir, GAME), &bytes).unwrap();
    fs::write(dir.join("db_seq"), "7").unwrap();
    bytes
}

/// The rebase as D4 will drive it, at file level: repair the file state, scan, and if the
/// journal is damaged (a damaged middle, or `db_seq > last_intact_seq`, D-66) rebuild it. The
/// "database step" is writing `db_seq`.
fn rebase_routine(dir: &Path) -> Result<(), JournalError> {
    let game = Uuid::from_u128(GAME);
    let dirj = jdir(dir);
    settle_rebase_files(&dirj, game).map_err(|e| JournalError::write_failed_for_test(&e))?;
    let path = jpath(dir, GAME);
    let Ok(bytes) = fs::read(&path) else {
        return Ok(());
    };
    let scan = scan_bytes(&bytes, game, Uuid::from_u128(STORE))?;
    let seq = db_seq(dir);
    if scan.damaged.is_none() && seq <= scan.last_seq() {
        return Ok(());
    }
    let folded = fold(&scan.records);
    let rows = stored_rows();
    let plan = reconcile(&ReconcileInput {
        journal: &folded,
        stored: &rows,
        scan_damaged: scan.damaged.is_some(),
        db_seq: seq,
        last_intact_seq: scan.last_seq(),
        stored_clocks: Some((7, 8)),
    });
    let rebased = build_rebased(&RebaseInput {
        game,
        store: Uuid::from_u128(STORE),
        intact_bytes: &bytes[HEADER_LEN..scan.intact_end],
        intact: &scan.records,
        source: plan.source,
        journal_last_ply: u32::try_from(folded.moves.len()).unwrap(),
        stored: &rows,
        clocks: reconciled_clocks(&plan, Some((7, 8))),
        start: None,
    })
    .unwrap();
    let file = write_rebase_file(&dirj, game, &rebased)?;
    // Stand-in for step 2 (one store transaction, part D4): the database now says `N`.
    fs::write(dir.join("db_seq"), rebased.last_seq.to_string()).unwrap();
    chess_store::fault::trigger("journal.rebase.db_committed", &path)
        .map_err(|e| JournalError::write_failed_for_test(&e))?;
    swap_in_rebased(&dirj, game, file).map(drop)
}

trait TestErr {
    fn write_failed_for_test(e: &std::io::Error) -> JournalError;
}

impl TestErr for JournalError {
    fn write_failed_for_test(e: &std::io::Error) -> JournalError {
        JournalError::WriteFailed {
            cause: chess_store::WriteCause::Io(e.kind()),
        }
    }
}

// ------------------------------------------------------------------------------ the child

#[test]
fn crash_child_entry() {
    let Ok(scenario) = std::env::var(ENV_SCENARIO) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(ENV_DIR).unwrap());
    let plan = FaultPlan::parse(&std::env::var(ENV_PLAN).unwrap()).unwrap();
    let _guard = plan.install(&dir).unwrap();
    match scenario.as_str() {
        "append" => {
            let w = journal(&dir).create(&gid(GAME)).unwrap();
            let ack = |seq: u64| println!("CHILD ACK {seq}");
            ack(block_on(w.append(1, jb::start(GAME))).unwrap());
            for ply in 0..8 {
                // A failed append (injected error) is retried once, like a session would.
                let wall = 10 + i64::from(ply);
                let seq = match block_on(w.append(wall, jb::mv(ply))) {
                    Ok(seq) => seq,
                    Err(_) => block_on(w.append(wall, jb::mv(ply))).unwrap(),
                };
                ack(seq);
            }
        }
        "terminal" => {
            let reason = std::env::var(ENV_REASON).unwrap();
            let (_, end) = REASONS.iter().find(|(name, _)| *name == reason).unwrap();
            let w = journal(&dir).create(&gid(GAME)).unwrap();
            block_on(w.append(1, jb::start(GAME))).unwrap();
            for ply in 0..3 {
                block_on(w.append(10 + i64::from(ply), jb::mv(ply))).unwrap();
            }
            println!("CHILD ACK final move");
            // The kill comes before this record is written.
            let _ = block_on(w.append(50, game_end(*end)));
        }
        "rebase" => {
            rebase_routine(&dir).unwrap();
        }
        other => panic!("unknown scenario {other}"),
    }
}

fn run(scenario: &str, dir: &Path, plan: &str, reason: &str) -> Spawned {
    child::spawn(
        "crash_child_entry",
        &[
            (ENV_SCENARIO, scenario),
            (ENV_DIR, dir.to_str().unwrap()),
            (ENV_PLAN, plan),
            (ENV_REASON, reason),
        ],
    )
}

fn kill_at(scenario: &str, dir: &Path, plan: &str, reason: &str, point: &str) -> Vec<String> {
    let mut child = run(scenario, dir, plan, reason);
    let at = child
        .killed_at(Duration::from_secs(60))
        .unwrap_or_else(|| panic!("never reached {point}"));
    assert_eq!(at, point);
    let mut acks = Vec::new();
    while let Some(line) = child.line(Duration::from_millis(200)) {
        acks.push(line);
    }
    child.kill();
    acks
}

// --------------------------------------------------------------------------- CT-ST-003

/// CT-ST-003: kill at each `journal.*` failpoint of the fourth append; every acknowledged
/// record is present with its exact payload, the scan finds no damage, `seq` is contiguous and
/// the clocks are the last recorded ones. A kill before the sync leaves the record in the file
/// (the OS cache survives a process kill; power loss is `[UNV]`); `partial_write` leaves half a
/// record, which is a torn tail that hides nothing.
#[test]
fn ct_st_003_a_kill_at_each_journal_failpoint_loses_no_acknowledged_record() {
    for (point, torn, last_present) in [
        ("journal.before_write", false, false),
        ("journal.partial_write", true, false),
        ("journal.after_write", false, true),
        ("journal.before_fsync", false, true),
        ("journal.fsync_error", false, true),
        ("journal.after_fsync", false, true),
    ] {
        let dir = TempDir::new().unwrap();
        // Hits are counted per append: Start, Move 0, Move 1 pass, the fourth append is killed.
        let acks = kill_at("append", dir.path(), &format!("{point}=kill@3"), "", point);
        let acked: Vec<u64> = acks
            .iter()
            .filter_map(|l| l.strip_prefix("ACK ").and_then(|n| n.trim().parse().ok()))
            .collect();
        assert_eq!(
            acked,
            vec![1, 2, 3],
            "{point}: the fourth append was never acknowledged"
        );
        let s = scan_file(
            &jpath(dir.path(), GAME),
            Uuid::from_u128(GAME),
            Uuid::from_u128(STORE),
        )
        .unwrap();
        assert!(s.damaged.is_none(), "{point}");
        assert_eq!(s.torn_tail_dropped, torn, "{point}");
        let seqs: Vec<u64> = s.records.iter().map(|r| r.seq).collect();
        assert_eq!(
            seqs,
            if last_present {
                vec![1, 2, 3, 4]
            } else {
                vec![1, 2, 3]
            },
            "{point}"
        );
        // The acknowledged records are exactly what was appended.
        assert_eq!(s.records[0].payload, jb::start(GAME), "{point}");
        assert_eq!(s.records[1].payload, jb::mv(0));
        assert_eq!(s.records[2].payload, jb::mv(1));
        // Clocks: the last recorded Move's clocks.
        let folded = fold(&s.records);
        let last_ply = if last_present { 2 } else { 1 };
        assert_eq!(
            folded.clocks,
            Some(Some((300_000 - last_ply, 300_000 + last_ply))),
            "{point}"
        );
    }
}

/// CT-ST-003 (D-65): a kill between the final `Move` record and the `GameEnd` record, for each
/// terminal reason: the journal ends with the `Move`, the scan and reconcile return that state
/// unchanged and no `GameEnd` is applied.
#[test]
fn ct_st_003_a_kill_between_the_last_move_and_game_end_leaves_the_move_for_every_reason() {
    for (name, _) in REASONS {
        let dir = TempDir::new().unwrap();
        // Hits of before_write: Start, three moves pass; the fifth append (GameEnd) is killed.
        kill_at(
            "terminal",
            dir.path(),
            "journal.before_write=kill@4",
            name,
            "journal.before_write",
        );
        let s = scan_file(
            &jpath(dir.path(), GAME),
            Uuid::from_u128(GAME),
            Uuid::from_u128(STORE),
        )
        .unwrap();
        assert_eq!(s.records.len(), 4, "{name}");
        assert!(
            matches!(s.records.last().unwrap().payload, Payload::Move(_)),
            "{name}"
        );
        assert!(
            !s.records
                .iter()
                .any(|r| matches!(r.payload, Payload::GameEnd(_))),
            "{name}"
        );
        assert!(s.damaged.is_none() && !s.torn_tail_dropped, "{name}");
        let folded = fold(&s.records);
        let rows: Vec<StoredMove> = stored_rows().into_iter().take(3).collect();
        let plan = reconcile(&ReconcileInput {
            journal: &folded,
            stored: &rows,
            scan_damaged: false,
            db_seq: 4,
            last_intact_seq: 4,
            stored_clocks: Some((300_000 - 2, 300_000 + 2)),
        });
        assert!(plan.is_empty(), "{name}: {plan:?}");
    }
}

// --------------------------------------------------------------------------- CT-ST-087

/// CT-ST-087: a failed append (injected error at `journal.partial_write` or
/// `journal.fsync_error`, cleaned up) is retried; the process is then killed at the next
/// append. Every acknowledged record is present, the scan finds no damage and no torn tail
/// that hides a later record, and no `seq` appears twice.
#[test]
fn ct_st_087_a_cleaned_up_failure_then_a_kill_leaves_no_duplicate_and_no_damage() {
    for failure in ["journal.partial_write", "journal.fsync_error"] {
        let dir = TempDir::new().unwrap();
        // Appends 1..=3 pass; the 4th fails (once) and is retried; the retry reaches
        // `after_write` for the 4th time overall and is killed there.
        let plan = format!("{failure}=errno:5@3!,journal.after_write=kill@3");
        let acks = kill_at("append", dir.path(), &plan, "", "journal.after_write");
        let acked: Vec<u64> = acks
            .iter()
            .filter_map(|l| l.strip_prefix("ACK ").and_then(|n| n.trim().parse().ok()))
            .collect();
        assert_eq!(acked, vec![1, 2, 3], "{failure}");
        let s = scan_file(
            &jpath(dir.path(), GAME),
            Uuid::from_u128(GAME),
            Uuid::from_u128(STORE),
        )
        .unwrap();
        let seqs: Vec<u64> = s.records.iter().map(|r| r.seq).collect();
        assert_eq!(
            seqs,
            vec![1, 2, 3, 4],
            "{failure}: the failed record's seq was reused, none twice"
        );
        assert!(s.damaged.is_none() && !s.torn_tail_dropped, "{failure}");
        assert_eq!(s.records[3].payload, jb::mv(2));
        let mut sorted = seqs.clone();
        sorted.dedup();
        assert_eq!(sorted, seqs);
    }
}

// --------------------------------------------------------------------------- CT-ST-088

fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for sub in [jdir(dir), jdir(dir).join("damaged")] {
        if let Ok(entries) = fs::read_dir(&sub) {
            for e in entries.flatten() {
                if e.path().is_file() {
                    out.insert(
                        e.path()
                            .strip_prefix(dir)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        fs::read(e.path()).unwrap(),
                    );
                }
            }
        }
    }
    out
}

fn check_final_state(
    dir: &Path,
    original: &[u8],
    reference: &BTreeMap<String, Vec<u8>>,
    label: &str,
) {
    let jnl_name = format!("journal/{}.jnl", Uuid::from_u128(GAME).hyphenated());
    let snap = snapshot(dir);
    assert_eq!(
        snap.get(&jnl_name),
        reference.get(&jnl_name),
        "{label}: final journal equals the uninterrupted result"
    );
    let damaged: Vec<&String> = snap
        .keys()
        .filter(|k| k.starts_with("journal/damaged/"))
        .collect();
    assert_eq!(
        damaged.len(),
        1,
        "{label}: exactly one damaged file: {damaged:?}"
    );
    assert_eq!(
        snap[damaged[0]], original,
        "{label}: the damaged file is the original, byte for byte"
    );
    assert!(
        !snap.keys().any(|k| k.ends_with(".rebase")),
        "{label}: no .rebase remains"
    );
    for name in snap.keys() {
        assert_eq!(
            fs::metadata(dir.join(name)).unwrap().nlink(),
            1,
            "{label}: {name} has one name"
        );
    }
    assert_eq!(db_seq(dir), 7, "{label}: journal_seq equals the last seq");
    let s = scan_file(
        &jpath(dir, GAME),
        Uuid::from_u128(GAME),
        Uuid::from_u128(STORE),
    )
    .unwrap();
    assert!(s.damaged.is_none());
    assert_eq!(s.last_seq(), 7);
}

/// CT-ST-088 (file half): a kill at each of the six rebase failpoints, then a restart (the
/// stand-in routine): the final journal equals the uninterrupted result, exactly one
/// `damaged/<uuid>.<k>.jnl` exists, no `.rebase` remains, no file has two names, `journal_seq`
/// equals the last `seq`, and a second restart changes nothing. Deferred to D4: the same test
/// with the real database step and the real startup scan.
#[test]
fn ct_st_088_a_kill_at_each_of_the_six_rebase_points_is_repaired_to_the_same_result() {
    let reference_dir = TempDir::new().unwrap();
    let original = make_damaged(reference_dir.path());
    rebase_routine(reference_dir.path()).unwrap();
    let reference = snapshot(reference_dir.path());
    assert_eq!(
        reference.len(),
        2,
        "journal and one damaged file: {:?}",
        reference.keys()
    );
    for point in [
        "journal.rebase.built",
        "journal.rebase.db_committed",
        "journal.rebase.old_linked",
        "journal.rebase.old_aside",
        "journal.rebase.new_linked",
        "journal.rebase.new_in_place",
    ] {
        let dir = TempDir::new().unwrap();
        let again = make_damaged(dir.path());
        assert_eq!(again, original, "the fixture is deterministic");
        kill_at("rebase", dir.path(), &format!("{point}=kill"), "", point);
        // Restart: the repair of the file state and the rebuild if the journal is still damaged.
        rebase_routine(dir.path()).unwrap();
        check_final_state(dir.path(), &original, &reference, point);
        // A second restart changes nothing.
        let before = snapshot(dir.path());
        rebase_routine(dir.path()).unwrap();
        assert_eq!(
            snapshot(dir.path()),
            before,
            "{point}: a second restart changes nothing"
        );
    }
}

/// The two states that kill points leave which the table of D-110 names: after `old_linked`
/// the damaged file has two names until the restart; after `new_linked` the rebased file has
/// two names. Checked before the restart.
#[test]
fn ct_st_088_the_states_between_the_two_links_and_the_removals_have_two_names() {
    for (point, name_a, name_b) in [
        (
            "journal.rebase.old_linked",
            "journal/{g}.jnl",
            "journal/damaged/{g}.1.jnl",
        ),
        (
            "journal.rebase.new_linked",
            "journal/{g}.jnl",
            "journal/{g}.jnl.rebase",
        ),
    ] {
        let dir = TempDir::new().unwrap();
        make_damaged(dir.path());
        kill_at("rebase", dir.path(), &format!("{point}=kill"), "", point);
        let g = Uuid::from_u128(GAME).hyphenated().to_string();
        let (a, b) = (
            dir.path().join(name_a.replace("{g}", &g)),
            dir.path().join(name_b.replace("{g}", &g)),
        );
        let (ma, mb) = (fs::metadata(&a).unwrap(), fs::metadata(&b).unwrap());
        assert_eq!(
            (ma.dev(), ma.ino()),
            (mb.dev(), mb.ino()),
            "{point}: one file, two names"
        );
        assert_eq!(ma.nlink(), 2, "{point}");
    }
}

/// The window (c) state (between the two moves): no `<uuid>.jnl`, a complete `.rebase`, the old
/// file in `damaged/`. The file-level repair renames the `.rebase` into place and no second
/// `damaged/` file is made (CT-ST-089, "row with no journal" path at file level).
#[test]
fn ct_st_089_after_the_first_move_a_restart_renames_the_rebase_into_place() {
    let dir = TempDir::new().unwrap();
    let original = make_damaged(dir.path());
    kill_at(
        "rebase",
        dir.path(),
        "journal.rebase.old_aside=kill",
        "",
        "journal.rebase.old_aside",
    );
    let g = Uuid::from_u128(GAME).hyphenated().to_string();
    assert!(!jpath(dir.path(), GAME).exists());
    assert!(jdir(dir.path()).join(format!("{g}.jnl.rebase")).exists());
    let rebase_bytes = fs::read(jdir(dir.path()).join(format!("{g}.jnl.rebase"))).unwrap();
    rebase_routine(dir.path()).unwrap();
    assert_eq!(
        fs::read(jpath(dir.path(), GAME)).unwrap(),
        rebase_bytes,
        "the complete .rebase became the journal"
    );
    let damaged: Vec<_> = fs::read_dir(jdir(dir.path()).join("damaged"))
        .unwrap()
        .collect();
    assert_eq!(damaged.len(), 1);
    assert_eq!(
        fs::read(damaged[0].as_ref().unwrap().path()).unwrap(),
        original
    );
}
