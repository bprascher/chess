# Step 2, chess-store part D4: resume, rebase, startup recovery (chess-a0r.3.40), verification and evidence

Engineer stance. Gate: **GATE-112** (G3, **OPEN, NOT APPROVED**, PENDING Brian Prascher). Baseline B6, CR chess-a0r.20. Nothing here is approved by anyone. **No Reviewer pass has happened.** This file records the verification of code that another engineer wrote (the part D4 build is described in GATE-112 and in the file headers); this engineer ran it, attacked it with mutants, added tests for the gaps the mutants found, and measured it. All results below are `[RAN]` on this host (macOS arm64, rustc/cargo 1.99.1) on 2026-11-24 unless tagged otherwise. Nothing was committed or pushed; no bead was closed.

## 1. What was built (summary; the full list is in GATE-112)

New: `src/journal/recover.rs` (`open_for_resume`, `StartupRecovery`, `recover_game`), `src/repo/resume.rs`, `src/store/d4.rs`. Edited: `journal/files.rs` (`settle_rebase_files` completeness check, `finish_close_final`), `journal/writer.rs` (`lock_with_grace`), `fault.rs`, `open.rs`, `store.rs`, `api.rs`, `repo/apply.rs`.

### 1.1 Public additions and changes (G2 surface, Q2)

| Item | Kind |
|---|---|
| `MoveJournal::open_for_resume(&Store, &GameId) -> Result<(Option<JournalWriter>, Recovered), ResumeError>` | new |
| `journal::{Recovered, RecoveryReport, ResumeError, StartupReport, RecoveredGame, RecoveryOutcome, CloseRepair}` | new types |
| `OpenReport.recovery: StartupReport` | new field of an existing public struct |
| failpoint `journal.close_final.linked` (and `journal.rebase.db_committed` wired) | new / wired |
| `journal::settle_rebase_files(journal_dir, game, store)` | **changed** (third argument); `Settled` gained `MovedAsideIncomplete { damaged }` and lost `Copy` |
| `Store::open` runs the startup recovery; `JournalWriter::close_final` is crash-idempotent; `MoveJournal::create` no longer removes a file whose lock another holder keeps (B-R04) | behavior |

### 1.2 SDD conflicts found by the author (SDDs not edited)

1. SDD 3.7.3 `open_for_resume -> Result<(JournalWriter, Recovered), JournalError>` cannot carry a database error nor a final game with no writer. Built as `Result<(Option<JournalWriter>, Recovered), ResumeError>`.
2. `Recovered.moves` and `.clocks` use `StoredMove` and `ClockPair`; the SDD's `MoveRecord` and `Clocks` do not exist.
3. SDD 3.9.2 and 3.11.3 RR-A-1 (a) both say who writes the `Start` of a journal with no record. Built: the rebase writes it (empty intact run); the no-journal path creates a header-only file and the app appends `Start`.
4. Startup recovery runs inside `Store::open` (as SEQ-ST-01 draws it), removes a header-only journal whose game has no row (STATE-ST-01), and the plan applier waits for the database lock for the connection's normal 5 s, not the 100 ms of `apply_records`.

## 2. Step 1: the full suites, repeated `[RAN]`

| Command | Runs | Result |
|---|---|---|
| `cargo test -p chess-store` | 5 | 5 of 5 exit 0; 487 passed, 0 failed, 0 ignored in each run (summed over all test binaries) |
| `cargo test -p chess-store --features test-support` | 5 | 5 of 5 exit 0; 575 passed, 0 failed in each run |
| `cargo test -p chess-store --features test-support --test journal_race` | 10 | 10 of 10 exit 0; 3 tests passed each (the race binary), 16 s each |

No failure, no flake. These counts are from before the tests of section 3 were added. After them, `ci/check.sh` (section 7) printed: workspace `cargo test` 1 127 passed, 0 failed; chess-store with `test-support` 593 passed, 0 failed (575 plus the 18 tests added in section 3).

## 3. Step 2: hand mutants and the tests they led to

