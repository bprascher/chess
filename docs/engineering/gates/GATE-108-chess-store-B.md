# GATE-108: chess-store part B (per-game journal, leases, process registry, F-A03)

| Field | Value |
|---|---|
| Gate | **GATE-108** (follows GATE-107, part D1) |
| Tier | **G3**: the journal file format is a **stored format** (SWE-001 section 3: "a stored-format change (journal, archive `format_version`, schema)"; SDD 3.11.1, architecture 7.6, SF-07 to SF-16, frozen at the first release by D-41). The change also adds public items to `chess-store` (`journal`, `StoreOptions::process`, the registry and lease methods, two error types) and changes the open path (F-A03), which is G2 on its own (Q2, Q5 of `swe-governance`). The highest tier any question gives applies |
| Approver | **Brian Prascher** (G2 and G3 per SWE-001 section 3). A desk review of the System Architect never satisfies G2 or G3 |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-11 |
| Opened by | Software engineer (task chess-a0r.3.7, Engineer stance, run-to-gate, CR chess-a0r.20, baseline B6) |
| Covers | `crates/chess-store` only (and the two new documents named below): new `src/journal/{mod,format,reader,reconcile,rebase,files,writer}.rs`, `src/repo/coord.rs`, `src/proc.rs`; changed `src/{open,migrate,backup,datadir,store,api,error,fault,lib}.rs`, `Cargo.toml` (one dependency, `crc32fast`, inherited from the workspace); tests `tests/{journal,journal_faults,journal_crash,coordination,hardening}.rs`, `tests/common/{mod,child}.rs`, and edits to `tests/{crash,open,fault_injection}.rs` for the new temp name |

## CHG-108 Implement chess-store part B

- **Goal and driver:** acceptance of chess-a0r.3.7 (journal records per architecture 7.6; `content_hash` fields; 2 s, 10 s and 10 s heartbeat values; stale lease takeover; two-process tests; stored format exactly as approved) plus F-A03 (guard wired to the registry, `Exclusive`, `OtherVersionRunning`, the PENDING marker gone, CT-ST-013 other-version half, CT-ST-014, CT-ST-015 with two real processes), FIND-010 (guard before backup), close-on-exec (CT-ST-006) and the scope additions of 2026-11-11 (NF-1, NF-2, NF-3, D1-01, NF-7, the `journal.*` failpoints and the kill tests). Scope per `store-completion-allocation.md` 5.2.
- **Components touched:** `crates/chess-store` only. Root `Cargo.toml`, `deny.toml`, `ci/` and every SDD are not edited.
- **Interfaces touched (all inside the `chess-store` public surface, none yet used by another crate):** `chess_store::journal::*` (new), `StoreOptions::process` and `with_process`, `Store::{journal, owner, process, register_process, process_heartbeat, deregister_process, list_live_processes, lease_upsert, lease_delete, lease_beat, lease_owner}`, `StoreError::{Exclusive, OtherVersionRunning}`, `JournalError`, `WriteCause`, `ExclusiveReason`, `LiveProcess`, `LeaseInfo`, `LeaseBeat`, `ProcessIdentity`; `fault` registry gains 5 names and the `!` (fire once) suffix of `FaultPlan::parse`.
- **Stored formats touched:** the journal file format (header and record frame, kind codes, payload field names, text forms) is **implemented exactly as written in SDD 3.11.1 and architecture 7.6**; no field was invented. Where the text is ambiguous the most conservative reading is a numbered choice (CH-1 to CH-14 in `step2-store-B-evidence.md` section 8), listed for Brian. The `process_heartbeat` and `game_lease` tables are used as the schema 0001 defines them; **no schema change**. New names in the data directory: `.jnl.rebase`, `damaged/<uuid>.<k>.jnl` (both already rated G2 by D-83), the per-process backup temp name `chess.db.tmp-bak-v<N>-<pid>` (NF-1, new, see CH-12).
- **Dependencies added:** `crc32fast` (admitted by DL2, pinned `=1.5.3` in the root `[workspace.dependencies]`, inherited with `workspace = true`). No `libc`, no `rustix`, no `unsafe` (`#![forbid(unsafe_code)]` kept). The pid-existence check runs the system `/bin/kill -0` (CH-9; an SDD conflict is reported, see FIND-B-01).
- **Gate answers:** Q1 no (one crate). Q2 yes (public items, error behaviour of `open`: `Exclusive`, `OtherVersionRunning`) -> G2. Q3 no. Q4 no (`node ci/check-crate-edges.mjs` OK). Q5 no (data ownership unchanged; journal and registry were allocated to this crate). Q6 no. Q7 no known ADR contradicted (SDD conflicts are listed in the evidence file). Q8 yes (stored format, irreversible once released) -> G3. **Result: G3.**
- **Design summary:** see `step2-store-B-evidence.md` section 1.
- **Options considered (G2/G3 requirement):** (1) journal payloads as typed structs with strict decode (chosen) versus an untyped `serde_json::Value` payload (rejected: it would accept any field and make UT-ST-070 untestable); (2) pid check by `/bin/kill -0` (chosen, std only) versus `libc::kill` (rejected: D-107 admits no `libc`/`unsafe`) versus heartbeat age alone (rejected as the only test: a killed process would stay live for up to 30 s, contradicting "dead at once"); (3) the process heartbeat timer owned by `chess-app` calling `process_heartbeat` (chosen) versus a timer task inside `Store` (rejected: `Store` has no runtime of its own and the SDD names the method).
- **Consumer impact:** none yet: `chess-app`, `chess-cli`, `chess-server` are empty scaffolds. `chess-app` must call `process_heartbeat` every `PROCESS_HEARTBEAT_EVERY` and `lease_beat` every `LEASE_BEAT_EVERY`, and pass `StoreOptions::with_process`.
- **Migration and rollback plan:** nothing is released and no store exists outside tests, so there is no data migration. Rollback: delete the files listed under "Covers" as new, revert the changed ones (the changed lines are named in the evidence file), delete this record and the evidence file. The `chess.db.tmp-bak-v<N>-<pid>` rename has no persistent effect.
- **Spike for the riskiest assumption (G3):** the riskiest assumptions are (a) a hard-link move is no-replace and leaves the crash windows of D-110 (spike OQ-14 and the six kill tests, `[RAN]`), and (b) the pid check works without `libc` (`[RAN]` with real children, see evidence section 3). Power-loss behaviour of truncate, rename and directory sync is **not run** (`[UNV]`).
- **Rollback tested:** no (nothing to roll back at runtime; a git revert is the rollback).
- **Independent reviewer:** the independent-reviewer step is waived for v1 by Brian (FIND-016, SWE-001 section 3). The Reviewer pass (a fresh invocation) has **not** happened.
- **Evidence:** docs/engineering/step2-store-B-evidence.md.
- **Tests added or changed:** see the evidence file section 4.
- **Reviewer findings and dispositions:** none yet. The Engineer's own findings are FIND-B-01 to FIND-B-nn in the evidence file, all OPEN.

