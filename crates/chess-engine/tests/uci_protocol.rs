//! UT-EN-010..019 and FZ-EN-001 (the UCI parser, formatter and bounded line splitter).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]

use chess_engine::uci::{
    InfoLine, LineSplitter, OptionKind, OptionSpec, OptionValue, SetOption, UciMessage, format_go,
    format_position, format_setoption, parse_line,
};
use chess_engine::{Score, SearchLimits, SearchPosition, UciMove};
use chess_rules::Fen;
use proptest::prelude::*;
use std::time::Duration;

fn mv(t: &str) -> UciMove {
    UciMove::parse(t).unwrap()
}

#[test]
fn ut_en_010_id_lines() {
    assert_eq!(
        parse_line("id name Stockfish 19"),
        UciMessage::IdName("Stockfish 19".into())
    );
    assert_eq!(
        parse_line("id author the Stockfish developers"),
        UciMessage::IdAuthor("the Stockfish developers".into())
    );
    assert_eq!(parse_line("id"), UciMessage::Malformed);
    assert_eq!(parse_line("id name"), UciMessage::Malformed);
    assert_eq!(parse_line("id colour red"), UciMessage::Malformed);
}

fn opt(line: &str) -> OptionSpec {
    match parse_line(line) {
        UciMessage::Option(s) => s,
        other => panic!("{line}: {other:?}"),
    }
}

#[test]
fn ut_en_011_option_lines_of_every_type() {
    assert_eq!(
        opt("option name UCI_Elo type spin default 1500 min 1000 max 2000"),
        OptionSpec {
            name: "UCI_Elo".into(),
            kind: OptionKind::Spin {
                default: 1500,
                min: 1000,
                max: 2000
            }
        }
    );
    assert_eq!(
        opt("option name Skill Level type spin default 20 min 0 max 20").name,
        "Skill Level"
    );
    assert_eq!(
        opt("option name UCI_LimitStrength type check default false").kind,
        OptionKind::Check { default: false }
    );
    assert_eq!(
        opt("option name Style type combo default Normal var Solid var Normal var Risky").kind,
        OptionKind::Combo {
            default: "Normal".into(),
            vars: vec!["Solid".into(), "Normal".into(), "Risky".into()]
        }
    );
    assert_eq!(
        opt("option name Clear Hash type button").kind,
        OptionKind::Button
    );
    assert_eq!(
        opt("option name EvalFile type string default nn-1a298aa575a0.nnue").kind,
        OptionKind::Str {
            default: "nn-1a298aa575a0.nnue".into()
        }
    );
    // Negative bounds are legal for a spin.
    assert_eq!(
        opt("option name Contempt type spin default 0 min -100 max 100").kind,
        OptionKind::Spin {
            default: 0,
            min: -100,
            max: 100
        }
    );
}

#[test]
fn ut_en_011b_unusable_option_lines_are_malformed_not_panics() {
    for bad in [
        "option",
        "option name",
        "option name X",
        "option name X type",
        "option name X type spin default 1",
        "option name X type spin default one min 0 max 1",
        "option name X type check default maybe",
        "option name X type nonsense default 1",
        "option type spin default 1 min 0 max 1",
        "option foo",
    ] {
        assert_eq!(parse_line(bad), UciMessage::Malformed, "{bad}");
    }
}

#[test]
fn ut_en_012_info_with_depth_score_multipv_pv_and_bounds() {
    let m = parse_line(
        "info depth 18 seldepth 24 multipv 2 score cp -35 nodes 123456 nps 999 hashfull 12 tbhits 0 time 140 pv e7e5 g1f3 b8c6",
    );
    assert_eq!(
        m,
        UciMessage::Info(InfoLine {
            depth: Some(18),
            multipv: Some(2),
            score: Some(Score::Cp(-35)),
            bound: false,
            pv: vec![mv("e7e5"), mv("g1f3"), mv("b8c6")],
        })
    );
    let UciMessage::Info(lower) = parse_line("info depth 5 score cp 10 lowerbound nodes 1 pv e2e4")
    else {
        panic!()
    };
    assert!(lower.bound);
    let UciMessage::Info(upper) = parse_line("info depth 5 score cp 10 upperbound pv e2e4") else {
        panic!()
    };
    assert!(upper.bound);
}

