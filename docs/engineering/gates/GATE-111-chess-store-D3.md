# GATE-111: chess-store part D3 (apply path: `apply_records`, reconcile-plan applier, apply backlog, `sync_backlog`)

| Field | Value |
|---|---|
| Gate | **GATE-111** (follows GATE-110, part D5) |
| Tier | **G3.** Q2 yes (new public items of `chess-store`: `Store::apply_records`, `Store::sync_backlog`, `Store::backlog_status`, `ApplyOutcome`, `BacklogStatus`) makes it G2 on its own. Q8 yes: the change writes **stored shapes and values** that are irreversible once released: `MOVE.ply = journal ply + 1` (D-92), the `GAME` row that a `Start` record creates (`started_at` = `wall_ms`, frozen `rated`/`self_play`/`custom_start`), the `GAME.termination` text that a `GameEnd` reason maps to, the void fields written from a `Voided` record, and `GAME.journal_seq` rules (monotone for a batch, exact for a plan). Per GATE-103 and the D2 precedent (GATE-109) the highest tier any question gives applies |
| Approver | **Brian Prascher** (G2 and G3 per SWE-001 section 3). A desk review of the System Architect never satisfies G2 or G3 |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate. GATE-108, GATE-109 and GATE-110 are also not approved |
| Date opened | 2026-11-23 |
| Opened by | Software engineer (task chess-a0r.3.39, Engineer stance, run-to-gate, CR chess-a0r.20, baseline B6) |
| Covers | `crates/chess-store` only, plus two documents. New: `src/repo/apply.rs`, `src/backlog.rs`, `src/store/d3.rs`, `tests/apply.rs`, `tests/apply_backlog.rs`. Additive or behavior-preserving edits: `src/thread.rs` (command loop, backlog, tests), `src/store.rs` (spawn wiring, one private option field, one test-support builder), `src/api.rs` (two types appended), `src/lib.rs` (module and exports), `src/repo/mod.rs` (one line), `src/repo/finalize.rs` (the bodies of `finalize_game` and `abort_game` moved into `finalize_in_tx` and `abort_in_tx`; the wrappers keep the transaction, the failpoint and the commit; D2 tests unchanged and green). Not edited: `error.rs`, `fault.rs` (the two failpoints were already registered), `lifecycle.rs`, every SDD, root `Cargo.toml`, `deny.toml`, `ci/` |

## CHG-111 Implement chess-store part D3

