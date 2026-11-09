//! UT-DOM-060 to 067, 109, 111 (DD-DOM-013, 014, 020): the seeded roster, `first_k_after`,
//! names, `Personality`. Written before the code. Test ranges are chosen relative to the class
//! baselines (400, 600, 1200, 2200, 2500), never the engine's real range (D-46).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use std::collections::BTreeSet;

use chess_domain::{
    CLASS_BOUNDS, Category, ComputerPlayerSpec, FAMILY_NAMES, GIVEN_NAMES, Personality,
    PersonalityError, ROSTER_NAMESPACE, RosterError, RosterGen, RosterSeed, UciEloRange, Uuid,
    roster_uuid,
};

const WIDE: UciEloRange = UciEloRange { min: 0, max: 4000 };
const GOLDEN: &str = include_str!("data/roster_seed1_golden.txt");

fn gen_with(seed: u32, range: UciEloRange) -> RosterGen {
    RosterGen::new(RosterSeed::new(seed), range)
}

fn never(_: &str) -> bool {
    false
}

fn ordinal(c: Category) -> u32 {
    u32::try_from(Category::ALL.iter().position(|x| *x == c).unwrap()).unwrap()
}

fn line(s: &ComputerPlayerSpec) -> String {
    let p = &s.personality;
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        s.roster_index,
        s.uuid,
        s.category,
        s.name,
        p.skill_level,
        p.depth_limit,
        p.movetime_ms,
        p.uci_elo.map_or("none".to_string(), |v| v.to_string()),
        p.error_permille,
        p.error_multipv,
        s.strict_elo_start,
        s.training_elo_start
    )
}

// ------------------------------------------------------------------ UT-DOM-060

#[test]
fn ut_dom_060_ply_4_roster_of_25_is_deterministic_from_the_seed() {
    let a = gen_with(1, WIDE).default_roster().unwrap();
    let b = gen_with(1, WIDE).default_roster().unwrap();
    assert_eq!(a.len(), 25);
    assert_eq!(a, b);
    assert_eq!(
        a.iter().map(|s| s.roster_index).collect::<Vec<_>>(),
        (0..25).collect::<Vec<_>>()
    );
    for (i, s) in a.iter().enumerate() {
        assert_eq!(s.category, Category::ALL[i % 5], "index {i}");
        assert_eq!(s.roster_seed, RosterSeed::new(1));
    }
    for c in Category::ALL {
        assert_eq!(a.iter().filter(|s| s.category == *c).count(), 5);
    }
    // another seed gives another roster
    let other = gen_with(2, WIDE).default_roster().unwrap();
    assert_ne!(a, other);
    assert_ne!(
        a.iter().map(|s| s.personality).collect::<Vec<_>>(),
        other.iter().map(|s| s.personality).collect::<Vec<_>>()
    );
}

#[test]
fn ut_dom_060_golden_file_for_seed_1() {
    // Characterisation of the first implementation (SDD 3.11.8: "pinned by a golden test").
    // Any change after release changes every future roster (FM-DOM-006).
    let got: Vec<String> = gen_with(1, WIDE)
        .default_roster()
        .unwrap()
        .iter()
        .map(line)
        .collect();
    let want: Vec<&str> = GOLDEN
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g, w);
    }
}

#[test]
fn ut_dom_060_the_roster_does_not_depend_on_the_taken_closure_for_parameters() {
    let a = gen_with(1, WIDE).default_roster().unwrap();
    let b = gen_with(1, WIDE)
        .generate(Category::Master, 0, 5, &|n| n.starts_with('A'))
        .unwrap();
    for s in &b {
        let same = a.iter().find(|x| x.roster_index == s.roster_index).unwrap();
        assert_eq!(same.personality, s.personality);
        assert_eq!(same.uuid, s.uuid);
    }
}

// ------------------------------------------------------------------ UT-DOM-061

