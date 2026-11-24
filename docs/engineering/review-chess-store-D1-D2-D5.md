# Review: chess-store parts D1, D2 and D5 (adversarial, Reviewer stance)

Reviewer pass 2 of 2 on chess-store, fresh invocation. I did not write the code and did not read the authors' claims until the end. Nothing here is an approval; GATE-107, GATE-109 and GATE-110 stay PENDING.

## 0. Snapshot and scope

| Item | Value |
|---|---|
| Snapshot taken | 2026-11-22T22:43:48Z, from the live tree at git HEAD `07a42b5` plus the uncommitted working tree |
| `crates/chess-store` tree hash | `4af9a9d1eda4babd` (first 16 hex of `sha256` over the sorted per-file `shasum -a 256` list, run in the crate directory, 64 files). A `diff -rq` of `src/` against the live crate at the end of the review printed nothing, so the live source did not change while I worked |
| Review copy | `scratchpad/rev_d125/` (crates chess-store, chess-domain, chess-rules, workspace manifests, `third_party`, docs). Workspace `members` cut to those three crates in the copy only. Own `CARGO_TARGET_DIR` per activity |
| Scratchpad note | the session scratchpad is shared; another agent had already created `scratchpad/snap`. I stopped using `snap` after seeing a foreign `TREEHASH` file there. All my work is in `rev_d125`, `probe`, `mutd125`, `mw1..mw4`, `covd125` and `tgt_*` |
| Scopes judged | allocation section 4.1 (D1), 4.2 (D2), 4.5 (D5) only, plus the 2026-11-22 rework (variation key `(game, from_ply, pgn)`, `fault.rs` canonical paths, ordering pin) |
| Brian's rulings applied | abort text `aborted`; `VoidSpec.game`; failpoints `abort./void./reset.before_commit`; store orders rating rows by `id`; F-A04 (NULL-game rows are points, `k` = 0); variation identity `(game, from_ply, pgn)`, save idempotent; `StatsGameRow` store-owned |
| Stance | Findings only. I changed no file under `crates/`, no other doc, made no commit, push or bead change |

## 1. Verdicts

| Part | Gate | Verdict | Reason in one line |
|---|---|---|---|
| **D1** FaultPlan and crash harness | GATE-107 | **PASS WITH CONDITIONS** | The feature-off claim holds and the harness is robust. Nine registered names have no call site, a few path shapes make a plan silently never fire, and the `!` parser branch is pinned only by part B. All Minor |
| **D2** finalize, abort, void, reset, invariant | GATE-109 (G3) | **PASS WITH CONDITIONS, one Major that blocks approval** | The write path is correct: 3,675 Elo vectors match an independent implementation, kills at every `before_commit` leave the database unchanged. `check_rating_invariant` returns false violations under a concurrent finalize (FIND-D2R-01, Major) |
| **D5** stats, evals, variations, jobs | GATE-110 | **PASS WITH CONDITIONS** | No defect found in the semantics; all 36 D5-area mutants were killed. Conditions are Minor: no size cap on `pgn`, query plans that scan, an ordering claim ("start order") that is really insertion order, a boundary scanner that skips the D5 files |

"PASS WITH CONDITIONS" for D2 is not a soft pass: GATE-109 should not be approved until FIND-D2R-01 is fixed and a concurrency test exists.

## 2. Findings

Severity: **Major** = wrong result in delivered behaviour that a consumer will hit, or a claim in the acceptance list that is false. Minor = defect or gap with a workaround or a bounded effect. Info = observation.

### Major

