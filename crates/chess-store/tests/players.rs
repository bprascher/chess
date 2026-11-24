//! DD-ST-008 `PlayerRepo` and DD-ST-010 read side through the public `Store` API, on a real
//! temporary database: register, roster batch, `roster_indexes()`, rename, archive,
//! preferences, settings, rating rows, rollback of failed transactions (UT-ST-039,
//! UT-ST-040 to UT-ST-049).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use chess_domain::{Category, Matchup, Mode, PlayerKind, RosterGen, RosterSeed, Track, WallMs};
use chess_store::{Entity, Store, StoreError};
use common::{block_on, computer, human, new_game, open_in, personality, pid, raw};
use tempfile::TempDir;
use uuid::Uuid;

fn store() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let store = open_in(dir.path()).store;
    (dir, store)
}

#[test]
fn register_writes_player_preferences_and_one_rating_row_per_track() {
    let (_d, s) = store();
    let record = block_on(s.register_player(human(1, "Hana"))).unwrap();
    assert_eq!(record.id, pid(1));
    assert_eq!(record.kind, PlayerKind::Human);
    assert_eq!(record.name, "Hana");
    assert_eq!((record.strict_elo, record.training_elo), (1200, 1200));
    assert_eq!(
        (record.strict_rated_games, record.training_rated_games),
        (0, 0)
    );
    assert!(!record.archived);
    assert_eq!(record.reset_at, None);
    assert!(
        record.created_at.0 >= 1_000,
        "created_at comes from the injected clock"
    );
    assert!(record.computer.is_none());
    assert_eq!(
        block_on(s.get_preferences(pid(1))).unwrap().as_deref(),
        Some("{\"v\":1}")
    );
    for track in [Track::Strict, Track::Training] {
        let history = block_on(s.rating_history(pid(1), track)).unwrap();
        assert_eq!(history.len(), 1, "{track}");
        let point = &history[0];
        assert_eq!((point.rating_before, point.rating_after), (1200, 1200));
        assert_eq!(point.game_id, None, "an initial point has no game");
        assert_eq!(point.track, track);
        assert_eq!(point.at, record.created_at);
    }
}

#[test]
fn register_uses_the_injected_clock_not_the_system_clock() {
    let (_d, s) = store();
    let a = block_on(s.register_player(human(1, "A"))).unwrap();
    let b = block_on(s.register_player(human(2, "B"))).unwrap();
    assert!(
        b.created_at.0 > a.created_at.0,
        "ticking clock: {a:?} {b:?}"
    );
    assert_eq!(a.created_at, WallMs(1_000 + 1)); // open() read the clock once (1000)
}

#[test]
fn ut_st_040_duplicate_name_is_name_taken_and_leaves_nothing_behind() {
    let (d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    let err = block_on(s.register_player(human(2, "Hana"))).unwrap_err();
    assert!(matches!(err, StoreError::NameTaken), "{err:?}");
    let conn = raw(d.path());
    let players: i64 = conn
        .query_row("SELECT count(*) FROM player", [], |r| r.get(0))
        .unwrap();
    let history: i64 = conn
        .query_row("SELECT count(*) FROM rating_history", [], |r| r.get(0))
        .unwrap();
    assert_eq!((players, history), (1, 2));
    // A case variant is a different name (PLY-6).
    block_on(s.register_player(human(3, "hana"))).unwrap();
}

#[test]
fn ut_st_041_a_failed_register_rolls_back_every_row() {
    let (d, s) = store();
    let mut bad = human(1, "Hana");
    bad.preferences_json = "{not json".to_owned(); // the last table CHECK fails after PLAYER was inserted
    let err = block_on(s.register_player(bad)).unwrap_err();
    assert!(matches!(err, StoreError::InvariantViolated), "{err:?}");
    let conn = raw(d.path());
    for table in ["player", "preferences", "rating_history"] {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "{table} must be empty after rollback");
    }
}

