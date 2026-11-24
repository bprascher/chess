//! DD-ST-007 `Schema`: every CHECK and UNIQUE of migrations/0001_init.sql rejects the bad row,
//! and the good row is accepted (UT-ST-030 to UT-ST-036, UT-ST-084). Run on the real database
//! file that `Store::open` creates, through a second connection, so the constraints tested are
//! the ones a store really has.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use common::{open_in, raw};
use rusqlite::{Connection, ErrorCode};
use tempfile::TempDir;

const H: &str = "00000000-0000-0000-0000-0000000000a1";
const C: &str = "00000000-0000-0000-0000-0000000000a2";

fn fresh() -> (TempDir, Connection) {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    let conn = raw(dir.path());
    conn.execute_batch(&format!(
        "INSERT INTO player (id, uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
           VALUES (1, '{H}', 'human', 'Hana', 'Novice', 1200, 1200, 1), \
                  (2, '{C}', 'computer', 'Comp', 'Beginner', 800, 800, 1);"
    ))
    .unwrap();
    (dir, conn)
}

/// The statement is rejected by a constraint (not by a typo).
fn rejected(conn: &Connection, sql: &str) {
    match conn.execute_batch(sql) {
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {}
        other => panic!("expected a constraint violation for {sql}, got {other:?}"),
    }
}

fn accepted(conn: &Connection, sql: &str) {
    conn.execute_batch(sql)
        .unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// An INSERT into `game` from defaults (an in-progress Strict h-c game) and overrides.
fn game_sql(overrides: &[(&str, &str)]) -> String {
    let mut cols: Vec<(&str, &str)> = vec![
        ("uuid", "'10000000-0000-0000-0000-000000000001'"),
        ("white_id", "1"),
        ("black_id", "2"),
        ("matchup", "'hc'"),
        ("mode", "'strict'"),
        ("rated", "1"),
        ("self_play", "0"),
        ("status", "'in_progress'"),
    ];
    for (name, value) in overrides {
        match cols.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = value,
            None => cols.push((name, value)),
        }
    }
    let names: Vec<&str> = cols.iter().map(|c| c.0).collect();
    let values: Vec<&str> = cols.iter().map(|c| c.1).collect();
    format!(
        "INSERT INTO game ({}) VALUES ({});",
        names.join(","),
        values.join(",")
    )
}

const HASH: &str = "'sha256:v1:0000000000000000000000000000000000000000000000000000000000000000'";

#[test]
fn ut_st_030_result_must_match_status() {
    let (_d, c) = fresh();
    rejected(
        &c,
        &game_sql(&[("status", "'completed'"), ("content_hash", HASH)]),
    ); // no result
    rejected(&c, &game_sql(&[("result", "'1-0'")])); // in_progress with a result
    rejected(
        &c,
        &game_sql(&[
            ("status", "'aborted'"),
            ("result", "'1-0'"),
            ("content_hash", HASH),
        ]),
    );
    rejected(
        &c,
        &game_sql(&[
            ("status", "'completed'"),
            ("result", "'2-0'"),
            ("content_hash", HASH),
        ]),
    );
    accepted(
        &c,
        &game_sql(&[
            ("status", "'completed'"),
            ("result", "'1/2-1/2'"),
            ("content_hash", HASH),
        ]),
    );
}

#[test]
fn ut_st_030_training_must_be_untimed_human_against_computer() {
    let (_d, c) = fresh();
    rejected(
        &c,
        &game_sql(&[("mode", "'training'"), ("matchup", "'hh'")]),
    );
    rejected(
        &c,
        &game_sql(&[("mode", "'training'"), ("matchup", "'cc'")]),
    );
    rejected(
        &c,
        &game_sql(&[
            ("mode", "'training'"),
            ("clock_spec", "'300+0'"),
            ("white_clock_ms", "300000"),
            ("black_clock_ms", "300000"),
        ]),
    );
    accepted(&c, &game_sql(&[("mode", "'training'")]));
}

