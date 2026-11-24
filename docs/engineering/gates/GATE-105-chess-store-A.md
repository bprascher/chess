# GATE-105: chess-store (implementation, part A)

| Field | Value |
|---|---|
| Gate | **GATE-105** (numbering from GATE-101 per SWE-001 ASM-S5; condition chess-ckq C-7, readiness-step2-3.md C-7: this record opens the `GATE-` file for the `chess-store` block) |
| Tier | **G1** for the code as written against SDD-chess-store 1.0 (one crate, architecture 3.2 edges unchanged, 5.7 contracts as the SDD states them), **with a carve-out**: the items in "Escalation check" below touch a stored shape, a data-directory file name, an interface or the error catalogue and are **not** decided by the SDD. When unsure between two tiers the higher applies, so those items are **G3 (stored shape or file name) or G2 (interface, catalogue) and need Brian Prascher**; they are listed, not decided |
| Approver | System Architect, **desk review**, to be requested after the Reviewer pass, for the G1 part. **Brian Prascher** for each carve-out item he decides to adopt or change. A desk review never satisfies G2 or G3 |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-09 |
| Opened by | Software engineer (task chess-a0r.3.6, part A, Engineer stance, run-to-gate per CR chess-a0r.19) |
| Covers | Part A: `DataDir` (DD-ST-004), schema DDL and migration 0001 (DD-ST-007), `Migrator` with the pre-upgrade backup (DD-ST-006, and the verified-copy half of DD-ST-020), the `Opener` up to the upgrade (DD-ST-005, part), `Store` handle, `StoreThread`, `ReadPool` (DD-ST-001 to 003), players, roster, settings and games read/start access (DD-ST-008, DD-ST-009 part), the rating-history read (DD-ST-010 part), `StoreError` (DD-ST-023, part). **Not covered:** journal, leases, process registry, `apply_records`, finalize, abort, void, reset, stats rows, evals, variations, job queue (see evidence finding F-A01), automatic backup, restore, row IO, `FaultPlan` |

## CHG-105 Implement chess-store part A per SDD-chess-store 1.0 BASELINED

- **Goal and driver:** DAT-1 (store, migrations), DAT-6 (backup before upgrade), DAT-4 (slot rows), DAT-7 (all-or-nothing writes), PLAT-3 (offline first launch), PLY-2, PLY-4, PLY-6 (register, roster, names), GAME-4 and D-27 (slots), D-40 (UUID-only boundary), D-46 (`roster_indexes()`), D-43 (`error_permille`), D-71 (volume probe, behaviour only), D-84 and OQ-26 (`started_at`). Functions F-6.1, F-6.5, F-6.6 (quick_check), F-6.8, F-6.9, F-6.4 (partly).
- **Components touched:** `crates/chess-store` only (`Cargo.toml`, `src/`, `tests/`, `migrations/`). Also `Cargo.lock`, which Cargo updated when this crate began to use the admitted pins (not edited by hand; the other crate in flight changes it too). Root `Cargo.toml`, `deny.toml`, `ci/` and every document except this record and `step2-store-A-evidence.md` were not edited.
- **Interfaces touched:** first implementation of the part-A subset of IFP-ST-01 and IFP-ST-03 (`Store::open`, `DataDir`, `OpenReport`). Nothing consumes them yet. Additions the SDD does not state are listed in the escalation check.
- **Dependencies added** (all inherited with `workspace = true` from the root entries pinned in step2-prep-evidence.md section 5; no entry added, no feature outside the lookup list): `tokio` (features `sync`, `rt`, `time`, DL2-5), `rusqlite` (bundled, no default features), `thiserror`, `serde`, `serde_json`, `tracing`, `uuid`; dev: `tempfile`, `proptest`. `crc32fast` and `test-support` were **not** added (parts B and C add them). No `libc`, no `rustix`, no `unsafe` (D-107, `#![forbid(unsafe_code)]` kept).
- **Gate answers:**
  - Q1 no (one crate).
  - Q2 no for the contracts the SDD states (first implementation); **yes-pending** for the additions in the escalation check (F-A05 a new error variant, F-A06 two DTO fields).
  - Q3 no (the crate exists in 3.1).
  - Q4 no (`node ci/check-crate-edges.mjs` -> `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`; the manifest names `chess-domain` only; no `chess-rules`).
  - Q5 no for the schema (implemented as approved in the stored-format batch, tested equal); **yes-pending** for stored shapes the SDD leaves open (F-A04, F-A06, F-A08).
  - Q6 no (store thread and read pool are the D-13 and 4.8 design).
  - Q7 no ADR or QAS contradicted. One possible conflict with the DAT-6 retry story is F-A02 (see below).
  - Q8 the stored-format and migration code is "irreversible" work in the SWE-001 sense, but nothing is released and no format is frozen; the carve-out items are rated G3 for that reason.
  - Q9 yes (new observable behaviour of one existing component, architecture intact). Q10 no.
  - **Result: G1, with the G2/G3 carve-out.**
