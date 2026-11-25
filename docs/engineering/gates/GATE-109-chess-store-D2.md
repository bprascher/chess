# GATE-109: chess-store part D2 (game lifecycle writes and ratings)

| Field | Value |
|---|---|
| Gate | **GATE-109** (follows GATE-108, part B) |
| Tier | **G3.** New public items of `chess-store` (Q2: `finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player`, `check_rating_invariant`, their DTOs, two error shapes, a changed error variant) make it G2 on their own. The change also writes **stored shapes and values** that are irreversible once released (Q8; D-116 rated stored shapes G3): `GAME.termination` texts `aborted`, `voided by reset`, `voided: unreadable`; the reset point rows in `RATING_HISTORY` (`game_id` NULL, `k` 0); the Elo columns of the terminal row. The highest tier any question gives applies |
| Approver | **Brian Prascher** (G2 and G3 per SWE-001 section 3). A desk review of the System Architect never satisfies G2 or G3 |
| Status | **APPROVED WITH CONDITIONS (Brian Prascher, 2026-11-25).** Conditions and their state: see "Approval 2026-11-25" at the end of this record. Header and sections above are history as of their dates (they say OPEN, NOT APPROVED). Recorded as **CR-36** |
| Date opened | 2026-11-11 |
| Opened by | Software engineer (task chess-a0r.3.38, Engineer stance, run-to-gate, CR chess-a0r.20, baseline B6) |
| Covers | `crates/chess-store` only, plus two documents: new `src/repo/{finalize,lifecycle,invariant}.rs`, `tests/{lifecycle,lifecycle_faults}.rs`; additive edits to `src/{api,error,store,lib,fault}.rs`, `src/repo/{mod,player}.rs`; test edits `tests/{boundary,model,players}.rs` for the `HasUnfinishedGames` shape; `docs/engineering/step2-store-D2-evidence.md` |

## CHG-109 Implement chess-store part D2

- **Goal and driver:** acceptance of chess-a0r.3.38: DD-ST-009 (rest) and DD-ST-010 (write path), SDD 3.11.4: `finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player`, `check_rating_invariant`, the shared void function (D-78 (5)), F-A04 option A (D-116), F-A05 option A (D-117).
- **Components touched:** `crates/chess-store` only. Root `Cargo.toml`, `deny.toml`, `ci/`, every SDD and `chess-domain` are not edited.
- **Interfaces touched (public surface of `chess-store`, no consumer yet):** new `Store::{finalize_game, abort_game, void_unreadable_game, reset_player, check_rating_invariant}`; new `FinalizeRequest`, `FinalizeOutcome`, `RatingChange`, `AbortRequest`, `EndOutcome`, `VoidSpec`, `RatingViolation`, `RatingViolationKind`; new `StoreError::UnlistedUnfinishedGames{ids}`; **changed** `StoreError::HasUnfinishedGames` from a unit variant to `{ids: Vec<GameId>}` (F-A05 option A, decided by Brian 2026-11-11, relayed). Four failpoints registered: `abort.before_commit`, `void.before_commit`, `reset.before_commit` (new) and `finalize.before_commit` (already in the SDD list).
- **Stored formats touched:** no DDL change. Values written: see Tier.
- **Dependencies added:** none.
- **Gate answers:** Q1 no. Q2 yes -> G2. Q3 no. Q4 no (`node ci/check-crate-edges.mjs`, see evidence). Q5 no. Q6 no. Q7 no known ADR contradicted (SDD conflicts are listed in the evidence file). Q8 yes -> G3. **Result: G3.**
- **Options considered:** (1) one `FinalizeOutcome` for all three endings versus a small `EndOutcome` for abort and void (chosen: `Finalized` carries rating changes that abort and void do not have); (2) invariant check returns `Result<(), StoreError>` with a catalogue error versus a list of violations (chosen: the list; FIND-ST-09 says no catalogue code for a failed invariant, and the caller needs to know which Player and track); (3) failpoint context is the data directory's database path passed in by `Store` versus `Connection::path()` (chosen: the first; the second is a canonical path that a `FaultPlan` root does not match on macOS, found by test).
- **Consumer impact:** `chess-app` (part A, task chess-a0r.3.12, blocked on this) must pass the hash and the three time totals, read `ended_at` back with `get_game` after `reset_player`, map `UnlistedUnfinishedGames` and `HasUnfinishedGames` to the protocol codes, and call `check_rating_invariant` where the SDD says.
- **Migration and rollback plan:** nothing is released and no store exists outside tests. Rollback: delete the three new repo files and two test files, revert the additive hunks in the files named in "Covers", delete this record and the evidence file.
- **Spike for the riskiest assumption (G3):** the riskiest assumption is that one transaction applies the settlement, the terminal row and the slot, lease and job work atomically. `[RAN]` by the fault tests (a fault just before the commit changes no row of any table; the retry succeeds). Power-loss behaviour is not tested (`[UNV]`).
- **Rollback tested:** no (nothing at runtime to roll back; a revert is the rollback).
- **Independent reviewer:** waived for v1 by Brian (FIND-016, SWE-001 section 3). The Reviewer pass has **not** happened.
- **Evidence:** docs/engineering/step2-store-D2-evidence.md.
- **Reviewer findings and dispositions:** none yet. The Engineer's own findings are FIND-D2-01 to FIND-D2-nn in the evidence file, all OPEN.

