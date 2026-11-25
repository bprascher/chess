# GATE-110: chess-store part D5 (statistics rows, evaluations, variations, analysis job queue)

| Field | Value |
|---|---|
| Gate | **GATE-110** (the number is the one named in the work order; no GATE-109 file existed in this directory when this was written, `[OBS ls]`) |
| Tier | **G2** (corrected 2026-11-23, G0 record fix). The header said "G1 as built" with two items that might raise it to G2: FIND-D5-01 (the store-owned row type) and FIND-D5-02 (the opaque `VariationRef`) are interface decisions, and the Engineer's own text said the higher tier applies when unsure. Approver for G2: **Brian Prascher**. The G1 text that stood here: "G1 as built, with two items for the architect that may raise it to G2" |
| Approver | System Architect, **desk review**, after the Reviewer pass, for G1. If item 1 or 2 is ruled to be an interface or edge decision, the approver is **Brian Prascher** (G2). A desk review never satisfies G2 or G3 |
| Status | **APPROVED WITH CONDITIONS (Brian Prascher, G2, 2026-11-26, relayed; CR-37, bead chess-a0r.3.61).** Conditions 1 and 2 open (bead chess-a0r.3.62); condition 3 closed. Header text above and the sections below are history up to 2026-11-25; see "Approval 2026-11-26". The author is the Engineer stance and did not approve this gate |
| Date opened | 2026-11-11 |
| Opened by | Software engineer (task chess-a0r.3.41, Engineer stance, run-to-gate, CR chess-a0r.20, baseline B6) |
| Covers | `src/api_analysis.rs`, `src/repo/stats.rs`, `src/repo/analysis.rs`, `src/repo/jobs.rs`, `src/store/d5.rs` (new); small additive edits to `src/lib.rs`, `src/repo/mod.rs`, `src/error.rs` (`VariationAnchor`, `NotOwner`), `src/fault.rs` (`D5_FAILPOINTS`), `src/store.rs` (`mod d5;`, one `Inner` field); tests `tests/stats_rows.rs`, `tests/analysis_variations.rs`, `tests/job_queue.rs`, `tests/d5_faults.rs`, `tests/stats_bench.rs`, `tests/d5_support/mod.rs` |

## CHG-110 Implement chess-store part D5

