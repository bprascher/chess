# REV-chess-engine-code: adversarial code review of `crates/chess-engine`

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-engine-code.md |
| Task | chess-a0r.3.11 (baseline B5) |
| Date | 2026-11-09 |
| Stance | **Reviewer** (software-engineer agent, fresh invocation, not the author of `chess-engine`). Independence: independent of the author invocation; same agent type. **Desk review only.** It is not the System Architect desk review and not an approval of GATE-106 |
| Scope | `crates/chess-engine/{Cargo.toml,src,src/bin,tests,tests/data}`, working tree on top of `07a42b5` (uncommitted, untracked files) |
| Intent sources | docs/design/SDD-chess-engine.md 1.0 BASELINED with the 2026-11-09 G0 notes; docs/engineering/SWE-001-engagement-profile.md; architecture engine sections as cited by the SDD. The author's evidence file (step2-engine-evidence.md) was read **after** the code and tests; its claims were re-run, not trusted |
| Standard | SEI CERT applied **by intent only** (SWE-001 section 6): untrusted input, panics, unchecked arithmetic, resource handling. **No conformance claim is made** |
| Method | Code read first; baseline and gates re-run; throwaway probes and hand mutants lived in a scratch workspace outside the repo (`/private/tmp/claude-501/-Users-brianprascher-Projects-chess/e49c1e94-65b2-4e2e-b7bf-dacbcb40ad6c/scratchpad/`). The repo working tree is unchanged by this review except this file. One side effect on build outputs: I re-signed `target/debug/fake_uci` (ad-hoc codesign) during the FIND-E13 reproduction; this touches only a git-ignored build artifact |
| Verdict | **PASS WITH CONDITIONS** (section 8). No Blocker. **Four Major** (EF-01 to EF-04), all CONFIRMED by reproduction. Eight Minor, four Info |

Confidence tags: CONFIRMED (reproduced), LIKELY (strong reading, not reproduced), SUSPECTED. Evidence tags: `[OBS]`, `[RAN]`, `[INF]`, `[UNV]`.

## 1. Baseline and tool runs

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | `[RAN]` exit 0 |
| Clippy | `RUSTFLAGS="-D warnings" cargo clippy -p chess-engine --all-targets -- -D warnings`, and again with `--features test-support` | `[RAN]` both `Finished`, no diagnostics |
| Tests | `cargo test -p chess-engine` | `[RAN]` 114 tests, 0 failed (6 lib + 24 driver_scripted + 8 locate + 7 plan + 11 pool + 9 process + 1 real_cancel_latency + 7 real_stockfish + 13 select + 10 types_config_error + 18 uci_protocol). **The very first run failed once** (`ct_en_054`, EF-09); the next 15 full runs and every later run were green |
| Coverage claims | `cargo llvm-cov -p chess-engine --summary-only` (scratch copy, `CHESS_STOCKFISH` set so the real tests ran) | `[RAN]` driver.rs 521/576 lines (90.45 %), plan.rs 66/66, locate.rs 42/46, select.rs 76/77, uci 256/262; TOTAL as printed 1631/1861 lines = 87.64 % (includes `bin/fake_uci.rs` at 0 %; without it 1631/1716 = 95.05 %). **The author's coverage numbers are reproduced.** Line coverage is high and several behaviors it "covers" are unasserted (section 6) |
| Real Stockfish | binary `third_party/stockfish/build/extract/stockfish/stockfish-macos-universal`; `id name Stockfish 19`; UCI_Elo range read by the wrapper `(1320, 3190)`; probe with `BinarySource::Bundled` passed the id check and `PlanReport { clamped: false, differs_from_pinned: false }` | `[RAN]` |
| Cancel/drain stress, real Stockfish | 150 x (search 5 s, drop the future after 0 to 30 ms, then a mate-in-one search at depth 6 must return `a1a8`), alternating `new_game`; 150 x (`stop()` at 0 to 4 ms after a short search, then a 150 ms search must not be `stopped_early`) | `[RAN]` 0 bad of 300. No stale `bestmove`, no leaked `stop` |
| Orphan check | probe starts a 10-minute real Stockfish search, then SIGKILLs itself | `[RAN]` no `stockfish-macos-universal` process 1 s later: Stockfish exits on end-of-file mid-search. This closes the "kill -9 of the parent" half of CT-EN-020 for the real binary (the author did not run it) |
| Zombies | `ps -axo pid,ppid,stat,command` after all stress runs | `[RAN]` no leftover engine or `Z` process |
| Panic-site grep | `grep` for `unwrap`, `expect`, `panic!`, indexing in non-test `src` | `[OBS]` none; workspace lints `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` are denied. **Two panics exist anyway** through `Ord::clamp` and `Instant + Duration` (EF-02, EF-03), which those lints do not see |
| Hand mutation | 48 mutants, scratch copy | `[RAN]` section 6 |