#[test]
fn registering_the_same_id_twice_is_an_invariant_violation() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    let err = block_on(s.register_player(human(1, "Other"))).unwrap_err();
    assert!(matches!(err, StoreError::InvariantViolated), "{err:?}");
}

#[test]
fn get_and_find_by_name_use_uuid_and_exact_name() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().name, "Hana");
    let missing = block_on(s.get_player(pid(99))).unwrap_err();
    match missing {
        StoreError::NotFound { entity, id } => {
            assert_eq!(entity, Entity::Player);
            assert_eq!(id, Uuid::from_u128(99));
        }
        other => panic!("{other:?}"),
    }
    assert!(
        block_on(s.find_player_by_name("Hana".into()))
            .unwrap()
            .is_some()
    );
    assert!(
        block_on(s.find_player_by_name("hana".into()))
            .unwrap()
            .is_none()
    );
    assert!(
        block_on(s.find_player_by_name(String::new()))
            .unwrap()
            .is_none()
    );
}

#[test]
fn list_players_filters_by_kind_and_archived() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "A"))).unwrap();
    block_on(s.register_player(human(2, "B"))).unwrap();
    block_on(s.insert_computers(vec![computer(10, "Bot", 1, 0, 3)])).unwrap();
    block_on(s.archive_player(pid(2))).unwrap();
    let names = |kind, archived| -> Vec<String> {
        block_on(s.list_players(kind, archived))
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect()
    };
    assert_eq!(names(None, false), ["A", "Bot"]);
    assert_eq!(names(None, true), ["A", "B", "Bot"]);
    assert_eq!(names(Some(PlayerKind::Human), true), ["A", "B"]);
    assert_eq!(names(Some(PlayerKind::Computer), false), ["Bot"]);
}

#[test]
fn rename_changes_the_name_and_refuses_a_taken_one() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "A"))).unwrap();
    block_on(s.register_player(human(2, "B"))).unwrap();
    let renamed = block_on(s.rename_player(pid(1), "A2".into())).unwrap();
    assert_eq!(renamed.name, "A2");
    let same = block_on(s.rename_player(pid(1), "A2".into())).unwrap();
    assert_eq!(
        same.name, "A2",
        "renaming to the own name is not a conflict"
    );
    let err = block_on(s.rename_player(pid(1), "B".into())).unwrap_err();
    assert!(matches!(err, StoreError::NameTaken), "{err:?}");
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().name, "A2");
    let gone = block_on(s.rename_player(pid(77), "X".into())).unwrap_err();
    assert!(matches!(
        gone,
        StoreError::NotFound {
            entity: Entity::Player,
            ..
        }
    ));
}

#[test]
fn archive_is_idempotent_and_refused_while_a_game_is_in_progress() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "A"))).unwrap();
    block_on(s.insert_computers(vec![computer(10, "Bot", 1, 0, 3)])).unwrap();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    let err = block_on(s.archive_player(pid(1))).unwrap_err();
    assert!(
        matches!(&err, StoreError::HasUnfinishedGames { ids } if *ids == vec![common::gid(1)]),
        "{err:?}"
    );
    assert!(!block_on(s.get_player(pid(1))).unwrap().archived);
    block_on(s.register_player(human(2, "B"))).unwrap();
    assert!(block_on(s.archive_player(pid(2))).unwrap().archived);
    assert!(block_on(s.archive_player(pid(2))).unwrap().archived);
}

