# Step 2: chess-store part A, evidence note (chess-a0r.3.6)

Software engineer, Engineer stance, 2026-11-09, baseline B5. Gate record: [gates/GATE-105-chess-store-A.md](gates/GATE-105-chess-store-A.md) (G1 with a G2/G3 carve-out, OPEN, NOT APPROVED). Code: `crates/chess-store`. Tags: `[OBS]` read, `[RAN]` executed, `[INF]` inferred, `[UNV]` unverified. No SEI CERT conformance is claimed (SWE-001 section 6). Nothing here is approved by anyone; no Reviewer pass has happened.

## 1. What was built

| Area | State |
|---|---|
| `DataDir` (DD-ST-004) | resolution (flag, `CHESS_DATA_DIR`, default), `ensure_root`/`ensure_layout`/`ensure`, `0700` dirs, `0600` files, loose permissions reported and never changed, `store.id` read/write, trace test for "store missing", path helpers |
| Schema (DD-ST-007) | `migrations/0001_init.sql`, all 14 tables and 6 indexes of SDD 3.5.2, equal to the SDD block (statement by statement) and to the architecture 7.2 ER block (table, column, order, primary key, type) |
| `Migrator` (DD-ST-006) | numbered `(version, sql)` list, `with()` for tests, guard, verified `VACUUM INTO` copy before `BEGIN EXCLUSIVE`, version re-read, guard hook, `user_version` in the same transaction; backup failure stops the upgrade |
| `Opener` (DD-ST-005, part) | ensure dir, volume probe (D-71 behaviour), missing-store test, read-only version probe and `quick_check`, write connection with the pragmas, migrate, `store.id` on first launch |
| `Store`, `StoreThread`, `ReadPool` (DD-ST-001..003) | one concrete `Clone + Send + Sync` handle; writes as closures on one thread (panic caught, connection reopened once, then `Unavailable`); reads on a 2-connection pool through `spawn_blocking` |
| Players (DD-ST-008) | `register_player`, `insert_computers`, `roster_indexes`, `rename_player`, `archive_player`, `update_preferences`, `get_player`, `list_players`, `find_player_by_name`, `get_preferences`, `get_app_setting`, `set_app_setting` |
| Ratings (DD-ST-010, read) | `rating_history`; initial rows written by register and roster insert |
| Games (DD-ST-009, part) | `start_game` (slots, idempotent, `started_at`), `get_game`, `list_games`, `list_unfinished`, `game_moves`, `set_analysis_used` |
| `StoreError` (DD-ST-023, part) | category-only `Display`; variants part A raises |

Not built (deferred, see F-A01): `apply_records`, `finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player`, `check_rating_invariant`, stats row queries, evals, variations, jobs, coordination, `lease_*`, `sync_backlog`, `RowIo`, `BackupManager` (automatic, pruning), `RestoreService`, `FaultPlan`, the journal, `CreateFreshStore`.

## 2. SDD entity to code map

| DD | Code |
|---|---|
| DD-ST-001 `Store` | `src/store.rs` (`Store`, `StoreOptions`, `OpenedStore`, `WallClock`) |
| DD-ST-002 `StoreThread` | `src/thread.rs` |
| DD-ST-003 `ReadPool` | `src/readpool.rs` |
| DD-ST-004 `DataDir` | `src/datadir.rs` |
| DD-ST-005 `Opener` | `src/open.rs` (`open_blocking`, `probe_durability`, `classify_probe_error`, `open_write_connection`, `open_read_connection`) |
| DD-ST-006 `Migrator` | `src/migrate.rs`, `src/backup.rs` (`write_verified_copy`) |
| DD-ST-007 `Schema` | `migrations/0001_init.sql` |
| DD-ST-008 `PlayerRepo` | `src/repo/player.rs` |
| DD-ST-009 `GameRepo` (part) | `src/repo/game.rs` |
| DD-ST-010 `RatingRepo` (read) | `src/repo/rating.rs` |
| DD-ST-023 `StoreError` (part) | `src/error.rs` |
| DTOs of 3.7.2 (part) | `src/api.rs` |
| not part A | DD-ST-011..022, 024 |

## 3. Tests (113: 19 unit in `src/`, 94 integration in `tests/`)

All integration tests use a real database file in a temporary directory (`tempfile`), through the public `Store` API; the second raw connection is used only to seed rows and to inspect.

