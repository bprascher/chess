# REV-chess-engine-rework: adversarial re-review of the `crates/chess-engine` rework

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-engine-rework.md |
| Task | chess-a0r.3.10 rework, re-review (original review chess-a0r.3.11, closed), baseline B6 |
| Date | 2026-11-10 |
| Stance | **Reviewer** (software-engineer agent, fresh invocation; did not write the code and did not write the first review). **Desk review only.** Not the System Architect desk review, not an approval of GATE-106, not an approval of the EF-03 CI diff |
| Scope | `crates/chess-engine` working tree (uncommitted); `ci/check.sh` and `.github/workflows/ci.yml` hunks of the EF-03 proposal (reviewed for correctness only, **NOT approved**, Brian Prascher decides) |
| Inputs | review-chess-engine-code.md; step2-engine-evidence.md section 10; GATE-106; SDD-chess-engine.md 3.10.13 (AN-EN-01..07). Brian's ruling MAX_MOVETIME = 1 hour (2026-11-10) applied |
| Standard | SEI CERT by intent only (SWE-001 section 6). **No conformance claim** |
| Method | Code read first. Every Major reproduced with my own probes (a separate crate in the session scratchpad, outside the repo, depending on `chess-engine` by path), not the author's tests. Mutants applied to a scratch copy of the crate. The repo working tree is unchanged by this review except this file; I wrote nothing under `crates/`, `ci/` or other docs. Scratch: `/private/tmp/claude-501/-Users-brianprascher-Projects-chess/e49c1e94-65b2-4e2e-b7bf-dacbcb40ad6c/scratchpad/{rr,mut2,rrloop,ef03}` |
| Verdict | **PASS WITH CONDITIONS**. All four Majors are fixed in code. No Blocker, no new Major. Conditions are in section 6 (EF-03 is only locally proven and its CI diff is unapproved and depends on an untracked file; SDD notes promise mechanisms the code does not have) |

Tags: `[RAN]`, `[OBS]`, `[INF]`, `[UNV]`. Confidence: CONFIRMED (reproduced), LIKELY, SUSPECTED.

## 1. Disposition of the original findings

