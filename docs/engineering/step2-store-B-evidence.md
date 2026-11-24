# Step 2, chess-store part B: journal, leases, process registry, F-A03 (evidence)

Task chess-a0r.3.7, CR chess-a0r.20, baseline B6, 2026-11-11. Engineer stance. Gate: [GATE-108](gates/GATE-108-chess-store-B.md), **G3**, **OPEN, NOT APPROVED**. Nothing here is reviewed or approved. Working tree only, nothing committed, no issue closed. Evidence tags: `[OBS]` read, `[RAN]` executed, `[INF]` inferred, `[UNV]` unverified.

## 1. What was built `[OBS]`

| Area | Where | Notes |
|---|---|---|
| Journal format (DD-ST-015): header, record frame, CRC-32/ISO-HDLC via `crc32fast`, kind codes, typed payloads with strict decode and encode | `src/journal/format.rs` | exactly SDD 3.11.1 / architecture 7.6; choices CH-1 to CH-8 below |
| Reader and scan (DD-ST-017): torn tail, damaged middle by look-ahead | `src/journal/reader.rs` | D-66 `StoreAhead` is the caller's (reconcile) |
| Pure reconcile (DD-ST-018): `fold`, `reconcile`, `Plan`, `apply_plan` (model of D3) | `src/journal/reconcile.rs` | total, idempotent (property test) |
| Rebase **builder**, rebase file write, swap (3a to 3f) | `src/journal/rebase.rs` | database step 2 is D4 |
| No-replace move helper (`hard_link` then `remove_file`), same-inode check (window e), `move_aside_damaged`, `settle_rebase_files` (windows a, b, c, f) | `src/journal/files.rs` | D-83, D-110 |
| Writer (DD-ST-016) with D-82 rules, `MoveJournal::{create, probe}`, `close_final` | `src/journal/writer.rs` | `File::try_lock`, `sync_all`, `set_len`, explicit seek |
| Process registry and lease rows (DD-ST-014) | `src/repo/coord.rs`, `Store` methods in `src/store.rs`, DTOs in `src/api.rs` | `register`, `heartbeat`, `deregister`, `list_live`, `lease_upsert`, `lease_delete`, `lease_beat`, `lease_owner` |
| Pid existence check | `src/proc.rs` | `/bin/kill -0`, see FIND-B-01 |
| F-A03 wiring, FIND-010 order | `src/open.rs` (`finish_open`, `upgrade_registered`), `src/migrate.rs` | registration, then guard before backup, guard again in the exclusive transaction |
| NF-1 per-process backup temp name and stale sweep | `src/backup.rs` | `chess.db.tmp-bak-v<N>-<pid>` |
| NF-3 `write_store_id_once`, D1-01 `remove_stale_scratch` | `src/datadir.rs` | |
| NF-2 hot-journal recovery for an interrupted first creation | `src/open.rs` (`open_probe`), `src/datadir.rs` (`is_interrupted_first_creation`) | |
| Failpoint wiring: the 6 `journal.*` points of SDD section 4 and 3.7.3, `journal.truncate_error`, 4 `sync.journal_*`, 5 `journal.rebase.*` | `src/journal/{writer,rebase,files}.rs`, registry in `src/fault.rs` | `journal.rebase.db_committed` has no call site in B (D4) |
| `FaultPlan::parse` suffix `!` (fire once) | `src/fault.rs` | needed to retry after an injected failure in a child |

Design points. **Append** is one `spawn_blocking` task; a guard poisons the writer if the future is dropped before it received the outcome (D-82 (3)); the task runs to completion either way. **Registration** happens in one `BEGIN IMMEDIATE` (delete stale rows of others, refuse on another build version, upsert); the **guard** (`other_live` empty) runs before the pre-upgrade backup and again inside the exclusive transaction. **Liveness**: heartbeat at most 30 s old (inclusive) and the pid exists; a future-dated heartbeat counts as fresh.

## 2. Acceptance, criterion by criterion