**Method.** 62 prepared hand mutants (one source edit each, in `recover.rs` 44, `files.rs` 14, `writer.rs` 4; script `mutants.py`, adapted into a scratchpad runner) over `recover.rs`, `resume.rs` paths, `d4.rs` callers, `close_final`/`finish_close_final`, `settle_rebase_files`/`rebase_is_complete`, and the create lock grace. Each was applied to a scratch **copy** of the workspace (rsync of the working tree, three copies, three workers), the D4 test binaries were run (`resume`, `recovery`, `journal`, `journal_faults`, `d4_faults`, `recovery_kill` with `CHESS_STORE_KILL_ROUNDS=3`, feature `test-support`), and the file was restored. A mutant is **killed** when any test fails. Not a tool such as `cargo-mutants`: hand-picked edits, so the score is not a mutation score of the crate.

**Round 1 (original tests):** 36 killed, 18 survived, 6 did not compile (the edit left an unused item under `-D warnings`), 2 not applied (edit text did not match the source).

**Round 2:** the 8 invalid mutants were re-written (and built with `RUSTFLAGS=--cap-lints=warn`), the 16 tests of the list below were added, and the 26 open mutants were re-run with `d4_verify`, `d4_verify_faults`, `journal_race` and the lib tests added to the set.

**Final: 57 killed, 5 survived, of 62.**

| Mutant (round 1 index) | Edit | Result | Killed by |
|---|---|---|---|
| 3, 4 | `nothing_stored` uses `\|\|` / drops a half | killed (new) | `a_row_with_a_journal_seq_and_no_moves_and_no_journal_is_rebuilt_not_created_empty`, `a_row_with_moves_and_a_zero_journal_seq_and_no_journal_is_rebuilt` |
| 16 | `short` is `len <= HEADER_LEN` | killed (new) | `a_header_only_journal_with_stored_moves_is_the_store_ahead` |
| 18 | `short && !final_row` without `nothing_stored` | killed (new) | `a_journal_cut_inside_its_header_with_stored_moves_is_rebased_and_keeps_the_moves` |
| 1 | `StoreAhead` set when `db_seq < last_seq` | killed (new) | `a_final_game_whose_journal_is_ahead_of_the_store_is_not_damage` |
| 34 | `StoreAhead` never set from the view | killed (new) | `a_final_game_whose_store_is_ahead_of_its_journal_is_moved_aside` |
| 40 | rebased file's terminal record not applied | killed (new) | `a_rebased_journal_that_ends_in_a_game_end_finalizes_the_game_at_startup` |
| 25 | `Terminal(_)` (busy) reported as `Finalized` | killed (new) | `a_finalize_that_cannot_commit_at_startup_is_queued_and_the_journal_stays` (feature) |
| 26 | `Terminal(Applied)` arm never taken (re-written) | killed | existing tests |
| 33 | `lock_open` compares the device, not the inode | killed (new) | lib unit test `lock_open_refuses_a_file_that_was_replaced_while_it_waited` |
| 48 | `ends_a_rebase` accepts a running `Clock` | killed (new) | `a_rebase_is_complete_when_it_ends_in_a_terminal_or_a_stopped_clock_only` |
| 49 | `ends_a_rebase` accepts only `GameEnd` | killed (new) | same test (Abort, Voided) |
| 47 | torn tail not checked by `rebase_is_complete` | killed (new) | `a_rebase_with_no_record_or_with_a_bad_tail_is_incomplete` |
| 35, 38, 51, 61 | re-written compile failures | killed | existing tests |
| 41 | `scan_damaged: false` (re-written) | killed | existing tests |
| 58, 59 | no lock grace in `create` / `LOCK_GRACE_TRIES = 0` | killed | `journal_race` (the B-R04 binary), see section 6 |
| 60 | `create` removes the file when the lock stays held | killed (new) | `b_r04_a_create_that_loses_to_a_holder_reports_locked_and_leaves_the_file` |

**The 5 survivors, with the reason `[INF]` for each (none is a defect):**

