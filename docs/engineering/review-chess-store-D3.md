# Review: chess-store part D3 (apply path, reconcile-plan applier, apply backlog, `sync_backlog`), fresh review

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-store-D3.md |
| Stance | Reviewer, adversarial, **fresh invocation**, agent `software-engineer`. I did not write the D3 code or the earlier reviews. I read `review-chess-store-fixes-2.md`, `review-chess-store-B.md` (format and method only), the SDD sections, D-66/D-72/D-87/D-92/D-94 and `store-completion-allocation.md` section 4.3, then judged the code. I did not rely on `step2-store-D3-evidence.md` for any result below. Findings only: I changed nothing under `crates/` and no other document; no commit, push or bead change. |
| Gate | GATE-111, G3, **not approved**. This is a desk review by an agent. It does not stand in for Brian Prascher. |
| Baseline | B6. Repo HEAD `07a42b5` plus an uncommitted working tree. |
| Snapshot | 2026-11-23T17:52:11Z, copied to `<scratchpad>/revD3-se-82362/` (uniquely named; I deleted nothing I did not create). Tree hash of `crates/chess-store` (sorted `shasum` of per-file `shasum` lines, first 16 hex): **de8b36fd1596f602**. Recomputed on the live crate at the end: identical, so the snapshot equals the code at hand-off. |
| Scope | `src/repo/apply.rs`, `src/backlog.rs`, `src/store/d3.rs`, `src/thread.rs`, and the `finalize_in_tx` / `abort_in_tx` split of `src/repo/finalize.rs`. |
| Evidence tags | `[RAN]` executed here, `[OBS]` read here, `[INF]` inferred, `[UNV]` not verified |

## 0. Verdict

**PASS WITH CONDITIONS, with two open Majors.** Read this carefully: the earlier reviews of this crate used "PASS WITH CONDITIONS" to mean "no Major". That is **not** true here. I still do not say FAIL, because the defects are in policy for batches that cannot be applied, which is open for a ruling (FIND-D3-02, FIND-D3-01), and not in the transaction, ordering, idempotence or crash behavior, which held on everything I threw at it. But GATE-111 should not be approved until both Majors have a named disposition by Brian Prascher.

| Area | Result |
|---|---|
| Atomicity, rollback on every error path, idempotent replay of the same batch, SIGKILL at and around the commit | **Held.** 300 kill rounds, 0 failures (section 3). |
| Ply base `+1` (D-92, including journal ply 0), Takeback, Clock, Start, GameEnd, Abort, Voided via the D2 code, D-87 inside one call, `analysis_used` latch | **Held** on my probes and on the existing suite. |
| 100 ms lock wait, 5 s restored, store thread not held 5 s by an apply, retry schedule 0.1 s doubling to 5 s on the injected clock, `StoreBacklog` at 30 s | **Held** in the schedule tests. Two schedule defects are Minor (FIND-D3R-03, -04). |
| What to do with a batch that cannot be applied | **Not sound.** The first attempt and the retry behave differently, and a parked game is dead for the life of the process (FIND-D3R-01, **Major**). |
| `ended_at <= 0` | **Confirmed Major** (FIND-D3R-02): nothing is half-written, but the whole batch, including the last Move, is refused and the journal can never be finalized. |

Open findings: **2 Major, 10 Minor, 4 Info.** No Mermaid in this document. Mutation: 83 hand mutants, 69 killed, 14 survived (8 judged equivalent, 6 are real test gaps).

## 1. Method and independence

- Read first: CLAUDE.md, SWE-001, the SDD (3.7.2, 3.11.1 to 3.11.4, 3.12 and the dated note of 2026-11-23), architecture D-66, D-72, D-87, D-92, D-94, allocation 4.3, then the five source files in full.
- **Clean copy** (`revD3-se-82362-pristine/`) for every official run and **own `CARGO_TARGET_DIR`**; a **probe copy** (`revD3-se-82362/`) with my tests; a **mutation copy** (`revD3-se-82362-mut/`) restored after every mutant (`diff -rq` against the clean copy empty at the end). The first baseline loop ran in the probe copy and picked up my probe file, so I discarded it and re-ran everything in the clean copy. The numbers below are from the clean copy only.
- Probe tests (`tests/review_d3.rs`, 15 tests, plus one unit test in my copy of `backlog.rs`) are in the probe copy only. They are not delivered.
- Process hygiene: I killed only PIDs I started (my own hung test binaries and my watcher script). No `pkill`, no `pgrep -f`.