#[test]
fn ut_en_013_mate_scores_and_info_string() {
    let UciMessage::Info(i) = parse_line("info depth 9 score mate -3 pv a1a2") else {
        panic!()
    };
    assert_eq!(i.score, Some(Score::Mate(-3)));
    assert_eq!(
        parse_line("info string NNUE evaluation using nn-1a298aa575a0.nnue"),
        UciMessage::InfoString
    );
    // A move-looking word inside `string` is not a pv.
    assert_eq!(parse_line("info string pv e2e4"), UciMessage::InfoString);
}

#[test]
fn ut_en_014_bestmove_variants() {
    assert_eq!(
        parse_line("bestmove e2e4"),
        UciMessage::BestMove(mv("e2e4"))
    );
    assert_eq!(
        parse_line("bestmove e7e8q ponder a1a2"),
        UciMessage::BestMove(mv("e7e8q"))
    );
    assert_eq!(parse_line("bestmove (none)"), UciMessage::NoMove);
    assert_eq!(parse_line("bestmove 0000"), UciMessage::NoMove);
    assert_eq!(parse_line("bestmove zzzz"), UciMessage::MalformedBestMove);
    assert_eq!(parse_line("bestmove"), UciMessage::MalformedBestMove);
    assert_eq!(parse_line("uciok"), UciMessage::UciOk);
    assert_eq!(parse_line("readyok"), UciMessage::ReadyOk);
}

#[test]
fn ut_en_015_tokens_are_matched_by_keyword_not_position() {
    let a = parse_line("info score cp 7 depth 3 pv e2e4");
    let b = parse_line("info depth 3 nodes 1 extra tokens here score cp 7 pv e2e4");
    let (UciMessage::Info(a), UciMessage::Info(b)) = (a, b) else {
        panic!()
    };
    assert_eq!(
        (a.depth, a.score, a.pv.clone()),
        (Some(3), Some(Score::Cp(7)), vec![mv("e2e4")])
    );
    assert_eq!((b.depth, b.score, b.pv.clone()), (a.depth, a.score, a.pv));
    // A broken move ends the pv, the rest of the line is still understood.
    let UciMessage::Info(c) = parse_line("info depth 4 score cp 1 pv e2e4 nonsense e7e5") else {
        panic!()
    };
    assert_eq!(c.pv, vec![mv("e2e4")]);
}

#[test]
fn ut_en_016_unknown_lines_are_unknown() {
    assert_eq!(parse_line(""), UciMessage::Unknown(String::new()));
    assert_eq!(
        parse_line("Stockfish 19 by the Stockfish developers"),
        UciMessage::Unknown("Stockfish 19 by the Stockfish developers".into())
    );
}

fn spin(min: i64, max: i64) -> OptionSpec {
    OptionSpec {
        name: "UCI_Elo".into(),
        kind: OptionKind::Spin {
            default: min,
            min,
            max,
        },
    }
}

fn set(name: &str, v: OptionValue) -> SetOption {
    SetOption {
        name: name.into(),
        value: v,
    }
}

#[test]
fn ut_en_017_setoption_is_range_checked_because_stockfish_accepts_anything() {
    let s = spin(1000, 2000);
    for v in [1000, 1001, 1500, 1999, 2000] {
        let line = format_setoption(&s, &set("UCI_Elo", OptionValue::Int(v))).unwrap();
        assert_eq!(line, format!("setoption name UCI_Elo value {v}"));
    }
    for v in [999, 2001, i64::MIN, i64::MAX, -1] {
        assert!(
            format_setoption(&s, &set("UCI_Elo", OptionValue::Int(v))).is_err(),
            "{v}"
        );
    }
    // Wrong type and wrong name.
    assert!(format_setoption(&s, &set("UCI_Elo", OptionValue::Bool(true))).is_err());
    assert!(format_setoption(&s, &set("Hash", OptionValue::Int(1500))).is_err());
    let check = OptionSpec {
        name: "UCI_LimitStrength".into(),
        kind: OptionKind::Check { default: false },
    };
    assert_eq!(
        format_setoption(&check, &set("UCI_LimitStrength", OptionValue::Bool(true))).unwrap(),
        "setoption name UCI_LimitStrength value true"
    );
    assert!(format_setoption(&check, &set("UCI_LimitStrength", OptionValue::Int(1))).is_err());
    let combo = OptionSpec {
        name: "Style".into(),
        kind: OptionKind::Combo {
            default: "A".into(),
            vars: vec!["A".into(), "B".into()],
        },
    };
    assert!(format_setoption(&combo, &set("Style", OptionValue::Text("B".into()))).is_ok());
    assert!(format_setoption(&combo, &set("Style", OptionValue::Text("C".into()))).is_err());
    let string = OptionSpec {
        name: "EvalFile".into(),
        kind: OptionKind::Str {
            default: String::new(),
        },
    };
    assert!(
        format_setoption(
            &string,
            &set("EvalFile", OptionValue::Text("a\nquit".into()))
        )
        .is_err(),
        "no command injection"
    );
    assert!(
        format_setoption(
            &string,
            &set("EvalFile", OptionValue::Text("a\rquit".into()))
        )
        .is_err()
    );
}