## 2. Gate check

The author recorded GATE-106 as G1 (one crate, edges and 5.7 contract unchanged). Checked, not approved:

- No new crate edge. `Cargo.toml` depends on `chess-rules`, `tokio` (features `process, io-util, sync, time, rt, macros`), `thiserror`, `tracing`; dev: `proptest`, `tempfile`. Matches SDD 3.4 `[OBS Cargo.toml]`.
- Additions beyond the SDD tables, all disclosed by the author (F-E5, FIND-E3, FIND-E6): error variants `Busy`, `Closed`, `InvalidConfig`; `fake_uci` built by a plain `cargo build` without `required-features`; `test-support` feature as the only gate for `FakeEngine`; no `fake-uci-bin` or `stockfish-tests` features; `ProcessSpawner::with_args` public. None changes the 5.7 meaning. **FIND-E4 (`Engine` is not dyn-compatible) is an interface question for `chess-app`: if the app needs `dyn Engine` it becomes G2.** I agree it is not decided here.
- The reversed order in the driver `select!` (SDD 3.10.2) is a design deviation that should be recorded as an SDD amendment before the SA desk review (EF-12).

Nothing in this review approves GATE-106.

## 3. Findings

### Major

#### EF-01 [Major][CONFIRMED] Continuous engine output starves the soft-stop and hard-kill timers, so ENG-4 is not enforced against a chatty hung engine
- Location: `src/driver.rs:631-699` (`search_loop`). The `select!` is `biased`; the order is `reply.closed()`, command channel, `read_msg()`, then `soft`, then `hard` (timers last, lines 686-698). SDD 3.10.2 prescribes the same order, so the code conforms to a design that has this flaw.
- Mechanism `[INF]`: a biased `select!` polls branches in order and stops at the first ready one. While `read_msg()` always has data, the timer branches are never reached. `tokio` coop budget does not help: a poll of a timer branch after the budget is spent returns `Pending` again on the next task poll.
- Reproduction `[RAN]` (scratch probe `info_flood_timeout`, multi-thread runtime): scripted engine answers `go` with an endless stream of `info depth 1 score cp 1 pv e2e4` lines and never sends `bestmove`. Config: handshake 300 ms, soft grace 80 ms, hard grace 300 ms, `movetime` 100 ms. Expected `SearchTimeout` at about 400 ms. Observed: `search` still pending after 10.0 s (my own outer timeout fired). Soft `stop` is never written either.
- Related `[RAN]` (probe `newline_flood`): the reader is also slow on bare newlines. 4 MiB of `\n` took 3.8 s and 16 MiB took 15.1 s (about 1 MiB/s), against 1.5 s for 16 MiB of ordinary `info` lines. Cause `[INF]`: `LineSplitter::next_line` does `self.buf.drain(..=pos)` per line (`src/uci/lines.rs`), a memmove of the rest of the 4 KiB chunk for every empty line.
- Impact: ENG-4 ("a hung engine is killed 2 s after the limit") and the SDD 3.4/3.9 deadlines do not hold for an engine that keeps printing. Real Stockfish prints a few lines per second, so this is not a production-likelihood defect with the bundled binary. It is a defect in the one component whose job is to bound a misbehaving engine, and `CHESS_STOCKFISH` lets any binary in.
- Verify the fix by: a driver test with the endless-`info` script above that expects `SearchTimeout` within limit + grace + margin (this test fails today); the same for a flood of empty lines; a re-run of CT-EN-013.
- Trace: SDD 3.10.2 step 4, 3.4 state diagram (`Searching --> Dead: T plus 2 s`), ENG-4. The SDD pseudo-code needs an amendment as well as the code.

#### EF-02 [Major][CONFIRMED] A `Duration` that passes validation panics the driver task and kills the instance
- Location: `src/driver.rs:620-622` (`TokioInstant::now() + req.limits.movetime`, then `base + grace`); `src/types.rs:103-114` (`SearchLimits::validate` rejects zero only, no upper bound); `SearchLimits` fields are public (`UT-EN-004` itself says public fields exist).
- Reproduction `[RAN]` (probe `movetime_max max`): `SearchLimits { movetime: Duration::MAX, .. }`; `limits.validate()` returns `Ok(())`. The driver task panics: `tokio-1.54.1/src/time/instant.rs:161:27: overflow when adding duration to instant`. The caller gets `Err(Closed)` for this and every later call; `health()` stays `Searching` forever; no restart is possible. Values up to about 10^10 s worked (probe `huge`, `year`); the overflow starts near the platform `Instant` range.
- Why it matters: "no limit" expressed as `Duration::MAX` is a natural thing for the analysis caller ("refined until the user leaves", SDD 3.6.1) to write. `select_move` cannot trigger it (`movetime_ms` is `u32`); direct `Engine::search` callers and the analysis lanes can.
- Impact: permanent, silent loss of an engine instance (and of the shared analysis instance behind the arbiter) from a value the validator accepted. A panic in an actor task is also invisible to `health()`.
- Verify the fix by: `SearchLimits::validate` (and the `format_go` conversion) rejecting or saturating above a stated cap; a unit test with `Duration::MAX`, `Duration::from_secs(u64::MAX)` and the cap boundary; a driver test that the instance is still `Ready` afterwards.
- Trace: SDD 3.3 ("movetime is mandatory ... wall-clock bound"), 4 ("constructors validate"), DD-EN-002, NFR-3.

