# GATE-112: chess-store part D4 (`open_for_resume`, rebase orchestration, `StartupRecovery`)

| Field | Value |
|---|---|
| Gate | **GATE-112** (follows GATE-111, part D3) |
| Tier | **G3.** Q2 yes (new public items, a changed public signature: see CHG-112) makes it G2 on its own. Q8 yes: the change decides **stored file names and states** that are irreversible once released: `journal/damaged/<uuid>.<k>.rebase` for an incomplete `.rebase` (B-R05), the removal of a header-only journal whose game has no row (STATE-ST-01), and the truncation of a torn tail at startup. |
| Approver | **Brian Prascher** (G2 and G3, SWE-001 section 3). A desk review never satisfies G2 or G3 |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-24 |
| Opened by | Software engineer (task chess-a0r.3.40, Engineer stance, run-to-gate, CR chess-a0r.20, baseline B6) |
| Covers | `crates/chess-store` only, plus the evidence file `docs/engineering/step2-store-D4-evidence.md` and this record. New: `src/journal/recover.rs`, `src/repo/resume.rs`, `src/store/d4.rs`, `tests/resume.rs`, `tests/recovery.rs`, `tests/recovery_kill.rs`, `tests/d4_faults.rs`, `tests/startup_bench.rs`, `tests/journal_race.rs`, `tests/d4_support/mod.rs`. Edited: `src/journal/files.rs`, `src/journal/writer.rs`, `src/journal/mod.rs`, `src/fault.rs`, `src/api.rs`, `src/open.rs`, `src/store.rs`, `src/repo/mod.rs`, `src/repo/apply.rs` (one `allow` removed), `tests/journal.rs`, `tests/journal_faults.rs`, `tests/journal_crash.rs`, `tests/fault_injection.rs`. No SDD, CI or root `Cargo.toml` edit |

## CHG-112 Implement chess-store part D4

- **Goal and driver:** acceptance of chess-a0r.3.40: DD-ST-019, SDD 3.7.3 (`open_for_resume`), 3.9.1, 3.9.2, 3.11.3 steps 2 and 3, crash windows (a) to (f), D-66, D-72, D-83, D-87, D-110, D-152, D-153; B-R03, B-R05, B-R04 status.
- **Components touched:** `crates/chess-store` only. Q1 no. Q3 no. Q4 no (`node ci/check-crate-edges.mjs` OK). No dependency added.
- **Interfaces touched (public surface):**
  - new: `MoveJournal::open_for_resume`, `journal::{Recovered, RecoveryReport, ResumeError, StartupReport, RecoveredGame, RecoveryOutcome, CloseRepair}`, `OpenReport.recovery` (a new field of an existing public struct), the failpoint `journal.close_final.linked`.
  - **changed:** `journal::settle_rebase_files(journal_dir, game)` is now `settle_rebase_files(journal_dir, game, store)`, and `Settled` gained `MovedAsideIncomplete { damaged }` and lost `Copy`.
  - **behavior:** `Store::open` now runs the startup recovery; `JournalWriter::close_final` is crash-idempotent; `MoveJournal::create` no longer removes a file whose lock another holder keeps.