| Finding | Disposition | Evidence |
|---|---|---|
| EF-01 timers starved | **FIXED** | Own probe, scripted engine on `duplex`, `movetime` 100 ms, soft 80 ms, hard 300 ms. Endless valid `info` flood: `SearchTimeout` at 401.5 ms (multi-thread) and 401.2 ms (current-thread). Bare-newline flood 401.9 / 456.2 ms. Junk-line flood 402.1 / 405.1 ms. Stoppable flood: soft `stop` written, `Ok(stopped_early=true)` at 181 / 190 ms. Handshake flood (endless `option` lines, no `uciok`): `HandshakeTimeout` at 1.00 s (two 500 ms attempts). Dropped future during a flood: child dead at 355 ms (drain 300 ms), next search returns `SearchTimeout` at 402 ms. `shutdown()` during a flooded search: 17 to 32 us, search gets `Closed`. **Real pipe** (bash script `exec yes "info depth 1 score cp 1 pv e2e4"` as an override binary): `SearchTimeout` at 1.27 s for limit 100 ms + hard 1 s; no `yes` or zombie left (`pgrep`, `ps`). Mutant N1 (timer arms moved behind commands and output) is KILLED by three `ef_01_*` tests. I did not measure the splitter's MiB/s; the linear-work claim rests on reading `uci/lines.rs` (read index plus scan index, one compaction per `push`) and the flood tests ending on time |
| EF-02 `Duration` overflow | **FIXED** | Own probe: `validate` rejects `Duration::MAX`, `from_secs(u64::MAX)`, cap + 1 ms and cap + 1 ns with `InvalidRequest`; accepts exactly 1 h and 1 ns. `search(Duration::MAX)` and `search(cap+1ms)` return `InvalidRequest`, health stays `NotStarted`. A search at exactly 1 h runs, `stop()` gives `Ok(stopped_early)`, health `Ready`. Config: `Duration::MAX` in `hard_kill_grace` or `update_interval` gives `InvalidConfig`; 1 h accepted; 1 h movetime with 1 h hard grace runs and stops. `format_go` clamps to at least 1 ms (`uci/format.rs:84`). Mutant N2 (cap removed) KILLED. Mutant N2b (`checked_add` replaced by `+` in `deadlines`) SURVIVES: the second line of defence is unreachable behind validation, so it is untestable from outside, not a gap in the first line |
| EF-03 FF-7 skip is a pass | **PARTIAL** (fixed locally, not proven hosted, not approved) | See section 3 |
| EF-04 arbiter lost wakeup | **FIXED** | Own probe, real `fake_uci` children behind `EnginePool::analysis`, two OS threads with a barrier, interactive started 0 to 396 us after background (background `movetime` 100 ms, a lost preemption = interactive waiting over 60 ms with background finishing normally). Idle machine: 3500 races: 3299 `Preempted`, 201 interactive got the lock first, **0 lost**. Under 28 CPU-burner processes on 14 CPUs: 3500 races: 3397 / 103 / **0 lost**. Mutant N7 (lane polls the stop signal before its `Search` is queued: the old failure shape) is KILLED by `ef_04_..._2000_races`, so that test is sensitive to the defect it names. I could not run the old code (it is gone), so my probe's sensitivity to the old bug is not demonstrated |
| EF-04 suspected `RunningGuard` clobber | **REFUTED by structure, not by a failing test** | `[OBS arbiter.rs]` in both lanes `_lock` is declared before `_running`, so `_running` drops first (locals drop in reverse order): the guard clears `running` before the lock is released and the next owner cannot publish in between. The CAS is belt and braces. Own stress (a 30 ms background, an interactive 400 ms search, `stop()` 70 ms later, 800 rounds, run under the 28 burners): 800 of 800 stops honoured. Mutant N6 (unconditional `store(NONE)`) SURVIVES, which is consistent: with the drop order the unconditional store is harmless. No test pins the CAS; none needs to |
| EF-05 inverted `UCI_Elo` | **FIXED** | `uci_elo_range()` returns `None` for min > max (`uci/options.rs`); plan returns `InvalidPersonality`. Mutant N9 (`Some((min, max))` always) KILLED by `ef_05_*` (instance survives) |
| EF-06 shutdown not bounded | **FIXED** | Own probe, engine floods `option` lines after `uci` (default 5 s handshake): `shutdown()` 96 us, `warm` gets `Closed`. Pool of 4 stuck-start instances: `pool.shutdown()` 417 us. Shutdown during a flooded search: see EF-01. Residual: section 4 NF-05 and NF-06 |
| EF-07 write deadline, move cap | **FIXED** | `write()` wraps every write in `timeout(handshake_timeout)`, kills the child through `die()` (`start_kill` then `wait`, reaped) and returns `SearchTimeout`. Mutant N5 (deadline removed) KILLED by `ef_07_*`. `MAX_MOVES` 10 000: boundary tests exist and pass. Own real-process probe with a bash engine that `exec sleep 600` after the handshake, requests with 10 000, 3 000 and 0 moves: all end `SearchTimeout` at 1.11 to 1.25 s (the hard deadline, limit 100 ms + 1 s), health `Dead`, **no `sleep` child and no zombie** afterwards. The write-deadline branch itself needs a blocked pipe and was exercised only by the author's scripted test (a 10 000-move line fit in the pipe on this machine) |
| EF-08 pv/score/depth vs best | **FIXED** (with a documented semantics change, NF-03) | `Accum::finish` reads as described; mutant N3 (check ignored) KILLED by both `ef_08_*` tests. I did not re-measure the real-Stockfish rate (1 in 300 originally) |
| EF-09 fragile margins | **FIXED** (by margins) | 10 consecutive full runs with `CHESS_REQUIRE_STOCKFISH=1`: 147 passed, 0 failed, 14 of 14 binaries ok each time (logs in `rrloop/run1..10.txt`). Three more full runs under 28 CPU burners: 0 failed. First-exec cost not re-measured. Real timers and upper-bound assertions remain (SDD 3.11 asks for paused time; `test-util` still not admitted) |
| EF-10 unasserted behaviours | **FIXED** | Re-run mutants, section 2 |
| EF-11 error-pick replay | **FIXED** | M41 and M42 KILLED. Vectors are engine-only (`tests/data/error_pick_vectors.txt`), per AN-EN-05. I did not recompute the 32 rows with an independent implementation; the header says an independent Python SplitMix64 produced them `[UNV]` |
| EF-12 FakeEngine | **FIXED** by reading `src/fake.rs`: validation with the real multipv cap, latency capped by `movetime`, `stop()` ends a latency wait or `Hang`, `SearchingGuard` returns health to `Ready` unless a fault set `Dead`, `elapsed` is the capped latency. I did not write my own probe for the four items |
| EF-13 (Info) | Unchanged; the code now matches AN-EN-01 only in part (NF-01) |
| EF-14 `Debug` | **FIXED** | `error.rs` hand-written `Debug` prints only the char count of engine text for `WrongVersion.found` and `InvalidPersonality.option`; `ef_14_*` passes. `error.rs` line coverage 61.54 % (the rest of the arms are trivial) |
| EF-15, EF-16 (Info) | DEFERRED by the author; I agree with the deferral |

