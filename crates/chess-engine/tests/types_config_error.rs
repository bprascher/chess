//! UT-EN-001..006 (types), UT-EN-050 (config), UT-EN-060 and CT-EN-060 (errors), UT-EN-061.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]

use std::time::Duration;

use chess_engine::{
    EngineConfig, EngineError, ErrorClass, PINNED_UCI_ELO_RANGE, SearchLimits, SearchPosition,
    SearchRequest, UciMove,
};

#[test]
fn ut_en_001_uci_move_accepts_four_and_five_character_moves() {
    for ok in ["e2e4", "a1h8", "e7e8q", "a7a8r", "b7b8b", "h2h1n"] {
        assert_eq!(UciMove::parse(ok).unwrap().as_str(), ok);
        assert_eq!(UciMove::parse(ok).unwrap().to_string(), ok);
    }
}

#[test]
fn ut_en_002_uci_move_rejects_every_bad_shape() {
    for bad in [
        "", "e2", "e2e", "e2e4q1", "E2E4", "i2e4", "e0e4", "e2e9", "e2e4k", "e2e4x", "0000",
        "(none)", "e2 e4", "e2e4\n", "é2e4", "e2e4 ",
    ] {
        assert!(UciMove::parse(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn ut_en_003_search_limits_need_a_positive_movetime_depth_and_nodes() {
    assert!(SearchLimits::new(Duration::from_millis(1), None, None).is_ok());
    assert!(SearchLimits::new(Duration::ZERO, None, None).is_err());
    assert!(SearchLimits::new(Duration::from_secs(1), Some(0), None).is_err());
    assert!(SearchLimits::new(Duration::from_secs(1), None, Some(0)).is_err());
    assert!(SearchLimits::new(Duration::from_secs(1), Some(12), Some(1000)).is_ok());
}

#[test]
fn ut_en_004_public_fields_cannot_bypass_validation() {
    let limits = SearchLimits {
        movetime: Duration::ZERO,
        depth: None,
        nodes: None,
    };
    let req = SearchRequest::new(
        SearchPosition::Moves {
            start: None,
            moves: vec![],
        },
        limits,
    );
    assert!(matches!(
        req.validate(8),
        Err(EngineError::InvalidRequest { .. })
    ));
}

#[test]
fn ut_en_005_multipv_is_bounded_by_one_and_the_cap() {
    let limits = SearchLimits::new(Duration::from_secs(1), None, None).unwrap();
    let mut req = SearchRequest::new(
        SearchPosition::Moves {
            start: None,
            moves: vec![],
        },
        limits,
    );
    for (n, ok) in [(0u8, false), (1, true), (8, true), (9, false), (255, false)] {
        req.multipv = n;
        assert_eq!(req.validate(8).is_ok(), ok, "multipv {n}");
    }
}

#[test]
fn ut_en_006_a_new_request_defaults_to_one_line_no_new_game_no_updates() {
    let limits = SearchLimits::new(Duration::from_secs(1), None, None).unwrap();
    let req = SearchRequest::new(
        SearchPosition::Moves {
            start: None,
            moves: vec![],
        },
        limits,
    );
    assert_eq!(req.multipv, 1);
    assert!(!req.new_game);
    assert!(req.updates.is_none());
}

#[test]
fn ut_en_050_config_defaults_are_valid_and_each_bad_field_is_named() {
    assert_eq!(EngineConfig::default().validate(), Ok(()));
    let d = EngineConfig::default;
    let cases: Vec<(EngineConfig, &str)> = vec![
        (
            EngineConfig {
                handshake_timeout: Duration::ZERO,
                ..d()
            },
            "handshake_timeout",
        ),
        (
            EngineConfig {
                soft_stop_grace: Duration::from_secs(2),
                ..d()
            },
            "soft_stop_grace",
        ),
        (
            EngineConfig {
                drain_timeout: Duration::ZERO,
                ..d()
            },
            "drain_timeout",
        ),
        (
            EngineConfig {
                line_cap: 10,
                ..d()
            },
            "line_cap",
        ),
        (
            EngineConfig {
                multipv_cap: 0,
                ..d()
            },
            "multipv_cap",
        ),
        (
            EngineConfig {
                computer_threads: 0,
                ..d()
            },
            "threads",
        ),
        (
            EngineConfig {
                analysis_hash_mb: 0,
                ..d()
            },
            "hash_mb",
        ),
        (
            EngineConfig {
                pinned_id_text: String::new(),
                ..d()
            },
            "pinned_id_text",
        ),
    ];
    for (cfg, field) in cases {
        assert_eq!(cfg.validate(), Err(EngineError::InvalidConfig { field }));
    }
}

#[test]
fn ut_en_060_display_never_shows_a_path_or_engine_output() {
    let all = all_errors();
    for e in &all {
        let text = e.to_string();
        assert!(!text.contains('/'), "{text}");
        assert!(!text.contains("SECRET-ENGINE-TEXT"), "{text}");
        assert!(!text.is_empty());
    }
}

fn all_errors() -> Vec<EngineError> {
    vec![
        EngineError::BinaryMissing,
        EngineError::NotExecutable,
        EngineError::InvalidOverride,
        EngineError::SpawnFailed,
        EngineError::HandshakeTimeout,
        EngineError::WrongVersion {
            found: "SECRET-ENGINE-TEXT".into(),
        },
        EngineError::ProcessExited { code: Some(3) },
        EngineError::SearchTimeout,
        EngineError::BadOutput,
        EngineError::Preempted { partial: None },
        EngineError::InvalidPersonality {
            option: "Skill Level".into(),
        },
        EngineError::InvalidRequest { reason: "r" },
        EngineError::InvalidConfig { field: "f" },
        EngineError::Busy,
        EngineError::Closed,
    ]
}

#[test]
fn ct_en_060_every_error_maps_to_a_documented_class_and_cause() {
    let expect = [
        ErrorClass::Unavailable("binary_missing"),
        ErrorClass::Unavailable("binary_missing"),
        ErrorClass::Unavailable("binary_missing"),
        ErrorClass::Unavailable("spawn_failed"),
        ErrorClass::Unavailable("handshake_timeout"),
        ErrorClass::Unavailable("wrong_version"),
        ErrorClass::Unavailable("crashed"),
        ErrorClass::Unavailable("timeout"),
        ErrorClass::Unavailable("bad_output"),
        ErrorClass::NotAFailure,
        ErrorClass::Internal,
        ErrorClass::Internal,
        ErrorClass::Internal,
        ErrorClass::Internal,
        ErrorClass::Internal,
    ];
    let all = all_errors();
    assert_eq!(all.len(), expect.len());
    for (e, want) in all.iter().zip(expect) {
        assert_eq!(e.class(), want, "{e:?}");
    }
}

#[test]
fn ut_en_061_the_pinned_range_is_ordered_and_a_plain_constant() {
    // Review item: it is a `const`, so it cannot be derived from a handshake value at run
    // time. The live comparison is EI-EN-002.
    const RANGE: chess_engine::EloRange = PINNED_UCI_ELO_RANGE;
    const { assert!(RANGE.min <= RANGE.max) };
}
