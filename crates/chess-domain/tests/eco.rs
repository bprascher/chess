//! UT-DOM-070 (part A subset), UT-DOM-114 (ply 0 of a custom start), UT-DOM-038-adjacent: the
//! deepest-named-position rule that `Game` uses. Import classification (UT-DOM-071, 112, 113) is
//! part B. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{Actor, EcoMatch, EcoTable, GameCommand as C};
use chess_rules::{Position, PositionKey};
use common::*;

fn keys_of(start: &Position, ucis: &[&str]) -> Vec<PositionKey> {
    let mut pos = start.clone();
    let mut keys = vec![pos.repetition_key()];
    for u in ucis {
        pos = pos.play(mvu(u)).unwrap();
        keys.push(pos.repetition_key());
    }
    keys
}

#[test]
fn ut_dom_070_dat_3_known_opening_deepest_named_position_wins() {
    let start = Position::standard();
    let ruy = ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"];
    let keys = keys_of(&start, &ruy);
    let m = EcoTable::classify(&keys).expect("the Ruy Lopez is named");
    assert!(m.code.starts_with('C'), "{m:?}");
    assert!(!m.name.is_empty());
    assert_eq!(m.ply, 5, "named at the last ply");
    // two more moves leave the table's named positions: the match stays at its ply
    let longer = keys_of(
        &start,
        &[
            "e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5c6", "d7c6", "e1g1", "f7f6", "d2d4",
            "c8g4", "c2c3",
        ],
    );
    let deeper = EcoTable::classify(&longer).unwrap();
    assert!(deeper.ply >= 5);
    assert_eq!(deeper.code.chars().next(), Some('C'));
    // observe agrees with classify
    let mut cur: Option<EcoMatch> = None;
    for (i, k) in keys.iter().enumerate() {
        cur = EcoTable::observe(cur, k, u32::try_from(i).unwrap());
    }
    assert_eq!(cur, Some(m));
}

#[test]
fn ut_dom_070_dat_3_transposition_gives_the_same_match_and_an_unnamed_game_none() {
    let start = Position::standard();
    // The final position is named, so both move orders end on the same entry at the same ply.
    let a =
        EcoTable::classify(&keys_of(&start, &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"])).unwrap();
    let b =
        EcoTable::classify(&keys_of(&start, &["g1f3", "b8c6", "e2e4", "e7e5", "f1b5"])).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.ply, 5);
    // Observed (GATE-103 F-D9): when the final position is NOT named, two orders that reach it
    // can differ, because "deepest named position" is taken along the path. 1.d4 Nf6 2.c4 e6
    // and 1.c4 e6 2.d4 Nf6 reach one position and classify differently.
    let d = EcoTable::classify(&keys_of(&start, &["d2d4", "g8f6", "c2c4", "e7e6"])).unwrap();
    let c = EcoTable::classify(&keys_of(&start, &["c2c4", "e7e6", "d2d4", "g8f6"])).unwrap();
    assert_ne!((d.code, d.ply), (c.code, c.ply));
    assert_eq!(
        EcoTable::classify(&keys_of(&start, &[])),
        None,
        "the standard start is not named"
    );
    assert_eq!(EcoTable::classify(&[]), None);
}

#[test]
fn ut_dom_114_cr_9_11_a_custom_start_that_is_a_named_position_is_classified_at_ply_0() {
    // The position after 1.e4 e5 as a custom start.
    let fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2";
    let mut r = Run::new(with_start(training(), fen));
    let ply0 = r.game.eco().expect("named at ply 0").clone();
    assert_eq!(ply0.ply, 0);
    // a custom start with no named position and no later named position has none
    let quiet = Run::new(with_start(training(), "4k3/8/8/8/8/8/8/R3K3 w - - 0 1"));
    assert_eq!(quiet.game.eco(), None);
    // a takeback to ply 0 keeps the ply 0 classification
    r.mv("g1f3");
    r.cmd(C::EngineStarted);
    r.cmd(r.move_cmd("b8c6", Actor::Engine));
    r.mv("f1b5");
    r.cmd(C::EngineStarted);
    r.cmd(C::Takeback);
    r.cmd(C::Takeback);
    assert_eq!(r.game.ply(), 0);
    assert_eq!(r.game.eco(), Some(&ply0));
}

#[test]
fn ut_dom_114_a_start_fen_equal_to_the_standard_start_classifies_like_the_standard_game() {
    let std_run = Run::new(training());
    let norm = Run::new(with_start(training(), START));
    assert_eq!(std_run.game.eco(), norm.game.eco());
    assert!(!norm.game.setup().start.is_custom());
}
