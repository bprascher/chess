//! DD-DOM-013, 014, 020 (SDD 3.11.8): the seeded computer-player roster, `first_k_after`, the
//! name generator and `Personality`. Pure: the stream is `SplitMix64` seeded from
//! `(roster_seed, roster_index, attempt)`, so the same inputs give the same Players on every
//! machine and engine build (D-34, D-46). The `UCI_Elo` range is a plain value passed in.
//!
//! Draw order for one parameter set (fixed): skill level, depth limit, movetime, `uci_elo`,
//! error rate, error MultiPV. Bounded draws use `Rng::below` (multiply-shift on 128 bits).
//! Class bounds below are `ESTIMATE` placeholders until calibration (ENG-2, R-2).

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::edge::{Rng, SplitMix64};
use crate::error::{PersonalityError, RosterError};
use crate::types::Category;

/// `ESTIMATE`: a sanity bound on `roster_index` so that a corrupt stored index cannot make the
/// uniqueness walk (which recomputes every lower index) run for billions of steps.
pub const MAX_ROSTER_INDEX: u32 = 5_000;

/// `ESTIMATE`: the most name candidates tried for one position (pairs, roman and decimal
/// suffixes). The SDD calls the sequence endless; a stored set of players is nowhere near this
/// size, and the bound keeps a `taken` that is always true from looping for minutes.
const MAX_NAME_SUFFIX: u32 = 100_000;

/// The attempts allowed for one index before `ParameterSpaceExhausted` (SDD 3.11.8).
const MAX_ATTEMPTS: u32 = 64;

/// The stream that draws names, kept apart from the attempts `0..MAX_ATTEMPTS`.
const NAME_STREAM: u32 = u32::MAX;

/// The roster seed. A `u32` so that it stores in any integer column and crosses to TypeScript
/// as a number without a 2^53 concern (D-50); the SDD fixes no width (finding F-B6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RosterSeed(u32);

impl RosterSeed {
    pub fn new(seed: u32) -> RosterSeed {
        RosterSeed(seed)
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for RosterSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The pinned `UCI_Elo` range of the bundled engine, passed as a value by `chess-app`. This
/// crate never names the engine constant and the value is not written in any document (D-46).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UciEloRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<T> {
    pub min: T,
    pub max: T,
}

const fn span<T>(min: T, max: T) -> Span<T> {
    Span { min, max }
}

/// Parameter bounds of one class. Data, not code (SDD 3.11.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassBounds {
    pub skill_level: Span<u8>,
    pub depth_limit: Span<u32>,
    pub movetime_ms: Span<u32>,
    /// The band `uci_elo` is drawn from (intersected with the engine range). It holds the
    /// class baseline.
    pub uci_elo: Span<u32>,
    pub error_permille: Span<u16>,
    pub error_multipv: Span<u8>,
}

/// `ESTIMATE` placeholders, indexed by class ordinal (Beginner 0 .. Grand Master 4). Not
/// calibrated; the numbers come from `chess calibrate` before release.
pub const CLASS_BOUNDS: [ClassBounds; 5] = [
    ClassBounds {
        skill_level: span(0, 2),
        depth_limit: span(1, 2),
        movetime_ms: span(50, 200),
        uci_elo: span(400, 800),
        error_permille: span(150, 350),
        error_multipv: span(3, 5),
    },
    ClassBounds {
        skill_level: span(3, 6),
        depth_limit: span(2, 4),
        movetime_ms: span(100, 400),
        uci_elo: span(600, 1000),
        error_permille: span(80, 200),
        error_multipv: span(2, 4),
    },
    ClassBounds {
        skill_level: span(7, 12),
        depth_limit: span(5, 8),
        movetime_ms: span(200, 800),
        uci_elo: span(1200, 1800),
        error_permille: span(20, 100),
        error_multipv: span(2, 3),
    },
    ClassBounds {
        skill_level: span(13, 17),
        depth_limit: span(9, 14),
        movetime_ms: span(500, 1500),
        uci_elo: span(2200, 2400),
        error_permille: span(0, 30),
        error_multipv: span(2, 3),
    },
    ClassBounds {
        skill_level: span(18, 20),
        depth_limit: span(14, 20),
        movetime_ms: span(1000, 3000),
        uci_elo: span(2500, 2800),
        error_permille: span(0, 0),
        error_multipv: span(2, 2),
    },
];

