# Step 2, chess-store part D2: game lifecycle writes and ratings (chess-a0r.3.38)

Engineer stance. Gate: GATE-109 (G3, **OPEN, NOT APPROVED**). Baseline B6, CR chess-a0r.20. Nothing here is approved by anyone. No Reviewer pass has happened.

## 1. What was built

| Item | Where |
|---|---|
| `finalize_game` (Elo, rating rows, terminal columns, slots, leases, job, idempotent `AlreadyFinal`) and `abort_game` | `crates/chess-store/src/repo/finalize.rs` |
| the shared void function (`void_game`, `VoidFields`), `release_and_queue`, `void_unreadable_game`, `reset_player`, `unfinished_game_ids` | `src/repo/lifecycle.rs` |
| `check_rating_invariant` | `src/repo/invariant.rs` |
| DTOs `FinalizeRequest`, `FinalizeOutcome`, `RatingChange`, `AbortRequest`, `EndOutcome`, `VoidSpec`, `RatingViolation`, `RatingViolationKind` | `src/api.rs` (appended) |
| `Store::{finalize_game, abort_game, void_unreadable_game, reset_player, check_rating_invariant}`; exports | `src/store.rs`, `src/lib.rs` |
| `StoreError::UnlistedUnfinishedGames{ids}`; `HasUnfinishedGames` becomes `{ids: Vec<GameId>}` (F-A05 option A) | `src/error.rs`, `src/repo/player.rs` (`archive_player`) |
| failpoints `abort.before_commit`, `void.before_commit`, `reset.before_commit` | `src/fault.rs` (appended to `D1_FAILPOINTS`) |
| tests | `tests/lifecycle.rs` (20), `tests/lifecycle_faults.rs` (4, feature `test-support`); shape edits in `tests/{boundary,model,players}.rs` |

Elo arithmetic is `chess_domain::EloCalc::settle` and `rating_track` (whole integers, `f64::round` = half away from zero) [OBS crates/chess-domain/src/elo.rs]. The store reads the two rows inside the transaction, settles once for both sides, writes one `rating_history` row and one `player` update per side. A game is rated only when `game.rated` is set and `rating_track` returns `Rated(track)`; self-play and custom start are unrated whatever `rated` says.

Choices inside the SDD's silence (flagged, not approved): `GAME.termination` of an abort is `aborted` (the domain's `EndReason::Aborted` text [OBS types.rs:276]); `void_unreadable_game(game, content_hash)` takes `ended_at` from the store clock and leaves `journal_seq`; an empty `content_hash` is `InvariantViolated`; `reset_player(player, voids, default_preferences_json)` returns `()`; a listed game that is already terminal is skipped, a listed in-progress game of another Player is `InvariantViolated`, an unknown one `NotFound`; `check_rating_invariant` returns the list of violations (empty = holds); `abort_game` and `void_unreadable_game` answer `EndOutcome::{Applied, AlreadyFinal}`.

## 2. Criteria and tests

