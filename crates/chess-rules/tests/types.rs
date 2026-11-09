//! UT-RUL-001, UT-RUL-002 (DD-RUL-001): core value types. Requirement RULE-1.

use chess_rules::{File, Move, MoveParseError, PieceKind, Rank, Square, SquareError};

#[test]
fn ut_rul_001_rule_1_square_text_round_trip_for_all_64_squares() {
    let mut count = 0;
    for idx in 0..64u8 {
        let s = Square::from_index(idx).unwrap();
        assert_eq!(s.index(), idx);
        let text = s.to_string();
        assert_eq!(text.len(), 2);
        assert_eq!(Square::from_algebraic(&text).unwrap(), s);
        count += 1;
    }
    assert_eq!(count, 64);
}

#[test]
fn ut_rul_001_rule_1_square_corners_and_coordinates() {
    assert_eq!(Square::from_algebraic("a1").unwrap().index(), 0);
    assert_eq!(Square::from_algebraic("h1").unwrap().index(), 7);
    assert_eq!(Square::from_algebraic("a8").unwrap().index(), 56);
    assert_eq!(Square::from_algebraic("h8").unwrap().index(), 63);
    let e4 = Square::new(File::new(4).unwrap(), Rank::new(3).unwrap());
    assert_eq!(e4.to_string(), "e4");
    assert_eq!(e4.file().index(), 4);
    assert_eq!(e4.rank().index(), 3);
}

#[test]
fn ut_rul_001_rule_1_square_rejects_out_of_range_and_bad_text() {
    assert!(matches!(
        Square::from_index(64),
        Err(SquareError::OutOfRange { .. })
    ));
    assert!(matches!(
        Square::from_index(255),
        Err(SquareError::OutOfRange { .. })
    ));
    assert!(matches!(File::new(8), Err(SquareError::OutOfRange { .. })));
    assert!(matches!(Rank::new(8), Err(SquareError::OutOfRange { .. })));
    for bad in [
        "", "a", "a9", "i1", "A1", "a0", "e44", "1a", " a1", "a1 ", "é1",
    ] {
        assert!(
            Square::from_algebraic(bad).is_err(),
            "{bad:?} must be rejected"
        );
    }
}

#[test]
fn ut_rul_002_rule_1_move_uci_round_trip_plain_and_promotion() {
    for text in [
        "e2e4", "a1h8", "e7e8q", "a7a8r", "b2b1b", "h7h8n", "e1g1", "e1c1",
    ] {
        let m = Move::from_uci(text).unwrap();
        assert_eq!(m.to_uci(), text);
    }
    let m = Move::from_uci("e7e8q").unwrap();
    assert_eq!(m.from.to_string(), "e7");
    assert_eq!(m.to.to_string(), "e8");
    assert_eq!(m.promotion, Some(PieceKind::Queen));
    assert_eq!(Move::from_uci("e2e4").unwrap().promotion, None);
}

#[test]
fn ut_rul_002_rule_1_from_uci_rejects_upper_case_wrong_length_and_bad_promotion() {
    for bad in [
        "", "e2", "e2e", "E2E4", "e2E4", "e7e8Q", "e7e8k", "e7e8p", "e7e8x", "e2e4e", "e2e4qq",
        "e2e9", "i2e4", "e2-e4", "e2e4 ", " e2e4", "0000",
    ] {
        assert_eq!(
            Move::from_uci(bad),
            Err(MoveParseError::BadUci),
            "{bad:?} must be rejected"
        );
    }
}
