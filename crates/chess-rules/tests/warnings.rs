//! B6 / C-R5 (chess-pdc), desk review R7, RF-04: a reader item carries the list of lossy
//! import cases it hit (`PgnGame::warnings`). Warnings never change `canonical_form`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{
    ImportWarning, PgnDate, PgnGame, PgnLimits, PgnReader, PgnResult, canonical_form, write_pgn,
};
use common::game;

fn read(bytes: &[u8]) -> PgnGame {
    let mut all: Vec<_> = PgnReader::new(bytes, PgnLimits::default()).collect();
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0).unwrap()
}

fn warnings(text: &str) -> Vec<ImportWarning> {
    read(text.as_bytes()).warnings
}

#[test]
fn b6_a_game_built_in_code_and_a_clean_import_have_no_warnings() {
    assert!(game(&["e2e4"], PgnResult::Unknown).warnings.is_empty());
    assert!(warnings("[Event \"x\"]\n[Date \"2026.11.05\"]\n[TimeControl \"300+2\"]\n[Termination \"normal\"]\n\n1. e4 {fine} e5 *\n").is_empty());
}

#[test]
fn b6_a_comment_before_the_first_move_is_reported_with_its_position() {
    assert_eq!(
        warnings("[Event \"x\"]\n\n{ opening remark } 1. e4 e5 *\n"),
        vec![ImportWarning::CommentDropped { line: 3, col: 1 }]
    );
    // a semicolon comment before the first move
    assert_eq!(
        warnings("[Event \"x\"]\n\n; remark\n1. e4 *\n"),
        vec![ImportWarning::CommentDropped { line: 3, col: 1 }]
    );
    // a game without tags: the comment is read before the game starts and is reported on it
    assert_eq!(
        warnings("{ opening remark } 1. e4 e5 *\n"),
        vec![ImportWarning::CommentDropped { line: 1, col: 1 }]
    );
    assert_eq!(
        warnings("; remark\n1. e4 *\n"),
        vec![ImportWarning::CommentDropped { line: 1, col: 1 }]
    );
}

#[test]
fn b6_a_comment_right_after_an_opening_parenthesis_is_reported() {
    assert_eq!(
        warnings("1. e4 (1. d4 {why} d5) e5 *\n"),
        Vec::<ImportWarning>::new()
    );
    assert_eq!(
        warnings("1. e4 ( {first} 1. d4 d5) e5 *\n"),
        vec![ImportWarning::CommentDropped { line: 1, col: 9 }]
    );
}

#[test]
fn b6_an_empty_comment_before_the_first_move_loses_nothing() {
    assert!(warnings("{ } 1. e4 *\n").is_empty());
}

#[test]
fn b6_unparseable_date_parts_are_reported_with_the_tag_text() {
    for (value, lossy) in [
        ("2024.13.45", true),
        ("2024.7.5", true),
        ("garbage", true),
        ("2024.??.??", false),
        ("????.??.??", false),
        ("2026.11.05", false),
    ] {
        let got = warnings(&format!("[Date \"{value}\"]\n\n1. e4 *\n"));
        let want = if lossy {
            vec![ImportWarning::DateIgnored {
                value: value.to_string(),
            }]
        } else {
            vec![]
        };
        assert_eq!(got, want, "{value}");
    }
}

#[test]
fn b6_unparseable_time_control_is_reported_but_dash_and_question_mark_are_not() {
    assert_eq!(
        warnings("[TimeControl \"40/9000:3600\"]\n\n1. e4 *\n"),
        vec![ImportWarning::TimeControlIgnored {
            value: "40/9000:3600".into()
        }]
    );
    assert!(warnings("[TimeControl \"300+2\"]\n\n1. e4 *\n").is_empty());
    assert!(warnings("[TimeControl \"-\"]\n\n1. e4 *\n").is_empty());
    assert!(warnings("[TimeControl \"?\"]\n\n1. e4 *\n").is_empty());
}

#[test]
fn b6_a_termination_outside_the_written_values_is_reported() {
    assert_eq!(
        warnings("[Termination \"rules infraction\"]\n\n1. e4 *\n"),
        vec![ImportWarning::TerminationIgnored {
            value: "rules infraction".into()
        }]
    );
    assert!(warnings("[Termination \"Time forfeit\"]\n\n1. e4 *\n").is_empty());
}

