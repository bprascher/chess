# Review: chess-store D3 rulings of 2026-11-24 (limited, adversarial, fresh)

| Field | Value |
|---|---|
| Stance | **Reviewer** only. Fresh invocation: I did not write the code or the earlier reviews. I read `review-chess-store-D3.md` and `review-chess-store-fixes-2.md` for what was found, and judged the code. |
| Scope | Only what the rulings changed (decision bead chess-a0r.3.56, architecture Addendum N D-137..D-153, `step2-store-D3-evidence.md` "Rulings 2026-11-24"), baseline B6. |
| Gate | GATE-111 (D3). Status of the gate itself stays **PENDING (Brian Prascher, G3)**. This review approves nothing. |
| Snapshot | Copy made 2026-11-24T01:17Z of the working tree (HEAD `63b65a0` plus the uncommitted changes). Tree hash of `crates/chess-store` excluding `proptest-regressions`: `812a4b537b251bae` (`find ... \| sort \| xargs shasum \| shasum`). At 2026-11-24T01:48Z `diff -rq` of the live `crates/chess-store` against the copy was empty, so the review covers the tree as it stands. Copy: `scratchpad/rv-d3r-10328` (workspace trimmed to `chess-rules`, `chess-domain`, `chess-store`; own `CARGO_TARGET_DIR`). Attack copy: `scratchpad/rv-d3r-attack-*`. Mutation copy: `scratchpad/rv-d3r-mut-*`. Nothing in `crates/` or any other doc was edited. |

## 0. Verdicts

| Gate | Verdict | Why in one line |
|---|---|---|
| **GATE-111 (D3, the rulings)** | **PASS WITH CONDITIONS, no Major** | The code does what the rulings say. Nothing I tried lost, half-wrote or double-applied a record, and every Major of the two earlier D3 reviews that the rulings were meant to close is closed. Open: 2 documented-but-unenforced contracts (FIND-D3RL-01, -02), a sync timeout that overshoots badly (-03), a global transient mapping with no retry cap (-04), 4 surviving non-equivalent mutants (-05). Conditions in section 8. |
| Files of GATE-108 changed by the rulings: `journal/format.rs` (and `journal/writer.rs`, `journal/reader.rs` tests) | **PASS WITH CONDITIONS stands (no new finding against it)** | `ended_at`, `Start.wall_ms` and `matchup` rules hold at encode, decode, writer, reader, apply. 9 of 9 format mutants killed. The only residual is the stored-format break for pre-ruling test journals (FIND-D3RL-08, Info). |
| Files of GATE-109 changed by the rulings: `repo/finalize/lifecycle.rs` (`void_game`, `void_game_repair_pair` removal) and `repo/game.rs::set_analysis_used` | **PASS WITH CONDITIONS stands (no new finding against it)** | `void_game` refuses `ended_at <= 0` and an empty hash (mutants killed). No caller of the removed pair is left (grep, section 3). F2-05 behaves as ruled. |
| Files of GATE-110 changed by the rulings: `repo/stats.rs` and the `ORDER BY` in `repo/game.rs` | **PASS WITH CONDITIONS stands** | F2-01, F2-02 and F2-03 behave as ruled; 7 of 7 stats and list mutants killed. Condition: FIND-D3RL-06 (the tie-break test works only because a planted index steers the query planner). |

I changed no gate state. "PASS WITH CONDITIONS" here means: no Major, no data-loss path found, conditions listed. Brian Prascher's G3 approval is still needed for the stored-format and stored-value items (`ended_at`, `Start.wall_ms`, `matchup`, park policy). I have not received it and do not claim it.

## 1. Method and independence

I read the code first (`thread.rs`, `backlog.rs`, `repo/apply.rs`, `journal/format.rs`, `journal/writer.rs`, `journal/reader.rs`, `repo/stats.rs`, `repo/game.rs`, `repo/lifecycle.rs`, `error.rs`, `store.rs`, `store/d3.rs`), then ran the standard commands myself, then wrote my own attacks (`tests/rv_d3r.rs`, kept in the attack copy only, not added to the repo), then mutated. The author's transcript was not trusted: every number below is from my own runs.

## 2. Standard commands `[RAN]` (clean copy, `CARGO_TARGET_DIR` own)

