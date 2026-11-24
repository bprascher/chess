//! DD-EN-008: personality and config to a `setoption` list, checked against the handshake.
//! Pure. The `UCI_Elo` clamp is for the search only and never changes the stored value
//! (D-46; placement is OQ-EN-002).

use crate::config::EngineConfig;
use crate::error::EngineError;
use crate::pinned::PINNED_UCI_ELO_RANGE;
use crate::types::EnginePersonality;
use crate::uci::{OptionKind, OptionValue, SetOption, UciOptions};

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceProfile {
    Computer(EnginePersonality),
    Analysis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanReport {
    /// The requested `UCI_Elo` was moved into the advertised range.
    pub clamped: bool,
    /// The advertised `UCI_Elo` range is not `PINNED_UCI_ELO_RANGE` (an installation defect).
    pub differs_from_pinned: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionPlan {
    pub options: Vec<SetOption>,
    pub report: PlanReport,
}

fn int(name: &str, v: i64) -> SetOption {
    SetOption {
        name: name.to_string(),
        value: OptionValue::Int(v),
    }
}

fn flag(name: &str, v: bool) -> SetOption {
    SetOption {
        name: name.to_string(),
        value: OptionValue::Bool(v),
    }
}

fn invalid(option: &str) -> EngineError {
    EngineError::InvalidPersonality {
        option: option.to_string(),
    }
}

impl OptionPlan {
    pub fn build(
        profile: &InstanceProfile,
        config: &EngineConfig,
        advertised: &UciOptions,
    ) -> Result<OptionPlan, EngineError> {
        let (threads, hash) = match profile {
            InstanceProfile::Computer(_) => (config.computer_threads, config.computer_hash_mb),
            InstanceProfile::Analysis => (config.analysis_threads, config.analysis_hash_mb),
        };
        let mut options = Vec::new();
        // Threads and Hash are sent only when advertised (a foreign override binary may lack them).
        if advertised.get("Threads").is_some() {
            options.push(int("Threads", i64::from(threads)));
        }
        if advertised.get("Hash").is_some() {
            options.push(int("Hash", i64::from(hash)));
        }
        let range = advertised.uci_elo_range();
        let mut report = PlanReport {
            clamped: false,
            differs_from_pinned: range
                != Some((PINNED_UCI_ELO_RANGE.min, PINNED_UCI_ELO_RANGE.max)),
        };
        match profile {
            InstanceProfile::Analysis => {
                if advertised.get("UCI_LimitStrength").is_some() {
                    options.push(flag("UCI_LimitStrength", false));
                }
            }
            InstanceProfile::Computer(p) => match p.uci_elo {
                Some(elo) => {
                    let (min, max) = range.ok_or_else(|| invalid("UCI_Elo"))?;
                    if advertised.get("UCI_LimitStrength").is_none() {
                        return Err(invalid("UCI_LimitStrength"));
                    }
                    // `uci_elo_range` never returns an inverted range, so `clamp` cannot panic.
                    let sent = elo.clamp(min, max);
                    report.clamped = sent != elo;
                    options.push(flag("UCI_LimitStrength", true));
                    options.push(int("UCI_Elo", i64::from(sent)));
                }
                None => {
                    if advertised.get("UCI_LimitStrength").is_some() {
                        options.push(flag("UCI_LimitStrength", false));
                    }
                    match advertised.get("Skill Level").map(|s| &s.kind) {
                        Some(OptionKind::Spin { min, max, .. }) => {
                            let v = i64::from(p.skill_level);
                            if !(*min..=*max).contains(&v) {
                                return Err(invalid("Skill Level"));
                            }
                            options.push(int("Skill Level", v));
                        }
                        _ => return Err(invalid("Skill Level")),
                    }
                }
            },
        }
        Ok(OptionPlan { options, report })
    }
}