| # | Edit | Why it survives |
|---|---|---|
| 24 | `order_key`: a game that finalizes nothing keyed `(1, 0, id)` instead of `(0, 0, id)` | `ended_at` is greater than 0 for every `GameEnd` (D-144, refused by the applier and the decoder otherwise), so such a game still sorts before all real ends. Equivalent while that rule holds |
| 28 | `classify_error`: the `GameParked` branch removed | Not reached from the scan: the parked set is not persisted, so a start passes the whole journal again, the store refuses it again, and the scan finds the cause through `parked_games()` (pinned by `a_game_parked_at_one_start_is_parked_again_at_the_next_with_its_cause`). The branch is defensive code. See FIND-D4V-04 |
| 44 | `rebase_is_complete`: `len <= HEADER_LEN` becomes `<` | A file that is only a header has no last record, and the last check fails it anyway. Redundant guard |
| 45 | `rebase_is_complete`: `intact_end != len` dropped | Implied by the `damaged` and `torn_tail_dropped` checks. Redundant guard |
| 46 | `rebase_is_complete`: `damaged.is_some()` dropped | Implied by the `intact_end != len` check (tested with an undecodable CRC-valid tail, which is refused by the others). Redundant guard |

**Tests added (all in `crates/chess-store`; 18 in total):** `tests/d4_verify.rs` (14 tests), `tests/d4_verify_faults.rs` (1, feature `test-support`), lib unit tests in `src/journal/recover.rs` (2), `tests/journal_race.rs` (1). Beyond the mutant list, coverage led to three more in `d4_verify.rs`: a rebase from a row with a computer seat and a clock (`start_from_row`), an incomplete `.rebase` at startup that is rebuilt (B-R05), and an incomplete `.rebase` for a final or an empty row. One test pins an observed behavior that the author thinks is open (FIND-D4V-03).

**Code fixes:** none. No mutant or test exposed a defect in the D4 code; every gap was a missing test.

## 4. Step 3: kill rounds `[RAN]`

`CHESS_STORE_KILL_ROUNDS=50 cargo test -p chess-store --features test-support --test recovery_kill`: 7 passed, 0 failed, 120.20 s. Real child processes killed with SIGKILL at a failpoint, then `Store::open` and a resume.

| Test | Rounds run | Failpoints |
|---|---|---|
| CT-ST-003 (every `journal.*` failpoint) | 300 (6 x 50) | `before_write`, `partial_write`, `after_write`, `before_fsync`, `fsync_error`, `after_fsync` |
| CT-ST-088 (six rebase kill points through the real recovery) | 300 (6 x 50) | `built`, `db_committed`, `old_linked`, `old_aside`, `new_linked`, `new_in_place` |
| CT-ST-009 (kill between journal `GameEnd` and finalize commit; and after it) | 50 per test | `apply.*` / `journal.*` as the test names |
| B-R03 (kill between the link and the removal of `close_final`) | 50 | `journal.close_final.linked` |
| CT-ST-003 move to `GameEnd`, CT-ST-088 resume path | per the test (at least 2 rounds) | |

So about 300 rounds for each of the two failpoint families and 650 or more in all. The default is 3 rounds per failpoint (the mutant runs used 3). A kill proves a **process** crash only; a power loss is not simulated.

## 5. Step 4: line coverage `[RAN]`

`cargo llvm-cov -p chess-store --features test-support` (cargo-llvm-cov 0.9.1). The children killed in the kill tests leave corrupt `.profraw` files (2 to 4 per run), which were removed before the merge; their lines are therefore **under-counted**. The two exports disagree and the cause was not found, so both are given as printed.

| File | `llvm-cov report` lines (missed, cover) | lcov `DA` lines (hit of total) |
|---|---|---|
| `src/journal/recover.rs` (includes its `#[cfg(test)]` module) | 778, 88 missed, 88.69 % | 701 of 729, 96.16 % |
| `src/repo/resume.rs` | 38, 2 missed, 94.74 % | 35 of 37, 94.59 % |
| `src/store/d4.rs` | 23, 0 missed, 100.00 % | 19 of 19, 100.00 % |
| `src/journal/files.rs` | 130, 8 missed, 93.85 % | 121 of 127, 95.28 % |
| `src/journal/writer.rs` | 193, 13 missed, 93.26 % | 170 of 183, 92.90 % |

