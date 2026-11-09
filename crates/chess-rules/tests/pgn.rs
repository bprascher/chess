//! Part B PGN tests: writer (UT-RUL-017, 019), round trip (UT-RUL-018), reader (UT-RUL-020,
//! 021), line text (UT-RUL-022), `eco_tag` (UT-RUL-029), `canonical_form` (UT-RUL-026, 031,
//! CT-RUL-004), error messages (UT-RUL-024), `Send + Sync` (UT-RUL-025) and the random-input
//! test FZ-RUL-003. Requirements IF-1, IF-2, RULE-6, RULE-7, ANA-2, DAT-3, D-42, NFR-3, NFR-9.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{
    PgnDate, PgnErrorKind, PgnGame, PgnLimits, PgnNode, PgnParseError, PgnReader, PgnResult,
    PgnWriteError, Position, Termination, TimeControl, canonical_form, format_line, parse_line,
    write_pgn, write_pgn_many,
};
use common::{START_FEN, game, mv, nodes, pos};
use proptest::prelude::*;

fn read_all(bytes: &[u8]) -> Vec<Result<PgnGame, PgnParseError>> {
    PgnReader::new(bytes, PgnLimits::default()).collect()
}

fn read_one(text: &str) -> PgnGame {
    let mut all = read_all(text.as_bytes());
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0).unwrap()
}

fn read_err(text: &str) -> PgnParseError {
    let mut all = read_all(text.as_bytes());
    all.remove(0).unwrap_err()
}

fn ucis(g: &PgnGame) -> Vec<String> {
    g.moves.iter().map(|n| n.mv.to_uci()).collect()
}

// ------------------------------------------------------------------ UT-RUL-017 writer golden

#[test]
fn ut_rul_017_if_1_writer_golden_seven_tag_roster_extras_and_movetext() {
    let mut g = game(
        &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6"],
        PgnResult::WhiteWins,
    );
    g.tags.event = "Test Event".into();
    g.tags.site = "Local".into();
    g.tags.date = PgnDate::new(Some(2026), Some(11), Some(5));
    g.tags.round = "1".into();
    g.tags.white = "Alice".into();
    g.tags.black = "Bob".into();
    g.tags.time_control = Some(TimeControl {
        seconds: 300,
        increment: Some(2),
    });
    g.tags.termination = Some(Termination::Normal);
    g.tags.extras.insert("Zzz".into(), "last".into());
    g.tags.extras.insert("Annotator".into(), "me".into());
    g.tags.extras.insert("ECO".into(), "C60".into());
    let expected = "[Event \"Test Event\"]\n[Site \"Local\"]\n[Date \"2026.11.05\"]\n[Round \"1\"]\n\
[White \"Alice\"]\n[Black \"Bob\"]\n[Result \"1-0\"]\n[Annotator \"me\"]\n[Termination \"normal\"]\n\
[TimeControl \"300+2\"]\n[Zzz \"last\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 1-0\n";
    assert_eq!(
        write_pgn(&g).unwrap(),
        expected,
        "ECO is never written (Q-SE-7)"
    );
}

#[test]
fn ut_rul_017_if_1_writer_defaults_and_time_control_without_increment() {
    let mut g = game(&[], PgnResult::Unknown);
    g.tags.time_control = Some(TimeControl {
        seconds: 600,
        increment: None,
    });
    let expected = "[Event \"?\"]\n[Site \"?\"]\n[Date \"????.??.??\"]\n[Round \"-\"]\n[White \"?\"]\n\
[Black \"?\"]\n[Result \"*\"]\n[TimeControl \"600\"]\n\n*\n";
    assert_eq!(write_pgn(&g).unwrap(), expected);
}

#[test]
fn ut_rul_017_if_1_writer_emits_set_up_and_fen_for_a_custom_start_in_ascii_order() {
    let start = pos("4k3/8/8/8/8/8/4P3/4K3 w - - 0 7");
    let moves = nodes(&start, &["e2e4", "e8d8"]);
    let g = PgnGame::new(start, moves, PgnResult::Draw);
    let text = write_pgn(&g).unwrap();
    assert!(
        text.contains("[Result \"1/2-1/2\"]\n[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 7\"]\n[SetUp \"1\"]\n\n7. e4 Kd8 1/2-1/2\n"),
        "{text}"
    );
    // Black to move first: "N..." on the first move.
    let start_b = pos("4k3/8/8/8/8/8/4P3/4K3 b - - 0 7");
    let moves_b = nodes(&start_b, &["e8d8"]);
    let gb = PgnGame::new(start_b, moves_b, PgnResult::Unknown);
    assert!(write_pgn(&gb).unwrap().ends_with("\n\n7... Kd8 *\n"));
}

#[test]
fn ut_rul_017_plat_1_writer_numbers_after_comments_and_variations_and_orders_tokens() {
    let start = Position::standard();
    let mut moves = parse_line(&start, "1. e4 e5 2. Nf3 Nc6 3. Bb5").unwrap();
    moves[0].comment = Some("best by test".into());
    moves[1].variations =
        vec![parse_line(&start.play(mv("e2e4")).unwrap(), "1... c5 2. Nf3").unwrap()];
    moves[4].nags = vec![1];
    let g = PgnGame::new(start, moves, PgnResult::Unknown);
    let text = write_pgn(&g).unwrap();
    assert!(
        text.ends_with(
            "\n\n1. e4 {best by test} 1... e5 (1... c5 2. Nf3) 2. Nf3 Nc6 3. Bb5 $1 *\n"
        ),
        "{text}"
    );
}

#[test]
fn ut_rul_017_if_1_writer_lines_are_under_80_characters_and_the_text_is_unchanged_by_wrapping() {
    // A long game: shuffle knights, 60 plies.
    let mut ucis_list = Vec::new();
    for _ in 0..15 {
        ucis_list.extend(["g1f3", "g8f6", "f3g1", "f6g8"]);
    }
    let g = game(&ucis_list, PgnResult::Draw);
    let text = write_pgn(&g).unwrap();
    let movetext = text.split("\n\n").nth(1).unwrap();
    assert!(movetext.ends_with("1/2-1/2\n"));
    for line in movetext.lines() {
        assert!(line.chars().count() <= 79, "{line:?}");
    }
    assert!(movetext.lines().count() > 1);
    let one_line = movetext.replace('\n', " ");
    assert!(one_line.starts_with("1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3"));
    assert_eq!(
        text.matches("\n\n").count(),
        1,
        "one blank line after the tags"
    );
    assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
}

#[test]
fn ut_rul_017_plat_1_writer_is_deterministic() {
    let g = game(&["e2e4", "e7e5"], PgnResult::Unknown);
    assert_eq!(write_pgn(&g).unwrap(), write_pgn(&g.clone()).unwrap());
}

