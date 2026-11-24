# Step 2 evidence: chess-engine (chess-a0r.3.10)

Software engineer, Engineer stance, 2026-11-09, baseline B5. Task chess-a0r.3.10, autonomy run-to-gate (CR chess-a0r.19). Gate record: [gates/GATE-106-chess-engine.md](gates/GATE-106-chess-engine.md) (G1, OPEN, NOT APPROVED). Design: [SDD-chess-engine.md](../design/SDD-chess-engine.md) 1.0 BASELINED with the 2026-11-09 G0 notes. Tags: `[OBS]` read, `[RAN]` executed, `[INF]` inferred, `[UNV]` unverified. Nothing was committed, closed or pushed. No approval is claimed. This is the Engineer's evidence, not a Reviewer pass. `PINNED_UCI_ELO_RANGE` is a proposal that nobody has approved (see 6); this document does not repeat its value.

## 1. What was built

`crates/chess-engine` (about 3,300 lines of source and about 3,100 of tests `[RAN wc -l]`): the UCI process wrapper with lazy start, handshake, option plan, search, `stop`, soft and hard deadlines, cancel by dropping the future with a drain, failure recovery, and restart by the next request; a binary locator; an engine pool and a shared analysis arbiter with preemption lanes; the seeded error-pick move selector and the clock budget; a deterministic `FakeEngine`; a `fake_uci` test executable; the config and error types; and the `PINNED_UCI_ELO_RANGE` constant. `#![forbid(unsafe_code)]`. Manifest: `chess-rules` plus the admitted workspace entries `tokio` (features `process, io-util, sync, time, rt, macros`), `thiserror`, `tracing`; dev `proptest`, `tempfile`. Nothing outside `crates/chess-engine` (and `Cargo.lock`, shared with `chess-store`) was edited.

## 2. SDD entity to code map

| DD | Code | Notes |
|---|---|---|
| DD-EN-001 `Engine` | `src/engine.rs` (`Engine`), `src/types.rs` (`Health`), `src/pool.rs` (`EngineHandle`) | native async fn in trait; not dyn-compatible (FIND-E4) |
| DD-EN-002 types | `src/types.rs` | `UciMove`, `Score`, `PvLine`, `SearchResult`, `SearchInfo`, `SearchLimits`, `SearchPosition`, `SearchRequest`, `EnginePersonality`, `Health` |
| DD-EN-003 `uci` | `src/uci/{mod,parse,format,options,lines}.rs` | pure; `LineSplitter` is the bounded reader |
| DD-EN-004 `UciEngine` | `src/driver.rs` | actor, state machine, deadlines, drain, restart |
| DD-EN-005 `Spawner` | `src/spawn.rs` | `ProcessSpawner` (production), trait for tests |
| DD-EN-006 `BinaryLocator` | `src/locate.rs` | |
| DD-EN-007 `EnginePool` | `src/pool.rs` | `ComputerKey`, `GameKey` |
| DD-EN-008 `OptionPlan` | `src/plan.rs` | `PlanReport { clamped, differs_from_pinned }` |
| DD-EN-009 selector | `src/select.rs` | `SearchLimits::for_move`, `clock_budget`, `SplitMix64`, `select_move` |
| DD-EN-010 `AnalysisArbiter` | `src/arbiter.rs` | `AnalysisPriority` |
| DD-EN-011 `FakeEngine` | `src/fake.rs` | `test-support` feature or `cfg(test)` |
| DD-EN-012 `fake-uci` | `src/bin/fake_uci.rs` | always built (FIND-E3) |
| DD-EN-013 `EngineError` | `src/error.rs` | plus `ErrorClass` |
| DD-EN-014 `EngineConfig` | `src/config.rs` | |
| DD-EN-015 pin | `src/pinned.rs` | single source of the value |

The SDD "Code:" column (SDD section 4) was not updated: the work order forbids editing other documents. The SDD owner can take the paths above.

## 3. Tests (114, all passing) `[RAN]`

