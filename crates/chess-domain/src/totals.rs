//! SDD 3.11.13 (D-63, SF-01, D-78): the accrual rule for the three time totals of a finished
//! game. Pure; the session feeds elapsed monotonic time between state changes.

use chess_rules::Side;

use crate::game::Fact;
use crate::types::GameState;

/// Where elapsed time in a state goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accrual {
    /// The side whose used time is charged, if any.
    pub side: Option<Side>,
    /// Whether it also counts toward `total_active_ms`.
    pub total: bool,
}

/// The table of 3.11.13. `DrawOffered` is active time for the total but charged to no side.
pub fn time_accrual(state: &GameState, side_to_move: Side) -> Accrual {
    match state {
        GameState::AwaitingMove | GameState::EngineThinking | GameState::EngineError => Accrual {
            side: Some(side_to_move),
            total: true,
        },
        GameState::DrawOffered => Accrual {
            side: None,
            total: true,
        },
        GameState::Paused
        | GameState::Suspended
        | GameState::Completed
        | GameState::Finalized
        | GameState::Aborted
        | GameState::Voided => Accrual {
            side: None,
            total: false,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimeTotals {
    pub white_used_ms: u64,
    pub black_used_ms: u64,
    pub total_active_ms: u64,
}

impl TimeTotals {
    /// Charges `elapsed_ms` as `accrual` says. Saturating; time is never refunded.
    pub fn add(&mut self, accrual: Accrual, elapsed_ms: u64) {
        match accrual.side {
            Some(Side::White) => {
                self.white_used_ms = self.white_used_ms.saturating_add(elapsed_ms);
            }
            Some(Side::Black) => {
                self.black_used_ms = self.black_used_ms.saturating_add(elapsed_ms);
            }
            None => {}
        }
        if accrual.total {
            self.total_active_ms = self.total_active_ms.saturating_add(elapsed_ms);
        }
    }

    /// D-78 (option A): sum `think_ms` per mover over every `Move` fact given, taken-back plies
    /// included; a `Move` without `think_ms` adds 0; `Clock` facts are ignored; the total is
    /// the white sum plus the black sum. The mover of a `Move` is found from the ply number and
    /// the side to move at the start (the `Start` fact; White if the slice has none).
    ///
    /// Stated loss, part of the definition: a resumed game may omit the unfinished turn at a
    /// stop and any `DrawOffered` time; it is not required to.
    pub fn seed_from_facts(facts: &[Fact]) -> TimeTotals {
        let mut first = Side::White;
        let mut totals = TimeTotals::default();
        for fact in facts {
            match fact {
                Fact::Start { setup, .. } => first = setup.start.side_to_move(),
                Fact::Move { ply, think_ms, .. } => {
                    let mover = if ply % 2 == 0 { first } else { first.other() };
                    totals.add(
                        Accrual {
                            side: Some(mover),
                            total: false,
                        },
                        think_ms.unwrap_or(0),
                    );
                }
                _ => {}
            }
        }
        totals.total_active_ms = totals.white_used_ms.saturating_add(totals.black_used_ms);
        totals
    }
}
