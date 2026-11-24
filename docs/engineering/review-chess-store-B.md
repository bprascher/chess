# Review: chess-store part B (bead chess-a0r.3.7), Reviewer stance

Reviewer: software-engineer agent, fresh invocation, adversarial. Baseline B6. GATE-108 (G3, stored format) is OPEN and not approved by this review. No SEI CERT conformance is claimed.

## 1. Snapshot

- Taken 2026-11-11 21:11:41 UTC (`date -u`) into the session scratchpad (`.../scratchpad/snap`), with workspace manifests, `clippy.toml`, `rustfmt.toml`, `deny.toml`, `third_party/`, `chess-domain`, `chess-rules`, `docs/architecture.md`, `docs/design/SDD-chess-store.md`. Workspace `members` cut to rules, domain, store.
- No D2/D5 files existed in `crates/chess-store` at that time (no finalize, lifecycle, stats, analysis, jobs), so nothing was removed.
- Tree hash of `crates/chess-store` (45 files): sha1-of-sorted-file-sha1s `78d3425560a79785764eaef079e3fb0cad1a2b89`.
- Part B sources (`journal/*`, `proc.rs`, `repo/coord.rs`, `migrate.rs`, `open.rs`) were byte-identical to the live tree when checked at the end. The live tree now also holds D2/D5 files; they were not reviewed.
- Review tests were written only in a copy (`snap2`, own target dir). Mutants ran in another copy (`snap3`). `crates/` and other docs in the repo were not touched.

## 2. Verdict: PASS WITH CONDITIONS

No acknowledged record was lost in any run. No crash state left a file under two names after restart. Format bytes match the SDD independently. There is **no Critical and no Major finding against the code as built today.** Conditions (all before the format is frozen at release, G3):

1. Decide CH-5 and CH-4 together (FIND-B-R01 and R02). CH-4(B) applied at decode only makes CH-5's loss path reachable by this build's own writer.
2. Make `close_final` crash-idempotent or give D4 an explicit repair rule (FIND-B-R03).
3. Fix or accept the `create`/`probe` race (R04).
4. Add the missing tests listed in section 6 (fail-open pid check, second migration guard, NULL synthesized `think_ms`).

## 3. Findings

Severity scale: critical / major / minor / info. Evidence tags as in SWE-001.

