//! UT-EN-020..026 (option plan). Ranges are expressed through the constant, never as a literal.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]

use chess_engine::uci::{OptionValue, SetOption, UciMessage, UciOptions, parse_line};
use chess_engine::{
    EngineConfig, EngineError, EnginePersonality, InstanceProfile, OptionPlan, PINNED_UCI_ELO_RANGE,
};

fn table(extra_elo: Option<(u32, u32)>, skill: bool, limit: bool) -> UciOptions {
    let mut lines = vec![
        "option name Threads type spin default 1 min 1 max 1024".to_string(),
        "option name Hash type spin default 16 min 1 max 33554432".to_string(),
        "option name MultiPV type spin default 1 min 1 max 500".to_string(),
    ];
    if skill {
        lines.push("option name Skill Level type spin default 20 min 0 max 20".into());
    }
    if limit {
        lines.push("option name UCI_LimitStrength type check default false".into());
    }
    if let Some((min, max)) = extra_elo {
        lines.push(format!(
            "option name UCI_Elo type spin default {min} min {min} max {max}"
        ));
    }
    let mut t = UciOptions::new();
    for l in lines {
        let UciMessage::Option(s) = parse_line(&l) else {
            panic!("{l}")
        };
        t.insert(s);
    }
    t
}

fn pinned() -> (u32, u32) {
    (PINNED_UCI_ELO_RANGE.min, PINNED_UCI_ELO_RANGE.max)
}

fn p(elo: Option<u32>, skill: u8) -> EnginePersonality {
    EnginePersonality {
        skill_level: skill,
        depth_limit: 0,
        movetime_ms: 1000,
        uci_elo: elo,
        error_rate: 0.0,
        error_multipv: 3,
    }
}

fn find<'a>(o: &'a [SetOption], name: &str) -> Option<&'a OptionValue> {
    o.iter().find(|s| s.name == name).map(|s| &s.value)
}

#[test]
fn ut_en_020_elo_inside_the_range_is_sent_with_limit_strength_on() {
    let (min, max) = pinned();
    for e in [min, min + 1, (min + max) / 2, max - 1, max] {
        let plan = OptionPlan::build(
            &InstanceProfile::Computer(p(Some(e), 3)),
            &EngineConfig::default(),
            &table(Some(pinned()), true, true),
        )
        .unwrap();
        assert_eq!(
            find(&plan.options, "UCI_Elo"),
            Some(&OptionValue::Int(i64::from(e)))
        );
        assert_eq!(
            find(&plan.options, "UCI_LimitStrength"),
            Some(&OptionValue::Bool(true))
        );
        assert_eq!(
            find(&plan.options, "Skill Level"),
            None,
            "UCI_Elo replaces Skill Level"
        );
        assert!(!plan.report.clamped);
    }
}

#[test]
fn ut_en_021_elo_outside_the_range_is_clamped_for_the_search_only_and_reported() {
    let (min, max) = pinned();
    for (asked, sent) in [(min - 1, min), (0, min), (max + 1, max), (u32::MAX, max)] {
        let personality = p(Some(asked), 3);
        let before = personality.clone();
        let plan = OptionPlan::build(
            &InstanceProfile::Computer(personality.clone()),
            &EngineConfig::default(),
            &table(Some(pinned()), true, true),
        )
        .unwrap();
        assert_eq!(
            find(&plan.options, "UCI_Elo"),
            Some(&OptionValue::Int(i64::from(sent)))
        );
        assert!(plan.report.clamped);
        assert_eq!(
            personality, before,
            "the stored value is never changed (D-46)"
        );
    }
}

#[test]
fn ut_en_022_elo_needs_the_advertised_options() {
    let (min, _) = pinned();
    let cfg = EngineConfig::default();
    let prof = InstanceProfile::Computer(p(Some(min), 3));
    let no_elo = OptionPlan::build(&prof, &cfg, &table(None, true, true));
    assert!(
        matches!(no_elo, Err(EngineError::InvalidPersonality { ref option }) if option == "UCI_Elo")
    );
    let no_limit = OptionPlan::build(&prof, &cfg, &table(Some(pinned()), true, false));
    assert!(
        matches!(no_limit, Err(EngineError::InvalidPersonality { ref option }) if option == "UCI_LimitStrength")
    );
}

