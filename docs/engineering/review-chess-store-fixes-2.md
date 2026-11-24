# Review: chess-store after the fixes and rulings of 2026-11-22/23 (fresh review 2)

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-store-fixes-2.md |
| Stance | Reviewer, adversarial, **fresh invocation**. I did not write the code and did not write `review-chess-store-rework-1.md`, `review-chess-store-D1-D2-D5.md` or `review-chess-store-B.md`. I read them to learn what was found, then judged the code and re-ran the evidence. Findings only; I changed nothing under `crates/` and no other document, made no commit or push, closed no bead |
| Baseline / rulings | B6. Decision beads chess-a0r.3.50 (2026-11-13..22) and chess-a0r.3.52 (2026-11-23, all recommendations of `decision-package-store-rework-findings.md` accepted) |
| Snapshot | 2026-11-23T11:00:19Z, copied to `<scratchpad>/rv-fixes2-swe-20261123-045934/` (uniquely named; the scratchpad is shared, so I deleted nothing I did not create). Tree hash of `crates/chess-store` (every file, sorted `shasum` of the per-file `shasum` lines, first 16 hex): **307a807710fba3ce**. Recomputed at the end: identical. `diff -rq` against the live crate at the start and at the end: empty (the live source did not change while I worked). HEAD `07a42b5` plus the uncommitted working tree. Copies: `crates/chess-store`, `chess-domain`, `chess-rules`, workspace manifests (`members` cut to those three crates in the copy only), tool configs, `third_party`, `docs/architecture.md`, `docs/requirements.md`, `docs/design/SDD-chess-store.md`. Own `CARGO_TARGET_DIR` per activity |
| Evidence tags | `[RAN]` executed here, `[OBS]` read here, `[INF]` inferred, `[UNV]` not verified |

## 0. Verdicts

| Gate | Part | Verdict | One line |
|---|---|---|---|
| **GATE-108** | Part B incl. the open path (NF-4 two-class rule, NF-2 narrowed, journal fixes) | **PASS WITH CONDITIONS** | Both Majors of rework review 1 (FIND-RW1-01, FIND-RW1-02) are fixed on every test I could build: 1 300 SIGKILL rounds of a first launch with 0 refusals, 156 failpoint cases, 2 400 concurrent-opener rounds (threads and processes, failing creators included) with 0 `Missing` and 0 `Corrupt`, 80 lost-store cases with every byte unchanged. No Major open. Conditions are Minor: the journal accepts `ended_at <= 0` that the store refuses (F2-04), the NF-2 exception needs a usable temp directory (F2-06), the committed kill test runs 120 rounds, not the 500 that the package names (F2-07), SDD notes 27 and 28 still describe the old rule (F2-08) |
| **GATE-109** | D2 (finalize, abort, void, reset, invariant) | **PASS WITH CONDITIONS** | FIND-D2R-01 (Major in the earlier review) is fixed: 31 211 checks beside 500 finalizes, 0 false violations (the earlier review measured 25 in 31 023 before the fix). The `analysis_used` latch holds through crash and replay. No Major. Conditions are Minor: F2-04 (the same `ended_at` mismatch), F2-05 (`set_analysis_used` can flag a finished game), F2-09 (`void_game_repair_pair` is an unguarded bypass with no caller) |
| **GATE-110** | D5 (stats, evals, variations, jobs) | **PASS WITH CONDITIONS** | No Major. The `(started_at, id)` order, the oldest-N limit, the 1 MiB cap and the owner idempotence behave as ruled. Conditions are Minor: an imported game that has both seats makes every statistics query of that Player fail with `Corrupt` (F2-01); games with a NULL `started_at` sort first (F2-02, the question about FIND-RW2-02); two mutants that drop the `id` tie-break survive the whole suite (F2-03) |

"PASS WITH CONDITIONS" here means: no Major, no Blocker; every open item is Minor or Info and none changes a ruling. It is not an approval. **GATE-108 to GATE-110 stay Brian Prascher's** (G2 and G3 delegation is not granted). This review is a desk review by an agent and does not stand in for his sign-off.

## 1. Method and independence