#### EF-03 [Major][CONFIRMED] The FF-7 / EI-EN-002 fitness function cannot fail hosted CI: every real-Stockfish test skips there and a skip counts as a pass
- Location: `tests/real_stockfish.rs:30-68` (`binary()`, `require()`), `tests/real_cancel_latency.rs:47-60`, `ci/check.sh`, `.github/workflows/ci.yml`.
- Observation `[OBS]`: a missing binary prints `SKIPPED ...; not a pass` to stderr, appends to `target/tmp/stockfish-integration-status.txt` and returns Ok. It panics only with `CHESS_REQUIRE_STOCKFISH=1`. `grep -rn "REQUIRE_STOCKFISH\|fetch.sh\|stockfish" ci .github` finds nothing: neither `ci/check.sh` nor `ci.yml` sets the variable, fetches the binary or reads the status file. The binary is git-ignored build output (`third_party/stockfish/.gitignore` ignores `/build/`).
- Impact: on the hosted clean-machine run, EI-EN-001 to EI-EN-008 all pass vacuously. SDD 3.10.12 and the 6-row for EI-EN-002 promise "fails CI on a mismatch, so a change of the engine pin without a change of the constant cannot merge". That is not true of the hosted run. `ci.yml` itself says the local `ci/check.sh` is the gate of record (chess-85m option C); on Brian's Mac the binary exists and the tests run (`[RAN]`, status file shows `RAN` for all eight), but nothing enforces that the binary is present there either. A deleted `build/` directory turns the gate green.
- Verify the fix by: `CHESS_REQUIRE_STOCKFISH=1` exported in `ci/check.sh` (and a fetch step or an explicit documented skip in `ci.yml`); a CI run with the binary removed that goes red.
- Trace: SDD 3.10.12 "Check", section 6 (EI-EN-002), FF-7, architecture 4.6.
- Note: this is a CI/process defect, not a code defect. It sits here because the SDD makes the test the enforcement mechanism for DD-EN-015.

#### EF-04 [Major][CONFIRMED] A lost wakeup in the analysis arbiter: an interactive request can fail to preempt a background search
- Location: `src/arbiter.rs:99-108` (`interactive`: reads `running`, calls `engine.stop()`), `src/arbiter.rs:111-140` (`background`: stores `running = BACKGROUND` at 118, then sends its `Search` command), `src/driver.rs:277` (`Cmd::Stop => {}` when no search runs).
- Mechanism `[INF]` confirmed by the run below: `background` publishes `running = BACKGROUND` before its `Cmd::Search` is on the channel. An interactive request on another worker that sees `BACKGROUND` calls `engine.stop()`; the `Stop` lands in the unbounded channel **before** the background `Search`. The driver, idle, discards it ("nothing is running"), then starts the background search, which runs to its full `movetime`. The comment at 119-121 claims the window is closed; it is not, because the stop is consumed by the wrong state.
- Reproduction `[RAN]` (probe `arbiter_race`, real `fake_uci` child behind `EnginePool::analysis`, 4-worker runtime; background `movetime` 100 ms, interactive starts 0 to 400 us later): 1500 iterations: 1340 `Preempted`, 154 other (interactive won the lock first), **6 lost preemptions**, each with the interactive request waiting 131 to 141 ms for the whole background search (spin offsets 20 to 45 us).
- Impact: Training live analysis waits up to the background `movetime` (200 ms per ply in the SDD) instead of preempting. No data loss. About 0.4 % of races on this machine, so it would be seen in practice.
- Verify the fix by: the probe above as a repeat test (zero lost in a few thousand iterations); CT-EN-040 stays green. The same hazard exists for any caller that calls `stop()` before the driver has dequeued its `Search` (the Computer handle too): `stop()` has no memory.
- Related, **SUSPECTED, not reproduced**: `background` drops `lock` at line 131 but its `_running` guard (line 128) only drops at the end of the function. If the next waiter acquires the lock and stores `INTERACTIVE` in that gap, the guard then stores `NONE`, and a later `stop(Interactive)` is a no-op. The window is nanoseconds. Also `Arbiter::stop(priority)` reads `running` then sends: a lane's `stop()` can hit the other lane's search if the lanes swap in between.