| File | IDs | n |
|---|---|---|
| `tests/types_config_error.rs` | UT-EN-001..006, UT-EN-050, UT-EN-060, CT-EN-060, UT-EN-061 | 10 |
| `tests/uci_protocol.rs` | UT-EN-010..019 (011b, 019b, 019c extra), FZ-EN-001 (5 `proptest` properties) | 18 |
| `tests/plan.rs` | UT-EN-020..026 | 7 |
| `tests/select.rs` | UT-EN-030..039 (031, 033b, 036b are `proptest`), UT-EN-062 | 13 |
| `tests/locate.rs` | UT-EN-040..047 | 8 |
| `tests/driver_scripted.rs` | CT-EN-001..006, 010, 011, 011b, 012 (+021), 013..017, 070..078 | 24 |
| `tests/pool.rs` | CT-EN-030..036, CT-EN-040..043 | 11 |
| `tests/process.rs` (real `fake_uci` child) | CT-EN-020, 020b, 051, 051b, 051c, 052, 053, 054, 055 | 9 |
| `src/fake.rs` inline | CT-EN-050 (6 tests) | 6 |
| `tests/real_stockfish.rs` (real Stockfish 19) | EI-EN-001, 002, 003, 004, 006, 007, 008 | 7 |
| `tests/real_cancel_latency.rs` (own binary) | EI-EN-005 | 1 |

SDD IDs not delivered as such: UT-EN-027 (difference-only sending) is a driver behaviour and is tested as CT-EN-074; CT-EN-007..009, 018, 019 are undefined in the SDD (FIND-JR3-013); EI-EN-003 covers mate in one and two, not three; the "kill -9 of the parent leaves no child" half of CT-EN-020 was not run (needs a separate parent process; the end-of-file exit it relies on is tested by CT-EN-020b and EI-EN-006). The "every stored `uci_elo` inside the range" test belongs to `chess-app`.

Tests found one defect in my first driver (FIND-E1, fixed): CT-EN-004 and CT-EN-005 failed with `Busy` because the command channel was polled before the dropped-future check.

Traceability (requirement to design to test): ENG-1 -> DD-EN-003/004/005/006 -> CT-EN-001, 010, 011, 020, 051, EI-EN-001; ENG-2 -> DD-EN-008/009/015 -> UT-EN-020..026, 061, EI-EN-002, EI-EN-008; ENG-3 -> DD-EN-002/009 -> UT-EN-030..034, EI-EN-004; ENG-4 -> DD-EN-004/013/014 -> CT-EN-011..016, 052, 055, EI-EN-007; ENG-5 -> DD-EN-001/004 -> CT-EN-003, 051c, EI-EN-005; IF-4 -> DD-EN-006 -> UT-EN-044, CT-EN-017; NFR-1 -> DD-EN-007 -> CT-EN-030, 031; NFR-3 -> DD-EN-002/003 -> UT-EN-010..019, FZ-EN-001; NFR-8 -> DD-EN-004 -> CT-EN-003, 070; PLAT-3 -> DD-EN-005 -> CT-EN-020; PLY-4 -> UT-EN-035; UI-1 -> UT-EN-031.

