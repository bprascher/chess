//! Part D5: evaluations and variations through the public `Store` API (UT-ST-037, decision 12
//! and 13, CR-9.3, CR-9.4; OQ-ST-003 stays open: one line per row).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d5_support;

use std::sync::atomic::Ordering;

use chess_domain::WallMs;
use chess_store::{Entity, EvalFlag, JobState, MAX_VARIATION_PGN_BYTES, MoveEvalRow, StoreError};
use common::{block_on, gid, raw};
use d5_support::{G, seed, world, world_with_clock};
use tempfile::TempDir;

fn eval(game: u128, ply: u32) -> MoveEvalRow {
    MoveEvalRow {
        game: gid(game),
        ply,
        cp: Some(35),
        mate: None,
        best_uci: Some("e2e4".to_owned()),
        cp_loss: Some(12),
        flag: Some(EvalFlag::Inaccuracy),
        depth: Some(18),
        engine_version: "Stockfish 19".to_owned(),
    }
}

fn game_with(dir: &std::path::Path, n: u128, plies: u32) {
    let mut g = G::done(n, 1, 2, "1-0");
    g.plies = plies;
    seed(dir, &g);
}

#[test]
fn an_eval_is_stored_as_given_from_whites_point_of_view_and_read_back() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 4);
    let mut black_move = eval(1, 2);
    black_move.cp = Some(-250); // White is behind: negative, whoever moved
    black_move.flag = Some(EvalFlag::Blunder);
    let mate = MoveEvalRow {
        cp: None,
        mate: Some(-3),
        cp_loss: None,
        flag: None,
        best_uci: None,
        depth: None,
        ..eval(1, 4)
    };
    for e in [mate.clone(), black_move.clone(), eval(1, 1)] {
        block_on(s.upsert_move_eval(e)).unwrap();
    }
    let a = block_on(s.get_game_analysis(gid(1))).unwrap();
    assert_eq!(
        a.evals,
        [eval(1, 1), black_move, mate],
        "ply order, values unchanged"
    );
    assert_eq!((a.evaluated, a.total), (3, 4));
    assert_eq!(a.job, None, "no job row");
}

#[test]
fn a_second_upsert_of_a_ply_replaces_every_field() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 2);
    block_on(s.upsert_move_eval(eval(1, 1))).unwrap();
    let again = MoveEvalRow {
        cp: None,
        mate: Some(2),
        best_uci: None,
        cp_loss: Some(0),
        flag: None,
        depth: Some(30),
        engine_version: "Stockfish 20".to_owned(),
        ..eval(1, 1)
    };
    block_on(s.upsert_move_eval(again.clone())).unwrap();
    let a = block_on(s.get_game_analysis(gid(1))).unwrap();
    assert_eq!(a.evals, [again]);
    assert_eq!(a.evaluated, 1, "one row per ply");
}

#[test]
fn bad_evals_are_refused_and_write_nothing() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 2);
    let both = MoveEvalRow {
        mate: Some(1),
        ..eval(1, 1)
    };
    assert!(matches!(
        block_on(s.upsert_move_eval(both)),
        Err(StoreError::InvariantViolated)
    ));
    for ply in [0, 3] {
        assert!(
            matches!(
                block_on(s.upsert_move_eval(eval(1, ply))),
                Err(StoreError::InvariantViolated)
            ),
            "ply {ply} has no MOVE row"
        );
    }
    let unknown = block_on(s.upsert_move_eval(eval(77, 1)));
    assert!(
        matches!(unknown, Err(StoreError::NotFound { entity: Entity::Game, id }) if id == gid(77).as_uuid()),
        "{unknown:?}"
    );
    assert_eq!(block_on(s.get_game_analysis(gid(1))).unwrap().evaluated, 0);
}

#[test]
fn get_game_analysis_reports_the_job_state_and_unknown_games() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 3);
    block_on(s.enqueue_job(gid(1))).unwrap();
    assert_eq!(
        block_on(s.get_game_analysis(gid(1))).unwrap().job,
        Some(JobState::Queued)
    );
    let owner = "cli:1".to_owned();
    assert_eq!(block_on(s.claim_job(owner.clone())).unwrap(), Some(gid(1)));
    assert_eq!(
        block_on(s.get_game_analysis(gid(1))).unwrap().job,
        Some(JobState::Running)
    );
    block_on(s.complete_job(gid(1), owner)).unwrap();
    assert_eq!(
        block_on(s.get_game_analysis(gid(1))).unwrap().job,
        Some(JobState::Done)
    );
    assert!(matches!(
        block_on(s.get_game_analysis(gid(5))),
        Err(StoreError::NotFound {
            entity: Entity::Game,
            ..
        })
    ));
}