### Minor

#### EF-05 [Minor][CONFIRMED] `OptionPlan::build` panics on a handshake whose `UCI_Elo` range is inverted
- Location: `src/plan.rs:87` (`elo.clamp(min, max)`).
- Reproduction `[RAN]` (probe `elo_inverted`): scripted engine advertises `option name UCI_Elo type spin default 2000 min 2000 max 1000`, personality `uci_elo = Some(1500)`. Output: `panicked at plan.rs:87:36: min > max. min = 2000, max = 1000`. `warm()` and `search()` return `Err(Closed)`; `health()` is `NotStarted`.
- Impact: engine handshake output is untrusted input (SDD 3.10.1 "no panics"; SWE-001 6, ERR/INT intent). Real Stockfish cannot do this and a `Bundled` binary must pass the `id name` check, so it needs a foreign override binary. The instance is lost permanently with a panic message on stderr. `uci_elo_range()` (`src/uci/options.rs:80`) does not check `min <= max`.
- Verify the fix by: a handshake with `min > max` returns `InvalidPersonality`; a unit test in `tests/plan.rs`; the same for equal bounds.

#### EF-06 [Minor][CONFIRMED] `shutdown` is not bounded by the 1 s grace while the driver is starting, preparing or draining
- Location: `src/driver.rs:384-410` (`expect`, 5 s per call), `src/driver.rs:428-440` (`start`, one restart), `src/driver.rs:704-723` (`drain`), `src/driver.rs:269-317` (`run` serves `Shutdown` only between commands or inside `search_loop`); `src/pool.rs:120-131` (`shutdown` awaits instances one after another).
- Reproduction `[RAN]` (probe `shutdown_hang`, default config): engine never answers `uci`; `warm()` pending; `shutdown()` called 100 ms later returns after **9.9 s** (two 5 s handshake attempts). SDD 3.10.10: "waits up to 1 s ... the web server's 5 s stop budget leaves room".
- LIKELY: `EnginePool::shutdown` and `release_game` shut instances down sequentially, so several stuck instances add up (each up to 2 x `shutdown_grace` after the quit write even in the best stuck case).
- Impact: application stop can overrun the 5 s budget when an engine is hung at start. No resource leak: the kill still happens.
- Verify the fix by: a driver test with `no-uciok` and an immediate `shutdown`, asserting a bound near `shutdown_grace`; a pool test with several stuck instances.

#### EF-07 [Minor][CONFIRMED] Writes to the engine have no deadline and positions have no size cap, so a non-reading engine hangs `search` and `shutdown` forever
- Location: `src/driver.rs:211-216` (`write_line`: `write_all(..).await` outside any timer), `src/uci/format.rs` (`format_position` builds a line of any length), `src/types.rs:152-160` (`SearchRequest::validate` does not bound `moves`).
- Reproduction `[RAN]` (probe `write_block`): engine stops reading stdin after the handshake; request with a 200 000-move `SearchPosition::Moves`; `movetime` 100 ms, hard grace 300 ms. `search` still pending after 5 s; `shutdown()` still pending after 3 s. The hard-kill branch is not reachable because the actor is blocked inside the `write`, not inside the `select!`.
- Impact: reachable only with an absurd move list (real games are a few thousand plies at most, and a pipe takes 64 KiB). It is still the one place where the "every wait is bounded" claim fails with no recovery path.
- Verify the fix by: bounding the write by the hard deadline (or capping `moves`), and a test with the script above.

#### EF-08 [Minor][CONFIRMED] `SearchResult.pv`, `score` and `depth` can describe a different move from `best`
- Location: `src/driver.rs:752-824` (`Accum::push` skips bound lines; `finish` takes `pv` from the last accumulated lines, not from `best`).
- Reproduction `[RAN]` (probe `pv_vs_best`): script `info depth 5 score cp 10 pv e2e4 e7e5`, `info depth 6 score cp 50 lowerbound pv d2d4`, `bestmove d2d4`. Result: `best = d2d4`, `pv[0] = e2e4`, `depth = 5`, `score = Cp(10)`. **Real Stockfish** (probe `real_pv_best`, 300 searches of 5 to 45 ms on five positions): `best != pv[0]` in **1 of 300**.
- Impact: the score the app shows or stores as the evaluation of the chosen move (MOVE_EVAL, UR-15 normalisation) belongs to another move in about 0.3 % of short searches. SDD 3.6.1 says `search -> {best, pv, lines, score, depth}` without saying they agree, so this is a contract gap as much as a defect.
- Verify the fix by: either document that `pv` is the last completed iteration (and that `best` may differ), or reconcile (take the final bound line when it names `best`); a test with the script above.