| ID | Sev | Finding | Evidence |
|---|---|---|---|
| FIND-B-R01 | minor, latent, format-relevant | **CH-5 as built can silently drop an acknowledged record.** A CRC-valid last record the decoder refuses (extra field, bad value, unknown kind or seq) is reported as `torn_tail_dropped=true, damaged=None`; `Scan` carries no signal that a CRC-valid frame existed. If the store has not applied it (`db_seq == last_intact`), `plan.damaged=false` and the move is invisible. If the store applied it (`db_seq = last+1`), D-66 catches it (`plan.damaged=true`). Not reachable from this build's writer: 100k-case property "writer never writes what the reader refuses" passed and every kill/fuzz round decoded. Reachable by a newer or different writer, a downgrade, or a future writer bug. Same class: CRC-valid frame with wrong `seq` or unknown kind. | `format.rs:` decode; `reader.rs:84-100` (`decode_frame(...).ok()`); `[RAN rev_format ch5_acknowledged_but_undecodable_last_record]` output: `records=3 torn_tail_dropped=true damaged=None ... db_seq=3: plan.damaged=false ... db_seq=4: plan.damaged=true`. |
| FIND-B-R02 | minor, design hazard | **CH-4 recommendation B is incomplete.** Encode accepts `u64::MAX` for `think_ms`, `white_ms`, `black_ms`, and `GameEnd` time totals; decode accepts them too (CH-4 confirmed). If only decode gets an `i64::MAX` bound, this build's writer will write records its reader refuses, which is exactly the CH-5 loss path. The bound must be in `Payload::validate()` so encode refuses too. | `[RAN rev_format ch4_symmetry...]`: `encode accepts u64::MAX ... true`; strictness run: `u64>i64 think_ms records=1`. `format.rs` `validate` only checks `pair_ok`. |
| FIND-B-R03 | minor | **`close_final` is not crash-idempotent.** `hard_link` then `remove_file` leaves the journal under two names (journal/ and journal/done/). A retry hits `AlreadyExists` forever; the file stays in `journal/`. `same_file` exists (window e) but is not used here, and no repair function covers `done/`. No failpoint sits between the two calls. | `[RAN rev_done]`: `nlink=2 same inode=true`; `RETRY close_final ... Err(WriteFailed { cause: Io(AlreadyExists) }); journal/ name still exists: true`. `files.rs:20-23`, `writer.rs:246-258`. |
| FIND-B-R04 | minor | **`create` races with `probe`.** `probe` takes `LOCK_EX` briefly; a `create` whose `try_lock` lands in that window returns `Locked` and removes the file it just made. FIND-B-10 said "not reproduced [UNV]"; it reproduces. Reverse also holds: a probe can make a genuine open fail. | `[RAN rev_race]`: 3000 rounds, `ok=2930 spurious_Locked=70 (file removed: 70)`. `writer.rs:74-111,121-134`. |
| FIND-B-R05 | minor | **`settle_rebase_files` installs an incomplete `.rebase`.** With `<uuid>.jnl` absent it moves any `.rebase` into place with no completeness check. A 10-byte `.rebase` became an `Unreadable` journal. Reachable only if the journal is missing while a rebase is cut off mid-write (not the normal flow), so latent. | `[RAN rev_crash settle_with_partial_rebase_and_no_journal]`: `RenamedIntoPlace; jnl len=10; scan: Err(Unreadable)`. `files.rs:124-139`. |
| FIND-B-R06 | minor | **NF-2 exception is not limited to a store without data.** The condition is directory markers only. A real v1 store with a player, a hot `chess.db-journal`, and no markers in `backups/`, `journal/`, `chess.db.bak-*` gets the read-write recovery: `chess.db` bytes changed, open succeeded, data intact. The architect's claim that the state "holds no user data by its own conditions" is false. Practically unreachable for an app-made store (WAL mode has no rollback journal), and the effect is SQLite's own recovery, so no data loss. | `[RAN rev_nf2]`: `read-only probe fails=true ... open=Ok(first_launch:false, schema_version:1) chess.db bytes changed=true`, `players still there: Ann`. `open.rs:178-194`, `datadir.rs:263-279`. Existing negative test only uses marker files (`hardening.rs:86-118`). |
| FIND-B-R07 | minor | **Operations under 4-process lease contention stall 5.3 to 5.4 s and return `Busy`.** 5 of 6 stress runs (4 processes x 400 zero-delay lease ops, one game) had at least one `beat` or `delete` fail after about 5.4 s (the busy timeout). All store writes go through one writer thread per process, so such a stall also holds up unrelated writes in that process. Stress, not typical load (average 7 to 9 ms per op, F_FULLFSYNC). | `[RAN rev_coord lease_races_four_processes]` x6: `errs={"beat Busy after 5.397491834s": 1}`, `{"delete Busy after 5.358557584s": 1}`. |
| FIND-B-R08 | minor | **`Store` drop does not deregister (FIND-B-08 confirmed); late `open` errors leave the row.** Row count stays 1 after drop while the process lives; another build version is refused for up to 30 s. `finish_open` deregisters only if `upgrade_registered` fails; `open_read_connection(...)?` and `sync_dir(...)?` failures after registration leave the row, against "every error path removes this process's row". The latter is `[OBS]` only, not run. | `[RAN rev_coord drop_leaves_the_registry_row]`: `rows before drop=1, after drop=1`. `open.rs:362-368`. |
| FIND-B-R09 | info | **Two `Store`s in one process share an owner.** `B.lease_delete(owner)` deleted A's row (`true`); `A.deregister_process` removed the shared registration. The same game cannot be opened twice in one process (separate fds conflict on `flock`), so impact is low. | `[RAN rev_coord two_stores_...]`. `api.rs` `owner()`. |
| FIND-B-R10 | info | **Pid check edge cases (all fail safe).** Rows with pid 0, pid above `i32::MAX`, or a future `heartbeat_at` (even `i64::MAX`) stay "live"; unparsable pids fall to age only; a row of a live pid with `heartbeat_at = i64::MAX` blocks upgrades indefinitely (needs write access to the DB). Message match on "No such process" is text dependent (failure means "alive"). | `[RAN rev_coord thresholds_...]`: live set `["a:0","b:4294967295","c:2147483648","d:-1","e:1","g: 1","h:00001"]`. `proc.rs:25-32`, `coord.rs:73-76`. |
| FIND-B-R11 | info | **Pid check spawn failure is fail-safe, and untested by the authors.** With `ulimit -u 1` a dead pid row `z:999999` showed as live. Mutant `Err(_) => false` survives the suite. | `[RAN rev_coord pid_check_when_spawn_fails_counts_as_alive]`; mutant M60 SURVIVED. |
| FIND-B-R12 | info | **Look-ahead amplification and unbounded read.** 8 MiB of sync-marker-plus-len-65536 units cost 1.9 s in one scan (release build); `scan_file` reads the whole file with no cap. Bounded by file size; matters only for a hostile journal from an import or restore. | `[RAN rev_format lookahead_amplification]`: `len=8388614 took 1.919s`. `reader.rs:112-122,136`. |
| FIND-B-R13 | info | **Second migration guard leaves a replaced backup.** If the guard passes before the backup and fails inside the exclusive transaction, the upgrade is refused but `chess.db.bak-v1` was already replaced (by a verified copy of the same version). FIND-010 as stated ("never replaced") holds for the first guard only. Nothing migrated, user_version 1. | `[RAN rev_coord guard_double_check_inside_exclusive_tx]`: `guard_calls=2 ... backup_replaced=true`. |
| FIND-B-R14 | info | **Pid check holds the DB write lock while spawning.** `register` runs `/bin/kill` per foreign row inside `BEGIN IMMEDIATE`. Cost measured about 1.1 ms per call (debug, loaded machine): 40 dead rows took 51 ms inside the lock. Within the architect's 20 ms-per-check target. | `[RAN rev_coord thresholds_...]`: `COST list_live 40 rows live pid 51.9 ms; 40 dead pid 43.8 ms; register 40 dead rows 51.5 ms`. `coord.rs:117-132`. |
| FIND-B-R15 | info | **Silent fallbacks that would be destructive if ever reached.** `acked_len = u64::try_from(HEADER_LEN).unwrap_or(0)` (a 0 would overwrite the header) and similar `unwrap_or(0)` in `rebase.rs:172`, `writer.rs:282`. Unreachable on 64-bit. `build_rebased` takes `intact_bytes`, `intact`, and `journal_last_ply` as redundant inputs and does not check them against each other. | `writer.rs:109`, `rebase.rs:38-56`. |
| FIND-B-R16 | minor, test quality | Tests that cannot fail or are weak: `tests/journal.rs` dropped-future test asserts `!on_disk.records.is_empty()` after a `Start` was already appended (always true; my rerun shows the dropped record does land: `records=2`); `let _ = apply_plan(...)` (`journal.rs:598`) asserts nothing; `let _ = i;` (`journal_faults.rs:349`); 10 ms x 200 polling loop. `MAX_BODY` boundary test uses the constant, so mutant M01 survives. | `[OBS]`, mutants M01, M41. |

