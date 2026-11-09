//! Rework tests for the adversarial review REV-chess-rules-code (task chess-a0r.2.11):
//! RF-01 (variation depth ceiling), RF-02 (stray terminator, trailing Ctrl-Z), RF-03 (tag
//! sections without movetext), RF-06 (`IllegalReason::Other` from ordinary input) and RF-05
//! part (the `canonical_form` version constant). Each test failed before the fix.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{
    CANONICAL_FORM_VERSION, IllegalReason, MatchedMove, Move, MoveParseError, ParseMode,
    PgnErrorKind, PgnGame, PgnLimits, PgnParseError, PgnReader, PieceKind, Position,
};
use common::{pos, sq};

fn read_all(text: &str, limits: PgnLimits) -> Vec<Result<PgnGame, PgnParseError>> {
    PgnReader::new(text.as_bytes(), limits).collect()
}

fn nested(depth: usize) -> String {
    format!("1. Nf3 {}{} *", "(Nf3 ".repeat(depth), ")".repeat(depth))
}

/// Runs `f` on a thread with a small stack, so that a recursion proportional to the nesting
/// depth aborts the test process instead of passing by luck on the 8 MiB main stack.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

// ------------------------------------------------------------------ RF-01

#[test]
fn rf_01_huge_depth_limit_is_clamped_and_does_not_overflow_the_stack() {
    // Before the fix: limit 200_001 on 200_000 nested variations parsed, and dropping the tree
    // overflowed the stack (SIGABRT).
    on_small_stack(|| {
        let text = nested(200_000);
        for limit in [200_001, usize::MAX] {
            let items = read_all(
                &text,
                PgnLimits {
                    max_variation_depth: limit,
                },
            );
            assert_eq!(items.len(), 1);
            let err = items[0].as_ref().unwrap_err();
            assert!(matches!(err.kind, PgnErrorKind::VariationTooDeep { .. }));
        }
    });
}

#[test]
fn rf_01_the_ceiling_is_256_and_the_error_reports_the_effective_limit() {
    on_small_stack(|| {
        let at_ceiling = read_all(
            &nested(256),
            PgnLimits {
                max_variation_depth: usize::MAX,
            },
        );
        assert!(at_ceiling[0].is_ok(), "{:?}", at_ceiling[0]);
        let over = read_all(
            &nested(257),
            PgnLimits {
                max_variation_depth: usize::MAX,
            },
        );
        match &over[0].as_ref().unwrap_err().kind {
            PgnErrorKind::VariationTooDeep { limit } => assert_eq!(*limit, 256),
            other => panic!("{other:?}"),
        }
        // A limit below the ceiling is honoured as given.
        let small = read_all(
            &nested(5),
            PgnLimits {
                max_variation_depth: 4,
            },
        );
        match &small[0].as_ref().unwrap_err().kind {
            PgnErrorKind::VariationTooDeep { limit } => assert_eq!(*limit, 4),
            other => panic!("{other:?}"),
        }
    });
}

// ------------------------------------------------------------------ RF-02

#[test]
fn rf_02_a_stray_terminator_after_a_game_is_not_a_game() {
    for tail in ["*", "1-0", "0-1", "1/2-1/2"] {
        let text = format!("[Event \"a\"]\n\n1. e4 e5 *\n\n{tail}\n");
        let items = read_all(&text, PgnLimits::default());
        assert_eq!(items.len(), 1, "{tail}: {items:?}");
        assert_eq!(items[0].as_ref().unwrap().moves.len(), 2);
    }
}

#[test]
fn rf_02_a_lone_terminator_yields_no_game() {
    for text in ["*", "  1-0 \n", "\n\n*\n*\n"] {
        assert!(read_all(text, PgnLimits::default()).is_empty(), "{text:?}");
    }
}

#[test]
fn rf_02_a_tagged_game_with_only_a_terminator_is_still_a_game() {
    let items = read_all("[Event \"x\"]\n\n*\n", PgnLimits::default());
    assert_eq!(items.len(), 1);
    assert!(items[0].as_ref().unwrap().moves.is_empty());
}

#[test]
fn rf_02_trailing_ctrl_z_is_end_of_input() {
    for text in [
        "[Event \"a\"]\n\n1. e4 e5 *\n\x1a",
        "[Event \"a\"]\n\n1. e4 e5 *\n\x1a\n",
        "[Event \"a\"]\n\n1. e4 e5\n\x1a",
        "[Event \"a\"]\n\n1. e4 e5 *\n\x1a\x1a",
    ] {
        let items = read_all(text, PgnLimits::default());
        assert_eq!(items.len(), 1, "{text:?}: {items:?}");
        assert!(items[0].is_ok(), "{text:?}: {items:?}");
    }
}

#[test]
fn rf_02_ctrl_z_inside_the_text_is_still_an_error() {
    let items = read_all("[Event \"a\"]\n\n1. e4 \x1a e5 *\n", PgnLimits::default());
    assert_eq!(items.len(), 1);
    assert!(items[0].is_err());
}

