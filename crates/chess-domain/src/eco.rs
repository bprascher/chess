//! DD-DOM-015: the "deepest named position" rule over `chess_rules::eco::lookup` (SDD 3.11.9),
//! and the import side (IFP-DOM-6, D-59, D-60): `normalize_eco_tag`, `classify_import` and
//! `resolve`. All pure; the crate asks nothing and remembers no choice.

use core::fmt;

use chess_rules::PositionKey;

use crate::error::EcoUnanswered;

/// The code and name of the deepest named position reached, and the ply it was reached at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcoMatch {
    pub code: String,
    pub name: String,
    pub ply: u32,
}

pub struct EcoTable;

impl EcoTable {
    /// A deeper named position replaces a shallower one; an unnamed one leaves it.
    pub fn observe(current: Option<EcoMatch>, key: &PositionKey, ply: u32) -> Option<EcoMatch> {
        match chess_rules::eco::lookup(key) {
            Some(entry) => Some(EcoMatch {
                code: entry.code.to_string(),
                name: entry.name.to_string(),
                ply,
            }),
            None => current,
        }
    }

    /// Full recompute over the keys of a game: `keys[0]` is the start position, ply 0.
    pub fn classify(keys: &[PositionKey]) -> Option<EcoMatch> {
        let mut found = None;
        for (i, key) in keys.iter().enumerate() {
            let ply = u32::try_from(i).unwrap_or(u32::MAX);
            found = EcoTable::observe(found, key, ply);
        }
        found
    }
}

// ---------------------------------------------------------------- import (part B)

/// An ECO code in its one stored spelling: a letter `A` to `E` and two digits (`A00` to `E99`).
/// Only `normalize_eco_tag` builds one from text, so a value is always well formed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EcoCode(String);

impl EcoCode {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EcoCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The ECO tag of an imported game as a code, or `None` when it is malformed (D-60): ASCII
/// whitespace is trimmed at both ends and ASCII letters are upper-cased; the result must match
/// `^[A-E][0-9]{2}$`. A malformed tag counts as no tag.
pub fn normalize_eco_tag(tag: &str) -> Option<EcoCode> {
    let upper = tag
        .trim_matches(|c: char| c.is_ascii_whitespace())
        .to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let well_formed = matches!(bytes, [b'A'..=b'E', b'0'..=b'9', b'0'..=b'9']);
    well_formed.then_some(EcoCode(upper))
}

/// How an imported game's tag and the table's classification relate (IFP-DOM-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EcoImportOutcome {
    /// The normalised tag equals the table's code.
    Agree { code: EcoCode, name: String },
    /// Both exist and the codes differ; the player chooses.
    Mismatch { tag_code: EcoCode, table: EcoMatch },
    /// A well-formed tag and no match in the table.
    TagOnly { tag_code: EcoCode },
    /// No tag, or a malformed one, and a match in the table.
    TableOnly { table: EcoMatch },
    /// No usable tag and no match.
    Neither,
}

/// The player's answer to a mismatch. The domain never remembers it (OQ-ECO-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcoChoice {
    KeepPgn,
    KeepTable,
}

/// What is stored in `GAME.eco` and `GAME.opening_name`. `None` is "no value"; an empty name
/// (`Some("")`) is the stored result of keeping the PGN's code, which has no name (CR-8, DAT-3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EcoFields {
    pub code: Option<String>,
    pub name: Option<String>,
}

/// `tag` is the raw `ECO` tag of the file (if any); `classified` is `EcoTable::classify` over
/// the keys of the imported game.
pub fn classify_import(tag: Option<&str>, classified: Option<EcoMatch>) -> EcoImportOutcome {
    match (tag.and_then(normalize_eco_tag), classified) {
        (Some(tag_code), Some(table)) if tag_code.as_str() == table.code => {
            EcoImportOutcome::Agree {
                code: tag_code,
                name: table.name,
            }
        }
        (Some(tag_code), Some(table)) => EcoImportOutcome::Mismatch { tag_code, table },
        (Some(tag_code), None) => EcoImportOutcome::TagOnly { tag_code },
        (None, Some(table)) => EcoImportOutcome::TableOnly { table },
        (None, None) => EcoImportOutcome::Neither,
    }
}

/// The values to store. Only `Mismatch` needs `choice`; for every other variant it is ignored.
/// `Mismatch` without a choice is `EcoUnanswered`: the caller must ask, this crate asks nothing.
pub fn resolve(
    outcome: &EcoImportOutcome,
    choice: Option<EcoChoice>,
) -> Result<EcoFields, EcoUnanswered> {
    let from_table = |m: &EcoMatch| EcoFields {
        code: Some(m.code.clone()),
        name: Some(m.name.clone()),
    };
    let from_tag = |c: &EcoCode| EcoFields {
        code: Some(c.as_str().to_string()),
        name: Some(String::new()),
    };
    match outcome {
        EcoImportOutcome::Agree { code, name } => Ok(EcoFields {
            code: Some(code.as_str().to_string()),
            name: Some(name.clone()),
        }),
        EcoImportOutcome::Mismatch { tag_code, table } => match choice {
            Some(EcoChoice::KeepPgn) => Ok(from_tag(tag_code)),
            Some(EcoChoice::KeepTable) => Ok(from_table(table)),
            None => Err(EcoUnanswered),
        },
        EcoImportOutcome::TagOnly { tag_code } => Ok(from_tag(tag_code)),
        EcoImportOutcome::TableOnly { table } => Ok(from_table(table)),
        EcoImportOutcome::Neither => Ok(EcoFields::default()),
    }
}