## 2. The eight surviving mutants of the first review, re-run by me

Scratch copy, one mutant at a time, all unit and integration tests except `real_cancel_latency`, `CHESS_REQUIRE_STOCKFISH=1`. Baseline of the copy: 147 tests green.

| Mutant | Result |
|---|---|
| M05 `error_multipv >= 2` to `> 2` | KILLED (`ef_10_error_multipv_of_exactly_two_...`, `ef_11_select_move_replays_...`) |
| M12 drain failure or timeout no longer kills (the `Err` arm of `drain` does nothing) | KILLED (`ef_10_a_drain_that_fails_kills_the_child_...`) |
| M22 stderr `inherit` instead of `null` | KILLED (`ef_10_the_childs_stderr_is_the_null_device`) |
| M31 `try_wait` check removed | KILLED (`ef_10_a_child_that_exited_between_searches_is_restarted_transparently`) |
| M41 `next_f64` low 53 bits | KILLED (`ef_11_next_f64_...`, `ef_11_select_move_...`) |
| M42 mirrored pick | KILLED (`ef_11_select_move_replays_...`) |
| M46b `Preempted` without the preempt counter | KILLED (`ef_10_a_caller_stop_of_a_background_search_is_not_a_preemption`); the first form did not compile (unused variable) and was redone |
| M54 non-error branch keeps the caller's multipv | KILLED (`ef_10_without_an_error_roll_...`) |
| M19 background no longer waits for `waiting == 0` | SURVIVES. **Judged equivalent in behaviour.** `[INF]` Without the wait, background queues on the FIFO lock; when it gets the lock it publishes `running` and re-checks `waiting`; if an interactive request queued meanwhile it releases the lock and loops. The order in which searches run is the same. The only differences are lock churn and a spurious bump of `preempts`, which is read again after the retry and so cannot misreport. Not proven; no test distinguishes it |

Extra mutants of mine on the new code (same method): N1 KILLED, N2 KILLED, N2b SURVIVES (unreachable by design), N3 KILLED, N5 KILLED, N6 SURVIVES (harmless, see EF-04), N7 KILLED, N9 KILLED. **N4 SURVIVES: the pool's parallel `JoinSet` shutdown** (replaced by a sequential loop) passes every test, because with the `Closing` fix each stuck instance now shuts down in microseconds. The parallelism that AN-EN-03 requires is no longer pinned by any test (NF-07). **N8 SURVIVES: `Closing::wait` without its flag check** (`driver.rs` `wait`): the lost-wakeup window it closes (request between the driver's top-of-loop check and the first poll of `wait`) is not testable from outside; correct by reading, see section 4.