#[test]
fn a_game_without_moves_reports_zero_of_zero() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 0);
    let a = block_on(s.get_game_analysis(gid(1))).unwrap();
    assert_eq!((a.evaluated, a.total, a.evals.len()), (0, 0, 0));
}

// ------------------------------------------------------------------ variations

#[test]
fn ut_st_037_a_branch_from_the_final_position_is_refused_below_it_is_stored() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 5_000);
    game_with(dir.path(), 1, 4);
    for from_ply in [4, 5, 400] {
        let refused = block_on(s.save_variation(gid(1), from_ply, "1. e4".to_owned()));
        assert!(
            matches!(refused, Err(StoreError::VariationAnchor)),
            "{from_ply}: {refused:?}"
        );
    }
    assert!(
        block_on(s.list_variations(gid(1))).unwrap().is_empty(),
        "refusals stored nothing"
    );
    now.store(7_777, Ordering::SeqCst);
    let saved = block_on(s.save_variation(gid(1), 3, "3... Nf6 4. d4".to_owned())).unwrap();
    assert_eq!(saved.game, gid(1));
    assert_eq!(saved.from_ply, 3);
    assert_eq!(saved.pgn, "3... Nf6 4. d4");
    assert_eq!(
        saved.created_at,
        WallMs(7_777),
        "created_at is the injected clock"
    );
    assert_eq!(block_on(s.list_variations(gid(1))).unwrap(), [saved]);
    // The first move of the game is a legal anchor too.
    block_on(s.save_variation(gid(1), 0, "1. d4".to_owned())).unwrap();
}

#[test]
fn a_game_without_moves_has_no_anchor() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 0);
    assert!(matches!(
        block_on(s.save_variation(gid(1), 0, "1. e4".to_owned())),
        Err(StoreError::VariationAnchor)
    ));
}

#[test]
fn the_variation_table_has_no_player_column() {
    let dir = TempDir::new().unwrap();
    let _s = world(dir.path());
    let conn = raw(dir.path());
    let mut st = conn
        .prepare("SELECT name FROM pragma_table_info('variation')")
        .unwrap();
    let columns: Vec<String> = st
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(columns, ["id", "game_id", "from_ply", "pgn", "created_at"]);
}

#[test]
fn one_line_per_row_nested_text_is_stored_verbatim_and_listed_by_anchor() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 100);
    game_with(dir.path(), 1, 10);
    let nested = "5. Nf3 (5. d4 d5 (5... Nf6)) 5... Nc6";
    now.store(200, Ordering::SeqCst);
    let b = block_on(s.save_variation(gid(1), 6, nested.to_owned())).unwrap();
    now.store(300, Ordering::SeqCst);
    let a = block_on(s.save_variation(gid(1), 2, "2. Nc3".to_owned())).unwrap();
    now.store(400, Ordering::SeqCst);
    let c = block_on(s.save_variation(gid(1), 6, "5. g3".to_owned())).unwrap();
    let listed = block_on(s.list_variations(gid(1))).unwrap();
    assert_eq!(
        listed,
        [a, b.clone(), c],
        "from_ply, then creation order; one row each"
    );
    assert_eq!(
        listed[1].pgn, nested,
        "no interpretation of nesting (OQ-ST-003 is open)"
    );
    assert!(matches!(
        block_on(s.list_variations(gid(9))),
        Err(StoreError::NotFound {
            entity: Entity::Game,
            ..
        })
    ));
}

fn variation_rows(dir: &std::path::Path) -> i64 {
    raw(dir)
        .query_row("SELECT count(*) FROM variation", [], |r| r.get(0))
        .unwrap()
}

