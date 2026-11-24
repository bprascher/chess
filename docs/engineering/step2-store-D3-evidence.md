# Step 2, chess-store part D3: apply path (chess-a0r.3.39)

Engineer stance. Gate: GATE-111 (G3, **OPEN, NOT APPROVED**). Baseline B6, CR chess-a0r.20. Nothing here is approved by anyone. No Reviewer pass has happened. GATE-108, GATE-109 and GATE-110 are not approved either.

## 1. What was built

| Item | Where |
|---|---|
| `apply_records` for every record kind (one `BEGIN IMMEDIATE` per batch, 100 ms lock wait, `apply.before_commit` / `apply.after_commit` failpoints) and the applier of a `ReconcilePlan` (`apply_reconcile_plan`) | `crates/chess-store/src/repo/apply.rs` (new) |
| Backlog scheduler (per-game order, 0.1 s doubling to 5 s, park, `StoreBacklog` age), pure, injected time | `src/backlog.rs` (new) |
| Store thread command loop: `Run`, `Apply`, `Sync`, `Status`; panic rule shared; 100 ms poll | `src/thread.rs` |
| `Store::apply_records`, `Store::sync_backlog`, `Store::backlog_status` | `src/store/d3.rs` (new) |
| `ApplyOutcome`, `BacklogStatus` | `src/api.rs` (appended) |
| spawn wiring, private `StoreOptions.monotonic`, test-support `with_monotonic_clock` | `src/store.rs`, `src/lib.rs` |
| `finalize_in_tx`, `abort_in_tx` (bodies of the D2 functions; the wrappers keep transaction, failpoint, commit) | `src/repo/finalize.rs` |
| tests | `tests/apply.rs` (32), `tests/apply_backlog.rs` (9, feature `test-support`), unit tests in `repo/apply.rs` (10 and 1 fault test), `backlog.rs` (8), `thread.rs` (+9) |

Per kind: `Start` creates the row when absent (`started_at` = `wall_ms`, frozen `rated`/`self_play`/`custom_start`, clock from `clock_spec`, no slots); `Move` upserts at journal ply + 1 (D-92) and sets the game clocks; `Takeback` deletes above `to_ply`; `Clock` sets the clocks; `GameEnd`, `Abort`, `Voided` run the D2 code (`finalize_in_tx`, `abort_in_tx`, `void_game`); other kinds change nothing; `journal_seq` becomes `max(stored, last seq)` per batch. A `GameEnd` with a `Voided` after it in the same batch is skipped (D-87).

## 2. Acceptance and tests (all `[RAN]`, green)

| Criterion | Test |
|---|---|
| UT-ST-082 all columns, 7 tables | `ut_st_082_finalize_live_equals_finalize_from_game_end_alone`; also `each_game_end_reason_writes_the_domain_termination_text`, `each_result_text_is_applied_and_a_bad_one_is_refused`, `a_game_end_replay_does_not_clear_analysis_used` |
| UT-ST-085 | `ut_st_085_a_voided_record_voids_a_row_that_is_not_yet_voided`, `..._an_already_voided_row_only_gets_the_journal_sequence`, `an_unreadable_void_record_...` |
| UT-ST-089 | `ut_st_089_a_start_only_journal_creates_the_row_with_started_at_equal_to_wall_ms` (second apply changes nothing), `a_whole_journal_restores_a_game_that_has_no_row` |
| `Move{ply: 0}` gives `MOVE.ply = 1` | `a_journal_move_with_ply_zero_is_stored_at_ply_one`, `fold_and_apply_records_agree_on_the_ply_base`, `applying_a_whole_journal_gives_the_rows_of_the_fold` |
| plan applied twice | `the_plan_applied_twice_equals_the_plan_applied_once`, proptest `the_sql_applier_equals_the_model_and_leaves_an_empty_plan` (SQL equals `apply_plan`, reconcile again is empty), `applying_a_journal_again_changes_nothing` (records) |
| CT-ST-008 | `ct_st_008_a_database_locked_by_another_process_queues_play_and_drains_afterwards` (child process holds the lock; journal appends continue; `store_backlog` false at 29.999 s, true at 30 s on the injected clock; the timer drains; rows equal the journal fold) |
| CT-ST-007 for apply | `ct_st_007_apply_with_a_fault_before_the_commit_...`, `..._after_the_commit_...`, `a_fault_that_fires_once_is_retried_by_the_timer`, plan applier fault test (feature on) |
| kill tests | `a_process_killed_at_the_apply_failpoints_leaves_a_store_that_applies_the_batch_once` (SIGKILL at both points, reopen, apply again, third time no change, rating settled once, both end states equal) |
| `sync_backlog` Ok or `Busy` | `sync_backlog_is_busy_at_its_timeout_while_the_lock_is_held_and_ok_after`, `sync_backlog_with_an_empty_backlog_returns_at_once`, thread tests with injected clock |
| never blocked for 5 s | `an_apply_attempt_waits_at_most_100_ms_for_the_lock_and_restores_the_5_s` (lock wait 90 ms to 1.5 s, 5 s restored), `a_locked_database_costs_one_short_attempt_...` (real lock, follow-up calls under 80 ms), injected clock: `batches_of_a_game_apply_in_order_and_a_busy_pass_makes_one_attempt` (ten games, one attempt per pass), schedule tests |
| D-87, FIND-D2R-05, F2-09 | `d87_a_game_end_followed_by_voided_is_not_finalized`, `d87_a_voided_record_never_un_settles_a_game_whose_ratings_were_settled`, `a_completed_unrated_game_can_be_voided_by_a_record`, `an_abort_followed_by_voided_is_refused` |
| FIND-F2-04 | `a_terminal_record_with_a_non_positive_ended_at_is_refused_and_writes_nothing` (typed `InvariantViolated`, no panic, whole batch rolled back) |
| UT-ST-038, layering | `tests/boundary.rs` green (no SQL words in `thread.rs`, `store.rs`) |