#[test]
fn preferences_update_replaces_and_rejects_invalid_json() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "A"))).unwrap();
    block_on(s.update_preferences(pid(1), "{\"v\":2}".into())).unwrap();
    assert_eq!(
        block_on(s.get_preferences(pid(1))).unwrap().as_deref(),
        Some("{\"v\":2}")
    );
    let err = block_on(s.update_preferences(pid(1), "nope".into())).unwrap_err();
    assert!(matches!(err, StoreError::InvariantViolated), "{err:?}");
    assert_eq!(
        block_on(s.get_preferences(pid(1))).unwrap().as_deref(),
        Some("{\"v\":2}")
    );
    let gone = block_on(s.update_preferences(pid(5), "{}".into())).unwrap_err();
    assert!(matches!(gone, StoreError::NotFound { .. }));
    assert!(matches!(
        block_on(s.get_preferences(pid(5))).unwrap_err(),
        StoreError::NotFound { .. }
    ));
}

#[test]
fn a_computer_has_no_preferences_row() {
    let (_d, s) = store();
    block_on(s.insert_computers(vec![computer(10, "Bot", 1, 0, 3)])).unwrap();
    assert_eq!(block_on(s.get_preferences(pid(10))).unwrap(), None);
}

#[test]
fn app_settings_upsert_and_read() {
    let (_d, s) = store();
    assert_eq!(
        block_on(s.get_app_setting("roster_seed".into())).unwrap(),
        None
    );
    block_on(s.set_app_setting("roster_seed".into(), "7".into())).unwrap();
    block_on(s.set_app_setting("roster_seed".into(), "8".into())).unwrap();
    assert_eq!(
        block_on(s.get_app_setting("roster_seed".into()))
            .unwrap()
            .as_deref(),
        Some("8")
    );
}

#[test]
fn insert_computers_stores_profile_and_baseline_rating_rows() {
    let (_d, s) = store();
    let batch = vec![
        computer(10, "Bot A", 5, 0, 3),
        computer(11, "Bot B", 5, 1, 4),
    ];
    let records = block_on(s.insert_computers(batch)).unwrap();
    assert_eq!(records.len(), 2);
    let first = &records[0];
    assert_eq!(first.kind, PlayerKind::Computer);
    let profile = first.computer.unwrap();
    assert_eq!(profile.roster_seed, RosterSeed::new(5));
    assert_eq!(profile.roster_index, 0);
    assert_eq!(profile.personality, personality(3));
    assert_eq!(
        profile.personality.error_permille, 20,
        "an integer per mille (D-43)"
    );
    for id in [10, 11] {
        for track in [Track::Strict, Track::Training] {
            let h = block_on(s.rating_history(pid(id), track)).unwrap();
            assert_eq!(h.len(), 1);
            assert_eq!((h[0].rating_before, h[0].rating_after), (800, 800));
        }
    }
    assert!(block_on(s.insert_computers(Vec::new())).unwrap().is_empty());
}

#[test]
fn the_default_roster_of_the_domain_stores_and_feeds_first_k_after() {
    let (_d, s) = store();
    let range = chess_domain::UciEloRange {
        min: 1320,
        max: 3190,
    };
    let specs = RosterGen::new(RosterSeed::new(42), range)
        .default_roster()
        .unwrap();
    assert_eq!(specs.len(), 25);
    let batch: Vec<_> = specs.iter().map(chess_store::NewComputer::from).collect();
    let stored = block_on(s.insert_computers(batch)).unwrap();
    for (spec, record) in specs.iter().zip(&stored) {
        assert_eq!(record.id.as_uuid(), spec.uuid);
        assert_eq!(record.name, spec.name);
        let profile = record.computer.unwrap();
        assert_eq!(profile.personality, spec.personality);
        assert_eq!(profile.roster_index, spec.roster_index);
        assert_eq!(
            (record.strict_elo, record.training_elo),
            (spec.strict_elo_start, spec.training_elo_start)
        );
    }
    let indexes = block_on(s.roster_indexes()).unwrap();
    assert_eq!(indexes, (0..25).collect::<Vec<u32>>());
    for category in [
        Category::Beginner,
        Category::Novice,
        Category::Intermediate,
        Category::Master,
        Category::GrandMaster,
    ] {
        assert_eq!(
            RosterGen::first_k_after(category, &indexes),
            5,
            "{category}"
        );
    }
}