#[test]
fn ut_en_018_position_and_go_lines() {
    assert_eq!(
        format_position(&SearchPosition::Moves {
            start: None,
            moves: vec![]
        })
        .unwrap(),
        "position startpos"
    );
    assert_eq!(
        format_position(&SearchPosition::Moves {
            start: None,
            moves: vec![mv("e2e4"), mv("e7e5")]
        })
        .unwrap(),
        "position startpos moves e2e4 e7e5"
    );
    let fen = Fen::parse("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    assert_eq!(
        format_position(&SearchPosition::Fen(fen.clone())).unwrap(),
        "position fen 4k3/8/8/8/8/8/8/4K3 w - - 0 1"
    );
    assert_eq!(
        format_position(&SearchPosition::Moves {
            start: Some(fen),
            moves: vec![mv("e1e2")]
        })
        .unwrap(),
        "position fen 4k3/8/8/8/8/8/8/4K3 w - - 0 1 moves e1e2"
    );
    let l = |ms: u64, d, n| SearchLimits::new(Duration::from_millis(ms), d, n).unwrap();
    assert_eq!(format_go(&l(250, None, None)), "go movetime 250");
    assert_eq!(
        format_go(&l(250, Some(12), Some(5000))),
        "go movetime 250 depth 12 nodes 5000"
    );
    let sub_ms = SearchLimits::new(Duration::from_micros(200), None, None).unwrap();
    assert_eq!(
        format_go(&sub_ms),
        "go movetime 1",
        "UCI has no zero movetime"
    );
}

#[test]
fn ut_en_019_round_trips() {
    for m in ["a1h8", "e7e8q", "h2h1n"] {
        assert_eq!(
            parse_line(&format!("bestmove {m}")),
            UciMessage::BestMove(mv(m))
        );
    }
    let spec = OptionSpec {
        name: "Skill Level".into(),
        kind: OptionKind::Spin {
            default: 20,
            min: 0,
            max: 20,
        },
    };
    let text = "option name Skill Level type spin default 20 min 0 max 20";
    assert_eq!(parse_line(text), UciMessage::Option(spec));
}

#[test]
fn ut_en_019b_the_line_splitter_splits_trims_and_enforces_the_cap() {
    let mut s = LineSplitter::new(16);
    s.push(b"uci\r\nread");
    assert_eq!(s.next_line().unwrap().unwrap(), "uci");
    assert!(s.next_line().is_none());
    s.push(b"yok\n");
    assert_eq!(s.next_line().unwrap().unwrap(), "readyok");
    // Exactly the cap is fine, one more is not.
    s.push(&[b'a'; 16]);
    s.push(b"\n");
    assert_eq!(s.next_line().unwrap().unwrap().len(), 16);
    s.push(&[b'a'; 17]);
    s.push(b"\n");
    assert!(s.next_line().unwrap().is_err());
    assert!(s.next_line().unwrap().is_err(), "failure is sticky");
    assert_eq!(s.buffered(), 0);
    // An over-long line that follows a short one in the same chunk is still caught.
    let mut s2 = LineSplitter::new(8);
    s2.push(b"ok\n");
    s2.push(&[b'z'; 40]);
    s2.push(b"\n");
    assert_eq!(s2.next_line().unwrap().unwrap(), "ok");
    assert!(s2.next_line().unwrap().is_err());
    // A growing partial line over the cap fails before its newline arrives.
    let mut s3 = LineSplitter::new(8);
    s3.push(&[b'q'; 9]);
    assert!(s3.next_line().unwrap().is_err());
}