## 3. Outputs

- `cargo fmt -p chess-store --check` -> FMT-OK. `cargo clippy -p chess-store --all-targets -- -D warnings`, and with `--features test-support`, `RUSTFLAGS=-D warnings` -> `Finished`, no diagnostics. `[RAN]`
- `cargo test -p chess-store --locked`, 5 runs: 413 passed, 0 failed each. With `--features test-support`, 5 runs: 485 passed, 0 failed each. `[RAN]`
- `node ci/check-crate-edges.mjs` -> `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`. `[RAN]`
- Coverage (`cargo llvm-cov` 0.9.1, `--lib --test apply --test apply_backlog`, feature on), lines: `repo/apply.rs` 98.01 %, `backlog.rs` 99.58 %, `thread.rs` 97.81 %, `store/d3.rs` 100 %. `finalize.rs` shows 84.86 % in this subset because the D2 tests were not run in it. `[RAN]`
- `CHESS_CI_OFFLINE=1 ci/check.sh`: see section 7.

## 4. Hand mutants (cargo-mutants is not installed)

Scratch copy of the workspace (`mutws`), test set per mutant: `--lib --test apply --test apply_backlog` with the feature on, 90 to 420 s hang limit. **61 mutants over apply, backlog and the thread loop: 60 killed, 1 survived.** Eight first compiled with `-D warnings` errors and were rewritten (unviable variants not counted). Two survived at first and led to test changes: B13 (progress did not reset the delay; the test now grows the delay first) and T01 (poll 20 s; new test `a_jump_of_the_injected_clock_is_noticed_within_the_poll_...`); both are killed now. T04 and T05 (waiter deadline `>`, `sync` without a forced pass) are killed by a **hang**, not by an assertion.

| Survivor | Verdict |
|---|---|
| A18 `final_journal_seq: seq` -> 0 in the `GameEnd` request | equivalent: the batch update sets `journal_seq = max(before, last seq)` afterwards and `last seq >=` every record's seq. The field matters only for the live `finalize_game` |

## 5. Findings (Engineer's own, all OPEN)