#[test]
fn ut_dom_061_d_34_uuid_is_v5_over_the_pinned_name_string() {
    // ROSTER_NAMESPACE is frozen; how it was made is recorded and checked here.
    assert_eq!(
        ROSTER_NAMESPACE,
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"chess-roster.v1")
    );
    assert_eq!(
        ROSTER_NAMESPACE.to_string(),
        "aa98d4a0-fa10-585f-a6a6-268551d362c9"
    );
    // golden values from an independent Python uuid5 (name "chess-roster/v1/{seed}/{index}")
    assert_eq!(
        roster_uuid(RosterSeed::new(1), 0).to_string(),
        "e150175c-fc0c-5fb5-8ee4-cab604e95634"
    );
    assert_eq!(
        roster_uuid(RosterSeed::new(1), 24).to_string(),
        "a2fed9cb-e8d4-5902-9f96-b93ca8605556"
    );
    assert_eq!(
        roster_uuid(RosterSeed::new(u32::MAX), 7).to_string(),
        "fc511d5d-9057-5312-956a-94e6f015b721"
    );
    let r = gen_with(1, WIDE).default_roster().unwrap();
    assert_eq!(r[0].uuid, roster_uuid(RosterSeed::new(1), 0));
    let ids: BTreeSet<_> = r.iter().map(|s| s.uuid).collect();
    assert_eq!(ids.len(), 25, "different indices differ");
    assert_eq!(roster_uuid(RosterSeed::new(1), 0).get_version_num(), 5);
}

// ------------------------------------------------------------------ UT-DOM-062

