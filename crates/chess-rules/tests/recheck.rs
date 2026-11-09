//! Fixes for the Reviewer re-check REV-step1-recheck (task chess-k5l): RC-01 (a promotion
//! letter on a castling move is not `Other`), RC-03 (bounded import warning list) and RC-13
//! (a replaced tag leaves no stale warning). Each test failed before its fix.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{
    IllegalReason, ImportWarning, MAX_IMPORT_WARNINGS_PER_GAME, MatchedMove, Move, MoveParseError,
    ParseMode, PgnGame, PgnLimits, PgnReader, PieceKind, Position, canonical_form, write_pgn,
};
use common::{pos, sq};

const LETTERS: [Option<PieceKind>; 7] = [
    None,
    Some(PieceKind::Queen),
    Some(PieceKind::Rook),
    Some(PieceKind::Bishop),
    Some(PieceKind::Knight),
    Some(PieceKind::King),
    Some(PieceKind::Pawn),
];

fn typed(p: &Position, text: &str) -> Result<MatchedMove, MoveParseError> {
    p.parse_move(text, ParseMode::Typed)
}

/// Counts the probes (all square pairs, all promotion options) that are illegal and explained
/// as `Other`.
fn other_count(p: &Position) -> (usize, usize) {
    let squares: Vec<_> = (0u8..64)
        .map(|i| sq(&format!("{}{}", char::from(b'a' + i % 8), 1 + i / 8)))
        .collect();
    let (mut probes, mut others) = (0, 0);
    for &from in &squares {
        for &to in &squares {
            for promotion in LETTERS {
                let mv = Move {
                    from,
                    to,
                    promotion,
                };
                if !p.is_legal(mv) {
                    probes += 1;
                    if p.explain_illegal(from, to, promotion) == IllegalReason::Other {
                        others += 1;
                    }
                }
            }
        }
    }
    (probes, others)
}

// ------------------------------------------------------------------ RC-01