- **Design summary:** repository functions over `&Connection` (no thread knowledge); `StoreThread` runs closures on the one write connection; `ReadPool` serves reads through `spawn_blocking`; `Store` is the one concrete handle; `Migrator` runs numbered SQL in `BEGIN EXCLUSIVE` after a verified `VACUUM INTO` copy; the `Opener` refuses a volume without `F_FULLFSYNC`, a missing store, a newer or corrupt file, before any write.
- **Module seams kept for parts B and C** (decisions, evidence section 9): `thread.rs` takes closures (B adds the apply backlog and retry timer on the same channel); `migrate.rs` has the `exclusive_guard` hook (B passes "no other live process"); `open.rs` has marked places for process registration and startup recovery; `backup.rs` holds only `write_verified_copy` (C adds `BackupManager`); `repo/` takes new files per group; no rename is needed in part A, so there is **no D-83 code** (the backup uses `create_new`, never `hard_link` or rename).
- **Options considered:** not required below G2. For the carve-out items the options are in the evidence file (F-A02 has three).
- **Escalation check (G2/G3 triggers found; none decided here):**
  - **F-A02 (Major).** A pre-upgrade copy is never overwritten (`chess.db.bak-v<N>` exists -> `BackupFailed{TargetExists}`). After a failed migration, or a restore of a pre-upgrade copy, the retry of the same upgrade is blocked until the user removes the file. The SDD names the file and says "never pruned" but not what a second upgrade from the same version does. Needs a decision (reuse a verified copy, numbered suffix, or replace); touches DAT-6 and a data-directory name (G3 class).
  - **F-A03 (Major, obligation for part B).** The upgrade does not check that no other process is live (guard hook accepts). CT-ST-015 is not covered.
  - **F-A04, F-A06, F-A08 (stored shape).** `RATING_HISTORY.k = 0` for initial points; initial clock checkpoints equal the base time; `store.id` JSON field names.
  - **F-A05, F-A07 (interface and catalogue).** `StoreError::HasUnfinishedGames` needs a `chess-app` catalogue mapping; no store API exists for `CreateFreshStore`.
- **Evidence:** docs/engineering/step2-store-A-evidence.md.
- **Tests added:** 113 tests (unit 19, integration 94) as listed in the evidence file; IDs UT-ST-030 to 036, 038, 039, 042, 043 (partial), 060 (partial), 080, 084, 092, CT-ST-004 and 005 (in-process), 010, 012, 013, 016 (missing half), 017, 018, 019, 021, 080 (store half), plus the property test and the drift guards.
- **Risk and rollback:** working tree only, nothing committed. Rollback is deleting `crates/chess-store/{src,tests,migrations}`, restoring `crates/chess-store/Cargo.toml` and `Cargo.lock` (git), and deleting this record and the evidence file.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance in a fresh invocation (task chess-a0r.3.9) and has not happened. The Engineer's own findings are F-A01 to F-A14 in the evidence file, all OPEN.

| Item | Status |
|---|---|
| Reviewer pass (part A) | NOT DONE (chess-a0r.3.9) |
| System Architect desk review (G1 part) | NOT REQUESTED yet (after the Reviewer pass) |
| Brian Prascher decision on the carve-out items | NOT REQUESTED yet |
| Approved by | nobody |

## Rework record, 2026-11-10 (CR chess-a0r.20, baseline B6, bead chess-a0r.3.6)

- Authority: Brian Prascher's rulings of 2026-11-09 (relayed; D-108 F-A02 REPLACE for v1, D-109 any `sync_all` probe error), architecture Addendum K 11.4.19, store SDD dated notes 3.5.1, 3.9.1, 3.11.8 and CT-ST-090. The gate decisions for D-108 (G3 class) and D-109 (G2) are Brian's rulings; this record does **not** add any approval.
- Changed: `crates/chess-store/src/{backup,migrate,open,error}.rs`, `tests/open.rs`. Details, tests and results: step2-store-A-evidence.md section 12.
- Unchanged and still open (OQ-34, chess-a0r.3.36): F-A04, F-A05, F-A06, F-A07, F-A08. F-A03 stays PENDING for part B.
- Still pending: the kill-before-rename CT-ST-090 case (needs D1/FaultPlan); the Reviewer pass chess-a0r.3.9 of the rework; Brian's confirmation of the D-109 reading (scratch-file create, write, remove errors stay `Unavailable`); OQ-ST-006 revisit (chess-a0r.3.35).
- Status: working tree only, nothing committed. Reviewer pass NOT DONE. Approved by: nobody.

## Rework 2 record, 2026-11-10 (CR chess-a0r.20, baseline B6, bead chess-a0r.3.6)

- Driver: the Reviewer's `review-chess-store-A.md` (PASS WITH CONDITIONS): FIND-001, FIND-002, FIND-003 (Major) and the minors FIND-004 to FIND-009 as ordered. Details, tests, mutants and results: step2-store-A-evidence.md section 13.
- Gate: unchanged, G1 inside `chess-store` (no interface, edge, protocol or stored-format change). F-A04 to F-A08 untouched. This record adds no approval.
- Behaviour changes for the system architect's desk review (section 13.3 of the evidence file): a failed first launch no longer deletes `chess.db*` (an empty `chess.db` is left and counts as an interrupted first creation); `store.id` is written before the schema and atomically (transient `.store.id.tmp-*` name, format unchanged); `first_launch` is reported only by the opener that created the schema; a losing first launch retries on `Busy`. SDD 3.9.1 and DD-ST-005 say "leaves the files as found": a dated note may be needed `[UNV]`.
- Open for later owners: FIND-010 (part B, F-A03, comment in `open.rs`); FIND-011, 012, 014, 015 and the fault-injection mutants (D1).
- Reviewer re-check of this rework: NOT DONE. Approved by: nobody. Working tree only, nothing committed.
