# GATE-107: chess-store part D1 (FaultPlan and crash-test harness)

| Field | Value |
|---|---|
| Gate | **GATE-107** (follows GATE-106; GATE-105 is part A) |
| Tier | **G1**: one crate (`crates/chess-store`), architecture 3.2 edges unchanged, §5.7 contracts unchanged. `FaultPlan` is IFP-ST-06 "test-only" in SDD-chess-store; the failpoint list is DD-ST-024. With the feature off the public surface and the behaviour of the crate are unchanged. When unsure between two tiers the higher applies: see "Escalation check" (two items for the architect, none for Brian) |
| Approver | System Architect, **desk review**, after the Reviewer pass. A desk review never satisfies G2 or G3 |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-10 |
| Opened by | Software engineer (task chess-a0r.3.37, Engineer stance, run-to-gate, CR chess-a0r.20, baseline B6) |
| Covers | `src/fault.rs` (DD-ST-024), the Cargo feature `test-support`, failpoint calls in `backup.rs`, `datadir.rs`, `open.rs`, `migrate.rs`, the kill-at-failpoint harness (`tests/crash.rs`), `tests/fault_injection.rs` (registry, CT-ST-085, the six Reviewer survivors) |

## CHG-107 Implement chess-store part D1

- **Goal and driver:** DD-ST-024, R-11, R-15, NFR-2; CT-ST-085 (D-71, D-109), CT-ST-090 kill case (D-108); the Reviewer's FIND-013 survivors M16, M17, M22, M30, M31, M35 of `review-chess-store-A.md`; acceptance (1) to (5) of chess-a0r.3.37 (store-completion-allocation.md 4.1).
- **Components touched:** `crates/chess-store` only: `Cargo.toml` (feature `test-support = []`, no dependency), `src/lib.rs`, `src/fault.rs` (new), `src/backup.rs`, `src/datadir.rs`, `src/open.rs`, `src/migrate.rs`, `tests/crash.rs` and `tests/fault_injection.rs` (new). Root `Cargo.toml`, `deny.toml`, `ci/` and every SDD were not edited.
- **Interfaces touched:** the new `chess_store::fault` module exists **only** with the feature (IFP-ST-06). With the feature off nothing is added to the public surface (compile-fail doctest in `lib.rs`; `cargo doc` check in the evidence file). No other public item changed.
- **Dependencies added:** none. No `libc`, no `rustix`, no `unsafe` (`#![forbid(unsafe_code)]` kept). The kill is a parent calling `std::process::Child::kill`.
- **Gate answers:** Q1 no. Q2 no (test-only, feature gated; off-build surface unchanged). Q3 no. Q4 no (`node ci/check-crate-edges.mjs` OK, evidence). Q5 no. Q6 no (a process-wide `Mutex` registry exists only in the test-support build). Q7 no. Q8 no. Q9 no behaviour of the feature-off build changes. Q10 no (new feature code). **Result: G1.**
- **Design summary:** a plan is installed for one data directory and applies to a failpoint call whose path has that prefix, so tests in one process do not disturb each other. Actions: error (errno or `ErrorKind`), delay, kill. Names are validated against a registry. With the feature off the call sites are `check(..) -> Ok(())` (`#[inline(always)]`) and the sync helpers reduce to `sync_all`.
- **Escalation check (items for the architect, not decided here):**
  1. The SDD section 4 list does not name the failpoints that part D1 added for part A code: `sync.probe`, `sync.backup_copy_file`, `sync.backup_copy_dir`, `sync.backup_rename_dir`, `sync.store_id_file`, `sync.store_id_dir`, `backup.verify`, `migrate.after_backup`, `open.after_db_created`, `open.after_store_id`. They are needed to kill the Reviewer's sync survivors and for the first-launch and upgrade kill tests. A dated SDD note (G0 text) is requested; they are registered in `fault::D1_FAILPOINTS`.
  2. CT-ST-087 and CT-ST-088 name six more failpoints (`journal.partial_write`, `journal.fsync_error`, `journal.rebase.built`, `.db_committed`, `.old_aside`, `.new_in_place`) that are not in the section 4 list. They are registered (`CT_NAMED_FAILPOINTS`) with no call site (part B/D4 wires them).
- **Evidence:** docs/engineering/step2-store-D1-evidence.md.
- **Tests added:** see the evidence file (UT/CT ids CT-ST-085, CT-ST-090 kill case, DD-ST-024 registry tests).
- **Risk and rollback:** working tree only, nothing committed. Rollback: delete `src/fault.rs`, `tests/crash.rs`, `tests/fault_injection.rs`; revert the failpoint call sites and the `[features]` block in `crates/chess-store/Cargo.toml` (the call-site edits are listed in the evidence file); delete this record and the evidence file.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance in a fresh invocation and has not happened. The Engineer's own findings are FIND-D1-01 to FIND-D1-0n in the evidence file, all OPEN.

| Item | Status |
|---|---|
| Reviewer pass (part D1) | NOT DONE |
| System Architect desk review (G1) | NOT REQUESTED yet (after the Reviewer pass) |
| Approved by | nobody |