**FIND-D2R-01 (Major, D2). `check_rating_invariant` reads without a snapshot and reports violations that do not exist while a finalize commits.**
- Location: `src/repo/invariant.rs:74-107` (and `rows()` at 20-42). For each Player and track it runs `SELECT strict_elo` as one statement, then `SELECT ... FROM rating_history` as a second. Both run on a pooled read connection in autocommit, so a finalize that commits between the two statements gives `stored` from one state and the rows from another. `Store::check_rating_invariant` (`src/store.rs:357`) goes through the read pool. `get_game_analysis` in D5 does use `unchecked_transaction()`; D2 does not.
- Evidence `[RAN]`: probe `p_invariant_torn_read_stress` (`scratchpad/probe/crates/chess-store/tests/zz_review.rs`). One thread finalizes 300 games (human against computer, Strict) in a loop; another calls `check_rating_invariant(vec![pid(1), pid(10)])` in a loop. Result: `torn-read: 25 false violations in 31023 checks`. Sample: `CurrentMismatch { expected: 1156, found: 1150 }`, then `{ expected: 1122, found: 1156 }`. A final check after the writer stopped was clean (the single-threaded tests pass), so the data is right and the checker is wrong.
- Impact: the checker exists to detect corruption (ELO-1, startup, restore, tests). A false positive under load is either an alarm the user cannot act on or, if the app repairs on a violation, damage. No test runs the checker next to a writer. The D2 evidence says concurrency was not examined.
- To close: run the whole check inside one read transaction (the pattern of `get_game_analysis`), and add a test that finalizes in a loop on one handle while another checks; it must show zero violations over at least several thousand checks. Pre-fix the probe above is a failing test.

### Minor