Not findings (checked, held): CRC check value and byte-exact header and frame for every kind vs an independent encoder; field order equals SDD table; flush-before-ack (std `sync_all` is `fcntl(F_FULLFSYNC)` on Apple, `[OBS std sys/fs/unix.rs:1262-1274]`, mean append 11.6 ms debug); failed append does not consume `seq` for six failpoints; poisoning on cleanup failure and on dropped future; `flock` close-on-exec, kill -9 takeover, lock survives the swap rename; NF-1 sweep kept a live pid's temps and removed a dead pid's; lease compare-and-set; migration guard before backup leaves backup byte-identical and `chess.db` unchanged.

## 4. Verdicts on the architect's recommendations (code review support or contradiction)

| Item | Code review |
|---|---|
| CH-1 | Supports. Missing nullable key refused (`records=0`). |
| CH-2, CH-3 | Supports (not exercised beyond reading). D3 must map the DDL CHECK failure to a typed error. |
| CH-4 | **Supports the direction, contradicts "decode only".** Must also refuse in `validate()` at encode (R02). |
| CH-5 | **Supports B**, with two additions. Shown: the as-built tail case silently hides an acknowledged record when the store lags (R01). Qualify: the current writer cannot produce such a record. Generalize B to any CRC-valid rejected tail frame (bad payload, kind, or seq). |
| CH-6, CH-7 | Supports. 14 kinds, header flips, version 0 and mismatch behave as stated. |
| CH-8 | Supports. Mutant `k=0` killed. |
| CH-9 / FIND-B-01 | Supports A. 1.1 ms per call measured; fail-safe confirmed by run; caveats R10, R11, R14. Revisit trigger stands. |
| CH-10 | Supports. NULL stored values never get filled (`plan empty=true`); asymmetric (stored set, journal null: replaced). 20k-case model comparison, idempotence, and rebase-then-reconcile properties held with the wildcard. |
| CH-11 | **Evidence removes the correctness argument.** `Game::replay` accepts a `Clock` after `Ended`, `Aborted`, and `Voided` with no error and unchanged state (deliberate, `game.rs:640-645`). Option A is safe for replay; B is hygiene only (D-94 forbids a Clock before the terminal record, not after). Whether D4 assumes the terminal record is last is `[UNV]`. |
| CH-12 | Supports. |
| CH-13 | Supports; `lease_delete(game, owner)` compare-and-set verified single and multi-process; owner is per process (R09). |
| CH-14 | Supports; add R08 (late error paths leave the row). |
| FIND-B-06 | **Contradicts the stated justification** (R06). Narrow in practice; recommend the condition also require an empty database, or recovery on a scratch copy. |
| FIND-B-11 | Supports tolerant decode, with the CH-5 tail net kept. Strict decode plus CH-5(A) is the worst combination (R01). |

