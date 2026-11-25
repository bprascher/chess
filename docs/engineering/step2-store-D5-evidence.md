# Step 2, chess-store part D5: statistics rows, evaluations, variations, analysis job queue (evidence)

| Field | Value |
|---|---|
| Task | chess-a0r.3.41 (baseline B6, CR chess-a0r.20, run-to-gate), Engineer stance, 2026-11-11 |
| Gate | [GATE-110](gates/GATE-110-chess-store-D5.md): G1 as built, two items for the architect that may make it G2. **OPEN, NOT APPROVED** |
| Scope | DD-ST-011, 012, 013; SDD-chess-store 3.7.2 (Statistics, Analysis, Jobs), 3.11.7; store-completion-allocation.md 4.5 |
| Not touched | root `Cargo.toml`, `deny.toml`, every SDD, every crate except `crates/chess-store` |

Evidence tags: `[OBS]` read in this session, `[RAN]` executed, `[INF]` inferred, `[UNV]` not verified.

## 1. What was built `[OBS]`

| File | Content |
|---|---|
| `crates/chess-store/src/api_analysis.rs` (new) | DTOs: `StatsGameRow`, `StatsSide`, `MoveEvalRow`, `EvalFlag`, `JobState`, `GameAnalysis`, `VariationRef`, `VariationRecord` |
| `crates/chess-store/src/repo/stats.rs` (new) | `stats_rows`, `head_to_head_rows`: one parameterised `SELECT` over `GAME` joined to both seats. Structural keys only: seat of the Player, `started_at >= window`, side, `LIMIT`. No predicate on `self_play`, `status`, `custom_start`; no aggregate; no view |
| `crates/chess-store/src/repo/analysis.rs` (new) | `upsert_move_eval` (one `INSERT .. ON CONFLICT DO UPDATE` per ply), `get_game_analysis` (one read snapshot), `save_variation` (`VariationAnchor` when `from_ply >=` stored `MOVE` count), `delete_variation`, `list_variations`, the failpoint helper |
| `crates/chess-store/src/repo/jobs.rs` (new) | `enqueue_job`, `enqueue_unevaluated_games`, `claim_job` (lease from `ANALYSIS_LEASE`, 10 s; `now` is an argument), `job_heartbeat`, `complete_job`, `fail_job` (compare-and-set on `owner` and `state = 'running'`), `requeue_failed` |
| `crates/chess-store/src/store/d5.rs` (new) | the 15 `Store` methods; a child module of `store` so it uses the private `read`/`write` helpers; clock injected; once-per-start flag |
| `src/lib.rs`, `src/repo/mod.rs`, `src/error.rs`, `src/fault.rs`, `src/store.rs` | small additive edits (list in GATE-110 "Covers"). Each file was re-read immediately before its edit because task D2 edits the same crate |