/// D5-02 B (Brian, 2026-11-22) VAR-1: a variation is identified by `(game, from_ply, pgn)` and
/// `save_variation` is idempotent on that key: the second call returns the stored record with its
/// original `created_at`, even with the clock advanced, and writes nothing.
#[test]
fn var_1_saving_a_key_twice_returns_the_stored_record_and_adds_no_row() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 100);
    game_with(dir.path(), 1, 5);
    let first = block_on(s.save_variation(gid(1), 2, "2. Nc3".to_owned())).unwrap();
    assert_eq!(first.created_at, WallMs(100));
    now.store(999, Ordering::SeqCst);
    let again = block_on(s.save_variation(gid(1), 2, "2. Nc3".to_owned())).unwrap();
    assert_eq!(again, first, "same record, original created_at");
    assert_eq!(block_on(s.list_variations(gid(1))).unwrap(), [first]);
    assert_eq!(variation_rows(dir.path()), 1);
}

/// VAR-2, VAR-3: the key is the triple; text equality is exact.
#[test]
fn var_2_and_3_each_distinct_key_is_its_own_row() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 5);
    game_with(dir.path(), 2, 5);
    for (game, ply, text) in [
        (1, 1, "1... e5"),
        (1, 2, "1... e5"),  // same text, other from_ply
        (1, 1, "1... c5"),  // other text, same from_ply
        (2, 1, "1... e5"),  // same text, other game
        (1, 1, "1... e5 "), // one trailing space is a different line
        (1, 1, "1... E5"),  // case is not folded
    ] {
        block_on(s.save_variation(gid(game), ply, text.to_owned())).unwrap();
    }
    assert_eq!(variation_rows(dir.path()), 6);
    assert_eq!(block_on(s.list_variations(gid(1))).unwrap().len(), 5);
    assert_eq!(block_on(s.list_variations(gid(2))).unwrap().len(), 1);
}

/// VAR-5: the existing-row check comes before the anchor check, so a repeat of a stored key never
/// fails, even when the game's stored plies have shrunk below the anchor.
#[test]
fn var_5_a_repeat_of_a_stored_key_is_returned_before_the_anchor_check() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 4);
    let saved = block_on(s.save_variation(gid(1), 3, "3... Nf6".to_owned())).unwrap();
    raw(dir.path())
        .execute("DELETE FROM move WHERE ply >= 2", [])
        .unwrap();
    assert_eq!(
        block_on(s.save_variation(gid(1), 3, "3... Nf6".to_owned())).unwrap(),
        saved
    );
    assert!(matches!(
        block_on(s.save_variation(gid(1), 3, "3... c5".to_owned())),
        Err(StoreError::VariationAnchor)
    ));
}

/// VAR-6: an unknown game is `NotFound` for save, delete and list.
#[test]
fn var_6_an_unknown_game_is_not_found() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let not_found = |r: Result<(), StoreError>| {
        matches!(
            r,
            Err(StoreError::NotFound {
                entity: Entity::Game,
                ..
            })
        )
    };
    assert!(not_found(
        block_on(s.save_variation(gid(9), 0, "1. e4".to_owned())).map(drop)
    ));
    assert!(not_found(
        block_on(s.delete_variation(gid(9), 0, "1. e4".to_owned())).map(drop)
    ));
    assert!(not_found(block_on(s.list_variations(gid(9))).map(drop)));
}

/// VAR-7, VAR-8: delete by the key; the result says whether a row went; one changed character is
/// a different key and leaves the row.
#[test]
fn var_7_and_8_delete_variation_removes_the_keyed_row_and_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 5);
    game_with(dir.path(), 2, 5);
    let keep = block_on(s.save_variation(gid(1), 1, "1... e5".to_owned())).unwrap();
    block_on(s.save_variation(gid(1), 1, "1... c5".to_owned())).unwrap();
    let other_game = block_on(s.save_variation(gid(2), 1, "1... c5".to_owned())).unwrap();
    assert!(!block_on(s.delete_variation(gid(1), 1, "1... c6".to_owned())).unwrap());
    assert!(!block_on(s.delete_variation(gid(1), 2, "1... c5".to_owned())).unwrap());
    assert_eq!(variation_rows(dir.path()), 3, "a near miss deletes nothing");
    assert!(block_on(s.delete_variation(gid(1), 1, "1... c5".to_owned())).unwrap());
    assert!(
        !block_on(s.delete_variation(gid(1), 1, "1... c5".to_owned())).unwrap(),
        "already gone"
    );
    assert_eq!(block_on(s.list_variations(gid(1))).unwrap(), [keep]);
    assert_eq!(block_on(s.list_variations(gid(2))).unwrap(), [other_game]);
}