| # | Criterion | Status | Evidence (tests are in `crates/chess-store/`) |
|---|---|---|---|
| 1 | Journal records per architecture 7.6 table | MET `[RAN]` | `journal::format::tests::ut_st_070_every_kind_round_trips_and_has_exactly_the_table_fields` (one record of every kind, field names and order equal `Kind::fields()`, which is the SDD 3.11.1 table), `payload_json_keys_are_written_in_table_order...`, `text_forms_of_sf10_to_sf15_are_enforced`, `unknown_fields_missing_fields_and_wrong_types_are_decode_errors` |
| 2 | `content_hash` fields | MET `[RAN]` | `GameEnd`, `Abort`, `Voided` payload structs carry `content_hash` (text, not interpreted, D-42/D-68), `Voided.ended_at` (SF-05), the three time totals of `GameEnd` (SF-01); round trip in UT-ST-070; `Abort` and `Voided` without it are refused (`unknown_fields_missing_fields...`) |
| 3 | Lease 2 s / process 10 s / analysis 10 s heartbeat values | MET as values and methods `[RAN]`; **timer ownership is an interpretation (FIND-B-04)** | `repo::coord::tests::the_values_are_those_of_xc_16_and_oq_st_001` (`LEASE_BEAT_EVERY` 2 s, `PROCESS_HEARTBEAT_EVERY` 10 s, `ANALYSIS_LEASE` 10 s, `PROCESS_STALE_AFTER` 30 s), `liveness_is_age_at_most_30_s_and_an_existing_pid` (boundary at exactly 30 s, injected clock), `tests/coordination.rs::register_heartbeat_deregister_and_list_through_the_store`, `a_process_that_missed_its_heartbeats_for_30_s_is_stale...` |
| 4 | Stale lease takeover | MET `[RAN]` | `lease_rows_are_created_beaten_taken_over_and_deleted_by_owner` (takeover returns the old row, old owner's beat is `Lost`, old owner cannot delete the new row, `RowGone` carries the voided row's fields), `stale_lease_takeover_after_kill_9_with_two_processes` (child holds the journal lock, parent sees `Held`, `kill -9`, lock free at once, parent takes the row over) |
| 5 | Two-process tests | MET `[RAN]` | `ct_st_013_...`, `ct_st_014_...`, `ct_st_015_...`, `nf_1_concurrent_upgraders_...` (3 children, 6 rounds), `stale_lease_takeover_after_kill_9_with_two_processes` (child = the test binary re-run, `tests/common/child.rs`) |
| 6 | Stored format exactly as approved | MET with 8 numbered choices `[OBS]` | section 8, CH-1 to CH-8, for Brian. Header bytes pinned by `the_header_has_the_layout_and_checksum_of_the_sdd` and `create_writes_the_exact_header...`; frame bytes by `the_record_frame_has_the_layout_and_the_checksum_covers_len_and_body_only`; CRC check value `0xCBF43926` by `crc32_check_value_of_iso_hdlc` (UT-ST-081); limit 65 536 exact by `a_body_above_65536_bytes...`; kind codes by `kind_codes_are_those_of_sf07...` |
| 7 | F-A03: guard wired to the registry | MET `[RAN]` | `ct_st_015_...` (`Exclusive{Migration}` with a live process, upgrade succeeds after it is killed), `a_dead_process_row_does_not_block_an_upgrade` |
| 8 | `OtherVersionRunning{version, kind}` | MET `[RAN]` | `ct_st_014_...` (version `9.9.9`, kind `server`, no row for the refused process, same version coexists, `kill -9` of the other ends the refusal at once), `repo::coord::tests::another_live_build_version_refuses_and_writes_no_row` |
| 9 | The `PENDING part B` marker in `open.rs` is gone | MET `[RAN]` | `the_pending_part_b_marker_is_gone_from_open_rs` (also the accepting guard text) |
| 10 | CT-ST-013 (other-version half), CT-ST-014, CT-ST-015 with two real processes | MET `[RAN]` | `ct_st_013_newer_schema_is_refused_before_the_registry_is_touched`, `ct_st_014_...`, `ct_st_015_...` |
| 11 | FIND-010: guard before the backup | MET `[RAN]` | in `ct_st_015_...`: `chess.db.bak-v1` holds "last good copy" before the refused upgrade, byte-identical after; directory listing unchanged (no temp file); mutant O02 (guard call before the backup removed) is killed by it |
| 12 | Close-on-exec (SDD 3.11.6, CT-ST-006) with a child | MET `[RAN]` | `stale_lease_takeover_after_kill_9_with_two_processes`: the child opens the journal (lock) and starts a grandchild `sleep`; after `kill -9` of the child the lock is free (`probe` = `Free`) while the grandchild is alive. `[INF]` this shows std's `OpenOptions` descriptors are close-on-exec on this platform for `Command` children; it does not test `fork` without `exec` |
| 13 | NF-1 per-process backup temp name, D-108 rename-over kept | MET `[RAN]` | `backup::tests::the_temp_name_never_matches_the_backup_pattern`, `replacing_swaps_in_a_verified_copy...` (own, legacy and dead-pid temps swept), `a_live_processes_temp_file_is_left_alone`, `nf_1_concurrent_upgraders_...` (no `BackupFailed` or `Corrupt` in 6 rounds of 3 processes, no temp left, backup copy opens at version 1), existing CT-ST-090 tests (updated for the new name, not weakened) |
| 14 | NF-3 stale `.store.id.tmp-*` from a reused pid | MET `[RAN]` | `nf_3_a_stale_store_id_temp_with_a_reused_pid_does_not_break_the_first_launch` (temps `.store.id.tmp-<this pid>-0..7`); mutant O07 killed |
| 15 | D1-01 stale scratch cleanup by a later open | MET `[RAN]` | `d1_01_stale_scratch_files_of_dead_processes_are_removed_by_the_next_open` (dead pid removed; live pid, own pid, non-numeric and unrelated files left), `datadir::tests::remove_stale_scratch_counts_what_it_removed`; the D1 crash test that pinned "leftover stays" (`a_kill_at_every_sync_of_the_first_launch...`) still passes because it only checks that the directory opens `[OBS]` |
| 16 | NF-2 a cut-off first creation must not wedge the directory | MET `[RAN]`, **SDD exception needed (FIND-B-06)** | `nf_2_a_hot_rollback_journal_of_an_interrupted_first_creation_is_recovered` (the test first asserts that a read-only connection fails on the crafted state, then the open returns a first launch, then an ordinary open), `nf_2_the_exception_does_not_apply_to_a_directory_with_a_store_in_it` (a backup, a journal file or a `chess.db.bak-v1` keep the `ReadOnly` error and no byte changes) |
| 17 | NF-7 test gaps: R05 window, busy_timeout, Corrupt arms | MET `[RAN]` | `open::tests::r05_a_schema_committed_between_the_reads...`, `r05_tables_with_version_zero...`, `r05_a_newer_schema...` (a test seam `BETWEEN_READS` commits the schema between the two reads), `the_write_connection_waits_for_a_lock_during_the_wal_switch` (M16), `the_read_only_probe_and_read_connections_wait_for_a_lock` (M17, M18), `error::tests::stored_value_conversion_failures_map_to_corrupt` (M44, M45, all four arms), `a_successful_first_launch_leaves_no_scratch_file` (M38) |
| 18 | `journal.*` failpoints wired | MET `[RAN]` | `journal_faults::ct_st_001_...` (each of the six points reached once per append), `journal.truncate_error`, `journal.rebase.{built, old_linked, old_aside, new_linked, new_in_place}` (`a_full_swap_makes_four_directory_syncs_and_one_file_sync`); **`journal.rebase.db_committed` has no call site: D4 (FIND-B-05)** |
| 19 | CT-ST-003 / 087 / 088 / 089 kill tests as far as B allows | PARTLY MET `[RAN]`, deferrals in section 6 | `journal_crash::ct_st_003_...` (6 points), `ct_st_003_a_kill_between_the_last_move_and_game_end...` (9 terminal reasons), `ct_st_087_...` (2 variants), `ct_st_088_...` (6 points, file half), `ct_st_088_the_states_between...`, `ct_st_089_after_the_first_move...`, `journal_faults::ct_st_089_both_directories_are_synced_before_the_second_move`, `journal::...window_e...`, `settle_rebase_files_handles_the_crash_windows` |

## 3. Pid check without `libc` (the riskiest assumption) `[RAN]`

`kill(pid, 0)` is not in std and D-107 forbids `libc`, `rustix` and `unsafe`. `src/proc.rs` runs `/bin/kill -0 <pid>`: exit 0 means exists; stderr "No such process" means gone; anything else counts as "exists" (more cautious). On this Mac `/bin/kill -0 999999` printed `kill: 999999: No such process`, exit 1; `/bin/kill -0 1` printed `kill: 1: Operation not permitted`, exit 1 `[RAN]`. Tests: `proc::tests::this_process_and_a_running_child_exist_and_a_reaped_child_does_not`, `a_process_of_another_user_exists_by_eperm`, and every two-process test (a `kill -9` ends a refusal at once while the heartbeat is fresh). Cost: one process spawn per foreign registry row per check (not timed `[UNV]`).

## 4. Tests added (all in `crates/chess-store`; no test weakened or removed)

| File | Count | Content |
|---|---|---|
| `src/journal/format.rs` (unit) | 10 incl. 1 property | UT-ST-070, UT-ST-081, text forms, limits, header, never-panic property |
| `src/journal/reader.rs` (unit) | 8 incl. 1 property | UT-ST-074 to 079, R04 |
| `src/journal/reconcile.rs` (unit) | 9 incl. 2 properties | UT-ST-010 to 014, UT-ST-083, UT-ST-012 idempotence (gappy maps, damage flags, any seq) |
| `src/repo/coord.rs`, `src/proc.rs`, `src/datadir.rs`, `src/backup.rs`, `src/error.rs`, `src/open.rs` (unit) | 9 + 2 + 1 + 2 + 3 + 6 | registry with fake clock and pid table; NF-1, NF-3 parts, NF-7 |
| `tests/journal.rs` | 19 | create, append, lock, close, poison by dropped future, builder (UT-ST-088, 090), file steps, windows |
| `tests/journal_faults.rs` (feature) | 11 | CT-ST-001, 002, UT-ST-087, sync seams, CT-ST-089 order |
| `tests/journal_crash.rs` (feature) | 7 | kill tests |
| `tests/coordination.rs` | 15 | two-process tests, leases, scratch files |
| `tests/hardening.rs` | 2 | NF-2 |
| Edited | `tests/crash.rs`, `tests/open.rs`, `tests/fault_injection.rs` | the backup temp name is now `chess.db.tmp-bak-v<N>-<pid>` (3 assertions changed to match by prefix or by pid; the checks they make are unchanged) |

## 5. Commands run and results `[RAN]`