## 3. EF-03 CI diff: local behaviour and review of the diff (not approved)

`[RAN]` A scratch copy of `ci/check.sh` cut after the new first step (`$S/ef03/ci/check.sh`), run from a scratch tree whose `third_party/stockfish` is a symlink to the real one:

| Case | Result |
|---|---|
| (a) binary at the default path | prints `found ...; CHESS_REQUIRE_STOCKFISH=1`, next step sees `REQUIRE=1`, exit 0 |
| (b) `CHESS_STOCKFISH=/nonexistent/sf` | exit 1 with the three-line message |
| (c) default path absent | exit 1 |
| (d) `CHESS_ALLOW_STOCKFISH_SKIP=1` with `CHESS_REQUIRE_STOCKFISH=1` preset | warning printed, variable unset, exit 0 |
| (e) `CHESS_STOCKFISH` is a directory; (g) `SKIP=0` and no binary; (h) empty `CHESS_STOCKFISH` and no binary | exit 1 |
| (f) `CHESS_STOCKFISH=/bin/ls` (any regular file) | gate passes, `REQUIRE=1`; the real tests would then fail on the `id name` check, so this is not a hole, but the gate itself does not check what the file is |

Not re-run by me: the author's claim that `CHESS_REQUIRE_STOCKFISH=1` with an absent binary makes the real tests panic (I read `tests/real_stockfish.rs` `require()`, which does so `[OBS]`). My 13 full runs had the binary present and the variable set; they passed 147 tests.

Review of the diff (not an approval):
- The logic is correct for the cases above. `${CHESS_STOCKFISH:-...}` and the test's `binary()` resolve the same default file; `-f` matches `is_file()`.
- **The `deny_args` expansion and `npm audit` hunks are not part of EF-03** and were not reviewed.
- **EF-03 CI-1 [Major for the hosted claim]: `ci.yml` runs `third_party/stockfish/fetch.sh`, and `git ls-files third_party` lists only `third_party/eco/*`.** `[RAN git ls-files, git status]` `third_party/stockfish/` (including `fetch.sh` and its `.gitignore`) is untracked (`??`). A hosted run of the committed workflow would fail at that step ("No such file") unless `fetch.sh` and `.gitignore` are committed with the workflow change. The hosted run has never been done; `fetch.sh` itself says its asset name is `[UNV: not checked against the release]` and needs `jq`, `lipo`, `curl` on the runner. So the hosted half of EF-03 is unproven and, as the tree stands, would be red.
- CI-2 [Minor]: after an opt-out (`CHESS_ALLOW_STOCKFISH_SKIP=1`) the run ends with `All gates passed.` and the warning is not repeated, so a log reader or a release step that greps the last line sees an unqualified pass. Repeat the warning at the end, or change the final line.
- CI-3 [Info]: the gate of record can be bypassed locally by one environment variable; that is by design and printed, but nothing records it. `target/tmp/stockfish-integration-status.txt` is still never read by CI.

Disposition: EF-03 **PARTIAL**. Local enforcement works as the author describes; hosted enforcement is unrun and blocked by CI-1; the diff is pending Brian's decision.

## 4. New findings (regressions and gaps introduced or left by the rework)