fn profile(player: i64, index: i64, uci: &str, permille: i64, multipv: i64) -> String {
    format!(
        "INSERT INTO computer_profile (player_id, roster_seed, roster_index, skill_level, depth_limit, \
         movetime_ms, uci_elo, error_permille, error_multipv) VALUES ({player}, 1, {index}, 5, 8, 500, {uci}, {permille}, {multipv});"
    )
}

fn extra_computer(c: &Connection, id: i64) {
    accepted(
        c,
        &format!(
            "INSERT INTO player (id, uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
             VALUES ({id}, '00000000-0000-0000-0000-0000000001{id:02}', 'computer', 'C{id}', 'Beginner', 800, 800, 1);"
        ),
    );
}

#[test]
fn ut_st_031_parameter_set_is_unique_with_null_uci_elo_comparable() {
    let (_d, c) = fresh();
    accepted(&c, &profile(2, 0, "1500", 20, 2));
    extra_computer(&c, 3);
    rejected(&c, &profile(3, 1, "1500", 20, 2)); // same parameters, other index
    accepted(&c, &profile(3, 1, "1501", 20, 2)); // uci_elo differs
    extra_computer(&c, 4);
    accepted(&c, &profile(4, 2, "NULL", 20, 2));
    extra_computer(&c, 5);
    rejected(&c, &profile(5, 3, "NULL", 20, 2)); // NULL uci_elo duplicates are caught (COALESCE)
}

#[test]
fn ut_st_031_roster_seed_and_index_pair_is_unique() {
    let (_d, c) = fresh();
    accepted(&c, &profile(2, 7, "1500", 20, 2));
    extra_computer(&c, 3);
    rejected(&c, &profile(3, 7, "1600", 20, 2));
}

#[test]
fn ut_st_032_names_are_case_sensitive_and_nonempty() {
    let (_d, c) = fresh();
    let player = |id: i64, name: &str| {
        format!(
            "INSERT INTO player (id, uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
             VALUES ({id}, '00000000-0000-0000-0000-00000000020{id}', 'human', '{name}', 'Novice', 1, 1, 1);"
        )
    };
    accepted(&c, &player(3, "Ann"));
    accepted(&c, &player(4, "ann")); // case kept: a different name
    rejected(&c, &player(5, "Ann"));
    rejected(&c, &player(6, ""));
}

#[test]
fn ut_st_033_slot_key_is_player_and_mode() {
    let (_d, c) = fresh();
    accepted(&c, &game_sql(&[]));
    accepted(&c, "INSERT INTO unfinished_slot VALUES (1, 'strict', 1);");
    rejected(&c, "INSERT INTO unfinished_slot VALUES (1, 'strict', 1);");
    accepted(&c, "INSERT INTO unfinished_slot VALUES (1, 'training', 1);"); // other mode
    rejected(&c, "INSERT INTO unfinished_slot VALUES (1, 'blitz', 1);");
}

#[test]
fn ut_st_034_uuid_is_lower_case_and_36_long() {
    let (_d, c) = fresh();
    let with = |u: &str| {
        format!(
            "INSERT INTO player (uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
             VALUES ('{u}', 'human', 'X{n}', 'Novice', 1, 1, 1);",
            n = u.len()
        )
    };
    rejected(&c, &with("ABCDEF00-0000-0000-0000-000000000001"));
    rejected(&c, &with("00000000-0000-0000-0000-00000000001"));
    accepted(&c, &with("abcdef00-0000-0000-0000-000000000001"));
}