// ---------------------------------------------------------------- Personality

/// The stored and identity type of a computer Player's engine parameters (D-43). The error
/// rate is an integer per mille so that the ENG-2 uniqueness check and the store's UNIQUE index
/// compare integers. The engine's own `error_rate: f64 = permille / 1000.0` is converted in
/// `chess-app`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "RawPersonality")]
pub struct Personality {
    pub skill_level: u8,
    pub depth_limit: u32,
    pub movetime_ms: u32,
    pub uci_elo: Option<u32>,
    pub error_permille: u16,
    pub error_multipv: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPersonality {
    skill_level: u8,
    depth_limit: u32,
    movetime_ms: u32,
    #[serde(default)]
    uci_elo: Option<u32>,
    error_permille: u16,
    error_multipv: u8,
}

impl TryFrom<RawPersonality> for Personality {
    type Error = PersonalityError;

    fn try_from(raw: RawPersonality) -> Result<Personality, PersonalityError> {
        Personality::new(
            raw.skill_level,
            raw.depth_limit,
            raw.movetime_ms,
            raw.uci_elo,
            raw.error_permille,
            raw.error_multipv,
        )
    }
}

impl Personality {
    /// Rejects `error_permille` above 1000 and, when the error rate is above 0, an
    /// `error_multipv` of 0 or 1 (a non-best line needs at least two lines).
    pub fn new(
        skill_level: u8,
        depth_limit: u32,
        movetime_ms: u32,
        uci_elo: Option<u32>,
        error_permille: u16,
        error_multipv: u8,
    ) -> Result<Personality, PersonalityError> {
        if error_permille > 1000 {
            return Err(PersonalityError::ErrorPermilleOutOfRange);
        }
        if error_permille > 0 && error_multipv < 2 {
            return Err(PersonalityError::ErrorMultipvTooLow);
        }
        Ok(Personality {
            skill_level,
            depth_limit,
            movetime_ms,
            uci_elo,
            error_permille,
            error_multipv,
        })
    }
}

// ---------------------------------------------------------------- identity

/// The fixed namespace of roster identities: `UUIDv5(DNS namespace, "chess-roster.v1")`
/// (checked by `ut_dom_061`). Frozen once released: changing it breaks cross-install matching
/// (D-34). Chosen by the engineer because the SDD says only that it must be generated once
/// (finding F-B5).
pub const ROSTER_NAMESPACE: Uuid = Uuid::from_u128(0xaa98_d4a0_fa10_585f_a6a6_2685_51d3_62c9);

/// `UUIDv5(ROSTER_NAMESPACE, "chess-roster/v1/{seed}/{index}")` with decimal ASCII numbers.
pub fn roster_uuid(seed: RosterSeed, index: u32) -> Uuid {
    let name = format!("chess-roster/v1/{}/{}", seed.get(), index);
    Uuid::new_v5(&ROSTER_NAMESPACE, name.as_bytes())
}

// ---------------------------------------------------------------- names

/// Original project text, ASCII, no licence question (NFR-6).
pub const GIVEN_NAMES: &[&str] = &[
    "Adrian",
    "Beatrix",
    "Casimir",
    "Delphine",
    "Evander",
    "Fenella",
    "Gideon",
    "Helena",
    "Ignatius",
    "Josephine",
    "Konrad",
    "Leonora",
    "Matthias",
    "Nadia",
    "Octavia",
    "Percival",
    "Quentin",
    "Rosalind",
    "Sebastian",
    "Theodora",
    "Ulrich",
    "Valentina",
    "Walter",
    "Xenia",
    "Yannick",
    "Zelda",
    "Arlo",
    "Bruno",
    "Clara",
    "Dmitri",
];

pub const FAMILY_NAMES: &[&str] = &[
    "Ashworth",
    "Blackwood",
    "Castellan",
    "Dunmore",
    "Eastley",
    "Fairbanks",
    "Garrick",
    "Hawthorne",
    "Ironside",
    "Jessop",
    "Kingsley",
    "Lockhart",
    "Marlowe",
    "Northcott",
    "Oakhurst",
    "Pemberton",
    "Quillfeather",
    "Redfern",
    "Stanhope",
    "Thornbury",
    "Underhill",
    "Vance",
    "Whitlock",
    "Yarrow",
    "Zimmerman",
    "Abernathy",
    "Brightwater",
    "Calloway",
    "Dalrymple",
    "Everhart",
];

const SUFFIXES: [&str; 9] = ["II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

fn pick(rng: &mut SplitMix64, pool: &[&'static str]) -> &'static str {
    let n = u64::try_from(pool.len()).unwrap_or(1);
    let i = usize::try_from(rng.below(n)).unwrap_or(0);
    pool.get(i).copied().unwrap_or("Player")
}

/// The first candidate for which `free` holds: eight pseudo-random "given family" pairs
/// (duplicates removed), then the first pair with `II`, `III`, `IV`, ... `X`, then decimal
/// `11`, `12`, ... The sequence is effectively endless, so a request cannot fail on a name.
fn choose_name(
    seed: RosterSeed,
    index: u32,
    free: &dyn Fn(&str) -> bool,
) -> Result<String, RosterError> {
    let mut rng = stream(seed, index, NAME_STREAM);
    let mut pairs: Vec<String> = Vec::new();
    for _ in 0..8 {
        let name = format!(
            "{} {}",
            pick(&mut rng, GIVEN_NAMES),
            pick(&mut rng, FAMILY_NAMES)
        );
        if !pairs.contains(&name) {
            pairs.push(name);
        }
    }
    if let Some(found) = pairs.iter().find(|n| free(n)) {
        return Ok(found.clone());
    }
    let stem = pairs.first().cloned().unwrap_or_default();
    for n in 2..=MAX_NAME_SUFFIX {
        let suffix = match usize::try_from(n - 2).ok().and_then(|i| SUFFIXES.get(i)) {
            Some(roman) => (*roman).to_string(),
            None => n.to_string(),
        };
        let candidate = format!("{stem} {suffix}");
        if free(&candidate) {
            return Ok(candidate);
        }
    }
    Err(RosterError::ParameterSpaceExhausted)
}

// ---------------------------------------------------------------- generator

/// A fixed mixing of `(roster_seed, roster_index, attempt)`.
fn stream(seed: RosterSeed, index: u32, attempt: u32) -> SplitMix64 {
    let a = SplitMix64::new(u64::from(seed.get())).next_u64();
    let b = SplitMix64::new(a ^ u64::from(index)).next_u64();
    SplitMix64::new(b ^ (u64::from(attempt) << 32))
}

/// A value in `min..=max` (`min <= max` is checked by `validate`).
fn draw(rng: &mut SplitMix64, min: u64, max: u64) -> u64 {
    let width = max.saturating_sub(min).saturating_add(1);
    min.saturating_add(rng.below(width))
}

fn ordinal(category: Category) -> u32 {
    match category {
        Category::Beginner => 0,
        Category::Novice => 1,
        Category::Intermediate => 2,
        Category::Master => 3,
        Category::GrandMaster => 4,
    }
}

fn to_u8(v: u64) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

fn to_u16(v: u64) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

fn to_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

fn validate(bounds: &[ClassBounds; 5]) -> Result<(), RosterError> {
    for b in bounds {
        let ordered = b.skill_level.min <= b.skill_level.max
            && b.depth_limit.min <= b.depth_limit.max
            && b.movetime_ms.min <= b.movetime_ms.max
            && b.uci_elo.min <= b.uci_elo.max
            && b.error_permille.min <= b.error_permille.max
            && b.error_multipv.min <= b.error_multipv.max;
        let errors_ok =
            b.error_permille.max <= 1000 && (b.error_permille.max == 0 || b.error_multipv.min >= 2);
        if !(ordered && errors_ok) {
            return Err(RosterError::BoundsInvalid);
        }
    }
    Ok(())
}

/// One generated Player, not yet stored (`COMPUTER_PROFILE` plus its two rating rows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerPlayerSpec {
    pub uuid: Uuid,
    pub roster_seed: RosterSeed,
    pub roster_index: u32,
    pub category: Category,
    pub name: String,
    pub personality: Personality,
    pub strict_elo_start: i32,
    pub training_elo_start: i32,
}

#[derive(Debug, Clone)]
pub struct RosterGen {
    seed: RosterSeed,
    range: UciEloRange,
    bounds: [ClassBounds; 5],
}

impl RosterGen {
    pub fn new(seed: RosterSeed, range: UciEloRange) -> RosterGen {
        RosterGen {
            seed,
            range,
            bounds: CLASS_BOUNDS,
        }
    }

    /// For tests and calibration: other bounds, validated.
    pub fn with_bounds(
        seed: RosterSeed,
        range: UciEloRange,
        bounds: [ClassBounds; 5],
    ) -> Result<RosterGen, RosterError> {
        validate(&bounds)?;
        Ok(RosterGen {
            seed,
            range,
            bounds,
        })
    }

    /// The first free position within `category`: 0 when no stored index belongs to it,
    /// otherwise one more than the largest `k` among the stored indices of the class. An index
    /// `5*k + c` belongs to class ordinal `c`. Order and duplicates do not matter (D-46).
    pub fn first_k_after(category: Category, stored: &[u32]) -> u32 {
        let class = ordinal(category);
        stored
            .iter()
            .filter(|index| *index % 5 == class)
            .map(|index| *index / 5)
            .max()
            .map_or(0, |k| k.saturating_add(1))
    }

    /// The parameter set at `index`: attempts 0, 1, 2 ... until it differs from the sets of all
    /// lower indices (recomputed, not read from the store). Depends on `(seed, index, range)`.
    pub fn personality_at(&self, index: u32) -> Result<Personality, RosterError> {
        let all = self.personalities_up_to(index)?;
        all.last()
            .copied()
            .ok_or(RosterError::ParameterSpaceExhausted)
    }

    fn personalities_up_to(&self, index: u32) -> Result<Vec<Personality>, RosterError> {
        if index > MAX_ROSTER_INDEX {
            return Err(RosterError::ParameterSpaceExhausted);
        }
        let mut sets: Vec<Personality> = Vec::new();
        let mut seen: HashSet<Personality> = HashSet::new();
        for i in 0..=index {
            let class = usize::try_from(i % 5).unwrap_or(0);
            let bounds = self.bounds.get(class).ok_or(RosterError::BoundsInvalid)?;
            let baseline = Category::ALL
                .get(class)
                .map_or(0, |c| u32::try_from(c.baseline()).unwrap_or(0));
            let mut chosen = None;
            for attempt in 0..MAX_ATTEMPTS {
                let candidate = self.draw_personality(bounds, baseline, i, attempt)?;
                if seen.insert(candidate) {
                    chosen = Some(candidate);
                    break;
                }
            }
            sets.push(chosen.ok_or(RosterError::ParameterSpaceExhausted)?);
        }
        Ok(sets)
    }

    fn draw_personality(
        &self,
        b: &ClassBounds,
        baseline: u32,
        index: u32,
        attempt: u32,
    ) -> Result<Personality, RosterError> {
        let mut rng = stream(self.seed, index, attempt);
        let skill = to_u8(draw(
            &mut rng,
            b.skill_level.min.into(),
            b.skill_level.max.into(),
        ));
        let depth = to_u32(draw(
            &mut rng,
            b.depth_limit.min.into(),
            b.depth_limit.max.into(),
        ));
        let movetime = to_u32(draw(
            &mut rng,
            b.movetime_ms.min.into(),
            b.movetime_ms.max.into(),
        ));
        // The band intersected with the engine range; used only when the baseline is inside
        // the range. The draw is made either way so the stream does not shift.
        let low = b.uci_elo.min.max(self.range.min);
        let high = b.uci_elo.max.min(self.range.max);
        let usable = low <= high;
        let (lo, hi) = if usable { (low, high) } else { (0, 0) };
        let elo = to_u32(draw(&mut rng, lo.into(), hi.into()));
        let in_range = (self.range.min..=self.range.max).contains(&baseline);
        let uci_elo = (usable && in_range).then_some(elo);
        let permille = to_u16(draw(
            &mut rng,
            b.error_permille.min.into(),
            b.error_permille.max.into(),
        ));
        let multipv = to_u8(draw(
            &mut rng,
            b.error_multipv.min.into(),
            b.error_multipv.max.into(),
        ));
        // No error rate: the MultiPV value is not used, so it is normalised and cannot make two
        // equal parameter sets look different.
        let multipv = if permille == 0 { 1 } else { multipv };
        Personality::new(skill, depth, movetime, uci_elo, permille, multipv)
            .map_err(|_| RosterError::BoundsInvalid)
    }

    /// The default roster: five Players per class, indices 0 to 24, nothing taken.
    pub fn default_roster(&self) -> Result<Vec<ComputerPlayerSpec>, RosterError> {
        let indices: Vec<u32> = (0..25).collect();
        self.build(&indices, &|_| false)
    }

    /// `count` (1 to 25) Players of `category` starting at position `first_k` of the class
    /// (`first_k_after`). `taken` is case-sensitive and covers every Player, active or
    /// archived; names inside the batch are also kept apart.
    pub fn generate(
        &self,
        category: Category,
        first_k: u32,
        count: u8,
        taken: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<ComputerPlayerSpec>, RosterError> {
        if !(1..=25).contains(&count) {
            return Err(RosterError::CountOutOfRange);
        }
        let class = ordinal(category);
        let mut indices = Vec::with_capacity(usize::from(count));
        for i in 0..u32::from(count) {
            let index = first_k
                .checked_add(i)
                .and_then(|k| k.checked_mul(5))
                .and_then(|v| v.checked_add(class))
                .ok_or(RosterError::ParameterSpaceExhausted)?;
            indices.push(index);
        }
        self.build(&indices, taken)
    }

    fn build(
        &self,
        indices: &[u32],
        taken: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<ComputerPlayerSpec>, RosterError> {
        let top = indices.iter().copied().max().unwrap_or(0);
        let sets = self.personalities_up_to(top)?;
        let mut used: HashSet<String> = HashSet::new();
        let mut out = Vec::with_capacity(indices.len());
        for &index in indices {
            let class = usize::try_from(index % 5).unwrap_or(0);
            let category = *Category::ALL.get(class).ok_or(RosterError::BoundsInvalid)?;
            let personality = *sets
                .get(usize::try_from(index).unwrap_or(usize::MAX))
                .ok_or(RosterError::ParameterSpaceExhausted)?;
            let name = choose_name(self.seed, index, &|n| !taken(n) && !used.contains(n))?;
            used.insert(name.clone());
            out.push(ComputerPlayerSpec {
                uuid: roster_uuid(self.seed, index),
                roster_seed: self.seed,
                roster_index: index,
                category,
                name,
                personality,
                strict_elo_start: category.baseline(),
                training_elo_start: category.baseline(),
            });
        }
        Ok(out)
    }
}