- **Goal and driver:** DD-ST-011, DD-ST-012, DD-ST-013; SDD-chess-store 3.7.2 (Statistics, Analysis, Jobs), 3.11.7, STATE-ST-02; decisions D-44 (rows only, no view), 12 and 13 (CR-9.3, CR-9.4), D-116 to D-124 (relayed 2026-11-11, bead chess-a0r.3.48: rows with a `NULL` `game_id` in the rating history are points, not games); acceptance of chess-a0r.3.41 (store-completion-allocation.md 4.5).
- **Components touched:** `crates/chess-store` only. Root `Cargo.toml`, `deny.toml`, `Cargo.lock`, `ci/`, every SDD and every other crate were not edited by this task.
- **Interfaces touched:** additions only to the public surface of `chess-store`: 15 `Store` methods (`stats_rows`, `head_to_head_rows`, `games_for_history`, `upsert_move_eval`, `get_game_analysis`, `save_variation`, `delete_variation`, `list_variations`, `enqueue_job`, `enqueue_unevaluated_games`, `claim_job`, `job_heartbeat`, `complete_job`, `fail_job`, `enqueue_failed_once_per_start`), the types `StatsGameRow`, `StatsSide`, `MoveEvalRow`, `EvalFlag`, `JobState`, `GameAnalysis`, `VariationRef`, `VariationRecord`, and two `StoreError` variants (`VariationAnchor`, `NotOwner`; the enum is `non_exhaustive`). No existing signature, schema object or stored format changed. The migration list is untouched (no schema change: `variation`, `move_eval`, `analysis_job` exist in schema 1).
- **Dependencies added:** none. No `unsafe`, no `libc`, no `rustix`.
- **Gate answers:** Q1 no (one crate). Q2 **see item 1 and 2**: new provided methods are additive, but two of them carry a type decision the SDD does not make. Q3 no. Q4 no (`node ci/check-crate-edges.mjs`, evidence file). Q5 no. Q6 no (reads on the read pool, writes on the store thread, as before). Q7 no ADR contradicted. Q8 no (no stored-format change). Q9 yes (new behavior of one component). Q10 no. **Result: G1, pending items 1 and 2.**
- **Design summary:** statistics rows are a single parameterised `SELECT` over `GAME` joined to both seats (structural keys only: Player seat, `started_at >= window`, side, `LIMIT`); evaluations are one `INSERT .. ON CONFLICT DO UPDATE` per ply; `save_variation` counts stored `MOVE` rows and refuses `from_ply >=` that count with `VariationAnchor`; the job queue is compare-and-set on `owner` and `state = 'running'` inside `BEGIN IMMEDIATE`, with the claim lease read from `ANALYSIS_LEASE` (10 s) and `now` passed in from the injected clock. Each write ends with a named failpoint before its commit (CT-ST-007).
- **Options considered** (for items 1 and 2, the two points that are not in the SDD):
  1. *Row type of the statistics queries.* (a) Return a store-owned `StatsGameRow` whose `color` is the store's `StatsSide`; `chess-app` maps it to `chess_domain::StatsRow` (**chosen**: keeps the architecture 3.2 edge list). (b) Return `chess_domain::StatsRow` directly: impossible today because `StatsRow.color` is a `chess_rules::Side` and `chess-store` has no edge to `chess-rules`; it would need a new edge `chess-store -> chess-rules` (G2, `ci/check-crate-edges.mjs`, `deny.toml`). (c) Change `StatsRow.color` in `chess-domain` to a domain-owned side (G2, baselined SDD).
  2. *Identity of a saved variation for `delete_variation`.* Schema 1 gives `variation` an integer key and no uuid; D-40 forbids integer row keys across the API. (a) An opaque `VariationRef` that wraps the key with no public constructor or accessor (**chosen**). (b) Delete by natural key `(game, from_ply, pgn)`: no handle, but identical lines are deleted together. (c) Add a `uuid` column to `variation` (G3: schema and stored format; migration; not taken).
- **Escalation check (items for the architect, not decided here):**
  1. FIND-D5-01 (above, option 1). The App SDD and the Domain SDD say the store returns "rows" and say nothing of their type; `StatsRow` is what the Domain SDD names. Needs a ruling: (a) is accepted, or (b) or (c) by Brian.
  2. FIND-D5-02 (above, option 2).
  3. Three failpoint names added to the registry (`eval.before_commit`, `variation.before_commit`, `job.before_commit`), not in the SDD section 4 list: a dated SDD note is requested (FIND-D5-05).
- **Evidence:** docs/engineering/step2-store-D5-evidence.md.
- **Tests added:** UT-ST-037, UT-ST-060 (stats half), CT-ST-030, CT-ST-031, CT-ST-032, CT-ST-007 (D5 commands), BM-ST-03; names in the evidence file.
- **Risk and rollback:** working tree only, nothing committed. Another engineer (task D2) edits the same crate at the same time; shared files were re-read before each edit and the edits are small and localized (list under "Covers"). Rollback: delete the new files listed under "Covers", remove `mod d5;` and the `failed_jobs_requeued` field from `src/store.rs`, the `api_analysis` lines from `src/lib.rs`, the three `mod` lines from `src/repo/mod.rs`, the two variants from `src/error.rs`, `D5_FAILPOINTS` and its `.chain` line from `src/fault.rs`; delete this record and the evidence file.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance in a fresh invocation and has not happened. The Engineer's own findings are FIND-D5-01 to FIND-D5-0n in the evidence file, all OPEN.

| Item | Status |
|---|---|
| Reviewer pass (part D5) | **Done (superseded 2026-11-25; was "NOT DONE").** See "Record update 2026-11-25" |
| System Architect desk review (G1) | NOT REQUESTED yet (after the Reviewer pass; items 1 and 2 first) |
| Approved by | nobody |


