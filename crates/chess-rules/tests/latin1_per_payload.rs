//! chess-n3h (Brian Prascher's decision, 2026-11-09, "option B"): the PGN reader decodes every
//! text payload (tag value, comment) on its own. Valid UTF-8 is read as it is; a payload that is
//! not valid UTF-8 is read as Latin-1, and only that payload. One `ImportWarning::Latin1Fallback`
//! is recorded per payload that fell back (it carries no position). Replaces the whole-game rule
//! of SDD 3.11.8 / finding C-R5-F1.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

use chess_rules::{
    ImportWarning, MAX_IMPORT_WARNINGS_PER_GAME, PgnGame, PgnLimits, PgnReader, canonical_form,
    write_pgn,
};

fn read_all(bytes: &[u8]) -> Vec<Result<PgnGame, chess_rules::PgnParseError>> {
    PgnReader::new(bytes, PgnLimits::default()).collect()
}

fn read(bytes: &[u8]) -> PgnGame {
    let mut all = read_all(bytes);
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0).unwrap()
}

fn fallbacks(g: &PgnGame) -> usize {
    g.warnings
        .iter()
        .filter(|w| matches!(w, ImportWarning::Latin1Fallback))
        .count()
}

#[test]
fn zoe_in_utf_8_and_a_latin_1_byte_in_the_same_game_are_both_read_right() {
    // White is UTF-8 "Zoë" (C3 AB); Black is Latin-1 "Café" (E9).
    let g = read(b"[White \"Zo\xc3\xab\"]\n[Black \"Caf\xe9\"]\n\n1. e4 *\n");
    assert_eq!(g.tags.white, "Zo\u{eb}");
    assert_eq!(g.tags.black, "Caf\u{e9}");
    assert_eq!(g.warnings, vec![ImportWarning::Latin1Fallback]);
}

#[test]
fn order_of_the_payloads_does_not_matter() {
    let g = read(b"[White \"Caf\xe9\"]\n[Black \"Zo\xc3\xab\"]\n\n1. e4 *\n");
    assert_eq!(g.tags.white, "Caf\u{e9}");
    assert_eq!(g.tags.black, "Zo\u{eb}");
}

#[test]
fn comments_are_decoded_one_by_one_too() {
    let g = read(b"[Event \"x\"]\n\n1. e4 {Zo\xc3\xab} e5 {caf\xe9} 2. Nf3 ; \xc3\xa9\xe9\n");
    assert_eq!(g.moves[0].comment.as_deref(), Some("Zo\u{eb}"));
    assert_eq!(g.moves[1].comment.as_deref(), Some("caf\u{e9}"));
    // The `;` comment holds one valid UTF-8 character and one stray byte: the payload is not
    // valid UTF-8 as a whole, so it is Latin-1 as a whole.
    assert_eq!(g.moves[2].comment.as_deref(), Some("\u{c3}\u{a9}\u{e9}"));
    assert_eq!(fallbacks(&g), 2);
}

#[test]
fn a_payload_is_never_split_valid_prefix_and_invalid_tail_is_one_latin_1_payload() {
    let g = read(b"[White \"Zo\xc3\xab \xff\"]\n\n1. e4 *\n");
    assert_eq!(g.tags.white, "Zo\u{c3}\u{ab} \u{ff}");
    assert_eq!(fallbacks(&g), 1);
}

#[test]
fn one_warning_per_fallback_payload_in_the_order_met_and_none_for_utf_8_or_ascii() {
    let g = read(
        b"[White \"a\xe9\"]\n[Black \"b\xe9\"]\n[Event \"plain\"]\n[Site \"Zo\xc3\xab\"]\n\n1. e4 {c\xe9} e5 {ok} *\n",
    );
    assert_eq!(g.warnings, vec![ImportWarning::Latin1Fallback; 3]);
}

#[test]
fn warning_order_between_game_comments_then_fallbacks_and_typed_tag_warnings() {
    let g = read(b"{between}\n[Date \"bad\"]\n[White \"\xe9\"]\n\n{c} 1. e4 {\xe9} *\n");
    assert_eq!(
        g.warnings,
        vec![
            ImportWarning::CommentDropped { line: 1, col: 1 },
            ImportWarning::Latin1Fallback, // the White tag
            ImportWarning::DateIgnored {
                value: "bad".into()
            },
            ImportWarning::CommentDropped { line: 5, col: 1 },
            ImportWarning::Latin1Fallback, // the second comment
        ]
    );
}

#[test]
fn the_cap_holds_for_fallback_warnings() {
    let mut text = b"[Event \"x\"]\n\n1. e4 ".to_vec();
    for _ in 0..(MAX_IMPORT_WARNINGS_PER_GAME + 50) {
        text.extend_from_slice(b"{\xe9}");
    }
    text.extend_from_slice(b" *\n");
    let g = read(&text);
    assert_eq!(g.warnings.len(), MAX_IMPORT_WARNINGS_PER_GAME + 1);
    assert_eq!(
        g.warnings.last(),
        Some(&ImportWarning::WarningsTruncated { dropped: 50 })
    );
    assert_eq!(fallbacks(&g), MAX_IMPORT_WARNINGS_PER_GAME);
}

