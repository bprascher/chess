//! UT-DOM-071, 112, 113 (+ UT-DOM-114 additions): `normalize_eco_tag` (D-60),
//! `classify_import`, `resolve`. Written before the code.
//!
//! F-D9 (GATE-103): "same entry by transposition" holds only when the final position is itself
//! named. The transposition test here uses a named final position and asserts what is true; it
//! also pins the unnamed-final-position behaviour as observed, not as intended.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    EcoChoice, EcoCode, EcoFields, EcoImportOutcome, EcoMatch, EcoTable, EcoUnanswered,
    classify_import, normalize_eco_tag, resolve,
};
use chess_rules::{Position, PositionKey};
use common::*;
use proptest::prelude::*;

fn m(code: &str, name: &str) -> EcoMatch {
    EcoMatch {
        code: code.into(),
        name: name.into(),
        ply: 5,
    }
}

fn code(s: &str) -> EcoCode {
    normalize_eco_tag(s).unwrap()
}

fn keys_of(start: &Position, ucis: &[&str]) -> Vec<PositionKey> {
    let mut pos = start.clone();
    let mut keys = vec![pos.repetition_key()];
    for u in ucis {
        pos = pos.play(mvu(u)).unwrap();
        keys.push(pos.repetition_key());
    }
    keys
}

// ------------------------------------------------------------------ normalize_eco_tag

#[test]
fn ut_dom_071_d_60_normalisation_trims_upper_cases_and_checks_the_range() {
    for (input, want) in [
        ("C60", Some("C60")),
        ("c60", Some("C60")),
        (" C60 ", Some("C60")),
        ("\tc60\r\n", Some("C60")),
        ("A00", Some("A00")),
        ("E99", Some("E99")),
        ("a00", Some("A00")),
        ("e99", Some("E99")),
        ("B07", Some("B07")),
        ("D50", Some("D50")),
        // malformed: counts as no tag
        ("XYZ", None),
        ("F00", None),
        ("f00", None),
        ("A0", None),
        ("A000", None),
        ("", None),
        ("   ", None),
        ("C 60", None),
        ("C6O", None),
        ("CC6", None),
        ("60C", None),
        ("@00", None),
        ("A-1", None),
        ("\u{ff23}60", None), // full-width C
        ("C\u{0666}0", None), // Arabic-indic digit
        ("C60\u{a0}", None),  // NBSP is not ASCII whitespace
        ("\u{b}C60", None),   // vertical tab is not ASCII whitespace
        ("C60 extra", None),
    ] {
        let got = normalize_eco_tag(input);
        assert_eq!(got.as_ref().map(EcoCode::as_str), want, "{input:?}");
    }
    assert_eq!(code("c60").to_string(), "C60");
}

// ------------------------------------------------------------------ classify_import

#[test]
fn ut_dom_071_classify_import_gives_the_five_variants() {
    let table = m("C60", "Ruy Lopez");
    // agree: the normalised tag equals the table's code (case and padding do not matter)
    for tag in ["C60", "c60", " C60 "] {
        assert_eq!(
            classify_import(Some(tag), Some(table.clone())),
            EcoImportOutcome::Agree {
                code: code("C60"),
                name: "Ruy Lopez".into()
            },
            "{tag:?}"
        );
    }
    // mismatch: both exist, the codes differ; the tag code is normalised
    assert_eq!(
        classify_import(Some("b20"), Some(table.clone())),
        EcoImportOutcome::Mismatch {
            tag_code: code("B20"),
            table: table.clone()
        }
    );
    // tag only: a well-formed tag and no table match
    assert_eq!(
        classify_import(Some(" d00"), None),
        EcoImportOutcome::TagOnly {
            tag_code: code("D00")
        }
    );
    // table only: no tag, or a malformed one
    for tag in [None, Some("XYZ"), Some("F00"), Some("A0"), Some("")] {
        assert_eq!(
            classify_import(tag, Some(table.clone())),
            EcoImportOutcome::TableOnly {
                table: table.clone()
            },
            "{tag:?}"
        );
    }
    // neither
    for tag in [None, Some("XYZ"), Some("")] {
        assert_eq!(
            classify_import(tag, None),
            EcoImportOutcome::Neither,
            "{tag:?}"
        );
    }
}