| ID | Sev | Finding |
|---|---|---|
| NF-01 | Minor | **The amended SDD (AN-EN-01 to 04) promises mechanisms the code does not have.** `[OBS grep]` `max_lines_per_poll` (AN-EN-01.4), `stop_ack_bound` (AN-EN-02) and `max_position_moves` as an `EngineConfig` field (AN-EN-03) do not exist in `src/`; the position cap is the constant `SearchRequest::MAX_MOVES`. The write deadline is `handshake_timeout` for every write; AN-EN-03 says the hard deadline during a search and the shutdown grace at shutdown. The driver `select!` order is closed, hard, soft, commands, output; AN-EN-01.1 says the two timers first, then the drop check. Verification IDs the notes add (`CT-EN-080` to `086`, `UT-EN-063` to `065`) appear in no test (tests are named `ef_*`). The author listed several of these as "SDD conflicts for the architect", but the SDD was amended in parallel and now conflicts with the code. **FIND-E5 stays open**: a `stop`-ignoring engine is killed only at limit + 2 s, ENG-5's 200 ms holds only for a cooperating engine (`ef_01_..._soft_stopped` covers the cooperating case). Behaviour is safe (my floods: all bounded). This is a design/trace gap for the SA, not a defect |
| NF-02 | Minor | `SearchLimits::MAX_MOVETIME`, `SearchRequest::MAX_MOVES`, `EngineConfig::MAX_DURATION` are new public associated constants; `MAX_DURATION` (1 h, ESTIMATE) caps `handshake_timeout`, `shutdown_grace`, etc. and `MAX_MOVES` (10 000, ESTIMATE) caps plies. A caller that built a long game above 10 000 plies gets `InvalidRequest` forever for that game. 10 000 plies is far above any real game, but the number has no basis (OQ-EN-005 open) |
| NF-03 | Info | EF-08 semantics: `Cp(0)` with `depth 0` now also means "no line named the returned move" (about 0.3 % of short real searches, author FIND-E17). A bound score taken from `bound_main` is returned as an exact `Cp`/`Mate` (the `Score` type has no bound flag). `chess-app` must treat depth 0 as "no evaluation"; SDD 3.6.1 does not say so |
| NF-04 | Info | **Public API changes not in the SDD cards** `[OBS lib.rs, grep pub]`: the three constants above; hand-written `Debug` for `EngineError`; `EngineError::{InvalidConfig, Busy, Closed}` and `ErrorClass` (from the first pass); `ProcessSpawner::with_args`; `fake_uci` mode `stderr-probe`; `FakeEngine` behaviour changes (health, stop in latency, cap 8, latency capped by movetime, deterministic `elapsed`; one existing fake test changed from 60 ms to 40 ms latency). `UciEngine::shutdown` now also interrupts start-up, prepare, write, drain and rejects queued commands with `Closed`; `EnginePool::shutdown` and `release_game` run in parallel. No new crate, edge, dependency or feature (`Cargo.toml` unchanged in kind: `rt` only, no `rt-multi-thread`, no `test-util`) |
| NF-05 | Info | The soft or caller `stop` write in `search_loop` and the stop write in the dropped-future branch are not wrapped in `until_closing`. A `shutdown` that arrives while such a write is blocked waits for the write deadline (`handshake_timeout`, 5 s default) before the `Cmd::Shutdown` is seen, not the 1 s grace. Needs a full stdin pipe, which a 5-byte write cannot cause on a healthy engine; I could not construct it. Bounded, no leak |
| NF-06 | Info | `Arbiter::stop(priority)` still reads `running` and then bumps the lane signal, so a `stop()` meant for one interactive search can stop the next interactive search if the lanes swap in between (UCI has no search identity, `stop` has no memory). Not reproduced; inherent to the design; the arbiter hardening does not claim to remove it |
| NF-07 | Minor | **Test gaps left by the rework** `[RAN mutation]`: N4 (pool shutdown in parallel) and N8 (`Closing::wait` flag check) survive; no test pins either. The parallel shutdown matters again as soon as one instance ignores `quit` (each then costs up to 2 x grace) and AN-EN-03 requires it. Suggest a pool test with several engines that ignore `quit` (the scripted engine has `ignore_quit`) asserting total time near one grace, not the sum |
| NF-08 | Info | `ef_04_..._2000_races` takes about 24 s and `real_cancel_latency` about 25 s; the full engine test run is about 60 s `[RAN]`. Author FIND-E15 agreed. Because the arbiter test judges from the engine log, not timing, it is stable (13 of 13 full runs green, 3 of those under load) |
| NF-09 | Info | The author's FIND-E14 (one unexplained failed run) did not recur in my 13 runs (10 idle, 3 loaded). Cause still unknown; not claimed fixed |