Behavior pinned by tests (choices inside the SDD's freedom, no approval claimed): rows are in `game.id` (insertion) order and `limit` keeps the first N; in a self-play game the viewed Player is `White` unless a side is requested; `active_ms` is 0 for a `NULL` `total_active_ms`; a computer's `category_now` is `Category::from_rating(strict_elo)` read at query time; `claim_job` takes the lowest `game_id`, stale means heartbeat strictly older than 10 s (exactly 10 s is fresh), a `NULL` heartbeat of a running job is stale, a future heartbeat is fresh; `complete_job`/`fail_job`/`job_heartbeat` return `NotOwner` for an absent, not-running or foreign job; `enqueue_job` never changes an existing job; `enqueue_failed_once_per_start` is once per `Store` open.

## 2. Acceptance criteria

| Criterion | Status | Tests (`crates/chess-store/tests/`) |
|---|---|---|
| UT-ST-037 (`from_ply` at or beyond the ply count is `VariationAnchor`; below stores one line with `created_at`; no `player_id` column) | Met `[RAN]` | `analysis_variations.rs`: `ut_st_037_a_branch_from_the_final_position_is_refused_below_it_is_stored`, `a_game_without_moves_has_no_anchor`, `the_variation_table_has_no_player_column`, `one_line_per_row_nested_text_is_stored_verbatim_and_listed_by_anchor`, `delete_variation_removes_one_row_and_is_idempotent` |
| UT-ST-060 stats half (structural keys only; voided, self-play and custom-start games are rows) | Met `[RAN]` | `stats_rows.rs`: `ut_st_060_stats_rows_apply_no_counting_predicate`, `the_side_key_selects_the_seat_and_sets_the_colour`, `the_reset_window_is_started_at_greater_or_equal`, `limit_keeps_the_first_n_in_start_order`, `head_to_head_takes_both_seats_and_only_that_opponent`, `games_for_history_returns_whole_rows_of_every_kind_with_window_and_limit`, `imported_games_name_no_player_and_are_not_rows`, `unknown_players_are_not_found` |
| Rating points with `NULL game_id` are not games (decision of 2026-11-11) | Met `[RAN]` | `rating_points_without_a_game_are_not_games` |
| No SQL view | Met `[RAN]` | `no_sql_view_exists` (`sqlite_master` has no view) |
| Evals from White's point of view; `get_game_analysis` with `evaluated` and `total` | Met `[RAN]` | `an_eval_is_stored_as_given_from_whites_point_of_view_and_read_back`, `a_second_upsert_of_a_ply_replaces_every_field`, `bad_evals_are_refused_and_write_nothing`, `get_game_analysis_reports_the_job_state_and_unknown_games`, `a_game_without_moves_reports_zero_of_zero` (the store keeps `cp`/`mate` as given; the sign convention is the worker's, tested here only as "unchanged by the store") |
| CT-ST-030 (claim) | Met `[RAN]` | `job_queue.rs`: `ct_st_030_claim_takes_the_lowest_queued_job_and_marks_it_running`, `claim_with_an_empty_queue_is_none`, `the_claim_lease_is_ten_seconds` |
| CT-ST-031 (stale reclaim, injected clock) | Met `[RAN]` | `ct_st_031_a_running_job_is_reclaimable_only_when_its_heartbeat_is_older_than_the_lease`, `a_heartbeat_dated_in_the_future_counts_as_fresh`, `a_running_job_without_a_heartbeat_is_stale` |
| CT-ST-032 (lost-owner rejection) | Met `[RAN]` | `ct_st_032_a_worker_that_lost_the_job_cannot_touch_it`, `owner_commands_on_a_job_that_is_not_running_or_not_there`, `a_done_job_never_goes_back` |
| Queue: `enqueue_job`, `enqueue_unevaluated_games`, `enqueue_failed_once_per_start` | Met `[RAN]` | `enqueue_job_is_idempotent_and_leaves_running_and_failed_jobs_alone`, `enqueue_unevaluated_games_selects_finished_games_with_missing_evals`, `enqueue_failed_once_per_start_requeues_failed_jobs_once_per_open` |
| CT-ST-007 for these commands (FaultPlan, feature `test-support`, `src/fault.rs`) | Met `[RAN]` | `d5_faults.rs`: `ct_st_007_a_fault_before_the_commit_changes_nothing_and_the_command_works_afterwards` (11 calls: both `upsert_move_eval` paths, `save_variation`, `delete_variation`, `enqueue_job`, `enqueue_unevaluated_games`, `claim_job`, `job_heartbeat`, `complete_job`, `fail_job`, `enqueue_failed_once_per_start`; the failpoint fires, the three tables are identical before and after, the command then succeeds), `a_failed_requeue_does_not_use_up_the_once_per_start_retry`, `the_d5_failpoints_are_registered_and_distinct_from_the_sdd_list` |
| BM-ST-03 (10 000-game fixture; FF-5 store half) | Recorded `[RAN]`, see section 4 | `stats_bench.rs`: `bm_st_03_ten_thousand_game_fixture` |
| No dependency on B, D2, D3, D4 | Met `[OBS]` | the D5 code reads only `game`, `move`, `player`, `move_eval`, `variation`, `analysis_job`; no journal, no finalize |

## 3. Tests added `[OBS]`

41 tests in five files plus one helper module (no existing test changed or weakened): `stats_rows.rs` 11, `analysis_variations.rs` 10, `job_queue.rs` 12, `d5_faults.rs` 3 (feature `test-support`), `stats_bench.rs` 1, helper `tests/d5_support/mod.rs`. Because D2 owns finalize, the tests write finished game rows with SQL (`seed`), with constraint-consistent values; the store API is used for everything under test.

## 4. Commands run and results `[RAN]`

Machine: Apple M4 Pro, macOS 26.7.1, rustc/cargo 1.99.1, other engineers' builds and tests running at the same time (the figures are noisy).

