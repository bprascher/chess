//! The ECO table generator (DD-RUL-013, SDD 3.11.9, D-75). Included by `build.rs` and by
//! `tests/eco_build.rs`; it is not a module of the library. It replays every row of the five
//! TSV files with `shakmaty` (it cannot call the crate it builds), derives the key with the
//! shared `key_derive::epd`, resolves duplicate positions (UR-27) and renders the static table.
//! A row that does not replay is an error naming the file, the line and the SAN token.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use shakmaty::san::San;
use shakmaty::{CastlingSide, Chess, Color, EnPassantMode, Position as _, Role, Square};

use crate::key_derive;

/// One table entry: the key (EPD of the position), the code, the name and the ply count of the
/// line that names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub epd: String,
    pub eco: String,
    pub name: String,
    pub ply: usize,
}

pub struct Generated {
    /// Data rows read.
    pub rows: usize,
    /// Rows whose position was already in the table.
    pub duplicates: usize,
    /// Sorted by key, one per position.
    pub entries: Vec<Entry>,
    /// Rust source of the table (`TABLE_LEN` and `TABLE`).
    pub source: String,
}

const FILES: [&str; 5] = ["a.tsv", "b.tsv", "c.tsv", "d.tsv", "e.tsv"];

fn letter(role: Role) -> char {
    match role {
        Role::Pawn => 'p',
        Role::Knight => 'n',
        Role::Bishop => 'b',
        Role::Rook => 'r',
        Role::Queen => 'q',
        Role::King => 'k',
    }
}

fn epd_of(pos: &Chess) -> String {
    let board = pos.board();
    let castles = pos.castles();
    let rights = |color, side| castles.has(color, side);
    key_derive::epd(
        |i| {
            board
                .piece_at(Square::new(u32::from(i)))
                .map(|p| (p.color == Color::White, letter(p.role)))
        },
        pos.turn() == Color::White,
        [
            rights(Color::White, CastlingSide::KingSide),
            rights(Color::White, CastlingSide::QueenSide),
            rights(Color::Black, CastlingSide::KingSide),
            rights(Color::Black, CastlingSide::QueenSide),
        ],
        pos.ep_square(EnPassantMode::Legal)
            .and_then(|sq| u8::try_from(u32::from(sq)).ok()),
    )
}

/// A move-number token such as `1.` or `12...`.
fn is_move_number(token: &str) -> bool {
    token
        .strip_suffix('.')
        .map(|t| t.trim_end_matches('.'))
        .is_some_and(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
}

fn replay(pgn: &str, place: &str) -> Result<(String, usize), String> {
    let mut pos = Chess::default();
    let mut plies = 0usize;
    for token in pgn.split_whitespace().filter(|t| !is_move_number(t)) {
        let mv = San::from_ascii(token.as_bytes())
            .map_err(|_| format!("{place}: not a SAN move: {token:?} in {pgn:?}"))?
            .to_move(&pos)
            .map_err(|_| format!("{place}: illegal SAN move: {token:?} in {pgn:?}"))?;
        pos.play_unchecked(mv);
        plies += 1;
    }
    if plies == 0 {
        return Err(format!("{place}: the pgn column has no moves"));
    }
    Ok((epd_of(&pos), plies))
}

/// Reads `a.tsv` to `e.tsv` from `dir`. Columns `eco`, `name`, `pgn` (tab separated, one header
/// line). Duplicate positions keep the longer line; a tie keeps the smaller code, then the
/// smaller name (UR-27).
pub fn generate(dir: &Path) -> Result<Generated, String> {
    let mut table: BTreeMap<String, Entry> = BTreeMap::new();
    let mut rows = 0usize;
    let mut duplicates = 0usize;
    for file in FILES {
        let text = std::fs::read_to_string(dir.join(file))
            .map_err(|e| format!("{file}: cannot read the file: {e}"))?;
        for (index, line) in text.lines().enumerate() {
            let number = index + 1;
            let place = format!("{file}:{number}");
            let cols: Vec<&str> = line.split('\t').collect();
            if number == 1 {
                if cols != ["eco", "name", "pgn"] {
                    return Err(format!("{place}: the header must be eco, name, pgn"));
                }
                continue;
            }
            let [eco, name, pgn] = cols.as_slice() else {
                return Err(format!("{place}: expected three tab-separated columns"));
            };
            if eco.is_empty() || name.is_empty() {
                return Err(format!("{place}: empty eco or name column"));
            }
            let (epd, ply) = replay(pgn, &place)?;
            rows += 1;
            let entry = Entry {
                epd: epd.clone(),
                eco: (*eco).to_string(),
                name: (*name).to_string(),
                ply,
            };
            match table.get_mut(&epd) {
                None => {
                    table.insert(epd, entry);
                }
                Some(current) => {
                    duplicates += 1;
                    let better = entry.ply > current.ply
                        || (entry.ply == current.ply
                            && (entry.eco.as_str(), entry.name.as_str())
                                < (current.eco.as_str(), current.name.as_str()));
                    if better {
                        *current = entry;
                    }
                }
            }
        }
    }
    let entries: Vec<Entry> = table.into_values().collect();
    let mut source = String::new();
    let _ = writeln!(source, "pub const TABLE_LEN: usize = {};", entries.len());
    source.push_str("static TABLE: &[(&str, EcoEntry)] = &[\n");
    for e in &entries {
        let _ = writeln!(
            source,
            "    ({:?}, EcoEntry {{ code: {:?}, name: {:?} }}),",
            e.epd, e.eco, e.name
        );
    }
    source.push_str("];\n");
    Ok(Generated {
        rows,
        duplicates,
        entries,
        source,
    })
}