**FIND-D2R-02 (Minor, D2). `finalize_game` overwrites `analysis_used` (confirms the authors' FIND-D2-05).**
`finalize.rs:181` writes `i64::from(req.analysis_used)`. Probe `p_analysis_used_or_vs_overwrite`: `set_analysis_used(game)` then `finalize_game` with `analysis_used = false` prints `analysis_used after finalize(false) ... : false`. The flag set during play is lost if the `GameEnd` record did not carry it. The SDD says the request carries the value, so this is by design, but the design makes `set_analysis_used` ("idempotent") unreliable. Write the OR, or state in the SDD that `set_analysis_used` is only for games the `GameEnd` will also describe. Decision needed from the architect.

**FIND-D2R-03 (Minor, D2). Terminal-state data is not validated.**
Probe `p_finalize_inputs_not_validated`: `finalize_game` accepts `ended_at = -5` (before `started_at = 5001`) and an empty `termination`; `final_journal_seq` may go backwards; a hash is only checked to be non-empty. `abort_game` accepts a human-against-human game (probe `p_abort_and_finalize_paths`: `Applied`), although the requirements say only a computer-against-computer watcher can abort. The store trusts the caller by design (D-42); a restore from a damaged journal is the one caller that cannot be trusted. Decide where the rule lives (store CHECK, restore path, or the app) and test it there.

**FIND-D2R-04 (Minor, D2). Surviving mutants that are real test gaps.** See section 4. `abort_game` of a `voided` game must answer `AlreadyFinal` (architecture 7.4) and `void_unreadable_game` of a `completed`, `aborted` or `voided` game must also; no test covers either (M14, M25). My probe `p_void_unreadable_on_completed_is_noop_and_reset_skips_completed` shows the code is right today; the tests would not notice if it broke. N01: `INSERT OR IGNORE INTO analysis_job` in `release_and_queue` can be changed to a plain `INSERT` with no test failing, because the case it exists for (a void after a finalize) is only reachable from D3.

**FIND-D2R-05 (Minor, latent for D3). `void_game` has no precondition (confirms FIND-D2-09, with a demonstration).**
In-crate probe `void_game_has_no_precondition` (unit test in the probe copy, `src/repo/lifecycle.rs`): a `completed` row run through `void_game` ends `status=voided result=None white_elo_after=Some(1204)`, and the Player's `strict_elo` stays 1204 (printed `VOID-OF-COMPLETED`). The settled rating, the elo columns and the rating rows remain for a game that now claims to be void. No public path reaches it today (`reset_player` and `void_unreadable_game` check status first; `void_game` is `pub(crate)`). D3's `apply_records` will be the first caller that can; D3 acceptance must contain a test that a `Voided` record on a `completed` row is refused or defined.

**FIND-D2R-06 (Minor, D2). The invariant checks ratings only.**
It does not compare `*_rated_games` to the number of game rows, does not check the `k` of game rows, and does not compare the `GAME.*_elo_before/after` columns to the rating rows (FIND-D2-10 confirmed). An unknown Player anywhere in the list makes the whole call `NotFound` and discards the violations already found for the others (probe `p_invariant_unknown_player_and_dup_players`). A wrong K counter would produce a wrong K for the next game and no check would notice.

**FIND-D1R-01 (Minor, D1). Nine registered failpoints have no call site, and acceptance (2) is met only through `trigger`.**
`[RAN]` script over `fault.rs` and the sources: 43 names registered, 34 wired, 9 not: `apply.after_commit`, `apply.before_commit`, `backup.after_copy`, `finalize.after_commit_before_rename`, `finalize.after_journal_end`, `journal.rebase.db_committed`, `reset.after_commit_before_voided_record`, `restore.after_replay`, `restore.before_rename`. `install` accepts any of them, nothing fires, and `hits()` stays 0. "Reachable by a test" is true only because the test calls `trigger` directly. The SDD names `backup.after_copy` and part A has the backup code, so that one falls inside the D1 scope ("wire ... where the SDD names them"). The other eight belong to B, D3 and D4 and need to be on those tasks' acceptance lists. A test that every registered name is either wired or listed as pending would stop this drifting.

**FIND-D1R-02 (Minor, D1). Two path shapes make a plan silently never fire; the path logic has no signal at install.**
`canonical()` (`fault.rs:210-225`) walks up to the first existing ancestor. `[RAN]` probe `zz_fault_canonical` (`tests/zz_review2.rs`):
- symlinked root against real context, both directions: fires (good); an upper-cased spelling of an existing directory on case-insensitive APFS: fires (macOS `canonicalize` normalises case of existing parts);
- a root with `..` after a non-existent component (`real/nope/../sub`): `file_name()` is `None` for `..`, so the function returns the path unresolved and the plan never matches (`fires=false`);
- a root that does not exist at install and later becomes a symlink: never fires;
- a relative root that does not exist yet against an absolute context: never fires;
- the non-existent tail keeps the case as given, so two spellings of a tail that does not exist yet do not match (reasoned, not run);
- two plans installed for the same root: the last one wins, the first guard's `fired` stays 0, no error (`g1 fired 0 g2 fired 1`).
None of this affects the current tests (they use `TempDir`s that exist). It matters because the failure mode is a test that passes because nothing happened. Suggest: `install` refuses a root that does not exist and contains `..`, or returns an error for a duplicate root; keep the existing `hits()` assertions as the norm in every failure test.

**FIND-D1R-03 (Minor, D1). `FaultPlan::parse` once-suffix `!` is tested only by part B.** Mutant M94 (`Some(rest) => (rest, true)` changed to `false`) survives `fault_injection`, `crash`, `lifecycle_faults` and `d5_faults`, the D1 and D2/D5 suites. I then ran it against `journal_faults` and `journal_crash`: killed by `ct_st_087_a_cleaned_up_failure_then_a_kill_leaves_no_duplicate_and_no_damage` `[RAN]`. So the behaviour is pinned, but D1's own test file does not pin its own parser; if part B's tests are reworked the guard goes. Add a parse test for `!` and `@n!` in `fault_injection.rs`.

**FIND-D1R-04 (Minor, D1/process). The five failure and crash test files compile to zero tests without `--features test-support`.** `[RAN]`: default `cargo test -p chess-store` runs 321 tests, with `crash`, `d5_faults`, `fault_injection`, `journal_crash`, `journal_faults` reporting `0 passed`; with the feature 378. `ci/check.sh` lines 49-52 run both, so the gate is real; a developer who runs only the default command sees green with no failure injection. Say so in the crate README or `lib.rs`.

**FIND-D1R-05 (Minor, D1/D2/D5). No kill test exists for any D2 or D5 command.** The harness (`tests/crash.rs`, `tests/common/child.rs`) is used only for open and upgrade. D2 and D5 use error injection at `*.before_commit`, which proves rollback in the same process but not that a process killed at that point leaves a database that opens clean. I ran it: `zz_kill_at_each_d2_failpoint_and_reopen` kills a child at `finalize.`, `abort.`, `void.` and `reset.before_commit`, then compares every table (excluding the child's `process_heartbeat` row) with the state before, reopens, runs `check_rating_invariant`, and repeats the command: all four unchanged, invariant clean, retry succeeds. The result is good; it should be a committed test (the harness is ready for it).

**FIND-D5R-01 (Minor, D5). `pgn` has no size bound and a large one stalls every writer.**
Probe `p_variation_huge` (debug build): a 64 MiB `pgn` is accepted; `save_variation` took 8.39 s on the single store thread, during which no other write runs; the repeat took 26 ms. NUL, unicode (NFC and NFD stay separate keys) and `'; DROP TABLE variation; --` are stored and compared exactly (probe `p_variations_edge_cases`; a text sharing a prefix before a NUL is a different key). The app SDD limits PGN import to 10 MB, but the store is the last guard. Add a cap (an `InvariantViolated`/validation error above a stated size) or record that the app owns it.

**FIND-D5R-02 (Minor, D5). Several main queries scan; one index is missing.**
`EXPLAIN QUERY PLAN` `[RAN]` (probe `p_explain_plans`, parameters bound as NULL):
- `stats_rows` and `head_to_head_rows`: `SCAN g` then primary-key lookups of both players. The `OR` over `white_id`/`black_id` and the `?n IS NULL` branches defeat `game_white` and `game_black`, so cost grows with all games, not the Player's games.
- `variation` lookup, delete and list: `SCAN variation` (no index on `game_id`; list adds a temp B-tree).
- `claim_job`: `SCAN j` in `game_id` order, passing over every `done` job before a queued one.
Measured, release, 50,000 games and jobs and variations: `claim_job` on an empty queue 1.86 ms; `save_variation` 10.5 ms; `list_variations` 1.05 ms; `stats_rows` for a Player with no games 2.41 ms. At the stated scale (BM-ST-03, 10,000 games) the figures are fine: three re-runs, release, gave `stats_rows` 19.9 to 24.5 ms for 10,000 rows. This is a scaling note, not a failure. An index on `variation(game_id)` is a stored-format change (G3), so it needs Brian; do not add it quietly.

**FIND-D5R-03 (Minor, D5). The code and docs say "start order"; the order is insertion order.**
`stats.rs` and `list_games` use `ORDER BY g.id`; `d5.rs` documents "in start order". `GAME.id` follows insertion. A game restored from a journal (D3: `Start` creates the row when absent) or imported gets a later `id` than games that started after it. With `limit` keeping the first N (FIND-D5-04, the authors' finding, confirmed: probe `zz_stats`, `limit 2 -> started_at [100, 200]`), the "oldest N" can be the wrong set. Sort by `(started_at, id)` or document insertion order and name `limit` as "first N inserted". Also settle whether any consumer needs "most recent N" (open since FIND-D5-04).

**FIND-D5R-04 (Minor, D5/D2). The UT-ST-038 scanner skips the D5 public surface.**
`tests/boundary.rs:65-70` scans `api.rs`, `store.rs`, `error.rs`, `datadir.rs`. It does not scan `api_analysis.rs` (all D5 DTOs) or `store/d5.rs` (15 public methods). I read both by hand: no integer row key leaves the crate. A future `id: u64` in either file would pass. `RatingPoint.id: u64` (a `chess-domain` type) does carry the `rating_history` row id out of `Store::rating_history`; the domain needs it for `(at, id)` ordering, so I record it as Info, not a leak finding.

**FIND-D5R-05 (Minor, D5). `complete_job` and `fail_job` are not idempotent.** A worker whose reply was lost and who retries gets `NotOwner` (confirms FIND-D5-08); verified by `p_jobs_boundaries` (`A completes after reclaim: Err(NotOwner)`). Workers must treat `NotOwner` after their own success as success or re-read the job state.

### Info (no action required, recorded)

- **I-1.** `reset_player` takes the window caller-side: `stats_rows(window = None)` for a reset Player still returns pre-reset games (`after Bob reset ... returns 3 rows`). Intended (ASM-D5-2, domain `aggregate` takes `window`); `chess-app` must always pass `reset_at` for the Player's own view.
- **I-2.** `claim_job` claims a job whose game is still `in_progress` (probe `p_reset_prefs_and_job_states`: claimed, and after `finalize_game` the job stays `running`, owner `w`). The store leaves "finished games only" to the worker (documented in `jobs.rs`).
- **I-3.** `save_variation` with an empty `pgn` for an unknown game returns `InvariantViolated`, not `NotFound` (empty is checked first). Either is allowed by the SDD.
- **I-4.** No UNIQUE index on `(game, from_ply, pgn)`; the immediate transaction is the guarantee. Held across two `Store` handles and 200 rounds with 20 distinct keys (`rows 20 distinct keys 20`). A writer outside the API could still add a duplicate (authors' FIND-D5-R-03).
- **I-5.** `lib.rs` module doc (lines 4-9) still says finalize, abort, void, reset and `FaultPlan` are "not here yet". Stale text.
- **I-6.** Invalid JSON as default preferences makes `reset_player` fail with `InvariantViolated` and roll everything back (probe `p_reset_prefs_and_job_states`): correct.
- **I-7.** `get_game_analysis` uses `unchecked_transaction()` (deferred, a WAL snapshot at first read); I did not run a concurrent-writer test for it `[INF]`.

## 3. What I verified (so a reader can tell what was and was not found)

### 3.1 D2 behaviours `[RAN]` (probes in `scratchpad/probe/crates/chess-store/tests/zz_review.rs`, `zz_review2.rs`)

| Check | Result |
|---|---|
| Elo against an independent implementation (own `1/(1+10^(d/400))`, half away from zero by `floor(x+0.5)`, K 40 below 30 games else 32): ratings {100, 799, 800, 1200, 1500, 2400, 3100} for both sides, counters {0, 29, 30, 31, 100} for both sides, three results = 3,675 games through `start_game` and `finalize_game` | all equal: both deltas, both K, new counter, new rating (`ELO cases 3675`). The 29 to 30 boundary is inside the grid. Exact `x.5` ties are not reachable with this formula and I did not construct one; the rounding rule is `f64::round` in `chess-domain` |
| Repeat finalize, finalize after abort, abort after finalize, void of a finished game | `AlreadyFinal`, no change; two `Store` handles on one directory finalizing one game concurrently: one `Finalized`, one `AlreadyFinal`, exactly 2 rating rows, invariant clean |
| Self-play, custom start, `rated = 0` | unrated, elo columns NULL (existing tests, mutants M03/M04 killed) |
| `analysis_used` | overwritten (FIND-D2R-02) |
| Time totals, hash, `journal_seq` on terminal columns | written from the request (mutants M08, N06 killed) |
| Overflow input (`total_active_ms = u64::MAX`) | `InvariantViolated`; ratings, counters, status unchanged (`p_overflow_total_rolls_back_ratings`) |
| `reset_player` with an empty hash in a `VoidSpec`, discovered after the reset point rows and preferences were written | whole call rolled back: `reset_at` NULL, preferences unchanged, game `in_progress` (`p_reset_empty_hash_rolls_back_everything`) |
| `reset_at` equals `ended_at` of each voided game; other human's slot freed; computer and unknown Player refused | yes (`p_reset_ended_at_equals_reset_at`) |
| Void of a completed game through any public method | cannot happen; `AlreadyFinal` (`p_void_unreadable_on_completed_...`) |
| Kill (SIGKILL after the marker) at each of the four D2 `before_commit` failpoints, parent reopens | database content identical to before; invariant clean; retry succeeds; no stray files beyond `backups chess.db chess.db-shm chess.db-wal journal store.id` |
| Text fields with NUL, quotes, `DROP TABLE`, emoji | stored verbatim, tables intact |
| Error leaks | `StoreError` `Display` is a category only; `map_sqlite` and `map_io` log kinds and codes, never values (read `error.rs:176-232`) |

### 3.2 D5 behaviours `[RAN]`

`stats_rows`: window is inclusive (`started_at >= window`: 3 rows at 300, 2 at 301); side White returns games 1, 3, 4, Black returns 2, 3, 5; `limit 0` returns 0 rows; self-play with no side is `White`; `head_to_head_rows(p, p)` returns only self-play; unknown opponent or Player is `NotFound`; voided, in-progress, self-play rows are returned (no counting predicate); imported games (no seat) never appear; a computer opponent carries `category_now` from its Strict rating. `result_for_viewed` for a game the Player lost as Black is `Loss`. `upsert_move_eval` stores `cp`/`mate` as given (`i32::MIN` accepted), replaces on a second upsert, refuses ply 0, a ply with no `MOVE` row and both `cp` and `mate`; `get_game_analysis` reports `evaluated 3 total 10`. Variations: idempotent save returns the first `created_at`, a stored key beyond the anchor is returned before the anchor check, a new key at the ply count is `VariationAnchor` (also `u32::MAX`), delete is idempotent, NUL-prefix keys are distinct, two `Store` handles over 200 rounds give one row per key. Jobs, injected clock, lease 10,000 ms: heartbeat `t`, claim at `t + 10,000` does not take it, at `t + 10,001` it does; lowest `game_id` first; the original owner then gets `NotOwner` on complete; a stranger gets `NotOwner` on fail; a clock stepped back (heartbeat in the future) does not release it; `enqueue_failed_once_per_start`: first call 1, second 0.

### 3.3 D1

- **Feature off.** The two compile-fail doctests run in the default test (`Doc-tests chess_store ... 2 passed`). In the default release `libchess_store.rlib`, none of the object files contain `FAILPOINT-KILL`, `unknown failpoint`, `finalize.before_commit`, `job.before_commit`, `open.after_db_created` or `sync.probe` (`strings -a` count 0). The names `FaultPlan` and `UnknownFailpoint` and the constants appear only in `lib.rmeta`; I read that as rustc's record of cfg-stripped items used for diagnostics, not code `[INF]`. `cargo public-api` is not installed; the doctest plus the object-file check is my substitute. No test-only code in the default build: the `check` stub is `#[inline(always)]` and returns `Ok(())`. One cost remains in the default build: `db_path()` builds a `PathBuf` for every write command to pass as `ctx`. Negligible.
- **Registry.** Typos are refused at `install` (`finalize.befor_commit` fails) and at `trigger` (`unknown failpoint`, checked even when no plan is installed, so a call-site typo fails its first test). `parse` rejects bad actions and numbers.
- **Harness.** `common/child.rs`: `Drop` kills and reaps, so no zombies; `expect` times out at 60 s and prints the child's stderr; a Kill child aborts itself after 120 s if the parent dies. `crash.rs::run_child` has no `Drop` guard around a bare `Child`, but kills on timeout. No leftover child process of mine remained after the runs.
- **The six survivors the authors say they killed.** I rewrote them as mutants and ran them against `fault_injection`, `crash`, `lifecycle_faults`, `d5_faults`: X1 (rename directory sync removed), X2 (backup file sync removed), X3 (backup directory sync removed), X4 (`backup.verify` removed), X5 (`store.id` file sync removed), X6 (`store.id` directory sync removed), plus X7b (probe sync removed) and X8 (`backup.before_rename` removed): all 8 killed. The in-function `sync_all` mutant the authors name as not killable is indeed not killable here and I did not run it.

### 3.4 Gates re-run `[RAN]`

| Command | Result |
|---|---|
| `cargo fmt -p chess-store --check` | exit 0, no output |
| `cargo clippy -p chess-store --all-targets --offline -- -D warnings` | `Finished`, no diagnostics |
| same with `--features test-support` | `Finished`, no diagnostics |
| `cargo test -p chess-store` x5 | each run: 27 suites, 321 passed, 0 failed |
| `cargo test -p chess-store --features test-support` x5 | each run: 27 suites, 378 passed, 0 failed |
| `cargo llvm-cov -p chess-store --features test-support --summary-only` (line coverage as printed) | total 96.91 % lines, 92.49 % regions. `repo/finalize.rs` 94.55 % (165 lines, 9 missed), `repo/lifecycle.rs` 95.54 % (157, 7), `repo/invariant.rs` 96.05 % (76, 3), `repo/rating.rs` 94.59 %, `repo/stats.rs` 99.14 %, `repo/jobs.rs` 100 %, `repo/analysis.rs` 97.31 %, `store/d5.rs` 100 %, `fault.rs` 94.97 % |
| BM-ST-03 release x3 | `stats_rows` 19.9 to 24.5 ms for 10,000 rows; side white 11.1 to 16.0 ms; window and limit 0.5 to 0.6 ms; head-to-head 6.9 to 8.9 ms; `games_for_history` 7.3 to 8.6 ms. Machine load was 7 to 8 (other agents), so the figures are an upper bound. Matches the authors' order of magnitude |

Coverage is lines and regions only. `--branch` needs a nightly toolchain and was refused; statement and branch coverage beyond lines, and MC/DC, were not measured. The 90 % line target for the finalize functions is met.

## 4. Hand mutation (115 valid mutants, 110 killed, 5 survived)

Method: a script applies one text edit, runs the cargo tests of the area with `--features test-support`, records the first failing test, restores the file. Four workers ran in four copies of the snapshot; every copy was checked identical to `rev_d125` afterwards (`diff -rq` empty). Mutants that did not compile (warnings are errors here) are not counted; 12 were rewritten and counted once, in their rewritten form. Test selection: D2/D5 area `lifecycle lifecycle_faults job_queue analysis_variations stats_rows d5_faults`; D1 and sync-call mutants `fault_injection crash lifecycle_faults d5_faults`.

| Area | Mutants | Killed | Survived |
|---|---|---|---|
| finalize and abort (`finalize.rs`): AlreadyFinal arms, empty-hash checks, `rated` ignored, status fed to `rating_track`, white/black read order, rating-row `at`, `analysis_used`, wrong total column, wrong elo column, release removed, failpoint swallowed, `k`, counter, Training counter column, `journal_seq`, `termination` | 19 | 18 | 1 (M14) |
| void and reset (`lifecycle.rs`): abort text, slot and lease deletes, job state, `voided_by`, `ended_at`, empty-hash check, AlreadyFinal arm, unlisted check and filter, `reset_at`, reset point `k`, one track only, terminal skip, names-player check, termination text, preferences upsert, ordering, failpoints, `INSERT OR IGNORE` | 27 | 25 | 2 (M25, N01) |
| invariant and rating read (`invariant.rs`, `rating.rs`): chain, point change, delta sign, restart-sum, first-row, final compare, NULL test inverted, point flag, track column, `ORDER BY at`, `JOIN` for `LEFT JOIN`, skip(0) | 12 | 10 | 2 (M45, N03) |
| stats (`stats.rs`): window `>`, order DESC, seat swaps, colour default, result map, category kind, active_ms default, limit 0, head-to-head arm, opponent seat | 11 | 11 | 0 |
| jobs (`jobs.rs`, `d5.rs`): stale `<=`, order DESC, lease + 1 ms, heartbeat - 1, owner check, changed > 1, requeue state, sweep status and `<=`, `DO UPDATE`, once-flag inverted and not reset, claim failpoint | 13 | 13 | 0 |
| analysis (`analysis.rs`): `&&` to `||`, anchor `>`, empty check, key without `pgn` or `from_ply`, delete without `pgn`, changed > 1, list order, `evaluated + 1`, repeat `created_at`, failpoints | 12 | 12 | 0 |
| D1 (`fault.rs`) and call sites (`backup.rs`, `datadir.rs`, `open.rs`): skip, once, prefix filter, longest-root, canonical tail dropped, typo-accepting `install`, wrong errno, once-suffix, hit and fired counters, guard drop, root and context not canonical; X1 to X8 | 21 | 21 (M94 only by part B's `journal_faults`/`journal_crash`) | 0 |
| **Total** | **115** | **110** | **5** |

Survivors (M94, first reported as a survivor against the D1 and D2/D5 suites, is killed by part B's tests and is listed in FIND-D1R-03, not here):

| Mutant | Edit | Verdict |
|---|---|---|
| M14 | `abort_game`: `"voided"` removed from the AlreadyFinal arm | **Real gap** (FIND-D2R-04) |
| M25 | `void_unreadable_game`: `"completed"` removed from the AlreadyFinal arm | **Real gap** (FIND-D2R-04) |
| N01 | `release_and_queue`: `INSERT OR IGNORE` to `INSERT` | Real gap, reachable only through D3 (FIND-D2R-04) |
| M45 | invariant: point row no longer restarts the sum | Equivalent: the chain check already forces `expected == previous_after` at every row |
| N03 | invariant: `skip(1)` to `skip(0)` | Equivalent: the first row passes both checks by construction |

The authors' D2 evidence says 50 of 54 killed and D5's 49 of 50; my independent set found two real gaps and two equivalent mutants on D2 and none on D5. Mutation by hand is not `cargo-mutants` (installed on this machine, `~/.cargo/bin/cargo-mutants`, though the allocation said it was not; I did not run it).

## 5. Conformance statement

| Item | Basis |
|---|---|
| Standard in force | SWE-001: Rust 1.99.1 edition 2024, compiler warnings, `clippy` and `rustfmt --check` as errors. SEI CERT is applied by intent only; **no SEI CERT, MISRA or AUTOSAR conformance is claimed** |
| Tool-checked | `rustfmt --check` clean; `clippy -D warnings` clean with and without the feature (workspace lints deny `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_*` outside tests); `#![forbid(unsafe_code)]` in `lib.rs`; `warnings = deny` made 12 of my mutants fail to compile, which shows the lint is live |
| Manual (this review) | read of `finalize.rs`, `lifecycle.rs`, `invariant.rs`, `rating.rs`, `stats.rs`, `jobs.rs`, `analysis.rs`, `store/d5.rs`, `fault.rs`, `api.rs`, `api_analysis.rs`, `error.rs`, the schema and the test files named above; SQL is parameterised, the only `format!` into SQL are column names from a closed enum |
| Not checked | `cargo-deny`, `npm audit`, `cargo audit`, SBOM and any new dependency (D1, D2 and D5 add none that I saw; `Cargo.toml` of the crate lists no new entry); miri; sanitizers; `cargo public-api` |

## 6. Not run, not analyzed, assumed

- **Not run:** two separate OS processes finalizing, resetting or saving variations at once (I used two `Store` handles in one process: two connections, one process; the kill tests used a second process but not at the same moment as a writer). Power-loss behaviour and APFS journalling. Intel Mac. `cargo-mutants`. Branch coverage and MC/DC. The `chess-app` consumer of these APIs (not built). The in-function `File::sync_all` mutant. The case-insensitive match of a non-existent path tail (reasoned in FIND-D1R-02, not executed). A concurrent-writer stress of `get_game_analysis`.
- **Not analyzed:** parts A, B, C, D3, D4 except where a D1/D2/D5 test touches them; `thread.rs` panic handling; the SDD text beyond sections 3.7.2, 3.11.4, 3.11.7 and the notes; `chess-domain` beyond `elo.rs` and the ordering used by the pin test.
- **Assumed:** the live tree, not git `HEAD`, is the baseline B6 ("working tree plus untracked files"); `3.11.4` pseudocode is authoritative where the Rust differs only in structure. I took the SDD sentence "the store builds `GameFacts` ... from the rows it re-reads" to allow the `rated` flag to gate rating work.
- **Probe code** lives in the scratchpad only (`probe/crates/chess-store/tests/zz_review.rs`, `zz_review2.rs`, and a unit test in `probe/crates/chess-store/src/repo/lifecycle.rs`). It is not in the repository; if the authors want the torn-read, kill and Elo-grid tests, they are written to be lifted into `tests/`.
- I did not run any `bd` command and changed no bead.

## 7. Needs a human

- **Brian Prascher:** GATE-109 (G3) is not approvable until FIND-D2R-01 is closed. FIND-D2R-02 (OR or overwrite for `analysis_used`) and FIND-D2R-03 (where terminal-state validation lives) are decisions. An index on `variation(game_id)` (FIND-D5R-02) is a stored-format change, G3.
- **System Architect:** FIND-D5R-03 ordering key (`id` or `(started_at, id)`) and whether any consumer needs "most recent N"; `backup.after_copy` wiring (FIND-D1R-01) and ownership of the other eight unwired names.
- **Author of D3:** FIND-D2R-05 and the N01 gap come due when `apply_records` calls `void_game`.

## 8. Closing checklist for the authors

1. Put `check_rating_invariant` in one read transaction; add the concurrent finalize-and-check test; re-run the probe.
2. Add tests for M14, M25, a `parse` test for `!` in `fault_injection.rs`, and the four kill tests of FIND-D1R-05.
3. Decide FIND-D2R-02, FIND-D2R-03, FIND-D5R-03; add the `pgn` cap or record the app as owner.
4. Add `api_analysis.rs` and `store/d5.rs` to the UT-ST-038 scan.
5. Wire or list as pending every registered failpoint name; make `install` reject a root that cannot canonicalize.
