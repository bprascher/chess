//! C-R3 (chess-f6t): `canonical_form` v1 is frozen (B1, decided 2026-11-09). These tests pin
//! the bytes of v1 for a custom-start game, for a large deterministic corpus of legal games
//! (a digest taken from the code before the illegal-tail marker existed), for the accepted
//! control-character collapse, and the RF-05 (a) marker for an illegal mainline tail.
//! UT-RUL-026, D-42, OQ-RUL-005.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{
    CANONICAL_FORM_VERSION, Move, PgnDate, PgnGame, PgnNode, PgnResult, Position, Termination,
    TimeControl, canonical_form,
};
use common::{game, mv, nodes, pos};

fn text(g: &PgnGame) -> String {
    String::from_utf8(canonical_form(g)).unwrap()
}

// ------------------------------------------------------------------ custom-start golden

#[test]
fn c_r3_custom_start_golden_bytes_of_v1() {
    // FEN precedes SetUp (ASCII order, as the code writes them); TimeControl follows.
    let start = pos("4k3/8/8/8/8/8/4P3/4K3 w - - 0 7");
    let mut g = PgnGame::new(
        start.clone(),
        nodes(&start, &["e2e4", "e8d7"]),
        PgnResult::BlackWins,
    );
    g.tags.event = "Endgame".into();
    g.tags.date = PgnDate::new(Some(2026), None, None);
    g.tags.time_control = Some(TimeControl {
        seconds: 600,
        increment: None,
    });
    g.tags.termination = Some(Termination::Normal); // not part of v1
    let expected = "[Event \"Endgame\"]\n[Site \"?\"]\n[Date \"2026.??.??\"]\n[Round \"-\"]\n\
[White \"?\"]\n[Black \"?\"]\n[Result \"0-1\"]\n\
[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 7\"]\n[SetUp \"1\"]\n[TimeControl \"600\"]\n\n\
7. e4 Kd7 0-1\n";
    assert_eq!(text(&g), expected);
}

#[test]
fn c_r3_custom_start_with_black_to_move_numbers_the_first_move_with_dots() {
    let start = pos("4k3/8/8/8/8/8/4P3/4K3 b - - 3 12");
    let g = PgnGame::new(
        start.clone(),
        nodes(&start, &["e8d8", "e2e4"]),
        PgnResult::Unknown,
    );
    let t = text(&g);
    assert!(t.ends_with("[SetUp \"1\"]\n\n12... Kd8 13. e4 *\n"), "{t}");
}

// ------------------------------------------------------------------ control-character collapse

#[test]
fn c_r3_control_character_collapse_is_accepted_and_pinned() {
    // ASM-RUL-004 / accepted by Brian Prascher 2026-11-09: a tab (or any control character)
    // in a tag value is written as a space, so these two games have equal canonical bytes.
    let mut with_space = game(&["e2e4"], PgnResult::Unknown);
    with_space.tags.white = "A B".into();
    for control in ['\t', '\n', '\r', '\u{7}', '\u{85}'] {
        let mut g = game(&["e2e4"], PgnResult::Unknown);
        g.tags.white = format!("A{control}B");
        assert_eq!(
            canonical_form(&g),
            canonical_form(&with_space),
            "{control:?}"
        );
    }
    assert!(text(&with_space).contains("[White \"A B\"]\n"));
}

// ------------------------------------------------------------------ legal-game corpus digest

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }
}

const STARTS: [&str; 6] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1",
    "4k3/P6P/8/8/8/8/p6p/4K3 w - - 0 1",
    "4k3/8/8/8/8/8/4P3/4K3 b - - 0 40",
    "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
    "8/8/8/4k3/8/4K3/8/8 w - - 99 120",
];

const VALUES: [&str; 6] = [
    "?",
    "Club \"A\"",
    "back\\slash",
    "tab\there",
    "Zoë Müller",
    "",
];

fn corpus() -> Vec<PgnGame> {
    let mut rng = SplitMix(0x00C0_FFEE_2026_1109);
    let mut games = Vec::new();
    for i in 0..400usize {
        let start = pos(STARTS[i % STARTS.len()]);
        let mut cur = start.clone();
        let mut moves: Vec<PgnNode> = Vec::new();
        let plies = rng.below(90);
        for _ in 0..plies {
            let mut legal: Vec<Move> = cur.legal_moves();
            if legal.is_empty() {
                break;
            }
            legal.sort_by_key(Move::to_uci);
            let m = legal[rng.below(legal.len())];
            cur = cur.play(m).unwrap();
            let mut node = PgnNode::new(m);
            if rng.below(7) == 0 {
                node.comment = Some("note".into());
                node.nags = vec![1];
            }
            moves.push(node);
        }
        let result = [
            PgnResult::WhiteWins,
            PgnResult::BlackWins,
            PgnResult::Draw,
            PgnResult::Unknown,
        ][rng.below(4)];
        let mut g = PgnGame::new(start, moves, result);
        g.tags.event = VALUES[rng.below(VALUES.len())].into();
        g.tags.white = VALUES[rng.below(VALUES.len())].into();
        g.tags.round = VALUES[rng.below(VALUES.len())].into();
        if rng.below(2) == 0 {
            g.tags.date = PgnDate::new(
                Some(1900 + u16::try_from(rng.below(200)).unwrap()),
                Some(1 + u8::try_from(rng.below(12)).unwrap()),
                None,
            );
        }
        if rng.below(2) == 0 {
            g.tags.time_control = Some(TimeControl {
                seconds: u32::try_from(rng.below(7200)).unwrap(),
                increment: (rng.below(2) == 0).then(|| u32::try_from(rng.below(30)).unwrap()),
            });
        }
        games.push(g);
    }
    games
}