#### EF-09 [Minor][CONFIRMED occurrence, cause LIKELY] Real-timer tests with 300 ms margins are fragile; `ct_en_054` failed on my first run
- Location: `tests/common/mod.rs:366-375` (`fast_cfg`: handshake 300 ms, soft 80 ms, hard 300 ms, drain 300 ms, shutdown 200 ms); `tests/process.rs:252-259` (`ct_en_054`).
- Observation `[RAN]`: first `cargo test -p chess-engine` run: `ct_en_054_dropping_every_handle_kills_and_reaps_the_child` failed at `tests/process.rs:258` (`assert!(common::pid_alive(pid))` right after a successful `warm()`). The next 15 full runs, 15 runs of `tests/process.rs` alone, 12 under 28 CPU-burner processes on 14 CPUs, 4 after `cargo clean -p chess-engine`, and 5 after re-signing `fake_uci` all passed.
- Cause, by measurement and arithmetic `[RAN][INF]`: the test takes `sp.pids().first()`. If the first spawn misses the 300 ms handshake, the driver (by design, CT-EN-011b) kills it and restarts, so the recorded first PID is the dead one and the assertion fails although `warm()` succeeded. A never-seen executable pays a first-exec cost on this Mac: **130 to 204 ms** for a trivial Rust binary and for a copy of `fake_uci` with a new signature, against 2 ms on every later exec `[RAN]`. Nine tests in `process.rs` start in parallel. This also fits the author's FIND-E13 (2): "6 failures in 0.68 s" is two 300 ms attempts plus overhead, right after editing `fake_uci`.
- Not reproduced: I could not push first-exec past 300 ms on demand. This is the most likely cause, not a proof.
- Related: SDD 3.11 and 6 require paused time ("no real sleeps in timeout tests"); the author disclosed that `test-util` is not admitted (FIND-E2). Ten `sleep(` calls and several upper-bound timing assertions (`ct_en_003` < 500 ms, `ct_en_013` < limit + grace + 400 ms) remain. They will flake on a loaded hosted runner.
- Verify the fix by: asserting on the last PID, or on `sp.pids().len()` with a handshake timeout of at least a few seconds in process-spawning tests; or admit `test-util` (G2 per the author) and pause time in the scripted tests.

#### EF-10 [Minor][CONFIRMED by mutation] Behaviors the SDD specifies are not asserted by any test
Hand mutants that survived (details in section 6): drain failure does not kill the child (M12); a child that exits between searches is not noticed by `try_wait` (M31); stderr redirected to the log instead of null (M22); `error_multipv == 2` boundary (M05); the non-error branch forwarding the caller's `multipv` instead of 1 (M54); `Preempted` returned for any `stopped_early` result, not only a preemption (M46b); background not waiting on the waiting-interactive count (M19).
- Impact: the state-diagram edges `Draining --> Dead` and `Idle --> Dead: process exit seen by try_wait` (SDD 3.9, 3.10.4 first bullet) and the stderr rule of 3.10.5 have no test. The first one is a stale-`bestmove` hazard if it regresses.
- Verify the fix by: tests that kill each mutant: a script that never answers after `stop` plus an assertion that the process is killed and the next search restarts; kill the child between two searches and assert one transparent restart and no error; assert the child's stderr disposition.