| Item | Status |
|---|---|
| Reviewer pass (part D2) | **Done (superseded 2026-11-25; was "NOT DONE").** See "Record update 2026-11-25" |
| G3 review step | not requested yet |
| Approved by | nobody |


## Rework 2026-11-22 (decision bead chess-a0r.3.50)

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher.** The Engineer does not approve its own gate.
- Brian's rulings (FIND-D2-04 accepted, FIND-D2-02 accepted, names `game` and the three failpoints) need **no code change**. One test was added, `d2_02_the_store_orders_rating_rows_by_id_and_the_domain_by_at_then_id`, with hand mutants killed. Evidence: `step2-store-D2-evidence.md`, section "Rework 2026-11-22". The stored abort text `aborted` is a G3 value: its approval stays with Brian.
- Reviewer pass of the rework: NOT DONE. Approved by: nobody.


## Fixes 2026-11-22 after the Reviewer pass (`review-chess-store-D1-D2-D5.md`)

- **Status after these fixes: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** The Reviewer's condition "GATE-109 should not be approved until FIND-D2R-01 is fixed and a concurrency test exists" is met by the Engineer's work (test red before, green after: 32 false violations in 11 053 checks became 0); that is the Engineer's claim, not an approval, and a second Reviewer pass has not happened.
- **Fixed (evidence: `step2-store-D2-evidence.md`, section "Fixes 2026-11-22"):** FIND-D2R-01 (Major, one read transaction), FIND-D2R-05 (`void_game` refuses a completed row with settled ratings; `void_game_repair_pair` is the named bypass for D3), FIND-D2R-04 (M14, M25, N01 tests).
- **NOT touched, decisions pending:** FIND-D2R-02 (`analysis_used` overwrite or OR), FIND-D2R-03 (terminal-state validation), FIND-D2R-06 (invariant scope).
- **For D3:** the acceptance list gets "a `Voided` record on a completed row with settled ratings is refused or goes through `void_game_repair_pair` on purpose".
- No new dependency, edge or store; no stored-format change. Approved by: nobody.


## Rulings 2026-11-23

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** No approval has been received.
- **Built (details: `step2-store-D2-evidence.md`, section "Rulings 2026-11-23"):** FIND-D2R-02 `analysis_used` is a latch (`max(stored, request)`; stored-value meaning, G3) and FIND-D2R-03 terminal data is checked for shape only (`ended_at > 0`, non-empty `termination`; existing `InvariantViolated`; G1). Nothing else is checked: no time order, no journal sequence, no abort-who rule.
- **Gate answers added:** Q8 yes for the latch (meaning of a stored value). No signature, edge or schema change.
- **Rollback:** revert the `max(...)` and the two shape checks in `src/repo/finalize.rs`; tested: no.
- **Reviewer pass of the rulings:** NOT DONE. Approved by: nobody.

## Rulings 2026-11-24

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** No approval has been received (decision bead chess-a0r.3.56 records rulings, not a gate approval).
- **Built (details `step2-store-D2-evidence.md`, section "Rulings 2026-11-24"):** `void_game` refuses `ended_at <= 0`; `void_game_repair_pair` deleted (G1); `set_analysis_used` refuses a final game unless the flag is set (G1); D3-09 flush before the live terminal commands.
- **Gate answers added:** none beyond G3 for the stored-value rule `ended_at > 0`.
- **Rollback:** revert the three hunks (`lifecycle.rs`, `game.rs`, `store.rs`); tested: no.
- **Reviewer pass of the 2026-11-24 changes:** NOT DONE. Approved by: nobody.

