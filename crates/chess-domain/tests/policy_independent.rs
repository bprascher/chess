//! C-D4 / DF-08 / desk review D3: the policy table derived independently from the SDD 3.11.5 and
//! architecture 4.4 text (tests/data/policy_independent_table.txt), compared with
//! `Policy::check` and `Policy::allowed_actions` over every real context. It shares no code with
//! the oracle in tests/policy.rs. Where the table and the implementation differ, the difference
//! is a `KNOWN` row in the data file (a finding for the Systems Engineer); the implementation is
//! not changed here. A difference that is not a known row fails the test, and a known row that
//! no longer matches any difference fails it too.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use std::collections::BTreeMap;

use chess_domain::{
    Action, ActionKind, Actor, DeniedReason, GameCommand as C, GameContext, GameState as S,
    Matchup, Mode, PlayerKind as K, Policy,
};
use chess_rules::Side;
use common::*;

const TABLE: &str = include_str!("data/policy_independent_table.txt");

// ---------------------------------------------------------------- the table

#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    Ok,
    Reason(String),
    Deny,
    Unspecified,
}

impl Verdict {
    fn parse(text: &str) -> Verdict {
        match text.trim() {
            "ok" => Verdict::Ok,
            "deny" => Verdict::Deny,
            "?" => Verdict::Unspecified,
            reason => Verdict::Reason(reason.to_string()),
        }
    }

    fn of(result: Result<(), chess_domain::Denied>) -> Verdict {
        match result {
            Ok(()) => Verdict::Ok,
            Err(d) => Verdict::Reason(d.reason.as_str().to_string()),
        }
    }

    fn text(&self) -> String {
        match self {
            Verdict::Ok => "ok".into(),
            Verdict::Reason(r) => r.clone(),
            Verdict::Deny => "deny".into(),
            Verdict::Unspecified => "?".into(),
        }
    }
}

#[derive(Debug)]
struct Row {
    id: String,
    actions: Vec<String>,
    negate_actions: bool,
    conds: Vec<(String, bool, Vec<String>)>,
    verdict: Verdict,
}

fn parse_row(id: &str, body: &str, verdict: &str) -> Row {
    let mut tokens = body.split_whitespace();
    let first = tokens.next().unwrap();
    let (negate_actions, list) = match first.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, first),
    };
    let conds = tokens
        .map(|t| {
            let (key, value) = t.split_once('=').unwrap_or_else(|| panic!("bad token {t}"));
            let (neg, value) = match value.strip_prefix('!') {
                Some(v) => (true, v),
                None => (false, value),
            };
            (
                key.to_string(),
                neg,
                value.split(',').map(str::to_string).collect(),
            )
        })
        .collect();
    Row {
        id: id.to_string(),
        actions: list.split(',').map(str::to_string).collect(),
        negate_actions,
        conds,
        verdict: Verdict::parse(verdict),
    }
}

fn load() -> (Vec<Row>, Vec<(Row, Verdict)>) {
    let mut rows = Vec::new();
    let mut known = Vec::new();
    for line in TABLE.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (left, verdict) = line
            .split_once("=>")
            .unwrap_or_else(|| panic!("no verdict: {line}"));
        let parts: Vec<&str> = left.splitn(2, '|').collect();
        if parts[0].trim() == "KNOWN" {
            // KNOWN | FINDING-ID | ACTIONS CONDITIONS => impl=VERDICT
            let rest: Vec<&str> = parts[1].splitn(2, '|').collect();
            let row = parse_row(rest[0].trim(), rest[1], "?");
            let implv = verdict.trim().strip_prefix("impl=").unwrap();
            known.push((row, Verdict::parse(implv)));
        } else {
            rows.push(parse_row(parts[0].trim(), parts[1], verdict));
        }
    }
    (rows, known)
}

// ---------------------------------------------------------------- the cases

#[derive(Debug, Clone)]
struct Case {
    action: Action,
    ctx: GameContext,
}