| SDD test ID | Test (file::name) | Result |
|---|---|---|
| UT-ST-030 | schema.rs `ut_st_030_*` (result vs status; Training untimed h-c) | pass |
| UT-ST-031 | schema.rs `ut_st_031_*` (parameter set unique with NULL `uci_elo`; seed+index pair) | pass |
| UT-ST-032 | schema.rs `ut_st_032_*` (case-sensitive, non-empty names) | pass |
| UT-ST-033 | schema.rs `ut_st_033_*` (slot key) | pass |
| UT-ST-034 | schema.rs `ut_st_034_*` (uuid lower case, 36) | pass |
| UT-ST-035 | schema.rs `ut_st_035_*` (hash NULL exactly while in progress, all terminal statuses) | pass; set-by-finalize half is F-A01 |
| UT-ST-036 | schema.rs `ut_st_036_*` (0..=1000, one permille apart distinct) | pass |
| UT-ST-084 | schema.rs `ut_st_084_*` (custom start, clock pairs, multipv, NULL player id, self-play) | pass |
| UT-ST-038 | boundary.rs `ut_st_038_*` (no integer identity in public types, repos private, scanner self-test) | pass |
| UT-ST-039 | players.rs `ut_st_039_*` (every index, empty, no class column, taken index, concurrent insert one wins) | pass |
| UT-ST-040..042 | players.rs `ut_st_040_*`, `ut_st_041_*`, `ut_st_042_*` (name taken, register rollback, roster batch all-or-none) | pass |
| UT-ST-043 (slots) | games.rs `a_half_claimed_slot_set_rolls_back_ut_st_043` | pass |
| UT-ST-060 (part) | games.rs `list_games_applies_structural_keys_only_ut_st_060_part` | pass; stats row queries are F-A01 |
| UT-ST-080 | boundary.rs `ut_st_080_*`, `durability_unsupported_*` | pass |
| UT-ST-092 | games.rs `ut_st_092_*` (started_at unchanged, second start returns the existing row) | pass; "no `ts-rs` derive" holds by construction `[OBS]` |
| CT-ST-004 (in-process) | open.rs `ct_st_004_in_process_two_stores_on_one_directory_write_without_loss` | pass; **not** a two-process test |
| CT-ST-005 | open.rs `ct_st_005_readers_proceed_during_a_long_write_*` | pass |
| CT-ST-010 | open.rs `ct_st_010_first_launch_*` | pass (no network: nothing opens a socket `[INF]`) |
| CT-ST-012 | open.rs `ct_st_012_*` (garbage file; damaged page), bytes unchanged | pass |
| CT-ST-013 (part) | open.rs `ct_st_013_*` (SchemaTooNew, no write, decided before the WAL pragma) | pass; "other-version process" and "migration needs no live process" are F-A03 (part B) |
| CT-ST-016 (missing half) | open.rs `ct_st_016_*`, `logs_config_and_empty_layout_*`, `a_done_journal_*` | pass; `CreateFreshStore` half is F-A07 |
| CT-ST-017, 018 | open.rs `ct_st_017_*` (v1 fixture opens; synthetic v2 upgrades; backup opens at v1 with old data; v1 build then refuses v2) | pass |
| CT-ST-019 | open.rs `ct_st_019_*` (existing name, missing dir, read-only dir; retry after a good location) | pass; see F-A02 |
| CT-ST-021 | open.rs `ct_st_021_*`, `ct_st_010_*` (modes, `store.id`) | pass |
| CT-ST-080 (store half) | games.rs `ct_st_080_store_half_*` | pass |
| CT-ST-028 (slot part) | games.rs `computers_slot_in_cc_*` | pass; reset half is F-A01 |
| DD-ST-002 failure behaviour | thread.rs unit tests (result, panic -> `Internal`, one reopen then `Unavailable`, caller walks away, failed reopen, drop joins) | pass |
| DD-ST-003 | readpool.rs unit tests (waiting caller, panicking reader returns its connection) | pass |
| DD-ST-006 | open.rs migrator tests (rollback of a failed migration, guard, version race -> `Busy`, no-op at equal, newer refused, `with` validation); backup.rs unit tests (0600, never overwritten, verification of junk and version mismatch, partial file removed) | pass |
| D-71 classification | open.rs unit tests (errno 45 and 25 refuse with path and errno; EIO, ENODEV, other are `Unavailable`) | pass; not on a real volume (F-A12) |
| DDL equals architecture 7.2 | drift_guard.rs (2 tests) | pass |
| Property (model vs implementation, `proptest`, 24 cases) | model.rs | pass |
| Fixture per schema version | tests/fixtures/schema-v1.sql (frozen copy of 0001 plus sample rows; a test proves it contains the migration text) | pass |

Red/green: the drift guard and several tests failed first for real reasons and were fixed (DDL statement count; foreign-database check ran after the WAL pragma wrote the file; two opens in one process collided on the probe scratch name; first `model.rs` run disagreed on the order of two errors). Other tests passed on their first run; their strength was checked by manual mutation (section 6), not assumed.

## 4. Commands run and results

All `[RAN]`, 2026-11-09, rustc/cargo 1.99.1.

