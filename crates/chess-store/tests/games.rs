//! DD-ST-009 `GameRepo`, part A, through the public `Store` API on a real temporary database:
//! start (slots, idempotency, `started_at`), read, list by structural keys, moves, unfinished
//! list, `analysis_used` (UT-ST-092, UT-ST-060 part, CT-ST-080 store half).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use chess_domain::{GameResult, GameStatus, Matchup, Mode, PlayerId, WallMs};
use chess_store::{Entity, GameFilter, Store, StoreError};
use common::{blitz, block_on, computer, gid, human, new_game, open_in, pid, raw};
use tempfile::TempDir;

/// Players: humans 1, 2, 3; computers 10, 11.
fn world() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    for (n, name) in [(1, "Ann"), (2, "Bob"), (3, "Cy")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![
        computer(10, "Bot A", 1, 0, 3),
        computer(11, "Bot B", 1, 1, 4),
    ]))
    .unwrap();
    (dir, s)
}

fn slots(d: &TempDir) -> Vec<(String, String)> {
    let conn = raw(d.path());
    let mut st = conn
        .prepare("SELECT p.uuid, s.mode FROM unfinished_slot s JOIN player p ON p.id = s.player_id ORDER BY p.uuid, s.mode")
        .unwrap();
    st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn ut_st_092_start_writes_started_at_unchanged_and_the_row_reads_back() {
    let (_d, s) = world();
    let mut new = new_game(1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    new.clock = Some(blitz());
    new.started_at = WallMs(123_456_789);
    let game = block_on(s.start_game(new)).unwrap();
    assert_eq!(game.id, gid(1));
    assert_eq!(
        game.started_at,
        Some(WallMs(123_456_789)),
        "the store reads no clock for it"
    );
    assert_eq!((game.white, game.black), (Some(pid(1)), Some(pid(10))));
    assert_eq!(
        (game.matchup, game.mode),
        (Some(Matchup::HumanComputer), Some(Mode::Strict))
    );
    assert!(game.rated && !game.self_play && !game.custom_start);
    assert_eq!(game.start_fen, None);
    assert_eq!(game.clock, Some(blitz()));
    assert_eq!(
        (game.white_clock_ms, game.black_clock_ms),
        (Some(300_000), Some(300_000))
    );
    assert_eq!(game.status, GameStatus::InProgress);
    assert_eq!(
        (game.result, game.content_hash.as_deref(), game.ended_at),
        (None, None, None)
    );
    assert_eq!(game.journal_seq, 0);
    assert!(!game.analysis_used);
    assert_eq!(block_on(s.get_game(gid(1))).unwrap(), game);
}

#[test]
fn ut_st_092_a_second_start_with_the_same_id_returns_the_existing_game_unchanged() {
    let (d, s) = world();
    let mut first = new_game(1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    first.started_at = WallMs(111);
    let original = block_on(s.start_game(first)).unwrap();
    let mut again = new_game(1, pid(2), pid(11), Matchup::HumanComputer, Mode::Training);
    again.started_at = WallMs(999);
    let repeat = block_on(s.start_game(again)).unwrap();
    assert_eq!(repeat, original);
    assert_eq!(repeat.started_at, Some(WallMs(111)));
    assert_eq!(slots(&d).len(), 1, "no second slot row");
}

#[test]
fn training_game_is_untimed_and_has_null_clocks() {
    let (_d, s) = world();
    let game = block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    )))
    .unwrap();
    assert_eq!(
        (game.clock, game.white_clock_ms, game.black_clock_ms),
        (None, None, None)
    );
}

