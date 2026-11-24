# Step 2, chess-store part D1: FaultPlan and crash-test harness (evidence)

Task chess-a0r.3.37, CR chess-a0r.20, baseline B6, 2026-11-10. Engineer stance. Gate: [GATE-107](gates/GATE-107-chess-store-D1.md), G1, **OPEN, NOT APPROVED**. Nothing here is reviewed or approved. Working tree only, nothing committed. Evidence tags: `[OBS]` read, `[RAN]` executed, `[INF]`, `[UNV]`.

## 1. What was built `[OBS]`

| Item | Where |
|---|---|
| Feature `test-support = []` (no new dependency) | `crates/chess-store/Cargo.toml` |
| `fault` module (DD-ST-024): registry, `FaultPlan` builder, `install` -> `FaultGuard`, `trigger`, actions `Errno`, `Kind`, `Delay`, `Kill`, `skip`, `once`, text form `FaultPlan::parse`, counters `hits`/`fired`. Feature off: private module with only `check` and two sync helpers | `src/fault.rs` (424 lines), `src/lib.rs` |
| Failpoint calls in part A code | `src/backup.rs` (`backup.before_rename`, `backup.verify`, `sync.backup_copy_file`, `sync.backup_copy_dir`, `sync.backup_rename_dir`), `src/datadir.rs` (`sync.store_id_file`, `sync.store_id_dir`), `src/open.rs` (`sync.probe`, `open.after_db_created`, `open.after_store_id`), `src/migrate.rs` (`migrate.after_backup`) |
| Kill-at-failpoint harness and crash tests | `tests/crash.rs` (319 lines) |
| Registry, CT-ST-085, survivor tests | `tests/fault_injection.rs` (597 lines) |

Design points:
- **Scoping.** A plan is installed for a directory; a failpoint call passes a path and only a plan whose root is a prefix of that path applies (longest root wins). Tests in one process with their own temp directories therefore do not disturb each other. The registry is a process-wide `Mutex` (store thread, `spawn_blocking` threads and test threads all see it).
- **Kill.** The child prints `FAILPOINT-KILL <name>` to stderr and waits (120 s, then `abort`, so no orphan). The parent reads the line, calls `std::process::Child::kill` (SIGKILL) and reports the name. The child is the test binary itself (`current_exe`, `--exact crash_child_entry`). No `unsafe`, no `libc`, no `rustix`.
- **Typo guard.** A name outside `fault::all()` is refused at `install` and by `trigger`.
- **Sync seams.** Each named full-sync call is `fault::sync_file(name, ctx, &file)` or `fault::sync_dir_named(name, dir)`: the failpoint is checked, then the real `sync_all` runs. Feature off: `check` is `#[inline(always)]` and returns `Ok(())`, so the helpers reduce to the plain call.

## 2. Acceptance criteria