`cargo fmt --check`: clean. `cargo clippy -p chess-store --all-targets -- -D warnings`: clean with default features and with `--features test-support`. `node ci/check-crate-edges.mjs`: "crate-edge check OK: 8 crates, 11 workspace edges, all allowed".

`cargo test -p chess-store`, default features, 5 runs: every run `passed=277 failed=0 suites=25`. With `--features test-support`: runs 1 to 4 `passed=330 failed=0 suites=25`; **run 5 was still running when this was written and is not confirmed** `[UNV]`. The counts include task D2's additions as they stood during each run.

`CHESS_CI_OFFLINE=1 ci/check.sh`: **not run** `[UNV]`; to be run in the foreground by the caller.

BM-ST-03, 10 000 games, 40 plies each (400 000 `MOVE` rows), one Player in every game, `cargo test -p chess-store --test stats_bench -- --nocapture`:

| Query | Rows | Debug build | Release build |
|---|---|---|---|
| `stats_rows`, no key | 10 000 | 52.9 ms | 18.6 ms |
| `stats_rows`, side white | 9 000 | 46.6 ms | 10.9 ms |
| `stats_rows`, window, limit 100 | 100 | 1.6 ms | 0.5 ms |
| `head_to_head_rows` | 5 000 | 27.9 ms | 6.7 ms |
| `games_for_history` | 10 000 | 39.6 ms | 6.9 ms |

The test asserts the row counts and a 10 s sanity ceiling only. The NFR budget for the whole statistics path (rows plus `chess-domain::aggregate` plus mapping) is the `chess-app` half of FF-5 and was **not** measured here `[UNV]`; the domain `aggregate` is not called because `StatsGameRow` is not a `StatsRow` (FIND-D5-01). Per the SDD, no SQL fast path was added.

Coverage (`cargo llvm-cov -p chess-store --features test-support` over the four D5 test files, line coverage as printed): `repo/jobs.rs` 100.00 %, `store/d5.rs` 100.00 %, `repo/stats.rs` 99.14 %, `repo/analysis.rs` 96.43 %, `api_analysis.rs` 90.91 % (the uncovered lines are the `Err(())` arms of the two `FromStr` impls, which no stored value reaches). Statement/line level only; no branch or MC/DC claim.

## 5. Test strength: hand mutants `[RAN]`

