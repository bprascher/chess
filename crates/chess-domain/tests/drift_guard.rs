//! UT-DOM-115 drift guard (SF-02, D-64): the seven-row table shared with chess-rules
//! (UT-RUL-031 row 5). The single source is `crates/chess-rules/tests/data/fen_drift_guard.txt`
//! (C-R4, desk review R13); `chess-rules/tests/pgn.rs` reads the same file. This test reads it
//! by a relative `include_str!` (test-only, no Cargo edge between the crates). Only the
//! inputs are shared; the expected results here are Standard or Custom. The en passant row
//! cannot be built on the standard placement with the `chess-rules` FEN type: it is rejected
//! before classification (step1-domain-A-evidence section 8), so its expected result is a parse
//! error, not Standard and not Custom.
//!
//! Red-if-diverge: both sides read one file, so an input edit reaches both. The third column
//! (this crate's expectation) is checked against the real classifier; the negative test below
//! shows that a wrong expectation, a changed row or a dropped row turns the check red.
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

const TABLE: &str = include_str!("../../chess-rules/tests/data/fen_drift_guard.txt");

fn rows_of(table: &str) -> Vec<(&str, &str, &str)> {
    table
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split('|');
            (f.next().unwrap(), f.next().unwrap(), f.next().unwrap())
        })
        .collect()
}

/// Checks a table against the classifier; `Err` names the first row that disagrees.
fn check_table(table: &str) -> Result<(), String> {
    let rows = rows_of(table);
    if rows.len() != 7 {
        return Err(format!("{} rows, expected 7", rows.len()));
    }
    let std = rows[0].1;
    for (name, fen, want) in &rows {
        // exactly one field differs from the standard FEN (none for the first row)
        let diff = fen
            .split(' ')
            .zip(std.split(' '))
            .filter(|(a, b)| a != b)
            .count();
        if diff != usize::from(*name != "standard") {
            return Err(format!(
                "{name}: {diff} fields differ from the standard FEN"
            ));
        }
        let ok = match (Fen::parse(fen), *want) {
            (Ok(f), "standard") => StartPosition::from_fen(f) == StartPosition::Standard,
            (Ok(f), "custom") => StartPosition::from_fen(f).is_custom(),
            (Err(_), "parse_error") => true,
            _ => false,
        };
        if !ok {
            return Err(format!("{name}: classifier disagrees with {want}"));
        }
    }
    Ok(())
}

#[test]
fn ut_dom_115_sf_02_drift_guard_table_expected_start_kinds() {
    assert_eq!(check_table(TABLE), Ok(()));
}

#[test]
fn ut_dom_115_sf_02_the_check_is_red_when_an_expectation_or_a_row_diverges() {
    // A wrong expectation in the third column.
    assert!(check_table(&TABLE.replace("|custom", "|standard")).is_err());
    // A row whose FEN differs in two fields (the placement row also loses castling rights).
    assert!(check_table(&TABLE.replace("RNBQKB1R w KQkq - 0 1", "RNBQKB1R w Kkq - 0 1")).is_err());
    // A dropped row.
    let dropped = TABLE
        .lines()
        .filter(|l| !l.starts_with("castling|"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(check_table(&dropped).is_err());
}