| Command | Result |
|---|---|
| `cargo fmt -p chess-store -- --check` | exit 0 |
| `cargo fmt --all -- --check` | first attempt exit 1, all diffs in `crates/chess-engine/tests/zz_scratch.rs` (the other crate, mid-edit; not mine). The `cargo fmt --check` step of `ci/check.sh` later passed with no diff |
| `cargo clippy --offline -p chess-store --all-targets -- -D warnings` | `Finished`, no warning, no error |
| `cargo test --offline -p chess-store` | 8 result lines, all `ok`: lib 19, boundary 6, drift_guard 2, games 19, model 1, open 32, players 21, schema 13 = **113 passed, 0 failed**, doc-tests 0 |
| `cargo llvm-cov -p chess-store --offline --summary-only` (cargo-llvm-cov 0.9.1) | section 5 |
| `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| `CHESS_CI_OFFLINE=1 ci/check.sh` | **exit 101.** Passed: `cargo fmt --check`, `cargo build --locked`, `cargo clippy -D warnings`. Failed in `cargo test --workspace`: `chess-engine` `tests/process.rs` `ct_en_020_the_child_has_an_empty_environment_its_own_process_group_and_is_reaped` (`left: "info string env []"`, `right: "info string envcount 0"`). That is the other engineer's crate, **not chess-store**; every chess-store, chess-domain and chess-rules test passed. The script stops at the first failing stage, so the later stages did not run in that invocation |
| Remaining stages run by hand with the script's flags (`RUSTFLAGS="-D warnings"`) | `cargo test --workspace --locked --exclude chess-engine`: 51 result lines, 0 failures. `node ci/check-eco.mjs`: `eco snapshot check OK: 6 files match SHA256SUMS`. `cargo deny --offline check`: `advisories ok, bans ok, licenses ok, sources ok`. `npm audit` is skipped by `CHESS_CI_OFFLINE=1` |

`Cargo.lock` shows as modified in `git status`: Cargo recorded the new resolution when chess-store started to use the admitted pins (not hand-edited). The other crate in flight also changes it, so the two diffs are mixed.

## 5. Coverage (cargo-llvm-cov 0.9.1, `--summary-only`, as printed)

Line coverage per file (lines, missed, percent): `api.rs` 12/0 100.00; `backup.rs` 104/4 96.15; `datadir.rs` 157/14 91.08; `error.rs` 94/2 97.87; `migrate.rs` 80/7 91.25; `open.rs` 200/14 93.00; `readpool.rs` 88/1 98.86; `repo/game.rs` 220/3 98.64; `repo/mod.rs` 49/0 100.00; `repo/player.rs` 283/2 99.29; `repo/rating.rs` 37/2 94.59; `store.rs` 205/4 98.05; `thread.rs` 155/8 94.84. **TOTAL lines 1684, missed 61, 96.38 %**; regions 88.36 %; functions executed 92.06 %. Branches are not reported by this toolchain (0); MC/DC was not measured and none is claimed. Line counts include the `cfg(test)` modules in `src/`. The strategy gate (at least 90 % lines for the parser-class modules, and the SDD target for reconcile, format, reader and finalize repo functions, which are part B) is met by every part A file. Missed lines are error arms of OS calls (`create_dir`, `sync_all`, `read_dir` failures) and `?` lines in repos `[INF]`.

## 6. Test strength: manual mutation (cargo-mutants is not installed, so none was run)

Six hand-made mutants, each applied to a copy-restored source and run against the full suite `[RAN]`:

| Mutant | Outcome |
|---|---|
| slot-holder check disabled (`start_game`) | killed by 3 tests |
| rename's `id <> ?2` removed | killed (`rename_changes_the_name_*`) |
| backup call removed from the upgrade | compile error (dead-code lint on `write_verified_copy`); a compile-time kill, not a test kill |
| backup verification result ignored | first **survived**; a unit test for `verify_copy` was added; then killed |
| `RosterIndexTaken` pre-check disabled | killed by 3 tests |
| `SchemaTooNew` check removed from the opener | first **survived** (the later `upgrade` call returned the same error); `ct_st_013_schema_too_new_is_decided_before_the_wal_pragma_*` was added; then killed |

Sources were restored after each run (`diff -r` against the saved copy printed `restored`).

## 7. Findings (all OPEN; severity is the Engineer's, for the Reviewer and the approvers)

| ID | Sev | Finding | Kind |
|---|---|---|---|
| F-A01 | **Major** | The tracker split leaves operations with no owner task. Part A as written is "players and games access"; B is journal and leases; C is backup, restore, archive support, `FaultPlan`. None of the three names `apply_records`, `finalize_game` (with `EloCalc` and rating rows), `abort_game`, `void_unreadable_game`, `reset_player`, `check_rating_invariant`, the stats row queries, `upsert_move_eval`/`get_game_analysis`, variations, the job queue, `lease_beat`, `sync_backlog`. I built the part A acceptance list and the game and player reads and starts, and **did not** start these. The PM must assign them (they are in SDD 3.7.2 and the SDD tests UT-ST-050..056, 060, 082, 085, 086, 037, CT-ST-028, 030..032). No acceptance criterion of .3.6 is affected | planning |
| F-A02 | **Major** (design gap; G3 class: a data-directory name) | `chess.db.bak-v<N>` is never overwritten. If the backup succeeds and the migration then fails and rolls back, or a restored pre-upgrade copy is upgraded again, the next attempt gets `BackupFailed{TargetExists}` and the user cannot upgrade until the file is removed by hand. The SDD requires "retry succeeds" only for an unwritable folder (CT-ST-019). Options: (a) reuse the existing copy if it verifies at version N (risk: it is older than the data now in the store); (b) a numbered suffix `.bak-vN.k` (the `damaged/<uuid>.<k>.jnl` precedent); (c) replace it (loses the earlier copy, against "never pruned"). Implemented fail-safe, never replace. Needs a ruling by the architect and Brian | design |
| F-A03 | **Major** (obligation for part B) | The upgrade does not check that no other live process exists (SDD 3.11.5, CT-ST-015, `Exclusive`). The hook exists (`exclusive_guard`, runs inside `BEGIN EXCLUSIVE` after the version re-read) and the opener passes a guard that accepts, marked `PENDING part B`. `BEGIN EXCLUSIVE` still protects the file's integrity against a concurrent writer, but a running older build would not be told. Part B must wire the process registry in before any release | seam |
| F-A04 | Medium (stored shape) | `RATING_HISTORY.k` is `NOT NULL` and the SDD does not say its value for an initial point (nor, in D-26, for a reset point). Written as **0**, `game_id` NULL. The invariant check in part B must treat `game_id IS NULL` rows as points, not as games. Needs confirmation | stored format |
| F-A05 | Medium (catalogue) | `archive_player` has a guard (`has_unfinished`, SDD 3.7.2) but no error in the SDD 3.8 table. Added `StoreError::HasUnfinishedGames`. `chess-app` needs a catalogue mapping (architecture 5.5) or the guard belongs elsewhere | interface |
| F-A06 | Medium (stored shape, DTO) | (a) `NewPlayer` needs the initial preferences blob (UI-1), which the store cannot invent: added `preferences_json`; the SDD lists no field list for `NewPlayer`. (b) `GAME.white_clock_ms`/`black_clock_ms` of a timed game: the SDD says "checkpoint for resume"; `start_game` writes the base time for both. Not stated. (c) `NewGame` has no `self_play` or `custom_start`: both are derived (`white == black`, `start_fen.is_some()`), so an inconsistent pair cannot be sent | DTO, stored value |
| F-A07 | Medium (interface) | SDD 3.9.1 and CT-ST-016 describe `CreateFreshStore` (set the traces aside in `missing-<date>/`), but no store method or option names it. Not implemented. `Store::open` returns `Missing` and writes nothing | interface |
| F-A08 | Minor (stored shape) | No `v4` feature and no random crate are admitted for this crate (D-101, DL2-4), so `store_uuid` is built from 16 bytes of `std` `RandomState` through `uuid::Builder::from_random_bytes` (compiles without `v4` `[RAN]`; its randomness quality is not documented as guaranteed `[UNV]`; it only has to tell stores apart). The `store.id` JSON field names are those the SDD writes (`store_uuid`, `created_at`); `deny_unknown_fields` | stored format |
| F-A09 | Minor | A read-only connection on a WAL database creates empty `chess.db-wal` and `chess.db-shm` sidecar files, so "nothing written" for `SchemaTooNew` is true of `chess.db` (byte-identical, tested) but not of the directory listing | doc wording |
| F-A10 | Minor (observation for the app SDD) | `roster_indexes()` returns every stored index of every roster seed (as the SDD says). If `APP_SETTING.roster_seed` changes, the indexes of the old seed inflate `first_k_after` | observation |
| F-A11 | Minor (SDD text) | SDD 3.8 and section 4 still show `sys.rs`, "forbid unsafe except src/sys", and a `test-support` feature in the layout; the G0 note of 2026-11-09 covers only the 3.4 dependency table. Items that were `[UNV]` are now `[RAN]`: `json_valid` CHECK in the bundled SQLite (schema.rs), `VACUUM INTO` refuses inside a transaction (backup.rs `a_failed_copy_removes_the_partial_file`), the `uuid` builder. Close-on-exec of `File`/`OpenOptions` (SDD 3.11.6, CT-ST-006) is still `[UNV]` (needs a child process, part B) | docs |
| F-A12 | Info | D-71: the probe's error path cannot be driven on a real volume here (spike ASM-11) and `FaultPlan` is part C, so CT-ST-085 is covered at the classification function only. The errno values 25 and 45 are written in `open.rs` with their source (libc 0.2.191 `apple/mod.rs:2539, 2560` `[OBS]`) because no `libc` is allowed. `// PENDING D-71 refusal wording` marks the function; only front-end wording is open and it is not in this crate | scope |
| F-A13 | Info | `start_game` validates that the matchup agrees with the two Players' kinds (`InvariantViolated` otherwise). The SDD does not ask for it; it is a cheap guard on slot logic | choice |
| F-A14 | Info | The other crate's test failure and scratch file affected the workspace gate (section 4). `cargo test` workspace-wide is red until that engineer fixes it | process |