## 4. Commands and results `[RAN]` (2026-11-09, arm64, rustc/cargo 1.99.1)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy -p chess-engine --all-targets -- -D warnings` | `Finished`, no diagnostics. Also clean with `--features test-support` |
| `cargo test -p chess-engine` | 114 passed, 0 failed, 0 ignored (11 test binaries; per-binary counts in 3). Real-Stockfish status file after the run: EI-EN-001..008 each `RAN` |
| `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| `CHESS_CI_OFFLINE=1 ci/check.sh` | exit 0, `All gates passed.` Stages: fmt, build `--locked`, clippy, test (workspace), crate-edge, ECO checksums, `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`, with the RUSTSEC-2026-0441 `advisory-not-detected` warning), npm audit skipped (offline). The workspace was green with the concurrent `chess-store` edits at that moment; no interference seen |

Integration skip protocol `[RAN]`: with `CHESS_STOCKFISH=/nonexistent/stockfish` the eight real-engine tests passed as skips, printed `EI-EN-00n SKIPPED: the Stockfish binary is absent ...; not a pass`, and wrote the same lines to `target/tmp/stockfish-integration-status.txt`. With `CHESS_REQUIRE_STOCKFISH=1` and the binary absent, the tests panicked (CI can require the run). With the binary present each test writes `EI-EN-00n RAN`. The status file is the marker CI can read; a `SKIPPED` line means the real-engine slice did not run.

## 5. Coverage `[RAN cargo llvm-cov -p chess-engine --summary-only`, cargo-llvm-cov 0.9.1, line coverage as printed]

Target (SDD section 6): at least 90 % lines for `uci/*`, `plan.rs`, `select.rs`, `locate.rs`. Met.

| Module | Lines covered / total | Line % |
|---|---|---|
| `uci/*` (parse 97.50, format 95.65, lines 100, options 100) | 256 / 262 | 97.71 |
| `plan.rs` | 66 / 66 | 100.00 |
| `select.rs` | 76 / 77 | 98.70 |
| `locate.rs` | 42 / 46 | 91.30 |
| `driver.rs` | 521 / 576 (at the time of the first full run: 90.45 %) | about 90 |
| all `src/` except `src/bin` | 1631 / 1716 | 95.05 |
| TOTAL as printed (includes `src/bin/fake_uci.rs`) | 1631 / 1861 | 87.64 |

`src/bin/fake_uci.rs` shows 0 % because it runs as a child process with an empty environment (`env_clear` removes `LLVM_PROFILE_FILE`), so its counters are not merged; its behaviour is exercised by CT-EN-020, 051..055. The driver is covered by scripted tests, not by a percentage target (SDD section 6). No mutation testing was run (`cargo-mutants` not run) `[UNV]`. Branch and MC/DC coverage not measured.

## 6. Cancel latency against the real Stockfish 19 (ENG-5, 200 ms, EST TBD-12) `[RAN]`

Test `EI-EN-005` (`tests/real_cancel_latency.rs`). Machine: Apple M4 Pro, 14 CPUs, macOS 26.7.1, arm64 native slice of `stockfish-macos-universal` (`id name Stockfish 19`). One warm instance, `Threads=1`, `Hash=16`, full strength, position `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1`, movetime 10 s, `stop()` called 150 to 350 ms into the search (five offsets), time measured from the `stop()` call to `search` returning the result with `stopped_early = true`. n = 100 per run. Each sample passed the assertion (`stopped_early`, non-empty best move) and the 200 ms limit.

| Run | min | p50 | p90 | p99 | max | mean |
|---|---|---|---|---|---|---|
| 1 (first full `cargo test`) | 81.75 us | 124.6 us | 145.6 us | 1.035 ms | 1.035 ms | 135.0 us |
| 2 (final `cargo test -p chess-engine`) | 92.25 us | 120.4 us | 140.3 us | 153.0 us | 153.0 us | 121.0 us |
| 3 (separate run, after the skip-path check) | 92.9 us | 120.0 us | 136.0 us | 156.4 us | 156.4 us | 120.8 us |

The criterion "cancel <= 200 ms" is **met by a margin of over two orders of magnitude** for the cooperating real engine on this machine. The SDD design target is 100 ms. Not measured: the x86_64 slice under Rosetta, `Threads` above 1, a loaded machine, an engine that ignores `stop` (that case is bounded by the hard kill, FIND-E5), the end-to-end time through `GameSession` and the UI. Four further runs of the test passed; one earlier run of the same test failed after about 12 s for a reason I did not capture (FIND-E13).

## 7. Findings

Severity: Blocker (stops the gate), Major, Minor, Info. Engineer's list, not a Reviewer pass. **No blocker, no protocol or stored-format issue.** Design conflicts and interface questions are marked.

| ID | Sev | Finding | Status / action |
|---|---|---|---|
| FIND-E1 | Minor | Defect in my first driver: with the command channel polled before the dropped-future check, a search sent right after a dropped search got `Busy` instead of waiting for the drain. Found by CT-EN-004 and CT-EN-005 | **FIXED** in `src/driver.rs` (the dropped-future branch is first in the biased `select!`). **Design conflict (small):** SDD 3.10.2 lists the command channel first; the SDD text should say the dropped-future check comes first. For the SA desk review |
| FIND-E2 | Minor | `tokio` `test-util` is not in the admitted feature set (DL2-5), so `tokio::time::pause` is unavailable. SDD 3.11 and 6 call for paused time. Timeout tests use real timers with shrunken `EngineConfig` values (tens to hundreds of ms) | Choice made; stable over the repeated runs (24 scripted tests x at least 6 runs). Needs a decision if paused time is wanted: admit `test-util` as a dev-dependency feature (feature of an admitted crate, G2 under SWE-001 as the lookup entries are). Owner: Brian, via PM |
| FIND-E3 | Minor | **Design conflict with SDD 3.7 and 3.10.11.** `fake_uci` is built by a plain `cargo build` (no `required-features`, no `fake-uci-bin` feature), and there is no `stockfish-tests` feature. Reason: integration tests that spawn `fake_uci` need `CARGO_BIN_EXE_fake_uci` and would not run, or would not compile, in a plain `cargo test`/`ci/check.sh`. Real-Stockfish tests skip visibly instead of hiding behind a feature. `fake_uci` takes its fault mode from `argv[1]` (production spawns never pass arguments) | Implemented this way; for the SA desk review. Packaging must not bundle `fake_uci` (F-8.6 check) |
| FIND-E4 | Minor | **Interface question for the app owner.** `Engine` uses native async-fn-in-trait, so it is not dyn-compatible. If `chess-app`'s `EngineFactory` (seams.rs) is meant to return `Box<dyn Engine>`, it cannot. Generics or an enum over `EngineHandle` and `FakeEngine` work. `FakeEngine` and `EngineHandle` both implement `Engine` | Raised, not decided. Gate stays G1 because the 5.7 contract is unchanged in meaning. If `chess-app` needs `dyn`, that is an interface change (G2) |
| FIND-E5 | Minor | **SDD gap.** The hard-kill deadline is limit + 2 s whether or not `stop` was sent (SDD 3.10.2, as implemented). An engine that ignores `stop` after a long movetime (analysis, 10 s) is not killed until limit + 2 s, so ENG-5's 200 ms holds only for a cooperating engine. CT-EN-013 shows the limit + 2 s case with a short movetime | Implemented as written. Suggest an SDD amendment: hard kill at the earlier of limit + 2 s and stop time + an acknowledgement bound. Owner: SA/SE |
| FIND-E6 | Info | Additive error variants `Busy`, `Closed`, `InvalidConfig` (all `Internal`) and an `ErrorClass` accessor, beyond the SDD 3.6.3 table. `Preempted.partial` is `Option<Box<SearchResult>>` | Choice F-E5 |
| FIND-E7 | Info | Stockfish 19 with `Threads=1` and `go depth 14` did not find the mate in two in `kbK5/pp6/1P6/8/8/8/8/R7 w - - 0 1` (score cp 540), although `go ... searchmoves a1a6` returns `score mate 2` [RAN, shell]. An independent oracle (`chess-rules`) confirms the position is a mate in two. This is the engine's behaviour, not a wrapper defect. EI-EN-003 therefore uses two other positions that the oracle proves are mate in two and the engine solves | Closed. EI-EN-003 does not cover mate in three (the SDD names it): gap |
| FIND-E8 | Minor | `PINNED_UCI_ELO_RANGE` equals the live handshake range (EI-EN-002 passes, `differs_from_pinned` false) but the value is still a **proposal nobody approved** (chess-a0r.3.4). It is declared once, in `src/pinned.rs`, with that said in the comment; no test or document repeats it | PM-held. Brian decides |
| FIND-E9 | Info | `fake_uci` coverage is not measured; under `cargo llvm-cov` the instrumented child adds `__LLVM_PROFILE_RT_INIT_ONCE` to its environment, so CT-EN-020 tolerates `LLVM_`-prefixed names and still fails on any other variable | Choice made |
| FIND-E10 | Minor | **Replay-stability question for the SA.** The error pick depends on definitions the SDD leaves open: `next_f64` (top 53 bits of one `next_u64` draw) and the pick `1 + draw % (lines - 1)` (SDD formula). The shared vector file pins `next_u64` only. A later change to either definition silently changes every replayed error pick for a seed. No golden test pins (seed -> roll, pick) | Raised. Suggest the SA decide whether those two definitions join the FF-8 vector file |
| FIND-E11 | Info | `ComputerKey { game: GameKey(u128), side: chess_rules::Side }` and `AnalysisPriority` are my pool API choices (the SDD says `key = (game_id, side)` and `priority`). `chess-app` converts its `GameId` (for example from the UUID's 128 bits) | Choice F-E9 |
| FIND-E12 | Info | `Cargo.lock` is shared with the concurrent `chess-store` work; its diff contains both crates' entries (tokio, mio, signal-hook-registry, rusqlite and others) | Not mine to split |
| FIND-E13 | Minor, unresolved | Two transient failures I could not explain and did not reproduce: (1) one run of `EI-EN-005` failed after about 12 s with no captured message (the next eight runs passed); I then added the iteration number and error to its panic messages; (2) one run of `tests/process.rs` right after editing `fake_uci` showed 6 failures in 0.68 s and passed on every later run (about 12). Possible causes: a rebuild race, or a machine hiccup `[UNV]` | Watch in CI. A failure of EI-EN-005 now prints the run number and error |
| FIND-E14 | Info | Out-of-range `UCI_Elo` handling stays in the wrapper: `format_setoption` rejects any value outside the advertised range, and the plan clamps `UCI_Elo` for the search only (stored value untouched), because Stockfish accepted min-1 and max+1 silently (stockfish19-fetch-results.md section 4). OQ-EN-002 (where the clamp sits) remains open and is not closed here | Implemented per SDD 3.10.7 |

## 8. Choices inside the SDD's freedom (for the Reviewer and the desk review)

- **F-E1** `UciEngine` is the cloneable handle onto the actor (the SDD has both `UciEngine` and `EngineHandle`); `EngineHandle` is a thin enum over a `UciEngine` or an arbiter lane and is what the pool returns.
- **F-E2** The command channel is unbounded, so `stop()` never blocks and is never dropped. Searches sent while one runs get `Busy` immediately. A `stop` in the channel when nothing runs is ignored; one queued during `prepare` takes effect right after `go`.
- **F-E3** See FIND-E3.
- **F-E4** `prepare` (options, `ucinewgame`, `isready`) is not cancellable; it is bounded by the handshake timeout. A drop during `prepare` is honoured right after it: no `go` is sent.
- **F-E5** See FIND-E6.
- **F-E6** `multipv_cap = 8` (`ESTIMATE`, mine). `line_cap` 64 KiB, soft 500 ms, hard 2 s, drain 2 s, shutdown grace 1 s, update interval 100 ms as in SDD 3.11. Threads and Hash placeholders as AMD-12 (`ESTIMATE`).
- **F-E7** `next_f64` is the top 53 bits of one draw divided by 2^53 (see FIND-E10).
- **F-E8** `depth_limit == 0` means no depth cap. A zero resulting movetime in `for_move` is an `InvalidRequest`, not a 0 ms search. A result with no `info` line has `score = Cp(0)`, `depth = 0`, `pv = [best]`.
- **F-E9** Pool API shapes (FIND-E11); an existing key keeps its first personality (PLY-4); `hash_in_use_mb` counts configured instances.
- **F-E10** Option sending is difference-only: the applied map is seeded with the advertised defaults. `Threads` and `Hash` are planned only when advertised; `UCI_Elo`, `UCI_LimitStrength` and `Skill Level` are required where the profile needs them. `bestmove (none)`, `0000` and an unparsable `bestmove` kill the child and return `BadOutput`; any other unknown line is ignored.

## 9. Not run, not analyzed, assumptions

- **Not run:** Intel/x86_64 (no hardware; the x86_64 slice under Rosetta was not exercised by this crate's tests); `kill -9` of the parent process; mate in three; mutation testing; branch and MC/DC coverage; the offline start of Stockfish with networking disabled (architecture 8.1) and the packaging `lipo` check (packaging tasks); `Threads` above 1; a loaded or throttled machine; tests with `RUSTFLAGS` other than the CI value (the workspace `ci/check.sh` sets `-D warnings` and passed).
- **Not analyzed:** the Stockfish source (how `UCI_Elo` out-of-range values are handled inside the engine is still unknown; only that no message is printed); behaviour of the app-level `EngineGateway` (does not exist yet); Mermaid (this document has none).
- **Assumptions:** ASM-EN-002 is now checked for the real binary: it answered `uci`, `isready`, `setoption`, `go movetime`, `stop`, exited on end of file (EI-EN-006), and `id name` contains `Stockfish 19` `[RAN]`. ASM-EN-003 (`process_group`, `kill_on_drop`): own process group confirmed with `ps -o pgid` (CT-EN-020); `kill_on_drop` indirectly (CT-EN-054 relies on the driver's explicit kill, not on `kill_on_drop`) `[INF]`. ASM-EN-004 (the `ESTIMATE` numbers): cancel latency measured (6); the other timeouts are unchanged placeholders.
- **Needs a human:** the desk review of GATE-106 by the System Architect (after a Reviewer pass); Brian's decision on `PINNED_UCI_ELO_RANGE` (chess-a0r.3.4); decisions on FIND-E2 (admit `test-util`?), FIND-E3 and FIND-E5 (SDD changes), FIND-E4 and FIND-E10 (app owner and SA).

## 10. Rework after REV-chess-engine-code (chess-a0r.3.10, dispatch 2, 2026-11-10) `[RAN]`

Engineer stance. Input: review-chess-engine-code.md (4 Major, 8 Minor, 4 Info). Each Major was reproduced by a failing regression test on the reviewed code before the fix. No SDD was edited. Working tree only, nothing committed. Review dispositions are the reviewer's and the SA's to record; the status below is the Engineer's claim.

### 10.1 Per-finding status

| Finding | Status | Regression test (file `tests/rework.rs` unless noted) | Failed on old code `[RAN]` |
|---|---|---|---|
| EF-01 timers starved | FIXED. Poll order is now dropped future, hard, soft, commands, output. `LineSplitter` keeps a read index and a scan index (no per-line memmove, no rescans) | `ef_01_a_flood_of_info_lines_is_killed_at_the_hard_limit`, `..._soft_stopped_at_movetime_plus_grace`, `ef_01_a_flood_of_bare_newlines_is_killed_at_the_hard_limit`, `ef_01_line_splitter_work_per_chunk_is_linear_in_bare_newlines` | yes: the three flood tests timed out (engine on its own runtime thread); the splitter test took 7.74 s against a 1 s limit |
| EF-02 `Duration` overflow | FIXED. `SearchLimits::MAX_MOVETIME` = 1 h (**ruled value**: Brian Prascher 2026-11-10, chess-a0r.3.42, no longer an estimate; replaced the 24 h placeholder), `EngineConfig::MAX_DURATION` = 1 h (ESTIMATE); `InvalidRequest` / `InvalidConfig`; the driver also uses `checked_add` | `ef_02_validate_rejects_...`, `ef_02_a_huge_movetime_is_refused_and_the_instance_survives`, `ef_02_a_huge_grace_in_the_config_is_refused`, `ef_02_the_movetime_cap_boundary` (exactly 1 h accepted, 1 h + 1 ms rejected with `InvalidRequest`, `Duration::MAX` rejected, no panic) | yes: validate returned `Ok`; the driver task panicked (`Closed`) |
| EF-03 real tests skip in CI | FIXED in `ci/check.sh` and `ci.yml` (see 10.3). Hosted run not done | commands in 10.3 | n/a (process) |
| EF-04 arbiter lost wakeup | FIXED. Each lane has a stop signal; the search's own task sends `engine.stop()` after its `Search` is queued. Clobber: `RunningGuard` now clears with compare-and-swap and drops before the lock | `ef_04_interactive_always_preempts_a_background_search_2000_races` (2000 races, two caller threads plus a driver thread, loss judged from the engine log: bg `go`, then interactive `go`, no `stop` between) | yes: 12 lost of 2000 |
| EF-04 suspected `RunningGuard` clobber | RESOLVED, not proven. Reading shows the window existed (guard stored `NONE` unconditionally, after the lock was dropped). I could not reproduce it (nanosecond window). Fixed by construction; no test fails on the old code | none | not reproduced `[UNV]` |
| EF-05 inverted `UCI_Elo` range | FIXED. `uci_elo_range()` returns `None` when min > max, so the plan returns `InvalidPersonality("UCI_Elo")` | `ef_05_an_inverted_uci_elo_range_is_a_typed_error_and_the_instance_survives`, `ef_05_a_single_value_uci_elo_range_is_usable` | yes: panic at plan.rs, `Closed` |
| EF-06 shutdown while starting | FIXED. A shared `Closing` flag plus `Notify` interrupts start-up, prepare, writes and drain; commands queued behind it get `Closed`; pool shutdown runs in parallel (`JoinSet`) | `ef_06_shutdown_during_a_hung_start_is_bounded_by_the_grace`, `ef_06_pool_shutdown_of_several_stuck_instances_does_not_add_up` | yes: 3.90 s and 3.85 s against 1.5 s |
| EF-07 write deadline and position cap | FIXED. Every write is bounded by `handshake_timeout` and returns `SearchTimeout` after killing the child; `SearchRequest::MAX_MOVES` = 10 000 (ESTIMATE) | `ef_07_a_write_to_an_engine_that_does_not_read_ends_in_an_error`, `ef_07_shutdown_during_a_blocked_write_is_prompt`, `ef_07_a_position_with_too_many_moves_is_refused`, `ef_07_the_move_count_boundary` | yes |
| EF-08 pv/score/depth vs best | FIXED. Completed line if it names `best`; else the latest bound line of line 1 if it does; else `[best]`, `Cp(0)`, depth 0 (the existing "no info" convention). `lines` is unchanged: it still holds the last completed depth | `ef_08_pv_score_and_depth_describe_the_returned_best_move`, `ef_08_when_no_line_names_the_best_move_the_result_says_so` | yes |
| EF-09 fragile margins | FIXED by margins. Real-process tests use `common::process_cfg()` (5 s handshake); `ct_en_054` and `ct_en_055` take the last pid; five upper-bound timing assertions in `driver_scripted`, `pool`, `process` were widened (comments mark them). Paused time is still not available (`test-util` not admitted) | existing tests | not reproduced |
| EF-10 unasserted behaviours | FIXED, tests added: `ef_10_a_drain_that_fails_kills_the_child_and_the_next_search_restarts`, `..._exited_between_searches_is_restarted_transparently`, `..._caller_stop_of_a_background_search_is_not_a_preemption`, `..._background_waits_for_every_queued_interactive_request`, `ef_10_error_multipv_of_exactly_two_...`, `ef_10_without_an_error_roll_...` and (tests/process.rs) `ef_10_the_childs_stderr_is_the_null_device` | see 10.2 |
| EF-11 error-pick pin | FIXED with engine-side vectors: `tests/data/error_pick_vectors.txt` (32 rows, 8 seeds x 4 counts, oracle: independent Python SplitMix64). The shared file belongs to chess-domain and is untouched | `ef_11_next_f64_is_the_top_53_bits_of_the_first_draw`, `ef_11_select_move_replays_the_committed_rolls_and_picks` | M41, M42 now killed (10.2) |
| EF-12 FakeEngine | FIXED in `src/fake.rs`: health back to `Ready` after a dropped search; `stop()` ends a latency wait; multipv cap = `EngineConfig::default().multipv_cap`; latency capped by movetime; `elapsed` deterministic. Tests `ef_12_*` (4, in `src/fake.rs`). Doc comment lists the remaining differences | | |
| EF-13 (Info) | NO CODE CHANGE. SDD amendment is the architect's | | |
| EF-14 `Debug` | FIXED: hand-written `Debug` for `EngineError` shows only the length of engine text | `ef_14_debug_of_an_error_never_prints_engine_text` | the derive printed the text `[INF]`; not run on old code |
| EF-15, EF-16 (Info) | DEFERRED, no code. EF-15 (option flood, unbounded option map) is outside the v1 threat model (FIND-016); EF-16 items are by design or SDD-accepted | | |
| G0 pinned.rs comment | DONE: states the range is final (Brian "Ok" 2026-11-09, D-112); single definition kept; the number is not repeated anywhere new | | |
| E13 transients | Cause addressed for `process.rs` (first-exec cost versus the 300 ms handshake). EI-EN-005 transient: still unexplained, see 10.4 | | |

Nothing in the minor list was deferred except the two Info items. Not done: the Reviewer's suggested re-run of CT-EN-013 is covered by the existing test plus the new flood tests.

### 10.2 Reviewer's surviving mutants, re-run on the new tests `[RAN]` (hand mutation, scratch script, each mutant applied to the working tree and reverted)

| Mutant | Result |
|---|---|
| M05 `error_multipv >= 2` to `> 2` | KILLED (`ef_10_error_multipv_of_exactly_two_...`) |
| M12 drain failure no longer kills | KILLED (`ef_10_a_drain_that_fails_...`) |
| M22 stderr inherit | KILLED (`ef_10_the_childs_stderr_...`; the probe cannot see the mutant if the test binary itself runs with stderr on /dev/null) |
| M31 `try_wait` check removed | KILLED (`..._restarted_transparently`) |
| M41 `next_f64` low bits | KILLED (`ef_11_*`) |
| M42 mirrored pick | KILLED (`ef_11_select_move_replays_...`) |
| M46b `Preempted` without the preempt counter | KILLED (`ef_10_a_caller_stop_...`) |
| M54 non-error branch keeps caller's multipv | KILLED (`ef_10_without_an_error_roll_...`) |
| M19 background no longer waits for `waiting == 0` | SURVIVES, judged behaviourally equivalent: the re-check after acquiring the lock (`waiting > 0` then release and retry) gives the same order. The wait only avoids lock churn. `[INF]`, not proven |

### 10.3 Commands and results (arm64, rustc/cargo 1.99.1)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTFLAGS="-D warnings" cargo clippy -p chess-engine --all-targets -- -D warnings`, also `--features test-support` | both `Finished`, no diagnostics |
| `CHESS_REQUIRE_STOCKFISH=1 cargo test -p chess-engine` | 147 tests (was 114). Runs 1 to 4: 147 passed, 0 failed. **Run 5: 1 failed (70 passed before cargo stopped); the failing test name was not captured** (my loop printed counts only). Runs 6 to 9 (`--no-fail-fast`, output kept): 14 of 14 binaries ok, 0 failed. See 10.4 |
| `cargo llvm-cov -p chess-engine --summary-only` (binary present) | TOTAL lines 2182, missed 261 = 88.04 %; without `bin/fake_uci.rs` (0 %): 2030 lines, 109 missed = 94.63 % `[INF from printed columns]`. Per file as printed: driver.rs 90.62 %, arbiter.rs 100 %, lines.rs 100 %, plan.rs 100 %, types.rs 100 %, config.rs 100 %, pool.rs 99.22 %, select.rs 98.70 %, fake.rs 97.65 %, error.rs 61.54 % (the new `Debug` impl; most variants are not printed by any test) |
| `CHESS_STOCKFISH=/nonexistent/sf ci/check.sh` | exit 1 at the first step with the message "the Stockfish binary is missing"; nothing else ran |
| `CHESS_REQUIRE_STOCKFISH=1 CHESS_STOCKFISH=/nonexistent/sf cargo test -p chess-engine --test real_stockfish` | panics at real_stockfish.rs:61 (tests fail instead of skipping) |
| `CHESS_CI_OFFLINE=1 ci/check.sh` (binary present) | exit 0, "All gates passed."; workspace test lines sum to 746 passed, 0 failed (chess-store was mid-edit by another engineer but built and passed) |

EF-03 mechanics: `ci/check.sh` exports `CHESS_REQUIRE_STOCKFISH=1` when the binary (default path or `CHESS_STOCKFISH`) exists, exits 1 with a message when it does not, and with `CHESS_ALLOW_STOCKFISH_SKIP=1` unsets the variable and prints that this is not a full pass of FF-7. `ci.yml` runs `third_party/stockfish/fetch.sh` first (network; the pinned archive SHA-256 is enforced; needs curl, tar, shasum, lipo, jq on the runner). **The hosted workflow and the fetch step on a hosted runner were not run** `[UNV]`.

### 10.4 Open items and new findings

- **FIND-E14 (Minor, OPEN): one unexplained failure in test run 5.** The run printed `passed=70 failed=1`, so the failing test was in a binary after `process`/`real_*` ordering was reached; the name and message were lost. Nine further full runs (including four with kept logs) passed. Likely candidates are the real-timer tests or `real_cancel_latency` (EI-EN-005, the unexplained transient of FIND-E13), but this is a guess `[UNV]`. The machine was also running another engineer's cargo build. Next step: loop the suite with `--no-fail-fast` and keep every log until it recurs.
- **FIND-E15 (Minor):** `ef_04_..._2000_races` takes about 24 s and `rework` is now the longest test binary next to `real_cancel_latency` (about 25 s). Timing-free in its verdict (judged from the engine log), but it adds to the gate time.
- **FIND-E16 (Info):** `error.rs` line coverage is 61.54 % because the new `Debug` is only partly exercised.
- **FIND-E17 (Info):** the EF-08 change makes `Cp(0)`, depth 0 appear for a real search when no line names the best move (about 0.3 % of short real searches by the reviewer's measure). `chess-app` must treat depth 0 as "no evaluation". SDD 3.6.1 should say so (architect's amendment).
- **SDD conflicts for the architect (not edited):** 3.10.2 poll order (EF-01 and EF-13); the write deadline uses `handshake_timeout` (no new config field); shutdown now interrupts non-reading phases (3.10.10); stop in the analysis arbiter is sent by the search task (3.10.9); new limits `MAX_MOVETIME`, `MAX_MOVES`, `MAX_DURATION` (3.3, 3.11 numbers are ESTIMATE).
- Not run: Intel slice, `Threads` above 1, TSan (still the right tool for EF-04; the 2000-race test is a stress, not a proof), miri, fuzzing, cargo-mutants, hosted CI, `cargo deny` online, a Reviewer pass of this rework.
- Assumptions: the concurrency tests use std threads with current-thread runtimes because `rt-multi-thread` is not admitted; timings come from one Apple-silicon machine.

### 10.5 Ruling applied after the first rework report (2026-11-10, relayed, chess-a0r.3.42)

- `SearchLimits::MAX_MOVETIME` is now 1 hour, a ruled value. `cargo clippy ... -D warnings` clean and `cargo test -p chess-engine --test rework ef_02` 4 passed `[RAN]` after the change. The 24 h placeholder no longer appears in `src/` or `tests/` (grep).
- GATE-106 section "Rework record" item 1 and this file section 10.1 were updated to say so. The SDD note is the architect's.
- The EF-03 CI change is PENDING Brian Prascher's decision; its separate diff is in GATE-106 (section "EF-03 CI diff").

### 10.6 FIND-E13/E14 loop result `[RAN]` (after the 1 h change)

`CHESS_REQUIRE_STOCKFISH=1 RUSTFLAGS="-D warnings" cargo test -p chess-engine --no-fail-fast`, 20 consecutive runs, logs kept in the session scratchpad (`.../scratchpad/loop/run1.txt` to `run20.txt`, summary `summary.txt`). Result: 20 of 20 runs exit 0, 14 of 14 test binaries ok each, **0 failed tests**. The single unexplained failure of the earlier run 5 (name not captured) did not recur in 29 further full runs (9 + 20). Cause remains unidentified `[UNV]`; FIND-E14 stays open as "not reproduced". The scratchpad logs are outside the repository.