- Read first: CLAUDE.md, SWE-001, the three earlier reviews, the two decision beads, `decision-package-store-rework-findings.md` (the two-class table and residual risks R-NF4-1, R-NF4-4), then the code of `open.rs`, `datadir.rs`, `repo/finalize.rs`, `repo/lifecycle.rs`, `repo/stats.rs`, `repo/game.rs`, `repo/invariant.rs`, `repo/jobs.rs`, `journal/rebase.rs`, `journal/format.rs`, `fault.rs`, `api_analysis.rs`. To see what changed since review 1 I diffed those files against the review-1 snapshot that sits in the scratchpad (`snap_clean`, another agent's folder, read only).
- I wrote my own tests, in a **separate copy** (`work/`), so that the official runs of the clean snapshot were not contaminated: `tests/rv2_open.rs` (16 tests) and `tests/rv2_life.rs` (12 tests). They are not delivered and I did not rely on the authors' evidence files.
- Official runs (fmt, clippy, the five-times test runs, the race binary, coverage, mutation) used a clean copy or the `mut/` copy restored after every mutant (`diff -rq mut/crates/chess-store/src crates/chess-store/src` empty at the end).
- Process hygiene: I killed only PIDs that I started (a stuck failpoint child and my own test binary, found by `ps`; no `pkill`, no `pgrep -f`).

## 2. Standard commands [RAN]

| Check | Command (clean copy, own `CARGO_TARGET_DIR`) | Result |
|---|---|---|
| rustfmt | `cargo fmt -p chess-store -- --check` | exit 0 |
| clippy, default | `cargo clippy -p chess-store --all-targets -- -D warnings` | exit 0 |
| clippy, test-support | `cargo clippy -p chess-store --all-targets --features test-support -- -D warnings` | exit 0 |
| tests, default, x5 | `cargo test -p chess-store` | 5 of 5 exit 0; each run **356 passed, 0 failed** (sum over all test binaries and doc tests) |
| tests, test-support, x5 | `cargo test -p chess-store --features test-support` | 5 of 5 exit 0; each run **418 passed, 0 failed** |
| race binary, x10 | `cargo test -p chess-store --features test-support --test first_launch` (200-round thread race, 60-round three-process race, 120-round kill test, others) | 10 of 10 exit 0, 34 to 37 s each, 0 failed in every log. (In review 1 the same race test failed once in ten.) |
| coverage | `cargo llvm-cov -p chess-store --features test-support --summary-only` (cargo-llvm-cov 0.9.1) | 417 tests passed, 0 failed. Total: regions 92.87 %, functions 95.07 %, **lines 96.98 %**. Lines: open.rs 94.79 %, datadir.rs 94.61 %, repo/finalize.rs 94.64 %, repo/lifecycle.rs 97.55 %, repo/stats.rs 99.14 %, repo/game.rs 98.64 %, repo/invariant.rs 96.15 %, repo/jobs.rs 100 %, journal/rebase.rs 92.31 %, journal/format.rs 98.31 %, fault.rs 93.75 %. Branch coverage is not reported by this tool build (no claim made) |
| stats benchmark | `cargo test -p chess-store --features test-support --test stats_bench -- --nocapture` (debug build) | `stats_rows` 10 000 rows 52 ms, side white 9 000 rows 47 ms, window+limit 100 rows 2.5 ms, head_to_head 5 000 rows 27 ms, `games_for_history` 10 000 rows 43 ms |
| my tests | `cargo test -p chess-store --features test-support --test rv2_open --test rv2_life` (single tests, `--nocapture`) | results in sections 3 to 7 |

## 3. Item 1: the two-class trace rule (FIND-RW1-01) and the open path

What the code does now [OBS open.rs:442-470, 577-595; datadir.rs:278-300]: provenance traces (journal content incl. `done/`, `damaged/`; a non-empty `backups/`; `chess.db.bak*`, `chess.db.tmp-bak-v*`, `chess.db.pre-import-*`, `chess.db.restoring`) refuse an empty or schemaless `chess.db` with `Missing`. A `-wal` or `-journal` is a reported fact and never refuses by itself, except that a zero-length `chess.db` beside a non-empty `-wal` is refused before SQLite can delete the WAL. The 1 s settle wait and `WAL_SETTLE` are gone [OBS: no `WAL_SETTLE`, no sleep in `nf4_verdict`].

### 3.1 SIGKILL at a random instant during a first launch [RAN]

`rv2_kill_rounds`: a child process runs `Store::open` on a fresh directory, prints `ready`, and the parent SIGKILLs it after 0 to 1.2 times the measured open time (about 55 ms here). Then the parent reopens, writes a player, and runs `quick_check`.

| Run | Rounds | Reopen refused (`Missing`, `Corrupt`, any error) | Notes |
|---|---|---|---|
| A (kill timer started at process spawn) | 600 | **0** | Only 78 kills landed inside the creation; I judge this run weak and use the next one |
| B (timer started at the child's `ready` line) | **700** | **0** | 192 reopened as a first launch, 508 as a later open |

States the kills left in run B (each reopened without error): `db=0` and no `store.id` 61; `db=0` with `store.id` 49; non-zero schemaless `db=4096` beside a non-empty `-wal` (32 bytes up to 140 112) about 190; `db=0` beside a 512-byte `-journal` 5 (plus 1 with a 0-byte journal); `db=4096` beside a 512-byte hot `-journal` 2; kills after the first transaction (db 32 768 to 110 592 with a `-wal`) about 55; complete stores about 300; directory empty 30. So both killed-creation states of review 1 (non-zero schemaless database beside a WAL; zero-length database beside a rollback journal) and the hot-journal state occurred and completed. **0 of 1 300 refused.** The previous review measured 8 to 10 % refusals.

### 3.2 Fault injection at every `open.*` and `sync.*` failpoint [RAN]

`rv2_failpoint_matrix`: each of the 13 registered names that start with `open.` or `sync.` x actions `errno:5`, `errno:28`, `delay:5`, `kill` x skip 0, 1, 2 = **156 child runs**; after each, a clean reopen plus a write plus `quick_check`. **156 of 156 reopened `ok`.** Reached on a first launch (child outcome differs from a clean open): `sync.probe`, `sync.store_id_file`, `sync.store_id_dir`, `open.after_db_created`, `open.after_store_id`; the others (`sync.backup_*`, `sync.journal_*`) are not on a first-launch path and the run is then a plain open. `open.nf2_copy` is only reached by the NF-2 copy, which this matrix does not reach; the author's tests nf2_7 and nf2_2b do, and mutant O20 is killed by them. A `kill` failpoint waits 120 s for its parent before it aborts (fault.rs `KILL_WAIT`); my harness kills the child when it sees the marker.

### 3.3 Concurrent openers [RAN]

| Test | Rounds | Result |
|---|---|---|
| `rv2_threads_concurrent`: 3, 4, 5 and 6 threads on a fresh directory, one `FaultPlan` per directory in one round of three (the first thread to reach `open.after_db_created`, `open.after_store_id` or `sync.store_id_dir` gets `errno:5` once: a failing creator) | 4 x 300 = **1 200** | Every opener ended `Ok`, or a typed `Unavailable` where a fault was injected; **0** `Missing`, **0** `Corrupt`, 0 `Busy`; after the round a reopen succeeded and the number of players equalled the number of acknowledged writes (**0 lost writes**) |
| `rv2_processes_concurrent`: 3, 4, 5 and 6 **processes** (children of the test binary, released by a go-file), child 0 carries one of seven plans (none, `open.after_db_created=kill`, `...=errno:5`, `open.after_store_id=kill`, `sync.store_id_dir=errno:5`, `sync.store_id_file=kill`, `sync.probe=errno:5`) | 4 x 300 = **1 200** | Over all rounds 486 children were killed at a failpoint, 369 returned an injected error; **exactly one first launch per round (1 200 of 1 200)** even when the creator died; **0** `Missing` (exit 31), **0** `Corrupt` (32), 0 `Busy`; reopen `ok` and no lost write in every round |
| `rv2_lost_store_concurrent`: six threads open a **lost** store (db removed + journal; db zero-length + backups; db zero-length + journal + 4 096-byte WAL) | 150 x 3 | All six answered `Missing` in every round, and every file byte-identical afterwards (`-shm` ignored): **0 bad** |

### 3.4 Sidecars, WAL mode and rollback mode [RAN] (`rv2_sidecar_states`)

| State | Result |
|---|---|
| Schemaless non-zero database beside a `-wal` of 32 bytes (WAL magic), 1 000 garbage bytes, or 40 000 garbage bytes, plus `-shm`, plus `store.id` | first launch `Ok` (3 of 3) |
| The same three, each with one more provenance trace (journal file, `backups/` file, `chess.db.bak-v1`, `chess.db.restoring`, `chess.db.pre-import-1`, `chess.db.tmp-bak-v1-7`) | `Missing` (18 of 18), every file byte-identical (`-shm` ignored) |
| Zero-length `chess.db` + 14-byte junk `-journal` | first launch `Ok` |
| Zero-length `chess.db` + 100-byte `-wal` | `Missing` on two calls, every byte identical |
| Zero-length `chess.db` + 512-byte `-journal` + `backups/z` | `Missing`, every byte identical |
| Rollback-mode killed creation (database with a hot rollback journal from an in-flight table creation that spilled pages) | first launch `Ok` |

### 3.5 A LOST store must still be refused, byte for byte [RAN] (`rv2_lost_store_matrix`)

A real store (3 players) was made by `Store::open`, and also a store left by a SIGKILLed process (a 189 552-byte WAL holding all committed data, never checkpointed). Then: chess.db removed; zero-length with sidecars kept; zero-length with sidecars removed; replaced by a schemaless valid empty SQLite file; 4 096 zero bytes. Each was combined with eight provenance sets (journal; `journal/done`; `backups/`; `chess.db.bak-v1`; `tmp-bak-v1`; `pre-import`; `restoring`; journal+backups+bak). 5 x 8 x 2 = 80 cases, each opened twice, bytes compared before and after.

- **56 of 56** cases in which the database was removed or zero-length (48), or a schemaless valid database after a clean close (8), answered `Missing` twice with every byte unchanged.
- The 24 others, all explained, none a loss: (a) 8 cases, schemaless valid database + the killed store's real WAL: the WAL holds every committed page, SQLite applies it and the store **opens with its 3 players** (the data was not lost, so no refusal is due); (b) 16 cases, 4 096 zero bytes: clean store `Corrupt` twice with every byte unchanged (8), killed store: WAL recovers all 3 players (8).

### 3.6 R-NF4-1: is the accepted residual exactly that and nothing wider? [RAN] (`rv2_residual_matrix`, `rv2_mature_leftover_wal`, no provenance trace in any of them)

| Lost store, no journal/backup/bak/restore file | Outcome |
|---|---|
| `chess.db` removed (`store.id` stays) | `Missing`, nothing written (also with a leftover real WAL) |
| `chess.db` zero-length, any sidecars | with a non-empty WAL: `Missing`, nothing written; with no WAL (clean close, or WAL removed): **first launch, data gone** (the accepted R-NF4-1) |
| non-zero schemaless database, **no WAL or a garbage/empty WAL** | first launch (the accepted R-NF4-1 / R-NF4-4: the first write resets a WAL SQLite cannot read) |
| non-zero schemaless database beside a **valid WAL that holds everything** (killed store, never checkpointed) | store **opens with its 3 players**: the WAL is applied, nothing lost |
| non-zero schemaless, 4 096 zero bytes, 100-byte or one-page database beside a **valid WAL that holds only the later pages** (store checkpointed after 3 players, 3 more written, killed) | **`Corrupt`**, nothing written (5 variants: `Corrupt` x3, `Missing` x2 for the removed and zero-length forms) |

Conclusion: the open-as-new residual is **narrower than the decision package states**. The package says a lost store with a leftover WAL beside a truncated database "opens as new". That happens only when SQLite can read nothing from the WAL (absent, empty, header-only or garbage). A valid WAL either restores the data or makes the open fail `Corrupt` with nothing written. I found **no state wider** than: no provenance trace, and the database reads as zero tables after SQLite has applied whatever valid WAL there is. `[UNV]` A WAL that is valid but incomplete in a way that leaves a readable zero-table schema beside real data was not constructed; my one "partial WAL" fixture gave `Corrupt`.

## 4. Item 2: the real v1 store case of the narrowed FIND-B-06 [RAN] (`rv2_real_v1_hot_journal`, `rv2_stale_sweep`)

Fixtures: a store made by `Store::open` with 3 players and a raw game row, switched to rollback mode, an in-flight transaction with `cache_size=1` that spilled pages, then `chess.db`, `chess.db-journal` and `store.id` copied (no marker files). Every row below: open answered as shown, **`chess.db` and `chess.db-journal` byte-identical, no `chess-nf2-<pid>-*` left**.

| Case | Result |
|---|---|
| player and game present, in-flight table creation | `Unavailable{ReadOnly}`, nothing changed |
| in-flight `DELETE FROM player` (raw pages show no players, the rolled-back state has them) | `Unavailable{ReadOnly}` (the decision is on the rolled-back copy) |
| in-flight rename of every player | `Unavailable{ReadOnly}` |
| empty v1 store with an in-flight insert of the first player (rolls back to zero players) | exception applies, open succeeds, journal rolled back |
| database of 4 833 280 bytes (above the 4 MiB cap) | `Unavailable{ReadOnly}`, no copy, nothing changed |
| `TMPDIR` is a symlink to a directory | original error; the symlink target stays empty; no scratch left |
| `TMPDIR` is a 0500 directory, or missing | original error; nothing changed |

Stale-scratch sweep (`chess-nf2-<pid>-<n>` in the temp directory) with a real dead pid (`true` reaped), a real live pid (`sleep 60`):

| Entry | Result |
|---|---|
| dead pid, real directory, mode 0700, same owner | **removed** |
| live pid, 0700 | **kept** |
| dead pid, mode 0755; dead pid, mode 0770 | kept |
| dead pid, a **symlink** to a directory that holds a file | link kept, target and file intact |
| dead pid, a regular file with the name | kept |
| names that do not parse (`chess-nf2-notapid-0`, `...-<pid>-x`, `...-<pid>`, `...-<pid>-5-extra`) | kept |
| pid 0 and pid 4294967295 | kept (fail safe: `kill -0` answers alive or unknown) |
| foreign-owned | not testable as a non-root user; the in-crate unit test passes a different uid (`open.rs:1164`), and mutant O12b (uid test disabled) is killed by it |

Gaps I could not close: a directory owned by another user is tested by a function argument only [UNV as a live test]. The exception depends on a writable temp directory: see FIND-F2-06.

## 5. Item 3: finalize, terminal shape, stats order [RAN] (`rv2_life`)

**`analysis_used` latch (OR).** All four combinations through the public API: set no / request no -> 0; no / yes -> 1; yes / no -> **1**; yes / yes -> 1. A second `finalize_game` with the opposite flag answers `AlreadyFinal` and the flag does not move. Crash: with `finalize.before_commit` failing, the row stays `in_progress`, the flag stays 1; the replay with a rebuilt request that says `false` finalizes and the flag **stays 1** (`latch_crash_before_commit_then_replay`). Mutants F01 and F07 (overwrite, `min`) are killed.

**Terminal shape checks.** `finalize_game`: `ended_at` 0 and -5 -> `InvariantViolated`, status stays `in_progress`, rating rows 10 -> 10; `ended_at` 1 accepted (my loop stopped there because the game became final, so `i64::MAX` and the empty-termination case were **not** run by me `[UNV]`; the author's `d2r_03_finalize_refuses_a_non_positive_ended_at_and_an_empty_termination_and_writes_nothing` covers the empty termination and kills mutants F02 and F03); a game that is already final answers `AlreadyFinal` even with `ended_at 0` (the status test comes first). `abort_game`: `ended_at` 0 and -1 and an empty hash -> `InvariantViolated`; a valid one -> `Applied`; again with `ended_at 0` -> `AlreadyFinal`. The checks refuse before any write (nothing is written on the refusal path: `[OBS finalize.rs:128-134, 207-209]`, and the rating-row count test above).

**Is the shape check right for `abort_game` and for replay?** For the live call, yes: `ended_at` is epoch milliseconds from the injected clock. For **replay** it is right only if the journal cannot hold a value the store refuses, and it can: `Payload::from_json` and `to_json` accept `GameEnd.ended_at` 0 and -5 and `Abort.ended_at` 0 and -1 and re-encode them [RAN `journal_accepts_ended_at_the_store_refuses`]. A restore or a startup scan that replays such a record gets `InvariantViolated` from `finalize_game`/`abort_game`, so the game cannot be finalized from its own journal. That is the CH-4 defect class (writer writes what the consumer refuses) that Brian closed for the u64 fields. "Older data" does not exist (nothing is released, no store from before the rework), so no migration issue arises. `Voided.ended_at` and `void_unreadable_game(now)` accept any value. -> FIND-F2-04.

**Stats and list order.** `ORDER BY g.started_at, g.id LIMIT ?4` in `stats.rs:32` and `game.rs:115` [OBS]. [RAN] with games id 1 (started 100), id 3 and id 5 (both 50), id 2 imported with NULL `started_at`: `list_games` with no filter gives order id 2 (NULL), 3, 5, 1; with `limit = 2` the **imported game with a NULL start comes first and is one of the "oldest 2"**. With a status filter `Completed` and `limit 2`: ids 3, 5. With `from = 0` the NULL game is not returned (`NULL >= 0` is not true). `stats_rows(limit 2)` returns the two games started at 50. Where NULL sorts and who sees it: FIND-F2-02.

## 6. Item 4: earlier Majors stay fixed [RAN]

| Earlier Major | Now |
|---|---|
| FIND-D2R-01 (`check_rating_invariant` torn read) | `invariant.rs` reads inside `unchecked_transaction()` [OBS]. My `invariant_under_concurrent_finalize`: one writer finalizes 500 games, three reader threads loop `check_rating_invariant` for the whole time: **31 211 checks, 0 violations**, final check clean. The author's `find_d2r_01_...` passes in all ten suite runs. Mutant S08 (transaction removed) is killed by the author's test. Fixed |
| FIND-RW1-02 (a live creator refused) | The verdict no longer depends on a sidecar. The author's 200-round race passed in 10 of 10 runs of the race binary and in 10 of 10 full suite runs; my 2 400 concurrent rounds had 0 refusals; mutant O04 (the second `!db.exists()` re-check removed) and O21 (one attempt) are killed by the race tests. Fixed. Note: the O04 kill depends on race timing (section 9) |
| FIND-RW1-01 (killed first launch refused) | Fixed, section 3.1 |
| FIND-001, FIND-002 (first-launch race, interrupted creation) | Fixed: 156 failpoint cases, 2 400 concurrent rounds, `a_failed_first_launch_after_store_id_is_a_first_launch_again_not_missing` passes |

## 7. Item 5: regressions from the fixes [RAN]

| Fix | Check | Result |
|---|---|---|
| `void_game` precondition and repair pair | in-crate tests (7) plus mutants L01 to L05 | Precondition refuses a `completed` row with an Elo column or a rating row and leaves it unchanged; an in-progress row and an unrated completed row are voided. All five mutants killed. `void_game_repair_pair` is the same update without the check, has **no caller** and carries `#[allow(dead_code)]` (`lifecycle.rs:97-101`): FIND-F2-09 |
| Rebase idempotence | `rebase_idempotence_property`: 4 000 random journals (Start, 0 to 4 moves, a tail of nothing, Clock, a closing Clock, Pause, Abort; clocks set or null), rebase, scan the result, rebase again | **0 of 4 000** differ. Mutant S09 (the "already the closing Clock" test removed) killed; R01 and R02 (`check_intact` weakened) killed. I did not test `RunSource::Store` with synthesized moves in this property `[UNV]`; the author's test covers a terminal record and its own Clock |
| Payloads must be JSON objects | `array_payloads_and_clock_text` | `[]`, `[1]`, `null`, a BOM, `{}{}`, `{} x`, a vertical tab or form feed before `{` refused; `{}` and ` \t\n{}` accepted; a duplicate key refused. Good |
| Clock text follows the domain | same | `0+0`, `86401+0`, `05+0`, `+5+0`, `5+ 0` refused; `300+2`, `1+0` accepted; `null` accepted for an untimed game. `ClockText::parse` is `ClockSpec::from_spec_text` itself [OBS format.rs:178-182] |
| `complete_job` / `fail_job` idempotence | `jobs_idempotence` | owner repeats: `Ok`; other owner: `NotOwner`; `fail` after `done` and `complete` after `failed`: `NotOwner`; heartbeat after done: `NotOwner`; after a re-queue the old owner gets `NotOwner` until it claims again; unknown game `NotFound`. Mutant S10 killed |
| PGN cap | `pgn_cap` | exactly 1 048 576 bytes stored; +1 -> `VariationTooLarge{max_bytes}`; 524 289 two-byte characters (1 048 578 bytes) refused (it is bytes, as stated); an oversized text for an unknown game answers `VariationTooLarge` before `NotFound` (as documented); a repeat of the stored key is idempotent. Mutant S11 killed |
| `fault.rs` install errors | `fault_install_errors` | duplicate root, also through a symlink: `DuplicateRoot`; `nope/../x`: `UnresolvableRoot`; unknown name: `Unknown`; after the guard drops the root installs again. Two roots one inside the other **are both accepted** (a plan on `<root>/sub` while `<root>` has one): not an error and not tested; harmless for the tests that exist |

## 8. Findings

Severity: **Major** = wrong result in delivered behaviour that a consumer will hit, or a false claim in an acceptance list. **No Major finding is open.** Minor = defect or gap with a workaround or a bounded effect. Info = observation. All are CONFIRMED unless marked.

| ID | Sev | Gate | Summary |
|---|---|---|---|
| FIND-F2-01 | Minor, latent | 110 | An imported game that has both seats makes `stats_rows` and `head_to_head_rows` fail `Corrupt` for that Player |
| FIND-F2-02 | Minor | 110 | Games with a NULL `started_at` sort first and are the "oldest N" of `list_games` and `games_for_history`; a window drops them (FIND-RW2-02) |
| FIND-F2-03 | Minor | 110 | No test pins the `id` tie-break for equal `started_at` (mutants S02, S05 survive the whole suite) |
| FIND-F2-04 | Minor | 108, 109 | The journal accepts `GameEnd` and `Abort` with `ended_at <= 0` that `finalize_game` and `abort_game` refuse; `Voided` has no check at all |
| FIND-F2-05 | Minor | 109 | `set_analysis_used` has no status guard: it flags a game that is already final |
| FIND-F2-06 | Minor | 108 | The NF-2 exception needs a usable temp directory; without one a killed first launch with a hot rollback journal is refused `Unavailable{ReadOnly}` for ever |
| FIND-F2-07 | Minor | 108 | The committed kill test runs 120 rounds by default; the package and spec name at least 500 |
| FIND-F2-08 | Minor (G0 doc) | 108 | SDD-chess-store notes 27 and 28 still describe the refused-as-built rule; the R-NF4-1 text overstates the residual |
| FIND-F2-09 | Info | 109 | `void_game_repair_pair` is an unguarded bypass with no caller; D3 must define what a voided game that kept settled ratings means |
| FIND-F2-10 | Info | 108 | Open-path tests that I could not make deterministic: the O04 kill depends on race timing |

### FIND-F2-01 (Minor, latent): imported game with seats breaks statistics

- Location: `repo/stats.rs:151-159`. The comment says "Both seats are filled, so the game is not imported and has a mode (schema CHECK)". The schema does not say that: `CHECK (status = 'imported' OR (white_id IS NOT NULL AND black_id IS NOT NULL))` allows an imported game **with** seats, and architecture 7.2 says imported games "may have no registered Players" (so some may have one) [OBS 0001_init.sql, architecture.md:1492].
- Reproduction `[RAN]` (`ordering_with_null_started_at`): raw insert of `status='imported'`, white 1, black 2, `mode` NULL, `started_at` NULL; then `stats_rows(Ann)` -> `Err(Corrupt)`; `head_to_head_rows(Ann, Bob)` -> `Err(Corrupt)`; `games_for_history` -> `Ok`. All statistics of that Player fail, not just the row.
- Impact: today nothing writes such a row (no import path in this crate), so it is latent. It becomes a Major the moment the import links a seat. 
- Close by: exclude it in SQL (`g.status <> 'imported'` or `g.mode IS NOT NULL`) or return the row without a mode; a test with an imported game that has seats; fix the comment.

### FIND-F2-02 (Minor): where NULL `started_at` sorts (the question about FIND-RW2-02)

- SQLite sorts NULL first in ascending order. [RAN] `list_games(limit 2)`: the imported NULL game is first. Callers: `Store::games_for_history` (history list, "includes everything", `store/d5.rs:48`) and `list_games`. `stats_rows` and `head_to_head_rows` never see an imported game without seats (inner join on both seats) and see an imported game with seats only as `Corrupt` (F2-01). With a reset window (`from`), NULL-start games drop out of the history although the history "includes everything".
- Impact: "oldest N" is not the oldest N by start for a Player who has imported games; the ruling and SDD 20(a) are silent on NULL. No such row can be created by this crate today.
- Close by: decide NULL placement (`NULLS LAST`, or order by a coalesced time), state it in SDD 3.7.2, pin with a test that has an imported row. I am not choosing.

### FIND-F2-03 (Minor): tie-break not pinned

- Mutants S02 (`ORDER BY g.started_at` in `stats.rs`) and S05 (same in `game.rs`) **survive the whole suite** (all 418 tests, `[RAN]` S02all, S05all). Two games with the same `started_at` (restored games, a fake clock) can come back in any order and a `limit` can cut either.
- Close by: a test with two equal `started_at` and `limit 1` for `stats_rows`, `head_to_head_rows`, `list_games`.

### FIND-F2-04 (Minor): the journal accepts what the store refuses

- Location: `journal/format.rs` (`GameEndPayload.ended_at`, `AbortPayload.ended_at`, `VoidedPayload.ended_at` are plain `i64`, no check at encode or decode) against `repo/finalize.rs:128-134` and `:207-209` (`ended_at.0 <= 0` -> `InvariantViolated`).
- Reproduction `[RAN]`: `Payload::from_json(GameEnd, ...ended_at 0 / -5...)` and `Abort ended_at 0 / -1` decode and re-encode. `void_game` and `void_unreadable_game` take any `ended_at`.
- Impact: a journal whose last record is such a `GameEnd` or `Abort` cannot be applied by the store that wrote it; the startup scan and restore (D3, D4) would retry it for ever, or have to invent a policy. The three terminal paths disagree on what a valid end time is.
- Close by: one rule for all three: put the `> 0` check in `Payload::validate` and the decoder (so the writer never writes what the store refuses, as for CH-4), or relax the store to `>= 0`. A format change is G3; Brian decides.

### FIND-F2-05 (Minor): `set_analysis_used` on a finished game

- `repo/game.rs:187-193` updates `analysis_used = 1` for any status. [RAN] after `finalize_game(analysis_used = false)` the call returns `Ok` and the flag becomes 1 (`latch_values`). A late or repeated grant after the end changes a finished game's flag, which the statistics rules read.
- Close by: `AlreadyFinal` or a no-op for a terminal game, with a test; or state in the SDD that the gateway never calls it after the end.

### FIND-F2-06 (Minor): NF-2 needs a temp directory

- `open.rs:283-340`: any failure to create the scratch directory or copy gives `false` ("cannot tell") and the original `Unavailable{ReadOnly}` stays. That is right for a real store. But the same path is the only way out for a killed first launch that left a hot rollback journal (about 2 of 700 kills here).
- Reproduction `[RAN]` (`rv2_killed_creation_with_unusable_tmpdir`): database 4 096 bytes + hot journal of an empty creation: `TMPDIR` good -> `Ok(first launch)`; `TMPDIR` missing -> `Unavailable{ReadOnly}`; `TMPDIR` 0500 -> `Unavailable{ReadOnly}`; no later open ever succeeds.
- Impact: the FIND-RW1-01 promise "a killed first launch is never refused" holds only when `$TMPDIR` is writable. On macOS that is the normal case, so the effect is small.
- Close by: state the dependency in SDD note 21, or accept it as a risk (Brian).

### FIND-F2-07 (Minor, test): 120 rounds, not 500

- `tests/first_launch.rs:207` `unwrap_or(120)`; `CHESS_STORE_KILL_ROUNDS` overrides. The decision package ("a committed test with at least 500 kills and 0 refusals") and the review-1 finding ask for 500., and it asserts nothing about how many kills landed inside the creation.
- Close by: default 500 or more, and an assertion that a minimum number of rounds left a sidecar or a zero-length database.

### FIND-F2-08 (Minor, G0 doc): SDD text is stale

- `SDD-chess-store.md` note 27 (b) says both killed-creation states are, "as built", refused with `Missing`; note 28 (a) is still a proposal "pending" (I read notes 27 and 28 only, not note 22). Both describe the code before the 2026-11-23 ruling. The package text on R-NF4-1 ("opens as new") is wider than what I observed (section 3.6). Docs are Brian's and the architect's; I did not edit them.

### FIND-F2-09 (Info): the repair pair

- `lifecycle.rs:97-110`. After `GameEnd` (ratings settled) then `Voided` through the bypass, the row is `voided`, `result` NULL, and the rating rows and Elo columns stay (D-30 says a void changes no rating). Whether that is intended for D-87 is D3's to define; the precondition protects only the other callers. Carry to D3 acceptance (already in `desk-review-G1-step2-B.md`).

### FIND-F2-10 (Info): race-dependent kill

- Mutant O04 survived my subset run (the two 200-round and 60-round race tests were skipped there) and was killed in the full suite by `ct_st_004_*` (one run). A guard that is only killed by a race test can survive on a quiet machine. No deterministic test removes `&& !db.exists()` and fails.

## 9. Mutation testing (hand mutants, one at a time, scratchpad copy `mut/`)

Test sets per mutant (the author's suite only, not my tests): open and datadir `--lib` + `nf2_narrowed nf4_refusal hardening open crash first_launch boundary` (200-round, 60-round and kill tests skipped except for O21); finalize and lifecycle `--lib lifecycle lifecycle_faults games model`; stats and list `--lib stats_rows games analysis_variations d5_faults`; journal `--lib journal journal_crash journal_faults`; jobs `--lib job_queue`; variations `--lib analysis_variations`. Three mutants were re-run against the **full** suite (418 tests).

**74 attempted. 4 unviable (did not compile under `-D warnings`): O10, O12, D10, L07; each was replaced by a viable variant (O10b, O12b, D10b, L07b). 70 viable: 66 killed, 4 survived (2 equivalent in effect, 2 real test gaps). Kill rate 66/70 = 94.3 %; 66/68 = 97.1 % when the two equivalent mutants are set aside.** All are new mutants over the changed code (`open.rs` 25, `datadir.rs` 15, `finalize.rs` 10, `lifecycle.rs` 11, stats and list queries 7, rebase 3, invariant 1, jobs 1, variations 1; more than the 40 asked for).

| ID | File | Mutation | Result | First killing test |
|---|---|---|---|---|
| O01 | open.rs | zero-length guard: `len() == 0` -> `> 0` | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| O02 | open.rs | zero-length guard: `wal && len==0` -> `wal || len==0` | KILLED | a_failed_first_launch_after_store_id_is_a_first_launch_again |
| O03 | open.rs | guard reads `hot_journal` instead of `wal` | KILLED | rw1_01_a_zero_length_database_beside_a_journal_completes_as_ |
| O04 | open.rs | drop the second `!db.exists()` re-check after the trace scan | SURVIVED in subset; KILLED in full suite | ct_st_004_three_processes_opening_a_fresh_directory_lose_not |
| O05 | open.rs | a non-empty `-wal` counts as a trace again | KILLED | rw1_01_a_sidecar_that_appears_before_the_trace_scan_is_not_a |
| O06 | open.rs | `nf4_verdict` schema-appeared test `||` -> `&&` | KILLED | nf4_the_schema_has_appeared_for_a_version_alone_or_a_table_a |
| O07 | open.rs | copy cap 4 MiB -> 4 GiB | KILLED | nf2_6_above_the_cap_the_copy_is_never_attempted |
| O08 | open.rs | `copy_capped` `copied > cap` -> `>=` | KILLED | nf2_the_cap_boundary_is_exact_for_the_database_and_for_the_j |
| O09 | open.rs | `take(cap + 1)` -> `take(cap)` | KILLED | nf2_a_file_that_grows_after_the_size_check_is_not_copied_pas |
| O10 | open.rs | sweep: skip-own-pid line removed (unused var) | UNVIABLE |  |
| O11 | open.rs | sweep: mode test `!= 0o700` -> `== 0o700` | KILLED | nf2_the_sweep_removes_only_private_scratch_of_dead_processes |
| O12 | open.rs | sweep: uid check removed (unused var) | UNVIABLE |  |
| O13 | open.rs | sweep: `!is_dir` -> `is_dir` (symlinks and files) | KILLED | nf2_the_sweep_removes_only_private_scratch_of_dead_processes |
| O14 | open.rs | sweep: `!live` -> `live` (removes live pids) | KILLED | nf2_the_sweep_removes_only_private_scratch_of_dead_processes |
| O15 | open.rs | scratch mode 0700 -> 0755 | KILLED | nf2_the_scratch_directory_is_private_and_removed |
| O16 | open.rs | copy check `empty &= rows == 0` -> `|=` | KILLED | nf2_2b_a_store_with_data_reaches_the_copy_and_is_refused |
| O17 | open.rs | copy check counts only `game`, not `player` | KILLED | nf2_2_a_v1_store_with_a_player_keeps_its_error_and_chess_db_ |
| O18 | open.rs | size pre-check `>` -> `>=` | KILLED | nf2_a_file_that_grows_after_the_size_check_is_not_copied_pas |
| O19 | open.rs | size pre-check skips the journal file | SURVIVED (equivalent, see below) |  |
| O20 | open.rs | NF-2 exception ignores the copy verdict (`|| true`) | KILLED | nf2_7_a_failed_copy_keeps_the_original_error |
| O21 | open.rs | `FIRST_LAUNCH_ATTEMPTS` 8 -> 1 | KILLED | ct_st_004_three_processes_opening_a_fresh_directory_lose_not |
| O22 | open.rs | after `SchemaAppeared`, `fresh = true` instead of re-deciding | KILLED | nf4_18_a_schema_committed_before_the_trace_scan_is_not_a_ref |
| O23 | open.rs | `settle_fresh`: tables with version 0 not `Corrupt` | KILLED | r05_a_newer_schema_committed_between_the_reads_is_schema_too |
| D01 | datadir.rs | `tmp-bak-v*` no longer a provenance trace | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| D02 | datadir.rs | `pre-import-*` no longer a trace | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| D03 | datadir.rs | `chess.db.restoring` no longer a trace | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| D04 | datadir.rs | `wal` flag always true | KILLED | nf4_12_and_13_benign_names_beside_an_empty_database_are_a_fi |
| D05 | datadir.rs | `hot_journal` flag always true | SURVIVED (equivalent, see below) |  |
| D06 | datadir.rs | `backups/` not a trace | KILLED | nf_2_the_exception_does_not_apply_to_a_directory_with_a_stor |
| D07 | datadir.rs | `journal/` content not a trace | KILLED | nf4_18_control_the_same_trace_without_a_schema_is_missing |
| D08 | datadir.rs | `is_interrupted_first_creation` ignores `wal` | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| D09 | datadir.rs | `is_interrupted_first_creation` ignores `provenance` | KILLED | nf_2_the_exception_does_not_apply_to_a_directory_with_a_stor |
| D10 | datadir.rs | `content_of`: `len > 0` -> `>= 0` (clippy-absurd) | UNVIABLE |  |
| D11 | datadir.rs | `.DS_Store` no longer benign | KILLED | ds_store_files_are_not_traces_of_a_lost_store |
| D12 | datadir.rs | empty `journal/done/` counts as content | KILLED | a_failed_first_launch_after_store_id_is_a_first_launch_again |
| D13 | datadir.rs | `store.id` no longer a trace for an absent db | KILLED | ct_st_016_any_trace_without_chess_db_is_store_missing_and_no |
| D14 | datadir.rs | stale scratch: `!live` -> `live` | KILLED | remove_stale_scratch_counts_what_it_removed |
| F01 | repo/finalize.rs | finalize: latch removed (`analysis_used = ?10`) | KILLED | d2r_02_set_analysis_used_then_finalize_with_false_keeps_true |
| F02 | repo/finalize.rs | finalize: `ended_at <= 0` -> `< 0` | KILLED | d2r_03_finalize_refuses_a_non_positive_ended_at_and_an_empty |
| F03 | repo/finalize.rs | finalize: empty-termination check removed | KILLED | d2r_03_finalize_refuses_a_non_positive_ended_at_and_an_empty |
| F04 | repo/finalize.rs | abort: `ended_at <= 0` -> `< 0` | KILLED | d2r_03_abort_refuses_a_non_positive_ended_at_and_writes_noth |
| F05 | repo/finalize.rs | abort: empty-hash check removed | KILLED | abort_sets_the_hash_releases_slots_queues_a_job_and_rates_no |
| F06 | repo/finalize.rs | finalize: empty-hash check removed | KILLED | finalize_refuses_an_empty_hash_an_unknown_game_and_an_import |
| F07 | repo/finalize.rs | finalize: `max(...)` -> `min(...)` | KILLED | d2r_02_finalize_stores_the_or_of_the_stored_flag_and_the_req |
| L01 | repo/lifecycle.rs | void precondition: status test never true | KILLED | void_game_refuses_a_completed_row_with_a_rating_row |
| L02 | repo/lifecycle.rs | void precondition: rating-row test removed | KILLED | void_game_refuses_a_completed_row_with_a_rating_row |
| L03 | repo/lifecycle.rs | void precondition inverted | KILLED | void_game_on_an_already_voided_row_is_not_refused |
| L04 | repo/lifecycle.rs | `INSERT OR IGNORE` -> `INSERT` (job) | KILLED | release_and_queue_twice_keeps_one_job_and_does_not_fail |
| L05 | repo/lifecycle.rs | repair pair: empty-hash check removed | KILLED | void_game_refuses_an_empty_hash_and_an_unknown_row |
| L06 | repo/lifecycle.rs | reset: `voided` listed game not skipped | KILLED | reset_edge_cases_computer_foreign_game_terminal_listed_game_ |
| L07 | repo/lifecycle.rs | reset: names-player check removed (unused var) | UNVIABLE |  |
| L08 | repo/lifecycle.rs | reset point `k` 0 -> 1 | KILLED | ct_st_028_reset_voids_a_suspended_game |
| L09 | repo/lifecycle.rs | reset: unlisted-games check removed | KILLED | ct_st_028_an_unlisted_unfinished_game_rolls_the_whole_reset_ |
| S01 | repo/stats.rs | stats order `(started_at, id)` -> `id` | KILLED | d5r_03_stats_rows_are_ordered_by_start_then_id_and_limit_is_ |
| S02 | repo/stats.rs | stats order drops the `id` tie-break | **SURVIVED** (full suite too) |  |
| S03 | repo/stats.rs | stats order `started_at DESC` | KILLED | the_reset_window_is_started_at_greater_or_equal |
| S04 | repo/game.rs | list order `(started_at, id)` -> `id` | KILLED | d5r_03_games_for_history_and_list_games_use_the_same_key |
| S05 | repo/game.rs | list order drops the `id` tie-break | **SURVIVED** (full suite too) |  |
| S06 | repo/stats.rs | stats window `>=` -> `>` | KILLED | the_reset_window_is_started_at_greater_or_equal |
| S07 | repo/game.rs | list window `>=` -> `>` | KILLED | list_games_applies_structural_keys_only_ut_st_060_part |
| S08 | repo/invariant.rs | invariant: snapshot transaction removed | KILLED | find_d2r_01_the_invariant_check_never_reports_a_false_violat |
| S09 | journal/rebase.rs | rebase: "last is the closing Clock" test removed | KILLED | rebase_of_a_rebased_file_is_idempotent_after_a_terminal_reco |
| S10 | repo/jobs.rs | jobs: repeat-by-owner idempotence disabled | KILLED | owner_commands_on_a_job_that_is_not_running_or_not_there |
| S11 | repo/analysis.rs | pgn cap `>` -> `>=` | KILLED | d5r_01_a_variation_text_above_one_mib_is_refused_and_the_bou |
| O10b | open.rs | sweep: skip-own-pid disabled (`&& false`) | KILLED | nf2_the_sweep_removes_only_private_scratch_of_dead_processes |
| O12b | open.rs | sweep: uid check disabled (`&& false`) | KILLED | nf2_the_sweep_removes_only_private_scratch_of_dead_processes |
| D10b | datadir.rs | `content_of`: `len > 0` -> `len > 1` | KILLED | a_file_whose_length_cannot_be_read_counts_as_content |
| L07b | repo/lifecycle.rs | reset: names-player check disabled (`&& false`) | KILLED | reset_edge_cases_computer_foreign_game_terminal_listed_game_ |
| F08 | repo/finalize.rs | finalize: `voided` not AlreadyFinal | KILLED | a_game_voided_before_its_game_end_arrives_stays_voided_d_67_ |
| F09 | repo/finalize.rs | abort: `voided` not AlreadyFinal | KILLED | m14_abort_of_a_voided_or_completed_game_is_already_final_and |
| L10 | repo/lifecycle.rs | void_unreadable: `completed` not AlreadyFinal | KILLED | m25_void_unreadable_of_a_finished_game_is_already_final_and_ |
| R01 | journal/rebase.rs | rebase `check_intact`: drops the records comparison | KILLED | rebase_input_with_bytes_and_records_that_disagree_is_refused |
| R02 | journal/rebase.rs | rebase `check_intact`: drops the `intact_end` test | KILLED | rebase_input_with_bytes_and_records_that_disagree_is_refused |
| F10 | repo/finalize.rs | finalize: `imported` treated as AlreadyFinal | KILLED | finalize_refuses_an_empty_hash_an_unknown_game_and_an_import |

**Survivors.**

- **O19** (the size pre-check skips the journal): equivalent in outcome `[INF]`. `copy_capped` reads at most cap + 1 bytes of the journal and refuses above the cap, so the answer is the same; only the work done before the refusal differs (up to 4 MiB copied).
- **D05** (`hot_journal` flag always true): equivalent in outcome `[INF]`. The flag only gates `is_interrupted_first_creation`, which is only asked after the probe answered `ReadOnly`; a zero-length journal cannot cause that.
- **S02, S05**: real gap, FIND-F2-03.
- **O04**: killed only by the race tests, FIND-F2-10.

## 10. Conformance statement

| Item | Value |
|---|---|
| Scope | `crates/chess-store` at tree hash 307a807710fba3ce (the files named in section 1; not the whole crate line by line) |
| Standard | **None claimed.** SWE-001: Rust best practice, warnings as errors, SEI CERT applied by intent only. **No SEI CERT conformance claim is made.** MISRA and AUTOSAR do not apply (Rust) |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1 `[RAN]`, pinned by `rust-toolchain.toml`; edition 2024; clippy 0.1.99 and rustfmt 1.10.0-stable per SWE-001 `[OBS]`; cargo-llvm-cov 0.9.1 `[RAN]` |
| Tool-checked | `rustfmt --check` clean; compiler warnings denied (`[workspace.lints.rust] warnings = "deny"`); clippy `-D warnings` clean for both feature settings, with the workspace denials `clippy::all`, `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation`, `cast_sign_loss` outside tests; `#![forbid(unsafe_code)]` [OBS lib.rs] |
| Manually reviewed | the functions listed in section 1, for input, state, failure, concurrency, resources, time, contract and test strength (adversarial method of `code-review-conformance`). Sampling: all of `open.rs` open path, `datadir.rs` trace code, `finalize.rs`, `lifecycle.rs`, `stats.rs`, the list query, `invariant.rs`, the rebase builder, the diffs of `format.rs`, `fault.rs`, `jobs.rs`, `analysis.rs` against review 1 |
| Dynamic evidence | the test runs, kill, failpoint, concurrency, process and mutation runs above |
| Not checked | see section 11 |
| Deviations (`DEV-`) | none recorded; none approved by anyone |
| Overall | **NOT-DETERMINED as a conformance claim** (none is made). As a review: no Blocker, no Major; Minor and Info findings open |

## 11. Not run, not analyzed, assumptions

**Not run**
- ASan, UBSan, TSan, miri (the crate forbids `unsafe`; the concurrency evidence is dynamic only, no data-race tool). `cargo deny`, `cargo audit`, SBOM and the vulnerability scan (not in scope of this task).
- A real power cut, a full disk (`errno:28` was injected at the sync points of the first launch only), a read-only volume, a network or external disk, a sandboxed or notarized app (`/bin/kill` use), Intel hardware (no Intel Mac).
- Branch or MC/DC coverage (this llvm-cov build prints none).
- The kill, failpoint and concurrency rounds ran on one machine and one filesystem (APFS, internal disk, `F_FULLFSYNC` on); a different timing may reach states I did not. Under extra CPU or disk load I ran nothing new; review 1 did.
- The full mutation run was not done with my own tests in the set.
- A live test of a foreign-owned scratch directory (unit test by argument only).

**Not analyzed**
- Parts A, C and D3/D4 (apply_records, startup recovery, `create_fresh`, restore): not built or not in scope. `RebaseInput` and `reconcile` have no non-test caller yet, so the end-to-end replay of a journal into the store (including FIND-F2-04) is `[UNV]`.
- `desktop/`, `chess-app` and the front ends: how they map `Unavailable{ReadOnly}`, `Missing` and `Corrupt` is not reviewed.
- The text of the SDD, architecture and requirements beyond the places quoted.
- Whether `/bin/kill -0` works in the shipped sandbox.

**Assumptions**
- ASM-F2-1: the earlier reviews' snapshot folders in the scratchpad are what their documents say; I used `snap_clean` only to read diffs.
- ASM-F2-2: SQLite's behaviour that a read-only open deletes a `-wal` beside a zero-length database was taken from the earlier review and the decision package `[RAN by them]`; my own `rv2_residual_matrix` shows the store refuses before opening, and I did not re-run a raw read-only open for it.

## 12. Needs a human

- **GATE-108, GATE-109, GATE-110**: Brian Prascher. This document supports no approval by itself.
- F2-01 (where to filter imported games out of statistics), F2-02 (NULL start placement), F2-04 (one rule for `ended_at`; a format change is G3), F2-06 (accept the temp-directory dependency or not), F2-09 (D3 meaning of a voided game that kept ratings).
- F2-03, F2-05, F2-07 are test or small code items for the engineer; F2-08 is a document update for the architect and Brian.
- B-R03 and B-R05 (not dispositioned by Brian; the PM recommends fixing both in D4) are not part of this review's code scope; I did not re-test them.

## 13. Status block

```
Mode: Review (Reviewer) | Gate: GATE-108, GATE-109, GATE-110 (Brian Prascher, PENDING; no tier restated here) | Confidence: High for the open path and D2 (many independent rounds), Medium for D5 (latent items only)
Delivered: docs/engineering/review-chess-store-fixes-2.md
Ran: fmt exit 0; clippy -D warnings both settings exit 0; tests default 5x 356 passed; test-support 5x 418 passed; race binary 10x green; coverage lines 96.98 %; my kill rounds 1 300 (0 refused); failpoint matrix 156 (0 refused); threads 1 200 and processes 1 200 rounds (0 Missing, 0 Corrupt, 0 lost writes); lost store 80 cases and 450 concurrent; mutation 70 viable, 66 killed, 4 survived
Not run / not analyzed: sanitizers, power cut, Intel, D3/D4, front-end mapping, vulnerability scan, branch coverage
Open findings: FIND-F2-01..10 (no Major): 01-08 Minor, 09-10 Info
Needs a human: gate approvals; rulings on F2-01, F2-02, F2-04, F2-06
Next step: decide F2-04 (journal vs store end time) before D3 builds apply_records; add the tie-break and NULL-start tests (F2-02, F2-03)
```