#[test]
fn ut_st_039_roster_indexes_returns_every_stored_index_and_nothing_else() {
    let (d, s) = store();
    assert!(
        block_on(s.roster_indexes()).unwrap().is_empty(),
        "empty table, empty vector"
    );
    block_on(s.insert_computers(vec![
        computer(12, "C", 1, 7, 3),
        computer(10, "A", 1, 0, 4),
        computer(11, "B", 1, 12, 5),
    ]))
    .unwrap();
    assert_eq!(block_on(s.roster_indexes()).unwrap(), [0, 7, 12]);
    // The class arithmetic stays in the domain (D-46): 0 is class 0 (k 0); 7 and 12 are class 2
    // (k 1 and 2).
    let stored = block_on(s.roster_indexes()).unwrap();
    assert_eq!(RosterGen::first_k_after(Category::Beginner, &stored), 1);
    assert_eq!(RosterGen::first_k_after(Category::Intermediate, &stored), 3);
    assert_eq!(RosterGen::first_k_after(Category::Novice, &stored), 0);
    let _ = d;
}

#[test]
fn ut_st_039_schema_has_no_roster_class_column() {
    let (d, _s) = store();
    let conn = raw(d.path());
    let has_class: i64 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('computer_profile') WHERE name LIKE '%class%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_class, 0);
}

#[test]
fn ut_st_039_a_second_insert_with_the_same_seed_and_index_is_roster_index_taken() {
    let (d, s) = store();
    block_on(s.insert_computers(vec![computer(10, "A", 1, 4, 3)])).unwrap();
    let err = block_on(s.insert_computers(vec![computer(11, "B", 1, 4, 9)])).unwrap_err();
    assert!(matches!(err, StoreError::RosterIndexTaken), "{err:?}");
    // The same index under another seed is a different key.
    block_on(s.insert_computers(vec![computer(12, "C", 2, 4, 9)])).unwrap();
    let conn = raw(d.path());
    let n: i64 = conn
        .query_row("SELECT count(*) FROM computer_profile", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
}

#[test]
fn ut_st_039_concurrent_batches_with_the_same_index_one_wins_one_is_taken() {
    let (_d, s) = store();
    let (a, b) = (s.clone(), s.clone());
    let t1 =
        std::thread::spawn(move || block_on(a.insert_computers(vec![computer(10, "A", 1, 4, 3)])));
    let t2 =
        std::thread::spawn(move || block_on(b.insert_computers(vec![computer(11, "B", 1, 4, 9)])));
    let results = [t1.join().unwrap(), t2.join().unwrap()];
    let wins = results.iter().filter(|r| r.is_ok()).count();
    let taken = results
        .iter()
        .filter(|r| matches!(r, Err(StoreError::RosterIndexTaken)))
        .count();
    assert_eq!((wins, taken), (1, 1), "{results:?}");
    assert_eq!(block_on(s.roster_indexes()).unwrap(), [4]);
}

#[test]
fn ut_st_042_a_roster_batch_is_all_or_nothing() {
    let (d, s) = store();
    block_on(s.insert_computers(vec![computer(10, "A", 1, 0, 3)])).unwrap();
    // The third element repeats an index of the first: nothing of the batch may remain.
    let batch = vec![
        computer(11, "B", 1, 1, 4),
        computer(12, "C", 1, 2, 5),
        computer(13, "D", 1, 0, 6),
    ];
    let err = block_on(s.insert_computers(batch)).unwrap_err();
    assert!(matches!(err, StoreError::RosterIndexTaken), "{err:?}");
    assert_eq!(block_on(s.roster_indexes()).unwrap(), [0]);
    let conn = raw(d.path());
    let players: i64 = conn
        .query_row("SELECT count(*) FROM player", [], |r| r.get(0))
        .unwrap();
    let history: i64 = conn
        .query_row("SELECT count(*) FROM rating_history", [], |r| r.get(0))
        .unwrap();
    assert_eq!((players, history), (1, 2));
}

#[test]
fn a_repeated_parameter_set_is_refused_by_the_backstop_index_and_rolls_back() {
    let (_d, s) = store();
    block_on(s.insert_computers(vec![computer(10, "A", 1, 0, 3)])).unwrap();
    let err = block_on(s.insert_computers(vec![computer(11, "B", 1, 1, 3)])).unwrap_err();
    assert!(matches!(err, StoreError::InvariantViolated), "{err:?}");
    assert_eq!(block_on(s.roster_indexes()).unwrap(), [0]);
}

#[test]
fn a_computer_name_clash_with_a_human_is_name_taken() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "Same"))).unwrap();
    let err = block_on(s.insert_computers(vec![computer(10, "Same", 1, 0, 3)])).unwrap_err();
    assert!(matches!(err, StoreError::NameTaken), "{err:?}");
}

