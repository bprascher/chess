//! C-R5 (chess-pdc), desk review R1 / RF-07: the error taxonomy is matched by variant, not by
//! English text. One test row per promoted reason. Also the `Fen::parse` upstream-error arms
//! that were deleted as unreachable (property test).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{Fen, FenError, PgnErrorKind, PgnLimits, PgnReader, SyntaxReason, TagReason};
use proptest::prelude::*;

fn first_error(text: &str) -> chess_rules::PgnParseError {
    PgnReader::new(text.as_bytes(), PgnLimits::default())
        .next()
        .unwrap()
        .unwrap_err()
}

#[test]
fn c_r5_each_syntax_reason_is_its_own_variant() {
    let rows: [(&str, SyntaxReason); 8] = [
        ("1. e4 } e5 *", SyntaxReason::StrayClosingBrace),
        ("1. zz *", SyntaxReason::NotAMove),
        ("1. nf3 *", SyntaxReason::LowercasePieceLetter),
        ("( 1. e4 ) *", SyntaxReason::VariationWithoutMove),
        ("1. e4 $999 *", SyntaxReason::BadNag),
        ("$1 1. e4 *", SyntaxReason::NagWithoutMove),
        ("1. e4 \u{ff}\u{fe} *", SyntaxReason::NotAMove),
        (
            "[Event \"a\"]\n\n[Event \"b\"]\n\n1. e4 *\n",
            SyntaxReason::TagSectionWithoutMovetext,
        ),
    ];
    for (text, want) in rows {
        let e = first_error(text);
        assert_eq!(e.kind, PgnErrorKind::BadSyntax { reason: want }, "{text:?}");
    }
}

#[test]
fn c_r5_non_utf8_move_token_is_not_text() {
    // A move token with a byte that is not UTF-8 (Latin-1 decoding applies to payloads only).
    let bytes = b"1. e4 \xe9\xe9 *";
    let e = PgnReader::new(bytes, PgnLimits::default())
        .next()
        .unwrap()
        .unwrap_err();
    assert_eq!(
        e.kind,
        PgnErrorKind::BadSyntax {
            reason: SyntaxReason::NotText
        }
    );
}

#[test]
fn c_r5_each_tag_reason_is_its_own_variant() {
    let rows: [(&str, TagReason); 4] = [
        ("[event \"x\"]\n\n*\n", TagReason::NameStart),
        ("[Event x]\n\n*\n", TagReason::ValueNotQuoted),
        ("[Event \"x\n\n*\n", TagReason::ValueNotClosed),
        ("[Event \"x\" y]\n\n*\n", TagReason::MissingClosingBracket),
    ];
    for (text, want) in rows {
        let e = first_error(text);
        assert_eq!(e.kind, PgnErrorKind::BadTag { reason: want }, "{text:?}");
    }
}

#[test]
fn c_r5_messages_keep_their_wording() {
    let e = first_error("1. e4 } e5 *");
    assert_eq!(
        e.to_string(),
        "game 1, line 1, column 7: bad syntax: a closing brace without an opening one"
    );
    let e = first_error("[Event x]\n\n*\n");
    assert_eq!(
        e.to_string(),
        "game 1, line 1, column 8: bad tag: a tag value is in double quotes"
    );
}

// The five upstream `ParseFenError` arms of `Fen::parse` were deleted (RF-07): after the crate's
// own field checks the upstream FEN reader cannot fail. This sweeps structurally valid inputs.
fn rank() -> impl Strategy<Value = String> {
    // Eight squares: pieces and runs of empties, never two digits in a row.
    proptest::collection::vec(
        prop_oneof![
            Just("1"),
            Just("2"),
            Just("3"),
            Just("p"),
            Just("P"),
            Just("k"),
            Just("K"),
            Just("q"),
            Just("R"),
            Just("n"),
        ],
        1..12,
    )
    .prop_map(|parts| {
        let mut out = String::new();
        let mut squares = 0u32;
        let mut last_digit = false;
        for p in parts {
            let w = p.parse::<u32>().ok();
            let add = w.unwrap_or(1);
            if squares + add > 8 || (w.is_some() && last_digit) {
                continue;
            }
            squares += add;
            last_digit = w.is_some();
            out.push_str(p);
        }
        if squares < 8 {
            if last_digit {
                out.push('p');
                squares += 1;
            }
            if squares < 8 {
                out.push_str(&(8 - squares).to_string());
            }
        }
        out
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]
    #[test]
    fn c_r5_rf_07_prechecked_fens_never_reach_an_upstream_parse_error(
        ranks in proptest::collection::vec(rank(), 8),
        side in prop_oneof![Just("w"), Just("b")],
        castling in prop_oneof![Just("-"), Just("K"), Just("KQ"), Just("Kk"), Just("KQkq"), Just("q")],
        ep in prop_oneof![Just("-"), Just("e3"), Just("e6"), Just("a3"), Just("h6")],
        half in 0u32..200,
        full in 1u32..500,
    ) {
        let text = format!("{} {side} {castling} {ep} {half} {full}", ranks.join("/"));
        if let Err(FenError::BadField { detail, .. }) = Fen::parse(&text) {
            // A field error from the crate's own checks has a specific detail. The deleted
            // arms' fallback detail ("not accepted") must never appear.
            prop_assert_ne!(detail, "not accepted", "{}", text);
        }
    }
}
