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

/// A case where the reader dropped or changed information from the file and said nothing in
/// the data (RF-04, desk review R7, B6). The list is `PgnGame::warnings`; it never affects
/// `canonical_form` or the writer. A comment that has text and sits between two games (before
/// the tag section, or before the movetext of a game without tags) belongs to no move; it is
/// reported as `CommentDropped` on the game that follows it, and not reported at all when no
/// game follows.
///
/// The list of one game holds at most [`MAX_IMPORT_WARNINGS_PER_GAME`] warnings of the kinds
/// below (RC-03); any further warning is counted and the list ends with one
/// [`ImportWarning::WarningsTruncated`] entry, so the list has at most cap + 1 entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportWarning {
    /// A comment before the first move of a sequence (at the start of the movetext or right
    /// after `(`) has no move to attach to and was dropped. 1-based line and byte column of
    /// its first character.
    CommentDropped { line: usize, col: usize },
    /// The `Date` tag was not `YYYY.MM.DD`, or a part was out of range; the unreadable part
    /// (or the whole date) is unknown. `value` is the tag text.
    DateIgnored { value: String },
    /// A `TimeControl` not of the form `seconds[+increment]` was dropped (`-` and `?` are
    /// not reported: they say "none" and "unknown").
    TimeControlIgnored { value: String },
    /// A `Termination` outside the three written values was dropped.
    TerminationIgnored { value: String },
    /// One text payload (a tag value or a comment) was not valid UTF-8 and was decoded as
    /// Latin-1 (IF-2). The decision is per payload: other payloads of the game that are valid
    /// UTF-8 are read as UTF-8. There is one entry per payload that fell back, in the order the
    /// payloads were met (so a game can carry several); the entry has no position. A comment
    /// that is dropped (`CommentDropped`) is still decoded and still counts.
    Latin1Fallback,
    /// The last entry of a list that hit the cap: `dropped` further warnings of this game were
    /// counted and not kept (RC-03). Always the final entry, and never present with fewer
    /// than [`MAX_IMPORT_WARNINGS_PER_GAME`] entries before it.
    WarningsTruncated { dropped: usize },
}

/// The most warnings kept for one game (RC-03); a bound on memory for hostile input such as a
/// file of millions of `{a}` comments. Further warnings are summarized by
/// [`ImportWarning::WarningsTruncated`]. ESTIMATE: far above what a real game produces (a
/// handful), small enough that a game costs a few tens of kilobytes at most.
pub const MAX_IMPORT_WARNINGS_PER_GAME: usize = 256;

impl fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportWarning::CommentDropped { line, col } => write!(
                f,
                "the comment at line {line}, column {col} comes before the first move of its line and was dropped"
            ),
            ImportWarning::DateIgnored { value } => {
                write!(
                    f,
                    "the date {value:?} is not fully readable; the unreadable parts are unknown"
                )
            }
            ImportWarning::TimeControlIgnored { value } => {
                write!(
                    f,
                    "the time control {value:?} is not seconds or seconds+increment and was dropped"
                )
            }
            ImportWarning::TerminationIgnored { value } => {
                write!(
                    f,
                    "the termination {value:?} is not one this application writes and was dropped"
                )
            }
            ImportWarning::Latin1Fallback => f.write_str(
                "a text in the game is not valid UTF-8, so that text was read as Latin-1",
            ),
            ImportWarning::WarningsTruncated { dropped } => {
                write!(
                    f,
                    "{dropped} further import warnings for this game were not kept"
                )
            }
        }
    }
}

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
    /// What the reader dropped or changed (empty for a game built in code). Not part of
    /// `canonical_form` and not written by `write_pgn`.
    pub warnings: Vec<ImportWarning>,
}

impl PgnGame {
    /// A game with default tags.
    pub fn new(start: Position, moves: Vec<PgnNode>, result: PgnResult) -> PgnGame {
        PgnGame {
            tags: PgnTags::default(),
            start,
            moves,
            result,
            warnings: Vec::new(),
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
/// NAGs or variations, no wrapping.
///
/// **Illegal tail (RF-05 a).** A `PgnGame` from the reader or built from legal moves never has
/// an illegal mainline move. If one is present anyway, the movetext does not end silently
/// there: the legal prefix is followed by the token `{illegal <ply> <uci> <uci> ...}` (the
/// 0-based index of the first illegal move, then that move and every move after it in UCI
/// text, separated by single spaces) and then the result. The movetext line of a legal game
/// (the line after the blank line) never contains `{`: it holds only move numbers, SAN and the
/// result. Tag values are not covered by this claim: a tag value may contain `{`, and
/// `canonical_form` writes it unchanged on a tag line (a tag value cannot contain a newline, so
/// it cannot reach the movetext line). So the marker changes no byte of any legal game's
/// output, and two games that differ only in an illegal tail have different bytes.
///
/// **Accepted collision (control characters).** A control character in a tag value is written
/// as a space (ASM-RUL-004), so a tab and a space in a tag value give the same bytes. This is
/// accepted and documented, not a defect to fix: changing it would change the bytes of `v1`
/// (a G3 stored-format matter; decided by Brian Prascher, 2026-11-09).
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
        let (Ok(san), Ok(next)) = (cur.san(node.mv), cur.play(node.mv)) else {
            tokens.push(illegal_tail_marker(i, &game.moves));
            break;
        };
        match cur.turn() {
            Side::White => tokens.push(format!("{}.", cur.fullmove_number())),
            Side::Black if i == 0 => tokens.push(format!("{}...", cur.fullmove_number())),
            Side::Black => {}
        }
        tokens.push(san.to_string());
        cur = next;
    }
    tokens.push(game.result.as_str().to_string());
    out.push_str(&tokens.join(" "));
    out.push('\n');
    out.into_bytes()
}

/// `{illegal <ply> <uci> ...}`: the first illegal mainline move and everything after it.
fn illegal_tail_marker(ply: usize, moves: &[PgnNode]) -> String {
    let mut marker = format!("{{illegal {ply}");
    for node in moves.iter().skip(ply) {
        marker.push(' ');
        marker.push_str(&node.mv.to_uci());
    }
    marker.push('}');
    marker
}