// ---------------------------------------------------------------- review FIND-004 .. FIND-006

/// F-A04 is NOT ruled by Brian: this pins the behaviour as built (k = 0, no game, before equal
/// to after) so a change is deliberate. Do not read it as a decision.
#[test]
fn f_a04_initial_rating_points_carry_k_zero_and_no_game_pinned_not_ruled() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    for track in [Track::Strict, Track::Training] {
        let history = block_on(s.rating_history(pid(1), track)).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].k, 0, "INITIAL_POINT_K (F-A04, not ruled)");
        assert_eq!(history[0].game_id, None);
        assert_eq!(history[0].rating_before, history[0].rating_after);
    }
}

#[test]
fn text_in_an_integer_column_is_corrupt_not_internal() {
    let (d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    raw(d.path())
        .execute("UPDATE player SET strict_elo = 'abc'", [])
        .unwrap();
    assert!(matches!(
        block_on(s.get_player(pid(1))).unwrap_err(),
        StoreError::Corrupt
    ));
    assert!(matches!(
        block_on(s.list_players(None, true)).unwrap_err(),
        StoreError::Corrupt
    ));
}

#[test]
fn invalid_utf8_in_a_text_column_is_corrupt_not_internal() {
    let (d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    raw(d.path())
        .execute("UPDATE player SET name = CAST(x'ff' AS TEXT)", [])
        .unwrap();
    assert!(matches!(
        block_on(s.get_player(pid(1))).unwrap_err(),
        StoreError::Corrupt
    ));
}

/// FIND-005, a choice: the SDD is silent, so the behaviour as built is kept and pinned. A
/// computer Player gets a preferences row when `update_preferences` is called for it.
#[test]
fn choice_update_preferences_on_a_computer_creates_a_row_pinned() {
    let (_d, s) = store();
    block_on(s.insert_computers(vec![computer(10, "Bot", 1, 0, 3)])).unwrap();
    block_on(s.update_preferences(pid(10), "{}".into())).unwrap();
    assert_eq!(
        block_on(s.get_preferences(pid(10))).unwrap().as_deref(),
        Some("{}")
    );
}

/// FIND-006, a choice: the SDD is silent on a retried registration, so the behaviour as built
/// is kept and pinned: the same id again is an invariant violation, whatever the name.
#[test]
fn choice_register_twice_with_the_same_id_is_not_idempotent_pinned() {
    let (_d, s) = store();
    block_on(s.register_player(human(1, "Hana"))).unwrap();
    let same_name = block_on(s.register_player(human(1, "Hana"))).unwrap_err();
    let other_name = block_on(s.register_player(human(1, "Other"))).unwrap_err();
    assert!(
        matches!(other_name, StoreError::InvariantViolated),
        "{other_name:?}"
    );
    assert!(matches!(same_name, StoreError::NameTaken), "{same_name:?}");
}