#[test]
fn custom_start_is_derived_from_start_fen_and_is_never_rated() {
    let (_d, s) = world();
    let fen = "8/8/8/8/8/8/8/K6k w - - 0 1".to_owned();
    let mut new = new_game(1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    new.start_fen = Some(fen.clone());
    new.rated = true;
    let err = block_on(s.start_game(new.clone())).unwrap_err();
    assert!(
        matches!(err, StoreError::InvariantViolated),
        "custom start with rated=1: {err:?}"
    );
    new.rated = false;
    let game = block_on(s.start_game(new)).unwrap();
    assert!(game.custom_start);
    assert_eq!(game.start_fen, Some(fen));
}

#[test]
fn slots_hh_hc_and_cc_follow_architecture_7_4() {
    let (d, s) = world();
    let uuid = |n: u128| format!("{}", pid(n));
    // h-h: both humans hold a slot.
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
    assert_eq!(
        slots(&d),
        [(uuid(1), "strict".into()), (uuid(2), "strict".into())]
    );
    // h-c: the human only, so the computer can be played by anyone, also while busy elsewhere.
    block_on(s.start_game(new_game(
        2,
        pid(3),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    assert_eq!(slots(&d).len(), 3);
    assert!(!slots(&d).iter().any(|(p, _)| *p == uuid(10)));
    // c-c: each computer holds one.
    block_on(s.start_game(new_game(
        3,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap();
    assert_eq!(slots(&d).len(), 5);
}

#[test]
fn self_play_has_one_slot_and_one_id_twice() {
    let (d, s) = world();
    let game = block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(1),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
    assert!(game.self_play);
    assert_eq!((game.white, game.black), (Some(pid(1)), Some(pid(1))));
    assert_eq!(slots(&d).len(), 1);
    let cc = block_on(s.start_game(new_game(
        2,
        pid(10),
        pid(10),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap();
    assert!(cc.self_play);
    assert_eq!(slots(&d).len(), 2);
}

#[test]
fn a_held_slot_is_slot_taken_naming_the_holder_and_nothing_is_inserted() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    let err = block_on(s.start_game(new_game(
        2,
        pid(1),
        pid(11),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap_err();
    match err {
        StoreError::SlotTaken {
            game_id,
            mode,
            player_id,
        } => {
            assert_eq!((game_id, mode, player_id), (gid(1), Mode::Strict, pid(1)));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        block_on(s.get_game(gid(2))).unwrap_err(),
        StoreError::NotFound {
            entity: Entity::Game,
            ..
        }
    ));
    assert_eq!(slots(&d).len(), 1);
    // The other mode is free (the limit is one unfinished game per mode, GAME-4).
    block_on(s.start_game(new_game(
        3,
        pid(1),
        pid(11),
        Matchup::HumanComputer,
        Mode::Training,
    )))
    .unwrap();
    assert_eq!(slots(&d).len(), 2);
}

#[test]
fn a_half_claimed_slot_set_rolls_back_ut_st_043() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(2),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    // Ann is free, Bob is busy: the h-h game must claim neither.
    let err = block_on(s.start_game(new_game(
        2,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap_err();
    assert!(
        matches!(err, StoreError::SlotTaken { player_id, .. } if player_id == pid(2)),
        "{err:?}"
    );
    assert_eq!(slots(&d).len(), 1);
}

#[test]
fn computers_slot_in_cc_blocks_a_second_cc_but_not_a_human_game_ct_st_028_part() {
    let (_d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap();
    let err = block_on(s.start_game(new_game(
        2,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap_err();
    assert!(matches!(err, StoreError::SlotTaken { .. }), "{err:?}");
    block_on(s.start_game(new_game(
        3,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
}

#[test]
fn unknown_players_and_mismatched_matchups_are_refused() {
    let (d, s) = world();
    let err = block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(99),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap_err();
    assert!(
        matches!(err, StoreError::NotFound { entity: Entity::Player, id } if id == pid(99).as_uuid()),
        "{err:?}"
    );
    for (white, black, matchup) in [
        (pid(1), pid(2), Matchup::HumanComputer),
        (pid(1), pid(10), Matchup::HumanHuman),
        (pid(1), pid(10), Matchup::ComputerComputer),
        (pid(10), pid(11), Matchup::HumanHuman),
    ] {
        let err =
            block_on(s.start_game(new_game(2, white, black, matchup, Mode::Strict))).unwrap_err();
        assert!(
            matches!(err, StoreError::InvariantViolated),
            "{matchup:?}: {err:?}"
        );
    }
    assert!(slots(&d).is_empty());
}

#[test]
fn training_outside_human_against_computer_is_refused_by_the_schema() {
    let (_d, s) = world();
    let err = block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Training,
    )))
    .unwrap_err();
    assert!(matches!(err, StoreError::InvariantViolated), "{err:?}");
    let mut timed = new_game(2, pid(1), pid(10), Matchup::HumanComputer, Mode::Training);
    timed.clock = Some(blitz());
    let err = block_on(s.start_game(timed)).unwrap_err();
    assert!(matches!(err, StoreError::InvariantViolated), "{err:?}");
}

#[test]
fn ct_st_080_store_half_a_game_row_without_moves_reads_as_zero_moves() {
    let (_d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    assert!(block_on(s.game_moves(gid(1))).unwrap().is_empty());
    let unfinished = block_on(s.list_unfinished(pid(1))).unwrap();
    assert_eq!(unfinished.len(), 1);
    assert_eq!(unfinished[0].id, gid(1));
}

#[test]
fn game_moves_returns_rows_in_ply_order_with_optional_clocks() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    )))
    .unwrap();
    let conn = raw(d.path());
    conn.execute_batch(
        "INSERT INTO move (game_id, ply, san, uci, fen_after, think_ms, white_clock_ms, black_clock_ms, played_at) \
           VALUES (1, 2, 'e5', 'e7e5', 'f2', 40, 5, 6, 77); \
         INSERT INTO move (game_id, ply, san, uci, fen_after) VALUES (1, 1, 'e4', 'e2e4', 'f1');",
    )
    .unwrap();
    let moves = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(moves.iter().map(|m| m.ply).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(moves[0].san, "e4");
    assert_eq!(moves[0].clocks_ms, None);
    assert_eq!((moves[0].think_ms, moves[0].played_at), (None, None));
    assert_eq!(moves[1].clocks_ms, Some((5, 6)));
    assert_eq!(
        (moves[1].think_ms, moves[1].played_at),
        (Some(40), Some(WallMs(77)))
    );
    assert_eq!(moves[1].uci, "e7e5");
    assert_eq!(moves[1].fen_after, "f2");
    assert!(matches!(
        block_on(s.game_moves(gid(9))).unwrap_err(),
        StoreError::NotFound { .. }
    ));
}

#[test]
fn list_games_applies_structural_keys_only_ut_st_060_part() {
    let (d, s) = world();
    let mut a = new_game(1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    a.started_at = WallMs(100);
    let mut b = new_game(2, pid(2), pid(10), Matchup::HumanComputer, Mode::Strict);
    b.started_at = WallMs(200);
    let mut c = new_game(3, pid(3), pid(3), Matchup::HumanHuman, Mode::Strict);
    c.started_at = WallMs(300);
    for g in [a, b, c] {
        block_on(s.start_game(g)).unwrap();
    }
    // Complete game 1 through SQL (finalize is not part A) so that a status filter has two values.
    raw(d.path())
        .execute_batch("UPDATE game SET status='completed', result='1-0', content_hash='sha256:v1:x' WHERE id=1; DELETE FROM unfinished_slot WHERE game_id=1;")
        .unwrap();
    let ids = |f: GameFilter| -> Vec<_> {
        block_on(s.list_games(f))
            .unwrap()
            .into_iter()
            .map(|g| g.id)
            .collect()
    };
    assert_eq!(ids(GameFilter::default()), [gid(1), gid(2), gid(3)]);
    assert_eq!(
        ids(GameFilter {
            player: Some(pid(3)),
            ..GameFilter::default()
        }),
        [gid(3)]
    );
    assert_eq!(
        ids(GameFilter {
            player: Some(pid(1)),
            ..GameFilter::default()
        }),
        [gid(1)]
    );
    assert_eq!(
        ids(GameFilter {
            player: Some(pid(10)),
            ..GameFilter::default()
        }),
        [gid(1), gid(2)]
    );
    assert_eq!(
        ids(GameFilter {
            status: Some(GameStatus::Completed),
            ..GameFilter::default()
        }),
        [gid(1)]
    );
    assert_eq!(
        ids(GameFilter {
            from: Some(WallMs(200)),
            ..GameFilter::default()
        }),
        [gid(2), gid(3)]
    );
    assert_eq!(
        ids(GameFilter {
            limit: Some(2),
            ..GameFilter::default()
        }),
        [gid(1), gid(2)]
    );
    assert_eq!(
        ids(GameFilter {
            limit: Some(0),
            ..GameFilter::default()
        }),
        Vec::<chess_domain::GameId>::new()
    );
    // Self-play and the completed game are returned: no predicate beyond the keys (D-44).
    let all = block_on(s.list_games(GameFilter::default())).unwrap();
    assert!(all.iter().any(|g| g.self_play));
    let done = all.iter().find(|g| g.id == gid(1)).unwrap();
    assert_eq!(done.result, Some(GameResult::WhiteWins));
    assert_eq!(done.content_hash.as_deref(), Some("sha256:v1:x"));
    let unknown = block_on(s.list_games(GameFilter {
        player: Some(PlayerId::from_uuid(uuid::Uuid::nil())),
        ..GameFilter::default()
    }));
    assert!(matches!(
        unknown,
        Err(StoreError::NotFound {
            entity: Entity::Player,
            ..
        })
    ));
}

#[test]
fn list_unfinished_reads_games_not_slots() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    // The slot row is gone, the game is still in progress: the resume list still shows it.
    raw(d.path())
        .execute_batch("DELETE FROM unfinished_slot;")
        .unwrap();
    assert_eq!(block_on(s.list_unfinished(pid(1))).unwrap().len(), 1);
    assert!(block_on(s.list_unfinished(pid(2))).unwrap().is_empty());
}

#[test]
fn set_analysis_used_is_idempotent() {
    let (_d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    )))
    .unwrap();
    block_on(s.set_analysis_used(gid(1))).unwrap();
    block_on(s.set_analysis_used(gid(1))).unwrap();
    assert!(block_on(s.get_game(gid(1))).unwrap().analysis_used);
    assert!(matches!(
        block_on(s.set_analysis_used(gid(9))).unwrap_err(),
        StoreError::NotFound { .. }
    ));
}

#[test]
fn a_voided_by_player_is_returned_as_a_player_id() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    raw(d.path())
        .execute_batch("UPDATE game SET status='voided', voided_by=1, content_hash='sha256:v1:y', ended_at=9, termination='voided by reset' WHERE id=1;")
        .unwrap();
    let game = block_on(s.get_game(gid(1))).unwrap();
    assert_eq!(game.voided_by, Some(pid(1)));
    assert_eq!(game.ended_at, Some(WallMs(9)));
    assert_eq!(game.termination.as_deref(), Some("voided by reset"));
    assert_eq!(game.status, GameStatus::Voided);
}

#[test]
fn stored_text_that_no_enum_accepts_is_corrupt_not_a_panic() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    raw(d.path())
        .execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE player SET start_category='Wizard' WHERE id=1;")
        .unwrap();
    assert!(matches!(
        block_on(s.get_player(pid(1))).unwrap_err(),
        StoreError::Corrupt
    ));
    raw(d.path())
        .execute_batch(
            "PRAGMA ignore_check_constraints=ON; UPDATE game SET clock_spec='bogus' WHERE id=1;",
        )
        .unwrap();
    assert!(matches!(
        block_on(s.get_game(gid(1))).unwrap_err(),
        StoreError::Corrupt
    ));
}

#[test]
fn rating_history_returns_game_linked_points_with_their_game_id() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    raw(d.path())
        .execute_batch(
            "INSERT INTO rating_history (player_id, track, game_id, rating_before, rating_after, k, at) \
             SELECT p.id, 'strict', g.id, 1200, 1216, 40, 99 FROM player p, game g WHERE p.name = 'Ann' AND g.id = 1;",
        )
        .unwrap();
    let history = block_on(s.rating_history(pid(1), chess_domain::Track::Strict)).unwrap();
    assert_eq!(
        history.len(),
        2,
        "the initial point and the game point, in append order"
    );
    assert_eq!(history[0].game_id, None);
    assert_eq!(history[1].game_id, Some(gid(1)));
    assert_eq!(
        (
            history[1].rating_before,
            history[1].rating_after,
            history[1].k
        ),
        (1200, 1216, 40)
    );
    assert_eq!(history[1].at, WallMs(99));
    assert!(history[1].id > history[0].id);
    assert!(matches!(
        block_on(s.rating_history(pid(77), chess_domain::Track::Strict)),
        Err(StoreError::NotFound {
            entity: Entity::Player,
            ..
        })
    ));
}
