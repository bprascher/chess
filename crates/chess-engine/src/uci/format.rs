//! `format_*`: the only place a UCI command is built. Arguments are typed; no caller text is
//! concatenated, and every `setoption` value is checked against the handshake option
//! (Stockfish accepts out-of-range values silently, so the wrapper range-checks, TBC-8).
//! Every returned line is free of `\r` and `\n`; the driver appends the terminator.

use std::time::Duration;

use super::options::{OptionKind, OptionSpec, OptionValue, SetOption};
use crate::error::EngineError;
use crate::types::{SearchLimits, SearchPosition, UciMove};

pub fn format_uci() -> String {
    "uci".to_string()
}

pub fn format_isready() -> String {
    "isready".to_string()
}

pub fn format_ucinewgame() -> String {
    "ucinewgame".to_string()
}

pub fn format_stop() -> String {
    "stop".to_string()
}

pub fn format_quit() -> String {
    "quit".to_string()
}

/// Validates `set` against the advertised `spec` and builds the line.
pub fn format_setoption(spec: &OptionSpec, set: &SetOption) -> Result<String, EngineError> {
    let bad = || EngineError::InvalidPersonality {
        option: spec.name.clone(),
    };
    if spec.name != set.name || spec.name.contains(['\n', '\r']) {
        return Err(bad());
    }
    let text = match (&spec.kind, &set.value) {
        (OptionKind::Check { .. }, OptionValue::Bool(b)) => b.to_string(),
        (OptionKind::Spin { min, max, .. }, OptionValue::Int(v)) if (*min..=*max).contains(v) => {
            v.to_string()
        }
        (OptionKind::Combo { vars, .. }, OptionValue::Text(t)) if vars.contains(t) => t.clone(),
        (OptionKind::Str { .. }, OptionValue::Text(t)) if !t.contains(['\n', '\r']) => t.clone(),
        _ => return Err(bad()),
    };
    Ok(format!("setoption name {} value {}", spec.name, text))
}

pub fn format_position(position: &SearchPosition) -> Result<String, EngineError> {
    fn fen_text(fen: &chess_rules::Fen) -> Result<&str, EngineError> {
        let s = fen.as_str();
        if s.contains(['\n', '\r']) {
            return Err(EngineError::InvalidRequest {
                reason: "position text contains a line break",
            });
        }
        Ok(s)
    }
    fn moves_tail(moves: &[UciMove]) -> String {
        if moves.is_empty() {
            String::new()
        } else {
            let list: Vec<&str> = moves.iter().map(UciMove::as_str).collect();
            format!(" moves {}", list.join(" "))
        }
    }
    Ok(match position {
        SearchPosition::Fen(fen) => format!("position fen {}", fen_text(fen)?),
        SearchPosition::Moves { start: None, moves } => {
            format!("position startpos{}", moves_tail(moves))
        }
        SearchPosition::Moves {
            start: Some(fen),
            moves,
        } => format!("position fen {}{}", fen_text(fen)?, moves_tail(moves)),
    })
}

/// `go movetime T [depth D] [nodes N]`. `T` is whole milliseconds, at least 1.
pub fn format_go(limits: &SearchLimits) -> String {
    let ms = duration_ms(limits.movetime).max(1);
    let mut line = format!("go movetime {ms}");
    if let Some(d) = limits.depth {
        line.push_str(&format!(" depth {d}"));
    }
    if let Some(n) = limits.nodes {
        line.push_str(&format!(" nodes {n}"));
    }
    line
}

fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
