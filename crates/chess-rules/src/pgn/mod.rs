//! DD-RUL-009 to DD-RUL-012 and DD-RUL-016: the PGN model, writer, reader, line text and the
//! canonical form. The reader is an in-crate tokenizer (`reader`): `pgn-reader` 0.29.1 gives no
//! line and column and parses SAN with the upstream rules, so it cannot supply the error
//! taxonomy or the lenient forms (OQ-RUL-001, DD-RUL-011 escape route). No upstream type
//! appears here.

mod reader;

use std::collections::BTreeMap;
use std::fmt;

use crate::error::PgnWriteError;
use crate::position::Position;
use crate::types::{Move, Side};

pub use reader::{PgnReader, parse_line};

/// Default and writer limit of variation nesting (SDD 3.12, ESTIMATE).
pub const DEFAULT_MAX_VARIATION_DEPTH: usize = 64;

/// Hard ceiling of the reader's variation depth (RF-01). The reader is iterative, but its
/// output (`PgnNode.variations`) is a recursive tree whose `Drop`, `Clone`, `Debug` and
/// `PartialEq` recurse once per nesting level; an unbounded caller value would turn hostile
/// input into a stack overflow (a process abort). `PgnLimits::max_variation_depth` is clamped
/// to this value. 256 levels is 32 times the 8-level nesting of any real annotated game and
/// was exercised on a 256 KiB thread stack (tests/rework.rs).
pub(crate) const MAX_VARIATION_DEPTH_CEILING: usize = 256;

/// Limits of the reader. The caller enforces the file size (TBD-25).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgnLimits {
    /// Deepest variation nesting accepted; deeper input fails that game with `VariationTooDeep`.
    /// Values above 256 are treated as 256 (the error reports the effective limit).
    pub max_variation_depth: usize,
}

impl Default for PgnLimits {
    fn default() -> Self {
        PgnLimits {
            max_variation_depth: DEFAULT_MAX_VARIATION_DEPTH,
        }
    }
}

/// The game result: the movetext terminator and the `Result` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PgnResult {
    WhiteWins,
    BlackWins,
    Draw,
    Unknown,
}

impl PgnResult {
    pub fn as_str(self) -> &'static str {
        match self {
            PgnResult::WhiteWins => "1-0",
            PgnResult::BlackWins => "0-1",
            PgnResult::Draw => "1/2-1/2",
            PgnResult::Unknown => "*",
        }
    }

    pub(crate) fn from_token(token: &str) -> Option<PgnResult> {
        match token {
            "1-0" => Some(PgnResult::WhiteWins),
            "0-1" => Some(PgnResult::BlackWins),
            "1/2-1/2" => Some(PgnResult::Draw),
            "*" => Some(PgnResult::Unknown),
            _ => None,
        }
    }
}

/// `YYYY.MM.DD` with unknown parts. A part outside its range is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PgnDate {
    year: Option<u16>,
    month: Option<u8>,
    day: Option<u8>,
}

impl PgnDate {
    /// A year above 9999, a month above 12 or below 1, a day above 31 or below 1 is unknown.
    pub fn new(year: Option<u16>, month: Option<u8>, day: Option<u8>) -> PgnDate {
        PgnDate {
            year: year.filter(|y| *y <= 9999),
            month: month.filter(|m| (1..=12).contains(m)),
            day: day.filter(|d| (1..=31).contains(d)),
        }
    }

    pub fn year(&self) -> Option<u16> {
        self.year
    }

    pub fn month(&self) -> Option<u8> {
        self.month
    }

    pub fn day(&self) -> Option<u8> {
        self.day
    }

    /// `YYYY.MM.DD` with `????` or `??` for an unknown part; `None` for any other shape.
    pub(crate) fn parse(text: &str) -> Option<PgnDate> {
        let mut parts = text.split('.');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let number = |s: &str, width: usize| -> Option<Option<u16>> {
            if s.len() != width {
                return None;
            }
            if s.bytes().all(|b| b == b'?') {
                return Some(None);
            }
            if s.bytes().all(|b| b.is_ascii_digit()) {
                return s.parse().ok().map(Some);
            }
            None
        };
        let year = number(y, 4)?;
        let month = number(m, 2)?.and_then(|v| u8::try_from(v).ok());
        let day = number(d, 2)?.and_then(|v| u8::try_from(v).ok());
        Some(PgnDate::new(year, month, day))
    }
}