#[test]
fn ut_st_035_content_hash_is_null_exactly_while_in_progress() {
    let (_d, c) = fresh();
    rejected(&c, &game_sql(&[("content_hash", HASH)])); // in_progress with a hash
    rejected(
        &c,
        &game_sql(&[("status", "'completed'"), ("result", "'1-0'")]),
    );
    rejected(&c, &game_sql(&[("status", "'aborted'")]));
    rejected(&c, &game_sql(&[("status", "'voided'")]));
    rejected(
        &c,
        &game_sql(&[
            ("status", "'imported'"),
            ("matchup", "NULL"),
            ("mode", "NULL"),
            ("rated", "0"),
        ]),
    );
    accepted(
        &c,
        &game_sql(&[("status", "'aborted'"), ("content_hash", HASH)]),
    );
    accepted(
        &c,
        &game_sql(&[
            ("uuid", "'10000000-0000-0000-0000-000000000002'"),
            ("status", "'voided'"),
            ("content_hash", HASH),
        ]),
    );
    accepted(
        &c,
        &game_sql(&[
            ("uuid", "'10000000-0000-0000-0000-000000000003'"),
            ("status", "'imported'"),
            ("matchup", "NULL"),
            ("mode", "NULL"),
            ("rated", "0"),
            ("white_id", "NULL"),
            ("black_id", "NULL"),
            ("content_hash", HASH),
        ]),
    );
}

#[test]
fn ut_st_036_error_permille_range_and_distinct_by_one() {
    let (_d, c) = fresh();
    rejected(&c, &profile(2, 0, "1500", 1001, 2));
    rejected(&c, &profile(2, 0, "1500", -1, 2));
    accepted(&c, &profile(2, 0, "1500", 1000, 2));
    extra_computer(&c, 3);
    accepted(&c, &profile(3, 1, "1500", 999, 2)); // one permille apart: distinct
}

#[test]
fn ut_st_084_new_checks_reject_the_bad_row() {
    let (_d, c) = fresh();
    // custom_start must equal (start_fen IS NOT NULL)
    rejected(&c, &game_sql(&[("custom_start", "1")]));
    rejected(
        &c,
        &game_sql(&[("start_fen", "'8/8/8/8/8/8/8/K6k w - - 0 1'")]),
    );
    // custom_start = 1 with rated = 1
    rejected(
        &c,
        &game_sql(&[
            ("custom_start", "1"),
            ("start_fen", "'8/8/8/8/8/8/8/K6k w - - 0 1'"),
        ]),
    );
    accepted(
        &c,
        &game_sql(&[
            ("custom_start", "1"),
            ("start_fen", "'8/8/8/8/8/8/8/K6k w - - 0 1'"),
            ("rated", "0"),
        ]),
    );
    // one NULL clock of a pair, in GAME and in MOVE
    rejected(
        &c,
        &game_sql(&[
            ("uuid", "'10000000-0000-0000-0000-000000000009'"),
            ("white_clock_ms", "1000"),
        ]),
    );
    accepted(
        &c,
        &game_sql(&[
            ("uuid", "'10000000-0000-0000-0000-000000000008'"),
            ("white_clock_ms", "1000"),
            ("black_clock_ms", "1000"),
        ]),
    );
    let game_id: i64 = c
        .query_row("SELECT id FROM game ORDER BY id LIMIT 1", [], |r| r.get(0))
        .unwrap();
    rejected(
        &c,
        &format!(
            "INSERT INTO move (game_id, ply, san, uci, fen_after, white_clock_ms) VALUES ({game_id}, 1, 'e4', 'e2e4', 'f', 5);"
        ),
    );
    rejected(
        &c,
        &format!(
            "INSERT INTO move (game_id, ply, san, uci, fen_after) VALUES ({game_id}, 0, 'e4', 'e2e4', 'f');"
        ),
    );
    accepted(
        &c,
        &format!(
            "INSERT INTO move (game_id, ply, san, uci, fen_after, white_clock_ms, black_clock_ms) VALUES ({game_id}, 1, 'e4', 'e2e4', 'f', 5, 6);"
        ),
    );
    // error_permille > 0 needs error_multipv >= 2
    rejected(&c, &profile(2, 0, "1500", 10, 1));
    accepted(&c, &profile(2, 0, "1500", 0, 1));
    // a non-imported game with a NULL player id
    rejected(
        &c,
        &game_sql(&[
            ("uuid", "'10000000-0000-0000-0000-00000000000a'"),
            ("white_id", "NULL"),
        ]),
    );
    // self-play uses one id twice
    accepted(
        &c,
        &game_sql(&[
            ("uuid", "'10000000-0000-0000-0000-00000000000b'"),
            ("black_id", "1"),
            ("matchup", "'hh'"),
            ("self_play", "1"),
        ]),
    );
}