- **Goal and driver:** acceptance of chess-a0r.3.39: SDD 3.7.2 (`apply_records`, `sync_backlog`), 3.11.1 (DB effect per record kind), 3.11.3 (the plan and the ply base, D-92), 3.11.4 (`Voided` from a record, D-87), 3.12 (apply backlog).
- **Components touched:** `crates/chess-store` only. Q1 no. Q3 no. Q4 no (`node ci/check-crate-edges.mjs`, see evidence). No dependency added.
- **Interfaces touched (public surface of `chess-store`, no consumer yet):** new `Store::apply_records(GameId, Vec<JournalRecord>) -> Result<ApplyOutcome, StoreError>`, `Store::sync_backlog(Duration) -> Result<(), StoreError>`, `Store::backlog_status() -> Result<BacklogStatus, StoreError>`; new `ApplyOutcome { Applied, Queued }`, `BacklogStatus`; test-support only: `StoreOptions::with_monotonic_clock`, `MonotonicClock`. `StoreOptions` gains a **private** field (a struct literal outside the crate would no longer compile; `StoreOptions::new` is the constructor and no code in the workspace builds it by literal `[OBS grep]`). `backlog_status` is not in the SDD method list; it is how a caller reads the `StoreBacklog` condition (FIND-D3-12).
- **Stored formats touched:** no DDL change, no journal format change. Values written: see Tier.
- **Gate answers:** Q1 no. Q2 yes -> G2. Q3 no. Q4 no. Q5 no. Q6 no for the model (the single store thread still owns the connection; the backlog and its timer live on it, as SDD DD-ST-002 says). Q7 no ADR contradicted; SDD silences and conflicts are FIND-D3-xx in the evidence file. Q8 yes -> G3. **Result: G3.**
- **Options considered:** (1) the backlog scheduler as a pure type with an injected clock (`backlog.rs`) driven by the store thread, versus a timer task on tokio (a second execution context; rejected: SDD 3.12 and DD-ST-002 put the timer on the store thread); (2) queueing a batch whose first attempt hit a busy database versus returning `Busy` to the caller (chosen: queue; the journal already holds the record, and play must not wait, SEQ-ST-02); (3) an error that no retry cures: drop the batch, or park the game (chosen: return the error on the first attempt and queue nothing; on a retry park the game so order is kept and nothing is lost, FIND-D3-02); (4) `journal_seq` after a batch: set to the last record's `seq`, or never lower it (chosen: never lower; the plan applier sets it exactly because the rebase lowers it on purpose, FIND-D3-07).
- **Consumer impact:** `chess-app` (part A, task chess-a0r.3.12, blocked on this) calls `apply_records` after each journal append, reads `backlog_status` to emit `StoreBacklog`, passes a whole journal (or at least a `GameEnd` with its `Voided`) in one call for D-87, and treats `Err` as "this record cannot be applied" (FIND-D3-01). `chess-store` part D4 (`.3.40`) calls the plan applier (`repo::apply::apply_reconcile_plan`, crate-private, carries `#[allow(dead_code)]` until D4 uses it).
- **Migration and rollback plan:** nothing is released and no store exists outside tests. Rollback: delete the five new files and the evidence file, revert the additive hunks in the files named in "Covers" (the `finalize.rs` change is a move of two function bodies; reverting restores the single-function form), delete this record.
- **Spike for the riskiest assumption (G3):** the riskiest assumption is that a batch (rows, clocks, `journal_seq`, and the D2 finalize, abort or void) commits atomically and that a crash on either side of the commit is repaired by applying again. `[RAN]` by the fault tests (a fault just before the commit changes no row of any table; a fault just after it leaves the batch committed and the retry changes nothing; a SIGKILLed child at each point reopens and the same batch ends in the same state both times). Power-loss behavior is not tested (`[UNV]`).
- **Rollback tested:** no (nothing at runtime to roll back; a revert is the rollback).
- **Independent reviewer:** waived for v1 by Brian (FIND-016, SWE-001 section 3). The Reviewer pass has **not** happened.
- **Evidence:** docs/engineering/step2-store-D3-evidence.md.
- **Reviewer findings and dispositions:** none yet. The Engineer's own findings are FIND-D3-01 to FIND-D3-nn in the evidence file, all OPEN. FIND-D3-01 (a `GameEnd` or `Abort` record with `ended_at <= 0` cannot be applied) needs a ruling of Brian on whether the journal format must refuse it (FIND-F2-04); it is not decided.

| Item | Status |
|---|---|
| Reviewer pass (part D3) | NOT DONE |
| G3 review step | not requested yet |
| Approved by | nobody |

## Rulings 2026-11-24

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** No approval has been received (decision bead chess-a0r.3.56 records rulings, not a gate approval).
- **Built (details `step2-store-D3-evidence.md`, section "Rulings 2026-11-24"):** (1) `ended_at` rule, G3; (2) uncurable-batch policy, G3 (park on the first permanent error, `GameParked`, bounds 256 per game and 4 096 in all as **ESTIMATE**, `unpark_game` drops the held queue, `parked_games`, `sync_backlog` returns `GameParked`, transient SQLite codes `NOMEM`, `INTERRUPT`, `PROTOCOL`, `SCHEMA` classified transient via `Busy`); (3) `Start.matchup` `hh|hc|cc`, `BacklogStatus` `#[non_exhaustive]`, new public items `parked_games`, `unpark_game`, `ParkedGame`, `ParkCause`, `StoreError::{GameParked, BacklogFull}`, `MAX_BATCHES_PER_GAME`, `MAX_PENDING_BATCHES` (G2, public surface for Brian), D3-09 flush before live terminal commands, timer, waiter and sync fixes (FIND-D3R-03, -04, -08, -11).
- **Gate answers added:** Q2 yes (public surface, G2), Q8 yes (stored format, G3). No new component, edge or dependency.
- **Rollback:** revert the hunks named in the evidence file; tested: no.
- **Open for Brian / SDD owner:** `unpark_game` returns `Option<usize>`, the bounds are estimates, and `sync_backlog` now says `GameParked` where SDD 3.11.9 and the dated note (1) of 2026-11-23 say `Busy`; `StoreError` gained two variants (`non_exhaustive`).
- **Reviewer pass of the 2026-11-24 changes:** NOT DONE. Approved by: nobody.