## 5. Evidence: commands run

All in the snapshot with my own `CARGO_TARGET_DIR`.

| Command | Result |
|---|---|
| `cargo fmt --check -p chess-store` | exit 0 |
| `cargo clippy --offline -p chess-store --all-targets -- -D warnings` (and with `--features test-support`) | clean |
| `cargo test -p chess-store` x5 | 223 passed, 0 failed, each run |
| `cargo test -p chess-store --features test-support` x5 | 269 passed, 0 failed, each run |
| Coverage | **not re-run** (machine loaded; the author's 96.66 percent is unverified by me) |
| Fuzz of `scan_bytes`, 320 000 random and mutated inputs (bit flips, splices, deletions, CRC-refixed payload garbage, wild len/kind/seq, duplicated ranges) | no panic; invariants held (contiguous seq, `intact_end` in range, torn and damaged exclusive, intact region re-scans identically); total 0.78 s, worst single scan 92 microseconds |
| Exhaustive truncation at every byte offset and every single bit flip of a 14-kind file (10 976 flips) | records always an exact prefix; header flips all `Unreadable`; no flip hid a later intact record; flips in the last record are torn-dropped (the SDD's accepted I-1 limit) |
| 330 random-time SIGKILL rounds of a child appending in a loop | 0 acknowledged records lost; no "damaged middle" from a kill; 17 torn tails, each under 200 bytes; lock free after every kill. Limit: process kill, not power loss |
| Kill at `journal.before_write`, `partial_write`, `after_write`, `before_fsync`, `after_fsync` (skip 2) and at first-record points | records on disk 2 or 3 as expected; torn tail only at `partial_write` |
| Rebase kill at `built`, `old_linked`, `old_aside`, `new_linked`, `new_in_place`, two `sync.journal_move_dir` points, `sync.journal_rebase_file`, then two restarts | each: exactly one `damaged/<uuid>.1.jnl` byte-identical to the original, no `.rebase`, all link counts 1, rebased file clean, intact run byte-for-byte, second restart changes nothing |
| 20 000-case model comparison of `reconcile`+`apply_plan`; idempotence; rebase then reconcile empty (15 611 cases with synthesized moves, 3 980 non-empty plans, 2 287 journal-wins) | all held |
| 100 000-case "writer never writes what the reader refuses" | held |
| Lease race, 4 processes x 400 ops x 6 runs | CAS held; R07 stalls |

## 6. Hand mutation (journal/*, proc.rs, repo/coord.rs, migrate.rs)

Runner executed the author's own tests (lib plus the journal, journal_faults, journal_crash, coordination, open suites chosen per file), not my review tests. 77 mutants defined; 6 did not compile (unused variables under `warnings=deny`) and were rewritten as M06b, M13b, M19b, M28b, M67b, M69b; M31 and M77 did not compile and were not rewritten. **75 evaluated: 68 killed, 7 survived.**

| Survived | Mutation | Reason |
|---|---|---|
| M01 | `MAX_BODY` 65 536 to 65 535 | boundary test uses the constant; constants-drift weakness |
| M21 | remove explicit seek in `write_and_sync` | equivalent today (cleanup seeks, position already right); unguarded for D4 `from_locked` resume, which opens at position 0. D4 needs an "append after reopen" test |
| M30 | `probe_path` unreadable-file `Held` to `Free` | no test for the unopenable-file branch; fail-open on the lock probe |
| M34 | `AlreadyExists` on damaged name returns error instead of next `k` | non-first-k race path untested |
| M41 | synthesized `think_ms` for a NULL row 0 to 1 | value not pinned |
| M60 | pid check spawn failure `true` to `false` | the fail-safe is untested (R11) |
| M73 | remove second guard inside the exclusive transaction | double-check untested by the authors; my `guard_double_check...` run shows it works |

Killed (selection): M02 (CRC not covering len, one side), M03, M04, M05, M07 to M12, M14 to M20, M22 to M29 (incl. b-variants), M32, M33, M35 to M40, M42 to M59, M61 to M72, M74 to M76. Full list in the runner outputs in the scratchpad (`mut.out`, `mut2.out`, `mut3.out`).

## 7. Conformance statement

- Tool-checked: `rustfmt --check`, `clippy -D warnings` with workspace lints (unwrap, expect, panic, indexing, casts denied), `#![forbid(unsafe_code)]` (`lib.rs:13`), no `libc` or `rustix` (`grep` over `src`: none). Errors print categories only; no path or SQL text appears in `JournalError`/`Display` (read `error.rs:55-104`). `StoreError::DurabilityUnsupported` carries a path field (part A, not re-read).
- Manual: read of all part B files; CERT is applied by intent only. No SEI CERT, MISRA or AUTOSAR conformance is claimed.

## 8. Not checked

Power loss and F_FULLFSYNC durability (only process kill); sandboxed or localized `/bin/kill`; PID reuse; coverage re-run; branch or MC/DC; D3/D4/D5 code and any reconcile consumer; payload-embedded sync-marker false positives in look-ahead; Intel Mac; `ci/check.sh`; cargo-deny; the author's evidence claims were read last and compared only where noted (FIND-B-10 "not reproduced" is reproduced; FIND-B-06 justification is not supported). Mutants were hand-made, not `cargo-mutants`. Timing under heavy machine load (other engineers ran suites concurrently) may have affected durations, not pass/fail.

## 9. Needs a human

Brian Prascher: decisions on CH-4, CH-5, FIND-B-11 as one package; GATE-108 stays PENDING. System Architect: R03 repair rule for D4, R06 wording.
