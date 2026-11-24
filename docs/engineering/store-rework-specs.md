# chess-store rework specifications (bead chess-a0r.3.50, baseline B6)

Author: Architect stance (system-architect agent), 2026-11-22. For the software engineer (implementation) and the systems engineer (section 5). Source of authority: Brian Prascher's decisions of 2026-11-13 to 2026-11-22 recorded on bead chess-a0r.3.50 (relayed; no verbatim quote). **Status: specification text, not reviewed by the Review Board stance, not approved.** G2 and G3 approvals are Brian's (GATE-108 to GATE-110); nothing here approves a gate. Tags: `[OBS]` read in the code or document, `[INF]` inferred, `[UNV]` not verified, `ESTIMATE` a number I chose.

Code facts below were read from the working tree on 2026-11-22 (`open.rs`, `datadir.rs`, `journal/reader.rs`, `repo/analysis.rs`, `store/d5.rs`, `api_analysis.rs`); I did not build or run anything `[OBS]` only.

Sections: 1 FIND-B-06 narrowed exception; 2 NF-4 open-path rule and `create_fresh`; 3 journal items CH-4, CH-5, CH-11, FIND-B-11; 4 D5-02 variation API; 5 text for the systems engineer; 6 not changed, risks, unverified.

---

## 1. FIND-B-06, narrowed: when may the opener roll back a hot journal?

### 1.1 The rule

The exception to "nothing is written for a refused open" (SDD 3.9.1 note 1; code `open_probe`, `DataDir::is_interrupted_first_creation`) applies **only when all of the following hold**. A to E are the old conditions (the probe result and the four directory conditions, as built `[OBS]`); F to I are new.

Checklist (every item must be true; the first false item keeps the error `Unavailable{ReadOnly}` and writes nothing):

