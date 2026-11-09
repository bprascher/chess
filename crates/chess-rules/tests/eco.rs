//! DD-RUL-013 through the public API only (D-75): UT-RUL-023 (ECO lookup, agreement between the
//! build-time replay and `Position::play`) and UT-RUL-030 (the lookup depends only on the
//! position key). Requirements DAT-3, CR-9.11, D-20. The expected values below that name an
//! opening come from the checked-in snapshot `third_party/eco/` (read in this session).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use chess_rules::eco::{self, EcoEntry};
use chess_rules::{Fen, Position, PositionKey, parse_line};
use common::{START_FEN, pos};

struct Row {
    eco: String,
    name: String,
    pgn: String,
}

fn eco_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third_party/eco")
}

fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    for f in ["a", "b", "c", "d", "e"] {
        let text = std::fs::read_to_string(eco_dir().join(format!("{f}.tsv"))).unwrap();
        for line in text.lines().skip(1) {
            let mut cols = line.split('\t');
            out.push(Row {
                eco: cols.next().unwrap().to_string(),
                name: cols.next().unwrap().to_string(),
                pgn: cols.next().unwrap().to_string(),
            });
        }
    }
    out
}

/// The position after the moves of `pgn` from the standard start, and the ply count.
fn reach(pgn: &str) -> (Position, usize) {
    let nodes = parse_line(&Position::standard(), pgn).unwrap();
    let mut cur = Position::standard();
    for n in &nodes {
        cur = cur.play(n.mv).unwrap();
    }
    (cur, nodes.len())
}

fn key_after(pgn: &str) -> PositionKey {
    reach(pgn).0.repetition_key()
}

#[test]
fn ut_rul_023_dat_3_every_row_of_the_snapshot_is_found_through_a_position_reached_by_play() {
    // Oracle: for each position, the longest line wins, a tie goes to the smaller code and then
    // the smaller name (UR-27). This is computed here from the TSV text and `Position::play`,
    // not from the generator, so a disagreement between the build-time replay and `parse_move`
    // and `play` shows up as a wrong or missing entry.
    let mut best: HashMap<String, (usize, String, String)> = HashMap::new();
    let all = rows();
    assert!(all.len() > 3000, "the snapshot has {} rows", all.len());
    for r in &all {
        let (p, plies) = reach(&r.pgn);
        let epd = p.repetition_key().to_epd();
        let cand = (plies, r.eco.clone(), r.name.clone());
        best.entry(epd)
            .and_modify(|cur| {
                let better = cand.0 > cur.0
                    || (cand.0 == cur.0
                        && (cand.1.as_str(), cand.2.as_str()) < (cur.1.as_str(), cur.2.as_str()));
                if better {
                    *cur = cand.clone();
                }
            })
            .or_insert(cand);
    }
    assert_eq!(
        eco::TABLE_LEN,
        best.len(),
        "one table entry per distinct position"
    );
    for r in &all {
        let (p, _) = reach(&r.pgn);
        let key = p.repetition_key();
        let want = &best[&key.to_epd()];
        let got = eco::lookup(&key).unwrap_or_else(|| panic!("no entry for {}", r.pgn));
        assert_eq!(
            (got.code, got.name),
            (want.1.as_str(), want.2.as_str()),
            "{}",
            r.pgn
        );
    }
}

#[test]
fn ut_rul_023_dat_3_fixed_openings_give_the_expected_code_and_name() {
    // First data row of a.tsv: "A00  Amar Opening  1. Nh3".
    let e = eco::lookup(&key_after("1. Nh3")).unwrap();
    assert_eq!((e.code, e.name), ("A00", "Amar Opening"));
    // The entry type is plain data.
    let copy: EcoEntry = e;
    assert_eq!(copy, e);
    // A line from the file itself: the longest line to its position names it.
    let rows = rows();
    let r = rows
        .iter()
        .find(|r| r.name == "Italian Game")
        .expect("Italian Game row");
    let e = eco::lookup(&key_after(&r.pgn)).unwrap();
    assert!(e.name.starts_with("Italian Game"), "{}", e.name);
}

#[test]
fn ut_rul_023_dat_3_a_transposition_gives_the_same_entry_as_the_direct_order() {
    let direct = eco::lookup(&key_after("1. d4 d5 2. c4")).unwrap();
    let swapped = eco::lookup(&key_after("1. c4 d5 2. d4")).unwrap();
    assert_eq!(direct, swapped);
    let a = eco::lookup(&key_after("1. e4 e5 2. Nf3 Nc6 3. Bc4")).unwrap();
    let b = eco::lookup(&key_after("1. Nf3 Nc6 2. e4 e5 3. Bc4")).unwrap();
    assert_eq!(a, b);
}

#[test]
fn ut_rul_023_dat_3_an_unnamed_position_gives_none() {
    assert_eq!(eco::lookup(&Position::standard().repetition_key()), None);
    assert_eq!(
        eco::lookup(&key_after("1. h4 a5 2. h5 a4 3. h6 a3 4. hxg7 axb2")),
        None
    );
}

#[test]
fn ut_rul_023_dat_3_the_key_ignores_move_counters_and_an_unusable_en_passant_square() {
    let (reached, _) = reach("1. Nh3");
    let from_fen = pos("rnbqkbnr/pppppppp/8/8/8/7N/PPPPPPPP/RNBQKB1R b KQkq - 17 40");
    assert_eq!(reached.repetition_key(), from_fen.repetition_key());
    assert_eq!(
        eco::lookup(&from_fen.repetition_key()),
        eco::lookup(&reached.repetition_key())
    );
    assert!(eco::lookup(&from_fen.repetition_key()).is_some());
}

#[test]
fn ut_rul_030_cr_9_11_lookup_depends_only_on_the_position_key() {
    // A custom start that is a named position is classified exactly as when it is reached by
    // moves (ply 0 of a custom start).
    let reached = reach("1. e4 c5").0;
    let custom = Position::from_fen(&Fen::parse(reached.to_fen().as_str()).unwrap());
    assert_eq!(custom.repetition_key(), reached.repetition_key());
    let entry = eco::lookup(&custom.repetition_key());
    assert!(entry.is_some());
    assert_eq!(entry, eco::lookup(&reached.repetition_key()));
    // The same position reached from a custom start by play equals the one from the standard
    // start: 1. e4 played on the standard FEN taken as a custom start.
    let from_std_fen = pos(START_FEN);
    let m = chess_rules::Move::from_uci("e2e4").unwrap();
    let after = from_std_fen.play(m).unwrap();
    assert_eq!(after.repetition_key(), reach("1. e4").0.repetition_key());
    assert_eq!(
        eco::lookup(&after.repetition_key()),
        eco::lookup(&key_after("1. e4"))
    );
    // A FEN equal to the standard start gives the same key and the same answer as the start.
    assert_eq!(
        pos(START_FEN).repetition_key(),
        Position::standard().repetition_key()
    );
    assert_eq!(
        eco::lookup(&pos(START_FEN).repetition_key()),
        eco::lookup(&Position::standard().repetition_key())
    );
}