| # | Status | Evidence |
|---|---|---|
| (1) Feature off: `FaultPlan` absent, call sites compile to nothing, UT-ST-038 green | MET `[RAN]` | compile-fail doctest `lib.rs (line 13)` (`use chess_store::fault::FaultPlan;` must fail with E0432) passes only in the feature-off build (`cargo test -p chess-store --doc`: 1 passed). `cargo doc -p chess-store --no-deps`: no `target/doc/chess_store/fault`; with `--features test-support` it exists. `tests/boundary.rs` (UT-ST-038, 6 tests) green in both settings. "Compile to nothing" is by construction (`#[inline(always)]`, constant `Ok(())`) `[INF]`; I did not inspect assembly `[UNV]` |
| (2) Every SDD section 4 name plus the three new names reachable by a test | MET with a stated reading `[RAN]` | `every_sdd_failpoint_and_the_three_new_names_is_registered_and_reachable` (13 names copied literally from SDD 969 and the 3 of Addendum K: each armed with an errno, reached through `fault::trigger`, `hits == fired == 1`); `every_registered_name_accepts_every_action_kind`. **Reading:** of the 16 only `backup.before_rename` has a call site in part A code. The other 15 belong to code of parts B, C, D2 to D4 (journal, apply, finalize, reset, restore, rebase, automatic backup) that does not exist yet; they are registered and reachable through `trigger` and each later part wires its own call. `backup.after_copy` is the automatic backup of part C and I did not reuse it for the pre-upgrade copy (FIND-D1-04). `the_wired_failpoints_are_reached_by_the_real_open_and_upgrade` proves the 11 call sites that exist are reached by the real `Store::open` |
| (3) Kill test: child killed at a failpoint leaves a directory the parent can open; harness reports the failpoint | MET `[RAN]` | `tests/crash.rs`: `the_harness_reports_the_failpoint_the_child_died_at`, `..._never_reached_its_failpoint`, `skip_lets_the_first_hits_pass_in_the_child`, `a_child_that_fails_for_another_reason_...`; directories opened by the parent after a kill at `backup.before_rename`, `migrate.after_backup`, `open.after_db_created`, `open.after_store_id`, `sync.probe`, `sync.store_id_file`, `sync.store_id_dir` |
| (4) CT-ST-085 through the real `Store::open` | MET `[RAN]` | `ct_st_085_a_failed_probe_sync_refuses_a_new_directory_and_writes_nothing` (errno 45 ENOTSUP, 25 ENOTTY, 5, 19, 1234: `DurabilityUnsupported{path == directory, errno}`, the directory listing is empty: no `chess.db`, WAL, `store.id`, `journal/`, scratch file; no database means no heartbeat row, and part B's heartbeat table does not exist yet `[INF]`), `ct_st_085_a_failed_probe_sync_leaves_an_existing_store_byte_identical` (`chess.db` and `store.id` bytes and the file listing unchanged), `ct_st_085_the_probe_is_the_only_sync_that_refuses_with_a_path_and_errno` (another sync failure is `Unavailable`). The "failure to create or remove the scratch file stays `Unavailable`" half is part A's unit test `scratch_file_errors_stay_unavailable` (no failpoint exists for create or remove) |
| (4b) CT-ST-090 kill-before-rename | MET `[RAN]` | `ct_st_090_a_kill_before_the_rename_keeps_the_old_copy_and_the_retry_succeeds`: child killed at `backup.before_rename` (reported by the harness); previous `chess.db.bak-v1` byte-identical; stale `chess.db.tmp-bak-v1` present, a verified v1 copy, not counted as a backup; `chess.db` still at `user_version` 1; the parent's open upgrades, removes the temp file and replaces the old copy. Error half: `ct_st_090_an_error_before_the_rename_keeps_the_old_copy_and_removes_the_temp` |
| (5) Coverage per strategy; fmt, clippy, ci/check.sh | MET `[RAN]` | section 4 and 5 |

## 3. Tests added (28 integration tests in two files; no existing test changed or weakened)

`tests/fault_injection.rs` (19): registry and names (2 above), `an_unknown_failpoint_is_refused_at_install_and_at_trigger`, `a_plan_applies_only_under_its_directory_and_only_while_the_guard_lives`, `a_nested_directory_plan_overrides_the_outer_one`, `skip_and_once_count_hits`, `a_rule_without_once_fires_on_every_hit`, `delay_sleeps_and_continues`, `the_text_form_parses_every_action_and_rejects_bad_input`, `the_wired_failpoints_are_reached_by_the_real_open_and_upgrade`, three CT-ST-085 tests, `m16_...`, `m17_...` (two sync points), `m22_...`, `m30_...`, `m31_...`, `ct_st_090_an_error_before_the_rename_...`.
`tests/crash.rs` (10 including the child entry): the four harness tests, `ct_st_090_a_kill_before_the_rename_...`, `a_kill_after_the_new_backup_and_before_the_upgrade_...`, `a_kill_after_chess_db_is_created_...`, `a_kill_after_store_id_...`, `a_kill_at_every_sync_of_the_first_launch_...`.
Both files are empty without the feature.

## 4. Commands run and results `[RAN]`

| Command | Result |
|---|---|
| `cargo fmt --check` (workspace) | exit 0 |
| `cargo clippy -p chess-store --all-targets -- -D warnings` | clean |
| `cargo clippy -p chess-store --all-targets --features test-support -- -D warnings` | clean |
| `cargo test -p chess-store` x5 | 137 passed, 0 failed each time (136 as in part A plus the compile-fail doctest) |
| `cargo test -p chess-store --features test-support` x5 | exit 0 each time, 165 passed, 0 failed each time |
| `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| `CHESS_CI_OFFLINE=1 ci/check.sh` (foreground, at the end) | exit 0, "All gates passed" (stages: Stockfish binary, fmt, build, clippy, test, crate edges, ECO checksums, cargo deny, npm audit skipped by the offline flag). `cargo deny` printed one `advisory-not-detected` warning for RUSTSEC-2026-0441 in `deny.toml` (not mine, not edited) |
| `cargo llvm-cov -p chess-store --features test-support --summary-only` | lines TOTAL 2078, missed 85, **95.91 %**; `fault.rs` 174/9, 94.83 %; `backup.rs` 98.33 %; `datadir.rs` 93.01 %; `migrate.rs` 90.36 %; `open.rs` 90.68 % |
| `cargo llvm-cov -p chess-store --summary-only` (feature off) | lines TOTAL 1915, missed 81, **95.77 %**; `fault.rs` 11/0, 100 % |

No branch or MC/DC data (the toolchain reports none). The uncovered lines of `fault.rs` include the `Kill` arm: a child that waits for SIGKILL never writes its profile, so that arm shows as unexecuted although the crash tests run it `[INF]`. Strategy gate (at least 90 % lines) holds for the crate and for each changed file in both settings.

## 5. Test strength: the six survivors and extra mutants `[RAN]`

Method: a script (`scratchpad/mut.py`) edits one expression, runs `cargo test -p chess-store --features test-support --no-fail-fast`, lists the failing tests, restores the file; `diff -r src` against a saved copy printed identical after each run. `cargo-mutants` was not used. The M numbers are the Reviewer's (`review-chess-store-A.md` FIND-013); the Reviewer's mutant text is not in the review, so I rebuilt each from its description there `[INF]`.

| Mutant | Result |
|---|---|
| M16 directory sync after the backup rename removed | KILLED by `m16_a_failed_directory_sync_after_the_backup_rename_stops_the_upgrade`, `the_wired_failpoints_...` |
| M17 file sync of the backup copy removed | KILLED by `m17_...`, `the_wired_failpoints_...` |
| M17b directory sync of the backup copy removed | KILLED by `m17_...`, `the_wired_failpoints_...` |
| M22 `verify_copy` result ignored in the backup path | KILLED by `m22_a_copy_that_fails_verification_...` |
| M30 `store.id` file sync removed | KILLED by `m30_...`, `a_kill_at_every_sync_of_the_first_launch_...`, `a_child_that_fails_for_another_reason_...`, `the_wired_failpoints_...` |
| M31 `store.id` directory sync removed | KILLED by `m31_...`, `ct_st_085_the_probe_is_the_only_sync_...`, `a_kill_at_every_sync_...`, `the_wired_failpoints_...` |
| M35 probe `sync_all` replaced by a no-op | KILLED by the three CT-ST-085 tests, `a_kill_at_every_sync_...`, `the_wired_failpoints_...` |
| Kill-before-rename: the `backup.before_rename` call removed | KILLED by both CT-ST-090 tests, `the_wired_failpoints_...` |

Limit `[INF]`: a mutant that replaces the real `File::sync_all` **inside** `fault::sync_file` or `sync_dir_named` with `Ok(())` is not killable by fault injection or by any test that can run here (it needs power-loss or an OS-level observer). It is one mutant for all sync sites; I did not run it. The first M16 and M17b attempts did not compile (unused variable under `warnings = deny`); I fixed the mutant text and re-ran.

## 6. Findings (Engineer's own, all OPEN, severity is mine)

- **FIND-D1-01 (Minor, pinned not changed).** A process killed inside the probe (`sync.probe`) or inside the `store.id` write (`sync.store_id_file`) leaves `.durability-probe-<pid>-<n>` or `.store.id.tmp-<pid>-<n>` in the data directory, and no later open removes it. The open is unaffected; the files accumulate with each such crash. `a_kill_at_every_sync_of_the_first_launch_...` pins this and fails on any other leftover. Cleaning stale scratch files is a behaviour change for the architect or Brian to decide.
- **FIND-D1-02 (Minor, SDD gap, for the architect).** The SDD section 4 list lacks the 10 names part D1 added for part A code (`sync.*`, `backup.verify`, `migrate.after_backup`, `open.after_db_created`, `open.after_store_id`) and the 6 names CT-ST-087 and CT-ST-088 use (`journal.partial_write`, `journal.fsync_error`, `journal.rebase.built`, `.db_committed`, `.old_aside`, `.new_in_place`). A dated SDD note (G0 text) is requested. I did not edit the SDD.
- **FIND-D1-03 (Minor, CI gap, for the PM and the owner of ci/).** `ci/check.sh` runs `cargo test` without `--features test-support` for `chess-store`, so `tests/crash.rs` and `tests/fault_injection.rs` do not run in CI (they are empty without the feature). A stage `cargo test -p chess-store --features test-support` is needed. I did not edit `ci/` (ordered).
- **FIND-D1-04 (Info).** `backup.after_copy` (SDD list) belongs to the automatic backup of part C. The pre-upgrade copy has its own `backup.before_rename`; I did not reuse `backup.after_copy` there. Part C decides.
- **FIND-D1-05 (Info).** Of the 16 SDD and Addendum K names, 15 have no call site yet (their code is not written). Acceptance (2) is met through `trigger`; each later part must add its call sites and a test that reaches them through real code, as `the_wired_failpoints_...` does for part A.
- **FIND-D1-06 (Info).** The kill harness depends on libtest's `--exact`, `--nocapture` and on stderr carrying the marker; a child that is killed by a slow machine's 60 s timeout panics the parent test. The 120 s self-abort bounds orphans.
- **FIND-D1-07 (Info).** `fault::trigger` takes a process-wide mutex on every call in the feature-on build. Cost is irrelevant to tests; the feature-off build has none.
- **FIND-D1-08 (Info, `[UNV]`).** The SDD asks CT-ST-085 to assert "no heartbeat row". Part B's heartbeat table is not built, so the assertion is "no database file exists at all". Part B must add the row-level assertion when it adds registration.
- **No Major finding, no blocker, no SDD conflict that stops the work.**

## 7. Not run, not analyzed, assumed

Not run: `cargo-mutants`; the in-function `sync_all` mutant (section 5); assembly inspection for "compiles to nothing"; power-loss or APFS behaviour; Intel; MC/DC (not measurable here); kill tests for parts B to D4 (no code). `npm audit` skipped by the offline flag. Assumed: F-A04 to F-A08 are unchanged and not ruled; the Reviewer's M-numbers map as described in section 5.