#[test]
fn preferences_json_must_be_valid_and_present() {
    let (_d, c) = fresh();
    rejected(&c, "INSERT INTO preferences VALUES (1, '{bad');");
    rejected(&c, "INSERT INTO preferences VALUES (1, NULL);"); // NULL passes CHECK, NOT NULL stops it
    accepted(&c, "INSERT INTO preferences VALUES (1, '{\"a\":1}');");
}

#[test]
fn other_constraints_reject_the_bad_row() {
    let (_d, c) = fresh();
    // computers have no reset_at
    rejected(&c, "UPDATE player SET reset_at = 5 WHERE id = 2;");
    accepted(&c, "UPDATE player SET reset_at = 5 WHERE id = 1;");
    // negative counters, bad bool
    rejected(
        &c,
        "UPDATE player SET strict_rated_games = -1 WHERE id = 1;",
    );
    rejected(&c, "UPDATE player SET archived = 2 WHERE id = 1;");
    // voided_by only for voided games
    rejected(&c, &game_sql(&[("voided_by", "1")]));
    // foreign keys are enforced on this connection
    rejected(&c, "INSERT INTO unfinished_slot VALUES (1, 'strict', 999);");
    // an imported game: no matchup or mode, never rated
    rejected(
        &c,
        &game_sql(&[
            ("status", "'imported'"),
            ("content_hash", HASH),
            ("white_id", "NULL"),
            ("black_id", "NULL"),
        ]),
    );
    // move_eval: cp and mate are exclusive; flag is one of three
    accepted(&c, &game_sql(&[]));
    accepted(
        &c,
        "INSERT INTO move (game_id, ply, san, uci, fen_after) VALUES (1, 1, 'e4', 'e2e4', 'f');",
    );
    rejected(
        &c,
        "INSERT INTO move_eval (game_id, ply, cp, mate, engine_version) VALUES (1, 1, 10, 2, 'sf');",
    );
    rejected(
        &c,
        "INSERT INTO move_eval (game_id, ply, flag, engine_version) VALUES (1, 1, 'awful', 'sf');",
    );
    rejected(
        &c,
        "INSERT INTO move_eval (game_id, ply, engine_version) VALUES (1, 9, 'sf');",
    ); // no such move
    accepted(
        &c,
        "INSERT INTO move_eval (game_id, ply, cp, flag, engine_version) VALUES (1, 1, 10, 'mistake', 'sf');",
    );
    // rating_history and variation checks
    rejected(
        &c,
        "INSERT INTO rating_history (player_id, track, rating_before, rating_after, k, at) VALUES (1, 'blitz', 1, 1, 1, 1);",
    );
    rejected(
        &c,
        "INSERT INTO variation (game_id, from_ply, pgn, created_at) VALUES (1, -1, 'x', 1);",
    );
    accepted(
        &c,
        "INSERT INTO variation (game_id, from_ply, pgn, created_at) VALUES (1, 0, 'x', 1);",
    );
}

#[test]
fn schema_has_no_roster_class_column_and_no_stats_views() {
    let (_d, c) = fresh();
    let cols: Vec<String> = c
        .prepare("SELECT name FROM pragma_table_info('computer_profile')")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!cols.iter().any(|n| n.contains("class")), "{cols:?}");
    let views: i64 = c
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'view'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(views, 0, "D-44: no SQL views");
}