fn fnv1a(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
}

/// FNV-1a 64 over `length || bytes` of every game's canonical form, in corpus order.
fn corpus_digest() -> (u64, usize) {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let games = corpus();
    for g in &games {
        let bytes = canonical_form(g);
        fnv1a(&mut hash, &(bytes.len() as u64).to_le_bytes());
        fnv1a(&mut hash, &bytes);
    }
    (hash, games.len())
}

/// Taken from the code before the marker existed (see step1-rules-conditions-evidence.md).
/// A change of this value is a change of v1: stop, it is a G3 stored-format matter.
const CORPUS_DIGEST_V1: u64 = 0xb679_052c_1338_3ab7;

#[test]
fn c_r3_legal_game_corpus_bytes_are_those_of_v1() {
    assert_eq!(CANONICAL_FORM_VERSION, 1);
    let (digest, n) = corpus_digest();
    assert_eq!(n, 400);
    assert_eq!(digest, CORPUS_DIGEST_V1, "digest is {digest:#018x}");
}

// ------------------------------------------------------------------ RF-05 (a) marker

fn with_tail(prefix: &[&str], tail: &[&str]) -> PgnGame {
    // Built from nodes without replay checks so that the tail can be illegal.
    let start = Position::standard();
    let mut moves = nodes(&start, prefix);
    moves.extend(tail.iter().map(|u| PgnNode::new(mv(u))));
    PgnGame::new(start, moves, PgnResult::Unknown)
}

#[test]
fn c_r3_rf_05_an_illegal_tail_is_marked_not_dropped() {
    let g = with_tail(&["e2e4", "e7e5"], &["e1e5", "a1a2"]);
    let t = text(&g);
    assert!(t.ends_with("\n\n1. e4 e5 {illegal 2 e1e5 a1a2} *\n"), "{t}");
}

#[test]
fn c_r3_rf_05_games_that_differ_only_in_an_illegal_tail_no_longer_collide() {
    let a = with_tail(&["e2e4"], &["e1e5"]);
    let b = with_tail(&["e2e4"], &["e1e6"]);
    let c = with_tail(&["e2e4"], &["e1e5", "a1a2"]);
    let d = with_tail(&["e2e4"], &[]);
    let forms: Vec<Vec<u8>> = [&a, &b, &c, &d].iter().map(|g| canonical_form(g)).collect();
    for i in 0..forms.len() {
        for j in (i + 1)..forms.len() {
            assert_ne!(forms[i], forms[j], "{i} {j}");
        }
    }
    // The legal prefix alone is unchanged v1 text.
    assert!(text(&d).ends_with("\n\n1. e4 *\n"));
}

#[test]
fn c_r3_rf_05_the_marker_cannot_occur_in_the_movetext_line_of_a_legal_game() {
    // The true claim (RC-09): no `{` in the movetext line (after the blank line). A tag value
    // may contain `{` and is written unchanged on its tag line, so the whole output may.
    let mut g = corpus();
    let mut braced = game(&["e2e4", "e7e5"], PgnResult::Draw);
    braced.tags.event = "{illegal 1 e2e4}".into();
    braced.tags.white = "a { b".into();
    g.push(braced);
    for game in &g {
        let t = text(game);
        let (tags, movetext) = t.split_once("\n\n").unwrap();
        assert!(!movetext.contains('{'), "{movetext}");
        assert_eq!(movetext.lines().count(), 1, "{movetext}");
        assert!(!tags.contains("\n\n"));
    }
    // The tag value really is in the output: the claim is about the movetext line only.
    let t = text(g.last().unwrap());
    assert!(t.contains("[Event \"{illegal 1 e2e4}\"]"), "{t}");
    assert!(t.ends_with("\n\n1. e4 e5 1/2-1/2\n"), "{t}");
}

#[test]
fn c_r3_rf_05_an_illegal_first_move_for_black_and_a_promotion_in_the_tail() {
    let start = pos("4k3/8/8/8/8/8/4P3/4K3 b - - 0 9");
    let moves = vec![PgnNode::new(mv("a7a8q"))];
    let g = PgnGame::new(start, moves, PgnResult::Draw);
    let t = text(&g);
    assert!(t.ends_with("\n\n{illegal 0 a7a8q} 1/2-1/2\n"), "{t}");
}