## Rework 2026-11-22 (decision bead chess-a0r.3.50)

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G2).** The Engineer does not approve its own gate.
- **Changed:** FIND-D5-02 B (variation identity `(game, from_ply, pgn)`, idempotent `save_variation`, `delete_variation(game, from_ply, pgn)`, `VariationRef` and `VariationRecord.id` removed): a public interface change of `chess-store` (Q2 yes, **G2**), no schema change, no consumer built yet. FIND-D5-06 (canonical paths in `fault.rs`, test-support only, G1 inside the crate). FIND-D5-01 A: no change. Evidence: `step2-store-D5-evidence.md`, section "Rework 2026-11-22".
- Rollback: revert `src/api_analysis.rs`, `src/repo/analysis.rs`, `src/store/d5.rs`, `src/lib.rs`, `src/fault.rs` and the tests named in the evidence; no data is affected.
- Reviewer pass of the rework: NOT DONE. Approved by: nobody.


## Fixes 2026-11-22 after the Reviewer pass (`review-chess-store-D1-D2-D5.md`)

- **Status after these fixes: still OPEN, NOT APPROVED. PENDING.** No approval has been received; no second Reviewer pass has happened.
- **Fixed (evidence: `step2-store-D5-evidence.md`, section "Fixes 2026-11-22"):** FIND-D5R-05 (`complete_job` and `fail_job` answer `Ok` to the same owner's retry), FIND-D5R-01 (`save_variation` refuses a text above 1 MiB, ESTIMATE, with the new `StoreError::VariationTooLarge`).
- **NOT touched, decisions pending:** FIND-D5R-02 (index on `variation(game_id)` is a stored-format change, G3, Brian), FIND-D5R-03 (ordering key), FIND-D5R-04 (UT-ST-038 scan files).
- **Gate answers:** two additive public items (`VariationTooLarge`, `MAX_VARIATION_PGN_BYTES`) and one behaviour change of two job commands: Q2 yes, inside `chess-store`; the app maps the new error when it is built. Tier unchanged.
- Approved by: nobody.


## Rulings 2026-11-23

- **Tier corrected to G2 in the header (G0 record fix, 2026-11-23).** Status: still OPEN, NOT APPROVED. PENDING Brian Prascher. No approval has been received.
- **Built (details: `step2-store-D5-evidence.md`, section "Rulings 2026-11-23"):** FIND-D5R-03 order by `(started_at, id)`, `limit` is the oldest N (G1, no signature change). FIND-D5R-02: no index (the ruling), DDL unchanged.
- **Rollback:** revert the four `ORDER BY` clauses (`src/repo/stats.rs`, `src/repo/game.rs`); tested: no.
- **Reviewer pass of the rulings:** NOT DONE. Approved by: nobody.

## Rulings 2026-11-24

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G3).** No approval has been received (decision bead chess-a0r.3.56 records rulings, not a gate approval).
- **Built (details `step2-store-D5-evidence.md`, section "Rulings 2026-11-24"):** F2-01 imported games excluded in the statistics SQL (G1); F2-02 NULL `started_at` last in `list_games` and `games_for_history` (G1, Brian informed: it amends the ruled `(started_at, id)` order); F2-03 tie-break tests added, mutants S02 and S05 killed.
- **Gate answers added:** none. No signature or schema change.
- **Rollback:** revert the SQL text in `stats.rs` and `game.rs`; tested: no.
- **Reviewer pass of the 2026-11-24 changes:** NOT DONE. Approved by: nobody.

## Record update 2026-11-25 (G0 record keeping, systems engineer; baseline B6)