#[test]
fn ut_dom_062_eng_2_bounds_data_is_valid() {
    for (i, b) in CLASS_BOUNDS.iter().enumerate() {
        assert!(b.skill_level.min <= b.skill_level.max, "{i}");
        assert!(b.depth_limit.min <= b.depth_limit.max, "{i}");
        assert!(b.movetime_ms.min <= b.movetime_ms.max, "{i}");
        assert!(b.uci_elo.min <= b.uci_elo.max, "{i}");
        assert!(b.error_permille.min <= b.error_permille.max, "{i}");
        assert!(b.error_permille.max <= 1000, "{i}");
        assert!(b.error_multipv.min <= b.error_multipv.max, "{i}");
        let baseline = u32::try_from(Category::ALL[i].baseline()).unwrap();
        assert!(
            (b.uci_elo.min..=b.uci_elo.max).contains(&baseline),
            "the band holds the class baseline: {i}"
        );
    }
    let baselines: Vec<i32> = Category::ALL.iter().map(|c| c.baseline()).collect();
    assert!(baselines.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn ut_dom_062_eng_2_parameters_inside_bounds_and_no_two_sets_identical() {
    let g = gen_with(1, WIDE);
    let mut all = g.default_roster().unwrap();
    for c in Category::ALL {
        // 20 more in each class, after the first five (k = 5)
        let taken: BTreeSet<String> = all.iter().map(|s| s.name.clone()).collect();
        all.extend(g.generate(*c, 5, 20, &|n| taken.contains(n)).unwrap());
    }
    assert_eq!(all.len(), 125);
    let mut sets = std::collections::HashSet::new();
    let mut names = BTreeSet::new();
    for s in &all {
        let b = &CLASS_BOUNDS[usize::try_from(s.roster_index % 5).unwrap()];
        let p = &s.personality;
        assert!((b.skill_level.min..=b.skill_level.max).contains(&p.skill_level));
        assert!((b.depth_limit.min..=b.depth_limit.max).contains(&p.depth_limit));
        assert!((b.movetime_ms.min..=b.movetime_ms.max).contains(&p.movetime_ms));
        assert!((b.error_permille.min..=b.error_permille.max).contains(&p.error_permille));
        if p.error_permille > 0 {
            assert!((b.error_multipv.min..=b.error_multipv.max).contains(&p.error_multipv));
            assert!(p.error_multipv >= 2);
        }
        assert!(
            sets.insert(*p),
            "duplicate parameter set at {}",
            s.roster_index
        );
        assert!(names.insert(s.name.clone()), "duplicate name {}", s.name);
    }
}

#[test]
fn ut_dom_062_personality_at_depends_on_seed_and_index_only() {
    let g = gen_with(9, WIDE);
    let p40 = g.personality_at(40).unwrap();
    // the same value however the roster is asked for
    let via_generate = g.generate(Category::Beginner, 8, 1, &never).unwrap();
    assert_eq!(via_generate[0].roster_index, 40);
    assert_eq!(via_generate[0].personality, p40);
    assert_eq!(gen_with(9, WIDE).personality_at(40).unwrap(), p40);
    assert_ne!(gen_with(10, WIDE).personality_at(40).unwrap(), p40);
}

#[test]
fn ut_dom_062_bounds_invalid_is_rejected_and_a_tiny_space_is_exhausted() {
    let mut bad = CLASS_BOUNDS;
    bad[2].skill_level.min = bad[2].skill_level.max + 1;
    assert_eq!(
        RosterGen::with_bounds(RosterSeed::new(1), WIDE, bad).err(),
        Some(RosterError::BoundsInvalid)
    );
    let mut bad = CLASS_BOUNDS;
    bad[0].error_permille.max = 1001;
    assert_eq!(
        RosterGen::with_bounds(RosterSeed::new(1), WIDE, bad).err(),
        Some(RosterError::BoundsInvalid)
    );
    let mut bad = CLASS_BOUNDS;
    bad[3].error_multipv.min = 1; // a class with errors needs MultiPV >= 2
    bad[3].error_permille.min = 5;
    assert_eq!(
        RosterGen::with_bounds(RosterSeed::new(1), WIDE, bad).err(),
        Some(RosterError::BoundsInvalid)
    );
    // one possible parameter set in every class: index 5 (second Beginner) cannot be unique
    let mut one = CLASS_BOUNDS;
    for b in &mut one {
        b.skill_level.max = b.skill_level.min;
        b.depth_limit.max = b.depth_limit.min;
        b.movetime_ms.max = b.movetime_ms.min;
        b.error_permille.max = 0;
        b.error_permille.min = 0;
        b.error_multipv.max = b.error_multipv.min;
    }
    let g = RosterGen::with_bounds(
        RosterSeed::new(1),
        UciEloRange {
            min: 5000,
            max: 5001,
        },
        one,
    )
    .unwrap();
    assert!(g.personality_at(0).is_ok());
    // other classes have other single sets, so they pass; the second index of a class does not
    assert_eq!(
        g.personality_at(5),
        Err(RosterError::ParameterSpaceExhausted)
    );
    assert_eq!(
        g.generate(Category::Beginner, 0, 2, &never).err(),
        Some(RosterError::ParameterSpaceExhausted)
    );
}

// ------------------------------------------------------------------ UT-DOM-063

#[test]
fn ut_dom_063_uci_elo_only_for_classes_whose_baseline_is_in_the_range() {
    // 1000..=2300 holds the baselines 1200 (Intermediate) and 2200 (Master) only
    let range = UciEloRange {
        min: 1000,
        max: 2300,
    };
    let r = gen_with(3, range).default_roster().unwrap();
    for s in &r {
        let b = &CLASS_BOUNDS[usize::try_from(s.roster_index % 5).unwrap()];
        match s.category {
            Category::Intermediate | Category::Master => {
                let v = s.personality.uci_elo.expect("baseline inside the range");
                assert!((range.min..=range.max).contains(&v), "{v}");
                assert!((b.uci_elo.min..=b.uci_elo.max).contains(&v), "{v}");
            }
            _ => assert_eq!(s.personality.uci_elo, None, "{:?}", s.category),
        }
    }
    // a range with no baseline: every uci_elo is None
    let none = gen_with(
        3,
        UciEloRange {
            min: 700,
            max: 1100,
        },
    )
    .default_roster()
    .unwrap();
    assert!(none.iter().all(|s| s.personality.uci_elo.is_none()));
    // a range with all five baselines: all Some, inside range and band
    let all = gen_with(
        3,
        UciEloRange {
            min: 400,
            max: 2500,
        },
    )
    .default_roster()
    .unwrap();
    assert!(all.iter().all(|s| s.personality.uci_elo.is_some()));
    // the edge baselines are inclusive
    let edge = gen_with(
        3,
        UciEloRange {
            min: 1200,
            max: 1200,
        },
    )
    .default_roster()
    .unwrap();
    for s in &edge {
        let want = (s.category == Category::Intermediate).then_some(1200);
        assert_eq!(s.personality.uci_elo, want);
    }
    // an inverted range contains nothing
    let inv = gen_with(
        3,
        UciEloRange {
            min: 3000,
            max: 100,
        },
    )
    .default_roster()
    .unwrap();
    assert!(inv.iter().all(|s| s.personality.uci_elo.is_none()));
}

// ------------------------------------------------------------------ UT-DOM-064, 065

#[test]
fn ut_dom_064_starting_ratings_equal_the_class_baseline_on_both_tracks() {
    for s in gen_with(1, WIDE).default_roster().unwrap() {
        assert_eq!(s.strict_elo_start, s.category.baseline());
        assert_eq!(s.training_elo_start, s.category.baseline());
    }
}

#[test]
fn ut_dom_065_tbd_31_counts_1_5_25_succeed_0_and_26_are_rejected() {
    let g = gen_with(1, WIDE);
    for n in [1u8, 5, 25] {
        let v = g.generate(Category::Novice, 0, n, &never).unwrap();
        assert_eq!(v.len(), usize::from(n));
    }
    for n in [0u8, 26, 255] {
        assert_eq!(
            g.generate(Category::Novice, 0, n, &never).err(),
            Some(RosterError::CountOutOfRange),
            "{n}"
        );
    }
    // indices continue from first_k: 5*k + class ordinal
    let v = g.generate(Category::Master, 3, 2, &never).unwrap();
    assert_eq!(v[0].roster_index, 5 * 3 + 3);
    assert_eq!(v[1].roster_index, 5 * 4 + 3);
    assert!(v.iter().all(|s| s.category == Category::Master));
    // an index that does not fit u32
    assert_eq!(
        g.generate(Category::GrandMaster, u32::MAX / 5, 3, &never)
            .err(),
        Some(RosterError::ParameterSpaceExhausted)
    );
}

// ------------------------------------------------------------------ UT-DOM-066, 067

#[test]
fn ut_dom_066_a_taken_name_is_skipped_and_the_suffix_sequence_never_ends() {
    let g = gen_with(1, WIDE);
    let first = g
        .generate(Category::Novice, 0, 1, &never)
        .unwrap()
        .remove(0)
        .name;
    let skipped = g
        .generate(Category::Novice, 0, 1, &|n| n == first)
        .unwrap()
        .remove(0)
        .name;
    assert_ne!(first, skipped);
    // case-sensitive: a lower-case twin does not block the name
    let lower = first.to_lowercase();
    assert_ne!(lower, first);
    let kept = g
        .generate(Category::Novice, 0, 1, &|n| n == lower)
        .unwrap()
        .remove(0)
        .name;
    assert_eq!(kept, first);
    // only a name ending in " 57" is free: the candidate sequence reaches decimal suffixes
    let far = g
        .generate(Category::Novice, 0, 1, &|n| !n.ends_with(" 57"))
        .unwrap()
        .remove(0)
        .name;
    assert!(far.ends_with(" 57"), "{far}");
    // inside one batch no name repeats, even when the same candidate would win twice
    let batch = g.generate(Category::Beginner, 0, 25, &never).unwrap();
    let names: BTreeSet<_> = batch.iter().map(|s| s.name.clone()).collect();
    assert_eq!(names.len(), 25);
}

#[test]
fn ut_dom_066_suffixes_are_ordinal_then_decimal() {
    let g = gen_with(4, WIDE);
    // the first candidate with a suffix ends with " II"
    let two = g
        .generate(Category::Novice, 0, 1, &|n| !n.ends_with(" II"))
        .unwrap()
        .remove(0)
        .name;
    let stem = two.strip_suffix(" II").unwrap();
    assert!(!stem.is_empty());
    for (suffix, want) in [
        (" III", true),
        (" IV", true),
        (" X", true),
        (" 11", true),
        (" 12", true),
    ] {
        let name = g
            .generate(Category::Novice, 0, 1, &|n| !n.ends_with(suffix))
            .unwrap()
            .remove(0)
            .name;
        assert_eq!(name.ends_with(suffix), want, "{name}");
        assert_eq!(
            name.strip_suffix(suffix).unwrap(),
            stem,
            "same stem for {suffix}"
        );
    }
}

#[test]
fn ut_dom_067_pool_entries_satisfy_the_ply_6_rules() {
    assert!(GIVEN_NAMES.len() >= 20 && FAMILY_NAMES.len() >= 20);
    for pool in [GIVEN_NAMES, FAMILY_NAMES] {
        let mut seen = BTreeSet::new();
        for n in pool {
            assert!(!n.is_empty());
            assert!(n.chars().count() <= 128);
            assert!(
                n.chars().all(|c| c.is_ascii_alphabetic() || c == '\''),
                "{n}"
            );
            assert!(n.chars().next().unwrap().is_ascii_uppercase(), "{n}");
            assert!(seen.insert(*n), "duplicate {n}");
        }
    }
    // every generated name too (pair, suffix)
    for s in gen_with(1, WIDE).default_roster().unwrap() {
        assert!(!s.name.is_empty() && s.name.chars().count() <= 128);
        assert!(!s.name.chars().any(char::is_control));
        assert_eq!(s.name.trim(), s.name);
    }
}

// ------------------------------------------------------------------ UT-DOM-109

#[test]
fn ut_dom_109_first_k_after_table() {
    use Category::*;
    let k = RosterGen::first_k_after;
    assert_eq!(k(Beginner, &[]), 0);
    // indices of other classes only: 1 is Novice k=0, 7 is Intermediate k=1
    assert_eq!(k(Beginner, &[1, 7, 13, 19]), 0);
    // class Beginner holds 0, 5, 10: largest k is 2, so 3
    assert_eq!(k(Beginner, &[0, 5, 10]), 3);
    // a mix of classes: Master (ordinal 3) holds 3 (k=0) and 18 (k=3); the others do not count
    assert_eq!(k(Master, &[0, 3, 4, 9, 18, 24, 29]), 4);
    assert_eq!(k(GrandMaster, &[0, 3, 4, 9, 18, 24, 29]), 6);
    // unsorted and duplicated input gives the same result
    assert_eq!(k(Master, &[29, 18, 3, 24, 18]), k(Master, &[3, 18, 24, 29]));
    // k = 0 stored is not "none stored"
    assert_eq!(k(Novice, &[1]), 1);
    // the result continues the class: the next index stays in the class
    for c in Category::ALL {
        let stored: Vec<u32> = (0..40).collect();
        let next = k(*c, &stored);
        assert_eq!((5 * next + ordinal(*c)) % 5, ordinal(*c));
        assert!(!stored.contains(&(5 * next + ordinal(*c))));
    }
    // overflow saturates instead of wrapping
    assert_eq!(k(Beginner, &[u32::MAX]), u32::MAX / 5 + 1);
}

// ------------------------------------------------------------------ UT-DOM-111

#[test]
fn ut_dom_111_personality_validation_and_integer_equality() {
    let ok = Personality::new(3, 4, 200, Some(1500), 120, 3).unwrap();
    assert_eq!(ok.error_permille, 120);
    assert_eq!(
        Personality::new(3, 4, 200, None, 1001, 3).err(),
        Some(PersonalityError::ErrorPermilleOutOfRange)
    );
    assert!(Personality::new(3, 4, 200, None, 1000, 2).is_ok());
    for mpv in [0u8, 1] {
        assert_eq!(
            Personality::new(3, 4, 200, None, 1, mpv).err(),
            Some(PersonalityError::ErrorMultipvTooLow),
            "{mpv}"
        );
    }
    // no error rate: any MultiPV value is accepted
    assert!(Personality::new(3, 4, 200, None, 0, 0).is_ok());
    assert!(Personality::new(3, 4, 200, None, 0, 1).is_ok());
    // equality is integer equality
    assert_eq!(
        Personality::new(1, 2, 3, None, 5, 2).unwrap(),
        Personality::new(1, 2, 3, None, 5, 2).unwrap()
    );
    assert_ne!(
        Personality::new(1, 2, 3, None, 5, 2).unwrap(),
        Personality::new(1, 2, 3, None, 6, 2).unwrap()
    );
    // every permille 0..=1000 survives the f64 conversion that chess-app performs
    for p in 0..=1000u16 {
        let rate = f64::from(p) / 1000.0;
        assert_eq!((rate * 1000.0).round(), f64::from(p));
        assert!(Personality::new(0, 1, 1, None, p, 2).is_ok());
    }
}

#[test]
fn ut_dom_111_personality_deserialisation_validates() {
    use serde::Deserialize;
    use serde::de::value::{Error, MapDeserializer};
    let parse = |rows: Vec<(&'static str, u32)>| -> Result<Personality, Error> {
        Personality::deserialize(MapDeserializer::<_, Error>::new(rows.into_iter()))
    };
    let base = vec![
        ("skill_level", 3),
        ("depth_limit", 4),
        ("movetime_ms", 200),
        ("error_permille", 120),
        ("error_multipv", 3),
    ];
    let p = parse(base.clone()).unwrap();
    assert_eq!(p, Personality::new(3, 4, 200, None, 120, 3).unwrap());
    let mut bad = base.clone();
    bad[3].1 = 1001;
    assert!(parse(bad).is_err());
    let mut unknown = base.clone();
    unknown.push(("surprise", 1));
    assert!(parse(unknown).is_err(), "unknown fields are rejected");
    let mut too_big = base;
    too_big[0].1 = 300; // does not fit u8
    assert!(parse(too_big).is_err());
}

#[test]
fn ut_dom_100_roster_errors_render_plain_messages() {
    for m in [
        RosterError::CountOutOfRange.to_string(),
        RosterError::ParameterSpaceExhausted.to_string(),
        RosterError::BoundsInvalid.to_string(),
        PersonalityError::ErrorPermilleOutOfRange.to_string(),
        PersonalityError::ErrorMultipvTooLow.to_string(),
        chess_domain::EcoUnanswered.to_string(),
    ] {
        assert!(!m.is_empty() && !m.contains('/'), "{m}");
    }
}

#[test]
fn ut_dom_062_with_bounds_accepts_the_default_table_and_the_edge_cases() {
    assert!(RosterGen::with_bounds(RosterSeed::new(1), WIDE, CLASS_BOUNDS).is_ok());
    // no error rate: the MultiPV bounds are not used, so 0 is valid
    let mut zero = CLASS_BOUNDS;
    zero[4].error_multipv.min = 0;
    zero[4].error_multipv.max = 0;
    assert!(RosterGen::with_bounds(RosterSeed::new(1), WIDE, zero).is_ok());
    // an error rate with MultiPV exactly 2 is valid; 1000 per mille is valid
    let mut edge = CLASS_BOUNDS;
    edge[0].error_permille.max = 1000;
    edge[0].error_multipv.min = 2;
    assert!(RosterGen::with_bounds(RosterSeed::new(1), WIDE, edge).is_ok());
}

#[test]
fn ut_dom_062_the_index_bound_is_inclusive_and_names_are_bounded() {
    let g = gen_with(1, WIDE);
    assert!(g.personality_at(chess_domain::MAX_ROSTER_INDEX).is_ok());
    assert_eq!(
        g.personality_at(chess_domain::MAX_ROSTER_INDEX + 1),
        Err(RosterError::ParameterSpaceExhausted)
    );
    // a `taken` that is always true ends in an error instead of looping for ever
    assert_eq!(
        g.generate(Category::Novice, 0, 1, &|_| true).err(),
        Some(RosterError::ParameterSpaceExhausted)
    );
}