#[test]
fn ut_rul_017_if_1_write_pgn_many_separates_games_with_one_blank_line() {
    let a = game(&["e2e4"], PgnResult::Unknown);
    let b = game(&["d2d4"], PgnResult::Unknown);
    let text = write_pgn_many(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(
        text,
        format!("{}\n{}", write_pgn(&a).unwrap(), write_pgn(&b).unwrap())
    );
    assert_eq!(write_pgn_many(&[]).unwrap(), "");
}

#[test]
fn ut_rul_017_if_1_writer_errors_never_produce_partial_output() {
    let start = Position::standard();
    let mut bad = game(&["e2e4"], PgnResult::Unknown);
    bad.moves.push(PgnNode::new(mv("e2e4"))); // illegal after e4
    assert_eq!(
        write_pgn(&bad),
        Err(PgnWriteError::IllegalMove {
            variation_path: vec![],
            ply: 1
        })
    );
    // Illegal move inside a variation: path says where.
    let mut g = game(&["e2e4", "e7e5"], PgnResult::Unknown);
    g.moves[1].variations = vec![vec![PgnNode::new(mv("e7e4"))]];
    assert_eq!(
        write_pgn(&g),
        Err(PgnWriteError::IllegalMove {
            variation_path: vec![(1, 0)],
            ply: 0
        })
    );
    let mut t = game(&[], PgnResult::Unknown);
    t.tags.extras.insert("lower".into(), "x".into());
    assert!(matches!(
        write_pgn(&t),
        Err(PgnWriteError::BadTagName { .. })
    ));
    let mut t = game(&[], PgnResult::Unknown);
    t.tags.extras.insert("Result".into(), "x".into());
    assert!(matches!(
        write_pgn(&t),
        Err(PgnWriteError::ReservedTag { .. })
    ));
    for reserved in ["Event", "FEN", "SetUp", "TimeControl", "Termination"] {
        let mut t = game(&[], PgnResult::Unknown);
        t.tags.extras.insert(reserved.into(), "x".into());
        assert!(
            matches!(write_pgn(&t), Err(PgnWriteError::ReservedTag { .. })),
            "{reserved}"
        );
    }
    // The first failing game is named (1-based).
    let ok = game(&["e2e4"], PgnResult::Unknown);
    match write_pgn_many(&[ok, bad]) {
        Err(PgnWriteError::InGame { game_index, .. }) => assert_eq!(game_index, 2),
        other => panic!("{other:?}"),
    }
    // Over-deep nesting is an error, not a stack overflow.
    let mut deep = vec![PgnNode::new(mv("e2e4"))];
    for _ in 0..200 {
        let mut n = PgnNode::new(mv("e2e4"));
        n.variations = vec![deep];
        deep = vec![n];
    }
    let g = PgnGame::new(start, deep, PgnResult::Unknown);
    assert!(matches!(
        write_pgn(&g),
        Err(PgnWriteError::VariationTooDeep { .. })
    ));
}

// ------------------------------------------------------------------ UT-RUL-019

#[test]
fn ut_rul_019_if_1_names_with_quotes_backslashes_and_non_latin_1_text_survive() {
    let mut g = game(&["e2e4"], PgnResult::Unknown);
    g.tags.white = "Zo\u{eb} \"Z\" O\\x \u{65e5}\u{672c}".into();
    g.tags.black = "\u{1f600} \u{43f}\u{440}".into();
    let text = write_pgn(&g).unwrap();
    assert!(text.contains(r#"[White "Zoë \"Z\" O\\x 日本"]"#), "{text}");
    let back = read_one(&text);
    assert_eq!(back.tags.white, g.tags.white);
    assert_eq!(back.tags.black, g.tags.black);
}

#[test]
fn ut_rul_019_if_1_a_control_character_in_a_value_becomes_a_space() {
    let mut g = game(&[], PgnResult::Unknown);
    g.tags.white = "a\tb\nc\r\u{7f}d".into();
    let text = write_pgn(&g).unwrap();
    assert!(text.contains("[White \"a b c  d\"]\n"), "{text}");
    assert_eq!(read_one(&text).tags.white, "a b c  d");
}

#[test]
fn ut_rul_019_if_1_comment_text_cannot_break_the_brace_syntax() {
    let start = Position::standard();
    let mut moves = parse_line(&start, "1. e4 e5").unwrap();
    moves[0].comment = Some("a } b\nc".into());
    let g = PgnGame::new(start, moves, PgnResult::Unknown);
    let back = read_one(&write_pgn(&g).unwrap());
    assert_eq!(ucis(&back), ["e2e4", "e7e5"]);
    assert_eq!(back.moves[0].comment.as_deref(), Some("a   b c"));
}

// ------------------------------------------------------------------ UT-RUL-018

fn same_game(a: &PgnGame, b: &PgnGame) {
    assert_eq!(a.tags, b.tags);
    assert_eq!(a.moves, b.moves);
    assert_eq!(a.result, b.result);
    assert_eq!(a.start.to_fen(), b.start.to_fen());
}

#[test]
fn ut_rul_018_rule_6_rule_7_round_trip_of_fixture_games() {
    let mut a = game(&["e2e4", "e7e5", "g1f3"], PgnResult::BlackWins);
    a.tags.event = "E".into();
    a.tags.date = PgnDate::new(Some(2020), None, None);
    a.tags.time_control = Some(TimeControl {
        seconds: 180,
        increment: Some(0),
    });
    a.tags.termination = Some(Termination::TimeForfeit);
    a.tags.extras.insert("WhiteClass".into(), "club".into());
    a.moves[0].comment = Some("hello".into());
    a.moves[1].nags = vec![1, 14];
    let start = Position::standard();
    a.moves[2].variations = vec![
        parse_line(
            &start.play(mv("e2e4")).unwrap().play(mv("e7e5")).unwrap(),
            "2. Nc3 Nf6 (2... Nc6) 3. f4",
        )
        .unwrap(),
    ];
    let back = read_one(&write_pgn(&a).unwrap());
    same_game(&a, &back);
    let custom = pos("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 4 20");
    let b = PgnGame::new(
        custom.clone(),
        nodes(&custom, &["e8g8", "e1c1"]),
        PgnResult::Draw,
    );
    same_game(&b, &read_one(&write_pgn(&b).unwrap()));
    let many = write_pgn_many(&[a.clone(), b.clone()]).unwrap();
    let all = read_all(many.as_bytes());
    assert_eq!(all.len(), 2);
    same_game(&a, all[0].as_ref().unwrap());
    same_game(&b, all[1].as_ref().unwrap());
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn ut_rul_018_rule_7_write_then_read_gives_the_same_game_on_random_playouts(
        choices in proptest::collection::vec(any::<u16>(), 0..150),
        result in 0u8..4,
        comment in proptest::option::of("[a-zA-Z0-9 .,]{1,30}"),
        every in 1usize..9,
    ) {
        let start = Position::standard();
        let mut cur = start.clone();
        let mut moves = Vec::new();
        for (i, c) in choices.iter().enumerate() {
            let legal = cur.legal_moves();
            if legal.is_empty() { break; }
            let m = legal[usize::from(*c) % legal.len()];
            let mut node = PgnNode::new(m);
            if i % every == 0 {
                node.comment = comment.as_ref().map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
                node.nags = vec![u8::try_from(i % 20).unwrap()].into_iter().filter(|n| *n > 0).collect();
            }
            moves.push(node);
            cur = cur.play(m).unwrap();
        }
        let result = [PgnResult::WhiteWins, PgnResult::BlackWins, PgnResult::Draw, PgnResult::Unknown][usize::from(result)];
        let g = PgnGame::new(start, moves, result);
        let text = write_pgn(&g).unwrap();
        for line in text.lines() {
            prop_assert!(line.chars().count() <= 79 || line.starts_with('[') || line.contains('{'), "{}", line);
        }
        let back = read_all(text.as_bytes());
        prop_assert_eq!(back.len(), 1);
        let back = back.into_iter().next().unwrap().unwrap();
        prop_assert_eq!(&back.tags, &g.tags);
        prop_assert_eq!(&back.moves, &g.moves);
        prop_assert_eq!(back.result, g.result);
        prop_assert_eq!(canonical_form(&back), canonical_form(&g));
    }
}

// ------------------------------------------------------------------ UT-RUL-020 reader

#[test]
fn ut_rul_020_if_2_reader_multi_game_with_variations_comments_and_nags() {
    let text = "[Event \"A\"]\n[White \"W\"]\n[Black \"B\"]\n[Result \"1-0\"]\n\n\
1. e4 e5 {c1} 2. Nf3 $1 (2. f4 exf4) 2... Nc6 ; semicolon\n3. Bb5 1-0\n\n[Event \"B\"]\n\n1. d4 d5 1/2-1/2\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2);
    let g = all[0].as_ref().unwrap();
    assert_eq!(g.tags.event, "A");
    assert_eq!(g.tags.white, "W");
    assert_eq!(g.result, PgnResult::WhiteWins);
    assert_eq!(ucis(g), ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"]);
    assert_eq!(g.moves[1].comment.as_deref(), Some("c1"));
    assert_eq!(g.moves[2].nags, [1]);
    assert_eq!(g.moves[2].variations.len(), 1);
    let var: Vec<String> = g.moves[2].variations[0]
        .iter()
        .map(|n| n.mv.to_uci())
        .collect();
    assert_eq!(var, ["f2f4", "e5f4"]);
    assert_eq!(g.moves[3].comment.as_deref(), Some("semicolon"));
    let h = all[1].as_ref().unwrap();
    assert_eq!(h.tags.event, "B");
    assert_eq!(h.tags.white, "?", "missing roster tags are filled");
    assert_eq!(h.tags.round, "-");
    assert_eq!(h.result, PgnResult::Draw);
}

#[test]
fn ut_rul_020_if_2_reader_accepts_zero_castling_move_number_glue_and_suffix_annotations() {
    let g = read_one("1.e4 e5 2.Nf3 Nc6 3.Bc4 Nf6 4.0-0 Bc5! 5.d3?! d6!! *");
    assert_eq!(g.moves.len(), 10);
    assert_eq!(g.moves[6].mv.to_uci(), "e1g1");
    assert_eq!(g.moves[7].nags, [1]);
    assert_eq!(g.moves[8].nags, [6]);
    assert_eq!(g.moves[9].nags, [3]);
    assert_eq!(g.result, PgnResult::Unknown);
}

#[test]
fn ut_rul_020_if_2_result_token_is_authoritative_and_missing_terminator_uses_the_tag() {
    let g = read_one("[Result \"1-0\"]\n\n1. e4 0-1\n");
    assert_eq!(g.result, PgnResult::BlackWins);
    let g = read_one("[Result \"0-1\"]\n\n1. e4 e5");
    assert_eq!(g.result, PgnResult::BlackWins);
    let g = read_one("1. e4 e5");
    assert_eq!(g.result, PgnResult::Unknown);
    let g = read_one("[Event \"x\"]\n");
    assert!(g.moves.is_empty());
    assert_eq!(g.result, PgnResult::Unknown);
}

#[test]
fn ut_rul_020_if_2_set_up_and_fen_give_the_start_position() {
    let g = read_one("[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 7\"]\n\n7. e4 Kd8 *\n");
    assert_eq!(g.start.to_fen().as_str(), "4k3/8/8/8/8/8/4P3/4K3 w - - 0 7");
    assert_eq!(ucis(&g), ["e2e4", "e8d8"]);
    let e = read_err("[SetUp \"1\"]\n[FEN \"not a fen\"]\n\n1. e4 *\n");
    assert!(matches!(e.kind, PgnErrorKind::BadFen(_)), "{e:?}");
    assert_eq!(e.game_index, 1);
}

#[test]
fn ut_rul_020_if_2_reader_decodes_utf_8_and_latin_1_per_game_and_drops_a_bom() {
    let utf8 = "[White \"M\u{fc}ller\"]\n\n1. e4 *\n";
    assert_eq!(read_one(utf8).tags.white, "M\u{fc}ller");
    let mut latin1 = b"[White \"M\xfcller\"]\n\n1. e4 *\n".to_vec();
    assert_eq!(
        read_all(&latin1)[0].as_ref().unwrap().tags.white,
        "M\u{fc}ller"
    );
    let mut with_bom = vec![0xef, 0xbb, 0xbf];
    with_bom.append(&mut latin1);
    assert_eq!(
        read_all(&with_bom)[0].as_ref().unwrap().tags.white,
        "M\u{fc}ller"
    );
    // One invalid payload makes the whole game Latin-1; the next game is judged on its own.
    let mixed = b"[White \"M\xfcller\"]\n\n1. e4 {\xc3\xa9} *\n\n[White \"\xc3\xa9\"]\n\n1. d4 *\n";
    let all = read_all(mixed);
    assert_eq!(
        all[0].as_ref().unwrap().moves[0].comment.as_deref(),
        Some("\u{c3}\u{a9}")
    );
    assert_eq!(all[1].as_ref().unwrap().tags.white, "\u{e9}");
}

#[test]
fn ut_rul_020_if_2_line_numbers_count_lf_crlf_and_lone_cr_once_each() {
    for sep in ["\n", "\r\n", "\r"] {
        let text = format!("[Event \"A\"]{sep}{sep}1. e4 e5 2. Nf6 *{sep}");
        let e = read_err(&text);
        assert_eq!((e.line, e.col), (3, 13), "{sep:?}");
        assert!(matches!(e.kind, PgnErrorKind::IllegalSan { .. }));
    }
}

#[test]
fn ut_rul_020_if_2_malformed_input_reports_kind_line_and_column() {
    let e = read_err("[Event \"A\"]\n\n1. e4 {oops");
    assert_eq!((e.line, e.col, e.game_index), (3, 7, 1));
    assert_eq!(e.kind, PgnErrorKind::UnterminatedComment);
    let e = read_err("1. e4 ) e5 *");
    assert_eq!((e.line, e.col), (1, 7));
    assert_eq!(e.kind, PgnErrorKind::UnbalancedVariation);
    let e = read_err("1. e4 (1. d4 e5 *");
    assert_eq!(e.kind, PgnErrorKind::UnbalancedVariation);
    let e = read_err("[Event \"unterminated\n\n1. e4 *");
    assert!(matches!(e.kind, PgnErrorKind::BadTag { .. }));
    assert_eq!((e.line, e.col), (1, 8));
    let e = read_err("[event \"x\"]\n\n1. e4 *");
    assert!(matches!(e.kind, PgnErrorKind::BadTag { .. }));
    let e = read_err("1. e4 e5 2. Nd2 *");
    assert!(matches!(e.kind, PgnErrorKind::IllegalSan { .. }));
    let e = read_err("1. e4 e5 2. e5 $999 *");
    assert!(matches!(
        e.kind,
        PgnErrorKind::IllegalSan { .. } | PgnErrorKind::BadSyntax { .. }
    ));
    let e = read_err("1. e4 @@ *");
    assert!(matches!(e.kind, PgnErrorKind::BadSyntax { .. }));
    let e = read_err("(1. e4) 1. d4 *");
    assert!(
        matches!(e.kind, PgnErrorKind::BadSyntax { .. }),
        "a variation needs a move before it: {e:?}"
    );
    let e = read_err("1. e4 e5 {x} (2. Nf3) *");
    assert!(
        matches!(e.kind, PgnErrorKind::IllegalSan { .. }),
        "a variation replays from before the move it follows: {e:?}"
    );
}

#[test]
fn ut_rul_020_if_2_ambiguous_san_is_reported_with_its_candidates() {
    let e = read_err("[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1\"]\n\n1. Nd2 *");
    match e.kind {
        PgnErrorKind::AmbiguousSan { candidates } => assert_eq!(candidates.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn ut_rul_020_if_2_variation_depth_is_capped() {
    let text = "1. e4 (1. d4 (1. c4 (1. Nf3))) *";
    let limits = PgnLimits {
        max_variation_depth: 2,
    };
    let all: Vec<_> = PgnReader::new(text.as_bytes(), limits).collect();
    assert_eq!(all.len(), 1);
    assert!(matches!(
        all[0].as_ref().unwrap_err().kind,
        PgnErrorKind::VariationTooDeep { limit: 2 }
    ));
    let ok: Vec<_> = PgnReader::new(text.as_bytes(), PgnLimits::default()).collect();
    assert!(ok[0].is_ok());
}

#[test]
fn ut_rul_020_if_2_reader_skips_percent_lines_and_comments_between_games() {
    let text = "% escape line\n{between} ; line comment\n[Event \"A\"]\n\n1. e4 *\n\n% another\n[Event \"B\"]\n\n1. d4 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all.iter().all(Result::is_ok));
}

#[test]
fn ut_rul_020_if_2_a_game_without_terminator_ends_at_the_next_tag_section() {
    let text = "[Event \"A\"]\n\n1. e4 e5\n\n[Event \"B\"]\n\n1. d4 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(all[0].as_ref().unwrap().moves.len(), 2);
    assert_eq!(all[1].as_ref().unwrap().tags.event, "B");
}

// ------------------------------------------------------------------ UT-RUL-021 recovery

#[test]
fn ut_rul_021_if_2_a_bad_game_in_the_middle_leaves_the_neighbours_importable() {
    let text = "[Event \"A\"]\n\n1. e4 e5 *\n\n[Event \"B\"]\n\n1. e4 e5 2. Nf6 {the result 1-0 inside a comment} (3. Nf3 1-0) 2. Nf3 1-0\n\n[Event \"C\"]\n\n1. d4 d5 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 3, "{all:?}");
    assert_eq!(all[0].as_ref().unwrap().tags.event, "A");
    let e = all[1].as_ref().unwrap_err();
    assert_eq!(e.game_index, 2);
    assert_eq!(e.line, 7);
    assert!(matches!(e.kind, PgnErrorKind::IllegalSan { .. }));
    assert_eq!(all[2].as_ref().unwrap().tags.event, "C");
}

#[test]
fn ut_rul_021_if_2_a_bad_tag_section_does_not_swallow_the_next_game() {
    let text = "[Event \"A\"]\n[bad tag]\n\n1. e4 e5 1-0\n\n[Event \"B\"]\n\n1. d4 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all[0].is_err());
    assert_eq!(all[1].as_ref().unwrap().tags.event, "B");
}

#[test]
fn ut_rul_021_if_2_an_unterminated_comment_ends_the_input_and_is_reported_once() {
    let text = "[Event \"A\"]\n\n1. e4 *\n\n[Event \"B\"]\n\n1. d4 {never closed\n[Event \"C\"]\n\n1. c4 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all[0].is_ok());
    assert_eq!(
        all[1].as_ref().unwrap_err().kind,
        PgnErrorKind::UnterminatedComment
    );
}

// ------------------------------------------------------------------ UT-RUL-022 line text

#[test]
fn ut_rul_022_ana_2_format_line_and_parse_line_round_trip() {
    let start = Position::standard();
    let nodes = parse_line(&start, "1. d4 d5 2. c4").unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(format_line(&start, &nodes).unwrap(), "1. d4 d5 2. c4");
    let after_d4 = start.play(mv("d2d4")).unwrap();
    let black = parse_line(&after_d4, "1... d5 2. c4").unwrap();
    assert_eq!(format_line(&after_d4, &black).unwrap(), "1... d5 2. c4");
    let with_var = "1. d4 d5 (1... Nf6 2. c4) 2. c4";
    let n = parse_line(&start, with_var).unwrap();
    assert_eq!(n[1].variations.len(), 1);
    assert_eq!(format_line(&start, &n).unwrap(), with_var);
    // Illegal movetext is an error with a position; an illegal node is a write error.
    assert!(parse_line(&start, "1. e5").is_err());
    let bad = vec![PgnNode::new(mv("e2e5"))];
    assert!(matches!(
        format_line(&start, &bad),
        Err(PgnWriteError::IllegalMove { .. })
    ));
    assert_eq!(format_line(&start, &[]).unwrap(), "");
}

#[test]
fn ut_rul_022_ana_2_a_saved_variation_round_trips_inside_a_full_game() {
    let start = Position::standard();
    let mut g = game(&["e2e4", "e7e5", "g1f3"], PgnResult::Unknown);
    let branch_start = start.play(mv("e2e4")).unwrap().play(mv("e7e5")).unwrap();
    let branch = parse_line(&branch_start, "2. f4 exf4").unwrap();
    g.moves[2].variations.push(branch.clone());
    let back = read_one(&write_pgn(&g).unwrap());
    assert_eq!(back.moves[2].variations, vec![branch]);
}

// ------------------------------------------------------------------ UT-RUL-029 eco_tag

#[test]
fn ut_rul_029_dat_3_eco_tag_is_the_raw_text_of_the_tag() {
    for raw in ["C60", "c60", " a45 ", "", "Z99", "garbage \\\\ text"] {
        let escaped = raw.replace('\\', "\\\\");
        let g = read_one(&format!("[ECO \"{escaped}\"]\n\n1. e4 *\n"));
        assert_eq!(g.tags.eco_tag(), Some(raw), "{raw:?}");
    }
    assert_eq!(read_one("[Event \"x\"]\n\n1. e4 *\n").tags.eco_tag(), None);
    let custom = read_one(
        "[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1\"]\n[ECO \"B00\"]\n\n1. e4 *\n",
    );
    assert_eq!(custom.tags.eco_tag(), Some("B00"));
    // canonical_form is identical with and without the tag (D-42).
    let with = read_one("[ECO \"B00\"]\n\n1. e4 e5 *\n");
    let without = read_one("1. e4 e5 *\n");
    assert_eq!(canonical_form(&with), canonical_form(&without));
}

// ------------------------------------------------------------------ UT-RUL-026 canonical form

const GOLDEN_V1: &str = "[Event \"?\"]\n[Site \"?\"]\n[Date \"????.??.??\"]\n[Round \"-\"]\n[White \"?\"]\n[Black \"?\"]\n[Result \"*\"]\n\n1. e4 e5 *\n";

#[test]
fn ut_rul_026_d_42_canonical_form_golden_bytes_of_v1() {
    // OQ-RUL-005 (b), proposed fixtures: the bytes of canonical_form v1 for two fixed games.
    // A change here is a change of v1 (SDD 3.11.10 "Version rule").
    let g = game(&["e2e4", "e7e5"], PgnResult::Unknown);
    assert_eq!(String::from_utf8(canonical_form(&g)).unwrap(), GOLDEN_V1);
    let mut h = game(&["d2d4", "g8f6", "c2c4"], PgnResult::WhiteWins);
    h.tags.event = "Club \"A\"".into();
    h.tags.date = PgnDate::new(Some(2026), Some(11), Some(5));
    h.tags.white = "W\\x".into();
    h.tags.time_control = Some(TimeControl {
        seconds: 300,
        increment: Some(3),
    });
    let expected = "[Event \"Club \\\"A\\\"\"]\n[Site \"?\"]\n[Date \"2026.11.05\"]\n[Round \"-\"]\n[White \"W\\\\x\"]\n\
[Black \"?\"]\n[Result \"1-0\"]\n[TimeControl \"300+3\"]\n\n1. d4 Nf6 2. c4 1-0\n";
    assert_eq!(String::from_utf8(canonical_form(&h)).unwrap(), expected);
}

#[test]
fn ut_rul_026_d_42_canonical_form_ignores_custom_tags_comments_nags_and_variations() {
    let base = game(&["e2e4", "e7e5", "g1f3"], PgnResult::Draw);
    let expected = canonical_form(&base);
    let mut g = base.clone();
    for (k, v) in [
        ("GameUUID", "u"),
        ("WhiteUUID", "u"),
        ("Mode", "m"),
        ("Rated", "r"),
        ("WhiteClass", "c"),
        ("BlackClass", "c"),
        ("EndStatus", "e"),
        ("ECO", "C20"),
        ("Opening", "King's Pawn"),
        ("Unknown", "x"),
    ] {
        g.tags.extras.insert(k.into(), v.into());
    }
    g.tags.termination = Some(Termination::Abandoned);
    g.moves[0].comment = Some("c".into());
    g.moves[1].nags = vec![1];
    let start = Position::standard();
    g.moves[2].variations = vec![
        parse_line(
            &start.play(mv("e2e4")).unwrap().play(mv("e7e5")).unwrap(),
            "2. Nc3",
        )
        .unwrap(),
    ];
    assert_eq!(canonical_form(&g), expected);
    // A tag that is part of the set changes the bytes.
    let mut h = base.clone();
    h.tags.white = "X".into();
    assert_ne!(canonical_form(&h), expected);
    let mut h = base.clone();
    h.tags.time_control = Some(TimeControl {
        seconds: 60,
        increment: None,
    });
    assert_ne!(canonical_form(&h), expected);
    let mut h = base;
    h.result = PgnResult::WhiteWins;
    assert_ne!(canonical_form(&h), expected);
}

#[test]
fn ut_rul_026_if_2_a_foreign_pgn_and_our_export_give_equal_bytes() {
    let ours = game(&["e2e4", "e7e5", "g1f3", "b8c6"], PgnResult::Draw);
    let foreign = read_one(
        "[Event \"?\"]\n[Result \"1/2-1/2\"]\n[Opening \"Four Knights\"]\n[ECO \"C44\"]\n\n1.e4 {x} e5 2.Nf3 $1 Nc6 1/2-1/2\n",
    );
    assert_eq!(canonical_form(&foreign), canonical_form(&ours));
    let reread = read_one(&write_pgn(&ours).unwrap());
    assert_eq!(canonical_form(&reread), canonical_form(&ours));
    // Missing roster tags equal "?".
    let explicit = read_one(
        "[Event \"?\"]\n[Site \"?\"]\n[Date \"????.??.??\"]\n[Round \"-\"]\n[White \"?\"]\n[Black \"?\"]\n[Result \"1/2-1/2\"]\n\n1. e4 e5 2. Nf3 Nc6 1/2-1/2\n",
    );
    assert_eq!(canonical_form(&explicit), canonical_form(&ours));
}

#[test]
fn ut_rul_026_d_42_canonical_form_includes_set_up_fen_for_a_custom_start_in_ascii_order() {
    let start = pos("4k3/8/8/8/8/8/4P3/4K3 w - - 0 7");
    let moves = nodes(&start, &["e2e4"]);
    let mut g = PgnGame::new(start, moves, PgnResult::Unknown);
    g.tags.time_control = Some(TimeControl {
        seconds: 60,
        increment: None,
    });
    let text = String::from_utf8(canonical_form(&g)).unwrap();
    assert!(
        text.contains("[Result \"*\"]\n[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 7\"]\n[SetUp \"1\"]\n[TimeControl \"60\"]\n\n7. e4 *\n"),
        "{text}"
    );
}

// ------------------------------------------------------------------ UT-RUL-031, CT-RUL-004

/// UT-RUL-031 (5), SF-02, D-64: the seven-row drift-guard table. This file is the single source;
/// `chess-domain/tests/drift_guard.rs` reads the same file by a relative `include_str!`
/// (test-only, no Cargo edge). Format: `row_name|fen|chess-domain expected start kind`; only the
/// first two columns are used here.
const DRIFT_GUARD_TABLE: &str = include_str!("data/fen_drift_guard.txt");

fn drift_rows() -> Vec<(&'static str, &'static str)> {
    DRIFT_GUARD_TABLE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split('|');
            (f.next().unwrap(), f.next().unwrap())
        })
        .collect()
}

#[test]
fn ut_rul_031_5_drift_guard_table_has_the_standard_row_and_six_single_field_variants() {
    let rows = drift_rows();
    assert_eq!(rows.len(), 7);
    assert_eq!(rows[0], ("standard", START_FEN));
}

#[test]
fn ut_rul_031_5_every_drift_guard_row_keeps_or_drops_the_tags_by_its_fen() {
    for (name, fen) in drift_rows() {
        let input = format!("[SetUp \"1\"]\n[FEN \"{fen}\"]\n\n*\n");
        let parsed = chess_rules::Fen::parse(fen);
        let games = read_all(input.as_bytes());
        assert_eq!(games.len(), 1, "{name}");
        match (parsed, &games[0]) {
            // A FEN that the FEN type rejects (the en passant row) fails the game as BadFen.
            (Err(_), Err(e)) => assert!(matches!(e.kind, PgnErrorKind::BadFen(_)), "{name}"),
            (Ok(_), Ok(g)) => {
                let text = String::from_utf8(canonical_form(g)).unwrap();
                if fen == START_FEN {
                    assert!(!text.contains("[FEN") && !text.contains("[SetUp"), "{name}");
                } else {
                    assert!(
                        text.contains(&format!("[FEN \"{fen}\"]\n[SetUp \"1\"]\n")),
                        "{name}: {text}"
                    );
                }
            }
            (parsed, game) => panic!("{name}: {parsed:?} against {game:?}"),
        }
    }
}

#[test]
fn ut_rul_031_oq_27_standard_start_omits_set_up_and_fen() {
    let plain = read_one("1. e4 e5 *\n");
    let tagged = read_one(&format!(
        "[SetUp \"1\"]\n[FEN \"{START_FEN}\"]\n\n1. e4 e5 *\n"
    ));
    assert_eq!(canonical_form(&tagged), canonical_form(&plain)); // (1)
    let fen_alone = read_one(&format!("[FEN \"{START_FEN}\"]\n\n1. e4 e5 *\n"));
    assert_eq!(canonical_form(&fen_alone), canonical_form(&plain)); // (4)
    assert!(
        !String::from_utf8(canonical_form(&tagged))
            .unwrap()
            .contains("FEN")
    );
}

#[test]
fn ut_rul_031_oq_27_another_counter_or_placement_keeps_both_tags() {
    let counter = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 2";
    let other = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1";
    for fen in [counter, other] {
        let g = read_one(&format!("[SetUp \"1\"]\n[FEN \"{fen}\"]\n\n*\n"));
        let text = String::from_utf8(canonical_form(&g)).unwrap();
        assert!(
            text.contains(&format!("[FEN \"{fen}\"]\n[SetUp \"1\"]\n")),
            "{text}"
        );
    }
}

#[test]
fn ct_rul_004_d_42_canonical_form_survives_write_and_reread() {
    let mut fixtures = vec![game(&["e2e4", "e7e5"], PgnResult::Unknown)];
    let mut custom_tags = game(&["d2d4", "d7d5"], PgnResult::Draw);
    custom_tags
        .tags
        .extras
        .insert("GameUUID".into(), "abc".into());
    custom_tags.tags.extras.insert("ECO".into(), "D00".into());
    custom_tags.tags.termination = Some(Termination::Normal);
    custom_tags.tags.time_control = Some(TimeControl {
        seconds: 300,
        increment: Some(5),
    });
    fixtures.push(custom_tags);
    let start = pos("4k3/8/8/8/8/8/4P3/4K3 w - - 0 7");
    fixtures.push(PgnGame::new(
        start.clone(),
        nodes(&start, &["e2e4"]),
        PgnResult::WhiteWins,
    ));
    for g in fixtures {
        let back = read_one(&write_pgn(&g).unwrap());
        assert_eq!(canonical_form(&back), canonical_form(&g));
    }
    // A game imported with the standard FEN tag equals the hash input of our export for it.
    let imported = read_one(&format!(
        "[SetUp \"1\"]\n[FEN \"{START_FEN}\"]\n\n1. e4 e5 *\n"
    ));
    let exported = write_pgn(&imported).unwrap();
    assert!(!exported.contains("FEN"));
    assert_eq!(
        canonical_form(&read_one(&exported)),
        canonical_form(&imported)
    );
}

// ------------------------------------------------------------------ UT-RUL-024, UT-RUL-025

#[test]
fn ut_rul_024_nfr_9_part_b_errors_render_messages_without_paths_or_internals() {
    let mut messages: Vec<String> = Vec::new();
    for e in [
        chess_rules::MoveParseError::Empty,
        chess_rules::MoveParseError::TooLong,
        chess_rules::MoveParseError::Syntax,
        chess_rules::MoveParseError::LowercasePiece {
            suggestion: "Nf3".into(),
        },
        chess_rules::MoveParseError::PromotionRequired {
            choices: vec![chess_rules::PieceKind::Queen],
        },
        chess_rules::MoveParseError::Ambiguous { candidates: vec![] },
        chess_rules::MoveParseError::NoMatch {
            reason: chess_rules::IllegalReason::Other,
        },
    ] {
        messages.push(e.to_string());
    }
    let kinds = [
        PgnErrorKind::IllegalSan {
            reason: chess_rules::IllegalReason::Blocked,
        },
        PgnErrorKind::AmbiguousSan { candidates: vec![] },
        PgnErrorKind::BadSyntax {
            reason: chess_rules::SyntaxReason::NotAMove,
        },
        PgnErrorKind::UnterminatedComment,
        PgnErrorKind::UnbalancedVariation,
        PgnErrorKind::VariationTooDeep { limit: 64 },
        PgnErrorKind::BadTag {
            reason: chess_rules::TagReason::NameStart,
        },
        PgnErrorKind::BadFen(chess_rules::Fen::parse("x").unwrap_err()),
    ];
    for kind in kinds {
        messages.push(
            PgnParseError {
                game_index: 1,
                line: 2,
                col: 3,
                kind,
            }
            .to_string(),
        );
    }
    for e in [
        PgnWriteError::BadTagName { name: "x".into() },
        PgnWriteError::ReservedTag { name: "x".into() },
        PgnWriteError::IllegalMove {
            variation_path: vec![],
            ply: 0,
        },
        PgnWriteError::VariationTooDeep { limit: 64 },
        PgnWriteError::InGame {
            game_index: 2,
            source: Box::new(PgnWriteError::VariationTooDeep { limit: 64 }),
        },
    ] {
        messages.push(e.to_string());
    }
    for m in &messages {
        assert!(!m.is_empty());
        assert!(
            !m.contains('/') || m.contains("1-0") || m.contains("e.g."),
            "{m}"
        );
        assert!(!m.contains("src") && !m.contains(".rs"), "{m}");
    }
    assert!(
        messages.iter().any(|m| m
            == "game 1, line 2, column 3: a comment is not closed before the end of the input")
    );
}

#[test]
fn ut_rul_025_architecture_4_8_part_b_types_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<chess_rules::San>();
    assert_send_sync::<chess_rules::ParseMode>();
    assert_send_sync::<chess_rules::MatchedMove>();
    assert_send_sync::<PgnGame>();
    assert_send_sync::<chess_rules::PgnTags>();
    assert_send_sync::<PgnNode>();
    assert_send_sync::<PgnDate>();
    assert_send_sync::<PgnResult>();
    assert_send_sync::<Termination>();
    assert_send_sync::<TimeControl>();
    assert_send_sync::<PgnLimits>();
    assert_send_sync::<PgnReader<'static>>();
    assert_send_sync::<PgnParseError>();
    assert_send_sync::<PgnErrorKind>();
    assert_send_sync::<PgnWriteError>();
}

// ------------------------------------------------------------------ FZ-RUL-003

const VALID: &str = "[Event \"A\"]\n[White \"W\"]\n[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 7\"]\n\n7. e4 {c} Kd8 (7... Ke7 8. Kd2 $1) 8. Kd2 ; x\n\n1-0\n\n[Event \"B\"]\n\n1. d4 d5 1/2-1/2\n";

/// Runs the reader and checks termination: every item consumes input, so at most one item per
/// byte plus one.
fn exhaust(bytes: &[u8]) {
    let items = PgnReader::new(bytes, PgnLimits::default())
        .take(bytes.len() + 2)
        .count();
    assert!(items <= bytes.len() + 1);
    let mut r = PgnReader::new(bytes, PgnLimits::default());
    let mut n = 0usize;
    while r.next().is_some() {
        n += 1;
        assert!(n <= bytes.len() + 1, "reader did not terminate");
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn fz_rul_003_if_2_reader_never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..400)) {
        exhaust(&bytes);
    }

    #[test]
    fn fz_rul_003_if_2_reader_never_panics_on_pgn_shaped_text(
        text in "[\\[\\]\\(\\)\\{\\}\"; $%!?*0-9a-hKQRBNOx=+#\\\\ \\n\\r-]{0,300}",
    ) {
        exhaust(text.as_bytes());
    }

    #[test]
    fn fz_rul_003_if_2_reader_survives_mutations_of_a_valid_pgn(
        edits in proptest::collection::vec((0usize..VALID.len(), any::<u8>(), 0u8..3), 1..8),
        cut in proptest::option::of(0usize..VALID.len()),
    ) {
        let mut bytes = VALID.as_bytes().to_vec();
        for (at, byte, kind) in edits {
            let at = at.min(bytes.len().saturating_sub(1));
            match kind {
                0 => { if let Some(b) = bytes.get_mut(at) { *b = byte; } }
                1 => bytes.insert(at, byte),
                _ => { if !bytes.is_empty() { bytes.remove(at); } }
            }
        }
        if let Some(c) = cut { bytes.truncate(c.min(bytes.len())); }
        exhaust(&bytes);
    }
}

#[test]
fn fz_rul_003_fm_rul_004_hostile_inputs_terminate_with_typed_errors() {
    // Deep nesting: 100000 open parentheses after a move, then the same with closers.
    let mut deep = String::from("1. e4 ");
    deep.push_str(&"(1. d4 ".repeat(50_000));
    let items: Vec<_> = PgnReader::new(deep.as_bytes(), PgnLimits::default()).collect();
    assert_eq!(items.len(), 1);
    assert!(matches!(
        items[0].as_ref().unwrap_err().kind,
        PgnErrorKind::VariationTooDeep { .. } | PgnErrorKind::UnbalancedVariation
    ));
    let closers = format!("1. e4 {} *", ")".repeat(100_000));
    exhaust(closers.as_bytes());
    // A huge comment and a huge tag value.
    let big_comment = format!("1. e4 {{{}}} *", "x".repeat(2_000_000));
    let all = read_all(big_comment.as_bytes());
    assert_eq!(all.len(), 1);
    assert!(all[0].is_ok());
    let big_tag = format!("[Event \"{}\"]\n\n1. e4 *", "y".repeat(2_000_000));
    assert!(read_all(big_tag.as_bytes())[0].is_ok());
    // Binary data, NULs and a lone surrogate-like byte run.
    exhaust(&[0u8; 5000]);
    exhaust(&[0xff, 0xfe, 0x00, b'[', b'"', 0x80]);
    exhaust(&(0..=255u8).cycle().take(10_000).collect::<Vec<u8>>());
    // A single giant token.
    exhaust("N".repeat(1_000_000).as_bytes());
}

// ------------------------------------------------------------------ typed tags, NAGs, limits

#[test]
fn ut_rul_020_if_2_suffix_marks_become_the_six_standard_nags() {
    let g = read_one("1. e4! e5? 2. Nf3!! Nc6?? 3. Bb5!? a6?! *");
    let nags: Vec<Vec<u8>> = g.moves.iter().map(|n| n.nags.clone()).collect();
    assert_eq!(nags, [vec![1], vec![2], vec![3], vec![4], vec![5], vec![6]]);
    // A mark with a check sign, and an explicit NAG after a mark, keep their order.
    let g = read_one("1. e4+! e5 $14 *");
    assert_eq!(g.moves[0].nags, [1]);
    assert_eq!(g.moves[1].nags, [14]);
}

#[test]
fn ut_rul_020_if_2_typed_standard_tags_are_read_and_unusable_values_fall_back() {
    let g = read_one(
        "[Date \"2026.11.??\"]\n[TimeControl \"300+2\"]\n[Termination \"Time forfeit\"]\n\n*\n",
    );
    assert_eq!(
        (g.tags.date.year(), g.tags.date.month(), g.tags.date.day()),
        (Some(2026), Some(11), None)
    );
    assert_eq!(
        g.tags.time_control,
        Some(TimeControl {
            seconds: 300,
            increment: Some(2)
        })
    );
    assert_eq!(g.tags.termination, Some(Termination::TimeForfeit));
    let g = read_one("[TimeControl \"600\"]\n[Termination \"ABANDONED\"]\n\n*\n");
    assert_eq!(
        g.tags.time_control,
        Some(TimeControl {
            seconds: 600,
            increment: None
        })
    );
    assert_eq!(g.tags.termination, Some(Termination::Abandoned));
    let g = read_one("[Termination \"normal\"]\n\n*\n");
    assert_eq!(g.tags.termination, Some(Termination::Normal));
    // Values the model cannot hold: dropped to unknown / absent (finding F-B6).
    let g = read_one(
        "[Date \"yesterday\"]\n[TimeControl \"40/7200\"]\n[Termination \"Unterminated\"]\n\n*\n",
    );
    assert_eq!(g.tags.date, PgnDate::default());
    assert_eq!(g.tags.time_control, None);
    assert_eq!(g.tags.termination, None);
    for bad in ["+5", "5+", "-1", "", "5+-1", "5 ", "1e3"] {
        let g = read_one(&format!("[TimeControl \"{bad}\"]\n\n*\n"));
        assert_eq!(g.tags.time_control, None, "{bad:?}");
    }
    // A part out of range is unknown; the other parts stay.
    let g = read_one("[Date \"2026.13.32\"]\n\n*\n");
    assert_eq!(
        (g.tags.date.year(), g.tags.date.month(), g.tags.date.day()),
        (Some(2026), None, None)
    );
    for bad in [
        "2026.1.1",
        "26.01.01",
        "2026.01",
        "2026.01.01.01",
        "2026-01-01",
        "abcd.ef.gh",
    ] {
        let g = read_one(&format!("[Date \"{bad}\"]\n\n*\n"));
        assert_eq!(g.tags.date, PgnDate::default(), "{bad:?}");
    }
}

#[test]
fn ut_rul_020_if_2_a_repeated_tag_keeps_the_last_value_and_result_tags_are_read() {
    let g = read_one("[White \"A\"]\n[White \"B\"]\n[Result \"1/2-1/2\"]\n\n1. e4\n");
    assert_eq!(g.tags.white, "B");
    assert_eq!(g.result, PgnResult::Draw);
    let g = read_one("[Result \"nonsense\"]\n\n1. e4\n");
    assert_eq!(g.result, PgnResult::Unknown);
}

#[test]
fn ut_rul_009_if_1_pgn_date_new_drops_values_out_of_range() {
    let d = PgnDate::new(Some(2026), Some(11), Some(5));
    assert_eq!(
        (d.year(), d.month(), d.day()),
        (Some(2026), Some(11), Some(5))
    );
    assert_eq!(d.to_string(), "2026.11.05");
    let d = PgnDate::new(Some(10_000), Some(13), Some(32));
    assert_eq!((d.year(), d.month(), d.day()), (None, None, None));
    let d = PgnDate::new(Some(9999), Some(1), Some(1));
    assert_eq!(d.to_string(), "9999.01.01");
    let d = PgnDate::new(Some(5), Some(0), Some(0));
    assert_eq!(d.to_string(), "0005.??.??");
    assert_eq!(PgnDate::default().to_string(), "????.??.??");
}

fn nested(depth: usize) -> Vec<PgnNode> {
    let mut inner = vec![PgnNode::new(mv("e2e4"))];
    for _ in 0..depth {
        let mut n = PgnNode::new(mv("e2e4"));
        n.variations = vec![inner];
        inner = vec![n];
    }
    inner
}

#[test]
fn ut_rul_017_if_1_writer_accepts_64_levels_of_variations_and_rejects_65() {
    let ok = PgnGame::new(Position::standard(), nested(64), PgnResult::Unknown);
    let text = write_pgn(&ok).unwrap();
    // The writer's own output is within the reader's default limit.
    assert!(read_all(text.as_bytes())[0].is_ok());
    let too_deep = PgnGame::new(Position::standard(), nested(65), PgnResult::Unknown);
    assert!(matches!(
        write_pgn(&too_deep),
        Err(PgnWriteError::VariationTooDeep { limit: 64 })
    ));
}

#[test]
fn ut_rul_020_if_2_reader_accepts_the_configured_depth_and_rejects_one_more() {
    let text = |n: usize| format!("1. e4 {}{} *", "(1. d4 ".repeat(n), ")".repeat(n));
    let limits = PgnLimits {
        max_variation_depth: 3,
    };
    let ok: Vec<_> = PgnReader::new(text(3).as_bytes(), limits).collect();
    assert!(ok[0].is_ok(), "{ok:?}");
    let bad: Vec<_> = PgnReader::new(text(4).as_bytes(), limits).collect();
    assert!(matches!(
        bad[0].as_ref().unwrap_err().kind,
        PgnErrorKind::VariationTooDeep { limit: 3 }
    ));
}

#[test]
fn ut_rul_020_if_2_malformed_tag_lines_report_the_position_of_the_problem() {
    for (text, line, col) in [
        ("[Event \"x\" \n\n1. e4 *", 1usize, 12usize), // no closing bracket
        ("[Event x]\n\n1. e4 *", 1, 8),                // value not quoted
        ("[ \"x\"]\n\n1. e4 *", 1, 3),                 // no name
        ("[1abc \"x\"]\n\n1. e4 *", 1, 2),             // name starts with a digit
    ] {
        let e = read_err(text);
        assert!(
            matches!(e.kind, PgnErrorKind::BadTag { .. }),
            "{text:?}: {e:?}"
        );
        assert_eq!((e.line, e.col), (line, col), "{text:?}");
    }
    // Escapes: \" and \\ are decoded, any other backslash stays as written.
    let g = read_one("[Event \"a\\\"b\\\\c\\d\"]\n\n*\n");
    assert_eq!(g.tags.event, "a\"b\\c\\d");
    // Blanks around name and value are allowed.
    let g = read_one("[  Event   \"x\"  ]\n\n*\n");
    assert_eq!(g.tags.event, "x");
}

#[test]
fn ut_rul_020_if_2_nag_and_brace_errors() {
    for text in ["1. e4 $ *", "1. e4 $1000 *", "$1 1. e4 *", "1. e4 } *"] {
        let e = read_err(text);
        assert!(
            matches!(e.kind, PgnErrorKind::BadSyntax { .. }),
            "{text:?}: {e:?}"
        );
    }
    let e = read_err("1. e4 $1000 *");
    assert_eq!((e.line, e.col), (1, 7));
    // $0 is a valid NAG number.
    assert_eq!(read_one("1. e4 $0 *").moves[0].nags, [0]);
}

#[test]
fn ut_rul_020_if_2_comments_attach_to_the_move_before_them_and_join() {
    let g = read_one("1. e4 {one} {two} ; three\n e5 { padded } {} *");
    assert_eq!(g.moves[0].comment.as_deref(), Some("one two three"));
    assert_eq!(g.moves[1].comment.as_deref(), Some("padded"));
    // A comment before the first move has no node to attach to (finding F-B7).
    let g = read_one("{opening} 1. e4 *");
    assert_eq!(g.moves[0].comment, None);
    // A comment inside a variation attaches inside it.
    let g = read_one("1. e4 e5 (1... c5 {sicilian}) *");
    assert_eq!(
        g.moves[1].variations[0][0].comment.as_deref(),
        Some("sicilian")
    );
}

#[test]
fn ut_rul_020_if_2_move_numbers_are_skipped_in_all_their_spellings() {
    let g = read_one("1.e4 e5 2 . Nf3 2... Nc6 3... *");
    assert_eq!(ucis(&g), ["e2e4", "e7e5", "g1f3", "b8c6"]);
    let g = read_one("1. ... e4 *");
    assert_eq!(ucis(&g), ["e2e4"]);
}

#[test]
fn ut_rul_020_if_2_a_result_inside_a_variation_does_not_end_the_game() {
    let g = read_one("1. e4 (1. d4 1-0) e5 *");
    assert_eq!(ucis(&g), ["e2e4", "e7e5"]);
    assert_eq!(g.moves[0].variations.len(), 1);
}

#[test]
fn ut_rul_020_if_2_stray_bytes_between_games_are_an_error_not_a_hang() {
    let all =
        read_all(b"[Event \"A\"]\n\n1. e4 *\n\n\x01\x02 junk *\n\n[Event \"B\"]\n\n1. d4 *\n");
    assert_eq!(all.len(), 3, "{all:?}");
    assert!(all[0].is_ok() && all[1].is_err() && all[2].is_ok());
}

#[test]
fn ut_rul_022_ana_2_parse_line_reports_positions_in_the_text() {
    let start = Position::standard();
    let e = parse_line(&start, "1. e4 e5\n2. Nf6").unwrap_err();
    assert_eq!((e.game_index, e.line, e.col), (1, 2, 4));
    assert!(matches!(e.kind, PgnErrorKind::IllegalSan { .. }));
    // A result token ends the line text.
    assert_eq!(parse_line(&start, "1. e4 e5 *").unwrap().len(), 2);
}

// ------------------------------------------------------------------ recovery details (mutation survivors)

#[test]
fn ut_rul_017_if_1_writer_extra_tag_names_allow_underscore_and_digits_only() {
    let mut g = game(&[], PgnResult::Unknown);
    g.tags.extras.insert("My_Tag2".into(), "x".into());
    assert!(write_pgn(&g).unwrap().contains("[My_Tag2 \"x\"]\n"));
    for bad in ["A-B", "A B", "", "9A", "Ab\u{e9}", "A\"B"] {
        let mut t = game(&[], PgnResult::Unknown);
        t.tags.extras.insert(bad.into(), "x".into());
        assert!(
            matches!(write_pgn(&t), Err(PgnWriteError::BadTagName { .. })),
            "{bad:?}"
        );
    }
}

#[test]
fn ut_rul_021_if_2_skipping_ignores_result_text_in_line_comments_quotes_and_stray_braces() {
    // Each bad game has an illegal first move, then a decoy result, then the real terminator.
    let decoys = [
        "1. e5 ; 1-0 in a line comment\n 2. e4 *",
        "1. e5 \"1-0\" 2. e4 *",
        "1. e5 } 2. e4 *",
        "1. e5 (2. e4 1-0) 2. e4 *",
    ];
    for decoy in decoys {
        let text = format!("[Event \"A\"]\n\n{decoy}\n\n[Event \"B\"]\n\n1. d4 *\n");
        let all = read_all(text.as_bytes());
        let tail = all.last().unwrap().as_ref();
        assert!(
            tail.is_ok() && tail.unwrap().tags.event == "B",
            "{decoy:?}: {all:?}"
        );
        assert_eq!(all.len(), 2, "{decoy:?}: {all:?}");
    }
    // An unterminated quote ends at the line break, so the next line's result is seen.
    let text = "[Event \"A\"]\n\n1. e5 \"open\n 1-0\n\n[Event \"B\"]\n\n1. d4 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2, "{all:?}");
}

#[test]
fn ut_rul_021_if_2_a_movetext_error_stops_at_the_next_tag_section_when_there_is_no_terminator() {
    let text = "[Event \"A\"]\n\n1. e5 2. e4\n[Event \"B\"]\n\n1. d4 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all[0].is_err());
    assert_eq!(all[1].as_ref().unwrap().tags.event, "B");
    // An error inside a variation does not stop at a bracket that is not at a line start.
    let text = "1. e4 (1. e5 [Event \"x\"] 1-0) e5 *\n";
    let all = read_all(text.as_bytes());
    assert_eq!(all.len(), 1, "{all:?}");
}

#[test]
fn ut_rul_020_if_2_a_bracket_or_percent_inside_a_line_is_not_a_tag_or_an_escape() {
    let e = read_err("1. e4 [Event \"x\"]\n\n1. d4 *\n");
    assert!(matches!(e.kind, PgnErrorKind::BadSyntax { .. }), "{e:?}");
    let e = read_err("1. e4 %x\n *");
    assert!(matches!(e.kind, PgnErrorKind::BadSyntax { .. }), "{e:?}");
    // A bracket line inside a variation is not the next game.
    let e = read_err("1. e4 (1. d4\n[Event \"x\"]\n 1-0 *");
    assert!(matches!(
        e.kind,
        PgnErrorKind::BadSyntax { .. } | PgnErrorKind::UnbalancedVariation
    ));
}

#[test]
fn ut_rul_020_if_2_a_closing_brace_alone_says_so() {
    let e = read_err("1. e4 } e5 *");
    match e.kind {
        PgnErrorKind::BadSyntax { reason } => {
            assert_eq!(reason, chess_rules::SyntaxReason::StrayClosingBrace);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn ut_rul_020_if_2_set_up_zero_means_the_standard_start_even_with_a_fen_tag() {
    let g = read_one("[SetUp \"0\"]\n[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 7\"]\n\n1. e4 *\n");
    assert_eq!(g.start.to_fen().as_str(), START_FEN);
    let g = read_one("[SetUp \"1\"]\n\n1. e4 *\n");
    assert_eq!(g.start.to_fen().as_str(), START_FEN);
}

#[test]
fn ut_rul_021_if_2_an_unterminated_comment_between_games_is_reported_once() {
    let all = read_all(b"[Event \"A\"]\n\n1. e4 *\n\n{never closed\n[Event \"B\"]\n\n1. d4 *\n");
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(
        all[1].as_ref().unwrap_err().kind,
        PgnErrorKind::UnterminatedComment
    );
    assert_eq!(all[1].as_ref().unwrap_err().game_index, 2);
}