## 2. Standard commands [RAN] (clean copy)

| Check | Command | Result |
|---|---|---|
| rustfmt | `cargo fmt --check -p chess-store` | exit 0 |
| clippy, default | `cargo clippy --offline -p chess-store --all-targets -- -D warnings` | exit 0 (workspace lints deny `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, the two cast lints) |
| clippy, test-support | `cargo clippy --offline -p chess-store --all-targets --features test-support -- -D warnings` | exit 0 |
| tests, default, x5 | `cargo test --offline -p chess-store` | 5 of 5 exit 0; 413 `ok` lines each, 0 failed |
| tests, test-support, x5 | `cargo test --offline -p chess-store --features test-support` | 5 of 5 exit 0; 485 `ok` lines each, 0 failed |
| coverage | `cargo llvm-cov --offline -p chess-store --features test-support --summary-only` (0.9.1) | exit 0. Lines (the files include their in-file test modules): `repo/apply.rs` 98.01 % (12 missed of 602), `backlog.rs` 99.58 % (1 of 240), `thread.rs` 97.81 % (11 of 502), `store/d3.rs` 100 % (0 of 16), `repo/finalize.rs` 95.14 % (9 of 185). Whole crate: regions 93.62 %, functions 95.98 %, lines 97.35 %. Branch coverage is not reported by the tool here (column "-"). |
| missed lines | `cargo llvm-cov report --show-missing-lines` | apply.rs 64, 137, 184, 229, 293, 298, 335, 339, 352, 468; backlog.rs 141; thread.rs 127, 249, 371. All are `?` error edges, the restore-log branch or the `Start`-wrong-game arm. Line 229 is the computer-white matchup arm (see FIND-D3R-10). |

Counts are the number of `test ... ok` lines, not a tool total. No MC/DC was measured.

## 3. Attack results [RAN]

All probes ran in the probe copy against a real file database through the public `Store` API unless stated.

| Attack | Observation |
|---|---|
| **SIGKILL rounds, >= 200 required** | `rv_kill_rounds_failpoints_and_random`, 100 rounds of three kinds = **300 kills**: 100 at `apply.before_commit`, 100 at `apply.after_commit` (child stops at the failpoint, parent SIGKILLs), 100 SIGKILLs at a random 0 to 59 ms while a child applies 12 three-record batches. After each: reopen; rows form a gap-free prefix of whole batches (no torn batch); `journal_seq` and the game clocks equal the last applied batch; every acknowledged batch present; re-applying all 12 batches reaches the full state. **0 failures in 300.** 117.8 s. |
| Existing kill and fault tests | `apply_backlog` (CT-ST-007 before and after commit, CT-ST-008 with a second process holding the lock, kill at both failpoints) pass in all 5 + 5 runs. |
| Ply base | Journal ply 0 stores MOVE.ply 1 (suite). `ply = u32::MAX` is `InvariantViolated`, nothing written. |
| Half-null clocks, `u64::MAX` clock, `seq = u64::MAX` | Each `Err(InvariantViolated)`, state byte-equal to before (p04). `Takeback{to_ply: u32::MAX}` is `Applied` (deletes nothing). |
| `ended_at = 0` or `-5` on `GameEnd` in the same batch as the last two Moves | `Err(InvariantViolated)`; no move row, `journal_seq` unchanged, game `in_progress`, both unfinished slots still held (p12). No half write. See FIND-D3R-02. |
| Split D-87 pair | `[GameEnd]` in one call, `[Voided]` in the next: second call `Err(InvariantViolated)`, the row stays `completed` with its ratings (p07). Expected from the code; see FIND-D3R-14. |
| Lock held by another connection | `apply_records` returns `Queued` after about 100 ms; `busy_timeout` is back at 5 000 afterwards (suite and p06). A 4 000-record batch applies in 60 ms in a debug build (p13): no long hold. |
| Concurrent and cancelled `sync_backlog` waiters | 8 concurrent waiters with different deadlines, plus one dropped after one poll: all 8 get `Ok` once the lock is gone; a following `sync_backlog` on the empty backlog returns in under 300 ms (p09). No lost wake-up, no double completion seen. |
| Two `Store`s on one directory in one process | `Store::open` succeeds twice; both queue behind a held lock and both drain (p08). Order between the two backlogs is not defined; see FIND-D3R-15. |
| Panic in an attempt | Suite: `Internal`, thread survives, restart rule applies (`a_panic_in_an_attempt_is_internal_and_the_thread_survives`). I did not find a way to make `apply_records` itself panic. |
| Integer-id leak, error leak | `ut_st_038_*` pass. No `unwrap`, `expect`, indexing or `panic!` in the non-test code of the five files (clippy denies them and passes). `NotFound` carries the caller's own game uuid only. |

## 4. Findings

### FIND-D3R-01 [Major][CONFIRMED] A batch that cannot be applied either leaves a hole or kills the game's database updates for the rest of the process
- Location: `thread.rs:326-349` (`Worker::apply`), `backlog.rs:143-146`, `backlog.rs:75-78` (`has_game`), `repo/apply.rs:115` (journal_seq `max`).
- Observation: two paths, two behaviors.
  - **(a) first attempt refused** (`Permanent` on the direct attempt): the error is returned and the batch is dropped. A later valid batch applies, and `journal_seq` jumps to its last `seq` through the `max` rule. Probe p14: batch seq 2 (ply 1) applied; batch `[seq 3 Move with half-null clocks, seq 4 Clock]` refused with `InvariantViolated`; batch seq 5 applied. Result: rows `[(1,"u0"),(3,"u2")]` (ply 2 missing), `journal_seq = 5`, game `in_progress`. The stored high-water mark says "applied through 5" while seq 3 and 4 are not. The existing test `a_refused_batch_is_not_queued` pins "later batches still apply" as intended.
  - **(b) retry refused** (`Permanent` from the backlog pass): the game is parked. Every later batch of that game goes to the queue behind it (`has_game`) and is never attempted. Probe p05: lock held, batch A queued, a refusable batch B queued, lock released; the retry applies A and parks on B. Then 5 000 **valid** batches: each returns `Ok(Queued)`; final status `pending_batches: 5001, parked_games: 1`; the database stays at `journal_seq 2`, one move row, while the journal moves on. `sync_backlog` returns `Busy` for ever. With the injected clock at 31 000 s, `store_backlog: true`.
- Impact: (a) the stored sequence no longer means "everything up to here is applied" (D-66 reads it as a high-water mark); nothing signals the hole to the caller after the first `Err`. (b) A single bad record stops every later write of that game to the database for the life of the process, including its `GameEnd`: the game stays `in_progress` with its unfinished slots, the statistics and the export barrier (`sync_backlog`, SDD 3.11.9) are stuck, and the queue grows without bound (each queued batch is a `Vec<JournalRecord>`; bounded only by the journal itself). The journal is safe (NFR-2 holds), but the in-process database is dead until a restart, and the restart recovery (D4) does not exist yet (FIND-D3R-14). There is no API to inspect, drop or retry a parked head, and the signal (`store_backlog`) exists only if the caller polls `backlog_status`.
- Also: `classify` (`thread.rs:82-88`) treats every error except `Busy` and `Unavailable` as permanent, which includes `Internal`. `map_sqlite` (`error.rs:208`) sends every other SQLite result code there (for example out-of-memory, protocol, interrupt), and a caught panic is `Internal` too. A transient code on a retry therefore parks the game for good (LIKELY, not reproduced: I could not inject those codes). See FIND-D3R-09.
- Standards: none claimed.
- Verify fix by: a ruling on one policy for both paths (park on the first failure too, or never drop and report), a way out of the parked state, a bound or an alarm on the queue, and tests `apply_after_a_refused_batch_*` and `a_parked_game_*` that fail on today's behavior. Status: OPEN, needs Brian (this is FIND-D3-02, G3 as a stored-value rule).

### FIND-D3R-02 [Major][CONFIRMED] A `GameEnd` or `Abort` with `ended_at <= 0` refuses the whole batch, and the journal can never be applied (FIND-D3-01, FIND-F2-04 confirmed)
- Location: `repo/finalize.rs:116` and `:241` (the checks), reached from `repo/apply.rs:146-160`.
- Observation: p12: `[Move ply 0, Move ply 1, GameEnd{ended_at: 0}]` returns `Err(InvariantViolated)`. I checked for a half write: no move row, `journal_seq` 0, status `in_progress`, 2 slots held. Rollback is complete. The same journal fails the same way on every later call.
- Impact: the final Move shares the batch with the `GameEnd`, so the last move is not stored either, and no start, restore or resume can finalize the game. A journal with `ended_at <= 0` can be written (the journal format does not refuse it; `Voided.ended_at` and `Start.wall_ms` are not checked at all, FIND-D3R-07). A machine clock at the epoch is enough.
- Verify fix by: a ruling (refuse in the journal codec at encode and decode, or clamp, or accept in the store), then the same probe passes or fails by design. Status: OPEN, G3, Brian.

### FIND-D3R-03 [Minor][CONFIRMED] `sync_backlog` tries once, then waits for the timer, so it can answer `Busy` while the lock has been free for most of the timeout
- Location: `thread.rs:312-319` (`Command::Sync`: one forced pass, then waiters), `thread.rs:271-286`.
- Reproduction: p11. Lock held, one batch queued, 7 s pass so that the timer has grown (0.1 + 0.2 + ... + 3.2 s). `sync_backlog(2 s)` called while the lock is still held; the lock is released 400 ms later. Result: `Err(Busy)` after 2.27 s, backlog still holding the batch. The next timer pass was up to 5 s away.
- Impact: the export barrier fails for a free database. The caller can retry, so Minor.
- Verify fix by: p11 returns `Ok`. Status: OPEN.

### FIND-D3R-04 [Minor][CONFIRMED] After a game is parked, a new backlog starts with a stale timer
- Location: `backlog.rs:88-97` (`push` resets the timer only when `queues` is empty; a parked game keeps `queues` non-empty), `backlog.rs:159-162`.
- Reproduction: unit test in my copy of `backlog.rs`: drive one game's delay to 5 000 ms, park it, then `push` a second game at t = 72 300 ms. `wait_ms` is `Some(0)` (retry now, not in 0.1 s) and `delay_ms` is 5 000 (the next failure waits 5 s, not 0.2 s). Printed: `after push(game2) at 72300: wait_ms = Some(0) delay_ms = 5000`.
- Impact: SDD 3.12's schedule is not what a second game gets while any game is parked. No data effect. Mutants B13 and B19 survived (FIND-D3R-10).
- Status: OPEN.

### FIND-D3R-05 [Minor][CONFIRMED] A stale or duplicate batch moves rows and clocks back while `journal_seq` stays (FIND-D3-08 confirmed)
- Location: `repo/apply.rs:101-121`, `write_move`, `set_clocks`; no per-record seq guard.
- Reproduction: p01. Batch 1 = Moves ply 0, 1, 2. Batch 2 = Takeback to 1, then a different Move at ply 1 with clocks 111/222. After both: rows `[(1,u0),(2,ALT)]`, clocks 111/222, `journal_seq 6`. Replay batch 1: rows `[(1,u0),(2,u1),(3,u2)]`, clocks 299998/300002, `journal_seq 6`.
- Impact: the rows are older than the sequence says. Within one process the backlog keeps order, so only a duplicate delivery or a D4 replay can do this. Minor until D4 exists.
- Status: OPEN (D4 must pass monotonic batches or the applier should skip records with `seq <= journal_seq`).

### FIND-D3R-06 [Minor][CONFIRMED] `Move`, `Takeback` and `Clock` rewrite a terminal row (DR-10, documented)
- Location: `repo/apply.rs:133-145`, no status check.
- Reproduction: p02. Game finalized (`completed`, hash set, 2 moves). `apply_records([Takeback{to_ply: 0}])` returns `Applied`; the two move rows are gone; status, hash and ratings unchanged.
- Note on the trade-off: the SDD says the journal wins, and that is needed: a live `finalize_game` can commit before an earlier queued batch (FIND-D3-09), and then the queued Takeback is what makes the rows match the hash. A status guard would break that case. Either way the moves of a finalized game are not protected by `content_hash`. Needs the ruling to say so.
- Status: OPEN, documented.

### FIND-D3R-07 [Minor][CONFIRMED] The shape rule for times is applied to `GameEnd` and `Abort` only
- Location: `repo/lifecycle.rs:102-122` (`void_game_repair_pair`: no `ended_at` check), `repo/apply.rs:221-258` (`Start.wall_ms` to `started_at`).
- Reproduction: p03. `Voided{ended_at: 0}` and `Voided{ended_at: -7}` on an `in_progress` row: `Ok(Applied)`, stored `ended_at = 0` / `-7`, status `voided`. A `Start` with `wall_ms = 0` and `-3` creates the row with `started_at = 0` / `-3`.
- Impact: inconsistent with FIND-D2R-03 and FIND-D3R-02; a voided row can hold a non-positive end time. Whatever FIND-D3R-02 is ruled, these two should follow it. Status: OPEN.

### FIND-D3R-08 [Minor][LIKELY] Waiters of cancelled `sync_backlog` calls stay until their own deadline
- Location: `thread.rs:271-286`: `settle_waiters` never looks at `reply.is_closed()`.
- Observation: [OBS] only. A caller that wraps the call in its own timeout leaves its waiter in `self.waiters` until `deadline_ms`; with a parked game and a long `timeout` (up to `Duration::MAX`) that is for ever, and the thread wakes every 100 ms for as long as one waiter exists. p09 shows no lost wake-up, but it cannot see this leak, so I did not measure it.
- Verify fix by: a test that drops 1 000 `sync_backlog(1 h)` futures and checks the waiter count (needs a test hook). Status: OPEN.

### FIND-D3R-09 [Minor][LIKELY] "Permanent" includes `Internal` and a panic
- Location: `thread.rs:82-88`, `error.rs:187-217`. See FIND-D3R-01 for the effect (a transient SQLite code parks a game for the process). Not reproduced; only the classification is observed. Status: OPEN, goes with the ruling for FIND-D3R-01.

### FIND-D3R-10 [Minor][CONFIRMED] Mutation survivors that are real test gaps (section 5)
- A30: a restored computer-against-computer game with the matchup mapped to `hc` passes every test; the only restore test with computers (`the_matchup_follows_the_kinds_whatever_the_payload_text_says`) covers human against computer. Computer as White is also untested (apply.rs:229 is a missed line).
- B13 and B19 (timer base after a parked game; timer from a stale base): the schedule tests only advance to exactly the due time.
- T01 (poll 100 ms to 1 000 ms): the only guard is a 1.5 s bound.
- T04 (`has_game` dropped from the queueing rule): no test sends a batch for a game that is already parked. My probe p05 does and would kill it.
- T14 (a `Run` command forcing a backlog pass): no test says it must not.
- Status: OPEN. Verify fix by killing each of these mutants.

### FIND-D3R-11 [Minor][CONFIRMED] A regression in the timer or the sync code hangs the suite instead of failing it
- Evidence: mutants B17 (a pass skips unparked games), T05 (`>=` to `>` on the waiter deadline) and T07 (sync does not force a pass) each made the `--lib` test binary run for more than 5 minutes until I killed it (B17 459 s, T05 335 s, T07 207 s before my watcher killed them). The cause is a frozen injected clock: `sync_backlog`'s deadline is on that clock, so a test that waits for `Busy` waits for ever when the code is wrong.
- Impact: CI would time out with no failure message. Fix: an outer deadline (a real-time timeout around each `block_on`) in the thread tests. Status: OPEN.

### FIND-D3R-12 [Minor][CONFIRMED] New public surface (FIND-D3-11, G2 item, confirmed and one addition)
- Public: `Store::apply_records` (returns `ApplyOutcome`, the SDD list shows no return type), `Store::sync_backlog`, `Store::backlog_status` (not in the SDD list), `ApplyOutcome` (`#[non_exhaustive]`), `BacklogStatus` (all fields public, **not** `#[non_exhaustive]`: adding a field later breaks every constructor and exhaustive pattern), and under `test-support` only `StoreOptions::with_monotonic_clock` and `MonotonicClock` (the `pub use` is gated, `lib.rs:63-64`, good). `StoreOptions` gained a private field.
- Status: OPEN for G2 (Brian); the `#[non_exhaustive]` on `BacklogStatus` is a one-line change inside the same decision.

### FIND-D3R-13 [Info][CONFIRMED] Code that cannot run today
- `apply_reconcile_plan` has no caller (`#[allow(dead_code)]`, `apply.rs:437`); no code path reaches it through `Store` or the thread, so its failpoints and its lock behavior are exercised only by unit tests calling it directly. It uses the connection's 5 s lock wait (FIND-D3-14), has no status guard, and sets `journal_seq` exactly. D4 must decide each.
- `void_game_repair_pair` has no caller either (`#[allow(dead_code)]`, FIND-D3-03).
- `parked.clear()` at `backlog.rs:159-162` can never change anything: a parked game's queue is never emptied, so `queues.is_empty()` implies no parked game. Mutant B11 survived for that reason.

### FIND-D3R-14 [Info][CONFIRMED] Backlog loss on exit is recoverable only by D4, which does not exist
- `grep` finds no `open_for_resume`, no startup recovery, no caller of the plan applier in `src/`. The queue is in memory (`backlog.rs:21-23`) and the thread logs one `warn!` when it ends non-empty. Today a crash or a close with queued batches leaves the database behind the journal and nothing repairs it. This is the allocation's own split (D4 depends on D3), so Info, but it makes FIND-D3R-01 worse until D4 lands. Also: the D-87 pair must go to `apply_records` in one call (FIND-D3-10, p07: split calls give `InvariantViolated` and leave a settled `completed` row).

### FIND-D3R-15 [Info][CONFIRMED] Two Stores in one process have two independent backlogs
- p08. Per-game order is kept inside one `Store`, not across two. Part A (the open path) allows it; not a D3 defect, but it means D3's ordering rule assumes one `Store` per directory per process. State it in the SDD.

### FIND-D3R-16 [Info][CONFIRMED] `Abort` is applied to a human-against-human game
- p10: `Ok(Applied)`. D-87 says `Abort` exists only for computer-against-computer games; the store checks shape only (ruling FIND-D2R-03). Recorded so nobody expects the store to refuse it.

### Earlier findings: my disposition of the engineer's list (not re-ruled)
FIND-D3-01 = FIND-D3R-02 (Major, confirmed). FIND-D3-02 = FIND-D3R-01 (**raised from Minor to Major**, reproduced). FIND-D3-03 = FIND-D3R-13. FIND-D3-04: `move_eval` delete confirmed by mutant A12 (killed). FIND-D3-06: matchup ignored, confirmed, with the test gap A30. FIND-D3-07: `max` rule confirmed; A04 and A05 killed. FIND-D3-08 = FIND-D3R-05. FIND-D3-09: **not reproduced**: p06 (a live finalize racing queued batches behind a held lock) ended with the rows of the journal and an empty backlog; I did not build the `Voided`-behind-a-live-finalize case. FIND-D3-10 = p07. FIND-D3-11 = FIND-D3R-12. FIND-D3-13 = FIND-D3R-14. FIND-D3-14 = FIND-D3R-13. FIND-D3-15: the two `_in_tx` bodies are the old bodies; the D2 tests are unchanged and green, F01 to F05 mutants of that file were killed or equivalent (section 5).

## 5. Hand mutation (83 mutants) [RAN]

Method: one edit at a time in the mutation copy, run `cargo test --offline -p chess-store --features test-support --lib --test apply -q`; survivors also ran `--test apply_backlog`; survivors and unviable ones were re-run (unviable ones with `RUSTFLAGS=--cap-lints=allow`, survivors of interest with the **full** suite). Source restored after each (`diff -rq` empty).

| Result | Count |
|---|---|
| Killed (assertion failure, or hang killed by me after more than 2 minutes: B17, T05, T07) | **69** |
| Survived | **14** |
| Unviable after the re-run | 0 |
| Total | 83 |

Killed (69): A01 A02 A03 A04 A05 A07 A08 A09 A10 A11 A12 A13 A14 A15 A16 A17 A18 A19 A22 A23 A24 A25 A26 A27 A28 A29 A31 A32 A33 A34 A35 A36 A37 A38 A39 A40 A41 A42 A43 A44, F01 F02 F03 F04, B01 B02 B03 B04 B05 B06 B07 B08 B09 B10 B12 B14 B15 B16 B17 B18 B20, T02 T03 T05 T06 T07 T10 T12 T13. (A33 and A34, the two failpoints of `apply_records`, are killed only by `apply_backlog`.)

Survivors:

| ID | Mutant | Verdict |
|---|---|---|
| A06 | D-87 `index < v` to `<=` | Equivalent: the record at `v` is the `Voided`, the flag only changes the `GameEnd` arm. |
| A20 | Abort `final_journal_seq` set to 0 | Equivalent: the batch `max` update overwrites it. |
| A21 | GameEnd `final_journal_seq` set to 0 | Equivalent, same reason. |
| F05 | `finalize_game` commits an `AlreadyFinal` transaction | Equivalent up to the `finalize.before_commit` failpoint: nothing was written. |
| B11 | `parked.clear()` removed | Equivalent: dead code (FIND-D3R-13). |
| T08 | `.min(POLL_MS)` removed in `wait_ms` | Equivalent: the inner `unwrap_or(POLL_MS)` already caps it. |
| T09 | no `settle_waiters` after pushing a waiter | Equivalent: the next loop turn settles it. |
| T11 | waiters do not poll | Equivalent: a waiter exists only while the backlog is non-empty. |
| **A30** | matchup of a computer-against-computer restore set to `hc` | **Test gap** (FIND-D3R-10). |
| **B13** | `push` always resets the timer | **Test gap** (the correct rule is the opposite of FIND-D3R-04). |
| **B19** | next attempt computed from the stale timer | **Test gap**. |
| **T01** | poll 100 ms to 1 000 ms | **Test gap**. |
| **T04** | `has_game` dropped from the queueing rule | **Test gap**. |
| **T14** | a `Run` command forces a backlog pass | **Test gap** (unspecified behavior, weak test). |

"Equivalent" is my reasoning, not a proof. Kill rate 69/83 = 83.1 %; excluding the 8 judged equivalent, 69/75 = 92.0 %. The list of mutants (file, old text, new text) is in the mutation copy's script `scratchpad/mutants.py`; the raw results are `scratchpad/mutation-results.tsv` and `-p2.tsv`.

## 6. Conformance statement

Standard in force (SWE-001, CLAUDE.md): Rust 1.99.1, edition 2024; compiler warnings, clippy, rustfmt as errors; SEI CERT applied by intent only. **No SEI CERT conformance is claimed and none is determined.** MISRA and AUTOSAR do not apply to Rust.

| Category | Result |
|---|---|
| Tool-checked | `rustc` warnings as errors (workspace lint `warnings = deny`), `clippy -D warnings` with `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation`, `cast_sign_loss` denied, both feature settings; `rustfmt --check`. All clean. Tests allow unwrap/expect/panic/indexing through `clippy.toml`. |
| Manually reviewed | Transaction boundaries and rollback paths, ordering and queueing rules, timer arithmetic, waiter handling, error classification, new public surface, error and id leaks, `unsafe` (none: `#![forbid(unsafe_code)]`, [INF] from the crate rules, not re-grepped). |
| Not checked | Miri, ThreadSanitizer and AddressSanitizer (no nightly, D-76), `cargo-deny`, `cargo-audit`, fuzzing, power-loss behavior, Intel macOS, any CERT rule mapping. |

Overall: **NOT-DETERMINED** for any named standard. The lint profile is clean.

## 7. Test adequacy

Strong: ply base, per-kind row effects, atomicity (before and after dump equal), idempotence (apply twice equals once), D-87 inside a call, the 100 ms wait with the 5 s restore, the retry schedule on an injected clock, two real-process and kill tests. Weak: the policy for a batch that cannot be applied has one test per path and none for the later batches (T04, FIND-D3R-01); timer state after parking (B13, B19); the matchup arms (A30); the sync tests hang on regression (FIND-D3R-11); the plan applier has no test through the thread.

## 8. Not run, not analyzed, assumptions

- Not run: miri, sanitizers, fuzzing, `cargo-deny`, `cargo-audit`; power loss and `F_FULLFSYNC` behavior; Intel Mac; a two-process variant of my own beyond the existing CT-ST-008; the `chess-app` consumer (empty scaffold). Statements about `SQLITE_NOMEM`, `PROTOCOL` and `INTERRUPT` (FIND-D3R-01, -09) are from reading `error.rs`, not from an injected fault. FIND-D3R-08 is from reading.
- Not analyzed: D4 behavior, the `desk-review-G1-step2-D3.md` and `decision-package-store-D3.md` documents beyond the dated SDD note and the rows quoted above, other parts of `chess-store`.
- Assumptions: the working tree at snapshot time is what Brian's gate refers to (hash above). Section 2 counts are `ok` lines, not a tool-reported total; coverage includes in-file test modules.

## 9. What a human must do

Brian Prascher: rule on FIND-D3R-01 (policy for an uncurable batch, the way out of "parked", the bound and the alarm) and FIND-D3R-02 (`ended_at <= 0`, together with FIND-D3R-07); G2 on FIND-D3R-12. Until then GATE-111 stays PENDING; nothing in this review approves it.