impl Case {
    fn name(&self) -> &'static str {
        match self.action {
            Action::Takeback => "Takeback",
            Action::LiveAnalysis => "LiveAnalysis",
            Action::WhatIf => "WhatIf",
            Action::PostGameAnalysis => "PostGameAnalysis",
            Action::Rewind => "Rewind",
            Action::SelectClock => "SelectClock",
            Action::OfferDraw { .. } => "OfferDraw",
            Action::RespondDraw { .. } => "RespondDraw",
            Action::Pause => "Pause",
            Action::PlayResume => "PlayResume",
            Action::Step => "Step",
            Action::SetMoveDelay => "SetMoveDelay",
            Action::Resign { .. } => "Resign",
            Action::Abort => "Abort",
            Action::CancelEngine => "CancelEngine",
            Action::RetryEngine => "RetryEngine",
            Action::MakeMove { .. } => "MakeMove",
        }
    }

    fn side(&self) -> Option<Side> {
        match self.action {
            Action::OfferDraw { by }
            | Action::RespondDraw { by, .. }
            | Action::MakeMove { by, .. } => Some(by),
            Action::Resign { side } => Some(side),
            _ => None,
        }
    }

    fn attr(&self, key: &str) -> Option<String> {
        let c = &self.ctx;
        let kind = |k: K| if k == K::Human { "H" } else { "C" };
        Some(match key {
            "mode" => (if c.mode == Mode::Training { "T" } else { "S" }).into(),
            "matchup" => match c.matchup {
                Matchup::HumanHuman => "HH",
                Matchup::HumanComputer => "HC",
                Matchup::ComputerComputer => "CC",
            }
            .into(),
            "state" => match c.state {
                S::AwaitingMove => "A",
                S::EngineThinking => "ET",
                S::EngineError => "EE",
                S::DrawOffered => "DO",
                S::Paused => "P",
                S::Suspended => "SU",
                S::Completed => "CO",
                S::Finalized => "FI",
                S::Aborted => "AB",
                S::Voided => "VO",
            }
            .into(),
            "tomove" => kind(c.kind_of(c.turn)).into(),
            "by" => (if self.side()? == c.turn {
                "turn"
            } else {
                "other"
            })
            .into(),
            "bykind" => kind(c.kind_of(self.side()?)).into(),
            "actor" => match self.action {
                Action::RespondDraw { actor, .. } | Action::MakeMove { actor, .. } => {
                    (if actor == Actor::Human { "H" } else { "E" }).into()
                }
                _ => return None,
            },
            "byofferee" => match self.action {
                Action::RespondDraw { by, .. } => (if c.pending_offer == Some(by) {
                    "no"
                } else {
                    "yes"
                })
                .into(),
                _ => return None,
            },
            "blocked" => match self.action {
                Action::OfferDraw { by } => (if c.offer_blocked_for == Some(by) {
                    "by"
                } else {
                    "no"
                })
                .into(),
                _ => return None,
            },
            other => panic!("unknown key {other}"),
        })
    }
}

fn matches(row: &Row, case: &Case) -> bool {
    let named = row.actions.iter().any(|a| a == case.name());
    if named == row.negate_actions {
        return false;
    }
    row.conds.iter().all(|(key, neg, values)| {
        case.attr(key)
            .is_some_and(|actual| values.contains(&actual) != *neg)
    })
}

fn reachable(c: &GameContext) -> bool {
    if c.mode == Mode::Training && c.matchup != Matchup::HumanComputer {
        return false;
    }
    let offer_state = matches!(c.state, S::DrawOffered | S::Suspended);
    match c.pending_offer {
        None => c.state != S::DrawOffered,
        Some(by) => {
            offer_state && c.kind_of(by) == K::Human && c.matchup != Matchup::ComputerComputer
        }
    }
}

fn cases() -> Vec<Case> {
    let seatings = [
        (Matchup::HumanHuman, K::Human, K::Human),
        (Matchup::HumanComputer, K::Human, K::Computer),
        (Matchup::HumanComputer, K::Computer, K::Human),
        (Matchup::ComputerComputer, K::Computer, K::Computer),
    ];
    let sides = [Side::White, Side::Black];
    let mut out = Vec::new();
    for mode in [Mode::Training, Mode::Strict] {
        for (matchup, white_kind, black_kind) in seatings {
            for &state in S::ALL {
                for turn in sides {
                    for pending in [None, Some(Side::White), Some(Side::Black)] {
                        for blocked in [None, Some(Side::White), Some(Side::Black)] {
                            let ctx = GameContext {
                                mode,
                                matchup,
                                state,
                                turn,
                                white_kind,
                                black_kind,
                                pending_offer: pending,
                                offer_blocked_for: blocked,
                            };
                            if !reachable(&ctx) {
                                continue;
                            }
                            let mut actions = vec![
                                Action::Takeback,
                                Action::LiveAnalysis,
                                Action::WhatIf,
                                Action::PostGameAnalysis,
                                Action::Rewind,
                                Action::SelectClock,
                                Action::Pause,
                                Action::PlayResume,
                                Action::Step,
                                Action::SetMoveDelay,
                                Action::Abort,
                                Action::CancelEngine,
                                Action::RetryEngine,
                            ];
                            for by in sides {
                                actions.push(Action::OfferDraw { by });
                                actions.push(Action::Resign { side: by });
                                for actor in [Actor::Human, Actor::Engine] {
                                    actions.push(Action::RespondDraw { by, actor });
                                    actions.push(Action::MakeMove { by, actor });
                                }
                            }
                            out.extend(actions.into_iter().map(|action| Case { action, ctx }));
                        }
                    }
                }
            }
        }
    }
    out
}

