//! Property test (model against implementation, `proptest`, D-76): a random sequence of
//! register, rename and archive calls on a real temporary store gives the same answers as a
//! plain in-memory model, including the error for a taken name and for archiving with a game in
//! progress, and the table contents agree at the end (UT-ST-040 to UT-ST-049 family).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use std::collections::HashMap;

use chess_domain::{Matchup, Mode};
use chess_store::StoreError;
use common::{block_on, computer, human, new_game, open_in, pid};
use proptest::prelude::*;
use tempfile::TempDir;

const NAMES: [&str; 4] = ["Ann", "ann", "Bob", "Cy"];

#[derive(Debug, Clone)]
enum Op {
    Register { slot: u8, name: u8 },
    Rename { slot: u8, name: u8 },
    Archive { slot: u8 },
    StartGame { slot: u8 },
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u8..4, 0u8..4).prop_map(|(slot, name)| Op::Register { slot, name }),
        (0u8..4, 0u8..4).prop_map(|(slot, name)| Op::Rename { slot, name }),
        (0u8..4).prop_map(|slot| Op::Archive { slot }),
        (0u8..4).prop_map(|slot| Op::StartGame { slot }),
    ]
}

#[derive(Default)]
struct Model {
    names: HashMap<u8, String>,
    archived: HashMap<u8, bool>,
    in_progress: HashMap<u8, bool>,
}

impl Model {
    fn name_used(&self, name: &str, except: Option<u8>) -> bool {
        self.names
            .iter()
            .any(|(slot, n)| n == name && Some(*slot) != except)
    }
}

fn kind(result: &Result<impl Sized, StoreError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(StoreError::NameTaken) => "name_taken",
        Err(StoreError::NotFound { .. }) => "not_found",
        Err(StoreError::InvariantViolated) => "invariant",
        Err(StoreError::HasUnfinishedGames { .. }) => "unfinished",
        Err(StoreError::SlotTaken { .. }) => "slot_taken",
        Err(_) => "other",
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn the_store_agrees_with_a_plain_model(ops in proptest::collection::vec(op(), 1..14)) {
        let dir = TempDir::new().unwrap();
        let store = open_in(dir.path()).store;
        block_on(store.insert_computers(vec![computer(900, "Bot", 1, 0, 3)])).unwrap();
        let mut model = Model::default();
        let mut next_game = 1u128;
        for op in ops {
            match op {
                Op::Register { slot, name } => {
                    let name = NAMES[usize::from(name)];
                    let real = block_on(store.register_player(human(u128::from(slot) + 1, name)));
                    // The name check runs first; a repeated id then hits UNIQUE(uuid).
                    let expected = if model.name_used(name, None) {
                        "name_taken"
                    } else if model.names.contains_key(&slot) {
                        "invariant"
                    } else {
                        "ok"
                    };
                    prop_assert_eq!(kind(&real), expected);
                    if expected == "ok" {
                        model.names.insert(slot, name.to_owned());
                        model.archived.insert(slot, false);
                    }
                }
                Op::Rename { slot, name } => {
                    let name = NAMES[usize::from(name)];
                    let real = block_on(store.rename_player(pid(u128::from(slot) + 1), name.to_owned()));
                    let expected = if !model.names.contains_key(&slot) {
                        "not_found"
                    } else if model.name_used(name, Some(slot)) {
                        "name_taken"
                    } else {
                        "ok"
                    };
                    prop_assert_eq!(kind(&real), expected);
                    if expected == "ok" {
                        model.names.insert(slot, name.to_owned());
                    }
                }
                Op::Archive { slot } => {
                    let real = block_on(store.archive_player(pid(u128::from(slot) + 1)));
                    let expected = if !model.names.contains_key(&slot) {
                        "not_found"
                    } else if model.in_progress.get(&slot) == Some(&true) {
                        "unfinished"
                    } else {
                        "ok"
                    };
                    prop_assert_eq!(kind(&real), expected);
                    if expected == "ok" {
                        model.archived.insert(slot, true);
                    }
                }
                Op::StartGame { slot } => {
                    let g = new_game(next_game, pid(u128::from(slot) + 1), pid(900), Matchup::HumanComputer, Mode::Strict);
                    next_game += 1;
                    let real = block_on(store.start_game(g));
                    let expected = if !model.names.contains_key(&slot) {
                        "not_found"
                    } else if model.in_progress.get(&slot) == Some(&true) {
                        "slot_taken"
                    } else {
                        "ok"
                    };
                    prop_assert_eq!(kind(&real), expected);
                    if expected == "ok" {
                        model.in_progress.insert(slot, true);
                    }
                }
            }
        }
        // Final state agrees.
        for (slot, name) in &model.names {
            let p = block_on(store.get_player(pid(u128::from(*slot) + 1))).unwrap();
            prop_assert_eq!(&p.name, name);
            prop_assert_eq!(p.archived, model.archived[slot]);
        }
        let listed = block_on(store.list_players(Some(chess_domain::PlayerKind::Human), true)).unwrap();
        prop_assert_eq!(listed.len(), model.names.len());
    }
}