| ID | Severity | Finding |
|---|---|---|
| FIND-D3-01 | **Major (decision, G3)** | A journal `GameEnd` or `Abort` with `ended_at <= 0` cannot be applied (FIND-F2-04). `apply_records` returns `InvariantViolated`, writes nothing, does not queue it. Consequence: that journal can never be finalized by apply, restore or the startup scan; each start meets the same refusal. Brian has not ruled whether the journal format must refuse such a value at encode and decode (as CH-4 did for the `u64` fields). `Voided.ended_at` is not checked (any value). |
| FIND-D3-02 | Minor | The SDD is silent on a batch that fails with an error no retry cures. As built: the first attempt returns the error and queues nothing; on a retry the game is parked (order kept, nothing dropped, backlog stays non-empty, `sync_backlog` gives `Busy`, `BacklogStatus.parked_games`). Observable behavior. |
| FIND-D3-03 | Info | `void_game_repair_pair` has no caller. D-87 does not demand it: the pair is skipped before the void (no rating settled), and a pair cannot follow a live finalize because `reset_player` voids only `in_progress` games. A `Voided` on a settled row is refused. Recommend deleting the bypass (D2 file; SA or Brian). |
| FIND-D3-04 | Minor (stored content) | A takeback or a plan that removes plies also deletes their `move_eval` rows, because the foreign key would refuse the delete. The SDD does not say this. |
| FIND-D3-05 | Info | `Start` creates no slot rows. A restored `in_progress` game has none until the restore's last pass (SDD 3.11.10). |
| FIND-D3-06 | Minor (format gap) | The `Start.matchup` text form is not specified (the test builder writes `human-human`, the table `hh`). The store ignores the text and derives the matchup from the two kinds. |
| FIND-D3-07 | Minor | `journal_seq` after a batch is `max(stored, last seq)`; the SDD says "set to the record's seq". The plan applier sets it exactly (rebase may lower it). |
| FIND-D3-08 | Info | A stale batch (older `seq`) replayed alone moves the game clocks back; there is no per-record seq guard (journal wins). |
| FIND-D3-09 | Minor | Live `finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player` do not flush that game's apply backlog first; a queued `Voided` behind a live-finalized game is refused and parks the game. The caller can `sync_backlog` first. |
| FIND-D3-10 | Info (for D4) | D-87 pair detection works inside one call. The scan and restore must pass the whole journal (at least a `GameEnd` with its `Voided`) in one batch. |
| FIND-D3-11 | G2 item | New public surface (`apply_records`, `sync_backlog`, `backlog_status`, `ApplyOutcome`, `BacklogStatus`; test-support `with_monotonic_clock`, `MonotonicClock`) and a private field in `StoreOptions`. `backlog_status` is not in the SDD method list. |
| FIND-D3-12 | Info | `StoreBacklog` is read by polling `backlog_status`, not pushed. |
| FIND-D3-13 | Info | The backlog is in memory. Closing with batches pending loses the queue, not the data (the journal holds every record; the next start reconciles). |
| FIND-D3-14 | Minor | The plan applier uses the connection's normal 5 s lock wait, not 100 ms. D4 decides whether it needs the short wait. |
| FIND-D3-15 | Info | `finalize.rs` (D2 code) was edited: two bodies moved into `finalize_in_tx` and `abort_in_tx`. Behavior kept; the D2 tests are unchanged and green. |
| FIND-D3-16 | Info | One baseline run in the mutation copy failed once under machine load (test not identified); three reruns and all 10 loop runs passed. Timing tests use wide bounds (90 ms to 1.5 s). |

SDD conflicts: FIND-D3-07 (set versus never lower), FIND-D3-06 (field without text form). No ADR contradicted.

## 6. Not run / not analyzed / assumed

Not run: power-loss behavior; Intel Mac; the "done-journal rule holds for a reset-voided game" clause of UT-ST-085 (journal collection, 3.11.8, is not part of D3); a Reviewer pass. Not analyzed: `void_unreadable_game` and `reset_player` interplay with a non-empty backlog beyond FIND-D3-09. Assumed: `Unavailable` and `Busy` are the only retryable errors (SDD 3.12 says "busy, locked or unavailable").

## 7. `CHESS_CI_OFFLINE=1 ci/check.sh`

Foreground, whole workspace, exit 0, "All gates passed." (fmt, build, clippy, test, chess-store `--features test-support`, crate edges, ECO checksums, cargo deny; `npm audit` skipped by the offline flag). `[RAN]`

## Rulings 2026-11-24 (decision bead chess-a0r.3.56, package `decision-package-store-D3.md`)

Engineer stance. **GATE-111 stays PENDING (Brian Prascher, G3 for stored-format and stored-value items). No approval has been received.** Brian's rulings (relayed by the coordinator, no verbatim quote) are applied in `crates/chess-store`. Not reviewed by a Reviewer. Verification: `cargo test -p chess-store` 440 tests and with `--features test-support` 515 tests (5 runs each, all exit 0 `[RAN]`); race binary (`first_launch`, 500 SIGKILL rounds per run) 10 runs all exit 0 (6 tests each, 70 to 88 s); `cargo fmt --check`, `clippy -D warnings` both feature settings clean `[RAN]`; `node ci/check-crate-edges.mjs` and `CHESS_CI_OFFLINE=1 ci/check.sh` both exit 0 (`All gates passed.`, npm audit skipped by the offline flag).

Tests were written before the code where the API existed (the four format tests failed `[RAN]` on the old code: `d3_ended_at_...`, `d3_start_wall_ms_...`, `d3_matchup_text_...`, and the property test, minimal input `ended_at = 0`). For the backlog policy the new API did not exist, so the proof that each test bites is the hand mutants below (each mutant reverts one rule and a named test fails `[RAN]`).