No finding of Blocker or Major severity beyond the EF-03 hosted claim (CI-1), which is a process item outside this crate.

## 5. Regression hunt: what I specifically looked for

| Hypothesis | Result |
|---|---|
| Lost wakeup in `Closing` (flag + `Notify`) | `wait()` creates `notified`, calls `enable()`, then checks the flag; `request()` stores the flag before `notify_waiters()`. This is the correct pattern; no lost wakeup by reading. N8 shows tests cannot catch a regression here |
| Deadlock or hang at shutdown | None found. Hung start, hung handshake flood, flooded search, 4 stuck instances through the pool, a dropped future in a flood: all return in microseconds to the stated bounds. Double `shutdown()`: the second `Cmd::Shutdown` is dropped with the receiver and its `rx.await` errors, which `shutdown` ignores `[OBS]` |
| `Closing` stays set forever | Yes by design; the actor exits after `Cmd::Shutdown`. A request after shutdown gets `Closed` (send fails) |
| `checked_add` | `deadlines()` returns `None` and `begin_search` returns `InvalidRequest`; in `search_loop` the fallback fires both deadlines at once. Unreachable after validation, N2b |
| Write deadline kills the child: zombies? | `die()` is `start_kill` then `wait` bounded by `shutdown_grace`; a wait that times out leaves the child to `kill_on_drop`. Real runs left no `sleep`, `yes` or zombie process (`ps` stat Z count 0 after every real-process probe) |
| Drain under flood | `drain` is inside `timeout`, which tokio polls with budget-unconstrained delay; my drop-during-flood probe: dead at 355 ms for a 300 ms drain timeout |
| EF-08 behaviour change | NF-03 |
| `Debug` impl | Complete over all 15 variants; `Preempted.partial` still prints a `SearchResult` (no engine free text there beyond move strings) |
| Fake behaviour | Read in full; consistent with AN-EN-04 items 1 to 5; item 6 (one contract suite against fake and real) is not built (NF-01) |

## 6. Verdict and conditions

**PASS WITH CONDITIONS** (a desk review; a recommendation to the System Architect and Brian Prascher, not an approval).

All four Majors are fixed in code and survived my independent reproduction attempts: EF-01 (floods on scripted and real pipes), EF-02 (1 h cap and extremes), EF-04 (7000 races, 0 lost, half under CPU load), EF-03 (local enforcement works). The eight reviewer mutants are killed except M19, which I judge equivalent.

Conditions:
1. **EF-03 hosted:** `third_party/stockfish/fetch.sh` and its `.gitignore` are untracked, so the `ci.yml` step cannot work from a clean checkout. Brian decides the CI diff; if approved, commit the script with it, and run it once on a hosted runner (not done by anyone). Until then EF-03 is PARTIAL. Consider repeating the skip warning at the end of `ci/check.sh` (CI-2).
2. **NF-01:** the SA should reconcile AN-EN-01 to 04 with the code (the stop acknowledgement bound and FIND-E5, `max_lines_per_poll`, position cap as config or constant, the write-deadline rule, the poll-order wording, test IDs). Either the code or the notes must move.
3. **NF-07:** one pool test for parallel shutdown with `quit`-ignoring engines (N4 survives).
4. The minors NF-02 and NF-03 are for the SA and the `chess-app` owner (depth 0 means "no evaluation").