fn expected(rows: &[Row], case: &Case) -> (String, Verdict) {
    let row = rows
        .iter()
        .find(|r| matches(r, case))
        .unwrap_or_else(|| panic!("no row covers {case:?}: the table is incomplete"));
    (row.id.clone(), row.verdict.clone())
}

// ---------------------------------------------------------------- tests

#[test]
fn df_08_the_independent_table_is_complete_and_has_no_shadowed_rows() {
    let (rows, _) = load();
    let mut hits: BTreeMap<String, usize> = rows.iter().map(|r| (r.id.clone(), 0)).collect();
    assert_eq!(hits.len(), rows.len(), "row ids are unique");
    for case in cases() {
        let (id, _) = expected(&rows, &case);
        *hits.get_mut(&id).unwrap() += 1;
    }
    let dead: Vec<_> = hits.iter().filter(|(_, n)| **n == 0).collect();
    assert!(dead.is_empty(), "rows that decide no case: {dead:?}");
}

#[test]
fn df_08_policy_check_agrees_with_the_independent_table_except_the_known_findings() {
    let (rows, known) = load();
    let mut known_hits = vec![0usize; known.len()];
    let mut compared = 0usize;
    let mut unspecified = 0usize;
    let mut unspecified_impl: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut unexpected = Vec::new();
    for case in cases() {
        let (id, want) = expected(&rows, &case);
        let got = Verdict::of(Policy::check(case.action, &case.ctx));
        if want == Verdict::Unspecified {
            unspecified += 1;
            *unspecified_impl.entry((id, got.text())).or_insert(0usize) += 1;
            continue;
        }
        compared += 1;
        let agrees = match &want {
            Verdict::Deny => got != Verdict::Ok,
            other => *other == got,
        };
        if agrees {
            continue;
        }
        let mut explained = false;
        for (i, (row, implv)) in known.iter().enumerate() {
            if matches(row, &case) && *implv == got {
                known_hits[i] += 1;
                explained = true;
            }
        }
        if !explained {
            unexpected.push(format!(
                "{id}: {:?} in {:?}: table {} impl {}",
                case.action,
                case.ctx,
                want.text(),
                got.text()
            ));
        }
    }
    assert!(
        unexpected.is_empty(),
        "{} unexplained differences, first: {:#?}",
        unexpected.len(),
        &unexpected[..unexpected.len().min(5)]
    );
    for ((row, _), n) in known.iter().zip(&known_hits) {
        assert!(*n > 0, "stale KNOWN row {}", row.id);
    }
    assert!(compared > 5_000, "compared {compared} cases");
    println!(
        "compared {compared}, unspecified {unspecified} (implementation verdicts {unspecified_impl:?}), known hits {known_hits:?}"
    );
}

#[test]
fn df_08_allowed_actions_follow_the_independent_table() {
    let (rows, known) = load();
    let mut contexts: Vec<GameContext> = Vec::new();
    for case in cases() {
        if contexts.last().is_none_or(|c| *c != case.ctx) {
            contexts.push(case.ctx);
        }
    }
    let mut checked = 0usize;
    for ctx in contexts {
        let listed = Policy::allowed_actions(&ctx);
        for &kind in ActionKind::ALL {
            let mut any_ok = false;
            let mut any_unspecified = false;
            let mut explained_by_known = false;
            for case in cases_for(&ctx, kind) {
                let (_, want) = expected(&rows, &case);
                match want {
                    Verdict::Ok => any_ok = true,
                    Verdict::Unspecified => any_unspecified = true,
                    _ => {}
                }
                let got = Verdict::of(Policy::check(case.action, &case.ctx));
                if known.iter().any(|(r, v)| matches(r, &case) && *v == got) {
                    explained_by_known = true;
                }
            }
            if any_unspecified || explained_by_known {
                continue;
            }
            checked += 1;
            assert_eq!(
                listed.contains(&kind),
                any_ok,
                "{kind:?} in {ctx:?}: listed {listed:?}"
            );
        }
    }
    assert!(checked > 1_000, "checked {checked}");
}