// ------------------------------------------------------------------ RF-03

#[test]
fn rf_03_consecutive_tag_sections_do_not_merge() {
    let text = "[Event \"A\"]\n[White \"w1\"]\n\n[Event \"B\"]\n[White \"w2\"]\n\n1. e4 e5 *\n";
    let items = read_all(text, PgnLimits::default());
    assert_eq!(items.len(), 2, "{items:?}");
    let first = items[0].as_ref().unwrap_err();
    assert_eq!(first.game_index, 1);
    assert_eq!(first.line, 4);
    assert!(matches!(first.kind, PgnErrorKind::BadSyntax { .. }));
    let second = items[1].as_ref().unwrap();
    assert_eq!(second.tags.event, "B");
    assert_eq!(second.tags.white, "w2");
    assert_eq!(second.moves.len(), 2);
}

#[test]
fn rf_03_three_tag_only_sections_give_two_errors_and_the_game() {
    let text = "[Event \"A\"]\n\n[Event \"B\"]\n\n[Event \"C\"]\n\n1. d4 *\n";
    let items = read_all(text, PgnLimits::default());
    assert_eq!(items.len(), 3, "{items:?}");
    assert!(items[0].is_err());
    assert!(items[1].is_err());
    assert_eq!(items[2].as_ref().unwrap().tags.event, "C");
}

#[test]
fn rf_03_tags_without_a_blank_line_between_them_stay_one_section() {
    let text = "[Event \"A\"]\n[White \"w\"]\n[Black \"b\"]\n\n1. e4 *\n";
    let items = read_all(text, PgnLimits::default());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].as_ref().unwrap().tags.black, "b");
}

// ------------------------------------------------------------------ RF-06

fn typed(p: &Position, text: &str) -> Result<MatchedMove, MoveParseError> {
    p.parse_move(text, ParseMode::Typed)
}

#[test]
fn rf_06_a_promotion_letter_on_a_non_promoting_move_is_not_other() {
    let p = Position::standard();
    let play = p.play(Move {
        from: sq("e2"),
        to: sq("e4"),
        promotion: Some(PieceKind::Queen),
    });
    // C-R5 (RF-06 note): its own reason, not `CannotReach { Pawn }`.
    assert_eq!(play.unwrap_err().reason, IllegalReason::NotAPromotion);
    match typed(&p, "e2e4q") {
        Err(MoveParseError::NoMatch { reason }) => {
            assert_eq!(reason, IllegalReason::NotAPromotion);
        }
        other => panic!("{other:?}"),
    }
    // A knight move and a promotion letter of any kind are the same wrong request.
    for piece in [PieceKind::Queen, PieceKind::King, PieceKind::Pawn] {
        assert_eq!(
            p.explain_illegal(sq("g1"), sq("f3"), Some(piece)),
            IllegalReason::NotAPromotion
        );
    }
    // Impossible geometry is still reported as geometry, not as a promotion problem.
    assert_eq!(
        p.explain_illegal(sq("e2"), sq("e5"), Some(PieceKind::Queen)),
        IllegalReason::CannotReach {
            piece: PieceKind::Pawn
        }
    );
}

#[test]
fn rf_06_promotion_to_king_or_pawn_is_not_other() {
    let q = pos("8/P6k/8/8/8/8/7K/8 w - - 0 1");
    for piece in [PieceKind::King, PieceKind::Pawn] {
        let reason = q.explain_illegal(sq("a7"), sq("a8"), Some(piece));
        assert_eq!(reason, IllegalReason::CannotReach { piece });
    }
}

#[test]
fn rf_06_no_ordinary_malformed_move_text_reports_other() {
    let p = Position::standard();
    for text in ["e2e4q", "e4q", "e2e4k", "e2e5q", "g1f3n", "e7e8q", "a1a2r"] {
        if let Err(MoveParseError::NoMatch { reason }) = typed(&p, text) {
            assert_ne!(reason, IllegalReason::Other, "{text}");
        }
    }
}

// ------------------------------------------------------------------ RF-05 (part)

#[test]
fn rf_05_canonical_form_version_constant_is_1() {
    assert_eq!(CANONICAL_FORM_VERSION, 1);
}

#[test]
fn rf_06_sweep_of_the_start_position_never_yields_other_for_an_illegal_probe() {
    let p = Position::standard();
    let squares: Vec<_> = (0u8..64)
        .map(|i| {
            let name = format!("{}{}", char::from(b'a' + i % 8), 1 + i / 8);
            sq(&name)
        })
        .collect();
    for &from in &squares {
        for &to in &squares {
            for promotion in [None, Some(PieceKind::Queen), Some(PieceKind::King)] {
                let legal = p.is_legal(Move {
                    from,
                    to,
                    promotion,
                });
                let reason = p.explain_illegal(from, to, promotion);
                if !legal {
                    assert_ne!(reason, IllegalReason::Other, "{from:?}{to:?}{promotion:?}");
                }
            }
        }
    }
}