| Check | Command | Result |
|---|---|---|
| fmt | `cargo fmt -p chess-store --check` | exit 0, no output |
| clippy, default | `cargo clippy -p chess-store --all-targets --offline -- -D warnings` | exit 0, no warning |
| clippy, test-support | `cargo clippy -p chess-store --all-targets --features test-support --offline -- -D warnings` | exit 0, no warning |
| tests, default, x3 | `cargo test -p chess-store --offline` | 3 of 3 exit 0, **440 passed, 0 failed** each (sum of every `test result` line) |
| tests, test-support, x3 | `cargo test -p chess-store --features test-support --offline` | 3 of 3 exit 0, **515 passed, 0 failed** each |
| coverage | `cargo llvm-cov -p chess-store --features test-support --offline --show-missing-lines` | exit 0. TOTAL regions 93.98 %, functions 95.97 %, **lines 97.33 % (8 613 lines, 230 missed)**. Changed files, lines: `backlog.rs` 98.53 %, `thread.rs` 96.98 %, `repo/apply.rs` 97.72 %, `journal/format.rs` 99.14 %, `repo/stats.rs` 99.14 %, `repo/game.rs` 98.26 %, `repo/lifecycle.rs` 97.62 %, `store/d3.rs` 100 %. Missed lines in changed code are `?` error edges (`apply.rs` 63, 147, 168, 204, 250, 314, 356, 360, 373, 406, 489; `thread.rs` 156, 329, 551, 956, 1076-1078, 1081, 1128-1130, 1158-1160; `backlog.rs` 199, 275, 616-618; `format.rs` 396-401, 955, 1300). |

Counts match the author's 440 and 515. The full run includes the 500-round first-launch kill test (the F2-07 default): that binary took **75.8 s, 87.5 s and 92.5 s** in the three runs I timed, so the default `cargo test -p chess-store` now costs about 1.5 minutes more than before (FIND-D3RL-09).

## 3. Attack results `[RAN]` (my own tests, `tests/rv_d3r.rs` in the attack copy)

Scope item (1), `ended_at` and `wall_ms`:

| Attack | Result |
|---|---|
| SIGKILL a child at `apply.before_commit`, **400 rounds**, a parked game (no row) already in the child's backlog, batch `[Move, Move, GameEnd]` | 400 of 400: `PRAGMA integrity_check` = ok, database content byte-equal to the "before" dump (`pre=400 post=0`); reopen, apply again -> `Applied`, content equals the reference (one batch applied on a copy); a third apply changes nothing. |
| Same at `apply.after_commit`, **400 rounds** | 400 of 400: content equals the reference at once (`pre=0 post=400`), re-apply and third apply change nothing. The rating was settled once (it is in the compared dump). |
| **Random-time** SIGKILL, **400 rounds**: child opens the store, parks game 77, then applies 40 single-`Move` batches for game 1 as fast as it can; the parent kills after 0..60 ms | 400 of 400: `integrity_check` ok; the `move` rows are a contiguous prefix `1..m`; `journal_seq` is `0` when `m = 0` else `m + 1`; **no step the child printed as acknowledged was missing** after the kill (`acked < m` in every round); after reopen the whole journal applies and gives all 40 plies. Distribution: 1 round with 0 plies, 399 partial, 0 complete. |
| `GameEnd` `ended_at` = 0, -1, `i64::MIN` in a batch after a `Move`, straight to `apply_records` | each `Err(InvariantViolated)`; the `Move` before it is **not** in the database (0 rows), the game is parked with 1 held batch; `unpark_game` -> `Some(1)`. `ended_at = i64::MAX` is accepted and completes the game (no upper bound; none was ruled). |
| Writer refuses a record that the format refuses | covered by the repo test `the_writer_refuses_a_record_the_format_refuses_and_writes_nothing` (read: file bytes equal before and after, `next_seq` unchanged, next valid append gets the same seq). I did not duplicate it; mutants R08, R04, R07 attack it (killed). |
| CRC-valid tail with a broken bound -> `UndecodableTail` | read in `reader.rs:379` test (GameEnd 0, -1, Abort 0, Voided -4, Start `wall_ms` 0, long matchup); a CRC-valid broken record with an intact frame after it is `Middle` by the scan logic (`classify_tail`), matching SDD 3.11.1(1). Mutants R01-R04 killed. |