#[test]
fn ut_en_023_skill_level_path_checks_the_advertised_bounds() {
    let cfg = EngineConfig::default();
    let t = table(Some(pinned()), true, true);
    for ok in [0u8, 1, 19, 20] {
        let plan = OptionPlan::build(&InstanceProfile::Computer(p(None, ok)), &cfg, &t).unwrap();
        assert_eq!(
            find(&plan.options, "Skill Level"),
            Some(&OptionValue::Int(i64::from(ok)))
        );
        assert_eq!(
            find(&plan.options, "UCI_LimitStrength"),
            Some(&OptionValue::Bool(false))
        );
        assert_eq!(find(&plan.options, "UCI_Elo"), None);
    }
    for bad in [21u8, 255] {
        let r = OptionPlan::build(&InstanceProfile::Computer(p(None, bad)), &cfg, &t);
        assert!(
            matches!(r, Err(EngineError::InvalidPersonality { ref option }) if option == "Skill Level"),
            "{bad}"
        );
    }
    let missing = OptionPlan::build(
        &InstanceProfile::Computer(p(None, 5)),
        &cfg,
        &table(Some(pinned()), false, true),
    );
    assert!(matches!(
        missing,
        Err(EngineError::InvalidPersonality { .. })
    ));
}

#[test]
fn ut_en_024_the_report_flags_a_handshake_range_that_differs_from_the_pin() {
    let cfg = EngineConfig::default();
    let prof = InstanceProfile::Analysis;
    let same = OptionPlan::build(&prof, &cfg, &table(Some(pinned()), true, true)).unwrap();
    assert!(!same.report.differs_from_pinned);
    let (min, max) = pinned();
    for other in [(min + 1, max), (min, max + 1), (min - 1, max)] {
        let r = OptionPlan::build(&prof, &cfg, &table(Some(other), true, true)).unwrap();
        assert!(r.report.differs_from_pinned, "{other:?}");
    }
    let none = OptionPlan::build(&prof, &cfg, &table(None, true, true)).unwrap();
    assert!(none.report.differs_from_pinned);
}

#[test]
fn ut_en_025_threads_and_hash_follow_the_kind_and_the_config() {
    let cfg = EngineConfig {
        computer_threads: 3,
        computer_hash_mb: 33,
        analysis_threads: 4,
        analysis_hash_mb: 77,
        ..EngineConfig::default()
    };
    let t = table(Some(pinned()), true, true);
    let c = OptionPlan::build(&InstanceProfile::Computer(p(None, 5)), &cfg, &t).unwrap();
    assert_eq!(find(&c.options, "Threads"), Some(&OptionValue::Int(3)));
    assert_eq!(find(&c.options, "Hash"), Some(&OptionValue::Int(33)));
    let a = OptionPlan::build(&InstanceProfile::Analysis, &cfg, &t).unwrap();
    assert_eq!(find(&a.options, "Threads"), Some(&OptionValue::Int(4)));
    assert_eq!(find(&a.options, "Hash"), Some(&OptionValue::Int(77)));
    assert_eq!(
        find(&a.options, "UCI_LimitStrength"),
        Some(&OptionValue::Bool(false)),
        "analysis is full strength"
    );
    assert_eq!(find(&a.options, "Skill Level"), None);
}

#[test]
fn ut_en_026_only_advertised_options_are_planned() {
    let mut t = UciOptions::new();
    let UciMessage::Option(s) =
        parse_line("option name Skill Level type spin default 20 min 0 max 20")
    else {
        panic!()
    };
    t.insert(s);
    let plan = OptionPlan::build(
        &InstanceProfile::Computer(p(None, 5)),
        &EngineConfig::default(),
        &t,
    )
    .unwrap();
    assert!(find(&plan.options, "Threads").is_none());
    assert!(find(&plan.options, "Hash").is_none());
    assert!(find(&plan.options, "UCI_LimitStrength").is_none());
    assert!(find(&plan.options, "Skill Level").is_some());
}
