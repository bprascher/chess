//! UT-DOM-002 (SplitMix64 golden outputs, `below`), UT-DOM-101 (`error_pick_seed`), UT-DOM-102
//! (the vector file). Written before the code. The vector file and the `error_pick_seed` values
//! come from an independent Python 3 implementation of the published algorithm, not from this
//! crate.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{GameId, Rng, SplitMix64, Uuid, error_pick_seed};

const VECTORS: &str = include_str!("data/splitmix64_vectors.txt");

fn vectors() -> Vec<(u64, usize, u64)> {
    VECTORS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split(' ');
            let seed = f.next().unwrap().parse().unwrap();
            let idx = f.next().unwrap().parse().unwrap();
            let out = u64::from_str_radix(f.next().unwrap(), 16).unwrap();
            (seed, idx, out)
        })
        .collect()
}

#[test]
fn ut_dom_002_ff_8_splitmix64_matches_the_shared_vector_file() {
    let rows = vectors();
    assert_eq!(rows.len(), 20, "four seeds, five outputs each");
    for seed in [0u64, 1, 3_735_928_559, u64::MAX] {
        let mut g = SplitMix64::new(seed);
        for (_, idx, want) in rows.iter().filter(|r| r.0 == seed) {
            assert_eq!(g.next_u64(), *want, "seed {seed} output {idx}");
        }
    }
}

#[test]
fn ut_dom_002_splitmix64_published_first_outputs_for_seed_zero() {
    // The values every SplitMix64 reference gives for seed 0.
    let mut g = SplitMix64::new(0);
    assert_eq!(g.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(g.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(g.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn ut_dom_002_below_stays_in_range_for_1_2_and_2_pow_63() {
    let mut g = SplitMix64::new(7);
    for n in [1u64, 2, 3, 1000, 1 << 63, u64::MAX] {
        for _ in 0..200 {
            assert!(g.below(n) < n, "n = {n}");
        }
    }
    assert_eq!(g.below(1), 0);
    assert_eq!(g.below(0), 0, "n = 0 returns 0 rather than panicking");
    // both values of n = 2 occur
    let seen: std::collections::BTreeSet<u64> = (0..64).map(|_| g.below(2)).collect();
    assert_eq!(seen.len(), 2);
}

fn gid(n: u128) -> GameId {
    GameId::from_uuid(Uuid::from_u128(n))
}

#[test]
fn ut_dom_101_tr_12_error_pick_seed_golden_values_from_the_python_oracle() {
    assert_eq!(error_pick_seed(gid(0), 0), 0x2382_75bc_38fc_be91);
    assert_eq!(error_pick_seed(gid(1), 0), 0x44e5_b981_00c6_7fb0);
    assert_eq!(error_pick_seed(gid(1), 1), 0x4653_9195_8fc6_1f13);
    assert_eq!(
        error_pick_seed(gid(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef), 40),
        0xccd9_cb7f_e816_8839
    );
    assert_eq!(error_pick_seed(gid(u128::MAX), 9999), 0x2814_6ffb_56d0_ca0f);
}

#[test]
fn ut_dom_101_error_pick_seed_is_deterministic_and_differs_per_game_and_ply() {
    let a = error_pick_seed(gid(5), 3);
    assert_eq!(a, error_pick_seed(gid(5), 3));
    let mut seen = std::collections::BTreeSet::new();
    for g in 0..20u128 {
        for ply in 0..20u32 {
            assert!(seen.insert(error_pick_seed(gid(g), ply)), "{g} {ply}");
        }
    }
    // the high and the low half of the id both matter
    assert_ne!(error_pick_seed(gid(1 << 64), 0), error_pick_seed(gid(1), 0));
}
