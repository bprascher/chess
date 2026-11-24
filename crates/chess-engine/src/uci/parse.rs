//! `parse_line`: a pure, total function (SDD 3.10.1). Tokens are matched by keyword, not by
//! position. Unknown or malformed input never panics.

use super::options::{OptionKind, OptionSpec};
use crate::types::{Score, UciMove};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InfoLine {
    pub depth: Option<u32>,
    pub multipv: Option<u8>,
    pub score: Option<Score>,
    /// `lowerbound` or `upperbound` was present: not usable as a final line.
    pub bound: bool,
    pub pv: Vec<UciMove>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UciMessage {
    IdName(String),
    IdAuthor(String),
    Option(OptionSpec),
    UciOk,
    ReadyOk,
    BestMove(UciMove),
    /// `bestmove (none)` or `bestmove 0000`.
    NoMove,
    Info(InfoLine),
    /// `info string ...`: ignored by the driver.
    InfoString,
    /// A line that is not UCI output we know. Ignored by the driver.
    Unknown(String),
    /// A `bestmove` whose move is not a syntactic UCI move.
    MalformedBestMove,
    /// Another known keyword with an unusable payload.
    Malformed,
}

pub fn parse_line(line: &str) -> UciMessage {
    let mut tokens = line.split_whitespace();
    match tokens.next() {
        Some("id") => parse_id(line, tokens),
        Some("option") => parse_option(tokens).map_or(UciMessage::Malformed, UciMessage::Option),
        Some("uciok") => UciMessage::UciOk,
        Some("readyok") => UciMessage::ReadyOk,
        Some("bestmove") => parse_bestmove(tokens),
        Some("info") => parse_info(tokens),
        _ => UciMessage::Unknown(line.to_string()),
    }
}

fn parse_id<'a>(_line: &str, mut tokens: impl Iterator<Item = &'a str>) -> UciMessage {
    let which = tokens.next();
    let rest: Vec<&str> = tokens.collect();
    let text = rest.join(" ");
    match which {
        Some("name") if !text.is_empty() => UciMessage::IdName(text),
        Some("author") if !text.is_empty() => UciMessage::IdAuthor(text),
        _ => UciMessage::Malformed,
    }
}

fn parse_bestmove<'a>(mut tokens: impl Iterator<Item = &'a str>) -> UciMessage {
    match tokens.next() {
        Some("(none)" | "0000") => UciMessage::NoMove,
        Some(text) => {
            UciMove::parse(text).map_or(UciMessage::MalformedBestMove, UciMessage::BestMove)
        }
        None => UciMessage::MalformedBestMove,
    }
}

const OPTION_KEYWORDS: [&str; 4] = ["default", "min", "max", "var"];

fn parse_option<'a>(tokens: impl Iterator<Item = &'a str>) -> Option<OptionSpec> {
    let tokens: Vec<&str> = tokens.collect();
    let mut it = tokens.iter().copied().peekable();
    if it.next()? != "name" {
        return None;
    }
    let mut name = Vec::new();
    while let Some(t) = it.peek().copied() {
        if t == "type" {
            break;
        }
        name.push(t);
        it.next();
    }
    it.next()?; // "type"
    let kind_word = it.next()?;
    if name.is_empty() {
        return None;
    }
    let (mut default, mut min, mut max) = (None::<String>, None::<i64>, None::<i64>);
    let mut vars: Vec<String> = Vec::new();
    while let Some(key) = it.next() {
        if !OPTION_KEYWORDS.contains(&key) {
            continue;
        }
        let mut value = Vec::new();
        while let Some(t) = it.peek().copied() {
            if OPTION_KEYWORDS.contains(&t) {
                break;
            }
            value.push(t);
            it.next();
        }
        let text = value.join(" ");
        match key {
            "default" => default = Some(text),
            "min" => min = text.parse().ok(),
            "max" => max = text.parse().ok(),
            _ => vars.push(text),
        }
    }
    let kind = match kind_word {
        "check" => OptionKind::Check {
            default: match default.as_deref() {
                Some("true") => true,
                Some("false") => false,
                _ => return None,
            },
        },
        "spin" => OptionKind::Spin {
            default: default?.parse().ok()?,
            min: min?,
            max: max?,
        },
        "combo" => OptionKind::Combo {
            default: default.unwrap_or_default(),
            vars,
        },
        "button" => OptionKind::Button,
        "string" => OptionKind::Str {
            default: default.unwrap_or_default(),
        },
        _ => return None,
    };
    Some(OptionSpec {
        name: name.join(" "),
        kind,
    })
}

fn parse_info<'a>(mut tokens: impl Iterator<Item = &'a str>) -> UciMessage {
    let mut info = InfoLine::default();
    while let Some(t) = tokens.next() {
        match t {
            "string" => return UciMessage::InfoString,
            "depth" => info.depth = tokens.next().and_then(|v| v.parse().ok()),
            "multipv" => info.multipv = tokens.next().and_then(|v| v.parse().ok()),
            "score" => {
                let kind = tokens.next();
                let value = tokens.next().and_then(|v| v.parse::<i32>().ok());
                info.score = match (kind, value) {
                    (Some("cp"), Some(v)) => Some(Score::Cp(v)),
                    (Some("mate"), Some(v)) => Some(Score::Mate(v)),
                    _ => None,
                };
            }
            "lowerbound" | "upperbound" => info.bound = true,
            "pv" => {
                for m in tokens.by_ref() {
                    match UciMove::parse(m) {
                        Ok(mv) => info.pv.push(mv),
                        Err(_) => break,
                    }
                }
                break;
            }
            "refutation" | "currline" => break,
            // keyword followed by one value we do not use
            "seldepth" | "nodes" | "nps" | "hashfull" | "tbhits" | "time" | "currmove"
            | "currmovenumber" | "cpuload" | "sbhits" => {
                tokens.next();
            }
            _ => {}
        }
    }
    UciMessage::Info(info)
}