| Criterion | Test (all `[RAN]`, green) |
|---|---|
| UT-ST-035 hash NULL until terminal, set by finalize, abort, void, reset; CHECK refuses both wrong states | `ut_st_035_the_hash_is_null_until_terminal_and_set_by_every_ending` |
| UT-ST-050 Strict rated, columns, rows, slots, job | `ut_st_050_strict_rated_game_settles_both_sides_and_writes_rows_and_game_columns` |
| UT-ST-051 rounding vectors (3.64 to 4, -36.36 to -36, 20, -2.3 to -2, 0) | `ut_st_051_rounding_vectors_black_win_and_draw` |
| UT-ST-052 Training track separate, K counters per track (29 to 30 games: K 40 then 32) | `ut_st_052_training_track_is_separate_and_k_counters_are_per_track` |
| UT-ST-053 self-play, custom start, unrated: no rating row, Elo columns NULL | `ut_st_053_self_play_custom_start_and_unrated_games_change_no_rating` |
| UT-ST-054 idempotent repeat, byte-identical dump | `ut_st_054_finalize_twice_is_already_final_and_changes_nothing` |
| UT-ST-055 history sums to rating; invariant holds; domain `check_history` accepts | `ut_st_055_history_sums_to_the_rating_and_the_invariant_holds` |
| UT-ST-056 two games against one computer, two orders = in-order result by `EloCalc` | `ut_st_056_two_games_against_one_computer_give_the_in_order_result` |
| UT-ST-086 void half (zero moves, non-NULL hash, `voided_by` NULL, `RowGone` carries hash, `voided_by`, `ended_at`) | `ut_st_086_void_half_zero_moves_hash_is_stored_and_lease_beat_reports_the_row` |
| CT-ST-028 reset voids suspended, same-process, other-process games; no rating or counter change; reset points `k` 0, NULL game; `ended_at` = `reset_at`; opponent keeps the game and his slot is free | `ct_st_028_reset_voids_a_suspended_game`, `..._leased_by_this_process`, `..._leased_by_another_process` |
| CT-ST-028 extra game rolls the whole reset back | `ct_st_028_an_unlisted_unfinished_game_rolls_the_whole_reset_back` |
| CT-ST-007 fault just before commit leaves every table unchanged; retry succeeds | `tests/lifecycle_faults.rs`: finalize, abort, void, reset (4) |
| invariant: `game_id IS NULL` rows are points (`k` not read), each violation kind, unknown Player | `invariant_names_each_kind_of_damage_and_treats_null_game_rows_as_points`, `invariant_first_row_must_be_an_initial_point` |
| no integer id in new public types (UT-ST-038) | `tests/boundary.rs` green in all loops |
| other | `abort_sets_the_hash_...`, `finalize_refuses_an_empty_hash_...`, `a_game_voided_before_its_game_end_arrives_stays_voided_d_67_v`, `reset_edge_cases_...`, `reset_with_no_unfinished_game_...` |

## 3. Outputs

