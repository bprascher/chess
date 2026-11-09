//! UT-DOM-115 drift guard (SF-02, D-64): the seven-row table shared with chess-rules
//! (UT-RUL-031 row 5). The inputs are in `data/fen_drift_guard.txt`; the expected results here
//! are Standard or Custom. The en passant row cannot be built on the standard placement with
//! the `chess-rules` FEN type: it is rejected before classification (step1-domain-A-evidence
//! section 8), so its expected result is a parse error, not Standard and not Custom.
//!
//! The second test reads the rules test source (a relative `include_str!`, no Cargo edge) and
//! asserts that every input that the rules test uses is a row of this table, so the two lists
//! cannot change apart without a red case. Rules edits are out of scope for part B; see the
//! finding F-B3 in step1-domain-B-evidence.md.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use chess_domain::StartPosition;
use chess_rules::Fen;

const TABLE: &str = include_str!("data/fen_drift_guard.txt");
const RULES_PGN_TESTS: &str = include_str!("../../chess-rules/tests/pgn.rs");

fn rows() -> Vec<(&'static str, &'static str, &'static str)> {
    TABLE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split('|');
            (f.next().unwrap(), f.next().unwrap(), f.next().unwrap())
        })
        .collect()
}

#[test]
fn ut_dom_115_sf_02_drift_guard_table_expected_start_kinds() {
    let rows = rows();
    assert_eq!(
        rows.len(),
        7,
        "the standard FEN plus six one-field variants"
    );
    let std = rows[0].1;
    for (name, fen, want) in &rows {
        // exactly one field differs from the standard FEN (none for the first row)
        let diff = fen
            .split(' ')
            .zip(std.split(' '))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(diff, usize::from(*name != "standard"), "{name}");
        match (Fen::parse(fen), *want) {
            (Ok(f), "standard") => assert_eq!(StartPosition::from_fen(f), StartPosition::Standard),
            (Ok(f), "custom") => {
                let s = StartPosition::from_fen(f);
                assert!(s.is_custom(), "{name}");
            }
            (Err(_), "parse_error") => {}
            (other, want) => panic!("{name}: {other:?} against expected {want}"),
        }
    }
}

#[test]
fn ut_dom_115_sf_02_every_fen_the_rules_test_uses_is_a_row_of_the_table() {
    let table_fens: Vec<&str> = rows().iter().map(|r| r.1).collect();
    // The inputs UT-RUL-031 uses in chess-rules/tests/pgn.rs, other than the standard FEN.
    let in_rules: Vec<&str> = RULES_PGN_TESTS
        .lines()
        .filter(|l| l.contains("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKB"))
        .flat_map(|l| l.split('"'))
        .filter(|s| s.starts_with("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/"))
        .collect();
    assert!(
        in_rules.len() >= 5,
        "the rules test no longer holds the variants: {in_rules:?}"
    );
    for fen in in_rules {
        assert!(
            table_fens.contains(&fen),
            "chess-rules uses a FEN that the shared table does not have: {fen}"
        );
    }
    // and the table's rows that the rules test can construct are all present there
    for (name, fen, want) in rows() {
        if want != "parse_error" {
            assert!(
                RULES_PGN_TESTS.contains(fen) || name == "standard",
                "row {name} is not exercised by the rules test: {fen}"
            );
        }
    }
}