Lines of `recover.rs` not executed (lcov, current numbering): the `io_err` arms for `open`, `try_lock` and `fs::read` (259, 264), the `_ => false` of a failed `metadata` (272), `StoreError::Corrupt` for a row with no `started_at` or an unparsable clock (404), `Holder::None` in `path()` (195), join errors (501, 595, 631, 724), `scan_failed` (1028 to 1031), `Failed` outcomes of a failed close (1090 to 1092, 1101 to 1103), and the unreachable `GameParked` branch (1115). `files.rs`: I/O error arms (97 to 100, 195, 199, 250). No coverage target is set for this crate in the quality gates for these files; no MC/DC was measured.

## 6. Step 5: B-R04, the `create` / `probe` race `[RAN]`

Fix under test: `lock_with_grace` retries `try_lock` for up to 40 x 2 ms = 80 ms on a file this call just created; if the holder stays, `create` returns `Locked` and **does not remove the file**.

| Check | Rounds | Spurious `Locked` |
|---|---|---|
| In-thread: each `create` races a prober thread that locks and unlocks the name in a loop (`b_r04_create_is_never_refused_by_a_concurrent_probe`) | 3 000 per run x 10 runs = **30 000** | **0** |
| Cross-process: a child process probes the names while the parent creates (`b_r04_create_is_never_refused_by_a_probing_process`) | 400 per run x 10 runs = **4 000** | **0** |
| A holder that stays (new): a thread takes the lock the instant the name appears and keeps it; `create` must say `Locked` and the file must still exist | rounds until 5 losses (5 runs: each passed in about 0.6 s) | n/a: every `Locked` was a real holder, no file was removed |

All 10 runs of the binary passed. Both race tests assert zero spurious `Locked`, so a run with a spurious one fails. Mutants 58, 59 (no grace) and 60 (file removed on `Locked`) are killed. The same binary also ran inside each of the 5 + 5 full-suite runs, the coverage runs and `ci/check.sh`, all green.

## 7. Step 6: static checks and CI `[RAN]`