#[test]
fn rc_01_a_promotion_letter_on_a_legal_castling_move_is_not_a_promotion() {
    let w = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    assert!(w.is_legal(Move {
        from: sq("e1"),
        to: sq("g1"),
        promotion: None
    }));
    for text in ["e1g1q", "e1c1n", "e1g1r", "e1c1b"] {
        match typed(&w, text) {
            Err(MoveParseError::NoMatch { reason }) => {
                assert_eq!(reason, IllegalReason::NotAPromotion, "{text}");
            }
            other => panic!("{text}: {other:?}"),
        }
    }
    let b = pos("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
    for text in ["e8g8q", "e8c8n"] {
        match typed(&b, text) {
            Err(MoveParseError::NoMatch { reason }) => {
                assert_eq!(reason, IllegalReason::NotAPromotion, "{text}");
            }
            other => panic!("{text}: {other:?}"),
        }
    }
    // The plain castling move is still accepted.
    assert!(typed(&w, "e1g1").is_ok());
}

#[test]
fn rc_01_a_castling_failure_with_a_letter_keeps_its_castling_reason() {
    // The castling reason is more useful than "not a promotion" when castling itself fails.
    let p = pos("r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1");
    assert_eq!(
        p.explain_illegal(sq("e1"), sq("g1"), Some(PieceKind::Queen)),
        IllegalReason::CastlingRightsLost
    );
}

#[test]
fn rc_01_sweep_no_illegal_probe_is_other_in_castling_positions() {
    let fens = [
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
        "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
        "r3k2r/8/8/8/8/8/8/R3K2R w Kk - 0 1",
        "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1",
        "r3k2r/8/8/8/8/8/8/R3K1NR w KQkq - 0 1",
        "r3k2r/8/8/8/4r3/8/8/R3K2R w KQkq - 0 1",
        "r3k2r/8/8/8/5r2/8/8/R3K2R w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
        "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1",
    ];
    let mut total = 0;
    for fen in fens {
        let (probes, others) = other_count(&pos(fen));
        assert_eq!(others, 0, "{fen}");
        total += probes;
    }
    // 4096 pairs x 7 options per position, minus the legal ones.
    assert!(total > 250_000, "{total}");
}

// ------------------------------------------------------------------ RC-03

fn read_one(bytes: &[u8]) -> PgnGame {
    let mut all: Vec<_> = PgnReader::new(bytes, PgnLimits::default()).collect();
    assert_eq!(all.len(), 1, "{:?}", all.len());
    all.remove(0).unwrap()
}

fn assert_capped(g: &PgnGame, total: usize) {
    assert_eq!(g.warnings.len(), MAX_IMPORT_WARNINGS_PER_GAME + 1);
    assert_eq!(
        g.warnings.last(),
        Some(&ImportWarning::WarningsTruncated {
            dropped: total - MAX_IMPORT_WARNINGS_PER_GAME
        })
    );
    // Only the final entry is the marker.
    assert!(
        g.warnings[..MAX_IMPORT_WARNINGS_PER_GAME]
            .iter()
            .all(|w| matches!(w, ImportWarning::CommentDropped { .. }))
    );
}

#[test]
fn rc_03_comments_between_games_cannot_grow_the_list_past_the_cap() {
    let n = 200_000;
    let noisy = format!("{}1. e4 e5 *\n", "{a}".repeat(n));
    let g = read_one(noisy.as_bytes());
    assert_capped(&g, n);
    // The kept warnings are the first ones, in order.
    assert_eq!(
        g.warnings[0],
        ImportWarning::CommentDropped { line: 1, col: 1 }
    );
    assert_eq!(
        g.warnings[1],
        ImportWarning::CommentDropped { line: 1, col: 4 }
    );
    // Warnings never change the canonical form or the written PGN.
    let clean = read_one(b"1. e4 e5 *\n");
    assert!(clean.warnings.is_empty());
    assert_eq!(canonical_form(&g), canonical_form(&clean));
    assert_eq!(write_pgn(&g).unwrap(), write_pgn(&clean).unwrap());
}

#[test]
fn rc_03_comments_before_the_first_move_of_a_tagged_game_are_capped_too() {
    let n = 50_000;
    let noisy = format!("[Event \"x\"]\n\n{}1. e4 e5 *\n", "{a}".repeat(n));
    let g = read_one(noisy.as_bytes());
    assert_capped(&g, n);
    let clean = read_one(b"[Event \"x\"]\n\n1. e4 e5 *\n");
    assert_eq!(canonical_form(&g), canonical_form(&clean));
    assert_eq!(write_pgn(&g).unwrap(), write_pgn(&clean).unwrap());
}

#[test]
fn rc_03_the_cap_is_per_game_and_the_count_covers_every_dropped_warning() {
    let n = MAX_IMPORT_WARNINGS_PER_GAME + 10;
    let one = format!("{}1. e4 *\n", "{a}".repeat(n));
    let text = format!("{one}{one}[Event \"clean\"]\n\n1. d4 *\n");
    let all: Vec<_> = PgnReader::new(text.as_bytes(), PgnLimits::default())
        .map(Result::unwrap)
        .collect();
    assert_eq!(all.len(), 3);
    assert_capped(&all[0], n);
    assert_capped(&all[1], n);
    assert!(all[2].warnings.is_empty());
}

#[test]
fn rc_03_at_the_cap_there_is_no_marker() {
    let n = MAX_IMPORT_WARNINGS_PER_GAME;
    let text = format!("{}1. e4 *\n", "{a}".repeat(n));
    let g = read_one(text.as_bytes());
    assert_eq!(g.warnings.len(), n);
    assert!(
        g.warnings
            .iter()
            .all(|w| matches!(w, ImportWarning::CommentDropped { .. }))
    );
}

#[test]
fn rc_03_comment_only_input_yields_no_game_and_does_not_fail() {
    let text = "{a}".repeat(300_000);
    let all: Vec<_> = PgnReader::new(text.as_bytes(), PgnLimits::default()).collect();
    assert!(all.is_empty());
}

#[test]
fn rc_03_repeated_typed_tags_keep_at_most_one_warning_each() {
    let mut text = String::new();
    for i in 0..5_000 {
        text.push_str(&format!(
            "[Date \"bad{i}\"]\n[TimeControl \"x{i}\"]\n[Termination \"t{i}\"]\n"
        ));
    }
    text.push_str("\n1. e4 *\n");
    let g = read_one(text.as_bytes());
    assert_eq!(
        g.warnings,
        vec![
            ImportWarning::DateIgnored {
                value: "bad4999".into()
            },
            ImportWarning::TimeControlIgnored {
                value: "x4999".into()
            },
            ImportWarning::TerminationIgnored {
                value: "t4999".into()
            },
        ]
    );
}

#[test]
fn rc_03_the_truncation_marker_has_a_message() {
    let w = ImportWarning::WarningsTruncated { dropped: 7 };
    assert!(w.to_string().contains('7'));
}

// ------------------------------------------------------------------ RC-13

#[test]
fn rc_13_a_later_valid_tag_removes_the_warning_of_the_earlier_bad_one() {
    let g = read_one(b"[Date \"2024.13.05\"]\n[Date \"2024.01.05\"]\n\n1. e4 *\n");
    assert!(g.warnings.is_empty(), "{:?}", g.warnings);
    assert_eq!(g.tags.date.to_string(), "2024.01.05");
    let g = read_one(b"[TimeControl \"x\"]\n[TimeControl \"300+2\"]\n\n1. e4 *\n");
    assert!(g.warnings.is_empty(), "{:?}", g.warnings);
    assert!(g.tags.time_control.is_some());
    let g = read_one(b"[Termination \"x\"]\n[Termination \"abandoned\"]\n\n1. e4 *\n");
    assert!(g.warnings.is_empty(), "{:?}", g.warnings);
}

#[test]
fn rc_13_a_later_bad_tag_replaces_the_earlier_bad_warning() {
    let g = read_one(b"[Date \"2024.13.05\"]\n[Date \"junk\"]\n\n1. e4 *\n");
    assert_eq!(
        g.warnings,
        vec![ImportWarning::DateIgnored {
            value: "junk".into()
        }]
    );
}