fn cases_for(ctx: &GameContext, kind: ActionKind) -> Vec<Case> {
    let sides = [Side::White, Side::Black];
    let mut actions = Vec::new();
    match kind {
        ActionKind::Takeback => actions.push(Action::Takeback),
        ActionKind::LiveAnalysis => actions.push(Action::LiveAnalysis),
        ActionKind::WhatIf => actions.push(Action::WhatIf),
        ActionKind::PostGameAnalysis => actions.push(Action::PostGameAnalysis),
        ActionKind::Rewind => actions.push(Action::Rewind),
        ActionKind::SelectClock => actions.push(Action::SelectClock),
        ActionKind::Pause => actions.push(Action::Pause),
        ActionKind::PlayResume => actions.push(Action::PlayResume),
        ActionKind::Step => actions.push(Action::Step),
        ActionKind::SetMoveDelay => actions.push(Action::SetMoveDelay),
        ActionKind::Abort => actions.push(Action::Abort),
        ActionKind::CancelEngine => actions.push(Action::CancelEngine),
        ActionKind::RetryEngine => actions.push(Action::RetryEngine),
        ActionKind::OfferDraw => actions.extend(sides.map(|by| Action::OfferDraw { by })),
        ActionKind::Resign => actions.extend(sides.map(|side| Action::Resign { side })),
        ActionKind::RespondDraw => {
            for by in sides {
                for actor in [Actor::Human, Actor::Engine] {
                    actions.push(Action::RespondDraw { by, actor });
                }
            }
        }
        ActionKind::MakeMove => {
            for by in sides {
                for actor in [Actor::Human, Actor::Engine] {
                    actions.push(Action::MakeMove { by, actor });
                }
            }
        }
    }
    actions
        .into_iter()
        .map(|action| Case { action, ctx: *ctx })
        .collect()
}

#[test]
fn df_08_the_table_pins_the_reasons_in_the_data_file_not_only_ok_or_denied() {
    // A guard against a table that degenerates into ok/deny: it uses many distinct reasons.
    let (rows, _) = load();
    let mut reasons: Vec<String> = rows
        .iter()
        .filter_map(|r| match &r.verdict {
            Verdict::Reason(x) => Some(x.clone()),
            _ => None,
        })
        .collect();
    reasons.sort();
    reasons.dedup();
    for r in DeniedReason::ALL.iter().map(|r| r.as_str()) {
        // `training_untimed` as a DeniedReason is no longer produced by Policy::check after R-1
        // (2026-11-09); the clock rule lives in check_setup (SetupReason::TrainingUntimed). The
        // variant stays in the public enum (removing it is a protocol change, not made here).
        let exempt = matches!(
            r,
            "computer_player" | "active_player" | "archived_player" | "training_untimed"
        );
        assert!(
            exempt || reasons.iter().any(|x| x == r),
            "the table never expects {r}"
        );
    }
}

// ---------------------------------------------------------------- FD6 (f)

#[test]
fn fd6_f_an_engine_failure_drops_an_offer_to_a_computer_and_not_an_offer_to_a_human() {
    // Architecture 4.4: "An engine failure drops the offer (ENG-4)"; SDD 3.11.4: an engine
    // failure, or a suspend while the offer is to a computer, drops it with an explicit
    // `DrawDropped` fact. An offer to a human survives (no engine is involved).
    let mut hc = Run::new(strict_hc());
    hc.cmd(C::OfferDraw { by: Side::White });
    assert_eq!(hc.game.state(), S::DrawOffered);
    let step = hc.cmd(C::EngineFailed);
    assert!(
        step.facts.iter().any(|f| f.journal_kind() == "DrawDropped"),
        "{:?}",
        step.facts
    );
    assert_eq!(hc.game.pending_offer(), None);
    assert_eq!(hc.game.state(), S::AwaitingMove);

    let mut hh = Run::new(strict_hh());
    hh.cmd(C::OfferDraw { by: Side::White });
    assert!(hh.try_cmd(C::EngineFailed).is_err(), "no engine in h-h");
    assert_eq!(hh.game.pending_offer(), Some(Side::White));
}