This review does not approve GATE-106, the SDD amendment or the CI change; none has been approved by anyone.

## 7. Commands and results `[RAN]` (arm64, rustc/cargo 1.99.1, 2026-11-10)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` and `cargo fmt -p chess-engine -- --check` | exit 0 both |
| `RUSTFLAGS="-D warnings" cargo clippy -p chess-engine --all-targets -- -D warnings` | `Finished`, no diagnostics |
| same with `--features test-support` | `Finished`, no diagnostics |
| `CHESS_REQUIRE_STOCKFISH=1 RUSTFLAGS="-D warnings" cargo test -p chess-engine --no-fail-fast`, 10 runs | each: 147 passed, 0 failed, 14 of 14 binaries ok |
| same, 3 runs under 28 `sh` CPU burners (14 CPUs) | each exit 0, 0 failed |
| `cargo llvm-cov -p chess-engine --summary-only` (separate target dir, binary present) | lines TOTAL 2182, missed 261 = 88.04 % (includes `bin/fake_uci.rs` at 0 %, as before). Per file as printed: driver.rs 682/64 missed = 90.62 %, arbiter.rs 100 %, config.rs 100 %, plan.rs 100 %, types.rs 100 %, uci/lines.rs 100 %, uci/options.rs 100 %, uci/parse.rs 97.50 %, uci/format.rs 95.65 %, pool.rs 99.22 %, select.rs 98.70 %, fake.rs 97.65 %, locate.rs 91.30 %, spawn.rs 92.16 %, error.rs 61.54 %. The author's numbers are reproduced. Branch coverage column prints 0 (not measured) |
| probes | section 1 |
| mutation (scratch copy) | sections 2 |
| `ci/check.sh` first step, six-plus cases | section 3 |

## 8. Conformance statement

Standard: **none claimed**. This is a CERT-informed review (untrusted input, panics, overflow, resource and process handling).

| Category | Detail |
|---|---|
| Tool-checked | `cargo fmt --check`; `cargo clippy -D warnings` with and without `test-support` (workspace lints deny `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, cast lints); `cargo test`; `#![forbid(unsafe_code)]` `[OBS lib.rs]` |
| Tool-checked violations | none |
| Manually reviewed | `driver.rs`, `arbiter.rs`, `uci/lines.rs`, `uci/options.rs`, `types.rs`, `config.rs`, `error.rs`, `pool.rs`, `fake.rs`, `spawn.rs` in full; the tests in `tests/rework.rs` by name and by mutation, not line by line; the CI hunks of EF-03 |
| Not declared satisfied | any CERT rule |
| Overall | NOT-DETERMINED for any standard |

## 9. Not run, not analyzed, assumptions

- **Not run:** a hosted CI run or `fetch.sh` on a runner; the full `ci/check.sh` (cargo is shared with the chess-store engineer; I ran only its first step in a copy, plus the crate commands); `cargo deny` / `npm audit`; Intel slice; `Threads` above 1; TSan, miri, sanitizers, `cargo-mutants` (hand mutants only); fuzzing; branch/MC/DC coverage; the original EF-01/02/04 reproductions against the old code (it no longer exists; sensitivity is shown by mutants N1, N2, N7 instead); the 32 error-pick vector rows against an independent oracle; the real-Stockfish EF-08 rate; the splitter MiB/s; the FakeEngine EF-12 items by my own probe.
- **Not analyzed:** `chess-app` consumers (none exist); SDD 3.10.13 beyond the mapping in NF-01; the chess-store crate (another engineer's, untouched).
- **Assumptions:** probe engines are scripted in-process (tokio `duplex`), bash scripts or the real `fake_uci`; scripted results show driver properties, not Stockfish behaviour. All timings are from one Apple M4 Pro, 14 CPUs, macOS 26.7.1. Probe sources and logs are in the scratchpad named above and are not part of the deliverable.
- No claim of SA, SE or human approval is made.