/// VAR-9: an empty text is refused (`InvariantViolated`, the crate's error for a value no caller
/// may send) and nothing is stored.
#[test]
fn var_9_an_empty_text_is_refused() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 5);
    assert!(matches!(
        block_on(s.save_variation(gid(1), 1, String::new())),
        Err(StoreError::InvariantViolated)
    ));
    assert_eq!(variation_rows(dir.path()), 0);
}

/// VAR-10: two `Store` handles on one directory (two writers racing for the same key) give one
/// row per key and the same `created_at` from every call, over 200 rounds.
#[test]
fn var_10_concurrent_saves_of_one_key_give_one_row() {
    let dir = TempDir::new().unwrap();
    let (a, now) = world_with_clock(dir.path(), 1);
    game_with(dir.path(), 1, 5);
    let b = d5_support::reopen_with_clock(dir.path(), &now);
    for round in 0..200u32 {
        now.store(10 + i64::from(round), Ordering::SeqCst);
        let text = format!("1... line{round}");
        let (x, y) = std::thread::scope(|scope| {
            let (a, b) = (&a, &b);
            let (t1, t2) = (text.clone(), text.clone());
            let h1 = scope.spawn(move || block_on(a.save_variation(gid(1), 1, t1)));
            let h2 = scope.spawn(move || block_on(b.save_variation(gid(1), 1, t2)));
            (h1.join().unwrap(), h2.join().unwrap())
        });
        assert_eq!(x.unwrap(), y.unwrap(), "round {round}");
    }
    assert_eq!(variation_rows(dir.path()), 200);
}

/// FIND-D5R-01: the cap is 1 MiB; a text of exactly the cap is stored, one byte more is
/// `VariationTooLarge`, nothing is written for the refused call, and the size is counted in bytes
/// (a multi-byte character counts as its UTF-8 length).
#[test]
fn d5r_01_a_variation_text_above_one_mib_is_refused_and_the_boundary_is_exact() {
    assert_eq!(MAX_VARIATION_PGN_BYTES, 1024 * 1024);
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    game_with(dir.path(), 1, 4);
    let at_cap = "a".repeat(MAX_VARIATION_PGN_BYTES);
    let saved = block_on(s.save_variation(gid(1), 1, at_cap.clone())).unwrap();
    assert_eq!(saved.pgn.len(), MAX_VARIATION_PGN_BYTES);
    // A repeat of the stored key is idempotent, as before.
    assert_eq!(
        block_on(s.save_variation(gid(1), 1, at_cap)).unwrap(),
        saved
    );
    for over in [
        "a".repeat(MAX_VARIATION_PGN_BYTES + 1),
        "a".repeat(MAX_VARIATION_PGN_BYTES * 4),
        // 524 289 two-byte characters: 524 289 chars, 1 048 578 bytes.
        "\u{e9}".repeat(MAX_VARIATION_PGN_BYTES / 2 + 1),
    ] {
        let refused = block_on(s.save_variation(gid(1), 2, over));
        assert!(
            matches!(
                refused,
                Err(StoreError::VariationTooLarge {
                    max_bytes: MAX_VARIATION_PGN_BYTES
                })
            ),
            "{refused:?}"
        );
    }
    // Exactly the cap in bytes with two-byte characters is stored.
    let two_byte_at_cap = "\u{e9}".repeat(MAX_VARIATION_PGN_BYTES / 2);
    assert_eq!(two_byte_at_cap.len(), MAX_VARIATION_PGN_BYTES);
    block_on(s.save_variation(gid(1), 2, two_byte_at_cap)).unwrap();
    assert_eq!(
        block_on(s.list_variations(gid(1))).unwrap().len(),
        2,
        "the refused calls stored nothing"
    );
    // The error text names a category only.
    let shown = StoreError::VariationTooLarge { max_bytes: 5 }.to_string();
    assert_eq!(shown, "the variation text is too large");
    // An empty text is still `InvariantViolated`, and an unknown game with a huge text is the
    // size error (the cap is checked before the game is read).
    assert!(matches!(
        block_on(s.save_variation(gid(1), 1, String::new())),
        Err(StoreError::InvariantViolated)
    ));
    assert!(matches!(
        block_on(s.save_variation(gid(99), 1, "a".repeat(MAX_VARIATION_PGN_BYTES + 1))),
        Err(StoreError::VariationTooLarge { .. })
    ));
}
