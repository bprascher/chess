//! The option table from the `uci` handshake, and the typed `setoption` values.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionKind {
    Check { default: bool },
    Spin { default: i64, min: i64, max: i64 },
    Combo { default: String, vars: Vec<String> },
    Button,
    Str { default: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSpec {
    pub name: String,
    pub kind: OptionKind,
}

impl OptionSpec {
    /// The default as `setoption` text, for the applied-options map.
    pub fn default_text(&self) -> Option<String> {
        match &self.kind {
            OptionKind::Check { default } => Some(default.to_string()),
            OptionKind::Spin { default, .. } => Some(default.to_string()),
            OptionKind::Combo { default, .. } | OptionKind::Str { default } => {
                Some(default.clone())
            }
            OptionKind::Button => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

/// One planned `setoption` (name is the engine's own option name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetOption {
    pub name: String,
    pub value: OptionValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UciOptions {
    map: HashMap<String, OptionSpec>,
}

impl UciOptions {
    pub fn new() -> UciOptions {
        UciOptions::default()
    }

    pub fn insert(&mut self, spec: OptionSpec) {
        self.map.insert(spec.name.clone(), spec);
    }

    pub fn get(&self, name: &str) -> Option<&OptionSpec> {
        self.map.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &OptionSpec> {
        self.map.values()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `min` and `max` of the `UCI_Elo` spin, when advertised, non-negative and not inverted
    /// (an inverted range is engine output we cannot use: `None`, EF-05).
    pub fn uci_elo_range(&self) -> Option<(u32, u32)> {
        match self.get("UCI_Elo").map(|s| &s.kind) {
            Some(OptionKind::Spin { min, max, .. }) => {
                let (min, max) = (u32::try_from(*min).ok()?, u32::try_from(*max).ok()?);
                (min <= max).then_some((min, max))
            }
            _ => None,
        }
    }
}