50 hand mutants were applied one at a time to a copy of the working tree in the scratchpad (so the concurrent engineer's runs never saw a mutant), each followed by `cargo test -p chess-store --features test-support` over the four D5 test files. Mutants that did not compile (the workspace denies warnings: an unused variable or import) were replaced by a compiling variant of the same fault; the replaced ones are not counted.

**49 killed, 1 survived, 0 unexplained.**

| Area | Mutants (all killed unless noted) |
|---|---|
| `stats.rs` | window `>=` to `>`; order `DESC`; white seat reads `black_id`; head-to-head second seat; draw maps to win; Black-wins seat swapped; default colour inverted; opponent seat swapped; category for humans; `NULL` active time 1; side key ignored for colour; `self_play` and `custom_start` inverted; ply count +1 |
| `analysis.rs` | anchor `>=` to `>`; `cp`/`mate` check `&&` to `\|\|`; upsert keeps old `engine_version`; keeps old `depth`; evals order `DESC`; variations order; `delete` always true; delete by game; `total` forced to `evaluated`; `created_at` zero; eval and variation failpoints removed |
| `jobs.rs` | stale `<=`; lease halved, shortened, lengthened; claim highest; `NULL` heartbeat fresh; owner check dropped; state check dropped; `NotOwner` never; enqueue overwrites; sweep: only completed, includes in-progress, `<=`; requeue keeps owner, requeues all not done; claim keeps old heartbeat; heartbeat unchanged; five failpoints removed (claim, owner update, requeue, `enqueue_job`, sweep) |
| `store/d5.rs` | once-per-start guard off; flag not reset after an error |

Survivor: **sweep ignores moveless games** (replace `AND EXISTS (SELECT 1 FROM move ..)` by `AND 1 = 1`). It is an equivalent mutant `[INF]`: a game with no moves has 0 evaluations and 0 moves, so `count(evals) < count(moves)` is already false. The `EXISTS` clause is kept as documentation of intent; the test `enqueue_unevaluated_games_selects_finished_games_with_missing_evals` includes a moveless game.

## 6. Findings (Engineer's own, all OPEN; severity is mine)

| ID | Severity | Finding |
|---|---|---|
| **FIND-D5-01** | **Major, SDD/architecture conflict, possible G2** | The store cannot return `chess_domain::StatsRow`: its `color` is a `chess_rules::Side` and `chess-store` has no edge to `chess-rules` (`ci/check-crate-edges.mjs`: "all allowed" with the current edges `[RAN]`; `chess-domain` states it re-exports nothing from `chess-rules`, `crates/chess-domain/src/lib.rs:6-9` `[OBS]`). The SDDs say the store returns "rows" without naming the type. Chosen: a store-owned `StatsGameRow` with `StatsSide`; `chess-app` maps to `StatsRow`. Needs a ruling: accept, or add the edge (G2), or give `StatsRow` a domain-owned side (G2, domain SDD baselined). Blocks nothing in the store; `chess-app` part B must map |
| **FIND-D5-02** | **Major, SDD gap, G2/G3 if a column is added** | `delete_variation` needs an identity; schema 1 gives `variation` an integer key only, D-40 forbids integer row keys across the API, and SDD 3.7.2 names no argument. Chosen: opaque `VariationRef` (no constructor, no accessor, valid for the returned store). Alternatives in GATE-110. A `uuid` column would be a stored-format change (G3) |
| FIND-D5-03 | Minor | `games_for_history` returns whole `GameRecord`s (the same query as `list_games` with player, window and limit). The SDD names no return type. History needs the opponent name only via a second call |
| FIND-D5-04 | Minor | `limit` keeps the first N in start order (oldest). A "recent games" view wants the last N. The SDD says only "a `LIMIT`". Ask `chess-app` which it needs before the caller relies on it |
| FIND-D5-05 | Minor, SDD gap | Three failpoints (`eval.before_commit`, `variation.before_commit`, `job.before_commit`) are not in the SDD section 4 list; registered in `fault::D5_FAILPOINTS`. A dated SDD note is requested |
| FIND-D5-06 | Minor, for D2 and all failpoint users | A failpoint context built from `Connection::path()` did **not** match a plan installed for the test's temporary directory; the same call with the path built from `DataDir::db_path()` did `[RAN]` (first run failed, second passed). Cause not isolated; a symlinked temporary directory on macOS (`/var` versus `/private/var`) is the likely reason `[INF]`. D5 passes the `DataDir` path. D2's `abort/void/reset.before_commit` call sites should do the same |
| FIND-D5-07 | Minor | `save_variation` counts stored `MOVE` rows. For a game whose newest moves are still in the journal backlog (part B/D3), the guard sees fewer plies and may accept the final position or refuse a valid anchor. The app guards first (decision 13); revisit when apply exists |
| FIND-D5-08 | Minor | `NotOwner` is also returned for a missing or not-running job, and a second `complete_job` is `NotOwner` (not idempotent). SDD says only that a worker that lost ownership gets `NotOwner` |
| FIND-D5-09 | Minor | "Once per start" is once per `Store` open (an in-memory flag). Two processes each requeue once; a second `Store::open` in one process counts as a new start |
| FIND-D5-10 | Info | `enqueue_job` does not check the game is finished (the worker's rule, DAT-3); the sweep does. Empty PGN text and empty `engine_version` are accepted (SDD silent) |
| FIND-D5-11 | Info | `get_game_analysis` reads job, evals and counts in one `unchecked_transaction` on a read connection (a WAL snapshot `[INF]`); no concurrent-writer test was written `[UNV]` |
| FIND-D5-12 | Info | `cargo fmt -p chess-store` formats the whole crate, so it may have touched D2's in-progress files while D2 worked; `cargo fmt --check` was clean afterwards. I cannot tell whether any of D2's lines were reformatted |

SDD conflicts: FIND-D5-01 (row type), FIND-D5-02 (variation identity). No G3 change was made. No blocker for the Reviewer pass.

## 7. Not run, not analyzed, assumed

- Not run: the `chess-app` half of FF-5 and any `chess-domain::aggregate` differential (needs FIND-D5-01 settled); concurrent reader and writer tests; two-process job claims (a second process claiming the same job through the file lock-free queue is covered only by the `BEGIN IMMEDIATE` argument `[INF]`); Intel Mac.
- Not analyzed: the SQL query plans (`EXPLAIN QUERY PLAN`) of the row queries; the effect of a Player with a very large game count on the correlated `count(*)` of plies beyond 10 000 games.
- Assumed (ASM-D5-1): the structural key "Player" means a seat of the game (white or black), not "any game the Player's name appears in"; imported games name no seat and are never rows. ASM-D5-2: the window is passed by the caller (the Player's own `reset_at` for the Player's own view, `None` for an opponent's view, D-26), not read by the store.
- The Reviewer pass (fresh invocation) and the System Architect desk review have not happened. Nothing is approved. No commit, push or bead close was made.


## Rework 2026-11-22 (chess-a0r.3.50, Brian's rulings)

FIND-D5-01 A: nothing to change (`StatsGameRow` and `StatsSide` stay store-owned; `chess-app` maps). FIND-D5-02 B and FIND-D5-06 are done. **Nothing here is approved; GATE-110 stays PENDING.** Public interface change of `chess-store` (G2; the consumer `chess-app` is not built, so no caller is broken).

### D5-02 B: a variation is `(game, from_ply, pgn)`

- `VariationRef` and the `id` field of `VariationRecord` are gone (`api_analysis.rs`, `lib.rs`). `Store::save_variation(game, from_ply, pgn)` is idempotent on that key: one `BEGIN IMMEDIATE` transaction, the game is resolved (`NotFound`), an empty text is `InvariantViolated` (the crate's existing error for a value no caller may send; nothing is written), a stored key returns the stored record with its original `created_at` before the anchor check, a new key at or beyond the stored ply count is `VariationAnchor`, else insert. `Store::delete_variation(game, from_ply, pgn) -> bool` deletes the keyed row (`NotFound` for an unknown game, `false` when no row matches). `list_variations` unchanged. No DDL, no UNIQUE index (the immediate transaction is the guarantee), so no stored-format change.
- Tests (`tests/analysis_variations.rs`, `tests/d5_faults.rs`, doc test in `lib.rs`): VAR-1 `var_1_saving_a_key_twice_returns_the_stored_record_and_adds_no_row`; VAR-2 and 3 `var_2_and_3_each_distinct_key_is_its_own_row` (trailing space and case are different lines); VAR-5 `var_5_a_repeat_of_a_stored_key_is_returned_before_the_anchor_check`; VAR-6 `var_6_an_unknown_game_is_not_found`; VAR-7 and 8 `var_7_and_8_delete_variation_removes_the_keyed_row_and_is_idempotent`; VAR-9 `var_9_an_empty_text_is_refused`; VAR-10 `var_10_concurrent_saves_of_one_key_give_one_row` (two `Store` handles, 200 rounds, equal records, 200 rows); VAR-11 the existing `variation.before_commit` cases in `d5_faults.rs`, now calling `delete_variation(game, from_ply, pgn)`; VAR-12 a `compile_fail,E0432` doc test for `use chess_store::VariationRef;` (runs in both feature settings). VAR-4 is the existing `ut_st_037_...`. Red first: the new tests did not compile against the old API `[RAN]`.
- Mutants (9 killed, 0 survived): existing-row check disabled, the key ignores `pgn`, ignores `from_ply` (written as `?2 = ?2` so the kill is not a parameter-count error), delete ignores `from_ply`, `game`, `pgn`, the empty-text check removed, a repeat returns `created_at + 1`, anchor `>=` changed to `>`. Two further mutants did not compile and were not rewritten (`created_at` replaced by `now` in the early return: the variable becomes unused; a constant game key: unused variable); the `created_at + 1` mutant covers the same line.

### FIND-D5-06: failpoint paths are canonical

`fault.rs` `canonical()` resolves the longest existing ancestor (symlinks such as `/var` to `/private/var`) and appends the rest; `install` stores the canonical root, `trigger` compares the canonical context; with no plan installed `trigger` returns before any path work. Test `tests/fault_injection.rs` `a_plan_and_a_context_match_across_a_symlinked_directory` (plan through a link and context under the real path, a context that does not exist yet, the reverse direction, an outside path and a `..` path do not match, counters are exact). Red first `[RAN]` (it failed before the change). Mutants: root not canonical, context not canonical, ancestor walk dropped: killed (3); fallback for a path with nothing resolvable returns an empty path: **survived**, and is unreachable in practice (the filesystem root always resolves) `[INF]`.

### Findings of this rework

| ID | Severity | Finding |
|---|---|---|
| FIND-D5-R-01 | Minor, SDD text | SDD-chess-store 3.7.2 table row and note 20 still say `delete_variation` with an open identity (FIND-D5-02); update to the triple, the idempotent save and the existing-row-before-anchor order. The app SDD's `save_variation(game_id, from_ply, moves)` must produce **one canonical PGN text per line**: the store compares bytes, so two spellings of one line are two variations. |
| FIND-D5-R-02 | Info | An idempotent repeat returns before the anchor check, so a repeat still succeeds after the game's stored plies shrank (VAR-5); a new key is always checked. |
| FIND-D5-R-03 | Info | No UNIQUE index: another writer that bypasses the store API could insert a duplicate key; `list_variations` would then show both and `delete_variation` would remove both. A UNIQUE constraint is a stored-format change (G3), not part of the ruling. |

Final runs: see the part B rework section R4 (one set of runs for the crate). Not run: a consumer test in `chess-app` (bead .3.13).


## Fixes 2026-11-22 (after the Reviewer pass `review-chess-store-D1-D2-D5.md`, decision bead chess-a0r.3.50)

Engineer stance. **Nothing here is approved; GATE-110 stays PENDING.** Hand mutants were run one at a time and restored (`[RAN]`).

| Finding | Status | Fix | Tests | Mutants |
|---|---|---|---|---|
| FIND-D5R-05 | FIXED | `complete_job` and `fail_job` (src/repo/jobs.rs `update_owned`): when the compare-and-set changes no row, and the job is already in the state the command asks for and this owner holds it, the answer is `Ok` and nothing is written. Another owner, or the other command (`fail` after `complete`, `complete` after `fail`), or a heartbeat after the end, is `NotOwner` as before | `complete_and_fail_are_idempotent_for_the_owner_that_already_did_it` (row unchanged by the retry; B refused; A refused after a lease reclaim by B; unknown game `NotFound`). Changed: `owner_commands_on_a_job_that_is_not_running_or_not_there` used to pin "a second complete by the same owner is refused"; it now pins `Ok` for the same owner and `NotOwner` for `fail` and heartbeat | J1 (owner not compared), J2 (state not compared), J4 (always repeat), J5 (only `Done`), J6 (only `Failed`) killed |
| FIND-D5R-01 | FIXED | `MAX_VARIATION_PGN_BYTES = 1 MiB` (ESTIMATE; exported) and new `StoreError::VariationTooLarge { max_bytes }` (`StoreError` is `non_exhaustive`). `save_variation` checks the byte length first, before the game lookup; a text of exactly the cap is stored | `d5r_01_a_variation_text_above_one_mib_is_refused_and_the_boundary_is_exact` (exact cap stored and its repeat idempotent; cap + 1, 4x cap, and 524 289 two-byte characters refused; exactly cap bytes of two-byte characters stored; nothing written for a refusal; unknown game with a huge text gets the size error; empty text still `InvariantViolated`) | S1 (`>=`), S2 (no check), S3 (`chars().count()`), S4 (cap + 1), S5 (check after the game lookup) killed |

Public surface added: `StoreError::VariationTooLarge`, `MAX_VARIATION_PGN_BYTES`. The app maps the new error (not built yet).

Left alone on purpose (decisions pending): FIND-D5R-02 (indexes, a stored-format change, G3), FIND-D5R-03 (ordering key), FIND-D5R-04 (UT-ST-038 scan of `api_analysis.rs` and `store/d5.rs`: not in the fix list; I read the new constant and error and no integer row key leaves the crate).

Not run: a measured stall for a 1 MiB text (the review measured 64 MiB at 8.4 s; the cap is an estimate, not a measurement).


## Rulings 2026-11-23 (decision bead chess-a0r.3.52, package `decision-package-store-rework-findings.md`)

Engineer stance. **GATE-110 stays PENDING; its tier is corrected to G2 in the gate record (G0 record fix). No approval has been received.**

- **FIND-D5R-03 (d1, G1) built:** `stats_rows`, `head_to_head_rows` (both `src/repo/stats.rs`), `games_for_history` and `list_games` (`src/repo/game.rs`) order by `(started_at, id)`; `limit` is the oldest N. Tests: `stats_rows::d5r_03_stats_rows_are_ordered_by_start_then_id_and_limit_is_the_oldest_n` (rows inserted in the order 1, 4, 2, 3 with starts 40 000, 30 000, 10 000, 10 000: result is 2, 3, 4, 1; limit 1 gives game 2; window 20 000 with limit 1 gives game 4; head to head limit 2 gives 2, 3), `d5r_03_games_for_history_and_list_games_use_the_same_key`. Failed on the old code `[RAN]`. Mutants KILLED: M4a (order by `id`), M4b (start descending), M4c (tie on `id` descending), M4d (`list_games` back to `id`), M4e (`list_games` descending). The existing `limit_keeps_the_first_n_in_start_order` still passes.
- **FIND-D5R-02 (d2):** no index, no DDL change (the ruling). Revisit trigger as in the package: `save_variation`, `claim_job` or `stats_rows` p95 above 50 ms at real row counts, or more than 50 000 variation rows.
- **Imported games:** `game.started_at` is nullable (schema 1) and SQLite sorts `NULL` first, so an imported game comes first in `list_games` with no player filter. `games_for_history` and the stats queries name a Player's seat, which an imported game has not, so they are not affected `[OBS migrations/0001_init.sql:56, INF]`. Reported, not decided.
- `cargo test -p chess-store`, 5 runs, and with `--features test-support`, 5 runs: all exit 0 `[RAN]`.

## Rulings 2026-11-24 (decision bead chess-a0r.3.56, package `decision-package-store-D3.md`)

Engineer stance. **GATE-110 stays PENDING (Brian Prascher, G3 for stored-format and stored-value items). No approval has been received.** Brian's rulings (relayed by the coordinator, no verbatim quote) are applied in `crates/chess-store`. Not reviewed by a Reviewer. Verification: `cargo test -p chess-store` 440 tests and with `--features test-support` 515 tests (5 runs each, all exit 0 `[RAN]`); race binary (`first_launch`, 500 SIGKILL rounds per run) 10 runs all exit 0 (6 tests each, 70 to 88 s); `cargo fmt --check`, `clippy -D warnings` both feature settings clean `[RAN]`; `node ci/check-crate-edges.mjs` and `CHESS_CI_OFFLINE=1 ci/check.sh` both exit 0 (`All gates passed.`, npm audit skipped by the offline flag).

- **F2-01 (G1, RULE-7, D-150):** `stats.rs` SQL gets `AND g.status <> 'imported'`, so an imported game that has both seats is no row of `stats_rows` or `head_to_head_rows` (before: `Corrupt` for every statistic of both Players). Test `tests/stats_rows.rs::f2_01_an_imported_game_with_both_seats_is_not_a_statistics_row` (the history list still has it). Mutant (filter removed) KILLED.
- **F2-02 (G1, D-150):** `list_games` (and so `games_for_history`) order `g.started_at IS NULL, g.started_at, g.id`: a game with no start time comes last. Test `f2_02_a_game_without_a_start_time_is_listed_last` (limit 2 keeps the two played games; a window excludes the NULL row as before). Mutant KILLED.
- **F2-03:** the `id` tie-break is now pinned. The plain scan returns equal start times in `id` order anyway, so the mutants that drop `g.id` were equivalent in black-box terms; the test creates two expression indexes `(started_at, 0 - id)` and `(started_at IS NULL, started_at, 0 - id)` with SQL so that SQLite scans ties in the opposite order. `tests/stats_rows.rs::f2_03_equal_start_times_are_ordered_by_id_and_a_limit_keeps_the_lower_id` covers `stats_rows`, `head_to_head_rows`, `list_games`, `games_for_history` with `limit 1`. **Mutants S02 (statistics) and S05 (`list_games`) now KILLED** (also with the `IS NULL` term in place) `[RAN]`.
- Verification: as above. Not reviewed. Approved by: nobody.


## GATE-110 conditions 2026-11-26 (task chess-a0r.3.62, baseline B6)

Engineer stance. **GATE-110 stays PENDING (Brian Prascher, G2). Nothing here is an approval.** Only `crates/chess-store/tests` changed; **no `src` change** (a test-support hook was tried and removed, see below). Not reviewed by a Reviewer.

### Condition 1: F2-03 tie-break pinned without a planted index (FIND-D3RL-06)

- **New file `tests/tie_break.rs`** (no feature needed, 4 tests, no index created):
  - `equal_start_times_are_ordered_by_id_whichever_seat_the_player_has` and `..._after_analyze`: five games, three with equal `started_at`; the tied games are inserted in the order 7, 3, 5 with mixed seats for Ann (black, white, black), so their row keys are 1, 2, 3 and the tie order is neither the game number nor the uuid order. Covers `stats_rows` (no limit, limits 1 to 4, window then limit), `head_to_head_rows` (limits 1 to 4), `games_for_history` (all, limit 3), `list_games` (no player, limits 2 and 3).
  - `tie_break_is_named_in_every_order_by`: reads `src/repo/stats.rs` and `src/repo/game.rs`, takes every `ORDER BY` that sorts by `started_at` and asserts it ends in `, g.id` (exactly one clause per file). `the_order_by_scanner_finds_what_it_is_meant_to_find` is its self-test.
- **Premise that did not hold `[RAN]`:** `PRAGMA reverse_unordered_selects = ON` cannot expose the missing tie-break. I added a test-support hook (`Store::set_reverse_unordered_selects`, via `ReadPool`) and ran S02 and S05 with the pragma on: both survived all behavioural assertions. `[INF]` SQLite applies the pragma only to queries with no `ORDER BY`; I did not read SQLite's source to confirm. I removed the hook (`git checkout` of `readpool.rs` and `store.rs`), so `src` is unchanged.
- **Honest limit:** with the `, g.id` dropped, no data-only test I could build changes the answer: `game.id` is the rowid, ties come out in rowid order on the scan and sorter plans SQLite chose here (`[INF]`, I did not print `EXPLAIN QUERY PLAN` from the bundled SQLite). **S02 and S05 are killed only by the source-text test**, a structural pin that does not depend on the planner. The behavioural tests kill the other mutants below without any index. Brian may accept the structural pin or ask for a different seam.
- **Mutants, run on `tie_break.rs` alone, planted index helper not involved `[RAN]`** (each applied to the original file, then restored):

| Mutant | Change | Result |
|---|---|---|
| S02 | stats `ORDER BY g.started_at, g.id` -> `g.started_at` | KILLED by `tie_break_is_named_in_every_order_by` only |
| S05 | list `... g.started_at, g.id` -> `g.started_at` | KILLED by `tie_break_is_named_in_every_order_by` only |
| M-id-desc-stats | stats `g.id DESC` | KILLED (3 tests) |
| M-id-desc-list | list `g.id DESC` | KILLED (3 tests) |
| M-no-start-stats | stats `ORDER BY g.id` | KILLED (3 tests) |
| M-tie-by-uuid-stats | stats tie on `g.uuid` | KILLED (3 tests) |
| M-tie-by-uuid-list | list tie on `g.uuid` | KILLED (3 tests) |
| M-swap-list | list `ORDER BY g.id, started_at IS NULL, started_at` | KILLED (3 tests) |
| M-start-desc-stats | stats `started_at DESC, g.id` | KILLED (2 behavioural tests) |

### Condition 2: D5R-04, UT-ST-038 scan coverage

- **Answer: it did NOT scan `api_analysis.rs` or `store/d5.rs`.** The scanner's file list was `["api.rs", "store.rs", "error.rs", "datadir.rs"]` `[OBS tests/boundary.rs before this change]`.
- **Extended** (`tests/boundary.rs`): `SCANNED_PUBLIC_FILES` = `api.rs`, `api_analysis.rs`, `store.rs`, `store/d3.rs`, `store/d4.rs`, `store/d5.rs`, `error.rs`, `datadir.rs`. All clean `[RAN]`. New self-test `ut_st_038_a_planted_leak_in_the_d5_files_is_detected` plants `pub game_id: i64` and a `player: Option<u64>` parameter into the text of `api_analysis.rs` and `store/d5.rs` and asserts each is found.
- **Planted in the real files `[RAN]`:** appending a `pub game_id: i64` struct to `src/api_analysis.rs`, then to `src/store/d5.rs`, made `ut_st_038_no_public_type_or_method_takes_or_returns_an_integer_identity` and the self-test fail; files restored (`git status`: only the two test files differ).
- **Not scanned:** `backlog.rs`, `backup.rs`, `migrate.rs`, `journal/`, `repo/coord.rs` (public via `pub use`/`pub mod`). Not in scope of this condition; reported.

### Verification `[RAN]`
`cargo test -p chess-store` 3 runs and `--features test-support` 3 runs: all exit 0 (537 and 642 tests passed in the first run of each). `cargo fmt --check`, `clippy --all-targets -D warnings` both feature settings clean. `node ci/check-crate-edges.mjs` OK. `CHESS_CI_OFFLINE=1 ci/check.sh`: exit 0, "All gates passed." Unverified: `EXPLAIN QUERY PLAN` of the bundled SQLite for the seat `OR`.