impl fmt::Display for PgnDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.year {
            Some(y) => write!(f, "{y:04}.")?,
            None => f.write_str("????.")?,
        }
        match self.month {
            Some(m) => write!(f, "{m:02}.")?,
            None => f.write_str("??.")?,
        }
        match self.day {
            Some(d) => write!(f, "{d:02}"),
            None => f.write_str("??"),
        }
    }
}

/// `Termination` tag values this application writes (SDD 3.11.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Termination {
    Normal,
    TimeForfeit,
    Abandoned,
}

impl Termination {
    fn as_str(self) -> &'static str {
        match self {
            Termination::Normal => "normal",
            Termination::TimeForfeit => "time forfeit",
            Termination::Abandoned => "abandoned",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Termination> {
        match text.to_ascii_lowercase().as_str() {
            "normal" => Some(Termination::Normal),
            "time forfeit" => Some(Termination::TimeForfeit),
            "abandoned" => Some(Termination::Abandoned),
            _ => None,
        }
    }
}

/// `TimeControl` as `seconds` or `seconds+increment` (SDD 3.11.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeControl {
    pub seconds: u32,
    pub increment: Option<u32>,
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.increment {
            Some(i) => write!(f, "{}+{i}", self.seconds),
            None => write!(f, "{}", self.seconds),
        }
    }
}

impl TimeControl {
    pub(crate) fn parse(text: &str) -> Option<TimeControl> {
        // `parse` rejects an empty string; the digit test rejects a sign.
        let number = |s: &str| -> Option<u32> {
            if s.bytes().all(|b| b.is_ascii_digit()) {
                s.parse().ok()
            } else {
                None
            }
        };
        match text.split_once('+') {
            None => Some(TimeControl {
                seconds: number(text)?,
                increment: None,
            }),
            Some((s, i)) => Some(TimeControl {
                seconds: number(s)?,
                increment: Some(number(i)?),
            }),
        }
    }
}

/// The tags of a game: the Seven Tag Roster (minus `Result`, which is `PgnGame::result`), the
/// typed standard tags, and free-form extras. `SetUp` and `FEN` are not tags here: the start
/// position is `PgnGame::start`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnTags {
    pub event: String,
    pub site: String,
    pub date: PgnDate,
    pub round: String,
    pub white: String,
    pub black: String,
    pub time_control: Option<TimeControl>,
    pub termination: Option<Termination>,
    /// Free-form tags, written in byte order of the name after the roster. Names must not be
    /// the roster names, `FEN`, `SetUp`, `TimeControl` or `Termination` (the writer rejects
    /// them); the writer never emits `ECO` or `Opening` (Q-SE-7).
    pub extras: BTreeMap<String, String>,
}

impl Default for PgnTags {
    /// The writer's defaults: `?` for unknown text, `-` for the round, unknown date.
    fn default() -> Self {
        PgnTags {
            event: "?".into(),
            site: "?".into(),
            date: PgnDate::default(),
            round: "-".into(),
            white: "?".into(),
            black: "?".into(),
            time_control: None,
            termination: None,
            extras: BTreeMap::new(),
        }
    }
}

impl PgnTags {
    /// The raw text of the `ECO` tag exactly as read: no trimming, no validation, no case
    /// change. `None` when absent. What it means is for `chess-domain` (CR-9.12, D-59).
    pub fn eco_tag(&self) -> Option<&str> {
        self.extras.get("ECO").map(String::as_str)
    }
}

/// One move of a line. `variations` holds alternatives to **this** node's move, each starting
/// from the position before it (standard PGN meaning, DD-RUL-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnNode {
    pub mv: Move,
    pub nags: Vec<u8>,
    pub comment: Option<String>,
    pub variations: Vec<Vec<PgnNode>>,
}

impl PgnNode {
    pub fn new(mv: Move) -> PgnNode {
        PgnNode {
            mv,
            nags: Vec::new(),
            comment: None,
            variations: Vec::new(),
        }
    }
}

/// One game: tags, the start position, the mainline and the result.
#[derive(Debug, Clone)]
pub struct PgnGame {
    pub tags: PgnTags,
    pub start: Position,
    pub moves: Vec<PgnNode>,
    pub result: PgnResult,
}

impl PgnGame {
    /// A game with default tags.
    pub fn new(start: Position, moves: Vec<PgnNode>, result: PgnResult) -> PgnGame {
        PgnGame {
            tags: PgnTags::default(),
            start,
            moves,
            result,
        }
    }
}