| Item | Status |
|---|---|
| Reviewer pass (part B) | NOT DONE |
| G3 review step | not requested yet |
| Approved by | nobody |


## Rework 2026-11-22 (decision bead chess-a0r.3.50)

- **Status after this rework: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** The Engineer does not approve its own gate. No approval has been received.
- **What was added** (details and evidence: `step2-store-B-evidence.md`, section "Rework 2026-11-22"): CH-4 (no `u64` above `i64::MAX`, encode and decode, including `seq`), CH-5 B (`DamageKind::UndecodableTail`, a CRC-valid refused frame is damage), CH-11 B (no closing `Clock` after a terminal record), FIND-B-11 B (tolerant decode, verbatim rebase, `journal_version` stays 1), FIND-B-06 narrowed (rollback decided on a private copy; no players and no games), NF-4 (an empty or schemaless `chess.db` beside data traces is refused with `Missing`; `create_fresh` not built).
- **Gate answers added:** stored format changed in content rules (CH-4, CH-5, CH-11, B-11): Q8 yes, **G3**. The open path (B-06, NF-4): observable behaviour of `Store::open`, Q2 yes, **G2**, and G3 where it decides what may be written to `chess.db` (B-06). New public items: `DamageKind::UndecodableTail`, the failpoint name `open.nf2_copy`. No new dependency (`cargo-deny` part of `ci/check.sh` passed), no new component, no edge change (`ci/check-crate-edges.mjs` OK).
- **Consumer impact:** none built yet. D4 must handle `UndecodableTail` like `Middle`; D3 and `chess-app` must map `Missing` for the NF-4 case (reason is logged only).
- **Rollback:** nothing is released; revert the hunks in `src/journal/{format,reader,rebase,reconcile}.rs`, `src/open.rs`, `src/datadir.rs`, `src/fault.rs` and delete `tests/nf2_narrowed.rs`, `tests/nf4_refusal.rs`; tested: no.
- **Reviewer pass of the rework:** NOT DONE (a fresh Reviewer invocation is needed; the spec asks the reviewer to re-check the real v1 store case NF2-2).
- **Open findings from the rework:** FIND-ST-RW-01 to FIND-ST-RW-10 in the evidence file. SDD-conflicts: listed there; the architect edits the SDDs.
- Approved by: nobody.


