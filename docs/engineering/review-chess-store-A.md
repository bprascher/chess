# REV-chess-store-A: adversarial review of chess-store part A (bead chess-a0r.3.6)

| Field | Value |
|---|---|
| Stance | Reviewer, independent (fresh invocation, did not write the code, did not rely on the author's evidence file) |
| Baseline | B6 |
| Date | 2026-11-10 |
| Scope | `crates/chess-store` part A: `DataDir`, schema and migration 0001, `Migrator`, pre-upgrade backup, open sequence up to the upgrade, store thread, read pool, players and games access, `roster_indexes()`, UUID-only boundary |
| Out of scope (not findings) | Operations owned by D1..D5 and parts B and C (journal, leases, registry, apply, finalize, void, reset, backup manager, row IO, FaultPlan, CreateFreshStore) |
| Gate | G0 review artifact. No code or other document edited. No git commit, no bd close |

## 0. Verdict

**PASS WITH CONDITIONS.**

- The rework scope of section 5.1 of `store-completion-allocation.md` (D-108 replace-by-rename, D-109 any-`sync_all`-error refusal) is implemented as specified and its observable behaviour is tested. The DDL equals SDD 3.5.2 byte for byte after comment and whitespace removal.
- Three **Major** findings stay open and are conditions of acceptance: FIND-001 (a first-launch race deletes a live database), FIND-002 (a failed first launch poisons the directory with `Missing`), FIND-003 (the normative write-connection pragmas are not tested at all). FIND-001 is a data-destroying defect that I reproduced; I would not release with it open. It is not a reason to block D1, which does not need to touch `open_blocking`'s cleanup logic, but D1 and part B must not build on that cleanup path as it is.
- 123 tests pass 10 out of 10 runs, fmt and clippy are clean, and 58 of 77 hand mutants were killed. 19 survived, and they cluster on exactly the places where the Major findings are (FIND-013).

Brian has not ruled F-A04 to F-A08. They are judged as risks and options in section 4 and nothing in the code is called wrong for them.

## 1. Snapshot

| Item | Value |
|---|---|
| Snapshot time (UTC) | 2026-11-10T20:18:14Z |
| `crates/chess-store` tree hash | `883f94a68f3ae3f476c7c9803bf858fea26df16bf99eec1daf68818a8793da86` (25 files) |
| Hash command | `cd crates/chess-store && find . -type f | sort | xargs shasum -a 256 | shasum -a 256` |
| Copied with it | `crates/chess-domain`, `crates/chess-rules`, root `Cargo.toml` (member list trimmed to the three crates, nothing else changed), `Cargo.lock` (cargo rewrote it with `--offline`), `deny.toml`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `third_party/` (chess-rules build script needs it), and the documents `drift_guard.rs` reads at test time (`docs/architecture.md` sha256 `84ec218e...c99af`, `docs/design/SDD-chess-store.md` sha256 `d85adc9f...6f38`) |
| Review location | Session scratchpad `.../scratchpad/ws`, `ws2`, `ws3` with their own `CARGO_TARGET_DIR` (`target`, `target2`, `target3`, `target-cov`). Nothing under `crates/` was edited |
| Live tree vs snapshot | At 20:43Z `diff -rq` between the snapshot and the live `crates/chess-store` printed nothing: D1 had not yet changed the crate. Line numbers below are valid for both |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1, cargo-llvm-cov 0.9.1 `[RAN]` |

## 2. Commands and results `[RAN]`

| Command (in the scratch workspace) | Result |
|---|---|
| `cargo fmt --check -p chess-store` | exit 0, no diff |
| `cargo clippy -p chess-store --all-targets --offline -- -D warnings` | exit 0. Workspace lints deny `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, both lossy-cast lints, and `warnings = deny` |
| `cargo test -p chess-store --offline --no-fail-fast`, 10 consecutive runs | 10/10 identical: result lines ok for 24 (lib), 6, 2, 13, 19, 21, 37, 1, plus 0 doc tests = **123 passed, 0 failed, 0 ignored**. Per run about 4 s |
| `cargo test -p chess-store` without `--no-fail-fast` | The first two runs of my session failed `drift_guard` (2 tests) because the snapshot lacked `docs/`. After copying the two documents they pass. This is finding FIND-014 (tests depend on files outside the crate) |
| `cargo llvm-cov -p chess-store --offline` | Lines 1783, missed 70, **96.07 %**; regions 3092, missed 348, 88.75 %. Per file lines: api 100, backup 95.98, datadir 91.08, error 97.87, migrate 91.25, open 91.27, readpool 98.86, repo/game 98.64, repo/mod 100, repo/player 99.29, repo/rating 94.59, store 98.05, thread 94.84 |
| `cargo llvm-cov ... --branch` | Fails: `-Z coverage-options=branch` needs nightly, and D-76 forbids nightly. **No branch or MC/DC coverage exists or is claimed** |
| Python check of the DDL | `migrations/0001_init.sql` and the SDD 3.5.2 SQL block, comments removed and whitespace collapsed: equal, 5627 characters, 21 statements, none missing on either side |

Line coverage includes the `#[cfg(test)]` modules inside `src/`. The author's evidence file `step2-store-A-evidence.md` lists 113 tests and 96.38 %; it predates the rework and is stale (123 and 96.07 % now). I used only my own runs.

## 3. Findings

Severity: critical / major / minor / info. Confidence: CONFIRMED (reproduced here), LIKELY (reading, not reproduced), SUSPECTED. Probe code lived in the scratch tree only (`tests/review_probe.rs`, not part of the snapshot hash).

### FIND-001 [major][CONFIRMED] A first-launch race deletes the live database of the process that won

- Location: `src/open.rs:137-192` (`open_blocking`), cleanup at `:185-190`; contributing: `:146` (`db.exists()` then `:172` `create_new_0600`, a check-then-act), `src/migrate.rs:117-121` (the re-read that returns `Busy`).
- Mechanism `[OBS]`: opener A creates the empty `chess.db` (`created_db_file = true`). Opener B sees the file, reads `user_version` 0, sets `fresh = true`, wins the `BEGIN EXCLUSIVE`, migrates, writes `store.id`, and returns `Ok`. Opener A then loses the version re-read (`Busy`), and its error arm removes `chess.db`, `chess.db-wal` and `chess.db-shm`. B holds a `Store` on an unlinked file.
- Reproduction `[RAN]`: 40 rounds, each with 3 threads calling `Store::open` on one fresh directory (same code path as three processes). Result: **5 of 40 rounds ended with a successful open and no `chess.db` in the directory.** A separate 6-thread round printed `["Unavailable{Io(AlreadyExists)}", "Busy", "Busy", "Busy", "Busy", "ok"]`, directory listing `["backups","journal","store.id"]`, and the next open returned `Missing`.
- Impact: the winner's later writes go to a deleted inode and are lost at exit, silently. This breaks NFR-2 (no completed move lost) for the first session after a simultaneous first start of two front-ends (CLI plus GUI is the plausible case; all front-ends share one data directory by design). The same race also gives losers a spurious `Missing` (seen in two rounds) and `Unavailable{AlreadyExists}`. With `CreateFreshStore` (F-A07, part C) a spurious `Missing` would lead to a live store's files being moved to `missing-<date>/`.
- Trace: SDD 3.9.1 and DD-ST-005 ("every error path ... leaves the files as found"); DAT-1, NFR-2. The code comment at `open.rs:184` says "a first launch that failed leaves no store", which is false when another opener is using the file.
- Verify the fix by: a test that runs N >= 4 concurrent first launches, in threads and in child processes, 100+ rounds, and asserts that if any open returned `Ok`, `chess.db` exists and contains that opener's writes after all openers finish; and that no opener removes a file it did not prove it alone created and still alone uses. Mutant 48 (cleanup disabled) survives today, so the cleanup path has no test either way.

### FIND-002 [major][CONFIRMED] A failed first launch after `store.id` is written leaves the directory as `Missing`

- Location: `src/open.rs:196-231` (`finish_open`): `store.id` written at `:214`, reader connections opened at `:221`, directory sync at `:225`; cleanup at `:185-190` removes only `chess.db*`.
- Reproduction `[RAN]`: `ulimit -n 120`, `StoreOptions.read_connections = 5000`, fresh directory. First open: `Unavailable{Io(Other)}`. Directory afterwards: `["backups","journal","store.id"]`. Second open with default options: **`Err(Missing)`**. Any later failure at `:221` or `:225` (fd exhaustion, an `fsync` error on the directory) does the same.
- Impact: a store that never held data is reported as a lost store, and the user is offered "fresh store or import". The only recovery is deleting `store.id` by hand, which no message tells them. `has_traces_of_a_store` treats `store.id` as a trace by design (`datadir.rs:181`).
- Trace: DD-ST-005 failure behaviour; SDD 3.9.1 volume probe paragraph ("a first-launch directory it just created stays, empty").
- Verify by: a test that forces a failure after `store.id` is written (fd limit as above, or the part D1 failpoint) and asserts the next open is a first launch again.

### FIND-003 [major][CONFIRMED] The normative write-connection pragmas have no test; four mutants of them survive

- Location: `src/open.rs:36-53` (`open_write_connection`).
- Evidence `[RAN]` (hand mutation, section 6): `synchronous=FULL` changed to `NORMAL` (M41), `fullfsync` ON to OFF (M42), `checkpoint_fullfsync` ON to OFF (M43) and **`foreign_keys` ON to OFF (M44) all survive; all 123 tests pass.** M49 (delete the check that `journal_mode` really became WAL) also survives.
- Impact: the durability configuration of architecture 7.1 and SDD 3.12 (NFR-2: on macOS a plain fsync does not flush the drive cache) and the foreign-key backstop can regress silently. `busy_timeout` (M45) is covered only indirectly by one concurrency test.
- Trace: architecture section 7.1 pragma list (line 1323), SDD 3.12 pragma row.
- Verify by: a unit test that opens a connection through `open_write_connection` and asserts each `PRAGMA` value, and one that opens a non-WAL-capable location and asserts the refusal; re-run M41 to M44 and M49 and see them killed.

### FIND-004 [minor][CONFIRMED] Stored-value corruption maps to `Internal`, not `Corrupt`

- Location: `src/error.rs:104-127` (`map_sqlite` falls through to `Internal`); read paths `src/repo/player.rs:35-80`, `repo/game.rs:36-80`.
- Reproduction `[RAN]`: `UPDATE player SET strict_elo='abc'` then `get_player` gives `Internal`; invalid UTF-8 in `player.name` also gives `Internal`; `list_players` the same.
- Why it can happen: SQLite accepts text in an `INTEGER` column unless a CHECK forbids it, and the SDD-normative DDL has no `typeof` checks. The error doc on `Corrupt` says "or stored text no enum accepts", and the SDD table maps this class to `StoreCorrupt`; `Internal` maps to "a bug" in `chess-app`.
- Impact: a hand-edited or foreign-written database produces a "bug" report instead of a corruption report. Minor.
- Verify by: a test with a text value in each integer column read by part A, expecting `Corrupt`.

### FIND-005 [minor][CONFIRMED] `update_preferences` on a computer Player creates a preferences row

- Location: `src/repo/player.rs:294-307`.
- Reproduction `[RAN]`: `update_preferences(computer_id, "{}")` returns `Ok`, then `get_preferences` returns `Some("{}")`. The doc comment on `get_preferences` (`:310`) says `None` for a computer, and `players.rs::a_computer_has_no_preferences_row` covers only the insert path.
- Impact: contradicts the documented contract; a computer then has a row that export and reset code written later will not expect. The SDD 3.7.2 is silent. Minor.
- Verify by: a decision (reject with a typed error, or document), then a test.

### FIND-006 [minor][CONFIRMED for a new name, LIKELY for the retry] `register_player` is not idempotent, so a retry after a cancelled call gets `NameTaken`

- Location: `src/repo/player.rs:141-175`; `src/thread.rs:60-66` (the job runs to completion after the caller stops waiting, by design and tested).
- Reproduction `[RAN]`: second `register_player` with the same `PlayerId` and a different name gives `InvariantViolated` (also pinned by `players.rs::registering_the_same_id_twice_is_an_invariant_violation`). The same id and the same name gives `NameTaken` by reading (`:147` runs before the insert); I did not run that case. `start_game` is idempotent on `GameId` (`game.rs:206`); registration is not.
- Impact: a front-end that times out or is cancelled while the write commits, then retries, is told the name is used by someone else. Minor; policy belongs to `PlayerService`.
- Verify by: decision (idempotent on id and name, or documented) plus a test.

### FIND-007 [minor][LIKELY] `store.id` is written non-atomically and never repaired

- Location: `src/datadir.rs:169-175`: `create_new`, then `write_all`, then `sync_all`. A crash between create and write leaves an empty file; `read_store_id` then returns `Corrupt` (`:158-166`).
- Reproduction `[RAN]` for the related case: deleting `store.id` of an existing store, then reopening, succeeds and does not recreate it (`fresh` is false). In part A nothing reads it after creation, so no failure shows yet; part B's foreign-store journal check will.
- Impact: part B inherits a torn-file and a missing-file state with no defined handling. Also `deny_unknown_fields` makes an older build refuse a `store.id` written by a newer build as `Corrupt`.
- Verify by: temp-file-and-rename write, plus a test for the empty and the missing file.

### FIND-008 [minor][CONFIRMED] A `.DS_Store` in `backups/` (or any file in `journal/`) is a trace of a lost store

- Location: `src/datadir.rs:191-223` (`has_traces_of_a_store`, `dir_has_entries`, `journal_has_content`).
- Reproduction `[RAN]`: open, delete `chess.db*` and `store.id`, add `backups/.DS_Store`: next open returns `Missing`. Finder creates that file when a user browses the folder. Only matters when `chess.db` is absent. Minor.
- Verify by: ignoring OS metadata names, with a test.

### FIND-009 [minor][CONFIRMED] A relative data directory whose name starts with `file:` is parsed as a SQLite URI

- Location: `src/open.rs:38` (`Connection::open`) and `:154` (read-only probe): rusqlite's default flags include `SQLITE_OPEN_URI`.
- Reproduction `[RAN]`: `DataDir::new("file:dd?mode=memory&cache=shared")` creates a directory with that literal name, then `open` returns `Internal`. Absolute paths cannot start with `file:`, and `VACUUM INTO` binds the path as a parameter, so this needs a contrived relative `--data-dir`. Info-level in practice.
- Verify by: reject or prefix such paths with `./`, with a test.

### FIND-010 [minor][LIKELY] Order of backup and guard will make a refused upgrade overwrite the last good backup once part B wires the guard

- Location: `src/migrate.rs:111-122`: the backup is written at `:114`, then `BEGIN EXCLUSIVE`, then `exclusive_guard` at `:122`. Part B's acceptance line (`store-completion-allocation.md` 5.2) wires the guard to "no other live process".
- Impact: an upgrade refused with `Exclusive` or `OtherVersionRunning` has already replaced `chess.db.bak-v<N>` with a fresh copy. The new copy is verified, so no corruption, but SDD 3.9.1 checks "alone" before the backup. A backup taken before the exclusive lock can also miss rows another process commits in between. Part A alone does not enforce "no other live process" and says so (`migrate.rs:10-13`, `open.rs:206`).
- Verify by: part B test with two real processes where the refused upgrade leaves `chess.db.bak-v<N>` byte-identical.

### FIND-011 [minor][CONFIRMED by reading] Public surface exceeds the SDD

- Location: `src/lib.rs:25-33`, `src/migrate.rs:86-92`, `src/store.rs:35-42`.
- `Migrator::upgrade` is public and takes any `&mut Connection`, any backup path closure and any guard closure; a closure that returns the database path itself makes `write_replacing_copy` rename a copy over the live file. `Migrator::with`, `StoreOptions.migrator` and `read_connections` (no upper bound, one fd each) are public and not behind a test feature. `StoreId`, `EnsureReport` and the layout paths are public. No integer key leaks (D-40 holds, see section 5). Minor.
- Verify by: narrowing visibility or gating test support behind the `test-support` feature that D1 adds.

### FIND-012 [info][CONFIRMED] Unbounded writer queue; `Drop` joins on the caller's thread

- Location: `src/thread.rs:39` (`mpsc::channel`), `:77-85`. The SDD fixes the unbounded channel (DD-ST-002). I measured 2000 concurrent `register_player` calls: all 2000 succeeded, 19.3 s wall, about 9.6 ms each (a full fsync per commit); nothing pushes back.
- Dropping the last `Store` handle drains the whole queue before returning, on whichever thread drops it, including a runtime worker. Reader calls hold one `spawn_blocking` thread each while waiting for one of 2 connections (`readpool.rs:44-60`). No deadlock found: the pool holds no lock while a query runs, panics return the connection (tested), poisoned mutexes are recovered. Info: parts B and C add the backlog on top of this.

### FIND-013 [minor][CONFIRMED] Test adequacy: 19 of 77 hand mutants survive

See section 6. Survivors that matter: M22 (result of `verify_copy` ignored in the backup path), M16 and M17 (directory sync after rename, file sync of the copy), M30 and M31 (`store.id` file and directory sync), M35 (the real `File::sync_all` replaced by a no-op in `probe_durability`), M41 to M44 and M49 (FIND-003), M48 (cleanup of FIND-001 and FIND-002), M53 (the `k` value of the initial rating points, see F-A04), M11 (negative `user_version`), M10 (`BEGIN EXCLUSIVE` replaced by `DEFERRED`), M3 (second table-count check in `migrate.rs`; `open.rs` catches the same case first), M32 (the `NotADirectory` kind is not asserted). The sync and verify survivors need fault injection and are D1's to close (`backup.before_rename`, CT-ST-085 and CT-ST-090 kill cases); nothing else in the crate can observe them. M57 (secondary sort of equal `u32` values) and M68 (half-null clock pair, which the `move` CHECK already forbids) are equivalent or unreachable.

### FIND-014 [info][CONFIRMED] Crate tests depend on documents outside the crate

- Location: `tests/drift_guard.rs:25-30, 83, 147` read `docs/architecture.md` and `docs/design/SDD-chess-store.md` through `CARGO_MANIFEST_DIR/../..`; the parser finds the section by the literal heading `### 7.2 Data model`. Without those files, two tests fail with a panic, and `cargo test` without `--no-fail-fast` then stops before running the other test binaries (I saw exactly that). The guard is good for drift; the coupling should be a conscious choice. The ER comparison checks table names, column names and order, primary-key columns and coarse types; the ER block carries no constraints, so constraints are guarded only against the SDD text (verified independently in section 2).

### FIND-015 [info][CONFIRMED] Smaller observations

- `probe_durability` leaves `.durability-probe-<pid>-<n>` behind if the process is killed between create and remove (`open.rs:77-103`); nothing cleans them. Harmless.
- `ensure_root` creates missing parents with mode 0700 (`datadir.rs:226-231`), so `--data-dir ~/Documents/new/chess` makes `new` private too. Documented in the doc comment; confirm intent.
- `ensure_layout` follows a symlinked `journal/` to another directory and creates `done/` there (probe `e7`); a symlinked data directory and a dangling `chess.db` symlink (reported `Missing`) behave sensibly.
- Names: the store stores `a\0b`, whitespace-only names and un-normalized Unicode as distinct names, and rejects `"\0x"` because SQLite `length()` stops at the NUL. `PlayerService` owns name policy (SDD 3.7.2); noted so it does not assume the store normalizes.
- `list_games` with an unknown `filter.player` returns `NotFound`, not an empty list (`game.rs:110`), and `limit` returns the oldest N games (`ORDER BY g.id`). Both are consistent with the SDD text but worth stating to the app.
- `Store::open` outside a tokio runtime panics inside tokio (`store.rs`, doc says "must be called inside a tokio runtime").

## 4. Items Brian has not ruled (risks and options only)

| Item | What the code does | Risk | Options |
|---|---|---|---|
| F-A04 rating row of a new Player | `game_id` NULL, `k` = 0, `rating_before == rating_after` (`repo/player.rs:15, 123`) | A reset point (part B) also has NULL `game_id`. Initial and reset rows are told apart only by values, and a reset to an equal rating is indistinguishable. `INITIAL_POINT_K` has no test: mutant M53 (k = 1) survives | (a) keep k = 0 and document that no real game has k = 0; (b) nullable `k`, a stored-format change needing G3; (c) a marker column. (a) costs nothing now; the Elo invariant check in part B must use before and after, not `k` |
| F-A05 `HasUnfinishedGames` | New variant, Display "the player has unfinished games", guard runs inside the same `BEGIN IMMEDIATE` | Not in the SDD error table (3.8) or in the protocol mapping of architecture 5.5; `chess-app` must map it | Name it in the SDD table, or reuse an existing mapped variant. Cheap either way |
| F-A06 (a) `preferences_json`, (b) base time at both clock checkpoints, (c) derived `self_play` and `custom_start` | As described in the evidence file | (c) is safe by construction (tested). (b) is a stored value the SDD does not state; a resume reads the checkpoint. (a) adds a DTO field `chess-app` must fill | Confirm in the SDD; (b) needs a test that pins the value, which exists (`games.rs`) |
| F-A07 `CreateFreshStore` | Not built | FIND-001 and FIND-008 give `Missing` false positives. If part C moves traces aside on a spurious `Missing`, it moves a live store | Decide with FIND-001 fixed first, and require the operation to take the exclusive lock |
| F-A08 `store_uuid` source | 16 bytes from two `RandomState` hashers (`open.rs:112-120`), version-4 bits set | Quality of randomness is not documented as guaranteed `[UNV]`. Per-thread hasher keys differ by a counter, so the two halves are related keys. Needs only to tell two stores apart | Accept as is; or read 16 bytes from `/dev/urandom` with std (no new dependency); or admit `getrandom` (DL2-4 says no) |

## 5. What I attacked and found sound `[RAN]` or `[OBS]`

- **Schema:** DDL equals SDD 3.5.2 (section 2). `NOT NULL` is present wherever `CHECK` guards a column that could otherwise be NULL (`preferences.json`, `uuid`, `kind`, `track`, `mode`, `status`). `json_valid` rejects `{bad`, `NaN` and JSON5 `{'a':1}` (`[RAN]` via `update_preferences`: all `InvariantViolated`). Foreign-key behaviour and CHECKs are exercised by `schema.rs` (13 tests).
- **UUID-only boundary:** no integer id in any public type; the textual scan in `boundary.rs` plus my reading of `api.rs`, `store.rs`, `error.rs` found none. `roster_indexes()` returns `Vec<u32>` of roster indexes, which are not row keys (D-46).
- **Injection and strings:** every value is a bound parameter; the only `format!` SQL (`player.rs`, `game.rs`) interpolates string constants. `x'); DROP TABLE player;--` registered as a name and all 9 players remained (`[RAN]`).
- **Open and probe:** the probe runs before any database write; any `sync_all` error is `DurabilityUnsupported` with errno (tested for 45, 25, 5, 19 and `/dev/null`); create, write and remove failures stay `Unavailable`; the scratch file is removed in all tested paths (`open.rs:77-103`). `SchemaTooNew` and `Corrupt` are decided from a read-only connection before any write pragma (tests compare bytes).
- **Backup (D-108):** stale temp removed first; temp name `chess.db.tmp-bak-v<N>` never matches `chess.db.bak-v*`; copy created `0600`, `VACUUM INTO`, read-only open, `quick_check`, `user_version` equal, file sync, directory sync, `rename`, directory sync. A failed `VACUUM INTO` and a failed `rename` leave the old copy byte-identical and remove the temp (tests `a_failed_replacement_...`, `a_failed_rename_...`, `ct_st_090_*`; mutants M14, M15, M19, M20, M23 killed). No `-wal`, `-shm` or temp files remain next to the copy after an upgrade (`[RAN]` listing). `chess.db`, `-wal`, `-shm` and `store.id` are all mode 0600 (`[RAN]`).
- **Migration:** a failed migration rolls back and keeps the backup; user_version is set in the same transaction; `from > latest` is `SchemaTooNew` before any write; equal is a no-op; a version change before the exclusive lock is `Busy`. M1, M2, M4 to M9, M12, M13 killed.
- **Store thread and pool:** a panic returns `Internal`, one reopen, then `Unavailable` (tested, M75 killed); a caller that stops waiting does not cancel the work (tested); a panicking reader returns its connection (tested); a missing `notify_one` hangs the pool (M77 caught only as a hang).
- **Unsafe, panics:** `#![forbid(unsafe_code)]` at `lib.rs:13`; no `allow(` attribute in `src/`; clippy denies `unwrap`, `expect`, `panic`, indexing and lossy casts outside tests, and passes. All `unwrap`, `expect` and `panic!` counted in `src/` are inside `#[cfg(test)]` modules.
- **Info leaks:** `Display` prints categories only. `Debug` of `DurabilityUnsupported` carries the path by design, and `tracing` calls log result codes and `io::ErrorKind` only, never a path or SQL.

## 6. Mutation table (hand mutation, one change at a time, full `cargo test -p chess-store --no-fail-fast` per mutant)

cargo-mutants is not installed. Method: a script applied one exact text replacement to the scratch copy, ran the suite, restored the file. K = killed (at least one test fails), S = survived, T = killed only by a hang (the run hit the timeout; there is no per-test timeout in the suite). Compile errors were redone with a different edit. M78 was a no-op text and is not counted.

| # | File | Mutation | Result |
|---|---|---|---|
| 1 | migrate.rs | `from > latest` to `>=` | K (5) |
| 2 | migrate.rs | skip the backup | K (10) |
| 3 | migrate.rs | version-0 table count `> 0` to `> 1` | **S** |
| 4 | migrate.rs | drop the version re-read check | K (1) |
| 5 | migrate.rs | ignore the guard result | K (1) |
| 6 | migrate.rs | run migrations `> from` to `>= from` | K (7) |
| 7 | migrate.rs | set `user_version` to `from` | K (14) |
| 8 | migrate.rs | drop `commit` | K (72) |
| 9 | migrate.rs | `backed_up` always false | K (7) |
| 10 | migrate.rs | `Exclusive` to `Deferred` | **S** |
| 11 | migrate.rs | negative `user_version` read as 0 | **S** |
| 12 | migrate.rs | `Migrator::with` contiguity `==` to `>=` | K (1) |
| 13 | migrate.rs | `with` accepts an empty list | K (1) |
| 14 | backup.rs | do not remove the stale temp | K (3) |
| 15 | backup.rs | keep the temp after a failed rename | K (1) |
| 16 | backup.rs | no directory sync after the rename | **S** |
| 17 | backup.rs | no file sync of the copy | **S** |
| 18 | backup.rs | version check `==` to `>=` | K (1) |
| 19 | backup.rs | keep the partial file on failure | K (2) |
| 20 | backup.rs | temp name equals the target name | K (5) |
| 21 | backup.rs | existing target no longer refused | K (1) |
| 22 | backup.rs | ignore `verify_copy` result | **S** |
| 23 | backup.rs | temp-name rule always "other" | K (4) |
| 24 | backup.rs | map any temp-write error to `CopyFailed` | K (2) |
| 25 | datadir.rs | file mode 0600 to 0644 | K (4) |
| 26 | datadir.rs | dir mode 0700 to 0755 | K (2) |
| 27 | datadir.rs | empty env value counts as set | K (1) |
| 28 | datadir.rs | trace test `starts_with` to `==` | K (1) |
| 29 | datadir.rs | journal subdirectory not inspected | K (1) |
| 30 | datadir.rs | no `sync_all` of `store.id` | **S** |
| 31 | datadir.rs | no directory sync after `store.id` | **S** |
| 32 | datadir.rs | `NotADirectory` kind changed | **S** |
| 33 | open.rs | probe ignores the `sync_all` error | K (2) |
| 34 | open.rs | `errno` always 0 | K (2) |
| 35 | open.rs | real probe passes a no-op sync | **S** |
| 36 | open.rs | scratch file not removed | K (6) |
| 37 | open.rs | `found > latest` to `>=` | K (5) |
| 38 | open.rs | skip `quick_check` | K (1) |
| 39 | open.rs | foreign-database table test `> 0` to `> 5` | K (1) |
| 40 | open.rs | skip the `Missing` test | K (2) |
| 41 | open.rs | `synchronous` FULL to NORMAL | **S** |
| 42 | open.rs | `fullfsync` ON to OFF | **S** |
| 43 | open.rs | `checkpoint_fullfsync` ON to OFF | **S** |
| 44 | open.rs | `foreign_keys` ON to OFF | **S** |
| 45 | open.rs | `busy_timeout` 5000 to 0 | K (1) |
| 46 | open.rs | never write `store.id` | K (3) |
| 47 | open.rs | `read_connections.max(1)` to unclamped | T |
| 48 | open.rs | no cleanup of a failed first creation | **S** |
| 49 | open.rs | no check that `journal_mode` became WAL | **S** |
| 50 | open.rs | interrupted creation not a first launch | K (1) |
| 51 | repo/player.rs | `id <> ?2` to `id = ?2` | K (2) |
| 52 | repo/player.rs | `register_player` name pre-check removed | K (2) |
| 53 | repo/player.rs | initial rating `k` 0 to 1 | **S** |
| 54 | repo/player.rs | roster index pre-check removed | K (3) |
| 55 | repo/player.rs | archive guard on wrong status | K (1) |
| 56 | repo/player.rs | `archived` filter inverted | K (1) |
| 57 | repo/player.rs | drop secondary sort of `roster_indexes` | **S** (equivalent: equal `u32` values) |
| 58 | repo/player.rs | `list_players` order reversed | K (2) |
| 59 | repo/player.rs | rename: no `NameTaken` | K (2) |
| 60 | repo/game.rs | `self_play` always 0 | K (2) |
| 61 | repo/game.rs | c-c slot holders none | K (3) |
| 62 | repo/game.rs | h-c kind check always true | K (1) |
| 63 | repo/game.rs | `started_at >= from` to `>` | K (1) |
| 64 | repo/game.rs | no-limit sentinel -1 to 0 | K (4) |
| 65 | repo/game.rs | idempotent `start_game` removed | K (1) |
| 66 | repo/game.rs | `list_games` order reversed | K (1) |
| 67 | repo/game.rs | `analysis_used` set to 0 | K (1) |
| 68 | repo/game.rs | half-null move clocks accepted | **S** (unreachable: `move` CHECK forbids it) |
| 69 | repo/game.rs | `rated` always 1 | K (1) |
| 70 | repo/game.rs | matchup/kind check removed | K (1) |
| 71 | repo/game.rs | self-play duplicate slot allowed | K (2) |
| 72 | repo/rating.rs | history order reversed | K (1) |
| 73 | repo/rating.rs | track filter removed | K (3) |
| 74 | repo/mod.rs | uuid lookup `OR 1` | K (23) |
| 75 | thread.rs | two restarts instead of one | K (1) |
| 76 | thread.rs | reopen after panic disabled | K (1) |
| 77 | readpool.rs | no `notify_one` | T |

Totals: 77 mutants, **58 killed (2 of them by hang only), 19 survived**. Of the survivors, M57 is equivalent and M68 is unreachable, so 17 are real gaps. Kill rate 58 / 77 = 75 %, 58 / 75 = 77 % excluding those two. By file: migrate 10 of 13 killed, backup 8 of 11, datadir 5 of 8, open 11 of 18, repo/* 21 of 24, thread and readpool 3 of 3 (sum 58 of 77).

The mutation pass is a sample chosen by me, not an exhaustive operator set, and it is not a mutation score in the cargo-mutants sense.

## 7. Conformance statement

Standard: none claimed. SEI CERT is applied by intent only in this project (CLAUDE.md, SWE-001); **no SEI CERT conformance is claimed here**. MISRA and AUTOSAR do not apply to Rust.

| Category | Content |
|---|---|
| Tool-checked | rustc with `warnings = deny`; clippy `all` plus `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation`, `cast_sign_loss` at deny, run with `--all-targets -D warnings`: clean; `rustfmt --check`: clean; `#![forbid(unsafe_code)]` enforced by the compiler; cargo-llvm-cov 0.9.1 line and region coverage |
| Manually reviewed | Every file under `src/` and `migrations/` read in full. Of the 8 test files (including `common/mod.rs`), `boundary.rs` read in full; `common/mod.rs` read in part; the other test files (`open.rs`, `schema.rs`, `players.rs`, `games.rs`, `model.rs`, `drift_guard.rs`) were reviewed by test name, mutation results and targeted reading, not line by line |
| Dynamic | 10 repeated test runs; ad hoc probe tests listed in section 3 (symlinks, URI path, fd exhaustion, concurrent first launch, text-in-integer values, name edge cases, file modes); hand mutation |
| Not checked | See section 8 |

Overall for the standards question: **NOT-DETERMINED** (no standard in force that a tool can certify; the coding standard of CLAUDE.md is met as far as clippy and the compiler check it).

## 8. Not checked, not run, assumed

- Two-process behaviour: all concurrency was threads in one process. Cross-process locking, `BUSY` across processes and CT-ST-004 as written ("two processes") were not run. FIND-001 is shown with threads; the code path is the same.
- Power loss, a volume where `F_FULLFSYNC` fails, APFS rename atomicity, Intel Macs: not testable here. The atomic-rename claim of D-108 is `[UNV on APFS through std]` as the SDD says.
- Failure injection (FaultPlan), kill-at-failpoint tests, CT-ST-085 end to end (no heartbeat row, no database, scratch removed, through `Store::open`) and CT-ST-090's kill-before-rename case: owned by D1, not available.
- `cargo deny`, `cargo audit`, `npm audit`, sanitizers, miri, Mermaid validation (no Mermaid in this review), `cargo-mutants`, benchmarks BM-ST-*.
- Branch and MC/DC coverage: not obtainable on stable.
- Read-side performance and `quick_check` start-up cost (ASM-ST-3).
- Part B and C behaviour, the `Exclusive` guard wiring, `CreateFreshStore`, any journal code.
- Assumed: the SDD (baselined 1.0 plus the dated notes of 2026-11-09), architecture section 7.1 and Addendum K, and the allocation document 5.1 are the intent. I did not read the full architecture.
- The snapshot's Cargo.toml has a trimmed member list and the chess-rules crate needed `third_party/`; I assumed this does not change chess-store behaviour.

## 9. Disposition and hand-off

| Finding | Severity | Status | Owner needed |
|---|---|---|---|
| FIND-001 | major | OPEN | software engineer; Brian to accept or schedule (data-destroying) |
| FIND-002 | major | OPEN | software engineer |
| FIND-003 | major | OPEN | software engineer |
| FIND-004 to FIND-011, FIND-013 | minor | OPEN | software engineer, or `ACCEPTED-RISK` by Brian |
| FIND-012, FIND-014, FIND-015 | info | OPEN, no action required | n/a |

Needs a human: Brian decides F-A04 to F-A08, and accepts or rejects the three Major findings as conditions. This review is a desk review by one agent; it is not an approval, and no gate was approved or changed.
