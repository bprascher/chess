//! UT-EN-030..039 and UT-EN-062 (limits, clock budget, SplitMix64, the seeded error pick).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]
mod common;

use std::sync::Mutex;
use std::time::Duration;

use chess_engine::{
    Engine, EngineError, EnginePersonality, Health, MoveKind, PvLine, Score, SearchLimits,
    SearchRequest, SearchResult, SplitMix64, UciMove, clock_budget, select_move,
};
use proptest::prelude::*;

const VECTORS: &str = include_str!("data/splitmix64_vectors.txt");
const DOMAIN_VECTORS: &str = include_str!("../../chess-domain/tests/data/splitmix64_vectors.txt");

fn pers(movetime_ms: u32, depth: u32) -> EnginePersonality {
    EnginePersonality {
        skill_level: 10,
        depth_limit: depth,
        movetime_ms,
        uci_elo: None,
        error_rate: 0.0,
        error_multipv: 3,
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn ut_en_030_movetime_is_the_minimum_of_the_three_limits() {
    let l = |max, budget| {
        SearchLimits::for_move(&pers(1000, 0), max, budget)
            .unwrap()
            .movetime
    };
    assert_eq!(l(None, None), ms(1000));
    assert_eq!(l(Some(ms(500)), None), ms(500));
    assert_eq!(l(None, Some(ms(300))), ms(300));
    assert_eq!(l(Some(ms(500)), Some(ms(800))), ms(500));
    assert_eq!(
        l(Some(ms(5000)), Some(ms(5000))),
        ms(1000),
        "a limit can only shorten, never lengthen"
    );
}

proptest! {
    #[test]
    fn ut_en_031_the_result_never_exceeds_any_limit(
        own in 1u32..60_000, max in proptest::option::of(1u64..60_000), budget in proptest::option::of(1u64..60_000),
    ) {
        let r = SearchLimits::for_move(&pers(own, 0), max.map(ms), budget.map(ms)).unwrap().movetime;
        prop_assert!(r <= ms(u64::from(own)));
        if let Some(m) = max { prop_assert!(r <= ms(m)); }
        if let Some(b) = budget { prop_assert!(r <= ms(b)); }
        prop_assert!(!r.is_zero());
    }
}

#[test]
fn ut_en_032_depth_zero_means_no_depth_cap() {
    assert_eq!(
        SearchLimits::for_move(&pers(100, 0), None, None)
            .unwrap()
            .depth,
        None
    );
    assert_eq!(
        SearchLimits::for_move(&pers(100, 12), None, None)
            .unwrap()
            .depth,
        Some(12)
    );
}

#[test]
fn ut_en_033_clock_budget_vectors_with_the_cap_applied_last() {
    let b = |rem, inc| clock_budget(ms(rem), ms(inc), 0);
    assert_eq!(b(60_000, 0), ms(2_000), "remaining / 30");
    assert_eq!(
        b(60_000, 4_000),
        ms(5_000),
        "remaining / 30 + 3 * increment / 4"
    );
    assert_eq!(b(1_000, 0), ms(50), "the 50 ms floor");
    assert_eq!(
        b(90, 0),
        ms(45),
        "under 100 ms remaining the cap (half) beats the floor"
    );
    assert_eq!(b(60, 0), ms(30));
    assert_eq!(b(0, 0), ms(0));
    assert_eq!(b(100, 0), ms(50));
    assert_eq!(
        b(10_000, 100_000),
        ms(5_000),
        "never more than half the remaining time"
    );
    assert!(
        clock_budget(Duration::MAX, Duration::MAX, 0) > Duration::ZERO,
        "no overflow panic"
    );
}

proptest! {
    #[test]
    fn ut_en_033b_the_budget_is_never_more_than_half_the_remaining_time(rem in 0u64..10_000_000, inc in 0u64..100_000, ply in 0u32..400) {
        prop_assert!(clock_budget(ms(rem), ms(inc), ply) <= ms(rem) / 2);
    }
}

#[test]
fn ut_en_034_a_zero_limit_is_an_error_not_a_zero_movetime() {
    let r = SearchLimits::for_move(&pers(1000, 0), None, Some(Duration::ZERO));
    assert!(matches!(r, Err(EngineError::InvalidRequest { .. })));
    let r = SearchLimits::for_move(&pers(0, 0), None, None);
    assert!(matches!(r, Err(EngineError::InvalidRequest { .. })));
}

/// A canned engine for the selector: records the requested multipv, returns N lines.
struct Canned {
    lines: Vec<&'static str>,
    asked: Mutex<Vec<u8>>,
}

fn mv(t: &str) -> UciMove {
    UciMove::parse(t).unwrap()
}

impl Engine for Canned {
    async fn search(&self, req: SearchRequest) -> Result<SearchResult, EngineError> {
        self.asked.lock().unwrap().push(req.multipv);
        let n = usize::from(req.multipv).min(self.lines.len());
        let lines: Vec<PvLine> = self
            .lines
            .iter()
            .take(n)
            .enumerate()
            .map(|(i, m)| PvLine {
                multipv: u8::try_from(i + 1).unwrap(),
                score: Score::Cp(0),
                depth: 5,
                pv: vec![mv(m)],
            })
            .collect();
        let best = mv(self.lines[0]);
        Ok(SearchResult {
            best: best.clone(),
            pv: vec![best],
            score: Score::Cp(0),
            depth: 5,
            lines,
            stopped_early: false,
            elapsed: Duration::ZERO,
        })
    }
    fn stop(&self) {}
    fn health(&self) -> Health {
        Health::Ready
    }
}

fn canned(lines: Vec<&'static str>) -> Canned {
    Canned {
        lines,
        asked: Mutex::new(vec![]),
    }
}

fn req(seed: u64) -> SearchRequest {
    let mut r = common::start_req(100);
    r.seed = seed;
    r
}

fn erring(rate: f64, multipv: u8) -> EnginePersonality {
    EnginePersonality {
        error_rate: rate,
        error_multipv: multipv,
        ..pers(100, 0)
    }
}

#[tokio::test]
async fn ut_en_035_the_personality_is_never_mutated() {
    let e = canned(vec!["a2a3", "b2b3", "c2c3"]);
    let p = erring(0.5, 3);
    let before = p.clone();
    for seed in 0..50 {
        select_move(&e, req(seed), &p).await.unwrap();
    }
    assert_eq!(p, before);
}

#[test]
fn ut_en_036_splitmix64_matches_the_shared_vector_file() {
    let mut checked = 0;
    for l in VECTORS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let mut f = l.split(' ');
        let seed: u64 = f.next().unwrap().parse().unwrap();
        let idx: usize = f.next().unwrap().parse().unwrap();
        let want = u64::from_str_radix(f.next().unwrap(), 16).unwrap();
        let mut g = SplitMix64::new(seed);
        for _ in 0..idx {
            g.next_u64();
        }
        assert_eq!(g.next_u64(), want, "seed {seed} index {idx}");
        checked += 1;
    }
    assert_eq!(checked, 20);
    let mut g = SplitMix64::new(0);
    assert_eq!(
        g.next_u64(),
        0xE220_A839_7B1D_CDAF,
        "the published first output for seed 0"
    );
}

proptest! {
    #[test]
    fn ut_en_036b_next_f64_is_in_the_unit_interval(seed in any::<u64>()) {
        let mut g = SplitMix64::new(seed);
        for _ in 0..20 { let u = g.next_f64(); prop_assert!((0.0..1.0).contains(&u)); }
    }
}

#[tokio::test]
async fn ut_en_037_same_seed_same_roll_and_same_pick_and_the_rate_is_respected() {
    let e = canned(vec!["a2a3", "b2b3", "c2c3", "d2d3"]);
    let p = erring(0.5, 4);
    let mut errors = 0u32;
    for seed in 0..2000u64 {
        let a = select_move(&e, req(seed), &p).await.unwrap();
        let b = select_move(&e, req(seed), &p).await.unwrap();
        assert_eq!((a.mv.clone(), a.kind), (b.mv, b.kind), "seed {seed}");
        if a.kind == MoveKind::Error {
            errors += 1;
        }
    }
    assert!(
        (900..=1100).contains(&errors),
        "about half of 2000 rolls err: {errors}"
    );
}

#[tokio::test]
async fn ut_en_038_an_error_pick_never_chooses_the_best_line() {
    let e = canned(vec!["a2a3", "b2b3", "c2c3", "d2d3"]);
    let p = erring(1.0, 4);
    let mut seen = std::collections::HashSet::new();
    for seed in 0..500u64 {
        let c = select_move(&e, req(seed), &p).await.unwrap();
        assert_eq!(c.kind, MoveKind::Error);
        assert_ne!(c.mv.as_str(), "a2a3");
        seen.insert(c.mv.as_str().to_string());
    }
    assert_eq!(seen.len(), 3, "every non-best line is reachable");
}

#[tokio::test]
async fn ut_en_039_fallbacks_to_the_best_move_and_the_multipv_asked() {
    // One legal move: one line, so Best even on an error roll.
    let one = canned(vec!["a2a3"]);
    let c = select_move(&one, req(1), &erring(1.0, 3)).await.unwrap();
    assert_eq!((c.mv.as_str(), c.kind), ("a2a3", MoveKind::Best));
    // Rate zero: always Best, one line asked.
    let e = canned(vec!["a2a3", "b2b3"]);
    for seed in 0..50 {
        assert_eq!(
            select_move(&e, req(seed), &erring(0.0, 3))
                .await
                .unwrap()
                .kind,
            MoveKind::Best
        );
    }
    assert!(e.asked.lock().unwrap().iter().all(|m| *m == 1));
    // error_multipv below two cannot err.
    let e2 = canned(vec!["a2a3", "b2b3"]);
    assert_eq!(
        select_move(&e2, req(1), &erring(1.0, 1))
            .await
            .unwrap()
            .kind,
        MoveKind::Best
    );
    // An error roll asks for error_multipv lines.
    let e3 = canned(vec!["a2a3", "b2b3", "c2c3"]);
    select_move(&e3, req(1), &erring(1.0, 3)).await.unwrap();
    assert_eq!(e3.asked.lock().unwrap().as_slice(), &[3]);
}

#[test]
fn ut_en_062_the_vector_file_is_byte_identical_to_the_one_in_chess_domain() {
    assert_eq!(VECTORS.as_bytes(), DOMAIN_VECTORS.as_bytes());
}