## Record update 2026-11-25 (G0 record keeping, systems engineer; baseline B6)

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** GATE-108 was approved with conditions on 2026-11-25 (CR-35). GATE-109 is presented to Brian after it, one gate at a time (relay, CR-34). No approval of GATE-109 has been received.
- **Reviewer reports (replace the stale "NOT DONE" lines above):** `docs/engineering/review-chess-store-D1-D2-D5.md` (PASS WITH CONDITIONS with one Major, FIND-D2R-01, concurrent `check_rating_invariant`); `review-chess-store-fixes-2.md` (PASS WITH CONDITIONS, no Major: FIND-D2R-01 fixed; Minor F2-04, F2-05, F2-09); `review-chess-store-D3-rulings.md` (PASS WITH CONDITIONS stands for the D2 files the D3 rulings changed: `void_game` and the removal of `void_game_repair_pair`, `set_analysis_used`; no new finding).
- **Decisions that touch this gate:** **D-149** (`void_game_repair_pair` deleted, G1), **D-144** (the `> 0` check also applies to `void_game`, `void_unreadable_game` and the `reset_player` commit time; approved under GATE-111), **D-152** (the store drains the game's queue before a live finalize, abort, void or reset; GATE-111), **D-138** (`analysis_used` latch) and **D-139** (terminal data shape only), earlier. GATE-111 edits D2 files (`finalize_in_tx` and `abort_in_tx`); the D2 suite must stay green there.
- **Open:** F2-05 (`set_analysis_used` can flag a finished game): the D3-rulings Reviewer says it "behaves as ruled", and the relay lists no ruling for it `[UNV]`; Brian's disposition is not recorded here.
- The Reviewer verdicts are the agents' own and are not approvals. G2 and G3 decisions stay with Brian Prascher; the systems engineer approved nothing in this edit.

## Approval 2026-11-25 (G0 record keeping, systems engineer; baseline B6)

- **Status: APPROVED WITH CONDITIONS (Brian Prascher, 2026-11-25).** Brian approved GATE-109 (G3; public items G2) in a genuine user message relayed by the coordinator (Brian, relayed; no verbatim quotation). Decision bead chess-a0r.3.59 (closed); Decision Log entry **CR-36**. Brian committed, merged to `main` and pushed himself. The Reviewer verdicts above (PASS WITH CONDITIONS, no Major) are the agents' own and were not the approval. The systems engineer approved nothing and edited records only.
- **Conditions and their state at 2026-11-25:**

| # | Condition | State | Owner and evidence |
|---|---|---|---|
| 1 | GATE-111 re-lists its edits to D2 files and the D2 tests stay green | **Pending until GATE-111 is decided.** The list is entered in `GATE-111-chess-store-D3.md` (section "D2 files edited by GATE-111", 2026-11-25). D2 suite green: 130 test binaries pass in the PM full gate of 2026-11-25 (check-12) (a PM figure, not re-run by the systems engineer) | GATE-111 record; the D2 tests must stay green at every later change |
| 2 | F2-05 accepted as built: `set_analysis_used` refuses a game that is not `in_progress` unless the flag is already set | **Closed** (accepted). Code read: `crates/chess-store/src/repo/game.rs`, `set_analysis_used`: for a status other than `in_progress` it returns `Ok(())` if the flag is already set, else `InvariantViolated` `[OBS]` | Brian |
| 3 | The three unwired failpoints `finalize.after_journal_end`, `finalize.after_commit_before_rename`, `reset.after_commit_before_voided_record` are wired before the first release | **Open.** Bead chess-a0r.3.60 (due before the first release, not before gate approval; also covers `backup.after_copy`, `restore.*`, `apply.*` per D-143). The names are registered in `src/fault.rs` and listed in `tests/fault_injection.rs`; no production call site exists `[OBS grep 2026-11-25]` | Store D2 follow-up (engineer); part C for `backup.after_copy` and `restore.*` |

- **Not verified:** the relay carries no verbatim text; the approval covers the code as it stood at the 2026-11-25 merge (`0b0a47d`); a fresh Reviewer pass of D2 after GATE-111 and D4 edits has not run; power loss is not tested.