// ---------------------------------------------------------------- writer (DD-RUL-010)

const ROSTER_AND_TYPED: [&str; 11] = [
    "Event",
    "Site",
    "Date",
    "Round",
    "White",
    "Black",
    "Result",
    "FEN",
    "SetUp",
    "TimeControl",
    "Termination",
];

fn valid_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `[Name "value"]` with `\"` and `\\` escaped and any control character a space (ASM-RUL-004).
fn tag_line(name: &str, value: &str) -> String {
    let mut out = String::with_capacity(name.len() + value.len() + 6);
    out.push('[');
    out.push_str(name);
    out.push_str(" \"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out.push_str("\"]\n");
    out
}

fn is_custom_start(game: &PgnGame) -> bool {
    game.start.to_fen() != Position::standard().to_fen()
}

fn roster(game: &PgnGame) -> Vec<(&'static str, String)> {
    let t = &game.tags;
    vec![
        ("Event", t.event.clone()),
        ("Site", t.site.clone()),
        ("Date", t.date.to_string()),
        ("Round", t.round.clone()),
        ("White", t.white.clone()),
        ("Black", t.black.clone()),
        ("Result", game.result.as_str().to_string()),
    ]
}

/// Comment text for the movetext: control characters and `}` become spaces, then trimmed.
fn comment_text(comment: &str) -> Option<String> {
    let cleaned: String = comment
        .chars()
        .map(|c| if c.is_control() || c == '}' { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn sequence_tokens(
    start: &Position,
    nodes: &[PgnNode],
    depth: usize,
    path: &mut Vec<(usize, usize)>,
    tokens: &mut Vec<String>,
) -> Result<(), PgnWriteError> {
    let mut cur = start.clone();
    let mut need_number = true;
    for (i, node) in nodes.iter().enumerate() {
        let illegal = |path: &[(usize, usize)]| PgnWriteError::IllegalMove {
            variation_path: path.to_vec(),
            ply: i,
        };
        let san = cur.san(node.mv).map_err(|_| illegal(path))?;
        match cur.turn() {
            Side::White => tokens.push(format!("{}.", cur.fullmove_number())),
            Side::Black if need_number => tokens.push(format!("{}...", cur.fullmove_number())),
            Side::Black => {}
        }
        tokens.push(san.to_string());
        need_number = false;
        for nag in &node.nags {
            tokens.push(format!("${nag}"));
        }
        if let Some(text) = node.comment.as_deref().and_then(comment_text) {
            tokens.push(format!("{{{text}}}"));
            need_number = true;
        }
        for (j, variation) in node.variations.iter().enumerate() {
            if variation.is_empty() {
                continue;
            }
            if depth + 1 > DEFAULT_MAX_VARIATION_DEPTH {
                return Err(PgnWriteError::VariationTooDeep {
                    limit: DEFAULT_MAX_VARIATION_DEPTH,
                });
            }
            path.push((i, j));
            let mut inner = Vec::new();
            sequence_tokens(&cur, variation, depth + 1, path, &mut inner)?;
            path.pop();
            if let Some(first) = inner.first_mut() {
                first.insert(0, '(');
            }
            if let Some(last) = inner.last_mut() {
                last.push(')');
            }
            tokens.append(&mut inner);
            need_number = true;
        }
        cur = cur.play(node.mv).map_err(|_| illegal(path))?;
    }
    Ok(())
}

/// Joins tokens with one space; a line break replaces the space when the next token would make
/// the line 80 characters or longer, so every line is at most 79 characters (SDD 3.11.7).
fn wrap(tokens: &[String]) -> String {
    let mut out = String::new();
    let mut line_len = 0usize;
    for token in tokens {
        let n = token.chars().count();
        if line_len == 0 {
            out.push_str(token);
            line_len = n;
        } else if line_len + 1 + n >= 80 {
            out.push('\n');
            out.push_str(token);
            line_len = n;
        } else {
            out.push(' ');
            out.push_str(token);
            line_len += 1 + n;
        }
    }
    out
}

/// One game as PGN text (IF-1): UTF-8, deterministic, ends with a single `\n`. Never partial.
pub fn write_pgn(game: &PgnGame) -> Result<String, PgnWriteError> {
    for name in game.tags.extras.keys() {
        if !valid_tag_name(name) {
            return Err(PgnWriteError::BadTagName { name: name.clone() });
        }
        if ROSTER_AND_TYPED.contains(&name.as_str()) {
            return Err(PgnWriteError::ReservedTag { name: name.clone() });
        }
    }
    let mut tokens = Vec::new();
    sequence_tokens(&game.start, &game.moves, 0, &mut Vec::new(), &mut tokens)?;
    tokens.push(game.result.as_str().to_string());

    let mut rest: Vec<(String, String)> = Vec::new();
    if is_custom_start(game) {
        rest.push(("FEN".into(), game.start.to_fen().to_string()));
        rest.push(("SetUp".into(), "1".into()));
    }
    if let Some(tc) = game.tags.time_control {
        rest.push(("TimeControl".into(), tc.to_string()));
    }
    if let Some(t) = game.tags.termination {
        rest.push(("Termination".into(), t.as_str().into()));
    }
    for (k, v) in &game.tags.extras {
        if k != "ECO" && k != "Opening" {
            rest.push((k.clone(), v.clone()));
        }
    }
    rest.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut out = String::new();
    for (name, value) in roster(game) {
        out.push_str(&tag_line(name, &value));
    }
    for (name, value) in &rest {
        out.push_str(&tag_line(name, value));
    }
    out.push('\n');
    out.push_str(&wrap(&tokens));
    out.push('\n');
    Ok(out)
}

/// Games separated by one blank line. The first failure is returned with its 1-based index.
pub fn write_pgn_many(games: &[PgnGame]) -> Result<String, PgnWriteError> {
    let mut out = String::new();
    for (i, game) in games.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let text = write_pgn(game).map_err(|e| PgnWriteError::InGame {
            game_index: i + 1,
            source: Box::new(e),
        })?;
        out.push_str(&text);
    }
    Ok(out)
}

/// Movetext of a line from `start` (DD-RUL-012), with the writer's numbering, NAG, comment and
/// variation rules and wrapping. No result token. Illegal nodes are an error.
pub fn format_line(start: &Position, nodes: &[PgnNode]) -> Result<String, PgnWriteError> {
    let mut tokens = Vec::new();
    sequence_tokens(start, nodes, 0, &mut Vec::new(), &mut tokens)?;
    Ok(wrap(&tokens))
}

// ---------------------------------------------------------------- canonical form (DD-RUL-016)

/// The version of [`canonical_form`]'s output (SDD 3.11.10 "Version rule", D-68 (c), SF-03,
/// OQ-RUL-005 (a)). `chess-app` stores `sha256:v<CANONICAL_FORM_VERSION>:<hex>`. A change to
/// the bytes `canonical_form` produces after release must bump this number in the same change.
pub const CANONICAL_FORM_VERSION: u32 = 1;

/// The bytes `chess-app` hashes for `content_hash` (SDD 3.11.10, `v1`): the Seven Tag Roster,
/// `FEN`, `SetUp` and `TimeControl` where present (`SetUp` and `FEN` are left out for the
/// standard start, all six fields, OQ-27), then the mainline SAN with the result. No comments,
/// NAGs or variations, no wrapping. A mainline move that is illegal in the replay ends the
/// movetext there: a `PgnGame` from the reader or built from legal moves never has one.
pub fn canonical_form(game: &PgnGame) -> Vec<u8> {
    let mut out = String::new();
    for (name, value) in roster(game) {
        out.push_str(&tag_line(name, &value));
    }
    if is_custom_start(game) {
        out.push_str(&tag_line("FEN", &game.start.to_fen().to_string()));
        out.push_str(&tag_line("SetUp", "1"));
    }
    if let Some(tc) = game.tags.time_control {
        out.push_str(&tag_line("TimeControl", &tc.to_string()));
    }
    out.push('\n');
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = game.start.clone();
    for (i, node) in game.moves.iter().enumerate() {
        let Ok(san) = cur.san(node.mv) else { break };
        match cur.turn() {
            Side::White => tokens.push(format!("{}.", cur.fullmove_number())),
            Side::Black if i == 0 => tokens.push(format!("{}...", cur.fullmove_number())),
            Side::Black => {}
        }
        tokens.push(san.to_string());
        let Ok(next) = cur.play(node.mv) else { break };
        cur = next;
    }
    tokens.push(game.result.as_str().to_string());
    out.push_str(&tokens.join(" "));
    out.push('\n');
    out.into_bytes()
}