### 1. `ended_at` rule (G3, D-144)
- `journal/format.rs`: `Payload::validate` refuses `ended_at <= 0` for `GameEnd`, `Abort` and `Voided` at encode (`to_json`, `encode_record`) and at decode (`from_json`, `decode_frame`); `check_record` refuses `Start.wall_ms <= 0` at both (FIND-D3R-07). A CRC-valid refused tail is `UndecodableTail` damage (CH-5 B), so D4 needs no new rule: test `journal::reader::tests::d3_a_crc_valid_tail_record_that_breaks_a_value_bound_is_undecodable_tail_damage`.
- Store second line: `repo/lifecycle.rs::void_game` refuses `ended_at <= 0` (used by `void_unreadable_game`, `reset_player` and the `Voided` record); `repo/apply.rs::apply_one` refuses it for every terminal record before anything else, also a superseded `GameEnd` and a `Voided` that meets an already voided row. `apply_records` meets `ended_at <= 0` only through a record built in memory or a hostile file: typed `InvariantViolated`, whole batch rolled back (dump equal).
- Property test: `journal::format::tests::d3_the_writer_never_writes_what_the_reader_refuses_for_times_and_matchup` (encode succeeds iff the bound holds; then decode returns the same record).
- Writer: `tests/journal.rs::the_writer_refuses_a_record_the_format_refuses_and_writes_nothing` (error `WriteFailed{Io(InvalidInput)}`, no byte written, sequence number not used up).
- Other tests: `d3_ended_at_at_or_below_zero_is_refused_at_encode_and_at_decode`, `d3_start_wall_ms_at_or_below_zero_is_refused_at_encode_and_at_decode`; `tests/apply.rs`: `d3_a_voided_record_with_a_non_positive_ended_at_is_refused_and_writes_nothing`, `d3_a_game_end_voided_pair_with_a_bad_void_time_writes_nothing`, `d3_the_end_time_rule_also_covers_a_record_that_would_change_no_row`, `d3_a_start_with_a_non_positive_wall_ms_is_refused_and_creates_no_game`; `tests/lifecycle.rs::d3_live_void_paths_refuse_a_clock_at_or_below_zero`; `repo::lifecycle::tests::void_game_refuses_an_ended_at_at_or_below_zero`.
- Test fixtures changed by the ruling: `tests/common/mod.rs::jb::start` and the unit `start()` now write `matchup` `hh` / `hc` (they wrote `human-human` / `human-computer`).