| Command | Result |
|---|---|
| `cargo clippy -p chess-store --all-targets -- -D warnings` | Finished, no diagnostics |
| `cargo clippy -p chess-store --all-targets --features test-support -- -D warnings` | Finished, no diagnostics |
| `cargo fmt --check` | exit 0 |
| `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| `CHESS_CI_OFFLINE=1 ci/check.sh` (foreground, last) | exit 0, "All gates passed." Steps: fmt, build, clippy, `cargo test --workspace` (1 127 passed, 0 failed), chess-store with `test-support` (593 passed, 0 failed), crate edges, ECO checksums, `cargo deny` (one warning `license-exception-not-encountered`, `deny.toml:51`; not a failure), `npm audit` skipped (offline) |

## 8. Dispositions of the three PM rulings (as verified, none approved)

| Ruling | Disposition | Evidence |
|---|---|---|
| **B-R03** `close_final` crash repair with `same_file`, no `AlreadyExists` loop | **Implemented and verified; open for Brian's gate.** A journal under two names is finished by the next start; a different file in `done/` is not touched and is reported | `recovery.rs` three `b_r03_*` tests, `d4_faults` `b_r03_*`, `recovery_kill` B-R03 at 50 rounds, mutants on `same_file` in `prepare` and `finish_close_final` killed, and the dev/inode comparison in `lock_open` now killed by the new unit test |
| **B-R05** completeness check before a `.rebase` is installed | **Implemented and verified; open for Brian's gate.** Complete: renamed into place (no-replace). Incomplete: kept as `damaged/<uuid>.<k>.rebase`, never installed, the game rebuilt from the store | resume path and startup path tests (4), `rebase_is_complete` cases (abort, voided, game end, stopped clock complete; running clock, header only, garbage tail, undecodable tail incomplete); mutants 47 to 51 killed, 44 to 46 argued equivalent (redundant guards) |
| **B-R04** create/probe race | **Fixed and verified.** 0 spurious `Locked` in 30 000 in-thread and 4 000 cross-process creates; the stay-holder case leaves the file | section 6 |
| D3-10 whole journal in one batch | Verified by `the_whole_journal_reaches_the_applier_in_one_call` (one `apply.before_commit` per journal) | `d4_faults.rs` |

## 9. Findings (Engineer's own; all OPEN; no Major)

| ID | Severity | Finding |
|---|---|---|
| FIND-D4V-01 | Minor | The first test set left 16 mutants alive, all at the edges of the case table in `recover.rs` (what "nothing stored" means, a cut or header-only journal against a row with moves, a final row against a journal shorter or longer than the store, a rebased file that ends in a terminal record, a busy database at startup, the `.rebase` completeness cases). Closed by the 18 tests of section 3. No code defect behind any of them |
| FIND-D4V-02 | Minor | Three redundant guards in `rebase_is_complete` (mutants 44, 45, 46) and an `order_key` fallback that depends on the `ended_at > 0` rule (24). Harmless; the guards are defense in depth. A cleanup is optional and would be G1 |
| FIND-D4V-03 | Minor, **question for the Systems Engineer** | Observed: the store holds the `GameEnd` (row `completed`), the journal holds a further `Voided` the store does not; the scan moves the journal to `done/` (`ClosedLeftover`) and does not apply the `Voided`. Pinned by `observed_a_voided_after_an_applied_game_end_is_closed_without_being_applied`. D-87 covers a `GameEnd` then a `Voided` before the finalize commit; this state needs a void after the commit and before the close, which the author believes cannot arise in normal play `[INF]`. Intent not decided |
| FIND-D4V-04 | Minor | The `GameParked` branch of the scan's error classification is not reachable: a parked game is parked again by the next start with its cause (observed). Either the parked set should persist (an architecture question, D-145) or the branch is dead. Pinned by a test; no change made |
| FIND-D4V-05 | Info | A state with `MOVE` rows, `journal_seq` 0 and no journal cannot arise through the API; the code lets the journal win and drops the rows (observed by the test in section 3). Not asserted |
| FIND-D4V-06 | Info | The kill tests leave corrupt `.profraw` files under `cargo llvm-cov`; coverage of the killed children is lost, and the `llvm-cov report` and lcov line figures disagree (section 5) |

## 10. Not run, not analyzed, assumed `[UNV]`

- **Reviewer pass:** not done. No independent review. Gate not approved.
- **Mutation tool:** only 62 hand mutants; `cargo-mutants` was not run on the crate; no mutation score is claimed. Mutant rounds used 3 kill rounds per failpoint, not 50.
- **MC/DC and branch coverage:** not measured (the toolchain's `llvm-cov` here reports no branch data). Line figures are those of section 5, with the two exports disagreeing.
- **Power loss and filesystem behavior:** kill tests are SIGKILL of a process on APFS on this host; no power cut, no other filesystem, no network volume. `fsync` durability is not measured.
- **Load:** B-R04 was measured on an otherwise idle machine; behavior of the 80 ms grace under heavy CPU contention was not measured. The new stay-holder test is statistical (it needs a few lost races); it passed in every run but it is not deterministic.
- **Intel Macs:** not tested (SWE-001, environment limits).
- **Benchmarks:** `startup_bench` ran as part of the suites; no separate BM-ST-02 number is quoted here.
- **Dependencies, SBOM, vulnerability scan:** no dependency was added; no new scan beyond `cargo deny` in `ci/check.sh`.
- **SDDs and requirements:** not edited; the conflicts of section 1.2 need the SDD owner and Brian's decision.
- **Assumption ASM-D4V-1:** the mutants that were killed by a test failure were killed for the reason shown; failures were not individually inspected beyond the test name.

## 11. Needs a human

Brian Prascher: GATE-112 (G3, public surface G2), the SDD conflicts of section 1.2, FIND-D4V-03 and FIND-D4V-04 (intent). A Reviewer pass by a fresh invocation is still to be scheduled.