| Command | Result |
|---|---|
| baseline before any change: `cargo test -p chess-store --features test-support` | all green (the part D1 state) |
| `cargo fmt --check` (workspace) | exit 0 |
| `cargo clippy -p chess-store --all-targets -- -D warnings` | clean |
| `cargo clippy -p chess-store --all-targets --features test-support -- -D warnings` | clean |
| `cargo test -p chess-store` x5 | 223 passed, 0 failed in each of 5 runs (includes unit, integration, the compile-fail doctest) |
| `cargo test -p chess-store --features test-support` x5 | 269 passed, 0 failed in each of 5 runs |
| `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| `CHESS_CI_OFFLINE=1 ci/check.sh` (foreground, at the end) | exit 0, "All gates passed" (fmt, build, clippy, test, crate edges, ECO checksums, `cargo deny`: advisories, bans, licenses and sources ok, one `advisory-not-detected` warning for RUSTSEC-2026-0441 in `deny.toml`, not mine and not edited; `npm audit` skipped by the offline flag) |
| `cargo llvm-cov -p chess-store --features test-support --summary-only` | lines TOTAL 4547, missed 152, **96.66 %**; `journal/format.rs` 98.13 %, `reader.rs` 100 %, `reconcile.rs` 100 %, `rebase.rs` 90.08 %, `files.rs` 95.59 %, `writer.rs` 90.31 %, `repo/coord.rs` 97.98 %, `open.rs` 93.50 %, `datadir.rs` 93.07 % |
| `cargo llvm-cov -p chess-store --summary-only` (feature off) | lines TOTAL 4378, missed 171, **96.09 %**; `format.rs` 98.13 %, `reader.rs` 100 %, `reconcile.rs` 100 %; **`writer.rs` 81.12 % and `rebase.rs` 85.95 %** (their failure paths are exercised by the feature-gated tests, which do not run in this setting) |

Strategy gate (at least 90 % lines for `reconcile`, `format`, `reader`): met in both settings. No branch or MC/DC data (the toolchain reports none). The kill-child code paths (`Kill` arm of `fault.rs`) show as unexecuted for the same reason as in the D1 evidence.

## 6. What is deferred, and why

| Item | Deferred to | Reason |
|---|---|---|
| `open_for_resume`, `Recovered`, `RecoveryReport`, truncation of a torn tail by a new owner, the "row with no journal" path | D4 (chess-a0r.3.40) | not in B by allocation 5.2; CT-ST-003's "restart" is a scan and fold here, not a resume |
| Rebase orchestration (step 2, the store transaction) and the failpoint `journal.rebase.db_committed` | D3/D4 | needs `apply_records` (D3). The kill test uses a stand-in `db_seq` file and triggers that failpoint itself; CT-ST-088 completes with D4 |
| `apply_records`, backlog, finalize/abort/void/reset | D2, D3 | allocation |
| A timer for the 10 s and 2 s heartbeats | `chess-app` | FIND-B-04 |

## 7. Test strength: hand mutants `[RAN]`

Method: `scratchpad/mut.py` applies one text change, runs `cargo test -p chess-store --features test-support --no-fail-fast` over the lib and the test files `journal`, `journal_faults`, `journal_crash`, `coordination`, `hardening`, `fault_injection`, `crash`, restores the file, and records the failing tests. `diff -r src` against a saved copy printed identical after the run `[RAN]`. `cargo-mutants` was not used.

**77 mutants run: 74 KILLED, 3 SURVIVED (all three equivalent or redundant, see below), 0 left uncompiled.** First pass: 11 did not compile (an unused variable is an error under `warnings = deny`); they were rewritten as compiling mutants (`... && false`, `... || true`) and all 11 were killed. Four survived the first pass for lack of a test (R04, C13, W08, O10); a test was added for each and the mutant re-run and killed. Also two extra (`O08b`, `W01` pair) below.

| Area | Mutants (killed) |
|---|---|
| format (F01 to F10, F07b, F10b) | CRC value, `FIXED_BODY`, `MAX_BODY`, the length range, the checksum not covering `len`, kind code 14, half-null clock check, UUID form check, clock-spec leading zero, header checksum check (12 of 12) |
| reader (R01 to R07, R02b, R03b) | sequence check, game check, store check, look-ahead `>` vs `>=` (R04, found by this run), intact end, version 0, look-ahead start (9 of 9) |
| reconcile (C01 to C13) | fold drops later plies, takeback bound, ply base, NULL wildcards (think, clocks), D-66 damage flag, D-66 clocks rule, source choice, delete beyond the journal (both branches), fill of absent rows, `min` of the delete points, (C13 found by this run) (12 of 13; **C12 survives**) |
| rebase and files (B01 to B08, L01 to L05) | synthesize only when the store is longer, `>` vs `>=` of the last ply, `wall_ms` fallback, ply base, `.rebase` removal, both directory syncs, skip of the move aside, same-inode test, first `k`, settle windows, the `old_linked` failpoint (13 of 13) |
| writer (W01 to W11, W04b) | `acked_len`, `next_seq`, cleanup, partial-write failpoint, poison on drop, seam bypass of the sync, close syncs (W08 found by this run), probe fallback, poisoned check, `set_len` (10 of 11 plus the pair below) |
| coordination (P01 to P10, P02b, P08b) | 30 s boundary, pid check, own-row skip, version comparison, compare-and-set on both lease statements, takeover return, re-registration, stale deletion, commit before refusal (12 of 12) |
| open path (O01 to O11, O01b, O04b, O08b) | guard result, guard before backup (O02), registration, deregistration on error, per-process temp name (O06), NF-3 retry bound, scratch removal, NF-2 condition, interrupted-creation test, sweep of live temps (O10, found by this run) (all killed) |

Survivors, with reasons. **C12** (`is_empty` ignores `rows_to_delete_from`): `rows_to_delete_from` is only ever set together with `replace_from`, so the mutant cannot change a result `[INF]`. **W01** and **W01b** (remove the explicit seek in `write_and_sync`, or the one in `cleanup`): each seek makes the other redundant; removing **both** at once is KILLED by `ct_st_087_a_cleaned_up_failure_then_a_kill_...` (a hole appears) `[RAN]`. Limit as in the D1 evidence: removing the real `sync_all` inside a seam cannot be observed without power loss.

## 8. Numbered choices inside the SDD's freedom (stored format and API) for Brian Prascher

I made no approval claim and added no format field. Each choice is the most conservative reading I found.

| # | Choice | Why conservative | Stored format? |
|---|---|---|---|
| CH-1 | Every nullable payload field must be present (`null` or a value); a missing key is a decode error | a missing key could hide a truncated writer | yes |
| CH-2 | `custom_start` is not checked against `start_fen` at decode (the DDL CHECK of SF-02 does that) | the SDD lists the decoder's refusals; this is not one | yes |
| CH-3 | `content_hash`, `result`, `matchup`, `mode`, `eco`, `opening_name`, `san`, `fen_after`, `uci` are plain text, not validated against a pattern or a domain type | the store does not interpret them (D-42, D-68) and journal DTOs must not follow domain types | yes |
| CH-4 | Field types: `ply` `u32`, `think_ms` and the time totals `u64`, `ended_at` and `wall_ms` `i64`; negative numbers are refused where the type is unsigned | the SDD says "whole milliseconds `>= 0`" and names no width | yes |
| CH-5 | A record whose checksum and kind are right but whose payload the decoder refuses is treated as a bad record (torn tail or damage), never returned or applied | an undecodable record must not be applied; cost: such a last record is dropped silently unless `db_seq` exceeds it (D-66 then reports damage) | yes |
| CH-6 | Header: version 0 is `Unreadable`; a header whose `game_uuid` is not the file's game is `Unreadable`; `VersionMismatch` is checked before `ForeignStore` | the SDD names the three outcomes and not these edges | yes |
| CH-7 | JSON is compact, keys in the table order, no trailing bytes | the decoder accepts any order; the writer's order is fixed so a hex dump matches the table | yes |
| CH-8 | `damaged/<uuid>.<k>.jnl` counts `k` from 1 | "the first unused integer" | name only |
| CH-9 | Pid existence by `/bin/kill -0` (FIND-B-01) | see section 3 | no |
| CH-10 | `matches` treats a NULL stored `think_ms`, clock pair or `played_at` as equal to anything; the plan is "delete every row with `ply >=` min(`replace_from`, `rows_to_delete_from`), then upsert `rows_to_write`" | needed for UT-ST-088's empty plan after a rebase from rows with NULL columns | no |
| CH-11 | The rebase always appends the closing `Clock`, also when the journal wins | the SDD sentence reads "then append one `Clock`" after both cases | yes (record content) |
| CH-12 | Backup temp name `<stem>-<pid>`; a sweep removes the part A name and temps of dead pids, never a live pid's | NF-1 | name only (new name in the data directory; D-83 rated such names G2) |
| CH-13 | API shapes: `append(wall_ms, payload) -> seq` (the writer assigns `seq`); `lease_delete(game, owner)` (compare-and-set, SDD lists only the game); `Exclusive{reason: ExclusiveReason}` (enum, `Migration`); `OtherVersionRunning{version: String, kind: String}`; `LeaseBeat::{Held, Lost{owner}, RowGone{..}}`; `StoreOptions::with_process(kind, version)` | the SDD says "names are design proposals" | no |
| CH-14 | For an existing store the process registers before the upgrade; for a first launch after the schema exists. The guard runs twice (before the backup, inside the exclusive transaction) | SDD 3.9.1 and 3.11.5 together | no |

## 9. Findings (Engineer's own, all OPEN, severity is mine)

- **FIND-B-01 (Medium, SDD conflict, for the architect and Brian, G2).** SDD 3.11.6 says a row is live when `kill(pid, 0)` succeeds; D-107 admits no `libc`, `rustix` or `unsafe`, and std cannot signal a pid. Implemented with `/bin/kill -0` (section 3). Alternatives: admit `rustix` (4 crates per the spike) or `libc` (one `unsafe` block), or accept heartbeat age only (a killed process then blocks for up to 30 s). Failure of the tool is treated as "exists".
- **FIND-B-02 (Minor, SDD text).** SDD 3.8's file list names `sys.rs` and `journal/recover.rs`; the code has neither (D-107; recovery is D4), and adds `journal/{rebase,files}.rs`, `proc.rs`.
- **FIND-B-03 (Minor, SDD gap).** Failpoints not in the SDD section 4 list: `journal.truncate_error` (named in 3.7.3 rule 6 only), `sync.journal_create_file`, `sync.journal_create_dir`, `sync.journal_move_dir`, `sync.journal_rebase_file`. Registered in `fault.rs`. A dated SDD note is requested (G0 text).
- **FIND-B-04 (Minor, interpretation).** The 10 s process refresh and the 2 s lease refresh are methods with constants; the timers belong to `chess-app`. If the architect wants a timer inside `Store`, that is a design change.
- **FIND-B-05 (Info).** `journal.rebase.db_committed` is registered and used by a test stand-in; its production call site is the D4 transaction.
- **FIND-B-06 (Medium, SDD exception, for the architect and Brian).** NF-2 needs an exception to SDD 3.9.1 note 1 ("no write before the checks"): when the read-only probe answers `ReadOnly` and the directory shows only a cut-off first creation (a non-empty `chess.db-journal`, no journal content, no backups, no `chess.db.bak-*`), the opener opens `chess.db` read-write once so SQLite rolls the hot journal back, then probes again. It changes bytes of `chess.db` in that one state; it is not a stored-format change. Not decided here.
- **FIND-B-07 (Minor).** Two processes that start at the same moment and need an upgrade can both be refused with `Exclusive` (each registers, then sees the other). The SDD says registration and check are one transaction "so two simultaneous starts see each other"; this is that. No data is at risk (`nf_1_...`), the user retries.
- **FIND-B-08 (Minor).** `Store` drop does not deregister; a stale row of a dead pid is deleted by the next registration. Two `Store`s in one process share one row (same owner); deregistering one removes it for both. Tests pin this (`two_stores_in_one_process_share_one_registry_row`).
- **FIND-B-09 (Minor).** `JournalError::Locked{owner_kind}` is `None` from `create`; the caller reads `Store::lease_owner` for the kind. `create` of an existing file whose lock is held is `Locked`; of a free existing file `WriteFailed{Io(AlreadyExists)}` (the SDD has no `Exists` variant; the §5.7 list is unchanged).
- **FIND-B-10 (Minor, race).** `create` makes the file and then locks it; a `probe` by another process in those microseconds takes the lock and makes `create` return `Locked` for a file it just made (the file is then removed). Not reproduced `[UNV]`.
- **FIND-B-11 (Medium, stored format, for Brian).** SDD 3.11.1 says the JSON payload is "tolerant of additive fields" and also `deny_unknown_fields`. Implemented: unknown fields are refused (the stricter reading). A newer build that adds a field to a payload without bumping `journal_version` would make this build treat those records as bad (CH-5).
- **FIND-B-12 (Minor).** `ci/check.sh` does not run `chess-store` with `--features test-support` (D1-03, pending Brian under chess-a0r.3.43): `journal_faults.rs`, `journal_crash.rs` and the D1 files do not run in CI. `journal.rs`, `coordination.rs`, `hardening.rs` and the unit tests do.
- **FIND-B-13 (Minor).** Feature-off line coverage of `writer.rs` (81.12 %) and `rebase.rs` (85.95 %) is under 90 % because their failure paths need the feature; the strategy's 90 % list (`reconcile`, `format`, `reader`) is met.
- **FIND-B-14 (Minor).** The test suite takes about 100 s per pass of both settings (a 25 s test of part A and the child-process tests). Not measured per test `[UNV]`.
- **FIND-B-15 (Info).** `Store::journal()` returns `Corrupt` when `store.id` is missing for an existing store (F-A FIND-007 is open for Brian; no identity is invented).
- **FIND-B-16 (Info).** The reconcile "nothing readable in either" row is not a function result; the caller decides it from a scan error and an empty store.
- **FIND-B-17 (Info).** The mutation runs left two `proptest-regressions` files (failing seeds recorded when a mutant broke a property). They were removed; the properties pass on the real code.
- **No blocker. No Major finding.** F-A04 to F-A08 are untouched.

## 10. Not run, not analyzed, assumed

Not run: power loss or sudden reboot during an append, a rename or a directory sync (all durability claims are about process kill, where the OS cache survives `[UNV]` for power loss); `F_FULLFSYNC` on an unsupported or network volume; APFS atomicity of `hard_link`; Intel; `SIGSTOP` of a lock holder; `fork` without `exec`; hard links on exFAT (NF-6); `cargo-mutants` (hand mutants only); MC/DC (not measurable here); the timing of the pid check; more than 3 concurrent child processes; a non-English locale for the `kill` message (the macOS `kill` text comes from `strerror`; the fallback is "exists"). Not analyzed: the app-side `JournalCodec`, `open_for_resume`, startup recovery. Assumed: the Reviewer of part A's M-numbers are not re-run; the workspace pin `crc32fast =1.5.3` is the genuine crate (it was admitted by DL2 and pinned by the PM, I did not re-verify it against the registry).


## Rework 2026-11-22 (chess-a0r.3.50, Brian's rulings; baseline B6)

Engineer stance. Authority: decision bead chess-a0r.3.50 (relayed, no verbatim quote), the Reviewer report `review-chess-store-B.md`, and the architect's `store-rework-specs.md` (sections 1 to 3). **Nothing here is approved.** GATE-108 stays PENDING; the author does not approve it. Only `crates/chess-store` and this file and the GATE records were touched. Tags: `[RAN]` executed here, `[OBS]` read in code, `[INF]` inferred, `[UNV]` not verified.

### R1. What changed (one line each; stored-format items are G3)

| Item | Ruling | Change | Gate class |
|---|---|---|---|
| CH-4 | refuse values above `i64::MAX` at encode and decode | `MAX_COUNT = u64::MAX >> 1`; `Payload::validate()` checks `think_ms`, `white_ms`, `black_ms` (Move, Clock) and the three `GameEnd` totals; `encode_record` and `decode_frame` check `seq`. One check serves encode and decode (`from_json` also calls `validate`). Error: `FormatError::BadPayload` (the writer maps it to `WriteCause::Io(InvalidInput)` as before), no panic | G3 (stored format) |
| CH-5 B | a CRC-valid frame the decoder refuses at the tail is damage | new `DamageKind::UndecodableTail { at }`; `scan_bytes` classifies the dropped region with `classify_tail`: a later resumable frame gives `Middle`; else any CRC-valid frame in the region (refused payload, kind 0 or above 14, wrong `seq`) gives `UndecodableTail{at}`, `torn_tail_dropped = false`; else a torn tail (silent, unchanged). `ReconcileInput.scan_damaged` keeps its shape (`scan.damaged.is_some()`) | G3 |
| CH-11 B | no closing `Clock` after a terminal record | `build_rebased` tracks the last record kind; `GameEnd`, `Abort`, `Voided` last gives no `Clock` | G3 (record content) |
| FIND-B-11 B | tolerant decode | `deny_unknown_fields` removed from every payload struct; encode emits only the table fields; the rebase copies `intact_bytes` as bytes (already so, now tested); `journal_version` stays 1 | G3 |
| FIND-B-06 narrowed | rollback only when the copy holds no players and no games | `open_probe` now also requires `rollback_copy_holds_no_players_or_games` (copy of `chess.db` and `chess.db-journal` into `std::env::temp_dir()/chess-nf2-<pid>-<n>`, mode 0700, rolled back there, counts of `player` and `game`); conditions F (more markers, non-empty `-wal`), H (cap `NF2_COPY_CAP` 4 MiB), I (any error means keep the error) per spec 1.1; failpoint `open.nf2_copy` registered | G3 / G2 per spec section 5 |
| NF-4 | refuse an empty or schemaless `chess.db` beside data traces | `DataDir::data_traces`, `nf4_verdict`, `settle_fresh` in `open_blocking`; refusal is `StoreError::Missing`, nothing written; `create_fresh` is NOT built (part C) | G2 |

### R2. Tests (names; all `[RAN]` green in the final runs)

- CH-4: `journal::format::tests::{ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode, ch4_seq_above_i64_max_is_refused_at_encode_and_at_decode, ch4_the_boundary_is_the_literal_i64_max, the_writer_never_writes_what_the_reader_refuses}` (property over arbitrary `u64` and `Option<u64>`: `encode_record` Ok implies `decode_record` Ok and equal).
- CH-5: `journal::reader::tests::{ch5_a_crc_valid_frame_the_decoder_refuses_at_the_tail_is_damage_not_a_torn_tail (9 causes: bad value, not JSON, kind 99, kind 0, kind 15, repeated seq, gap seq, seq above i64::MAX, number above i64::MAX), ch5_the_tail_damage_reaches_reconcile_for_db_seq_3_and_4, a_refused_frame_followed_by_an_intact_one_is_a_damaged_middle, a_frame_with_an_unknown_json_field_is_read_as_a_record}`; changed: `a_wrong_sequence_number_is_bad_and_a_repeated_seq_is_not_resumable`, `a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point` (see FIND-ST-RW-05).
- CH-11 and B-11: `tests/journal.rs` `rework::{ch11_no_closing_clock_after_a_terminal_last_record, ch11_the_closing_clock_stays_when_the_last_record_is_not_terminal, unknown_fields_survive_a_rebase_verbatim, rebase_of_a_rebased_file_is_idempotent_only_after_a_terminal_record}`; `format::tests::{unknown_fields_are_ignored_on_read_for_every_kind (all 14 kinds), the_writer_never_emits_an_unknown_field, missing_fields_and_wrong_types_are_decode_errors}` (the old "unknown field is a decode error" assertions were removed from `unknown_fields_missing_fields_and_wrong_types_are_decode_errors`, renamed).
- B-06: `tests/nf2_narrowed.rs`: NF2-1 (with the size measurement), NF2-2 (**real v1 store with Ann, hot journal, no markers: error kept, every file byte-identical, no `chess-nf2-*` left**), NF2-2b, NF2-3 (game), NF2-3b (imported game, no player), NF2-4 (schema with no data takes the exception), NF2-5 (8 markers), NF2-5b (benign names), NF2-6 (over cap, plus "copy never attempted"), NF2-7 (injected copy error keeps the original error), NF2-8 (copy that cannot be read), NF2-10 (no leak, asserted in each).
- NF-4: `tests/nf4_refusal.rs` NF4-1, 2, 3 (FIND-002), 4 to 8 (10 traces, each refused twice with the directory snapshot equal, refusal fast), 7b, 8, 9, 10, 11 (FIND-007), 12 and 13; `open::tests::{nf4_18_a_schema_committed_before_the_trace_scan_is_not_a_refusal, nf4_18_control_the_same_trace_without_a_schema_is_missing, a_wal_with_content_beside_an_empty_looking_database_waits_for_a_schema}`. NF4-15 and NF4-16 (`create_fresh`) are not built (part C). NF4-17: the FIND-001 (200 rounds, 4 threads) and FIND-002 (60 rounds, processes) tests are unchanged and pass in every run.

Red first `[RAN]`: CH-4 (3 tests failed before the code), CH-5 (4 failed, after adding the enum variant only), tolerant decode (1 failed), CH-11 (1 failed). **B-06 and NF-4 tests were written after the code** (the architect's spec was the design); their red is shown by mutation instead (mutants NF2-1, NF4-1 and others below fail them).

### R3. Hand mutants (killed means the targeted tests failed; targeted test targets only, not the whole suite)

CH-4: 9 of 9 killed (bound shifted, `<=` to `<`, each field dropped from the check, `seq` check removed at encode and at decode, `GameEnd` arm dropped, one of three totals dropped). CH-5: 6 of 6 killed (kind-only CRC-valid case removed, `at > pos` guard removed, Ok-arm marker removed, `UndecodableTail` mapped back to torn, scan starts at `pos+1`, wrong `at`). CH-11: 6 of 6 killed (each of the three kinds removed from `is_terminal`, the reset of the flag after synthesized moves removed, flag never set, condition forced true). B-11: 5 of 5 killed (`deny_unknown_fields` back on three structs, rebase re-encodes the intact run, the writer emits an unknown field). B-06: 12 of 12 killed (G forced true, game or player count ignored, cap check removed or moved to the journal only, copy error treated as "empty", scratch not removed, `tmp-bak-v`, `restoring`, `pre-import-`, `bak` markers removed, non-empty `-wal` condition removed). NF-4: 9 killed (verdict forced to proceed, re-read removed, hot-journal trace removed, journal content or backups trace removed, pre-probe zero-length WAL refusal removed, `-wal` marker removed, schema-appeared branch forced to a first launch, settle wait removed); **2 survived and are equivalent** (merging a pre-probe WAL flag into the later scan; a re-check of journal traces inside the settle loop): the code of both was deleted. Control mutant (no change) survived as it must. 41 plus 20 mutant runs; 5 did not compile and were rewritten. Hand mutants, not `cargo-mutants`.

### R4. Final runs `[RAN]`

`cargo fmt -p chess-store --check` clean; `cargo clippy -p chess-store --all-targets -- -D warnings` clean, also with `--features test-support`; `cargo test -p chess-store` 5 runs, 321 passed, 0 failed each; `cargo test -p chess-store --features test-support` 5 runs, 378 passed, 0 failed each (the reviewer's pre-rework baseline was 223 and 269); `node ci/check-crate-edges.mjs`: "crate-edge check OK: 8 crates, 11 workspace edges, all allowed"; `CHESS_CI_OFFLINE=1 ci/check.sh` in the foreground: exit 0, "All gates passed" (npm audit skipped offline; cargo-deny prints one `advisory-not-detected` warning for RUSTSEC-2026-0441, pre-existing). Coverage (`cargo llvm-cov -p chess-store --features test-support --summary-only`, line coverage): `journal/format.rs` 98.34 percent, `journal/reader.rs` 100.00, `journal/reconcile.rs` 100.00, `journal/rebase.rs` 91.47, `journal/writer.rs` 90.31, `open.rs` 93.76, `datadir.rs` 94.46, TOTAL 96.91 percent. Feature-off coverage was not re-measured.

### R5. Findings of this rework (all OPEN; none is a blocker; no Major)

| ID | Severity | Finding |
|---|---|---|
| FIND-ST-RW-01 | Minor, spec correction | Spec 2.2 says a zero-length `chess.db` with a non-empty `-wal` "can hold a committed schema that SQLite recovers". `[RAN]` (python3 sqlite3, `mode=ro`): opening a zero-length database **deletes** a garbage `-wal` beside it, even in read-only mode. So that state is never "schema", and the read-only probe would erase the trace. Built: before the probe, a zero-length `chess.db` with a non-empty `-wal` is refused with `Missing` (a first launch in progress has written the header before a WAL exists `[INF]`). Architect to amend 2.2 and rows 4 to 7. |
| FIND-ST-RW-02 | Minor, deviation from spec 2.3 step 2 (G1, inside the crate) | A lone `-wal` with content beside a non-zero schemaless database is not refused at once: the probe reads repeat for up to `WAL_SETTLE` (1 s), because a concurrent first launch can show WAL frames before its commit is visible. Spec says "re-read once". Costs 1 s only in the refusal state. Architect to accept or change. |
| FIND-ST-RW-03 | Minor | The read-only probe of a WAL database with a `-wal` creates a non-empty `-shm` (32 KiB observed), not only zero-length ones. Tests ignore every `-shm`. Spec 2.2 and 2.6 should say so. |
| FIND-ST-RW-04 | Minor, spec 3.3 (e) is wrong for the non-terminal case | Rebasing an already rebased file adds one more closing `Clock` each time unless the last record is terminal (`[RAN]` test `rebase_of_a_rebased_file_is_idempotent_only_after_a_terminal_record`). Not changed: the ruling covers the terminal case only. Architect to decide whether a trailing identical `Clock` should be reused. |
| FIND-ST-RW-05 | Minor, interpretation (G3 detail) | CH-5 B was built over the **whole dropped region**: any CRC-valid frame in it (not only the frame at the first bad position) is damage. Consequence: the old test "a CRC-bad record followed by a CRC-valid repeat of the last good `seq` is a silent torn tail" now reports `UndecodableTail`. This matches the ruling's words ("any CRC-valid frame the decoder refuses") and not the letter of spec 3.2 (frame at `pos`). Writer cannot produce either case (property and kill tests of the review). Architect to confirm. A duplicate `seq` at the tail is now damage (spec risk R-JF-2). |
| FIND-ST-RW-06 | Info, not done: dependency missing | Spec 3.2 (e) end-to-end recovery test, 3.3 (d) replay equality and (f) done-journal check need D4 and `chess-app` (not built). NF4-15 and NF4-16 and the shared `database_state` classifier need `create_fresh` (part C). Spec 2.5's "how do restore and import treat an empty `chess.db`": no restore or import code exists in the crate (only failpoint names), nothing to report. |
| FIND-ST-RW-07 | Info | `NF2_COPY_CAP` measured: an empty v1 schema is 27 pages of 4096 bytes = 110 592 bytes `[RAN]`; sixteen times that is 1.77 MiB, under the 4 MiB constant, which is unchanged. |
| FIND-ST-RW-08 | Info, test honesty | B-06 and NF-4 tests were written after the code (see R2). Hot-journal fixtures copy `chess.db` and its journal while a spilling transaction is open (the existing `hardening.rs` method), not a real kill. `NF2-9` (zero-length journal) is not added: that fixture would leave a torn database, and the existing open tests cover B false. |
| FIND-ST-RW-09 | Info, consumers | New public enum variant `DamageKind::UndecodableTail`: D4 must treat it exactly as `Middle` for repair and report, and set `StoreAhead` only when `scan.damaged` is `None` (spec 3.2). No consumer exists yet. |
| FIND-ST-RW-10 | Info | The hot journal itself is still a data trace in `data_traces` (spec 2.2 item 6); after a successful NF-2 rollback it is gone, so a first launch is not refused. A non-hot but non-empty `chess.db-journal` beside an empty database is refused (tested). |

SDD conflicts for the architect (text only, nothing edited): SDD-chess-store 3.11.1 still says "unknown fields refused" in places and "tolerant" in others; the module doc of `format.rs` now says tolerant. 3.9.1 note 1 needs the narrowed NF-2 rule and the NF-4 row; the failpoint list gets `open.nf2_copy`.

### R6. Not run, not analyzed, assumed

Not run: power loss; Intel; `cargo-mutants`; mutants against the whole suite (only the targeted test targets); feature-off coverage; npm audit (offline). The last edit after the `ci/check.sh` run was one test assertion in `tests/nf2_narrowed.rs` (tightened to `ReadOnly` only); fmt, clippy and that test file were re-run after it. Assumed: SQLite behaviour with a zero-length database and a WAL is as observed on this machine's bundled and system SQLite `[UNV]` for other versions. **Needs a human:** Brian for GATE-108 (G3); the architect for FIND-ST-RW-01 to 05 and the SDD sentences; the systems engineer for verification cases NF2-1..11, NF4-1..19 and the journal cases.


## Fixes 2026-11-22 (after the Reviewer pass `review-chess-store-rework-1.md`, decision bead chess-a0r.3.50)

Engineer stance. **Nothing here is approved; GATE-108 stays PENDING (Brian Prascher, G3).** Reviewer verdict on part 1 was FAIL (FIND-RW1-01, FIND-RW1-02 Major). This section records the fixes of FIND-RW1-02 and the Minors; FIND-RW1-01 is **not touched** (it needs a ruling). Hand mutants were run one at a time against the named tests, restored after each (`[RAN]`, killed unless noted).

| Finding | Status | Fix | Tests (new unless noted) | Mutants |
|---|---|---|---|---|
| FIND-RW1-02 (Major) | FIXED, pending re-review | `nf4_verdict` (src/open.rs): a `chess.db-journal` or `-wal` with content no longer refuses on the first scan. Only a trace that no creator produces (`other`: backups, journal dir content, `.bak`, `.pre-import`, `.restoring`) refuses at once. For a transient trace the opener re-scans the traces then reads the schema, every 25 ms, for up to `WAL_SETTLE` (1 s): schema appeared -> `SchemaAppeared`; all traces gone -> `Proceed` (the creator rolled back); a data trace appeared -> `Refuse`; still there at the end -> `Refuse`. Scan-then-read order means a creator that commits and removes its journal in between shows its schema | `find_rw1_02_a_live_creators_journal_is_not_a_refusal_when_its_schema_lands` (deterministic, `BEFORE_TRACE_SCAN` plus `DURING_WAL_SETTLE` seams: journal present at the scan, creator commits during the settle), `..._a_journal_that_vanishes_without_a_schema_is_a_first_launch`, `..._a_journal_that_stays_without_a_schema_is_still_missing_after_the_settle` (NF-4 not weakened: `Missing`, waited at least `WAL_SETTLE`, journal bytes unchanged), `..._a_non_transient_trace_is_refused_without_waiting`, `..._a_data_trace_that_appears_during_the_settle_ends_the_wait` | O1 (old rule restored) killed by 3 tests; O2 (no `other` check in the loop) killed after adding the last test (first run SURVIVED); O3, O4, O5 (settle 0), O6 (stale traces reused) killed |
| FIND-RW1-03, FIND-ST-RW-04 | FIXED | `build_rebased`: no closing `Clock` when the last intact record is already the closing Clock (same clocks as the reconciled pair, `running_side` null). A Clock with other values or a running side is not "the closing one": one is appended, and then the file is stable | `rebase_of_a_rebased_file_is_idempotent_after_a_terminal_record_and_after_its_own_clock` (replaces `..._only_after_a_terminal_record`, which pinned the old behaviour "a second Clock, observed"; spec 3.3 (e)). Three successive rebases are byte-equal for Move-, Clock(same)-, Clock(other)- and Clock(running)-ending files | R1, R2, R6 killed |
| FIND-RW1-04 | FIXED | `Payload::from_json`: the first non-white-space byte must be `{` | `every_kind_refuses_the_array_form_of_its_own_payload` (all 14 kinds: the positional array of a real payload, `[]`, `null`), cases in `text_forms_of_sf10_to_sf15_are_enforced` (`[1]`, `["white"]`, `[1,2,null]`, `1`, `"{}"`, leading white space still accepted) | F1, F2 killed. F3 (reject a leading `[` only) SURVIVED: equivalent, serde already refuses every other non-object |
| FIND-RW1-05 | FIXED | `ClockText::parse` is `chess_domain::ClockSpec::from_spec_text`: base 1..86 400 s, increment 0..3 600 s, no u32 overflow. **Behaviour change:** `0+0` and `0+10` were "good" in the old SF-11 test and are now refused (the domain refuses them). The SF-11 text in the SDD should say so | edge grid base {0,1,2,86399,86400,86401} x increment {0,1,3599,3600,3601} compared with `ClockSpec::from_seconds`; `0+0`, `86401+0`, `1+3601`, `4294967296+0`, `18446744073709551615+18446744073709551615` refused | F4, F5 killed |
| FIND-RW1-06 | FIXED | `copy_capped`: reads at most cap + 1 bytes into a `0600` file and refuses a file that grew after the size check. `nf2_copy_cap()` has a test-only override so the boundary sits in a small file | `nf2_the_cap_boundary_is_exact_for_the_database_and_for_the_journal` (exactly at the cap copied, one byte below refused, journal larger than the database and the reverse), `nf2_a_file_that_grows_after_the_size_check_is_not_copied_past_the_cap`, `nf2_the_scratch_directory_is_private_and_removed` (scratch `0700`, copies `0600`, gone afterwards) | C1, C3, C4, C5, C6 killed. C2 (journal left out of the size check) SURVIVED: equivalent now, `copy_capped` refuses the over-cap journal itself (M41 of the review is therefore no longer a gap) |
| FIND-RW1-08 | FIXED | `sweep_stale_nf2_scratch` at each open: removes `chess-nf2-<pid>-<n>` directories in the temp directory when the pid is dead and not this process, the entry is a real directory (never a symlink) of mode `0700` owned like the data directory | `nf2_the_sweep_removes_only_private_scratch_of_dead_processes` (live pid, own pid, mode 0755, no counter, non-numeric pid, other name, symlink to a directory, plain file, other uid, missing base), `injected::nf2_8b_a_killed_opener_leaves_scratch_and_the_next_open_sweeps_it` (child SIGKILLed at `open.nf2_copy`, scratch exists with mode 0700, the next open in another process removes it and still completes the first launch) | C7, C8, C9, C10, C11, C12 killed |
| FIND-RW1-10 | FIXED | `build_rebased` scans `header + intact_bytes` and refuses (`RebaseError::InconsistentIntact`, new variant) unless it scans to its own end and decodes to exactly `intact` | `rebase_input_with_bytes_and_records_that_disagree_is_refused` (empty bytes with one record, bytes without records, fewer, more, altered record, torn tail, flipped byte) | R3 (unviable: dead code), R5, R7, R8 killed. The reviewer's first idea of separate damage and torn-tail clauses was dropped: `intact_end == len` covers both, and the extra clauses were equivalent mutants |
| FIND-RW1-12 | FIXED | tests only, plus `content_of` extracted from `data_traces` | M36: `the_builder_synthesizes_only_for_a_store_source_not_for_both` (killed). M40/M41/M44: see FIND-RW1-06. M51: `nf4_the_schema_has_appeared_for_a_version_alone_or_a_table_alone` (version only, table only, neither) killed. M60: `a_file_whose_length_cannot_be_read_counts_as_content` (killed) | as listed |

Existing tests changed, with the reason: `nf4_refusal.rs::nf4_4_to_8...` expected every refusal except a lone `-wal` to be immediate; a lone `-journal` now waits the settle like a `-wal` (FIND-RW1-02), so the immediate-refusal clause no longer applies to it. `text_forms_of_sf10_to_sf15_are_enforced`: `0+0` and `0+10` moved from accepted to refused (FIND-RW1-05).

Public surface added: `RebaseError::InconsistentIntact`. Behaviour changed: `ClockText::parse` limits, `Payload::from_json` refuses non-objects, `Store::open` waits up to one second before refusing a store with a lone non-empty `chess.db-journal`.

Left alone on purpose (decisions pending): FIND-RW1-01 (what counts as a trace beside a schemaless database; a killed first launch is still answered `Missing`), FIND-RW1-07 (spurious `ReadOnly`/`Busy` for concurrent recoverers: not in the fix list), FIND-RW1-09, FIND-RW1-11.

Not run: a power cut, a real concurrent creator inside the WAL switch beyond the seam emulation and the 200-round test, Intel. The settle is a bound, not a proof: a creator stalled for more than one second between its journal and its commit is still refused (`[UNV]`, the reviewer's own estimate was "a hair"; the 200-round test under the 20-run loop below is the evidence).

### Runs after the fixes `[RAN]`, 2026-11-22

- `cargo fmt -p chess-store -- --check` exit 0; `cargo clippy -p chess-store --all-targets -- -D warnings`, with and without `--features test-support`: no diagnostics.
- `cargo test -p chess-store` x5: exit 0 each (347 tests passed, 0 failed, summed over the suites of each run). `cargo test -p chess-store --features test-support` x5: exit 0 each (409 passed, 0 failed). The reviewer's baseline was 321 and 378.
- The 200-round race test `first_launch::ct_st_004_concurrent_first_launches_never_delete_the_winners_database_200_rounds`, run as part of the whole `first_launch` binary, 20 consecutive full runs: all 20 exit 0 (4 passed each). These runs were made while other agents compiled on the same machine.
- `node ci/check-crate-edges.mjs`: "crate-edge check OK: 8 crates, 11 workspace edges, all allowed". `CHESS_CI_OFFLINE=1 ci/check.sh` in the foreground: exit 0, "All gates passed" (`npm audit` skipped by the offline flag).


## Rulings 2026-11-23 (decision bead chess-a0r.3.52, package `decision-package-store-rework-findings.md`)

Engineer stance. Brian Prascher ruled on 2026-11-23 that all recommendations of the package stand (bead chess-a0r.3.51/.52). **This section records the build of those rulings; GATE-108 stays PENDING, no approval has been received.** Failing test first for every item (`[RAN]`: the new open tests failed on the old code, the kill test with `Missing` in round 2 of 120).

### Item 1, FIND-RW1-01 (G2): two classes of trace

| What | Where |
|---|---|
| `DataTraces` is `{ provenance, wal, hot_journal }`. `provenance` is class P: content in `journal/` (all subdirectories), a non-empty `backups/`, `chess.db.bak*`, `chess.db.tmp-bak-v*`, `chess.db.pre-import-*`, `chess.db.restoring`. `wal` and `hot_journal` are facts, not traces on their own; `DataTraces::any()` is gone | `src/datadir.rs` |
| `nf4_verdict`: no class-P trace gives `Proceed`; a class-P trace gives `SchemaAppeared` when the probe now reads a schema and `Refuse` otherwise. Traces are scanned first, the schema read after (the FIND-R05 order). **`WAL_SETTLE`, the settle loop, `DURING_WAL_SETTLE` and the creator-aware wait are removed** | `src/open.rs` |
| Unchanged: the pre-probe guard (zero-length `chess.db` beside a non-empty `-wal` is refused `Missing` before SQLite opens it, nothing destroyed), the hot-journal narrowed rule B-06 (`is_interrupted_first_creation`, NF-2 copy check) and the NF-4 refusal for real traces | `src/open.rs`, `src/datadir.rs` |

Corrected rows of package 2.3 as built: 4 and 5 (zero-length, `-journal` of any size ignored, `-wal` zero-length or absent) first launch or completion; 5b (non-zero schemaless, `-wal` of any size) first launch or completion; 7 (empty either form, one or more class P) `Missing`, nothing written; 7b (zero-length plus non-empty `-wal`) `Missing`, `-wal` untouched; 9 (hot journal) as B-06.

Tests (new unless noted):

| Test | Pins |
|---|---|
| `nf4_refusal::rw1_01_a_schemaless_non_zero_database_beside_a_wal_completes_as_a_first_launch` | state 1: WAL-mode and rollback-mode schemaless database x garbage, 32-byte header and real committed frames x `store.id` present or absent; then registers a player and reopens as a later open |
| `nf4_refusal::rw1_01_a_zero_length_database_beside_a_journal_completes_as_a_first_launch` | state 2: garbage, journal-magic and one-byte journals x `store.id` present or absent; the `store.id` bytes are kept |
| `nf4_refusal::rw1_01_each_data_trace_beside_a_killed_creation_state_is_still_refused` | each of 7 class-P traces beside state 1 and beside state 2 gives `Missing`, whole directory byte-identical (twice) |
| `nf4_refusal::nf4_7b_a_zero_length_database_beside_a_non_empty_wal_is_refused_untouched` | row 7b for 3 WAL forms, with and without `store.id` (replaces the old "non-empty wal" row of NF4-7) |
| `open::tests::rw1_01_a_sidecar_that_appears_before_the_trace_scan_is_not_a_refusal_and_not_a_wait` | `-journal` and `-wal` appearing at the trace-scan seam: first launch, under 900 ms |
| `open::tests::rw1_01_a_provenance_trace_that_appears_before_the_trace_scan_is_a_refusal` | a `.bak` file appearing at the seam: `Missing`, file kept |
| `first_launch::rw1_01_a_first_launch_killed_at_a_random_instant_is_never_missing_afterwards` | 120 rounds (`CHESS_STORE_KILL_ROUNDS`): a child process opens a fresh directory, the parent SIGKILLs it at a random instant (0 to 1.5 x the slowest of 5 measured opens), then reopens: never an error |
| Changed: `nf4_4_to_8...` (the two sidecar rows removed, the immediate-refusal clause now applies to every row), `nf4_7_a_non_empty_wal_beside_a_schemaless_non_zero_database_is_refused` (**replaced** by the first-launch test above) | the old behaviour they pinned is the superseded RW1-02 design |
| Removed, with the code they tested: `a_wal_with_content_beside_an_empty_looking_database_waits_for_a_schema`, `find_rw1_02_*` (5 tests: settle, vanishing journal, journal that stays, trace during settle, non-transient trace) | `WAL_SETTLE` is gone |

Race and kill evidence `[RAN]`:
- `first_launch` test binary, 20 consecutive full runs (each holds the 200-round thread test, the 60-round process test and the 120-round kill test): all 20 exit 0, 6 passed each.
- The 120-round kill test, 2 more runs on its own: both exit 0. Over the 22 runs (2,640 kill rounds): 0 refusals. Each run reopened as a first launch 19 to 34 times and as a later open for the rest; 14 to 34 rounds per run left a non-empty `-wal` or `-journal` (the states of the finding).
- The same test on the code before this change: `Missing` in round 2.

Hand mutants (one at a time, restored; the full list is in the report of the task), all KILLED: M1a to M1i (wal or journal counted as provenance, pre-probe guard removed, provenance never refuses, schema re-read dropped, `backups/`, `journal/` or `bak*` not a trace, hot-journal exception ignoring the `-wal`).

### Item 4, FIND-D5R-03 (G1): order by `(started_at, id)`

`stats_rows`, `head_to_head_rows`, `games_for_history` and `list_games` now `ORDER BY g.started_at, g.id`; `limit` is the oldest N. No index, no DDL change. Imported games have a `NULL` `started_at` (schema 1) and sort first in `list_games` without a player filter (SQLite orders `NULL` first): see FIND-RW2-02 in the report of the task. Tests: `stats_rows::d5r_03_stats_rows_are_ordered_by_start_then_id_and_limit_is_the_oldest_n`, `d5r_03_games_for_history_and_list_games_use_the_same_key` (restored-style rows: later `id`, earlier `started_at`, and a tie). Mutants M4a to M4e all KILLED.

Not done on purpose, per the ruling: no `variation(game_id)` index (d2), CH-5 reading as built (g, ST-RW-05), payload JSON object and clock text as built (no change), rebase idempotence kept (`journal::rebase_of_a_rebased_file_is_idempotent_after_a_terminal_record_and_after_its_own_clock` still green).

## Rulings 2026-11-24 (decision bead chess-a0r.3.56, package `decision-package-store-D3.md`)

Engineer stance. **GATE-108 stays PENDING (Brian Prascher, G3 for stored-format and stored-value items). No approval has been received.** Brian's rulings (relayed by the coordinator, no verbatim quote) are applied in `crates/chess-store`. Not reviewed by a Reviewer. Verification: `cargo test -p chess-store` 440 tests and with `--features test-support` 515 tests (5 runs each, all exit 0 `[RAN]`); race binary (`first_launch`, 500 SIGKILL rounds per run) 10 runs all exit 0 (6 tests each, 70 to 88 s); `cargo fmt --check`, `clippy -D warnings` both feature settings clean `[RAN]`; `node ci/check-crate-edges.mjs` and `CHESS_CI_OFFLINE=1 ci/check.sh` both exit 0 (`All gates passed.`, npm audit skipped by the offline flag).

- **F2-04 / D-144 (G3), journal format:** `GameEnd`, `Abort` and `Voided` `ended_at <= 0` and `Start.wall_ms <= 0` are refused at encode and at decode; `Start.matchup` must be `hh`, `hc` or `cc` and equal the code of the two kinds. `journal_version` stays 1 (nothing is released). A CRC-valid tail that breaks a bound is `UndecodableTail` damage (CH-5 B), tested in `journal::reader::tests::d3_a_crc_valid_tail_record_that_breaks_a_value_bound_is_undecodable_tail_damage`. The writer refuses such a record before it writes a byte (`tests/journal.rs::the_writer_refuses_a_record_the_format_refuses_and_writes_nothing`). Details and mutants: `step2-store-D3-evidence.md`.
- **F2-07 (test):** the kill test `rw1_01_a_first_launch_killed_at_a_random_instant_is_never_missing_afterwards` now defaults to **500 rounds** (`CHESS_STORE_KILL_ROUNDS` still overrides). Cost `[RAN]`: the `first_launch` binary took 76 to 92 s per run (10 runs, one at 257 s while another build ran), and the kill test printed `kill rounds 500, ... reopened as first launch 56, as a later open 444, rounds that left a non-empty -wal or -journal 48`. The earlier 120-round run took a fraction of that; the whole `cargo test -p chess-store` is now about 2 to 4 minutes, dominated by this test. 10 runs of the race binary: all exit 0 (6 tests each, 70 to 88 s); that is 5 000 SIGKILL rounds, 0 refusals.
- Verification: as above. Not reviewed. Approved by: nobody.