#[test]
fn b6_latin_1_fallback_is_reported_once_per_fallback_payload() {
    let mut bytes = b"[White \"Caf\xe9\"]\n[Black \"ok\"]\n\n1. e4 {caf\xe9} e5 *\n".to_vec();
    let g = read(&bytes);
    // The White tag value and the comment each fell back (chess-n3h).
    assert_eq!(g.warnings, vec![ImportWarning::Latin1Fallback; 2]);
    assert_eq!(g.tags.white, "Caf\u{e9}");
    // Valid UTF-8 is not a fallback.
    bytes = "[White \"Caf\u{e9}\"]\n\n1. e4 *\n".as_bytes().to_vec();
    assert!(read(&bytes).warnings.is_empty());
}

#[test]
fn b6_warnings_arrive_in_the_order_they_were_met_and_only_for_their_own_game() {
    let text =
        "[Date \"bad\"]\n[TimeControl \"x\"]\n\n{c} 1. e4 *\n\n[Event \"clean\"]\n\n1. d4 *\n";
    let games: Vec<_> = PgnReader::new(text.as_bytes(), PgnLimits::default())
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        games[0].warnings,
        vec![
            ImportWarning::DateIgnored {
                value: "bad".into()
            },
            ImportWarning::TimeControlIgnored { value: "x".into() },
            ImportWarning::CommentDropped { line: 4, col: 1 },
        ]
    );
    assert!(games[1].warnings.is_empty());
}

#[test]
fn b6_warnings_never_change_canonical_form_or_the_written_pgn() {
    let lossy = read(
        b"[Date \"2024.13.45\"]\n[TimeControl \"junk\"]\n[Termination \"x\"]\n\n{ remark } 1. e4 e5 *\n",
    );
    assert!(lossy.warnings.len() >= 4);
    let mut clean = game(&["e2e4", "e7e5"], PgnResult::Unknown);
    // The lossy tags read as unknown / absent (the date keeps its valid year): the same game
    // built without them has the same bytes.
    clean.tags.date = PgnDate::new(Some(2024), None, None);
    assert_eq!(canonical_form(&lossy), canonical_form(&clean));
    assert_eq!(write_pgn(&lossy).unwrap(), write_pgn(&clean).unwrap());
    clean.warnings = lossy.warnings.clone();
    assert_eq!(canonical_form(&clean), canonical_form(&lossy));
}

#[test]
fn b6_warnings_have_messages_that_name_the_loss() {
    for w in [
        ImportWarning::CommentDropped { line: 1, col: 2 },
        ImportWarning::DateIgnored { value: "x".into() },
        ImportWarning::TimeControlIgnored { value: "x".into() },
        ImportWarning::TerminationIgnored { value: "x".into() },
        ImportWarning::Latin1Fallback,
    ] {
        let m = w.to_string();
        assert!(m.len() > 10, "{m}");
    }
}

#[test]
fn b6_a_comment_between_two_games_is_reported_on_the_game_after_it() {
    let text = "1. e4 *\n\n{ between } \n1. d4 *\n";
    let games: Vec<_> = PgnReader::new(text.as_bytes(), PgnLimits::default())
        .map(Result::unwrap)
        .collect();
    assert_eq!(games.len(), 2);
    assert!(games[0].warnings.is_empty());
    assert_eq!(
        games[1].warnings,
        vec![ImportWarning::CommentDropped { line: 3, col: 1 }]
    );
    // A comment after the last game belongs to no game and is not reported anywhere.
    let tail = "1. e4 *\n{ the end }\n";
    let games: Vec<_> = PgnReader::new(tail.as_bytes(), PgnLimits::default())
        .map(Result::unwrap)
        .collect();
    assert_eq!(games.len(), 1);
    assert!(games[0].warnings.is_empty());
}

#[test]
fn b6_latin_1_is_decided_per_payload_so_valid_utf_8_in_the_same_game_is_kept() {
    // chess-n3h (Brian Prascher, 2026-11-09): replaces the SUSPECT whole-game characterization
    // of finding C-R5-F1. White is valid UTF-8 "Zo\u{eb}"; Black holds a Latin-1 byte.
    let g = read(b"[White \"Zo\xc3\xab\"]\n[Black \"Caf\xe9\"]\n\n1. e4 *\n");
    assert_eq!(g.warnings, vec![ImportWarning::Latin1Fallback]);
    assert_eq!(g.tags.black, "Caf\u{e9}");
    assert_eq!(g.tags.white, "Zo\u{eb}");
}