## 8. Choices inside the SDD's freedom (for the Reviewer and the desk review)

1. Repositories are free functions over `&mut Connection` that each open a `BEGIN IMMEDIATE`; reads take `&Connection`.
2. `StoreThread` takes closures, not a command enum, so `thread.rs` has no SQL and parts B and C add no variants.
3. Pre-checks inside the immediate transaction (name in use, roster index taken, slot held) give the typed errors; the table constraints stay as backstops that map to `InvariantViolated`. The order of two errors: the name check runs before the uuid constraint.
4. `quick_check` runs on a read-only connection before the write connection opens (keeps a corrupt file byte-identical). This reorders two steps of the 3.9.1 diagram.
5. A database with `user_version` 0 and tables is refused as `Corrupt` before any write; one with no tables (an interrupted first creation) is a first launch again, and on a failed first launch the files this open created are removed.
6. `DataDir::ensure` creates `journal/`, `journal/done/` and `backups/` (owned by B and C, created empty so a first launch has the full layout); the "traces" test ignores empty ones.
7. Environment resolution takes values as arguments (`resolve`), so tests need no `set_var` (unsafe in edition 2024); an empty flag or variable counts as unset.
8. `list_games` orders by start (row id) ascending; `list_unfinished` lists `in_progress` games that name the Player on either side (for a computer this includes the human's h-c games).
9. `Store::open` takes a `StoreOptions` with an injected wall clock (IFR-ST-05), the migrator (for the synthetic v2) and the read pool size (clamped to at least 1).
10. Integer conversions use `try_from`; a stored value that does not fit is `Corrupt`, a caller value that does not fit is `InvariantViolated`.

## 9. Seams for parts B and C

| Seam | Where | For |
|---|---|---|
| closure channel | `thread.rs` `StoreThread::run` | apply backlog, retry timer, `apply_records` (B) |
| `exclusive_guard` | `migrate.rs` `Migrator::upgrade` argument; opener passes `&\|_\| Ok(())` marked `PENDING part B` | process registry and `Exclusive` (B) |
| registration and recovery steps | `open.rs` `finish_open` (comment markers) and module doc | `register_process`, `OtherVersionRunning`, startup recovery (B) |
| `write_verified_copy` | `backup.rs` | `BackupManager`, pre-import copies (C) |
| `StoreError` | `error.rs`, `#[non_exhaustive]` | `Journal`, `Leased`, `Exclusive`, `OtherVersionRunning`, `UnlistedUnfinishedGames`, `VariationAnchor`, `ImportInvariant` |
| `repo/` | one file per group | `eval`, `stats`, `jobs`, `coord`, finalize in `game.rs`/`rating.rs` |
| `StoreId` | `datadir.rs` | journal header's store id (B) |
| manifest | `Cargo.toml` | B adds `crc32fast`; C adds the `test-support` feature (neither was needed here, so neither was added) |

No rename or no-replace move exists in part A, so there is no `// PENDING D-83` code.

## 10. Acceptance criteria (verbatim from the work order)

| Criterion | Met? | Evidence |
|---|---|---|
| Per SDD-chess-store as approved at G-2 | **Partly, as scoped** | Everything named for part A is built and tested. The SDD operations in F-A01 are not built and need an owner; items F-A02 to F-A08 are SDD gaps or choices |
| DDL equals architecture 7.2 | Met | drift_guard.rs: 14 ER tables, columns, order, PK flags, types equal; SDD DDL block equals the migration statement by statement (20 statements); constraints of SF-02, 12, 17 to 20 and D-43 tested in schema.rs. The architecture has the ER block and prose, no DDL text, so equality of constraints is to the SDD DDL `[OBS architecture.md:1485-1486]` |
| UUID-only boundary API | Met | boundary.rs UT-ST-038 (grep scan with self-test; repos private); public types use `PlayerId`/`GameId`; `StoreError` carries only uuid-typed ids |
| `roster_indexes()` | Met | `Store::roster_indexes`; players.rs UT-ST-039 (4 tests) and the domain default roster feeding `first_k_after` |
| Migration backup failure stops upgrade | Met | open.rs `ct_st_019_*` (existing name, missing dir, read-only dir): `BackupFailed`, `user_version` unchanged, no migration, the existing file untouched; see F-A02 for the retry gap |
| Integration tests on real temp-dir DB | Met | 94 integration tests, section 3 |
| Coverage per strategy | Met | section 5: 96.38 % lines, every file at least 91 %; no branch or MC/DC data |

## 11. Not run, not analyzed, assumed

- Not run: any two-process test (CT-ST-004 is in-process only; CT-ST-006, 008, 015, 025..027 need parts B and C); `FaultPlan` failure injection and kill-at-failpoint tests (part C); power-loss durability (cannot be shown here); the probe on a volume that fails `F_FULLFSYNC` (no such volume, ASM-11); Intel Mac (none available); benchmarks BM-ST-01..03; `cargo-mutants` (not installed); `npm audit` (offline run); the Reviewer pass; `cargo test --workspace` fully green (blocked by the other crate).
- Not analyzed: close-on-exec of descriptors (SDD 3.11.6) `[UNV]`; APFS behaviour of the `0600` mode on `-wal`/`-shm` beyond the one observed run (modes 0600 observed on this machine `[RAN]`, ct_st_010); network volumes; behaviour when `HOME` is unset beyond `resolve` returning `None`.
- Assumed: ASM-A1 the stored values for the items in F-A04 and F-A06 stay as written until Brian decides; ASM-A2 the app will pass a valid JSON preferences blob; ASM-A3 the unit and integration test names are the IDs of record until the SDD verification table is updated (this note does not edit the SDD).
- Follow-ups for the document owners (not done by me, outside my file list): SDD 3.8 and section 4 `Code:` fields are now real for part A; F-A11 text updates; bd comment on chess-a0r.3.6 is left for the dispatcher to read from this note.

## 12. Rework under CR chess-a0r.20 (baseline B6), 2026-11-10

Scope: allocation doc section 5.1 items 1 to 4, under Brian Prascher's rulings of 2026-11-09 (relayed): F-A02 REPLACE (D-108), D-71 any probe error (D-109). Engineer stance. Not reviewed, nothing approved.

### 12.1 Changes `[OBS]`

| Item | Change | Files |
|---|---|---|
| D-108 / F-A02 | New `backup::write_replacing_copy`: remove stale `chess.db.tmp-bak-v<N>`, `write_verified_copy` to the temp name (0600, `VACUUM INTO`, `quick_check`, `user_version`, file and directory sync), `rename` over `chess.db.bak-v<N>`, directory sync; failed rename removes the temp. `temp_name_for` maps `.bak-v` to `.tmp-bak-v` (other names get `.tmp-bak`); neither matches `chess.db.bak-v*`. `Migrator::upgrade` calls it. `write_verified_copy` still refuses an existing target; `BackupCause::TargetExists` stays in the enum (public API unchanged) but the Migrator no longer returns it. | `src/backup.rs`, `src/migrate.rs`, `src/error.rs` (doc) |
| D-71 / D-109 | `probe_durability` now wraps `probe_durability_with(root, sync)` (sync call as a seam). Any error of `sync_all` on the scratch file gives `DurabilityUnsupported{path, errno}` (`raw_os_error`, 0 if none). Create, write and remove errors of the scratch file stay `Unavailable` via `map_io`. `ENOTTY`, `ENOTSUP` and `classify_probe_error` removed (F-A12). | `src/open.rs`, `src/error.rs` (doc) |
| Unsafe | `#![forbid(unsafe_code)]` kept; no `sys.rs`. F-A03 `PENDING part B` marker in `open.rs` kept. | |
| Not touched | F-A04, F-A05, F-A06, F-A07, F-A08 (not ruled; chess-a0r.3.36). | |

Decision for the reader: a failed scratch `write_all` is not an error of `sync_all`, so it stays `Unavailable` (my reading, consistent with the architect's D-109 reading, which Brian has not yet confirmed `[UNV]`).

### 12.2 Tests

| Test | Covers |
|---|---|
| `ct_st_019_an_existing_backup_is_replaced_by_a_verified_copy_and_the_upgrade_proceeds` (replaces the old "existing name stops the upgrade" test, whose assertion is now the opposite by ruling) | CT-ST-019 extended: replaced copy at v1, 0600, `quick_check` ok, no temp left |
| `ct_st_090_a_stale_temp_file_is_removed_and_never_counted_as_a_backup` | CT-ST-090 stale temp |
| `ct_st_090_a_failed_attempt_leaves_the_previous_copy_byte_identical` | temp name blocked by a non-empty directory: `BackupFailed{Io}`, old copy identical, upgrade not started, retry succeeds |
| `ct_st_090_an_unwritable_directory_keeps_the_previous_copy_byte_identical` | directory 0500: `PermissionDenied`, old copy identical |
| `ct_st_090_the_retry_after_a_rolled_back_migration_succeeds_without_a_manual_step` | the F-A02 failure, through `Store::open` path |
| `ct_st_090_a_restored_pre_upgrade_copy_can_be_upgraded_again` | restore then upgrade, copy replaced |
| unit `backup::tests`: `the_temp_name_never_matches_the_backup_pattern`, `replacing_swaps_in_a_verified_copy_and_leaves_no_temp_file`, `a_failed_replacement_keeps_the_old_copy_byte_identical_and_removes_the_temp`, `a_failed_rename_removes_the_temp_file_and_reports_io`, `write_verified_copy_itself_never_overwrites` | UT for the new function |
| unit `open::tests`: `any_sync_error_refuses_the_directory_with_path_and_errno` (errno 45, 25, 5, 19), `a_non_regular_file_sync_error_from_the_spike_refuses` (real `sync_all` on `/dev/null`), `scratch_file_errors_stay_unavailable`, `a_passing_probe_is_ok` | UT and CT-ST-085 as far as possible without a failing volume |

Removed tests (they pinned the old behaviour): `enotsup_and_enotty_refuse...`, `any_other_probe_error_is_unavailable_io`, `the_errno_constants_match_the_macos_values_read_from_libc`.

**CT-ST-090 gap (stated):** the kill-before-rename case (`backup.before_rename`) needs `FaultPlan` and is added with D1. The `/dev/null` test shows the real call fails there and refuses; whether it fails with ENODEV is platform output, the test compares to the direct call.

### 12.3 Results `[RAN]`

- `cargo fmt -p chess-store --check`: exit 0. Workspace `cargo fmt --check` fails only in `crates/chess-engine/tests/rework.rs` (the other engineer's file, mid-rework), so `CHESS_CI_OFFLINE=1 ci/check.sh` stopped at its first step, `cargo fmt --check`, on that file: **interference by the engine rework, not by chess-store**. The rest of the script did not run.
- `cargo clippy -p chess-store --all-targets -- -D warnings`: clean.
- `cargo test -p chess-store`: all suites ok (24 lib, 6, 2, 19, 1, 37, 21, 13 passed; 0 failed).
- `node ci/check-crate-edges.mjs`: "crate-edge check OK: 8 crates, 11 workspace edges, all allowed".
- `cargo llvm-cov -p chess-store --summary-only`: lines TOTAL 96.07 % (1783 lines, 70 missed); regions 88.75 %; functions 91.47 %. Changed files, lines: `backup.rs` 95.98 %, `migrate.rs` 91.25 %, `open.rs` 91.27 %. No branch or MC/DC data.
- Hand mutants (restored after each, files diffed identical): M1 skip stale-temp removal, M2 copy instead of rename, M3 temp name equals target, M4 ignore sync error: all four killed (named failing tests above). M5 (hard-code errno) was not run (my edit command failed); `any_sync_error...` asserts the errno for four values, so it should kill it `[INF]`. `cargo-mutants` 27.1.0 is installed now but was not run.

### 12.4 Findings (Engineer's own)

- **FIND-R01 (Minor).** `Path::new("x").parent()` is `Some("")` for a bare relative backup name, so the directory sync fails with `Io(NotFound)`; the same holds for part A before the rework. The Opener always passes an absolute data-directory path. Not changed.
- **FIND-R02 (Minor).** If the directory sync after the rename fails, the new verified copy is already in place but `BackupFailed{Io}` is returned and the upgrade does not start; the next attempt replaces it again. Safe, stated.
- **FIND-R03 (Minor, `[UNV]`).** Atomicity of `std::fs::rename` over an existing file on APFS is POSIX behaviour, not shown here; only the replace result is tested. Same unverified point as the SDD dated note.
- **FIND-R04 (Info).** Probe errno is 0 when the sync error carries no OS code; Rust's `sync_all` errors always do on Unix `[INF]`.
- No blockers. No Major finding.

## 13. Rework 2 under CR chess-a0r.20 (baseline B6), 2026-11-10: Reviewer findings of review-chess-store-A.md

Engineer stance. Source: [review-chess-store-A.md](review-chess-store-A.md) (PASS WITH CONDITIONS, 3 Major). Nothing here is reviewed or approved. F-A04 to F-A08 behaviour is unchanged.

### 13.1 Per finding

| Finding | Reproduced before the fix | Fix `[OBS]` | Tests |
|---|---|---|---|
| FIND-001 (Major) first-launch race | `[RAN]` new test failed on the part A code: `round 0: untyped or spurious error Unavailable { cause: Io(AlreadyExists) }` (threads) and `round 3: [20, 30, 10]` (three processes; 20 = Busy, 30 = other error) | `open_blocking` no longer deletes anything. `chess.db` is made with `create_new`; `AlreadyExists` means "someone else is the creator" and the opener carries on with the existing file. The trace scan is re-checked against `db.exists()` (a concurrent creator makes `chess.db` before `store.id`). A first launch that lost the exclusive migration (`Busy`) repeats the upgrade step, up to 8 times with a 5 ms times n pause; `first_launch` is now "my upgrade took the schema from 0" (`outcome.from == 0`). The write connection sets `busy_timeout` before the WAL switch, and the read-only probe connection has a busy timeout. | `ct_st_004_concurrent_first_launches_never_delete_the_winners_database_200_rounds` (4 threads, 200 rounds: all 4 open, exactly one reports first launch, `chess.db` exists, all 4 writes present after reopen, `store.id` present); `ct_st_004_three_processes_opening_a_fresh_directory_lose_nothing_60_rounds` (3 child processes of the test binary, 60 rounds, same checks) |
| FIND-002 (Major) `store.id` left behind | by the Reviewer (`[RAN]` there: next open `Missing`); my test needs the seam below | Rule: **the database file is created first, `store.id` second, the schema third, and a failed first launch removes nothing.** What stays is an empty `chess.db` (no tables, `user_version` 0), maybe `store.id` and the empty layout: the "interrupted first creation" state the opener already treats as a first launch, never `Missing` (the `Missing` test fires only when `chess.db` is absent). A retry keeps the `store.id` of the first attempt. `store.id` is written atomically (temp `.store.id.tmp-<pid>-<n>`, sync, `hard_link` to `store.id`, remove temp), content and field names unchanged; a loser that gets `AlreadyExists` reads the winner's file. | `open::tests::a_failed_first_launch_after_store_id_is_a_first_launch_again_not_missing` (thread-local `cfg(test)` failure seam right after `store.id` is written) |
| FIND-003 (Major) pragmas | by the Reviewer (mutants M41 to M44, M49 survived) | `apply_write_pragmas` split out of `open_write_connection` (same statements and order, plus `busy_timeout` first) | `open::tests::the_write_connection_has_every_normative_pragma_read_back` (WAL, synchronous 2, fullfsync 1, checkpoint_fullfsync 1, foreign_keys 1, busy_timeout 5000); `open::tests::the_open_is_refused_when_journal_mode_does_not_become_wal` (in-memory connection answers `memory`) |
| FIND-004 | `[RAN]` both new tests failed (`Internal`) | `map_sqlite`: `FromSqlConversionFailure`, `InvalidColumnType`, `IntegralValueOutOfRange`, `Utf8Error` give `Corrupt` | `text_in_an_integer_column_is_corrupt_not_internal`, `invalid_utf8_in_a_text_column_is_corrupt_not_internal` (`tests/players.rs`) |
| FIND-005 | n/a (choice) | SDD 3.7.2 silent: behaviour kept (a computer gets a preferences row), pinned | `choice_update_preferences_on_a_computer_creates_a_row_pinned` |
| FIND-006 | n/a (choice) | SDD silent: kept. Same id and same name gives `NameTaken` `[RAN]` (the Reviewer had it as LIKELY), same id and another name gives `InvariantViolated` | `choice_register_twice_with_the_same_id_is_not_idempotent_pinned` |
| FIND-007 | Reviewer, LIKELY | atomic `store.id` write (above); format unchanged. The "missing `store.id` of an existing store is not recreated" behaviour is **unchanged** (recreating an identity for a store with journals is not safe to decide here) | covered by the FIND-001/002 tests; no test for the missing-file case |
| FIND-008 | `[RAN]` new test failed (`Missing`) | `.DS_Store` and `._*` are not traces in `backups/`, `journal/`, `journal/done/` etc.; a real file still is | `ds_store_files_are_not_traces_of_a_lost_store` |
| FIND-009 | `[RAN]` new test failed (`Internal`) | The bundled SQLite is compiled with `SQLITE_USE_URI` (libsqlite3-sys 0.39.1 `build.rs:184` `[OBS]`), so open flags do not help: every path handed to SQLite goes through `datadir::sqlite_path` (relative path gets `./`): write, read, probe, `VACUUM INTO`, backup verify | `relative_path::a_relative_directory_named_like_a_sqlite_uri_is_used_as_a_path` (own test binary, changes the working directory) |
| INITIAL_POINT_K | n/a | **no behaviour change** | `f_a04_initial_rating_points_carry_k_zero_and_no_game_pinned_not_ruled` (comment states F-A04 is not ruled) |
| FIND-010 | n/a (part B) | comment in `open.rs::finish_open` at the `PENDING part B (F-A03)` marker: the backup runs before the guard, part B must move the "alone" check before the backup and test that a refused upgrade leaves the old copy byte-identical | none (part B) |

### 13.2 Deferred, not done (as ordered)

FIND-011 (public surface), FIND-012 (unbounded queue, info), FIND-014 (docs coupling of `drift_guard`), FIND-015 (info items) and the sync and fault-injection mutants M16, M17, M22, M30, M31, M35 (and the kill-before-rename CT-ST-090 case): D1 (`FaultPlan`). Also not done: M3, M10, M11, M32, M53 (M53 is now killed by the F-A04 pin test) and FIND-013's remainder.

### 13.3 Behaviour changes the architect should see (no approval is claimed)

1. A failed first launch **no longer removes `chess.db*`**; it leaves an empty `chess.db`. SDD 3.9.1 and DD-ST-005 say "every error path leaves the files as found"; part A tried to keep that by deleting, and that deletion is the FIND-001 defect. This is inside one crate, with the Reviewer's finding as the driver; I treat it as G1 and name it here for the system architect. `[UNV]` whether the SDD wording needs a dated note.
2. `store.id` is now written **before** the schema is created, and by temp file and `hard_link`. A transient name `.store.id.tmp-<pid>-<n>` appears in the data directory for a moment (like `.durability-probe-*`). Format and final name unchanged.
3. `OpenReport.first_launch` is true only for the opener whose upgrade created the schema.
4. A first launch that meets `Busy` retries (at most 8 times) instead of returning `Busy`.

### 13.4 New finding from my own loop (fixed)

**FIND-R05 (Major while open, now FIXED).** The first full 10x loop of the rework failed once (run 9): the 200-round thread test returned `Corrupt` in round 8 (`[RAN]`). Cause `[OBS]`: `open_blocking` read `user_version` and the table count as two separate reads, and `Migrator::upgrade` did the same; a concurrent first launch that committed its schema between them made a good database look like "tables with version 0". Fix: re-read the version when tables are found with version 0 (`open.rs`), and `Busy` instead of `Corrupt` in `Migrator::upgrade` when the version changed between the reads (`migrate.rs`). After the fix: the thread test 15 of 15 clean, the full suite 10 of 10 clean. The thread test is the regression test; there is no deterministic test of that window `[UNV]`.

### 13.5 Results `[RAN]`

- `cargo test -p chess-store` ten times in a row after the last code change: 10 of 10 exit 0, 136 tests passed each time (0 failed). Race tests: 200 rounds of 4 threads and 60 rounds of 3 processes per run.
- `cargo fmt -p chess-store --check`: exit 0. `cargo clippy -p chess-store --all-targets -- -D warnings`: clean. `node ci/check-crate-edges.mjs`: "crate-edge check OK: 8 crates, 11 workspace edges, all allowed".
- `cargo llvm-cov -p chess-store --summary-only`: lines 1889, missed 79, **95.82 %** (the rework 1 figure was 96.07 %; the new code has uncovered error arms). Changed files by lines: `backup.rs` 96.00, `datadir.rs` 92.39, `error.rs` 96.84, `migrate.rs` 90.24, `open.rs` 91.12. No branch or MC/DC data.
- Hand mutants (killed means a named test failed): pragma M41 `synchronous`, M42 `fullfsync`, M43 `checkpoint_fullfsync`, M44 `foreign_keys`, M45 `busy_timeout` killed by `the_write_connection_has_every_normative_pragma_read_back`; M49 WAL check killed by `the_open_is_refused_when_journal_mode_does_not_become_wal`; no re-check of `exists()` after the trace scan killed by the 200-round thread test; retry count 8 to 1 killed by both race tests; `.DS_Store` rule removed killed by `ds_store_files_are_not_traces_of_a_lost_store`; text-in-integer and UTF-8 mappings to `Internal` killed by the two FIND-004 tests. **Survived:** removing only the `FromSqlConversionFailure` arm of `map_sqlite` (the two FIND-004 tests reach `InvalidColumnType` and `Utf8Error`, not that variant); making `write_store_id` replace instead of first-wins (`rename` for `hard_link`) survives, because the race tests cannot tell whose `store.id` stands. The mutants were run before the FIND-R05 fix and not repeated after it.

### 13.6 Not run, unverified

Two-process behaviour for more than three children and for the upgrade (not first-launch) race; `cargo-mutants`; a deterministic test for the FIND-R05 window; the FIND-002 old-code failure (shown by the Reviewer, not re-run by me against the old code, since the seam did not exist there); power loss; APFS; Intel.