- **SDD conflicts found (not edited):** (1) the SDD signature `open_for_resume -> Result<(JournalWriter, Recovered), JournalError>` cannot carry a database error nor a final game with no writer; built as `Result<(Option<JournalWriter>, Recovered), ResumeError>`. (2) `Recovered.moves` and `.clocks` use `StoredMove` and `ClockPair` (the SDD's `MoveRecord` and `Clocks` do not exist). (3) SDD 3.9.2 and 3.11.3 RR-A-1 (a) both say who writes the `Start` of a journal with no record; built as: the rebase writes it (empty intact run); the no-journal path creates a header-only file and the app appends it.
- **Stored formats touched:** no DDL change, no record-format change. New file names and states: see Tier.
- **Gate answers:** Q1 no. Q2 yes -> G2. Q3 no. Q4 no. Q5 no. Q6 no. Q7 no ADR contradicted. Q8 yes -> G3. **Result: G3.**
- **Options considered:** (1) an incomplete `.rebase` moved aside as `damaged/<uuid>.<k>.rebase` (chosen: bytes kept, kept apart from the `.jnl` names that CT-ST-088 counts) versus `.jnl` naming or deletion (rejected: never delete); (2) B-R04 fixed by a bounded lock grace in `create` (chosen) versus creating under a temporary name and linking (rejected: a new file name in the data directory); (3) startup recovery inside `Store::open` (chosen, as SEQ-ST-01 draws it) versus a separate call (rejected: the SDD diagram).
- **Consumer impact:** `chess-app` (chess-a0r.3.12) calls `open_for_resume`, reads `Recovered.report.damaged` to emit `JournalUnreadable`, appends `Start` when `report.created_empty`, and reads `OpenReport.recovery` for `NeedsRecord` (voided row with no `Voided` record: the app appends it).
- **Migration and rollback plan:** nothing is released. Rollback: delete the new files, revert the additive hunks in the edited files, restore the two-argument `settle_rebase_files`.
- **Spike for the riskiest assumption (G3):** a kill at any of the six rebase points, at each `journal.*` point, and between the link and the removal of `close_final` is repaired by the next start: see the evidence file (kill runs, counts as run).
- **Rollback tested:** no (a revert is the rollback).
- **Independent reviewer:** waived for v1 (FIND-016). The Reviewer pass has **not** happened.
- **Evidence:** docs/engineering/step2-store-D4-evidence.md.

| Item | Status |
|---|---|
| Reviewer pass (part D4) | NOT DONE |
| G3 review step | not requested yet |
| Approved by | nobody |

## Verification 2026-11-24 (Engineer stance, run-to-gate; baseline B6)

- **Status: still OPEN. NOT APPROVED. PENDING Brian Prascher (G3; public surface G2).** No approval has been received from anyone. This section records verification only. The Reviewer pass has **not** happened.
- **Evidence:** `docs/engineering/step2-store-D4-evidence.md` (all numbers `[RAN]`).
- **Runs:** `cargo test -p chess-store` x5 (487 passed each), with `--features test-support` x5 (575 each), `journal_race` x10 (3 passed each): no failure. `recovery_kill` with `CHESS_STORE_KILL_ROUNDS=50`: 7 passed (about 300 kill rounds per failpoint family). Clippy `-D warnings` with and without `test-support`, `cargo fmt --check`, `node ci/check-crate-edges.mjs`, and `CHESS_CI_OFFLINE=1 ci/check.sh`: all pass ("All gates passed", workspace 1 127 passed, chess-store with `test-support` 593 passed).
- **Mutants:** 62 hand mutants: 57 killed, 5 survived (all argued equivalent or unreachable, FIND-D4V-02 and FIND-D4V-04). 18 tests were added (14 in `tests/d4_verify.rs`, 1 in `tests/d4_verify_faults.rs`, 2 lib unit tests in `journal/recover.rs`, 1 in `tests/journal_race.rs`). **No code of part D4 was changed**: no defect was exposed.
- **B-R03, B-R05, B-R04:** implemented and verified as above (B-R04: 0 spurious `Locked` in 30 000 in-thread and 4 000 cross-process creates). Dispositions are the Engineer's; none is accepted by anyone.
- **Open findings (Engineer's, none Major):** FIND-D4V-01 to FIND-D4V-06. FIND-D4V-03 (a `Voided` after an applied `GameEnd` is closed unapplied) and FIND-D4V-04 (the `GameParked` branch of the scan is unreachable) are questions for the Systems Engineer and the System Architect.
- **Not done:** Reviewer pass; MC/DC; `cargo-mutants`; power-loss tests; Intel Macs.
- Approved by: nobody.

## Fixes 2026-11-25 (Engineer stance; answers `review-chess-store-D4.md`)

- **Status: still OPEN. NOT APPROVED. PENDING Brian Prascher (G3; public surface G2).** No approval, desk review or sign-off has been received for these fixes. The Reviewer's verdict was PASS WITH CONDITIONS with FIND-D4R-01 (Major) open; this section records what was changed for those conditions. It closes nothing by itself: a fresh Reviewer pass over the fixes has **not** happened.
- **Evidence:** `docs/engineering/step2-store-D4-evidence.md`, section "Fixes 2026-11-25" (per-item table F.1, baseline-fails list F.2, new findings F.3, mutants F.4, runs F.5).
- **Scope of change:** `crates/chess-store` only (`src/journal/{recover,files,rebase,writer}.rs`, `src/store/d4.rs`, `src/repo/apply.rs`, `src/fault.rs`; new `tests/d4_fixes.rs`, `tests/d4_fixes_faults.rs`; `tests/recovery.rs` one expectation changed). No SDD, CI or root `Cargo.toml` edit; no dependency added.
- **Conditions of the review, as handled:**

| Review condition | Handled |
|---|---|
| 1. FIND-D4R-01 closed with a test that fails before the fix without a sleep in the library | **Done.** The window is widened by a test-only failpoint (`journal.resume.no_file_seen`, no-op without `test-support`) and by two real processes; both fail on the old code. The fix is a re-check under the exclusive `.rebase` plus a swap that never moves a journal it does not hold. Same-family windows found on the way: FIND-D4F-01 (fixed) |
| 2. FIND-D4R-08: Brian decides the `open_for_resume` signature and the public surface | **Not touched, as instructed.** Added meanwhile, for the same decision: new variant `RecoveryOutcome::DoneNameCollision` (enum already `#[non_exhaustive]`); `settle_rebase_files` and `swap_in_rebased` keep their public signatures (the new forms are `pub(crate)`) |
| 3. FIND-D4R-02, -03, -05, -06 fixed or deferred; -07 recorded | **Fixed** (all four); -07 fixed (10 s total budget `ESTIMATE`), so the NFR-1 risk is bounded rather than only recorded |
| 4. FIND-D4R-09/-10: tests for W01/W03, R04/R05, F01, D23, D36 | **Done**; 46 of 48 new hand mutants killed, the 2 survivors argued (M10 equivalent, M29 state not reachable) |

- **Needs Brian (decisions, not mine):** D4R-08 (signature and public surface, now with `DoneNameCollision`); FIND-D4R-11 semantics; D-72 across processes (left alone); the rule for a different file at `done/` (keep both, report `DoneNameCollision`, resume proceeds) and the `journal_seq` witness for `.rebase` completeness (FIND-D4F-03) are architect-level wording changes to SDD 3.11.3 and are **not** made here.
- **Rollback:** a revert of the uncommitted diff (no stored format changed: file names and journal bytes are as before).
- Approved by: nobody.