#[test]
fn ut_dom_071_classify_import_over_a_real_game() {
    let start = Position::standard();
    let ruy = EcoTable::classify(&keys_of(&start, &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"]));
    let table = ruy.clone().expect("named");
    let tag = table.code.to_lowercase();
    assert_eq!(
        classify_import(Some(&format!(" {tag} ")), ruy.clone()),
        EcoImportOutcome::Agree {
            code: code(&table.code),
            name: table.name.clone()
        }
    );
    // a game with no named position and a tag: tag only; without a tag: neither
    let quiet =
        Position::from_fen(&chess_rules::Fen::parse("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").unwrap());
    let none = EcoTable::classify(&keys_of(&quiet, &["a1a2"]));
    assert_eq!(none, None);
    assert!(matches!(
        classify_import(Some("A00"), none.clone()),
        EcoImportOutcome::TagOnly { .. }
    ));
    assert_eq!(classify_import(None, none), EcoImportOutcome::Neither);
}

// ------------------------------------------------------------------ resolve

fn fields(code: Option<&str>, name: Option<&str>) -> EcoFields {
    EcoFields {
        code: code.map(str::to_string),
        name: name.map(str::to_string),
    }
}

#[test]
fn ut_dom_112_resolve_mismatch_follows_the_choice() {
    let table = m("C60", "Ruy Lopez");
    let out = EcoImportOutcome::Mismatch {
        tag_code: code("b20"),
        table: table.clone(),
    };
    assert_eq!(
        resolve(&out, Some(EcoChoice::KeepPgn)),
        Ok(fields(Some("B20"), Some("")))
    );
    assert_eq!(
        resolve(&out, Some(EcoChoice::KeepTable)),
        Ok(fields(Some("C60"), Some("Ruy Lopez")))
    );
    assert_eq!(resolve(&out, None), Err(EcoUnanswered));
    assert!(!EcoUnanswered.to_string().is_empty());
}

#[test]
fn ut_dom_112_resolve_every_other_variant_ignores_the_choice() {
    let table = m("C60", "Ruy Lopez");
    let cases = [
        (
            EcoImportOutcome::Agree {
                code: code("C60"),
                name: "Ruy Lopez".into(),
            },
            fields(Some("C60"), Some("Ruy Lopez")),
        ),
        (
            EcoImportOutcome::TagOnly {
                tag_code: code("d00"),
            },
            fields(Some("D00"), Some("")),
        ),
        (
            EcoImportOutcome::TableOnly { table },
            fields(Some("C60"), Some("Ruy Lopez")),
        ),
        (EcoImportOutcome::Neither, fields(None, None)),
    ];
    for (out, want) in cases {
        for choice in [None, Some(EcoChoice::KeepPgn), Some(EcoChoice::KeepTable)] {
            assert_eq!(
                resolve(&out, choice),
                Ok(want.clone()),
                "{out:?} {choice:?}"
            );
        }
    }
}

// ------------------------------------------------------------------ property

fn outcome_of(tag: &str, table: Option<(u8, u8)>) -> (EcoImportOutcome, Option<EcoMatch>) {
    let table = table.map(|(l, d)| {
        let letter = char::from(b'A' + l % 5);
        m(&format!("{letter}{:02}", d % 100), "Name")
    });
    (classify_import(Some(tag), table.clone()), table)
}

fn is_code(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 3 && (b'A'..=b'E').contains(&b[0]) && b[1].is_ascii_digit() && b[2].is_ascii_digit()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    // UT-DOM-113
    #[test]
    fn ut_dom_113_resolve_never_panics_and_gives_only_a_normalised_or_table_code(
        tag in ".{0,8}",
        table in proptest::option::of((0u8..5, 0u8..100)),
        choice in proptest::option::of(prop_oneof![Just(EcoChoice::KeepPgn), Just(EcoChoice::KeepTable)]),
    ) {
        let (out, table) = outcome_of(&tag, table);
        let tag_code = normalize_eco_tag(&tag);
        match resolve(&out, choice) {
            Err(EcoUnanswered) => {
                let is_mismatch = matches!(out, EcoImportOutcome::Mismatch { .. });
                prop_assert!(is_mismatch && choice.is_none());
            }
            Ok(f) => {
                if let Some(c) = &f.code {
                    prop_assert!(is_code(c), "{c}");
                    let from_tag = tag_code.as_ref().map(EcoCode::as_str) == Some(c.as_str());
                    let from_table = table.as_ref().map(|t| t.code.as_str()) == Some(c.as_str());
                    prop_assert!(from_tag || from_table);
                } else {
                    prop_assert!(f.name.is_none());
                    prop_assert!(tag_code.is_none() && table.is_none());
                }
            }
        }
        // KeepTable on a mismatch equals TableOnly
        if let EcoImportOutcome::Mismatch { table: t, .. } = &out {
            prop_assert_eq!(
                resolve(&out, Some(EcoChoice::KeepTable)),
                resolve(&EcoImportOutcome::TableOnly { table: t.clone() }, None)
            );
        }
        // the normalisation is idempotent
        if let Some(c) = tag_code {
            prop_assert_eq!(normalize_eco_tag(c.as_str()), Some(c));
        }
    }
}

// ------------------------------------------------------------------ UT-DOM-070/114 additions

#[test]
fn ut_dom_070_f_d9_transposition_with_a_named_final_position_gives_the_same_entry() {
    let start = Position::standard();
    // Ruy Lopez by two orders: the final position is named, so the entry and its ply agree.
    let a = EcoTable::classify(&keys_of(&start, &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"]));
    let b = EcoTable::classify(&keys_of(&start, &["g1f3", "b8c6", "e2e4", "e7e5", "f1b5"]));
    assert!(a.is_some());
    assert_eq!(a, b);
    // The unnamed-final-position case is observed, not intended (F-D9): the tests in eco.rs
    // pin that two orders to one unnamed position can classify differently.
}

#[test]
fn ut_dom_114_cr_9_11_a_custom_start_that_transposes_into_a_named_position_is_classified() {
    // A custom start (1.e4 e5 2.Nf3 Nc6 as a position, not a move list) that then plays Bb5:
    // the Ruy Lopez position is reached from a start that is not the standard one.
    let fen = "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
    let mut r = Run::new(with_start(training(), fen));
    let before = r.game.eco().cloned();
    r.mv("f1b5");
    let after = r
        .game
        .eco()
        .cloned()
        .expect("the Ruy Lopez position is named");
    assert_ne!(before, Some(after.clone()));
    assert_eq!(after.ply, 1, "ply counts from the custom start");
    assert!(after.code.starts_with('C'));
    // the same moves as a standard game give the same entry (at their own ply)
    let std_keys = keys_of(
        &Position::standard(),
        &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"],
    );
    let std = EcoTable::classify(&std_keys).unwrap();
    assert_eq!(
        (after.code.as_str(), after.name.as_str()),
        (std.code.as_str(), std.name.as_str())
    );
}