### 2. Uncurable-batch policy (G3, D-145; names and bounds as in the architect's reading, Addendum N)
- A permanent error parks the game **on the first attempt** as on a retry: the batch is kept at the head, the error is returned (`thread.rs::Worker::apply`). Later `apply_records` for a parked game return `StoreError::GameParked{game}` and queue nothing (the journal is the only copy); an empty batch is `Applied` without a look. So no `journal_seq` claim over a hole (FIND-D3R-01): `tests/apply.rs::d3_a_refused_batch_parks_the_game_and_nothing_applies_over_the_hole` (probe p14: rows `[u0]`, `journal_seq` stays 2).
- Bounds (**ESTIMATE**, constants `MAX_BATCHES_PER_GAME = 256`, `MAX_PENDING_BATCHES = 4 096`, from the architect's reading of D-145, not measured): reaching the per-game or total bound answers `StoreError::BacklogFull`, parks a game that has a queue with `ParkCause::QueueFull` and drops nothing; a game with no queue only gets the error. Tests `a_game_at_its_queue_bound_is_parked_with_queue_full_and_nothing_is_dropped`, `the_backlog_as_a_whole_is_bounded_and_recovers`.
- Inspect and clear: `Store::parked_games() -> Vec<ParkedGame{game, cause: ParkCause, pending_batches, parked_for}>` and `Store::unpark_game(game) -> Option<usize>`; per D-145 unpark **drops the held queue** and clears the park; the caller then passes the whole journal in one call (idempotent). Passing only a later batch after an unpark would skip the dropped records; this is a caller contract, stated in the doc comment. `tests/apply.rs::d3_unpark_after_the_cause_is_fixed_applies_the_parked_batch`.
- `sync_backlog` returns `GameParked` at once when a game is parked (not `Busy` for ever).
- FIND-D3R-09: `error::map_sqlite` maps `NOMEM`, `INTERRUPT`, `PROTOCOL` and `SCHEMA` to `Busy` (transient); an unknown code and a panic stay `Internal` and park. `classify` is unchanged in shape; test `error::tests::sqlite_result_codes_map_to_the_error_table`, `thread::tests::classify_separates_what_a_retry_cures_from_what_parks`. The exact list for GATE-111: busy, locked (already), out of memory, interrupt, lock-protocol, schema change. Transient I/O codes already map to `Unavailable`, which is transient.
- FIND-D3R-04 (stale timer): `Backlog::push` restarts the timer when no game is waiting for a retry, parked games included (`backlog::tests::a_new_batch_behind_only_parked_games_starts_a_fresh_timer`, `apply_backlog.rs::a_game_parked_earlier_does_not_leave_a_stale_timer_to_the_next_backlog`).
- FIND-D3R-03: a waiter forces a pass every `max(100 ms, timeout / 4)` (`thread::tests::sync_backlog_succeeds_when_the_lock_is_released_inside_its_window`). For a timeout under 400 ms the interval is the 100 ms floor, because one attempt can take its 100 ms lock wait.
- FIND-D3R-08: waiters whose caller dropped the future are removed on the next loop (`a_cancelled_sync_leaves_no_waiter_behind`).
- FIND-D3R-11: every `thread.rs` test call has a 30 s real-time outer limit (`block_on` now `enable_time` + `tokio::time::timeout`), and `apply_backlog.rs` routes `sync_backlog` through `common::block_on_within` (60 s).

### 3. The rest
- `Start.matchup` is the code `hh|hc|cc` of the two kinds at encode, decode and apply (`format::matchup_code`, `apply.rs::check_start`): `d3_matchup_text_must_be_the_code_of_the_two_seat_kinds`, `d3_a_start_whose_matchup_text_disagrees_with_the_kinds_is_refused_at_apply`, `d3_a_second_start_with_a_wrong_matchup_or_a_non_positive_wall_ms_is_refused`. `journal_seq` rule unchanged (larger of stored and last `seq`).
- `BacklogStatus` is `#[non_exhaustive]`. `void_game_repair_pair` is deleted; its `UPDATE` is inline in `void_game` after the precondition; the two unit tests build the voided-and-settled row with SQL.
- D3-09: `RunFlushed` command: `finalize_game`, `abort_game`, `void_unreadable_game` and `reset_player` (the listed games) first apply that game's queued batches on the store thread; `Busy` if they cannot be applied, `GameParked` if the game is parked, and the command does not run. Tests `tests/apply_backlog.rs::d3_09_a_live_terminal_command_applies_the_queued_batches_of_its_game_first` (four commands, frozen timer), `d3_09_a_live_command_is_refused_while_the_queue_cannot_drain_or_the_game_is_parked`, `thread::tests::a_flushed_job_runs_after_the_queued_batches_of_its_game`. **Cost not measured** `[UNV]`: one extra attempt per live terminal command when a queue exists, none otherwise.
- D3-04 accepted as built; D3-10 nothing in D3.
- F2-01, F2-02, F2-03, F2-05, F2-07: see the D5, D2 and B evidence files (same date).

### Hand mutants `[RAN]`, all KILLED (37)
Item 1 (9): `GameEnd` `>` to `>=`; `Abort` rule removed; `Voided` rule weakened; `Start.wall_ms` `<=` to `<`; encode check removed; decode check removed; `void_game` `<=` to `<`; `apply_one` up-front check `<=` to `<`; `check_start` wall time removed. Item 2 (16): first attempt does not park; parked check removed; `map_sqlite` transient codes removed; per-game bound `>=` to `>`; `QueueFull` park removed; total bound `>=` to `>`; sync parked branch removed; unpark keeps the queue; `push` timer rule reverted (old `queues.is_empty()`); waiter cleanup removed; waiter forced pass removed; flush skipped; flush `Transient` ignored; `flush_game` parked early return removed; `NotFound` cause mapped to `Other`. Item 3 (12): matchup rule removed; `cc` code wrong; imported filter removed; NULL-last removed; `id` tie-break removed in `list_games` (S05) and in statistics (S02); `set_analysis_used` guard removed and inverted; flush removed from `abort`, `finalize`, `void_unreadable_game`, `reset_player` (listed games dropped). Runner: `scratchpad/sw/mut.py` (a copy of the workspace; the real tree was not touched).

Not run / not analyzed: power loss; Intel Mac; the Reviewer pass; D4 behavior (restart reconcile of a parked game) is not built. The 256 and 4 096 bounds and the `max(100 ms, timeout/4)` interval are estimates.