- `cargo fmt -p chess-store --check` -> FMT-OK. `[RAN]`
- `cargo clippy -p chess-store --all-targets -- -D warnings`, and with `--features test-support` -> both `Finished`, no diagnostics. `[RAN]`
- `cargo test -p chess-store`, 5 runs: 277 passed, 0 failed each. With `--features test-support`, 5 runs: 330 passed, 0 failed each. `[RAN]` (the counts include D5's tests, which had landed).
- `node ci/check-crate-edges.mjs` -> `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`. `[RAN]`
- `CHESS_CI_OFFLINE=1 ci/check.sh` (foreground, whole workspace) -> exit 0, "All gates passed."; `npm audit` skipped by the offline flag; cargo-deny printed an unmatched-advisory-ignore warning for RUSTSEC-2026-0441 (deny.toml, not mine) and finished `advisories ok, bans ok, licenses ok, sources ok`. `[RAN]`
- Coverage (`cargo llvm-cov` 0.9.1, only `--test lifecycle --test lifecycle_faults`, feature `test-support`), lines: `finalize.rs` 94.55 % (165 lines, 9 missed), `lifecycle.rs` 95.68 % (162, 7), `invariant.rs` 96.05 % (76, 3). Measured before the last two small edits (dedupe removal, one added test); not re-measured. Target 90 % met. `[RAN]`

## 4. Hand mutants (cargo-mutants is not installed)

Run on a private copy of the workspace (a copy was needed so no concurrent engineer compiled a mutated tree), `lifecycle`, `lifecycle_faults`, `players`, `games` test binaries per mutant. 54 distinct mutants over finalize, abort, void, reset, release and the invariant: **50 killed, 4 survived**.

| Survivor | Verdict |
|---|---|
| M07 `custom_start` ignored in `GameFacts` | equivalent: the schema CHECK forces `rated = 0` for a custom start, so the `rated` test already excludes it |
| M29 reset of a computer allowed | equivalent: the schema CHECK `kind = 'human' OR reset_at IS NULL` still returns `InvariantViolated` |
| M36 duplicate listed game not skipped | equivalent (the second entry finds the game terminal); the dedupe code was **removed** |
| M42 invariant: point does not restart the sum | equivalent once the chain check passes (the point's `before` equals the previous `after`); the line stays as the SDD wording |

One mutant (M43, first row need not be a point) survived the first run and was killed by a test added for it (`invariant_first_row_must_be_an_initial_point`, a draw of equals as first row). Several first-pass mutants did not compile (unused-variable errors); they were rewritten and re-run, none counted as killed by a compile error. Mutant list: scratchpad `mutants.py`, `mutants2.py`, not committed.

## 5. Findings (Engineer's own, all OPEN)

| ID | Severity | Finding |
|---|---|---|
| FIND-D2-01 | Info (answers the architect's question) | `chess-domain` `rating_series` does **not** read `k` (filters by track and window, orders by `(at, id)`); `check_history` does not read `k` and treats a `game_id` None row as a point that must not change the rating [OBS crates/chess-domain/src/stats.rs:190-]. F-A04 option A needs no domain change and no domain test for `k`. |
| FIND-D2-02 | Minor, SDD/domain decision | The store orders rating rows by `id`; the domain orders by `(at, id)`. `at` of a finalize row is `ended_at`. If a finalize is applied out of `ended_at` order, or `ended_at` is earlier than the initial point's `at`, domain `check_history` reports a gap although the store invariant holds. Found when the first version of UT-ST-055 used tiny `ended_at` values. Real runs are safe only while finalize order follows `ended_at` (D-72 startup order, D3 backlog). Owner: architect/domain SDD. |
| FIND-D2-03 | Minor, SDD conflict | `VoidSpec` field is `game` in SDD-chess-store 3.7.2 and `game_id` in SDD-chess-app (IFR-APP-02). Built as `game`. |
| FIND-D2-04 | Minor, SDD gap, G3 value | The SDD does not state the `termination` text of an abort; `aborted` chosen. Stored value, irreversible after release. Also `reset_player` extra argument (`default_preferences_json`) is the "..." of the SDD signature. |
| FIND-D2-05 | Minor | `finalize_game` writes `analysis_used` from the request verbatim (SDD), so a flag set by `set_analysis_used` and not carried in the `GameEnd` would be cleared. Decide: write the OR. |
| FIND-D2-06 | Minor, SDD gap | SDD failpoint list has `finalize.before_commit` only; three names were added (`abort.`, `void.`, `reset.before_commit`). Update the SDD list. |
| FIND-D2-07 | Minor (resolved here, same cause as D5's FIND-D5-06) | `Connection::path()` is a canonical path (`/private/var` for `/var`), so a `FaultPlan` rooted at a temp dir did not match [RAN: first fault tests did not fire]. Failpoint contexts are built from `DataDir::db_path()` and passed into the repo functions. |
| FIND-D2-08 | Minor | The "commit time" of `reset_at` and `ended_at` is read from the injected clock inside the store command just before `BEGIN IMMEDIATE`, not after the commit. Both columns carry the same value, which is what D-69 needs. |
| FIND-D2-09 | Minor, for D3 | `void_game` has no precondition check; callers check. A void applied to a `completed` row (the repair pair, D-87) would not undo a settled rating; D-87 says that pair is never finalized, so D3 must not call finalize before the void. |
| FIND-D2-10 | Info | `check_rating_invariant` checks ratings only (SDD wording); it does not compare `*_rated_games` with the number of game rows. Decide whether it should. |
| FIND-D2-11 | Info | `HasUnfinishedGames` shape change touched part A code (`archive_player`) and three earlier test files, by Brian's decision F-A05 A (relayed). |
| FIND-D2-12 | Info, disclosure | I ran `cargo fmt -p chess-store` (writes) several times; it formats every file of the crate, including files another engineer had in progress. `--check` is clean now. |

No Major finding, no blocker. SDD conflicts: FIND-D2-03, -04, -06. G2/G3 items for Brian: the whole gate (GATE-109), especially the stored texts (FIND-D2-04) and the changed public error variant.

## 6. Not run / not analyzed / assumed

- Power-loss and process-kill during these commands: not run (the fault tests stop before commit, not at a kill). `[UNV]`
- `chess-app` use of the commands: not built.
- Coverage not re-measured after the last edits; mutation by hand, not by a tool.
- Intel Mac: not applicable.
- ASM: the abort termination text and the empty-hash refusal are Engineer choices (section 1).


## Rework 2026-11-22 (chess-a0r.3.50, Brian's rulings)

Brian accepted FIND-D2-04 (stored abort text `aborted`) and FIND-D2-02 (store orders by `id`; finalizes are applied in `ended_at` order) and picked the SDD names: `VoidSpec` field `game`, failpoints `abort.before_commit`, `void.before_commit`, `reset.before_commit`. **No code of part D2 changed.** Nothing here is approved; GATE-109 stays PENDING.

- Pins that already existed `[OBS]`: the abort text `aborted` (`tests/lifecycle.rs`, `abort_sets_the_hash_releases_slots_queues_a_job_and_rates_nothing`, `termination == Some("aborted")`); the three failpoints (`tests/lifecycle_faults.rs`, `ct_st_007_{abort,void,reset}_with_a_fault_before_commit_changes_nothing_and_the_retry_succeeds`); `VoidSpec { game, .. }` is what every lifecycle test constructs.
- New test (the missing ordering pin, FIND-D2-02): `tests/lifecycle.rs` `d2_02_the_store_orders_rating_rows_by_id_and_the_domain_by_at_then_id`. In `ended_at` order the store's id order equals the domain's `(at, id)` order and `check_history` passes. When the second finalize has the earlier `ended_at` (a clock step back) the store still returns `[initial, game 1, game 2]` by `id`, `chess_domain::rating_series` returns `[initial, game 2, game 1]`, and `check_history` reports `HistoryError::Gap`. `[RAN]` green on its first run (a pin of existing behaviour, so there was no red step); hand mutants D206-1 (`ORDER BY h.at, h.id`), D206-2 (`h.id DESC`), D206-3 (`h.rating_after, h.id`) all killed.
- Final runs: see the part B rework section R4 (same runs; the lifecycle suite is in them). `lifecycle.rs` and `lifecycle_faults.rs` pass in all ten full runs.
- SDD text for the architect: SDD-chess-store 3.7.2 and the app SDD IFR-APP-02 use `game`; the SDD failpoint list gets the three names; the D2-02 sentence "the store orders by id; finalizes are applied in `ended_at` order, and the domain orders by `(at, id)`" (the D3 backlog and D-72 startup order must keep that order).
- Not run or analyzed: the D3 backlog order itself (D3 is not built), power loss.


## Fixes 2026-11-22 (after the Reviewer pass `review-chess-store-D1-D2-D5.md`, decision bead chess-a0r.3.50)

Engineer stance. **Nothing here is approved; GATE-109 stays PENDING (Brian Prascher, G3).** Hand mutants were run one at a time and restored (`[RAN]`).

| Finding | Status | Fix | Tests | Mutants |
|---|---|---|---|---|
| FIND-D2R-01 (Major) | FIXED, pending re-review | `check_rating_invariant` (src/repo/invariant.rs) runs the whole check in one `unchecked_transaction()` (a WAL snapshot at the first read, as `get_game_analysis`), so `strict_elo` and the rating rows come from one committed state | `find_d2r_01_the_invariant_check_never_reports_a_false_violation_beside_a_writer` (tests/lifecycle.rs): one thread starts and finalizes 300 games (human against computer, Strict), another loops the check. **Red before the fix `[RAN]`:** "32 false violations in 11053 checks, first: CurrentMismatch { expected: 1204, found: 1200 }". Green after | I1 (no transaction: back to the bug) killed by the new test; I2 (`!=` to `==`), I3 (`ORDER BY id DESC`), I4 (training column read from the strict one) killed by existing tests |
| FIND-D2R-05 | FIXED | `void_game` refuses (`InvariantViolated`, row unchanged) a row that is `completed` with settled ratings (an Elo column set, or a `rating_history` row of the game). The one bypass is `void_game_repair_pair`, the same update without the precondition, named for the D-87 repair pair that `apply_records` (D3) writes; it is `allow(dead_code)` with that reason until D3 calls it. `void_game` of an `in_progress` row, of a completed row that settled nothing (unrated) and of an already `voided` row is as before | unit tests in `src/repo/lifecycle.rs`: `void_game_refuses_a_completed_row_with_a_settled_elo_column`, `..._with_a_rating_row`, `void_game_voids_an_in_progress_row_and_a_completed_row_without_ratings`, `void_game_on_an_already_voided_row_is_not_refused`, `the_repair_pair_entry_voids_a_settled_completed_row`, `void_game_refuses_an_empty_hash_and_an_unknown_row` | V1 (check disabled), V2 (`status <> 'in_progress'`; first run SURVIVED, killed by the added voided-row test), V3, V4, V5 (inverted), V6 (hash check removed) killed |
| FIND-D2R-04 M14 | FIXED (test) | none needed | `m14_abort_of_a_voided_or_completed_game_is_already_final_and_changes_nothing` | removing `"voided"` or `"completed"` from the abort arm killed |
| FIND-D2R-04 M25 | FIXED (test) | none needed | `m25_void_unreadable_of_a_finished_game_is_already_final_and_changes_nothing` (completed, aborted, voided: `AlreadyFinal`, whole database dump unchanged; unknown game `NotFound`) | removing `"completed"` or `"aborted"` killed |
| FIND-D2R-04 N01 | FIXED (test) | none needed | `release_and_queue_twice_keeps_one_job_and_does_not_fail` (unit) | `INSERT OR IGNORE` to `INSERT` killed |

**Decision needed from the architect or Brian (D3):** what "the D-87 repair pair" does to a game whose `GameEnd` was already finalized (a completed row with settled ratings, then `Voided`). D-87 says the `GameEnd` is never finalized for a pair, so the row should still be `in_progress` and `void_game` suffices; if D3 ever meets a settled row it must call `void_game_repair_pair` knowingly, and the ratings stay settled (no un-settling exists).

Left alone on purpose (decisions pending): FIND-D2R-02 (`analysis_used` overwrite or OR), FIND-D2R-03 (terminal-state data validation), FIND-D2R-06 (the invariant checks ratings only).

Not run: the concurrent check against two OS processes (the test uses one process, two connections), power loss.


### D1 items of the same pass (FIND-D1R-02, FIND-D1R-03), recorded here because the D1 evidence file was outside this task's scope

Engineer stance. **Nothing here is approved; GATE-107 is not touched.** `FaultPlan::install` now returns `InstallError` (`Unknown(UnknownFailpoint)`, `UnresolvableRoot`, `DuplicateRoot`; the text of the unknown-name case is unchanged). `UnresolvableRoot`: the root has a `..` after a component that does not exist. `DuplicateRoot`: a plan is already installed for the same canonical root (any spelling). Tests (`tests/fault_injection.rs`): `a_root_with_a_parent_step_after_a_missing_component_is_refused_at_install`, `a_second_plan_for_the_same_root_is_refused_until_the_first_guard_drops`, `the_once_suffix_makes_a_parsed_rule_fire_once_after_its_skip` (`!`, `@n!`, `@n`, per-rule scope, bad forms). Mutants P1 to P6 and M94 (once-suffix) killed. Two existing tests relied on the silent override and were changed: `fault_injection.rs::the_text_form_parses_every_action_and_rejects_bad_input` (the empty plan goes to its own directory) and `journal_faults.rs::ct_st_001_...` (the first guard is dropped before the next plan for the same directory). Unwired failpoints (FIND-D1R-01) not touched.


## Rulings 2026-11-23 (decision bead chess-a0r.3.52, package `decision-package-store-rework-findings.md`)

Engineer stance. **GATE-109 stays PENDING (Brian Prascher, G3). No approval has been received.** Brian accepted the package recommendations on 2026-11-23. Failing test first (`[RAN]`: 5 of the new lifecycle tests failed on the old code; the "does not check" test passed, as it should).

### FIND-D2R-02 (b, G3): `analysis_used` is a latch

`finalize_game` writes `analysis_used = max(analysis_used, ?)` (`src/repo/finalize.rs`): the stored value is the OR of the stored flag and the request's, so it never goes from true to false. `set_analysis_used` is unchanged in code (its comment now says latch). Tests (`tests/lifecycle.rs`): `d2r_02_set_analysis_used_then_finalize_with_false_keeps_true`, `d2r_02_finalize_stores_the_or_of_the_stored_flag_and_the_request_flag` (4 cells of the truth table), `d2r_02_the_latch_survives_a_restart_before_the_finalize` (grant, drop the store, reopen, finalize with `false`: still true). Mutants KILLED: M2a (old overwrite), M2b (request ignored), M2c (`min`), M2d (`set_analysis_used` stores 0). The D3 apply-path test (`apply_records` of a `GameEnd`) is for part D3, not built `[UNV]`.

### FIND-D2R-03 (c, G1): terminal data is checked for shape only

`finalize_game` refuses `ended_at <= 0` and an empty `termination` with the existing `InvariantViolated`, nothing written; `abort_game` refuses `ended_at <= 0` (the same shape rule on the abort path; the package names finalize only, see the report of the task). Not added, per the ruling: `ended_at >= started_at`, journal sequence order, human-against-human abort. Tests: `d2r_03_finalize_refuses_a_non_positive_ended_at_and_an_empty_termination_and_writes_nothing` (0, -5, `i64::MIN`, empty termination; whole-database dump equal; `ended_at = 1` accepted), `d2r_03_abort_refuses_a_non_positive_ended_at_and_writes_nothing`, `d2r_03_the_store_does_not_check_time_order_journal_sequence_or_who_aborts` (ended before started, sequence 0, a human-against-human abort all stored). No existing test used a clock at 0 or below (the whole suite stayed green) `[RAN]`. Mutants KILLED: M3a (`< 0`), M3b (termination check dropped), M3c (abort check dropped), M3d (`||` to `&&`), M3e (boundary `<= 1`), M3f (an added `ended_at >= 5000` rule).

Runs `[RAN]` after the change: `cargo test -p chess-store` and with `--features test-support`, 5 runs each, all exit 0 (356 and 418 tests passed per run, 0 failed).

## Rulings 2026-11-24 (decision bead chess-a0r.3.56, package `decision-package-store-D3.md`)

Engineer stance. **GATE-109 stays PENDING (Brian Prascher, G3 for stored-format and stored-value items). No approval has been received.** Brian's rulings (relayed by the coordinator, no verbatim quote) are applied in `crates/chess-store`. Not reviewed by a Reviewer. Verification: `cargo test -p chess-store` 440 tests and with `--features test-support` 515 tests (5 runs each, all exit 0 `[RAN]`); race binary (`first_launch`, 500 SIGKILL rounds per run) 10 runs all exit 0 (6 tests each, 70 to 88 s); `cargo fmt --check`, `clippy -D warnings` both feature settings clean `[RAN]`; `node ci/check-crate-edges.mjs` and `CHESS_CI_OFFLINE=1 ci/check.sh` both exit 0 (`All gates passed.`, npm audit skipped by the offline flag).

- **F2-04 / D-144 (G3):** `void_game` (used by `void_unreadable_game`, `reset_player` and the `Voided` record) refuses `ended_at <= 0` and an empty hash with `InvariantViolated`, nothing written. Tests `repo::lifecycle::tests::void_game_refuses_an_ended_at_at_or_below_zero`, `tests/lifecycle.rs::d3_live_void_paths_refuse_a_clock_at_or_below_zero` (store clock 0 and -5: both commands refused, dump equal, then accepted at 2 000). Mutant `<=` to `<` KILLED.
- **F2-09 / D-149 (G1):** `void_game_repair_pair` deleted, `UPDATE` inline after the settled-ratings precondition. The two unit tests that used it now build the row with SQL (`void_game_on_an_already_voided_row_is_not_refused`); `the_repair_pair_entry_voids_a_settled_completed_row` is removed (the bypass it tested no longer exists; `void_game_refuses_a_completed_row_with_a_settled_elo_column` and `..._with_a_rating_row` still prove the refusal).
- **F2-05 (G1):** `set_analysis_used` refuses a game that is not `in_progress` with `InvariantViolated` unless the flag is already set (then `Ok`, idempotent). Test `tests/lifecycle.rs::f2_05_set_analysis_used_refuses_a_final_game_unless_the_flag_is_already_set` (completed unset, completed set, aborted, voided, in progress twice; dump equal). Mutants (guard removed, `already` inverted) KILLED.
- **D3-09:** live `finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player` flush the game's queued batches first (see `step2-store-D3-evidence.md`).
- Verification: as above. Not reviewed. Approved by: nobody.