## Fixes 2026-11-22 after the Reviewer pass (`review-chess-store-rework-1.md`, verdict FAIL for part 1)

- **Status after these fixes: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** No approval has been received; the Engineer does not approve its own gate. The Reviewer's FAIL is not lifted by the Engineer: a second fresh Reviewer pass is needed.
- **Fixed (details, tests and mutants: `step2-store-B-evidence.md`, section "Fixes 2026-11-22"):** FIND-RW1-02 (Major; a live creator's journal is no longer refused on a stale first scan: bounded re-scan), FIND-RW1-03 and FIND-ST-RW-04 (rebase is idempotent), FIND-RW1-04 (non-objects refused), FIND-RW1-05 (`ClockText` is the domain's `ClockSpec` check), FIND-RW1-06 (bounded copy, cap boundary, scratch mode), FIND-RW1-08 (stale NF-2 scratch swept at the next open), FIND-RW1-10 (`build_rebased` checks its two views of the intact run), FIND-RW1-12 (tests for M36, M40, M44, M51, M60; M41 is equivalent now).
- **NOT touched, decisions pending:** FIND-RW1-01 (Major: what counts as a trace beside a schemaless database; a killed first launch can still answer `Missing`). **GATE-108 cannot be approved on the Reviewer's terms while FIND-RW1-01 is open.** Also left: FIND-RW1-07, FIND-RW1-09, FIND-RW1-11.
- **Gate answers added by the fixes:** public surface `RebaseError::InconsistentIntact` (Q2, one crate, additive); behaviour of `Store::open` (a lone non-empty `-journal` is refused after up to 1 s instead of at once) and of `ClockText::parse` (`0+0`, `0+10`, values above the domain limits are refused): Q2 yes. No new dependency, edge or store; `ci/check-crate-edges.mjs` OK. Tier unchanged (G3 for this gate).
- **Rollback:** revert the hunks named in the evidence file; tested: no.
- Approved by: nobody.


## Rulings 2026-11-23

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3; the open-path part G2).** The Engineer does not approve its own gate. Brian's 2026-11-23 rulings (decision bead chess-a0r.3.52) are built: FIND-RW1-01 two-class trace rule (G2, amends D-126), `WAL_SETTLE` and the creator-aware wait removed, rebase idempotence kept, CH-5 as built, payload and clock text as built. Details, tests and mutants: `step2-store-B-evidence.md`, section "Rulings 2026-11-23".
- **Gate answers added:** Q2 yes (observable behaviour of `Store::open`: states 1 and 2 of the killed creation now complete; a lone `-wal` or `-journal` beside a non-zero schemaless database or a zero-length database is no longer a trace, except zero-length plus non-empty `-wal`). G2 by Brian's own ruling; G3 where it decides what may be written to `chess.db` (B-06, unchanged).
- **Residual risk accepted by the ruling, recorded:** R-NF4-1 widens by one case (a lost store whose only leftover is a WAL beside a truncated database opens as new). New R-NF4-4: an invalid leftover WAL beside a schemaless non-zero database is reset by the first write.
- **Rollback:** revert the hunks in `src/datadir.rs` (`DataTraces`, `data_traces`, `is_interrupted_first_creation`) and `src/open.rs` (`nf4_verdict`, `WAL_SETTLE`, seams) and the tests named in the evidence; tested: no.
- **Reviewer pass of the rulings:** NOT DONE.
- Approved by: nobody.

## Rulings 2026-11-24

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** No approval has been received (decision bead chess-a0r.3.56 records rulings, not a gate approval).
- **Built (details `step2-store-B-evidence.md`, section "Rulings 2026-11-24"):** the journal format refuses `ended_at <= 0` (`GameEnd`, `Abort`, `Voided`) and `Start.wall_ms <= 0` at encode and decode; `Start.matchup` is `hh|hc|cc` of the kinds (G3, stored-format rules, one-way at release; `journal_version` stays 1); F2-07 kill rounds default 500.
- **Gate answers added:** Q8 yes (stored format). No new component, edge or dependency.
- **Rollback:** revert `Payload::validate`, `check_record`, `matchup_code` in `journal/format.rs` and the test fixtures; tested: no.
- **Reviewer pass of the 2026-11-24 changes:** NOT DONE. Approved by: nobody.