| # | Condition | How it is tested | Status |
|---|---|---|---|
| A | The read-only probe returned `SQLITE_READONLY` (mapped `Unavailable{cause: ReadOnly}`) | the result of `read_only_probe` | old |
| B | `chess.db-journal` exists and has length greater than 0 | `fs::metadata` | old |
| C | `journal/` holds no content (a file, or a non-empty subdirectory; `.DS_Store` and `._*` ignored) | `journal_has_content` | old |
| D | `backups/` is empty (OS droppings ignored) | `dir_has_entries` | old |
| E | no entry named `chess.db.bak*` | directory scan | old |
| F (new, tightening) | no entry named `chess.db.tmp-bak-v*`, `chess.db.pre-import-*` or `chess.db.restoring`; `chess.db-wal` is absent or has length 0 | directory scan | architect's addition: these are the NF-4 data traces (section 2), so the two rules say the same thing |
| G (new, Brian's narrowing) | `chess.db` after a rollback **holds no players and no games** | the copy test in 1.2 | **ruling** |
| H (new) | `chess.db` and `chess.db-journal` are each at most `NF2_COPY_CAP` bytes | `fs::metadata` before the copy | architect's bound on the cost of G |
| I (new) | the copy test completed without any error | 1.2 | fail-safe |

F and H are mine and only narrow the rule further (fewer opens take the exception); they need no separate approval beyond Brian's narrowing, but he should see them (G1 inside the crate, listed in section 5).

Any failure of G, H or I returns the **same error as today** (`Unavailable{cause: ReadOnly}`), **before** the real `chess.db` is opened read-write, and so `chess.db` and `chess.db-journal` stay byte for byte as found.

### 1.2 How the opener answers G while the database is in the hot-journal state

**Chosen method: roll back a private copy.** Not the real file.

1. Create a private directory: `std::env::temp_dir()/chess-nf2-<pid>-<n>` with `std::fs::create_dir` (fails if the name exists; a pre-planted symlink is not followed because the last component must not exist), mode `0700` (`DirBuilder::mode`). Try up to 16 values of `<n>`. **Not under the data directory**: that would add a name there (the D-83 rule rates new names in the data directory as G2) and would be seen by the trace scan. On macOS `temp_dir()` is the per-user `/var/folders/.../T/` `[INF]`.
2. If H fails (either file over `NF2_COPY_CAP`), stop with the old error. `NF2_COPY_CAP = 4 MiB` is an `ESTIMATE`: a freshly created schema of about fifteen tables and their indexes should be well under 1 MiB `[INF]`. The engineer measures the size of an empty schema database and records it; set the cap to at least 16 times that and tell the architect the numbers (it is a constant, a two-way door).
3. Copy `chess.db` to `<tmp>/chess.db` and `chess.db-journal` to `<tmp>/chess.db-journal` (`fs::copy`, read-only access to the real files; the journal's name must be exactly the database name plus `-journal` so SQLite sees it as hot). Copy the journal **after** the database. The real files are only read.
4. Open `<tmp>/chess.db` with `SQLITE_OPEN_READ_WRITE | SQLITE_OPEN_NO_MUTEX` (no `CREATE`), path through `sqlite_path` (the `file:` URI rule, 3.9.1 note 6), `busy_timeout` set. SQLite rolls the copy back on its first read, as it would the real file. Then:
   - `tables_present(player)`, `tables_present(game)` from `sqlite_master` (a table that does not exist counts as 0 rows);
   - `SELECT count(*) FROM player` and `SELECT count(*) FROM game` for the tables that exist.
   - G is true only when both counts are 0 (or the tables are absent).
   - Any SQLite error on the copy (not a database, corrupt, locked, I/O) is I = false.
5. Close the copy connection, then delete `<tmp>` (the copy, its `-journal`, `-wal`, `-shm` if any, and the directory). Deleting these is allowed: they are this call's own scratch files, not the user's data. Removal errors are logged and ignored. A crash leaves the directory in the system temp area (the OS cleans it); the next open does not look at it.
6. Only if G and I are true: open the **real** file read-write once (as built), run `table_count`, drop the connection, and run `read_only_probe` again (as built). If the second probe fails the error is returned.

Why this method and not another (options considered):

| Option | Verdict |
|---|---|
| Copy, roll back the copy, count there (chosen) | The real file is never written when the answer is "has data". The count is taken on the same state SQLite would produce. Cost: a bounded copy, only in a rare state. |
| Open the real file with `immutable=1` or `mode=ro` and count | Rejected: a read-only or immutable connection does not apply the hot journal, so it reads the half-written pre-rollback pages. The answer would be wrong or an error. |
| Roll back the real file first, count, and "undo" if there is data | Rejected: this is exactly what the reviewer found (R06); a rollback cannot be undone and it changes `chess.db` bytes. |
| Parse the journal ourselves | Rejected: reimplements SQLite's recovery. |
| Refuse the exception altogether (always keep the error) | Safe, but then an interrupted first creation that left a hot journal could never complete, and 3.9.1 note 1 promises that it does. Brian ruled option A, narrowed. |

Residual points `[UNV]`: (a) a journal that names a super-journal (multi-file transaction) is ignored by SQLite when the super-journal file is absent; the copy has no such file, so the copy would not roll back, and the counts would be of the un-rolled-back file. The first-creation transaction uses one file and no `ATTACH`, so this cannot arise from this crate's writer `[INF]`; if it does arise in a foreign file, the count reads whatever the copy holds and G is judged on that. (b) Between the copy and the real rollback another process could write; a writer holding a lock makes the journal not hot, so the window is a live race that needs a second process on a store in this state. Accepted and listed as a risk in section 6.

### 1.3 Test cases (all in `crates/chess-store/tests/`, one file `hardening.rs` or a new `nf2_narrowed.rs`; names are proposals)

Fixtures: reuse the part A fixture that makes a hot `chess.db-journal` (the reviewer's `rev_nf2` and `hardening.rs:86-118` `[OBS]` by reference; I did not read them). To build a hot journal over a store **with data**: open a real store, register a player (and in a second case start and finalize a game), close it; copy `chess.db`; run a child process or a raw SQLite connection in rollback-journal mode, start a write, and `kill -9` after the journal is synced, or take a journal captured that way and place it with the data file in a fresh directory. The engineer chooses the method and says which in the evidence; the file pair must make the read-only probe fail with `SQLITE_READONLY` (assert that first, so the test cannot pass vacuously).

| ID | Case | Expected |
|---|---|---|
| NF2-1 | Interrupted first creation: empty schemaless `chess.db` plus hot journal, no other trace | Open succeeds, `first_launch = true`. The existing positive test must still pass. |
| NF2-2 | **The real v1 store case (R06):** a store with one Player (Ann), a hot `chess.db-journal`, no `journal/` content, empty `backups/`, no `chess.db.bak*` | Open returns `Err(Unavailable{ReadOnly})`. `chess.db` and `chess.db-journal` byte-identical to before (hash both); the directory listing is the same except for zero-length `-wal` and `-shm` that the read-only probe may create (F-A09); no `store.id` change; the temp directory `chess-nf2-*` is gone. |
| NF2-3 | As NF2-2 with the Player replaced by a finished game (a Player and a game) | Same. |
| NF2-4 | A store with a schema, version 1, and **no** players and no games, plus a hot journal | Exception applies (rolled back, opens as an existing empty store, `first_launch = false`). Pins that G is "no players and no games", not "no schema". |
| NF2-5 | Each of the conditions C, D, E and F alone (journal content; non-empty `backups/`; `chess.db.bak-v1`; `chess.db.tmp-bak-v1-1`; `chess.db.pre-import-x`; `chess.db.restoring`; non-empty `chess.db-wal`) with an otherwise empty database and a hot journal | Error `Unavailable{ReadOnly}`; `chess.db` and journal byte-identical. One sub-case per marker. |
| NF2-6 | H: `chess.db` larger than `NF2_COPY_CAP` (a sparse padded file) with a hot journal | Error as NF2-2; the file is not copied (assert no `chess-nf2-*` directory is created). |
| NF2-7 | I: the copy cannot be made (inject through a failpoint `open.nf2_copy` returning an I/O error, or an unwritable `TMPDIR`) | Error `Unavailable{ReadOnly}` (the original error, not the I/O error); real files untouched. |
| NF2-8 | I: the copy rolls back to a corrupt file (garbage journal) | Error as NF2-2; real files untouched. |
| NF2-9 | Zero-length `chess.db-journal` (condition B false) | Not hot for SQLite; open proceeds as an interrupted creation or an ordinary open (existing test). |
| NF2-10 | No leak: after NF2-1, NF2-4 and NF2-2, no `chess-nf2-*` entry remains in `temp_dir()` for this process id | Pass. |
| NF2-11 | Mutation check (engineer, reported in the evidence): removing the G test must fail NF2-2 and NF2-3; removing the H test must fail NF2-6. | Killed. |

---

## 2. NF-4: what `Store::open` does with a `chess.db` that is empty or has no schema

### 2.1 Ruling to implement

An **empty (zero-length) or schemaless `chess.db`** that sits next to **traces of an earlier store** must **not** open silently as a new store. `Store::open` refuses with an error, and the front-end offers `Store::create_fresh` (D-119). FIND-007 is unchanged (a missing `store.id` next to a `chess.db` that has a schema is not recreated; `Store::journal()` stays `Corrupt` for that state). The 43 s first-launch `Busy` retry stays. An empty `chess.db` plus `store.id` only (no journals, backups or backup files) is still a first launch (FIND-002).

### 2.2 Terms

**Database state** (what SQLite sees after the read-only probe and, if needed, the section 1 step):

- **absent**: no file `chess.db`.
- **empty**: the file exists and the probe reads `user_version = 0` **and** zero tables. This includes a zero-length file. (Decide by what SQLite reads, not by file length alone: a zero-length `chess.db` with a non-empty `chess.db-wal` can hold a committed schema that SQLite recovers from the WAL; that is state "schema".)
- **schema**: `user_version > 0` or at least one table. Normal open from here (`SchemaTooNew`, `Corrupt`, upgrade, as built). Tables with `user_version = 0` stay `Corrupt` (as built, 3.9.1 note 1).
- **unreadable**: the probe returns an error (`ReadOnly` from a hot journal that section 1 does not clear, `NOTADB`, `Busy`, ...). Unchanged: that error is returned, nothing is written.

**Data traces** (a set that makes an empty database suspicious). Any one of these is a data trace:

1. `journal/` holds content: a file or a non-empty subdirectory, `done/`, `damaged/`, `unrestored/` included (`journal_has_content`, as built).
2. `backups/` is non-empty.
3. a `chess.db.bak*` entry (this includes `chess.db.bak-v<N>`).
4. a `chess.db.tmp-bak-v*`, `chess.db.pre-import-*` or `chess.db.restoring` entry.
5. `chess.db-wal` with length greater than 0.
6. `chess.db-journal` with length greater than 0 (when state is not already cleared by section 1: after the section 1 rollback the journal is gone).

**Not a trace and not a data trace** ("benign"): `.DS_Store`, `._*`, `logs/`, `config.toml`, `missing-*` directories, empty `journal/`, `journal/done/` and `backups/`, zero-length `chess.db-wal`, `-shm`, `-journal`, `.durability-probe-*`, `.store.id.tmp-*`, the opener's own empty `chess.db`.

**`store.id`**: absent, valid, or unreadable (present but the content does not parse).

### 2.3 Decision table

Rows are evaluated top to bottom; the first row that matches decides. "Refuse" means `Err(StoreError::Missing)` with **nothing written or removed**.

| # | `chess.db` | Data traces | `store.id` | Result | Notes |
|---|---|---|---|---|---|
| 1 | absent | none | absent | **first launch** | As built. Create `chess.db` (create-new), then `store.id`, then schema. |
| 2 | absent | none | present (any) | refuse `Missing` | As built: `store.id` exists but `chess.db` was lost. |
| 3 | absent | any, or any other `chess.db*` sibling | any | refuse `Missing` | As built (`has_traces_of_a_store`). |
| 4 | **empty** | none | absent | **first launch** | The opener created the file itself (race loser carries on), or a killed creation left a bare file. `store.id` is written, then the schema. |
| 5 | **empty** | none | valid | **interrupted-creation completion**: first launch with the existing `store.id` kept | FIND-002. Reports `first_launch = true` (3.9.1 note 4: true for the opener whose upgrade creates the schema). |
| 6 | **empty** | none | unreadable | `Corrupt` | As built `[UNV]`: the engineer states the variant that `read_store_id` returns and keeps it. Nothing written. |
| 7 | **empty** | **one or more** | any (also unreadable) | **refuse `Missing`** (NF-4) | New. The data traces are checked **before** `store.id` is read. Nothing written: no `store.id`, no layout directories, no schema. Log at `warn` (reason: `empty_database_beside_data_traces`, without paths of user data). |
| 8 | schema | any | any | open as built | Includes FIND-007 (`store.id` absent: not recreated) and `SchemaTooNew`, `Corrupt`, upgrade. NF-4 does not apply to a database that has a schema. |
| 9 | unreadable (`ReadOnly` hot journal) | | | section 1 decides: exception applies and then the row is re-evaluated as 4, 5, 7 or 8 after the rollback; otherwise `Unavailable{ReadOnly}` | A hot journal is a **data trace** by item 6 above unless section 1 cleared it, so a hot journal beside `backups/` content is `ReadOnly`, not `Missing`: the database state is unknown there, not empty. |
| 10 | other probe error (`NOTADB`, `Busy`, I/O) | | | that error | As built. |

How rows 4, 5 and 7 are told apart in code (placement in `open_blocking`): after `read_version`, `quick_check` and the `fresh` decision (`found == 0` and no tables), and **before** `finish_open`:

1. If `fresh`: compute `data_traces = data_dir.has_data_traces()?` (a new function; the part of `has_traces_of_a_store` that is not `store.id`, per 2.2).
2. If `data_traces` is true: **re-run the probe reads once** (`read_version`, `table_count`) before the verdict. If the schema has appeared meanwhile (a concurrent first launch finished and a game's journal now exists, the same window as FIND-R05 and the `Missing` re-test), set `fresh = false` and continue as row 8. If the reread says `Busy`, return `Busy`. Otherwise return `Missing`.
3. Otherwise (no data traces): continue as built (rows 4 and 5).

Race safety (FIND-001 and FIND-002 stay as they are):

- No path in this rule deletes or moves anything. The refusal is a read-only decision made before the first write of `finish_open`.
- The refusal is stable and repeatable: a second open gives the same `Missing` and still writes nothing.
- The opener that created `chess.db` itself in this call (the `create_new_0600` success) passed the earlier `Missing` check, so no data trace existed then; the check above is vacuous for it except in a race, where step 2's re-read protects it.
- A first-launch loser (`AlreadyExists`) carries on with the winner's file; if the winner has already finished, the schema is found and the row is 8.
- Pre-existing scratch removal (`remove_stale_scratch`) is unchanged and removes only dead-process scratch names, which are not traces.
- Side effect of the read-only probe: it may leave zero-length `chess.db-wal` and `chess.db-shm` for a WAL database (F-A09); these are benign (2.2). Tests compare directory listings **ignoring zero-length `-wal` and `-shm`**.

### 2.4 The error variant

**Reuse `StoreError::Missing`.** It fits: its meaning is "the store this directory should hold is not there; the user chooses a fresh store or an import" (3.9.1 diagram, `chess-app` maps it to `StoreMissing`, protocol §5.5). No new variant, no new protocol code, no field. The reason (absent file or empty file) is logged only. If Brian wants the front-end to show different text for "database emptied", that needs a field and is a G2 change; not proposed.

### 2.5 What `create_fresh` does in the NF-4 case (change to D-119 item 1)

D-119 item 1 says `create_fresh` refuses with `StoreExists` whenever `chess.db` exists, an empty one included, on the reasoning that an empty `chess.db` next to traces would open as a first launch (D-124 (6)). NF-4 reverses that reasoning, so the refusal in item 1 would make the offered way out unusable. **Amend item 1 only** (items 2 to 10 unchanged):

> 1'. `create_fresh` refuses with `StoreError::StoreExists` when `chess.db` exists **and is not empty** (state "schema", or "unreadable"), nothing is moved. When `chess.db` is **empty** (2.2: zero-length, or no tables and `user_version` 0), it is **left in place** (not moved, not deleted) and the call goes on with items 2 to 8. After the traces are set aside, item 8's first-launch path finds that empty `chess.db` and completes it (rows 4 and 5 of the table: `store.id` written, schema created, `first_launch = true`).

Details for the engineer:

- The test "is `chess.db` empty" is the same function as `open`'s (one shared classifier, `fn database_state(&DataDir) -> DatabaseState {Absent, Empty, Schema, Unreadable(StoreError)}`), so the two cannot disagree. It uses a read-only connection (a read-only open of a database that has no WAL). `Unreadable` (including a hot journal) is treated as "not empty": `StoreExists`.
- Why leave the empty file in place and not move it: it holds no data; moving it would race with a concurrent opener that already holds it; the first-launch path already handles an existing empty `chess.db`.
- The `create_fresh` item 3 (`Busy` if a journal is locked) and item 2 (volume probe first) still run before anything is moved.
- `SetAside.moved` does not list `chess.db` (it is not moved).
- Case "nothing to set aside": `chess.db` empty and no traces to move means `open` would have been a first launch; as built `create_fresh` behaviour for an empty set of moves stays as built `[UNV]` (I did not read `create_fresh`; if it creates `missing-<date>/` even when nothing moves, leave that).
- **Restore and import (D-17) from the NF-4 state** are `chess-app` and `chess-store` operations that may assume `chess.db` is absent or has a schema. They must accept an empty `chess.db` as replaceable. `[UNV]` how as-built `restore_backup` and `import` treat an existing empty `chess.db`; the engineer reports it; the app SDD note (RESTORE and import) lists this for bead chess-a0r.3.13.

### 2.6 Test cases (names proposed; all on a temp data directory)

| ID | Setup | Expected |
|---|---|---|
| NF4-1 | Empty directory | First launch (row 1); unchanged test. |
| NF4-2 | Zero-length `chess.db` only | Row 4: first launch, `store.id` created. |
| NF4-3 | Zero-length `chess.db` + valid `store.id`, empty layout (`journal/`, `journal/done/`, `backups/` empty) | Row 5: opens, keeps the same `store.id` bytes, `first_launch = true`. (FIND-002 regression test.) |
| NF4-4 | Zero-length `chess.db` + valid `store.id` + one `journal/<uuid>.jnl` | **Refuse `Missing`**; nothing written (`chess.db` still zero length; no new files except benign; `store.id` bytes unchanged; no schema). |
| NF4-5 | Same with a file in `journal/done/` | Refuse `Missing`. |
| NF4-6 | Same with a non-empty `backups/` | Refuse `Missing`. |
| NF4-7 | Same with `chess.db.bak-v1` | Refuse `Missing`. One sub-case each for `chess.db.tmp-bak-v1-7`, `chess.db.pre-import-x`, `chess.db.restoring`, and a non-empty `chess.db-wal` (SQLite sees no schema in it). |
| NF4-8 | Zero-length `chess.db`, **no** `store.id`, journals present | Refuse `Missing` (data traces are checked before `store.id`). |
| NF4-9 | Schemaless non-zero `chess.db` (a SQLite file with no tables, `user_version` 0) + `store.id` + a backup | Refuse `Missing` (same state as zero length). |
| NF4-10 | `chess.db` with tables and `user_version` 0 | `Corrupt`, as built. |
| NF4-11 | `chess.db` with the v1 schema, journals present, **`store.id` absent** | Opens (FIND-007); `store.id` is **not** created; `Store::journal()` returns `Corrupt`. Unchanged test. |
| NF4-12 | `.DS_Store` and `._x` in `journal/` and `backups/` beside an empty `chess.db` and `store.id` | Row 5, not refused. |
| NF4-13 | Zero-length `chess.db-wal` and `-shm` beside an empty `chess.db` and `store.id` | Row 5. |
| NF4-14 | After a refusal (NF4-4), call `open` again | The same `Missing`; directory unchanged (compare a listing and hashes). |
| NF4-15 | After a refusal (NF4-4), `create_fresh` | Success: `missing-<date>/` holds `journal/`, `store.id`, the others; `chess.db` stays; new `store.id`, schema, `first_launch = true`; nothing deleted (compare the set of file contents before and after, as CT-ST-016). |
| NF4-16 | `create_fresh` when `chess.db` has a schema | `StoreExists`, nothing moved. When `chess.db` has a hot journal (unreadable) | `StoreExists`, nothing moved. |
| NF4-17 | Race: N threads or processes open a fresh directory (existing FIND-001/002 loops, 200 rounds of 4 threads, 60 rounds of 3 processes) | Still: no lost writes, no `Missing`, exactly one `first_launch = true`. |
| NF4-18 | Race: one opener finishes a first launch and a game's journal appears while a second opener sits between its probe read and the trace scan (use the existing `BETWEEN_READS` seam) | The second opener does **not** answer `Missing`: the re-read finds the schema (row 8). |
| NF4-19 | Mutation check (reported): removing the data-trace test makes NF4-4 to NF4-8 fail; removing the re-read makes NF4-18 fail. | Killed. |

The 43 s `Busy` figure (8 attempts, `busy_timeout` per attempt) is accepted and unchanged; no work for the engineer.

---

## 3. Journal format items: CH-4, CH-5, CH-11, FIND-B-11

Note `[OBS]`, 2026-11-22: the working tree already has code for CH-4 (`MAX_COUNT` in `format.rs`, `validate()` doc, test `ch4_...`) and CH-5 (`DamageKind::UndecodableTail { at: usize }` in `reader.rs`). This section states the acceptance details so the reviewer and I check the same thing; where the code already matches I say so as `[OBS]`, and I did not run the tests. Where this text and the code differ, the engineer says which is right and I adjust the text.

### 3.1 CH-4: no `u64` field above `i64::MAX`

- **Fields in scope:** every `u64` that reaches a SQLite INTEGER column or a signed comparison: `Move.think_ms`, `Move.white_ms`, `Move.black_ms` (Some), `Clock.white_ms`, `Clock.black_ms` (Some), `GameEnd.white_time_used_ms`, `GameEnd.black_time_used_ms`, `GameEnd.total_active_ms`, and the record `seq`. The engineer lists any other `u64` found in `format.rs` and includes it.
- **Where:** `Payload::validate()` (so **encode** refuses) and decode (which calls the same check). The bound is `i64::MAX` inclusive: `i64::MAX` is accepted, `i64::MAX + 1` is refused.
- **Result:** at encode, the existing validation error of `encode_record` (a `FormatError`; the engineer keeps the as-built name). At decode, the frame is a **refused** frame (a bad record), which is what CH-5 then classifies.
- **Tests:** (a) each field at `i64::MAX` encodes and decodes; at `i64::MAX + 1` and `u64::MAX` is refused at encode and at decode; use the literal `9223372036854775807`, not the constant, so the constant cannot hide a mutation (review M01). (b) Property: for any payload built from arbitrary `u64` and `Option<u64>`, `encode_record` is `Ok` **iff** `decode_frame(encode...)` is `Ok` ("the writer never writes what the reader refuses"; the 100k-case property of the review, now over the full `u64` range). (c) A hand-built CRC-valid frame with `think_ms = i64::MAX + 1` at the tail gives `UndecodableTail` (3.2), in the middle gives `Middle`.

### 3.2 CH-5 B: a CRC-valid frame that the decoder refuses is damage, never a silent drop

- **Definition of "refused frame":** a frame at `pos` for which `read_frame` is `Ok` (sync, length and CRC are right) and that is still not accepted: payload refused (extra or missing field, wrong type, `validate()` failure such as CH-4 or a half-null clock pair), kind code invalid or unknown, or `seq != expected`. This is the generalisation the reviewer asked for (R01: bad payload, kind or seq).
- **What `scan_bytes` returns** when a refused frame is found at `pos` and `look_ahead(bytes, pos, last_good)` finds **no** later intact frame:
  - `records` = the intact run `1..=last_good`; `intact_end = pos` (the start of the refused frame);
  - `torn_tail_dropped = false`;
  - `damaged = Some(DamagedInfo{ kind: DamageKind::UndecodableTail{ at: pos } })` (`[OBS]` the variant name and field `at` are in the tree; richer fields such as the refused `seq` or a reason are optional and go to the log, not to the type, unless the engineer needs them).
  - When `look_ahead` **does** find a later intact frame the result is `Middle{at, resumes_at}` as today.
  - When the bad frame is **not** CRC-valid (bad sync, short length, wrong CRC) and nothing intact follows, it is a torn tail, `torn_tail_dropped = true`, `damaged = None`, silent (unchanged: this is the genuine never-acknowledged case).
- **Consequence for the callers (D3, D4).** `ReconcileInput.scan_damaged = scan.damaged.is_some()` is unchanged (`[OBS]`, `reader.rs` test code) and therefore true for `UndecodableTail`; `plan.damaged` is true; the existing D-66 damage path runs: the journal is moved aside to `journal/damaged/<uuid>.<k>.jnl` (nothing deleted, the acknowledged record's bytes are kept there), the journal is rebased from the intact run plus the store's rows, and the damage is reported to `chess-app` through the same channel as a damaged middle (3.11.2, 3.11.3). D4's duty (recovery orchestration): treat `UndecodableTail` exactly as `Middle` for the repair and the report; the report text must not claim "torn" (the message names "a record this build cannot read", `[UNV]` the final wording belongs to `chess-app`, bead .3.13).
- **Interaction with D-66 (`db_seq > last_intact_seq` is damage, `DamageKind::StoreAhead`).** Precedence: the scan's `UndecodableTail` or `Middle` wins; the caller sets `StoreAhead` **only when `scan.damaged` is `None`**. Both cases reach the same repair. The cases:

  | `db_seq` | Scan | Today (before CH-5 B) | Required |
  |---|---|---|---|
  | `== last_intact` (store has not applied the refused record) | refused CRC-valid tail | `torn_tail_dropped`, `plan.damaged = false`, record invisible (R01) | `damaged = true` via `UndecodableTail`; repaired and reported |
  | `== last_intact + 1` (store applied it) | refused CRC-valid tail | `plan.damaged = true` through D-66 | `UndecodableTail` (more specific than `StoreAhead`); `damaged = true` |
  | any | genuine torn tail (CRC wrong) | silent drop if `db_seq <= last_intact`; `StoreAhead` if above | unchanged |

  `[OBS]` the engineer's test `ch5_the_tail_damage_reaches_reconcile_for_db_seq_3_and_4` covers rows 1 and 2 plus the no-regression row; keep it.
- **What is lost, honestly.** When the store has not applied the refused record, the rebase cannot rebuild it (the rebase takes the intact run plus the store's rows, 3.11.3). The record is **not destroyed**: its bytes are in `journal/damaged/`. It is reported, so the user or a later tool can recover it. This is the CH-5 B bargain.
- **One new false positive to accept `[UNV]`:** a CRC-valid frame with `seq` below `expected` (a duplicate) at the tail now reports damage instead of dropping silently. This build's writer does not produce one (reviewer: 100k-case property and kill rounds `[OBS review]`); if the engineer finds a path that does, tell me.
- **Tests:** (a) one test per refusal cause as a CRC-valid last frame: extra field **with tolerant decode (3.4) is not refused**, so use a missing required field, a wrong type, `think_ms` above `i64::MAX` (CH-4), unknown kind (15), kind 0, `seq` too high, `seq` repeated; each: `records.len() == 3`, `intact_end == base.len()`, `!torn_tail_dropped`, `damaged == UndecodableTail{at}`. (b) The same frame **followed by** an intact higher-`seq` frame gives `Middle`. (c) A flipped CRC byte gives torn tail, silent. (d) The two reconcile cases above at `db_seq` = 3 and 4. (e) End to end, one test at `chess-store` level: write a journal whose last frame is refused, run the recovery, assert: the journal file is in `journal/damaged/`, a fresh journal exists, the report names damage, no record of the intact run is lost.

### 3.3 CH-11 B: the rebase omits the closing `Clock` when the last record is terminal

- **Rule.** After the rebase has laid down the copied records of `R` and any synthesized `Move` records, look at the **last record of the rebased file**. If its kind is `GameEnd` (12), `Abort` (13) or `Voided` (14), **do not append the closing `Clock`**. Otherwise append it as today (reconciled clocks, `running_side` null, `wall_ms` of the previous record).
- **Why:** replay accepts a `Clock` after a terminal record (reviewer: `game.rs:640-645`), so it is hygiene (a rebased file that ends with its terminal record, which is what D-94 and the "done journal" rule 3.11.8 expect), not correctness.
- **Tests:** (a) `R` ends with `GameEnd`: the rebased file's records are `R` byte for byte, no `Clock`; same for `Abort`, `Voided`. (b) `R` ends with a `Move` (or `Pause`): the `Clock` is appended as before. (c) Synthesized moves after a non-terminal `R`: the `Clock` is last. (d) Replaying the rebased file gives the same `Game` state in all cases; (e) rebase of an already rebased file is idempotent (second rebase equals the first, bytes identical). (f) The 3.11.8 "done journal" check accepts a rebased voided game.

### 3.4 FIND-B-11 B: tolerant decode, verbatim rebase, version rule

- **Decode ignores unknown fields.** Payload structs do not use `deny_unknown_fields` (`[OBS]` the module doc line 9 already says so); required fields missing, wrong types, bad values and unknown kind codes are still refused.
- **Encode never writes a field the table does not list** (UT-ST-070 still compares the emitted field names with the table).
- **The rebase keeps record bytes verbatim.** The rebase copies the frames of `R` as bytes (the `intact_bytes` input of `build_rebased`, `[OBS review R15]`), never decodes and re-encodes them, so a field this build does not know survives. Test: a record with an extra field, rebase, assert the frame bytes in the new file equal the old ones (CRC included).
- **`journal_version` rule (the sentence for the SDD).** An **additive optional field** whose absence leaves every existing field's meaning unchanged needs **no** version bump (an older build ignores it). **Any change of meaning** bumps `journal_version`: a field whose meaning, unit or range changes, a field removed or made required, a new record kind, a changed order or checksum rule. A reader that sees `header.version > JOURNAL_VERSION` returns `VersionMismatch` (unchanged) and does not scan. `journal_version` **stays 1 now** (nothing is released).
- **Tests:** (a) for each of the 14 kinds, a payload with one extra field decodes and equals the same payload without it; (b) a payload with a missing required field is refused (still); (c) the rebase test above; (d) with CH-5 B: an extra-field record is **not** a refused frame (so no false damage).
- **Cost to record:** an older build that ignores a field that matters would apply a record wrongly; that is why a change of meaning must bump the version. The bump rule is a process rule, not enforced by code; add a one-line check to the format review checklist (section 5 item (g)).

---

## 4. D5-02 B: the variation API

Decision: a saved variation is identified by the triple `(game, from_ply, pgn)`; `save_variation` is idempotent on that key; `VariationRef` is dropped. `[OBS]` current code: `save_variation(game, from_ply, pgn) -> VariationRecord` with `id: VariationRef`, `delete_variation(VariationRef) -> bool`, `list_variations(game) -> Vec<VariationRecord>` (`store/d5.rs`, `repo/analysis.rs`, `api_analysis.rs`).

### 4.1 Signatures (on `Store`, async, as built style)

```rust
pub struct VariationRecord {            // VariationRef field removed
    pub game: GameId,
    pub from_ply: u32,
    pub pgn: String,
    pub created_at: WallMs,
}

/// Idempotent on (game, from_ply, pgn). Returns the stored record: the existing one (its
/// original created_at, no row added, nothing written) or the new one.
pub async fn save_variation(&self, game: GameId, from_ply: u32, pgn: String)
    -> Result<VariationRecord, StoreError>;

/// Deletes the row with this key. Ok(true) when a row was deleted, Ok(false) when the game
/// exists and no such row exists (the call is idempotent, like lease_delete).
pub async fn delete_variation(&self, game: GameId, from_ply: u32, pgn: String)
    -> Result<bool, StoreError>;

/// Unchanged: by from_ply, then creation order (row id); NotFound for an unknown game.
pub async fn list_variations(&self, game: GameId)
    -> Result<Vec<VariationRecord>, StoreError>;
```

`VariationRef` and its re-exports are removed from the public API and from `lib.rs`.

### 4.2 Behaviour

- **Key equality:** `game` by id; `from_ply` as integers; `pgn` by **exact byte equality of the UTF-8 text** (no trimming, case folding or normalization in the store). Two different strings for the same line are two variations. `chess-app` must produce one canonical text per line (section 5 and the app SDD note).
- **save, in one `BEGIN IMMEDIATE` transaction:** (1) resolve the game (`NotFound` if unknown); (2) `SELECT ... FROM variation WHERE game_id = ? AND from_ply = ? AND pgn = ?`; if a row exists, commit nothing and return it (**the existing-row check comes before the anchor check**, so a repeat of an already stored key never fails); (3) else check the anchor (`from_ply >= count of stored MOVE rows` gives `VariationAnchor`, as built); (4) insert with `created_at` from the injected clock; (5) the failpoint `variation.before_commit` as built; commit. Because the check and the insert are in one immediate transaction, two processes saving the same key give one row.
- **No DDL change.** No UNIQUE index: an index on a `pgn` text column is large, and the transaction already gives the guarantee. Adding a UNIQUE constraint (or a `uuid` column) is a stored-format change (G3, one-way at release) and is **not** part of this decision; I mention it only so the reviewer does not read the missing index as an omission.
- **delete:** `NotFound` for an unknown game; otherwise delete the matching row inside `BEGIN IMMEDIATE`; the failpoint `variation.before_commit`; return whether a row was deleted.
- **Empty `pgn`:** refuse with the existing validation error the crate uses for an empty or over-long text (`InvariantViolated`, `[UNV]` the engineer picks the as-built one and states it). Length limit: none new (4096 code points is a `chess-app` guard).
- **Errors:** `NotFound{entity: Game, id}`, `VariationAnchor`, `InvariantViolated` (empty text), `Busy`, `Unavailable`, `Corrupt`, `Internal` as the other write operations. No new variant.
- **Order of `list_variations`** unchanged; two variations with the same `(from_ply, pgn)` cannot exist, so the order is total.
- **Hidden and archived:** a variation is hidden with its game (PLY-7); no Player column; unchanged.

### 4.3 Tests

| ID | Case | Expected |
|---|---|---|
| VAR-1 | Save a key twice | The second call returns the **same record** (same `created_at`, even with the clock advanced); `list_variations` has one row; the row count is 1 in the table. |
| VAR-2 | Save the same `pgn` at two different `from_ply`, and two different `pgn` at one `from_ply` | Three rows, each its own key. |
| VAR-3 | Same text, two games | Two rows. |
| VAR-4 | Save at `from_ply` at or beyond the stored ply count | `VariationAnchor`; nothing written. |
| VAR-5 | Repeat of a stored key after the game's stored plies shrank (not reachable except by direct SQL; a unit test with a deleted `MOVE` row) | Returns the existing row (existing-row check first). |
| VAR-6 | Unknown game: save, delete, list | `NotFound` each. |
| VAR-7 | Delete an existing key; delete it again | `Ok(true)` then `Ok(false)`; the list is empty. |
| VAR-8 | Delete a key whose text differs by one character | `Ok(false)`; the stored row stays. |
| VAR-9 | Empty text | the error named in 4.2. |
| VAR-10 | Two processes (or threads with two `Store` handles) save the same key at once, 200 rounds | one row per key; every call returns `Ok` with the same `created_at`. |
| VAR-11 | `variation.before_commit` failpoint on save and on delete | the error surfaces; no row added or removed. |
| VAR-12 | Compile check: `VariationRef` is gone from the public API (a doc test or an API listing diff). | |

### 4.4 `StatsGameRow` (D5-01 A), as built

`[OBS]` the types `StatsGameRow` and `StatsSide` exist in `api_analysis.rs`, with the reason in the module doc; `stats_rows` and `head_to_head_rows` return them (`store/d5.rs`). **No code change is required.** The only open work is on the `chess-app` side (mapping to `chess_domain::StatsRow`, no new edge): see the app SDD note. A test in `chess-app` (bead .3.13) maps every `StatsGameRow` field and checks that a `StatsSide` maps to the domain colour without a conversion that loses a case.

### 4.5 D2 naming and ordering (accepted as built; nothing to change, two pins to add)

- The stored abort text is `aborted` (D2-04). Add a pin test if none exists: an aborted game's stored `termination` text is exactly `aborted`.
- `VoidSpec` field is named `game` (not `game_id`); failpoints are `abort.before_commit`, `void.before_commit` and `reset.before_commit`. Pin tests: each failpoint fires before commit and leaves the store unchanged (the `finalize.before_commit` pattern).
- **Rating row order (D2-02).** The store returns `RATING_HISTORY` rows **ordered by `id`** (insertion order); `chess-domain` orders by `(at, id)`. The two agree whenever `at` is non-decreasing in insertion order. Pin test: insert rating rows whose `at` goes backwards (a clock step) and assert that the store returns them in `id` order and that the domain function re-orders them by `(at, id)`. The test lives where both crates are visible (`chess-app` integration or the store test with `chess-domain` as a dev-dependency; the engineer picks the existing one).

---

## 5. Text the systems engineer must enter

The systems engineer owns `docs/requirements.md` (Decision Log, verification cases) and `RELEASE_NOTES.md`; I did not edit them. Suggested wording is plain; numbers are mine and may be shifted if the log has moved. Decisions are Brian Prascher's, bead chess-a0r.3.50, 2026-11-13 to 2026-11-22, relayed (no verbatim quote).

| Item | Text for a Decision Log (CR) entry | Architecture record | Gate |
|---|---|---|---|
| (a) FIND-B-06 narrowed | "Hot-journal rollback exception limited to a database that holds no players and no games (plus the earlier conditions); decided on a copy of the database." | D-125 | G3 (stored files) / G2 |
| (b) NF-4 | "An empty or schemaless `chess.db` beside journals, backups or backup files is refused with `Missing` and `create_fresh` is offered; empty plus `store.id` only is still a first launch; FIND-007 unchanged; 43 s first-launch Busy retry accepted." Supersedes D-124 (6). | D-126 (supersedes D-124 item 6) | G2 |
| (c) `create_fresh` amended | "`create_fresh` leaves an empty `chess.db` in place and goes on; it refuses `StoreExists` only for a database with a schema or an unreadable one." Amends D-119 item 1. **Flag: the architect's reading; Brian to confirm.** | D-127 | G2 |
| (d) CH-4, CH-5, CH-11, FIND-B-11 | "Journal: no `u64` above `i64::MAX` at encode and decode; a CRC-valid frame the decoder refuses at the tail is reported as damage, never dropped; the rebase omits the closing `Clock` after a terminal record; decode ignores unknown fields, the rebase keeps record bytes unchanged, `journal_version` is bumped for any change of meaning." | D-128 to D-131 | G3 |
| (e) Part B items "approve as built" | CH-1, 2, 3, 6, 7, 8, 10, 12, 13, 14, FIND-B-01 (`/bin/kill -0`), CH-12 temp name `chess.db.tmp-bak-v<N>-<pid>`, CH-13 (writer-assigned seq; `lease_delete` with owner). | D-132 | G3/G2 as the package classified them |
| (f) D5 | "FIND-D5-01 A: store-owned `StatsGameRow` and `StatsSide`, `chess-app` maps to `StatsRow`, no new edge. FIND-D5-02 B: variation identified by `(game, from_ply, pgn)`, idempotent save, `VariationRef` dropped." | D-133, D-134 | G1 / G2 |
| (g) D2 | "Stored abort text `aborted`; rating rows ordered by id in the store (domain orders by `(at, id)`); `VoidSpec` field `game`; failpoints `abort.before_commit`, `void.before_commit`, `reset.before_commit`." | D-135 | G1 |
| (h) FIND-E5 and OQ-EN-004 | "The 200 ms cancel guarantee holds only for an engine that honours `stop`; OQ-EN-004 resolved as accepted." (engine SDD already carries it; architecture §14 note.) | D-136 | G1 |
| (i) Confirmations recorded | `create_fresh` as specified in D-119 / Addendum L confirmed (with the amendment (c) flagged). | in D-127 | G2 |

Other items for the systems engineer:

1. Verification cases: add cases for NF2-1..11, NF4-1..19, VAR-1..12, and the journal tests of section 3 to the store verification set (CT-ST numbers are the engineer's; I did not allocate them).
2. Any requirement text that says an empty store opens silently after a data loss, or that "restore" is only offered when `chess.db` is absent, needs a look (NF-4). I found none by search; I did not read requirements.md in full `[UNV]`.
3. The user-facing text for the `StoreMissing` prompt (CLI, web, desktop) should cover "the database file is empty", not only "missing": front-end SDD owners' call; there is no protocol change.
4. RELEASE_NOTES: an entry listing the dated notes in the store SDD (3.9.1, 3.11.1, 3.7.2), the app SDD and architecture Addendum M.
5. (g) Format review checklist: "a change of meaning of any journal field bumps `journal_version`".

---

## 6. Not changed, risks, unverified

**Not changed by this work:** FIND-007 (missing `store.id` of a store with a schema is not recreated); the 43 s first-launch `Busy` retry; part B items "approve as built"; `StatsGameRow` code; D-119 items 2 to 10; no DDL, no `journal_version` change (stays 1).

**Risks (for Brian, G2/G3; I decide none):**

| ID | Risk | Mitigation |
|---|---|---|
| R-NF4-1 | A real store with **no journals, no backups and no backup files** (a store with players only) whose `chess.db` is truncated still opens as new, silently. NF-4 cannot see what is not there. | First auto-backup after the first game; the backup interval is a `chess-app` rule. Accepted by the ruling's scope. |
| R-NF4-2 | The refusal makes a data-lost store visible, but `create_fresh` sets all traces aside and the old games are reachable only by import by path or by moving the directory back. | D-119 item 6 text for the front-end. |
| R-NF4-3 | D-17 restore and import may assume `chess.db` is absent or has a schema. | Engineer reports the as-built behaviour (2.5); app SDD note for .3.13. |
| R-B06-1 | The copy test reads a state that can change before the real rollback if another process writes. | Needs a second live process on a store with a hot journal; accepted. |
| R-B06-2 | `NF2_COPY_CAP` is an `ESTIMATE`. | Engineer measures. |
| R-JF-1 | Tolerant decode lets an older build ignore a field that matters if someone forgets the version bump. | Section 5 (g) checklist; the CH-5 B net reports a refused frame, but an *accepted* frame with an ignored field is not detected. |
| R-JF-2 | `UndecodableTail` for a duplicate `seq` at the tail may report damage where the old code dropped silently. | Writer cannot produce it per review; the engineer reports. |

**Unverified `[UNV]` (read or not run by me):** I did not build or run any code or test. Not read: `create_fresh` code, `restore_backup`/import code, the `rev_nf2` fixtures, the rebase code path for the Clock, `read_store_id`'s error for an unparsable file, how the front-ends word the `StoreMissing` prompt. `temp_dir()` being per-user on macOS is inferred. The size of an empty-schema database is estimated, not measured.