Scope item (2), the park policy:

| Attack | Result |
|---|---|
| Game with no row: first apply | `Err(NotFound)`, parked, `pending_batches 1` (the batch stays at the head). Second apply -> `Err(GameParked)`, **nothing queued** (`pending_batches` stays 1). Empty batch for the parked game -> `Ok(Applied)`. |
| Parked game and an unrelated game 1 | game 1 applies directly (`Applied`). |
| Queue behind a held lock: game 1 x3 and game 5 x2, lock released | game 1 drained (3 rows), game 5 parked `NotFound` with 2 held batches; a new batch for game 5 -> `GameParked`, backlog stays 2. |
| Bounds | 256 batches for one game queue (`Queued` each); the 257th -> `BacklogFull`, game parked `QueueFull` with 256 held; the 258th -> `GameParked`. With 15 more games x 256 the backlog is 4 096; a game with **no queue** gets `BacklogFull` three times and is **not** parked (E-03 holds); a game with a queue (101) is parked. After release one forced pass parks the 15 row-less games `NotFound` (16 parked in all). `unpark_game(1)` -> `Some(256)`, then the whole journal (260 batches' worth in one call) applies: 260 rows. |
| `sync_backlog` with a parked, **unrelated** game | `Err(GameParked{game 5})` at once although game 1 had drained (FIND-D3RL-02). |
| A waiter that is already waiting when another game parks | gets `GameParked` 332 ms after start (lock released at 300 ms). |
| `unpark_game` and what is left | the batches are dropped; **the barrier then passes** (`sync_backlog` -> `Ok`) with the poisoned record not in the database; a **later-only** batch is then `Applied` and sets `journal_seq = 7` with plies `[1,2,3,6]` (FIND-D3RL-01). |
| Timer after a park (FIND-D3R-04 fix) | author's tests plus mutant R26/R27/R28/R30 killed; I did not find a stale timer. |
| Transient mapping | unit table covers the four codes; mutants R53, R54, S57, S58 killed. I could **not** induce a real `SQLITE_SCHEMA`, `NOMEM`, `INTERRUPT` or `PROTOCOL` `[UNV]`. By reading `backlog.rs::pass`, a batch that keeps failing `Busy` (or `Unavailable`) is retried for ever at 5 s and **stops the pass for every game after it** (`return` on the first transient, games in uuid order) (FIND-D3RL-04). |
| `sync_backlog` timeout accuracy under a foreign lock (50, 100, 200, 400, 1000 ms) | returned after **361, 356, 348, 671, 1 133 ms** (FIND-D3RL-03). |
| Panic inside apply | read: `Slot::run` catches it, the attempt is `Internal`, which is permanent: the game parks, the connection reopens once, a second panic leaves `conn = None` and every later attempt is `Unavailable` (transient), so `apply_records` keeps answering `Queued`. The repo test `a_panic_in_an_attempt_is_internal_and_the_thread_survives` covers the first part; I could not inject a panic through `Store` `[UNV]`. |

Scope item (3), D3-09 and the rest:

| Attack | Result |
|---|---|
| Deadlock | None possible by construction: `flush` runs on the store thread and calls the applier directly (`thread.rs:389`); there is no channel wait inside the thread. |
| `abort_game` with 3 queued batches while a foreign process-style lock is held | `Err(Busy)` after **117 ms** (the 100 ms apply wait; not 5 s). |
| `reset_player` listing **300** games (no queue, no rows) while the lock is held | `Err(Busy)` after **5.20 s**: the flush of 300 queue-less games costs nothing, then the job itself waits the normal 5 s lock. That is the pre-existing live-write contract, not new, but it holds the store thread for 5 s. |
| Parked game and live commands | `void_unreadable_game` and `reset_player` (game listed) -> `GameParked`; after `unpark_game` the void succeeds. So the repair command for a broken game is refused until the queue is dropped (FIND-D3RL-07). |
| Two `Store`s, one process, two threads, 100 single-ply batches each, distinct plies | 200 rows, `journal_seq` 201, both `sync_backlog` `Ok` within the limit. |
| Two **processes** with a third holding `BEGIN IMMEDIATE`: child applies 40 plies (queued), parent queues 30 | parent `sync_backlog(400 ms)` -> `Busy` (671 ms); after release both backlogs drain (child's in 315 ms), 70 distinct plies. Note: the child exits with an in-memory backlog lost if it ends first; recovery is D4 (FIND-D3-13, already known). |
| `void_game_repair_pair` | `grep -rn void_game_repair_pair crates` finds only a comment (`tests/apply.rs:604`): no code, no caller. The D-87 pair is still covered by `d87_a_game_end_followed_by_voided_is_not_finalized` (mutant S18, the pair not superseded, is killed by it). |
| F2-01 | `stats.rs` filters `g.status <> 'imported'` in the SQL for `stats_rows` and `head_to_head_rows` (one `SELECT`); `games_for_history` uses `list_games` and keeps imported games by design. Mutants R49 (filter on the wrong status) killed. |
| F2-02 | `list_games` orders `g.started_at IS NULL, g.started_at, g.id`; R51 (NULL first) killed. |
| F2-05 | `set_analysis_used`: final game and flag already set -> `Ok`; flag unset -> `InvariantViolated`; R52/S52 killed. |
| F2-07 | `first_launch.rs` default 500 rounds, env `CHESS_STORE_KILL_ROUNDS` overrides. Ran in every full run. |

## 4. Findings

Severity scale as in the earlier reviews. **No Major.**

### FIND-D3RL-01 [Minor][CONFIRMED] After `unpark_game` the store lets a later-only batch through and the barrier passes over the hole
- Location: `backlog.rs::unpark` (drops the queue), `thread.rs::Worker::apply`, `repo/apply.rs:114` (`journal_seq = max(stored, last seq)`), `store/d3.rs` doc comment.
- Evidence `[RAN]` `rv_hole_after_unpark_and_barrier`: plies 1 to 3 applied, a poisoned batch (ply 4) parks the game; `unpark_game` -> `Some(1)`; `sync_backlog` -> `Ok(())` (the barrier says the database is current); then `apply_records` of only ply 6 -> `Applied`; stored `journal_seq = 7`, rows `[1, 2, 3, 6]`.
- Impact: the stored `journal_seq` claims records 5 and 6 although ply 4 and 5 are absent, and an export barrier would pass. The ruling (D-145, E-02) chose "caller passes the whole journal" and the doc comment says so. The journal still holds every record and D4's reconcile (journal wins) heals it, so nothing is lost. But the contract is unenforced and **no test pins it**; the first-record `seq` is available, so a guard of the form "a batch whose first `seq` is above the stored `journal_seq` + 1, for a game that has had a drop" is possible, though the Start batch (live `start_game` leaves `journal_seq` 0 and the first apply is seq 2) needs care.
- Close by: either a test that pins the hole as the accepted behaviour with an explicit comment, or a refusal. Needs Brian's decision (it is the ruled design), so Minor, not Major.

### FIND-D3RL-02 [Minor][CONFIRMED] `sync_backlog` is global: any parked game, related or not, fails every caller's barrier, and the error names only the first one
- Location: `thread.rs::settle_waiters` (`first_parked()`, `backlog.rs:136`).
- Evidence `[RAN]` `rv_park_semantics`: game 5 parked, game 1 applied and drained, `sync_backlog` -> `GameParked{5}`.
- Impact: for the purpose the SDD gives it (a barrier before an export, 3.11.9) "any parked game" is the right answer, because the database lags the journal for that game. For any other caller (a caller that only cares about its own game) one bad game blocks every barrier until `unpark_game` or restart; the answer carries one game id, so a caller with several parked games finds them one call at a time (`parked_games()` exists and lists all). Not a defect against the ruling; recorded because the question was asked. No code change needed if documented as "barrier for the whole store".

### FIND-D3RL-03 [Minor][CONFIRMED] `sync_backlog(timeout)` returns long after `timeout` under a lock
- Evidence `[RAN]` `rv_sync_timeout_overshoot`, foreign lock held: timeout 50 ms -> 361 ms; 100 -> 356; 200 -> 348; 400 -> 671; 1000 -> 1 133.
- Cause `[INF]`: the first forced pass runs before the deadline is computed (`deadline_ms = now_after_pass + timeout`, `thread.rs:468`) and each forced pass takes its 100 ms lock wait; a timer pass and a forced pass can both run before the deadline check. A caller with a 50 ms budget waits seven times that.
- Impact: callers that rely on the timeout for UI responsiveness. Close by computing the deadline before the first pass and a test that bounds the overshoot (for example `elapsed <= timeout + 2 * 100 ms`).

### FIND-D3RL-04 [Minor][CONFIRMED by reading, not reproduced] Transient mapping is global and uncapped, and one transient game stops the pass for the games behind it
- Location: `error.rs:197-210` (`map_sqlite`: NOMEM, INTERRUPT, PROTOCOL, SCHEMA -> `Busy`), `backlog.rs::pass` (`return` at the first `Transient`), `thread.rs::apply` (`has_retrying()` queues every other game's batch).
- Evidence: unit table and mutants R53, R54, S57, S58 killed `[RAN]`. A real SQLite code of those four kinds could not be induced `[UNV]`.
- Impact: (a) `map_sqlite` is used by every store call, so `finalize_game`, reads and the rest now answer `Busy` for those codes too; callers read `Busy` as "another process holds the lock". (b) There is no retry limit: a batch that keeps failing with one of these codes is retried every 5 s for the life of the process; the pass stops at it, so later games (uuid order) are never attempted, and `has_retrying()` makes their new batches queue as well (up to 256 per game, then `QueueFull`). The failure is bounded only by the queue bounds. SQLite retries `SQLITE_SCHEMA` itself inside `prepare_v2`, so this is unlikely in practice. Close by: a retry cap per head batch (then park with a cause), or a test with an injected code that shows a second game is still served.

### FIND-D3RL-05 [Minor][CONFIRMED] Four non-equivalent mutants survive (test gaps)
See section 6. R17: a second `Start` in a batch that names another game is accepted silently (`apply.rs` Start arm, the `game` check is untested). R29: `flush_game` success does not reset the retry delay (no test). R37 and R48: the waiter's forced-pass interval can be made shorter (`min` instead of `max`, or every loop) and every test still passes, so a hot loop of 100 ms passes at 100 % duty cannot be caught. Close by one test each.

### FIND-D3RL-06 [Info][CONFIRMED] The F2-03 tie-break tests detect a missing `id` tie-break only because of the planted expression indexes
- Evidence `[RAN]`: with `, g.id` removed from `stats.rs` and `game.rs` **and** the index helper `scan_ties_in_descending_id_order` replaced by `SELECT 1`, the whole `stats_rows` binary passes (16 passed). With the helper in place the same mutants (T01 stats, T02 list) are killed by `f2_03_equal_start_times_...`.
- Judgement: honest. The helper does not change the code under test; it makes SQLite scan equal start times in the wrong order so a missing tie-break shows. It cannot make a correct query fail (the correct `ORDER BY` cannot use those indexes for its order). The cost: it depends on planner choice, so a future SQLite that plans differently would silently weaken the test, never produce a false failure. Recorded so nobody reads "tie-break tested" as "tie-break proven".

### FIND-D3RL-07 [Info][CONFIRMED] The command meant for a broken game is refused while the game is parked
- Evidence `[RAN]` `rv_void_on_parked_game`: parked game: `void_unreadable_game` and `reset_player` (game listed) -> `GameParked`; after `unpark_game` the void works. This is the ruling (D3-09: `GameParked` if the game is parked). It is surprising because a parked game is typically a broken one (NotFound, Corrupt, InvariantViolated) and the void is the repair. Not tested together; consider an `unpark`-then-void test and a line in the UI contract.
- Also `[RAN]`: a `Move` batch for a game that was voided is `Applied` (rows written under a terminal game): FIND-D3R-06 of the earlier review, still as documented.

### FIND-D3RL-08 [Info][INF] Stored-format break and the error class of a format refusal
- `Start.matchup` is now only `hh`, `hc`, `cc`; the earlier test builders wrote `human-human`. Any journal written by the pre-ruling build is `UndecodableTail` or `Middle` damage. No such journal exists outside test fixtures (project is pre-release), so no migration is needed, but nothing in the SDD states the compatibility decision. G3 approval covers it.
- The writer reports a format refusal as `WriteFailed{Io(InvalidInput)}` (`journal/writer.rs:277`), an I/O error class for a caller bug. `chess-app` must not treat it as a disk fault. For the app SDD.

### FIND-D3RL-09 [Info][CONFIRMED] Suite cost
The default `cargo test -p chess-store` now includes a 500-round kill test: 75.8 s, 87.5 s and 92.5 s in my three runs for that binary alone. The env override exists. CI budget is a matter for the PM.

### FIND-D3RL-10 [Info][INF] Total bound can be exceeded by one per new game
`thread.rs::apply`, `Attempt::Transient` branch pushes without the bound check when no game is retrying (all queued games parked). Each such game adds one batch past 4 096, then its own later batches hit the check. Not reproduced (needs 16 parked full queues). No impact beyond a few batches.

## 5. Answers to the specific questions

| Question | Answer |
|---|---|
| `sync_backlog` -> `GameParked` when ANY game is parked: right if the parked game is unrelated? | Right for a store-wide barrier (the DB lags for that game); wrong for a per-game caller. As built it is store-wide, with the first parked game's id in the error. FIND-D3RL-02. |
| Is SCHEMA/INTERRUPT -> `Busy` retried for ever? Livelock, starvation? | Retried for ever, at 5 s once the delay has doubled. No livelock of the thread (it still serves commands every <= 100 ms), but head-of-line blocking for later games and a global effect on every call's error class. Not reproducible without injecting a real code. FIND-D3RL-04. |
| Panics inside apply park as `Internal`? | Yes by reading and by the repo test; after a second panic the store answers `Queued` for ever while nothing applies (until the 256 and 4 096 bounds). |
| Timer restart, forced pass `max(100 ms, timeout/4)`, cancelled waiters, lock wait back to 5 s | Timer: killed mutants R26-R28, R30. Forced pass: works (waiter test at 332 ms) but the interval can be shortened with no test failing (R37, R48) and the timeout overshoots (FIND-D3RL-03). Cancelled waiters: author's test; the `retain(!is_closed)` mutant was killed by them (author's list; I did not re-run it). 5 s restore: R15 (S15) killed. |
| Deadlock in D3-09; 100 ms vs 5 s; `reset_player` with many games | No deadlock path. 100 ms for the flush (117 ms measured). `reset_player` of 300 queue-less games under a lock: 5.2 s, the pre-existing live-write wait. |
| `Start.matchup` at encode, decode, apply | Killed: R05, R06, R09, R10. One gap: R17. |
| `void_game_repair_pair` removal | No caller; pair still covered. |
| F2-01 to F2-05 | As ruled; FIND-D3RL-06 for the tie-break test. |
| Kill rounds 500 default | Yes, with the 75-92 s cost. |

## 6. Hand mutation: 69 valid new mutants over apply, backlog, thread, format, stats, game, lifecycle, error

Method: one mutant at a time on a private copy; run `cargo test -p chess-store --features test-support --lib --test apply --test apply_backlog --test lifecycle --test journal --test stats_rows --test games --test players --test boundary` (baseline green first: 145 lib tests, 38, 12, 6, 19 ... all ok). Runner `scratchpad/rv_mut.py`. 5 mutants did not compile (unused variable under `-D warnings`) and were re-run as build-fixed variants (S11, S15, S18, S22, S52); S65 (unused import) was replaced by S65b. Some mutants overlap boundaries the author also hit (R19, R20); the rest are not in the author's list.

Result: **69 valid mutants: 62 killed, 7 survived (3 equivalent, 4 real gaps)**.

| ID | File: mutation | Result |
|---|---|---|
| R01 | format: Abort `ended_at > 0` -> `>= 0` | KILLED `d3_ended_at_at_or_below_zero...` |
| R02 | format: Voided `> 0` -> `>= 0` | KILLED |
| R03 | format: Voided unchecked | KILLED |
| R04 | format: `wall_ms` rule applied to `Move` instead of `Start` | KILLED |
| R05 | format: `hh` code -> `hc` | KILLED `d3_matchup_text_...` |
| R06 | format: mixed code -> `cc` | KILLED |
| R07 | format: decode without `check_record` | KILLED `d3_start_wall_ms_...` |
| R08 | format: encode without `check_record` | KILLED |
| R09 | format: Start matchup unchecked in `validate` | KILLED |
| R10 | apply: `check_start` matchup test inverted | KILLED |
| S11 | apply: `journal_seq` may be lowered | KILLED `a_batch_never_lowers_the_journal_sequence` |
| R12 | apply: duplicate `seq` accepted (`>=` -> `>`) | KILLED |
| R13 | apply: ply base `+1` -> `+0` | KILLED |
| R14 | apply: already voided row refused | KILLED |
| S15 | apply: short wait never restored | KILLED `the_busy_timeout_is_restored...` |
| R16 | apply: 100 ms -> 2 000 ms | KILLED `an_apply_attempt_waits_at_most_100_ms...` |
| **R17** | apply: second `Start` need not name this game | **SURVIVED** (FIND-D3RL-05) |
| S18 | apply: D-87 pair not superseded | KILLED `d87_a_game_end_followed_by_voided...` |
| R19 | backlog: per-game bound 255 | KILLED |
| R20 | backlog: total bound 4 095 | KILLED |
| R21 | backlog: `has_retrying` counts parked games | KILLED |
| S22 | backlog: parked age wrong | KILLED |
| R23 | backlog: `park` without a queue | KILLED |
| R24 | backlog: `unpark` without `reset_when_empty` | SURVIVED, **equivalent** (the next `push` into an empty backlog resets delay and timer itself) |
| R25 | backlog: `reset_when_empty` keeps the delay | SURVIVED, **equivalent** (same reason) |
| R26 | backlog: first retry immediate | KILLED |
| R27 | backlog: forced pass moves the timer | KILLED |
| R28 | backlog: delay cap 10 s | KILLED |
| **R29** | backlog: `flush_game` success keeps the delay | **SURVIVED** (FIND-D3RL-05) |
| R30 | backlog: transient does not stop the pass | KILLED |
| R31 | backlog: StoreBacklog boundary `>=` -> `>` | KILLED |
| R32 | backlog: flush permanent reported transient | KILLED |
| R33 | backlog: oldest age uses `min` | KILLED |
| R34 | backlog: `wait_ms` counts parked | KILLED |
| R35 | thread: `POLL_MS` 1 000 | KILLED |
| R36 | thread: waiter deadline `>=` -> `>` | KILLED |
| **R37** | thread: forced interval `max` -> `min` | **SURVIVED** (FIND-D3RL-05) |
| R38 | thread: Corrupt cause -> Other | KILLED |
| R39 | thread: `Unavailable` permanent | KILLED |
| R40 | thread: empty batch not short-circuited | KILLED |
| R41 | thread: no load rule (`has_retrying`) | KILLED |
| R42 | thread: permanent batch not kept | KILLED |
| R43 | thread: parked answered before empty | SURVIVED, **equivalent** (a parked game always has a queue, so the backlog cannot be empty) |
| R44 | thread: two restarts after panic | KILLED |
| R45 | thread: flush ignores parked | KILLED |
| R46 | thread: job runs after a refused flush | KILLED |
| R47 | thread: `Sync` makes no immediate pass | KILLED |
| **R48** | thread: waiter forces a pass every loop | **SURVIVED** (FIND-D3RL-05) |
| R49 | stats: imported filter on the wrong status | KILLED `f2_01_...` |
| R50 | stats: window `>=` -> `>` | KILLED |
| R51 | game: NULL `started_at` first | KILLED `f2_02_...` |
| S52 | game: `set_analysis_used` on a final game always refused | KILLED `f2_05_...` |
| R53 | error: SCHEMA -> Internal | KILLED `sqlite_result_codes_map_to_the_error_table` |
| R54 | error: NOMEM -> Internal | KILLED |
| R55 | stats: order DESC | KILLED |
| R56 | lifecycle: `void_game` empty hash accepted | KILLED |
| S57 | error: INTERRUPT -> Internal | KILLED |
| S58 | error: PROTOCOL -> Internal | KILLED |
| S59 | thread: NotFound cause -> Other | KILLED |
| S61 | apply: up-front `ended_at` check disabled | KILLED `d3_the_end_time_rule_also_covers_a_record_that_would_change_no_row` |
| S63 | stats: head-to-head other-side opponent wrong | KILLED |
| S64 | game: list window `>=` -> `>` | KILLED |
| S65b | thread: total-bound check disabled | KILLED |
| S66 | thread: `QueueFull` park removed | KILLED |
| S67 | thread: `unpark` count off by one | KILLED |
| S68 | backlog: pass attempts parked games | KILLED |
| S69 | apply: strict-order check disabled | KILLED |
| T01 | stats: `id` tie-break removed | KILLED (needs the index helper, FIND-D3RL-06) |
| T02 | game: `id` tie-break removed | KILLED (same) |

Not counted: R11, R15, R18, R22, R52, S65 (build errors, replaced by the build-fixed variants above).

## 7. Conformance statement

Tool-checked: `rustfmt --check`; `clippy -D warnings` on all targets with and without `test-support`; rustc warnings as errors through the workspace lints. **No SEI CERT conformance is claimed**; the profile (SWE-001) applies CERT by intent only. Manual review, not a conformance claim: error handling (no swallowed error found in the changed code; `let _ = reply.send` is deliberate, a caller that stopped waiting), no `unwrap` in non-test code of the changed files (clippy passes with the repo lint set), concurrency (single writer thread, no lock held across an await in the changed code). Not checked: `cargo-deny` and `cargo-audit` (no dependency changed in this scope), miri, sanitizers, fuzzing.

## 8. Conditions for GATE-111 (for the owners; none is an approval)

1. Brian Prascher decides the G3 items (stored format `ended_at`, `Start.wall_ms`, `matchup`, park policy) and FIND-D3RL-01 (hole after `unpark_game`: pin or refuse).
2. Tests for the four live survivors (FIND-D3RL-05) and a bound on the `sync_backlog` overshoot (FIND-D3RL-03).
3. A decision on the retry cap for a persistent transient batch (FIND-D3RL-04), or a recorded acceptance.
4. SDD note (G0) for FIND-D3RL-02, -07, -08.

## 9. Not run, not analyzed, assumptions

- Not run: power-loss behavior; Intel Mac; miri, sanitizers, fuzzing; `cargo-deny`/`cargo-audit`; `ci/check.sh`; any real `SQLITE_SCHEMA`, `NOMEM`, `INTERRUPT` or `PROTOCOL` (could not be induced); a panic injected through `Store` (read and unit test only); 16 full parked queues of the "+1 over the total bound" case; a process killed with a parked game at **both** failpoints in the same child (the parked game in my kill tests is a row-less game, the kill test of the park state with a real queue behind a lock was not run); `chess-app`, D4 recovery (not built).
- Not analyzed: D2 `finalize` rating math (unchanged); the 100 ms wait versus a slow disk (timing only measured on this Mac, arm64, macOS).
- Assumed: my attack tests live only in the attack copy; the repo is untouched. The Mac was running other agent jobs during some runs; the mutation runs used `--test-threads=8`; every KILLED line names a failing test and the baseline in the mutation copy was green before the run. Timing numbers (FIND-D3RL-03) are from one machine and may vary.
- Mermaid: none in this document.

## 10. Status block

```
Mode: Review | Gate: GATE-111 (G3, Brian Prascher, PENDING; not approved by this review) | Confidence: High on the apply/backlog/format code, Medium on the transient-mapping effects (not induced)
Delivered: docs/engineering/review-chess-store-D3-rulings.md
Ran: fmt --check ok; clippy -D warnings x2 ok; cargo test 440 x3 and 515 x3 all ok; llvm-cov lines 97.33 %; SIGKILL 400 + 400 + 400 rounds ok; 69 valid mutants: 62 killed, 3 equivalent, 4 real survivors
Not run / not analyzed: see section 9
Open findings: FIND-D3RL-01..05 (Minor), -06..-10 (Info). No Major.
Needs a human: Brian Prascher (G3 approval; decision on FIND-D3RL-01 and -04)
Next step: close the four survivors with tests and bound the sync overshoot, then hand the package to Brian
```
