# GATE-103: chess-domain (implementation, part A)

| Field | Value |
|---|---|
| Gate | **GATE-103** (numbering from GATE-101 per SWE-001 ASM-S5; condition chess-ckq C-7, readiness-step2-3.md C-7) |
| Tier | **G1** (one crate; architecture 3.2 edges unchanged; 5.7 contracts unchanged; see the Q-answers) |
| Approver | System Architect, **desk review**, to be requested after the Reviewer pass |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-06 |
| Opened by | Software engineer (task chess-a0r.2.6, part A, Engineer stance) |
| Covers | Part A (chess-a0r.2.6): identity newtypes, `Fact` vocabulary, `Game` state machine (`decide`, `evolve`, `replay`), `Policy`, `Clock` and `ClockSpec`, `TimeTotals`, Elo and rating eligibility, category bands, the `TimeSource` and `Rng` traits, and the `observe` and `classify` half of `EcoTable`. Part B (chess-a0r.2.7) is covered by the section CHG-103-B below; the `ts-rs` derives are still not done |

## CHG-103 Implement chess-domain part A per SDD-chess-domain 1.0 BASELINED

- **Goal and driver:** PLY-5 (mode policy), RULE-1 to RULE-5 (termination, draw offer, takeback, clocks, resume by replay), RULE-8 and D-64 (`StartPosition`), GAME-1 to GAME-4 (setup, slots, stale ply), ELO-1 and ELO-2, DAT-3 (think time, time totals), NFR-2, NFR-3. Functions F-2.2.4, F-2.3.1, F-2.4.1, F-4.1, F-4.2, F-4.4; supports F-2.1.1, F-2.1.2, F-2.2.3, F-2.5.x, F-2.6.x. Design entities DD-DOM-001, 003, 004, 005, 006, 007, 008, 009, 010, 011, 012, and the `observe`/`classify` part of DD-DOM-015; DD-DOM-002 in part (traits and fakes; `SplitMix64` and `error_pick_seed` are part B); DD-DOM-019 (errors, part A types).
- **Components touched:** `crates/chess-domain` only (`Cargo.toml`, `src/`, `tests/`), plus `Cargo.lock`, which records the new dependency resolution (two new lock entries: `uuid` 1.29.0 and `sha1_smol` 1.0.1). No other crate, no document except this record and the evidence file, and no `ci/` or `deny.toml` file was edited.
- **Interfaces touched:** adds the part A provided interface of IFP-DOM-1, 2, 3, 4 and 10 (traits only) and the `EcoTable::observe`/`classify` half of IFP-DOM-6. Nothing exists yet that consumes it, so no existing contract changes.
- **Dependencies added:** `serde =1.0.229` (feature `derive`), `thiserror =2.0.21`, `uuid =1.29.0` (`default-features = false`, features `v5` and `serde`), dev-dependency `proptest =1.12.0`. All four are in `docs/engineering/dependency-lookup-step1.md` (rows F8, P1, P2; D-47). `ts-rs` is not added (part B, feature `ts`). `uuid` feature `v5` is declared now for the roster (part B) so that part B does not edit the manifest; part A uses only `serde`. `sha1_smol` (BSD-3-Clause, via `v5`) is a transitive crate named in the lookup (F1 (e)); `cargo deny` licences pass. A feature `test-support` (empty) is declared in `[features]` for `FakeTimeSource` and `ScriptedRng`.
- **Gate answers:**
  - Q1 no (one crate).
  - Q2 no (first implementation of the SDD cards; no existing interface changes).
  - Q3 no (the crate exists in 3.1; the workspace member list is unchanged).
  - Q4 no (`ci/check-crate-edges.mjs`: `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`; `domain -> rules` is an existing edge).
  - Q5 no (the crate owns no table; the `Fact` and text forms it defines are the shapes of SDD 3.5 and 3.11.1, and no stored format changes).
  - Q6 no (no thread, task, lock or channel; SDD section 6).
  - Q7 no (no ADR, quality attribute scenario or architecture statement is contradicted; the places where the SDD is silent or ambiguous are choices inside the SDD's freedom and are listed under "Choices" below).
  - Q8 no (ordinary criticality, SWE-001).
  - Q9 yes (new observable behaviour of one existing component, architecture intact).
  - Q10 no.
  - **Result G1.**
- **Design summary:** the SDD design as written. `Game` is an event-sourced value: `decide` checks a command and returns facts, `evolve` applies one fact to a copy and never reads a clock, `replay` folds facts through the same code. `Policy` is one decision function over `(Action, GameContext)`, with `allowed_actions` derived from it. `Clock` takes time as arguments. `EloCalc` follows ELO-1 exactly. `TimeTotals` states the accrual rule and the D-78 seed.
- **Choices inside the SDD's freedom (the Reviewer and the SA desk review should confirm; numbered as in the evidence file section 9):**
  - F-D1 test-only fakes behind a feature that plain `cargo test --workspace` does not enable for integration tests;
  - F-D2 `journal_eq` normalises states a journal cannot express (`EngineThinking`, `EngineError`, a step in progress, `Finalized`) and does not compare the clock of a finished game; `Pause` is accepted from `Paused` on replay;
  - F-D3 `Fact::Move.ply` is the zero-based index of the move and `think_ms` is `Option<u64>`;
  - F-D4 `GameCommand::Void` carries the reset Player; `GameCommand::RespondDraw` carries no actor;
  - F-D5 a game with no move played has no pending termination;
  - F-D6 the `Policy` table's open points (the analysis exemption list, `Suspended` is not "in progress", the order of reasons);
  - F-D7 `ClockSync` remaining times are `Option<u64>`; `ClockSpec` serialises as its `<base>+<increment>` text;
  - F-D8 `Checkpoint` re-bases the running clock so the live value equals the journaled value;
  - F-D9 "same entry by transposition" holds only when the final position is itself named (SDD 3.11.9 wording).
- **Options considered:** not required below G2. Two alternatives were weighed and are recorded for the Reviewer: (a) a `Fact` per transient state (`EngineStarted`, `Step`, `Finalized`) so that `evolve` is the only mutator without exception: rejected because it adds journal record kinds (the SDD fixes the 1:1 table, D-41, and a new kind is G3); (b) leaving `Game` equality to `PartialEq`: rejected because the live game holds runtime clock state a replay cannot have.
- **Evidence:** docs/engineering/step1-domain-A-evidence.md.
- **Tests added:** UT-DOM-001, 002 (fake time and `Rng::below` only), 010 to 014, 020 to 026, 030 to 046, 050 to 059, 070 (part A subset), 100, 103, 105, 106, 108, 114 (part A subset), 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, FZ-DOM-001; see the evidence file for the trace and for the IDs not delivered.
- **Risk and rollback:** working tree only, nothing committed. Rollback is deleting `crates/chess-domain/src`, `crates/chess-domain/tests`, restoring `crates/chess-domain/Cargo.toml` (git) and `Cargo.lock` (git), and deleting this record and the evidence file.
- **Escalation check (G2 triggers):** none found. The SDD did not force an edge or a 5.7 change. One observation for the app owner: the public API names three `chess-rules` types (`Side`, `Move`, `Fen`, plus `Position` in accessors), so a front-end that wants to name them goes through the curated `chess-app` re-export list (architecture 5.7) or the app wraps them. That is the app SDD's decision and does not change this crate's edges. See evidence section 9, F-D10.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance and has not happened.

## CHG-103-B Implement chess-domain part B per SDD-chess-domain 1.0 BASELINED (task chess-a0r.2.7)

| Field | Value |
|---|---|
| Tier | **G1** (same crate; no edge, no 5.7 contract, no stored format changed; see the Q-answers) |
| Approver | System Architect, **desk review**, to be requested after the Reviewer pass |
| Status | **OPEN. NOT APPROVED.** Nothing has been approved by anyone. The author is the Engineer stance |
| Date opened | 2026-11-06 |
| Covers | `BoardGrid` and friends (DD-DOM-017); `RosterGen`, `Personality`, names, `SplitMix64`, `error_pick_seed` (DD-DOM-002 remainder, 013, 014, 020); `normalize_eco_tag`, `classify_import`, `resolve` (DD-DOM-015); `VariationTree` (DD-DOM-016); stats (DD-DOM-018); error types of DD-DOM-019; the UT-DOM-115 drift-guard table |

- **Goal and driver:** PLY-4, ENG-2, D-34, D-43, D-46 (roster), TR-12 and FF-8 (`SplitMix64`, `error_pick_seed`), CLI-1, CLI-2, UI-3, XC-12, TR-1.6 (`BoardGrid`), DAT-3, CR-9.8, CR-9.11 to CR-9.13, D-59, D-60 (ECO), ANA-2, CR-9.4 (`VariationTree`), UI-2, UI-5, D-44 (stats), RULE-8, D-64, SF-02 (drift guard).
- **Components touched:** `crates/chess-domain` only: new `src/{board_grid,roster,stats,variation}.rs`; additions to `src/{edge,eco,error,lib}.rs`; new tests and test data under `tests/`. `Cargo.toml` and `Cargo.lock` are unchanged (`uuid` feature `v5`, `serde` and `thiserror` were already declared; no `ts-rs`, no `insta`). Nothing in `crates/chess-rules` was edited. Part A behaviour is unchanged: the existing 130 part A tests still pass.
- **Interfaces touched:** adds the part B provided interface of IFP-DOM-5 to IFP-DOM-9 and the remainder of IFP-DOM-6 and IFP-DOM-10. Nothing consumes it yet, so no existing contract changes.
- **Gate answers:** Q1 no; Q2 no (first implementation of the SDD cards); Q3 no; Q4 no (no manifest change; `ci/check-crate-edges.mjs` result in the evidence file); Q5 no (`Personality`, `RosterSeed`, the UUID derivation and the roster name string are the shapes the SDD and D-34 fix; the choices inside the SDD's freedom are listed below); Q6 no; Q7 no; Q8 no; Q9 yes; Q10 no. **Result G1.**
- **Choices inside the SDD's freedom, for the Reviewer and the SA desk review (F-B numbers as in docs/engineering/step1-domain-B-evidence.md section 9):** F-B2 `ROSTER_NAMESPACE` value; F-B3 the drift-guard table lives in `chess-domain/tests/data` and reads the rules test source; F-B4 the `EcoFields` empty name; F-B5 `RosterSeed(u32)`; F-B6 `MAX_ROSTER_INDEX` and `MAX_NAME_SUFFIX` bounds; F-B7 `saved_line` semantics; F-B8 `check_history` has no sum error; F-B9 `to_pgn_text` returns `Result`; F-B10 `StatsRow.result_for_viewed` is optional; F-B11 `Rewound` is set on the last-move squares.
- **Escalation check (G2 triggers):** none found. `ts-rs` and the `ts` feature were not needed by any part B item (they derive types that part A and the app SDD own) and were not added; the step that generates TypeScript types adds them (a `Cargo.toml` edit then).
- **Evidence:** docs/engineering/step1-domain-B-evidence.md.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance and has not happened.

| Item | Status |
|---|---|
| Reviewer pass (part B) | NOT DONE |
| System Architect desk review (part B) | NOT REQUESTED yet (after the Reviewer pass) |
| Approved by | nobody |

## Approval record

| Item | Status |
|---|---|
| Reviewer pass (part A) | NOT DONE |
| System Architect desk review (part A) | NOT REQUESTED yet (after the Reviewer pass) |
| Approved by | nobody |

## Rework note (chess-a0r.2.12, 2026-11-06, Engineer stance)

Minor findings DF-02, DF-03 and DF-06 of `review-chess-domain-code.md` were worked; DF-07 was resolved as "SDD silent, behavior unchanged, finding recorded". DF-01, DF-05, DF-08, DF-09 and DF-10 were not touched (owner decisions pending). No public signature change, no journal contract change, no change to `chess-rules`. Gate class of the rework: G0 (tests, doc comments) plus a behavior-preserving-in-range fix in one function (`Tally::games`). Evidence: `docs/engineering/step1-domain-rework-evidence.md`. Dispositions of the findings belong to the named owners; none is recorded here. Approved by: nobody.