- **Tier: G2** (corrected 2026-11-23, see the header). **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G2).** The "Rulings 2026-11-24" section above says "(G3)", which does not match the header; the tier is G2 as the header says `[INF: the 2026-11-24 section's "(G3)" is inconsistent with the corrected header; the header governs until Brian or the engineer states otherwise]`. GATE-108 was approved with conditions on 2026-11-25 (CR-35); GATE-110 is presented one gate at a time after GATE-109. No approval of GATE-110 has been received.
- **Reviewer reports (replace the stale "NOT DONE" lines above):** `docs/engineering/review-chess-store-D1-D2-D5.md` (PASS WITH CONDITIONS, all Minor); `review-chess-store-fixes-2.md` (PASS WITH CONDITIONS, no Major; Minor F2-01, F2-02, F2-03); `review-chess-store-D3-rulings.md` (PASS WITH CONDITIONS stands for `repo/stats.rs` and the `ORDER BY` in `repo/game.rs`; 7 of 7 mutants killed; condition FIND-D3RL-06, the tie-break test works only because a planted index steers the planner).
- **Decisions that touch this gate:** **D-150** (imported games excluded from statistics; NULL `started_at` sorts last; G1, CR-34 items F2-01 and F2-02), **D-140** (order `(started_at, id)`, oldest N, no `variation(game_id)` index now), and FIND-D5-02 B (CR-26). GATE-111 edits D5 files (`stats.rs`, `game.rs`).
- The Reviewer verdicts are the agents' own and are not approvals. G2 and G3 decisions stay with Brian Prascher; the systems engineer approved nothing in this edit.

## Approval 2026-11-26 (G0 record keeping, systems engineer; baseline B6)

- **Status: APPROVED WITH CONDITIONS by Brian Prascher (G2), 2026-11-26**, in a genuine user message relayed by the coordinator ("Brian, relayed"; decision bead chess-a0r.3.61; Decision Log CR-37). Brian committed and merged to `main` locally; not yet pushed. Earlier "OPEN. NOT APPROVED" lines in this record are history. The systems engineer approved nothing; this edit records Brian's decision.
- **Conditions and state:**

| # | Condition | State | Owner and evidence |
|---|---|---|---|
| 1 | The F2-03 tie-break (`id` for equal `started_at`) is pinned by a test that does not depend on a planted index, or accepted as is | **OPEN** | Bead chess-a0r.3.62 (software engineer). Reviewer finding FIND-D3RL-06: with the index helper removed the `stats_rows` binary passes with `, g.id` deleted from `stats.rs` and `game.rs` `[OBS review-chess-store-D3-rulings.md]`. Brian may instead accept as is; no such acceptance is recorded |
| 2 | The status of the UT-ST-038 integer-identity scanner (FIND-D5R-04) over `api_analysis.rs` and `store/d5.rs` is stated | **OPEN** (status is stated here, the engineer's confirmation or extension is due in chess-a0r.3.62) | `[OBS crates/chess-store/tests/boundary.rs line 66]` the scanner reads `api.rs`, `store.rs`, `error.rs` and `datadir.rs` only: it does **not** cover `api_analysis.rs` or `store/d5.rs`. `[OBS grep]` those two files declare no public `i64` identity; their public integers are `u32` and `u64` counts and plies (`plies`, `active_ms`, `ply`, `depth`, `evaluated`, `total`, `from_ply`) and `u32` job counts. `[UNV]` not checked by running the scanner over them |
| 3 | GATE-111's edits to D5 files are listed in the GATE-111 record | **CLOSED** by this edit | GATE-111 record, section "D5 files edited by GATE-111 (GATE-110 condition 3)" |

- Rollback, scope and the "Covers" list are unchanged. The Reviewer verdicts are the agents' own and are not approvals.

## GATE-110 conditions 2026-11-26

- **Status: still OPEN, NOT APPROVED. PENDING Brian Prascher (G2).** Brian approved with conditions (relayed by the coordinator, no verbatim quote); the engineer approves nothing.
- **Condition 1 (F2-03 tie-break without a planted index):** built `crates/chess-store/tests/tie_break.rs`. Mutants S02 and S05 are killed **by a source-text pin only**; the `reverse_unordered_selects` route did not work (survivors with the pragma on; hook removed, no `src` change). Seven further mutants are killed by behavioural tests with no index. Details: `step2-store-D5-evidence.md`, section "GATE-110 conditions 2026-11-26".
- **Condition 2 (D5R-04):** the UT-ST-038 scanner did not cover `api_analysis.rs` and `store/d5.rs`; it now covers eight files including them and `store/d3.rs`, `store/d4.rs`, with a planted-leak self-test.
- **Gate answers added:** none. No signature, schema or `src` change. Rollback: delete `tests/tie_break.rs`, revert `tests/boundary.rs`; tested: no.
- **Reviewer pass of these test changes:** NOT DONE. Approved by: nobody.