#[test]
fn games_are_still_independent_and_the_bom_crlf_and_lone_cr_still_work() {
    let mixed = b"[White \"M\xfcller\"]\n[Black \"\xc3\xa9\"]\n\n1. e4 {\xc3\xa9} *\n\n[White \"\xc3\xa9\"]\n\n1. d4 *\n";
    let all = read_all(mixed);
    let g0 = all[0].as_ref().unwrap();
    assert_eq!(g0.tags.white, "M\u{fc}ller");
    assert_eq!(g0.tags.black, "\u{e9}");
    assert_eq!(g0.moves[0].comment.as_deref(), Some("\u{e9}"));
    assert_eq!(all[1].as_ref().unwrap().tags.white, "\u{e9}");
    assert!(all[1].as_ref().unwrap().warnings.is_empty());
    for sep in ["\n", "\r\n", "\r"] {
        let mut b = vec![0xef, 0xbb, 0xbf];
        b.extend_from_slice(
            format!("[White \"Zo\u{eb}\"]{sep}[Black \"X\u{1}\"]{sep}{sep}1. e4 {{c}} *{sep}")
                .as_bytes(),
        );
        // Replace the Black placeholder by a Latin-1 byte, keeping the separator under test.
        let pos = b.iter().position(|c| *c == 1).unwrap();
        b[pos] = 0xe9;
        let g = read(&b);
        assert_eq!(g.tags.white, "Zo\u{eb}", "{sep:?}");
        assert_eq!(g.tags.black, "X\u{e9}", "{sep:?}");
    }
}

#[test]
fn canonical_form_and_writer_use_the_per_payload_text() {
    let g = read(b"[White \"Zo\xc3\xab\"]\n[Black \"Caf\xe9\"]\n\n1. e4 *\n");
    let expected = "[Event \"?\"]\n[Site \"?\"]\n[Date \"????.??.??\"]\n[Round \"-\"]\n[White \"Zo\u{eb}\"]\n[Black \"Caf\u{e9}\"]\n[Result \"*\"]\n\n1. e4 *\n";
    assert_eq!(String::from_utf8(canonical_form(&g)).unwrap(), expected);
    assert!(write_pgn(&g).unwrap().contains("[White \"Zo\u{eb}\"]"));
}

// ---------------------------------------------------------------- robustness

#[test]
fn invalid_byte_sequences_everywhere_never_panic_and_always_terminate() {
    let bad: [&[u8]; 9] = [
        b"\xff",
        b"\xc3",             // truncated 2-byte sequence
        b"\xe2\x82",         // truncated 3-byte sequence
        b"\xf0\x9f\x98",     // truncated 4-byte sequence
        b"\xc0\x80",         // overlong
        b"\xed\xa0\x80",     // surrogate
        b"\xf4\x90\x80\x80", // above U+10FFFF
        b"\x80\x80",
        b"\xef\xbb\xbf", // a BOM inside a payload is a character, not a BOM
    ];
    for b in bad {
        let wrap = |pre: &[u8], post: &[u8]| {
            let mut v = pre.to_vec();
            v.extend_from_slice(b);
            v.extend_from_slice(post);
            v
        };
        let inputs = [
            wrap(b"[White \"", b"\"]\n\n1. e4 *\n"), // tag value (whole)
            wrap(b"[White \"a", b"b\"]\n\n1. e4 *\n"), // inside
            wrap(b"[White \"\\", b"\"]\n\n1. e4 *\n"), // after a backslash
            wrap(b"[White \"x", b"\n\n1. e4 *\n"),   // unterminated value
            wrap(b"[Wh", b"ite \"x\"]\n\n1. e4 *\n"), // tag name
            wrap(b"[White \"x\"] ", b"\n\n1. e4 *\n"), // between tag and next
            wrap(b"[Event \"x\"]\n\n1. e4 {", b"} *\n"), // comment, whole
            wrap(b"[Event \"x\"]\n\n1. e4 {a", b"b} *\n"), // comment, inside
            wrap(b"[Event \"x\"]\n\n1. e4 {a", b"\n"), // unterminated comment
            wrap(b"[Event \"x\"]\n\n1. e4 ;", b"\n e5 *\n"), // line comment
            wrap(b"[Event \"x\"]\n\n1. e4 ;a", b""), // line comment at EOF
            wrap(b"[Event \"x\"]\n\n1. ", b"e4 *\n"), // movetext word
            wrap(b"[Event \"x\"]\n\n1. e4 (", b") *\n"), // inside a variation
            wrap(b"[Event \"x\"]\n\n1. e4 $", b"1 *\n"), // NAG
            wrap(b"{", b"}\n[Event \"x\"]\n\n1. e4 *\n"), // between games
            wrap(b";", b"\n[Event \"x\"]\n\n1. e4 *\n"), // between games
            wrap(b"[Event \"x\"]\n\n1. e4 *\n", b""), // after the game
            wrap(b"[Event \"x\"]\n\n1. e4 *\n\n[Site \"", b""), // truncated at EOF
        ];
        for input in inputs {
            let games = read_all(&input);
            assert!(games.len() <= 4, "{input:?}");
            // Every Ok game has Latin-1 or UTF-8 text only (valid `String`s by construction).
            for g in games.into_iter().flatten() {
                let _ = canonical_form(&g);
            }
        }
    }
}

#[test]
fn every_single_byte_at_every_position_of_a_small_game_terminates() {
    let base = b"[Event \"E\"]\n[White \"W\"]\n\n1. e4 {c} e5 (1... c5 ; l\n) 2. Nf3 $1 *\n";
    for i in 0..base.len() {
        for b in [0x80u8, 0xc3, 0xe9, 0xff, 0x00, 0x1a] {
            let mut v = base.to_vec();
            v[i] = b;
            let _ = read_all(&v);
            let mut ins = base.to_vec();
            ins.insert(i, b);
            let _ = read_all(&ins);
        }
    }
}