proptest! {
    // FZ-EN-001: random-input property tests on stable Rust (D-76: no coverage-guided fuzzer).
    #[test]
    fn fz_en_001_parse_line_never_panics_on_arbitrary_text(s in ".*") {
        let _ = parse_line(&s);
    }

    #[test]
    fn fz_en_001_parse_line_never_panics_on_arbitrary_bytes(b in proptest::collection::vec(any::<u8>(), 0..300)) {
        let _ = parse_line(&String::from_utf8_lossy(&b));
    }

    #[test]
    fn fz_en_001_ok_results_are_well_formed(s in "(info|bestmove|option|id) [a-z0-9 ()-]{0,80}") {
        match parse_line(&s) {
            UciMessage::BestMove(m) => prop_assert!(UciMove::parse(m.as_str()).is_ok()),
            UciMessage::Info(i) => {
                for m in &i.pv { prop_assert!(UciMove::parse(m.as_str()).is_ok()); }
            }
            UciMessage::Option(o) => {
                prop_assert!(!o.name.is_empty());
                if let OptionKind::Spin { .. } = o.kind {}
            }
            _ => {}
        }
    }

    #[test]
    fn fz_en_001_the_line_splitter_terminates_and_stays_bounded(
        chunks in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..200), 0..60),
        cap in 8usize..64,
    ) {
        let mut s = LineSplitter::new(cap);
        let mut failed = false;
        for c in &chunks {
            s.push(c);
            let mut guard = 0;
            while let Some(r) = s.next_line() {
                guard += 1;
                prop_assert!(guard < 1000, "next_line must terminate on a finite buffer");
                match r {
                    Ok(_) => prop_assert!(!failed, "no Ok after a failure"),
                    Err(_) => { failed = true; break; }
                }
            }
            prop_assert!(s.buffered() <= cap, "buffered {} > cap {}", s.buffered(), cap);
        }
    }

    #[test]
    fn fz_en_001_ascii_lines_over_the_cap_are_never_returned(
        lines in proptest::collection::vec("[a-z ]{0,40}", 0..30),
        cap in 8usize..40,
    ) {
        let mut s = LineSplitter::new(cap);
        let mut text = String::new();
        for l in &lines { text.push_str(l); text.push('\n'); }
        s.push(text.as_bytes());
        while let Some(r) = s.next_line() {
            match r {
                Ok(l) => prop_assert!(l.len() <= cap),
                Err(_) => break,
            }
        }
    }
}

#[test]
fn ut_en_019c_the_option_table_and_defaults() {
    use chess_engine::uci::UciOptions;
    let mut t = UciOptions::new();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
    for l in [
        "option name Hash type spin default 16 min 1 max 1024",
        "option name UCI_LimitStrength type check default false",
        "option name Style type combo default Normal var Solid var Normal",
        "option name Clear Hash type button",
        "option name EvalFile type string default x.nnue",
    ] {
        let UciMessage::Option(s) = parse_line(l) else {
            panic!("{l}")
        };
        t.insert(s);
    }
    assert_eq!(t.len(), 5);
    assert!(!t.is_empty());
    assert_eq!(t.iter().count(), 5);
    let default = |n: &str| t.get(n).unwrap().default_text();
    assert_eq!(default("Hash").as_deref(), Some("16"));
    assert_eq!(default("UCI_LimitStrength").as_deref(), Some("false"));
    assert_eq!(default("Style").as_deref(), Some("Normal"));
    assert_eq!(default("EvalFile").as_deref(), Some("x.nnue"));
    assert_eq!(default("Clear Hash"), None, "a button has no value");
    assert_eq!(t.uci_elo_range(), None);
    let UciMessage::Option(elo) =
        parse_line("option name UCI_Elo type spin default 5 min -1 max 9")
    else {
        panic!()
    };
    t.insert(elo);
    assert_eq!(
        t.uci_elo_range(),
        None,
        "a negative minimum is not a usable u32 range"
    );
}