#### EF-11 [Minor][CONFIRMED by mutation] The error-pick replay is unpinned (author FIND-E10 confirmed)
- Location: `src/select.rs:33-36` (`next_f64`: top 53 bits), `src/select.rs:96` (`1 + draws.next_u64() % (count - 1)`); `tests/select.rs` UT-EN-037/038; vectors `tests/data/splitmix64_vectors.txt` (byte-identical to chess-domain's, `[RAN] cmp`).
- Mutation `[RAN]`: M41 (`next_f64` uses the low 53 bits instead of the top 53) and M42 (mirrored pick `count - 1 - x % (count - 1)`) both **survive all 114 tests**. Either change alters every replayed error pick for every stored seed, and neither alters any distribution that UT-EN-037/038 measures (they only check self-consistency, about 50 % rate, and "never line 0").
- Impact: reproducible replays (architecture 4.6, D-34 intent) rest on two formulas nobody pinned. Only `next_u64` is in the shared vector file.
- Verify the fix by: golden `(seed, error_rate, line count) -> (roll, picked index)` vectors in the shared file (an SA decision, as the author said), and a test that M41 and M42 fail.

#### EF-12 [Minor][CONFIRMED] `FakeEngine` is faithful enough for flows but misleading in four ways the app tests may rely on
- Location: `src/fake.rs`.
- Observed `[RAN]` (probe `fake_probe`): (a) after a dropped `Hang` search `health()` stays `Searching` (real engine: `Ready` after drain); (b) `stop()` during `FirstLegalMove` + latency does not cut the search: `stopped_early = false`, returned after 280 ms; only `Hang` honours `stop`; (c) `multipv = 200` is accepted (`validate(u8::MAX)`), the real cap is 8 (`EngineConfig::multipv_cap`); (d) `movetime = 3600 s` is ignored (returns in 45 us). Also `SearchResult.elapsed` is wall-clock, so two results are never `==`.
- Impact: gateway and session tests of "move now", health after cancel and request validation written against the fake can pass while the real driver differs. The "deterministic" claim holds for `best`, `pv`, `lines`, `score`, `depth` only.
- Verify the fix by: tests per item above; or state the gaps in the `FakeEngine` doc comment.

### Info

#### EF-13 [Info] Command ordering, judged against SDD 3.10.2
The author put `reply.closed()` before the command channel (`src/driver.rs:635-643`), against SDD 3.10.2 (commands first, then the dropped future). I judge the deviation **correct and necessary**: with the SDD order a search sent right after a dropped one is answered `Busy` while the abandoned search drains, which contradicts SDD 3.10.3, CT-EN-004 and CT-EN-005. The SDD text is internally inconsistent and needs an amendment (G0/G1 with the SA); the author already flagged it as FIND-E1. Stress `[RAN]`: 300 cancel/stop races against the real binary, 0 stale results. Residual hazards I looked for and did not find: a queued `Stop` leaking into the next search (FIFO order plus the idle-loop discard prevents it, ct_en_006 and my stale-stop probe agree); a `Stop` during `prepare` is applied right after `go` (F-E2, documented) so ENG-5 is not met while the engine is starting (up to 10 s). Two consequences to note: (1) `Stop` that precedes its `Search` on the channel is lost, the root of EF-04, and applies to any `stop()` before the driver dequeues the search; (2) FIND-E5 (no stop escalation before limit + 2 s) is real: ENG-5's 200 ms holds only for a cooperating engine.

#### EF-14 [Info] Engine-supplied text can reach a `Debug` print
`EngineError::WrongVersion { found }` (truncated to 64 chars, `src/driver.rs:486`) and `InvalidPersonality { option }` carry text from the engine; `Display` is clean (tests UT-EN-047, CT-EN-017 check it) but `#[derive(Debug)]` prints both. If `chess-app` logs `{:?}` of an `EngineError`, engine text reaches the log (SDD 3.6.3 and NFR-10 intent). No path leaks anywhere: `SpawnFailed`, `BinaryMissing` and the rest carry no path `[OBS error.rs]`.

#### EF-15 [Info] Hostile-engine resource use (outside the v1 threat model, FIND-016)
`[RAN]` probe `option_flood`: 1 000 000 `option` lines in the handshake are accepted into an unbounded map until the 5 s timeout: 553 MB peak resident in 10 s. No cap on the number of options or on the `pv` length per line (the 64 KiB line cap bounds each line). The command channel is unbounded by design (F-E2). Only a foreign override binary can do this.

#### EF-16 [Info] Smaller observations
- `stopped_early` is set whenever a `stop` was written, even if the engine had already finished by itself (`src/driver.rs:676`).
- `computer_side` keeps the first personality for an existing key and ignores a different one silently (PLY-4 by design, F-E9).
- `BinaryLocator::from_environment` reads `CHESS_STOCKFISH` in every build and an override skips the `id name` check (`src/locate.rs:38-41`, SDD 3.10.6 and FIND-EN-010 accept this; D-18).
- `fake_uci` takes its advertised `UCI_Elo` range from `PINNED_UCI_ELO_RANGE`, so tests that compare "pin versus handshake" with the fake are circular; only EI-EN-002 against the real binary tests the pin. `PINNED_UCI_ELO_RANGE` is a proposal nobody approved (`src/pinned.rs`, FIND-E8, held by the PM for Brian). It equals the live handshake `[RAN]`.
- `Handle::try_current` failure in `UciEngine::new` is reported as `Closed` (`src/driver.rs:111`), not as a construction error.

## 4. Answers to the specific questions in the task

| Question | Answer |
|---|---|
| Cancel races, lost wakeups | One confirmed: EF-04 (arbiter). Driver cancel/drain/stop clean in 300 real races (section 1). `reply.closed()` is cancel-safe; `read` is cancel-safe with the partial line in the splitter `[OBS]` |
| Command ordering vs SDD 3.10.2 | EF-13: deviation justified, SDD needs the amendment |
| Zombies, leaked children, process groups | None found. `die()` always `start_kill` + `wait`; `kill_on_drop`; own process group confirmed by the author's `ps` check; SIGKILL of the parent leaves no Stockfish `[RAN]`. A hung `wait` after the grace falls back to `kill_on_drop` `[INF]` |
| Handshake and timeout edges | EF-06 (shutdown during start 9.9 s), EF-01 (timers starve), EF-07 (unbounded write). `uciok`/`readyok` timeouts and the single restart behave as designed (CT-EN-011, 011b) |
| Hostile or oversized output | Long line: handled (`BadOutput`, tested). Flood of lines or newlines: EF-01. Option flood: EF-15. Malformed info/bestmove: parser total, no panic found in the 5 proptest properties; `bestmove (none)`, `0000` and garbage kill the child by design |
| Option range checks, `UCI_Elo` | `format_setoption` range-checks every spin against the advertised min/max (`src/uci/format.rs`, test UT-EN-017, mutant M28 killed) and the plan clamps for the search (M16b, M17 killed). The check is against the **handshake** range, not the pinned `1320..3190`; with real Stockfish both agree `[RAN]`. A hostile handshake can advertise a wider range, or an inverted one (EF-05) |
| Env/PATH | `env_clear` (M21 killed); locator never reads `PATH` (UT-EN-046); override must be absolute; `current_dir` = binary dir; stderr null (M22 **survived**, EF-10) |
| Error variants leaking paths | None leak a path; Debug can leak engine text (EF-14) |
| Panics | EF-02, EF-05 |
| Unbounded channels | Command channel unbounded by design; `updates` channel is the caller's bounded one with `try_send`. EF-15 for growth |
| FakeEngine determinism, error-pick | EF-12, EF-11 |
| Test quality | Section 6 |
| FIND-E13 transients | Section 7 |

## 5. Conformance statement

Standard: **none claimed.** SWE-001 section 6 applies SEI CERT by intent only. This is a CERT-informed review, findings only.

| Category | Count | Notes |
|---|---|---|
| Tool-checked | 4 | `cargo fmt --check`; `cargo clippy -D warnings` (workspace denies `all`, `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation`, `cast_sign_loss`), with and without `test-support`; `cargo test`; `#![forbid(unsafe_code)]` `[OBS lib.rs]` |
| Tool-checked, violations | 0 | clippy is clean, but it missed EF-02 and EF-05 (panics through `Instant + Duration` and `Ord::clamp`) |
| Manually reviewed | all of `src/` (about 2 500 lines) and the test files | by reading, with probes for each finding |
| CERT-intent areas reviewed by reading plus probes | INT/NUM (overflow: EF-02), ARR/STR/MEM/EXP (no `unsafe`, bounded line reader), ERR (EF-14), CON/LCK (EF-04, EF-06), FIO/ENV/SIG/POS (env, process group, stdin EOF), IDS (typed `format_*`, no concatenation of caller text), SEC/SER (no deserialisation) | no rule is declared satisfied |
| Not checked | `cargo deny`/`cargo audit` for this crate (not re-run; the author reports `ci/check.sh` green offline), `miri`, sanitizers, `cargo-mutants` (hand mutation only), fuzzing, Intel slice | |
| Overall | **NOT-DETERMINED** for any standard (none claimed) | |

Suppressions: the only `allow(` in the crate are in tests and `tests/common`, each with a written `reason` `[OBS grep]`.

## 6. Test adequacy and hand mutation

Tests: 114, all executed `[RAN]`. Skipped real-Stockfish tests count as passes when the binary is absent (EF-03). Assertions that cannot fail: none found. Tests with a single assertion that I read were adequate for their intent; `fz_en_001` has no assertion by design ("does not panic"). No test covers the three confirmed driver failures (EF-01, EF-02, EF-05) or the arbiter race (EF-04).

Hand mutation, 48 mutants applied one at a time to a scratch copy and run against the unit, scripted, process, pool and real-Stockfish tests (not `real_cancel_latency`). This is a sample chosen by me, not a mutation score. Killed 36; survived 12, of which 3 are equivalent (M01 `<` to `<=` on a continuous roll, M38 redundant closed-check before `go`, M55 reply to an in-flight search on shutdown); **9 real survivors**:

| Mutant | Survived | Meaning |
|---|---|---|
| M05 `error_multipv >= 2` to `> 2` | yes | boundary 2 untested |
| M12 drain timeout/EOF no longer kills the child | yes | `Draining --> Dead` untested (EF-10) |
| M19 background no longer waits for `waiting == 0` | yes | queue-yield untested |
| M22 stderr inherit instead of null | yes | SDD 3.10.5 untested |
| M31 `try_wait` check removed | yes | transparent restart between searches untested |
| M41 `next_f64` low bits | yes | replay unpinned (EF-11) |
| M42 mirrored error pick | yes | replay unpinned (EF-11) |
| M46b `Preempted` without the preempt counter | yes | any early stop reported as preemption |
| M54 non-error branch keeps caller's `multipv` | yes | SDD 3.10.8 `multipv = 1` untested |

Killed by: M02/M03 (UT-EN-037/038 and EI-EN-008), M06 to M08 (UT-EN-033), M10/M11 (CT-EN-013), M13/M37 (stop and preemption tests), M14 (CT-EN-073), M15 (CT-EN-017/052), M16b/M17 (UT-EN-021, CT-EN-075), M18 (CT-EN-033), M20 (CT-EN-040), M21/M23 (CT-EN-020), M24/M25/M51/M52 (locator tests), M26/M27/M28/M57 (parser and format tests), M30 (CT-EN-072), M32, M33, M34 (CT-EN-004/005), M35b (fake), M36, M40, M47, M48. Compile errors on the first form of M16, M35, M39, M46 were redone (M16b, M35b, M46b) or dropped (M39, throttle, not re-run).

Coverage claim check: the author's figures are correct as arithmetic (section 1). They do not imply behavior is asserted: M12 and M31 sit in covered lines.

## 7. The two transient failures (FIND-E13)

| Item | Result |
|---|---|
| EI-EN-005 (author: one failure after about 12 s, no message) | `[RAN]` 12 consecutive runs of the test binary, 100 samples each (1 200 samples), all passed; then 5 more under 28 CPU-burner processes on 14 CPUs, all passed; worst sample under load 25.8 ms against the 200 ms limit; plain-run maxima 0.15 to 0.26 ms. **Not reproduced.** The test can fail only through a failed search, `stopped_early == false`, or a latency over 200 ms; my 300-race stress against the real binary found no failed or mis-reported search. A crash of the real child at about iteration 48 would show up as `ProcessExited` (the author's panic messages now print it). Cause **unexplained**; I make no claim |
| `tests/process.rs` (author: 6 failures in 0.68 s right after editing `fake_uci`) | `[RAN]` I saw a failure of the same test file on my first run (`ct_en_054`, EF-09), not 6. Mechanism measured: first exec of a new executable costs 130 to 204 ms here against 2 ms afterwards; 2 x 300 ms handshake attempts is about 0.6 s, which matches 0.68 s. **Not reproduced on demand** (15 + 15 + 12 loaded + 4 after clean + 5 after re-sign: 0 failures). Cause **LIKELY** |

## 8. Verdict

**PASS WITH CONDITIONS** (desk review; reviewer's recommendation to the System Architect and Brian Prascher, not an approval). No Blocker. The crate builds, is fmt and clippy clean, and 114 of 114 tests pass `[RAN]`; against the real Stockfish 19 the cancel, drain, stop and orphan behavior held in every race I ran. The conditions are the four Majors, each CONFIRMED by a reproduction that fails today:

1. EF-01: timers starve behind a flowing engine (fix needs an SDD 3.10.2 amendment as well).
2. EF-02: unbounded `movetime` panics the actor.
3. EF-03: the FF-7 gate is optional in CI (set `CHESS_REQUIRE_STOCKFISH=1`; this touches `ci/`, outside this crate).
4. EF-04: arbiter lost preemption.

Each is either fixed with a test, or accepted as a named risk by Brian. Likelihood against the bundled Stockfish is low for EF-01 and EF-02; EF-03 is a process hole; EF-04 was seen in 0.4 % of races. The Minor items (EF-05 to EF-12) may be dispositioned with the Majors or deferred with an `OQ-`/`RSK-` id. Dispositions: all findings OPEN. A fix touching `ci/` or the SDD text is outside G1 of this crate and needs its own gate.

## 9. Not reviewed, not run, assumptions

- **Not run:** Intel/x86_64 slice and Rosetta; `Threads` above 1; `cargo-mutants`; `miri`; sanitizers (TSan would be the right tool for EF-04); fuzzing; branch/MC/DC coverage; `cargo deny` and `npm audit` (author's `ci/check.sh` run not repeated); a hosted-CI run; behavior on a loaded hosted runner; real-Stockfish runs with `CHESS_REQUIRE_STOCKFISH` unset on a machine without the binary (read from code and the author's note, not re-run; the skip path was exercised by reading).
- **Not analyzed:** Stockfish source (how it treats out-of-range `UCI_Elo`; only that it is silent, from the author's evidence `[UNV]`); `chess-app` consumers (they do not exist yet; EF-04 and EF-12 matter to them); the `chess-domain` side of the seed derivation; Mermaid (this document has none, so "Mermaid not machine-validated" does not apply).
- **Not reproduced:** the EF-04 guard-clobber variant, the EF-06 multi-instance sum, the 6-failure `process.rs` burst, the EI-EN-005 failure.
- **Assumptions:** probes use scripted in-process engines (tokio `duplex`) for failure injection and the real binary or `fake_uci` where stated; the scripted harness is mine, so a scripted result shows a driver property, not Stockfish behavior. The probe sources are in the scratchpad named in the header, outside the repo, and are not part of the deliverable; the commands and observed outputs above are the evidence. Timing figures are from one machine (Apple M4 Pro, 14 CPUs, macOS 26.7.1).
- No claim of SA, SE or human approval is made.
