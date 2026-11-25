# Review: chess-store part D4 (open_for_resume, rebase orchestration, StartupRecovery, B-R03, B-R04, B-R05), adversarial, fresh

| Field | Value |
|---|---|
| Stance | **Reviewer** only. Fresh invocation: I did not write the code or the earlier reviews. I read `review-chess-store-D3-rulings.md` for context and judged the code. I fixed nothing and edited nothing in `crates/` or in any other document. |
| Gate | GATE-112 (D4), tier **G3**, status **PENDING (Brian Prascher)**. This review approves nothing and claims no approval. |
| Baseline | B6. Code at HEAD `0b0a47d` (D4 commit `21fb357`). |
| Snapshot | Copy made 2026-11-24T20:03Z of `crates/chess-store`, `crates/chess-domain`, `crates/chess-rules`, workspace files, `docs/architecture.md`, `docs/requirements.md`, `docs/design/SDD-chess-store.md`; workspace trimmed to the three crates; own `CARGO_TARGET_DIR`. Tree hash of `crates/chess-store` excluding `proptest-regressions` (`find . -type f \| sort \| xargs shasum \| shasum`): **`c23fd2c62cb50db8`**. At 2026-11-24T20:52Z `diff -rq` of the live `crates/chess-store` against the copy was empty, so the review covers the tree as it stands. Copies: `scratchpad/rev-d4-sw-1795550596` (clean), `-atk` and `-atk2` (my attack tests; `-atk2` has a sleep inserted in `recover_game` to widen one race window, nothing else), `-mut` (mutation). |
| Scope | `journal/recover.rs`, `repo/resume.rs`, `store/d4.rs`, `journal/files.rs`, `journal/writer.rs`, `journal/rebase.rs` (swap), `store.rs`/`open.rs` (startup call), `fault.rs`. Against SDD 3.7.3, 3.9.1, 3.9.2, 3.11.3, 3.11.4, 3.12, D-64..D-72, D-78, D-82, D-83, D-87, D-92, D-94, D-110, D-137..D-153, allocation 4.4 and bead chess-a0r.3.40. |

## 0. Verdict

| Item | Verdict |
|---|---|
| **GATE-112 (D4)** | **PASS WITH CONDITIONS, with one Major (FIND-D4R-01) that I will not call a minor.** If Brian treats "one writer per game, enforced by the lock" as a G3 invariant, read this as **FAIL until FIND-D4R-01 is closed or accepted by him by name**. |

Why. The crash-safety work the bead asks for is sound: 1 800 + 1 800 real SIGKILL rounds at the `journal.*` and `rebase.*` failpoints, 300 at `close_final.linked`, 300 at each finalize failpoint, plus 450 SIGKILLs at random instants, all end in the required state (exactly one `damaged/<uuid>.<k>.jnl`, no `.rebase`, no name twice, second start changes nothing). Nothing I tried lost a byte of a journal except through a symlink at `done/` (FIND-D4R-03). What is wrong is the part the kill tests cannot see: **concurrency between two processes on the same game**, and test strength (28 of 71 mutants survive; 21 are real gaps, including B-R04 and B-R03/B-R05 properties that no test pins).

I changed no gate state. Brian Prascher's G3 approval is still needed and I have not received it.

## 1. Commands re-run `[RAN]` (clean copy)

| Check | Command | Result |
|---|---|---|
| fmt | `cargo fmt -p chess-store --check` | exit 0, no output |
| clippy default | `cargo clippy -p chess-store --all-targets --offline -- -D warnings` | exit 0, no warning |
| clippy test-support | `... --features test-support ...` | exit 0, no warning |
| tests default x3 | `cargo test -p chess-store --offline` | 3 of 3 exit 0, **504 passed, 0 failed** each |
| tests test-support x3 | `cargo test -p chess-store --features test-support --offline` | 3 of 3 exit 0, **593 passed, 0 failed** each |
| coverage | `cargo llvm-cov -p chess-store --features test-support --offline --show-missing-lines` | exit 0. TOTAL lines **96.65 %** (9 515 lines, 319 missed), regions 93.54 %. Scope files, lines: `recover.rs` 88.69 % (778, 88 missed), `files.rs` 93.85 % (130, 8), `writer.rs` 93.26 % (193, 13), `rebase.rs` 93.71 % (143, 9), `resume.rs` 94.74 % (38, 2), `store/d4.rs` 100 % (23, 0). `recover.rs` missed lines include 195, 259, 264, 272, 393, 404, 501, 595, 631, 707, 724, 891-893, 919, 1028-1031, 1055, 1090-1092, 1101-1103, 1115, 1154. |
| kill rounds | `CHESS_STORE_KILL_ROUNDS=300 cargo test -p chess-store --features test-support --offline --test recovery_kill -- --test-threads=2` | 7 passed, 0 failed, 1 532 s. That is 300 rounds at each of 6 `journal.*` points (1 800), 300 at each of the 6 rebase points including `db_committed` (1 800), 300 at `journal.close_final.linked`, 300 each at `apply.before_commit` and `apply.after_commit`. Each round restarts, checks the invariants, restarts again and compares. |
| random SIGKILL | my `rv_random_sigkill_rebase`, 450 rounds, child opens a store with a damaged journal, parent kills at 0..90 ms | 450 of 450 pass: 295 killed mid-recovery, 155 had finished. After restart: one damaged file holding the old bytes, no `.rebase`, link counts 1, journal bytes equal the uninterrupted control, `journal_seq` = last `seq`, `integrity_check` ok, second start byte-identical. |
| BM-ST-02 | `startup_bench --nocapture` (debug) | "open + quick_check + scan of 25 journals (5 finalized): 206 ms"; second open 144 ms. Under the 10 s ceiling. |
| no unsafe | `lib.rs:13` `#![forbid(unsafe_code)]`; `grep` for `unsafe`, `libc::` in `src` | none. No new dependency in the D4 diff (`Cargo.toml`, `Cargo.lock` unchanged). |
| panics | `grep` for `unwrap(`, `expect(`, slice indexing in the scope files | none outside `#[cfg(test)]`; slices use `get`. |

## 2. Findings, most severe first

### FIND-D4R-01 [MAJOR][CONFIRMED with the race window widened] A second resumer of a game that has no journal file takes no lock and moves the first resumer's live, locked journal into `damaged/`
- Location: `journal/recover.rs` `prepare()` (a lock is taken only when a `.jnl` or `.rebase` exists), the "no journal file" branch of `recover_game` (rebase from the store with `Holder::None`), and `journal/rebase.rs::swap_in_rebased` (`move_aside_damaged` whenever `<uuid>.jnl` exists, whoever holds it).
- Mechanism `[OBS]`: process B calls `open_for_resume` for a row with moves and no journal. `prepare` sees no file, so it locks nothing. B then awaits `resume_row` on its own store thread (which may wait up to 5 s behind the D3 flush or the plan-applier wait). Meanwhile process A does the same, finishes the whole rebase and holds a writer on the new `<uuid>.jnl`. B resumes, finds `.rebase` gone (A removed it at 3e), `create_new` succeeds, step 2 is idempotent, and step 3 moves **A's locked journal** to `damaged/<uuid>.1.jnl` and links B's file in.
- Evidence `[RAN]` `rv_stale_second_resumer` in a copy where I added a 2.5 s sleep after `prepare` for the thread named "B" (nothing else changed): A holds a writer on `<uuid>.jnl` and appends seq 7; B then returns `Ok((Some(writer), rebased = true))`; final tree `<uuid>.jnl` 962 bytes plus `damaged/<uuid>.1.jnl` 1 216 bytes; A's next append (seq 8) returned `Ok(8)` and went into the file that is now in `damaged/`. **Two live writers for one game, and acknowledged records in `damaged/` that the live journal lacks.** If the process dies after the journal ack and before the database apply, those moves exist only in `damaged/`.
- What I did not show: the window without my sleep. In 200 unwidened rounds (`rv_two_resume_nojournal`) the second resumer always lost at `create_new(.rebase)` (see FIND-D4R-02). The widened window is the real latency of `resume_row` under a busy store thread, not a different code path.
- Impact: breaks the single-writer rule the `flock` exists for (SDD 3.7.3, 3.11.3 window (c) note "the flock probably excludes a second writer `[INF, not checked]`"). The note was not checked and it does not hold for the no-file case.
- Close by: a failing test that reproduces it without a sleep in the library (for example a store thread held by a queued backlog batch for that game), then the fix; verify by that test and by `rv_stale_second_resumer` returning `Locked` for B. Not an SDD change; G1 inside the crate, but it is the G3 gate's data-integrity subject.

### FIND-D4R-02 [Minor][CONFIRMED] The second concurrent resumer of a row-with-no-journal game is told `WriteFailed`, not `Locked`
- Evidence `[RAN]` `rv_two_resume_nojournal`, 200 rounds, two stores on one directory resuming together: 200 of 200 `Err("the journal write failed") + Ok(writer)`. The loser fails at `create_new(.rebase)` in `write_rebase_file` (`AlreadyExists` mapped to `WriteFailed`). The damaged-journal case gives `Locked` correctly (200 of 200).
- Impact: `chess-app` will report a disk fault for what is "the game is open elsewhere". Close by mapping `AlreadyExists` on the `.rebase` create to `Locked` and a test that pins it.

### FIND-D4R-03 [Minor][CONFIRMED] B-R03 follows a symlink at `done/<uuid>.jnl`: `close_final` returns `Ok` and the journal is gone
- Location: `journal/files.rs::same_file` uses `fs::metadata` (follows links); `finish_close_final` and `recover.rs::prepare` trust it.
- Evidence `[RAN]` `rv_b_r03_done_name_attacks`: a symlink at `done/<uuid>.jnl` pointing at the journal: `close_final` -> `Ok(())`, `<uuid>.jnl` no longer exists, `done/` entry is a dangling symlink. Startup variant: `FinishedClose`, journal gone. A different regular file at `done/`: `Err(WriteFailed{Io(AlreadyExists)})`, both files intact (correct). Dangling symlink: `Err(NotFound)`, journal intact. Extra hard link (count 3): `Ok`, done link count 2 (correct).
- Impact: needs write access to the 0700 data directory, so a hardening finding, but it is exactly the "a different file must not be destroyed" case the B-R03 text asks about. Close by `symlink_metadata` and refusing anything that is not a regular file.

### FIND-D4R-04 [Minor][CONFIRMED] The B-R03 repair at startup does not look at the row; and a `done/` collision makes a game unresumable under a misleading error
- Evidence `[RAN]` `rv_done_same_inode_startup`: `<uuid>.jnl` and `done/<uuid>.jnl` the same inode with an **in-progress** row: outcome `FinishedClose`, the live journal name removed; the next resume rebuilds from the store (`damaged = StoreAhead`). The bytes survive in `done/`. `matrix done_and_jnl_different` with an in-progress row: startup `Failed("the journal write failed")` on every start and `open_for_resume` returns the same error: the game cannot be resumed, and the error class says "write" for a name collision.
- Impact: artificial states, but both are unreachable by the stated design and neither is reported as damage a person must look at. Close by checking `status` before `FinishedClose` and by a dedicated outcome for a `done/` collision.

### FIND-D4R-05 [Minor][CONFIRMED] B-R05's completeness test accepts a truncated `.rebase` at any earlier closing-looking `Clock`
- Location: `files.rs::rebase_is_complete` / `ends_a_rebase` (last record is terminal or any `Clock` with no running side).
- Evidence `[RAN]` `rv_b_r05_truncation_sweep` (a 922-byte rebase with a `Clock{running_side: None}` in the middle, truncated at every byte offset 0..=922): accepted prefixes **[584, 922]**; 584 is the boundary after the mid-file `Clock`. Everything else, and a header naming another game or store at full length, is moved to `damaged/<uuid>.<k>.rebase` with its bytes (correct).
- Impact `[INF]`: the cut file is installed and the records after it are lost from the live journal; step 2 had set `journal_seq` to the full file's last `seq`, so D-66 then reports `StoreAhead` and a second rebase runs (rows are safe). A cut `.rebase` with the journal absent should not occur (the file is synced before step 2), so this is defence in depth that is weaker than described ("decodes to an intact run whose records end as the rebase writes them", SDD 3.11.3 note 2026-11-24 (3)). Close by a test with a mid-file stopped `Clock`.

### FIND-D4R-06 [Minor][CONFIRMED] `RecoveryReport.moved_aside` names the highest `k`, not the file just made
- Location: `recover.rs::last_damaged_name`.
- Evidence `[RAN]` `rv_damaged_gap_k`: `damaged/<uuid>.2.jnl` and `.3.jnl` exist; the rebase makes `.1.jnl` (first unused, as designed); the report says `.3.jnl`. Mutant D36 (lowest `k`) survives. `move_aside_damaged` already returns the right path and `swap_in_rebased` throws it away.

### FIND-D4R-07 [Minor][CONFIRMED] `Store::open` blocks 5 s per game that needs a store transaction when another process holds the write lock
- Location: `store/d4.rs::apply_reconcile` (keeps the connection's normal 5 s wait, decision FIND-D3-14) called once per damaged journal by `startup`.
- Evidence `[RAN]` `rv_recovery_wait_under_db_lock`: three damaged journals, another connection takes `BEGIN IMMEDIATE` right after the process registration. Hold 3 s: open 3.22 s, all three `Rebased`. Hold 12 s: open **10.56 s**, one `Rebased`, two `Failed("the store is busy")`. The wait is 5 s times the number of such games, with no overall bound. The journals stay and the next start or resume retries (nothing lost).
- Impact: NFR-1 start-up when a second front-end holds a long write. Note also that if the lock is taken before the registration, `open` itself answers `Busy` after 5.2 s (`rv_open_under_db_lock`).

### FIND-D4R-08 [Minor][CONFIRMED by diff] Public surface changed against the baselined SDD, not yet amended
- `open_for_resume` returns `Result<(Option<JournalWriter>, Recovered), ResumeError>`; SDD 3.7.3 says `Result<(JournalWriter, Recovered), JournalError>`. The author reports the conflict; it is an interface change (Q2, G2 by the gate table, inside a G3 gate) that needs Brian's decision and an SDD amendment.
- `settle_rebase_files(journal_dir, game)` became `(journal_dir, game, store)` and `Settled` gained `MovedAsideIncomplete` (git: `d45482e` vs `21fb357`). `Settled`, `CloseRepair`, `MovedAside`, `Recovered`, `RecoveryReport`, `StartupReport`, `RecoveredGame` are public and not `#[non_exhaustive]` (`RecoveryOutcome` is), so the next variant or field is a breaking change.
- `OpenReport.recovery` added. `Recovered.moves` is `StoredMove`, not the SDD's `MoveRecord`. Public structs carry paths (`MovedAside.damaged`, `RecoveryReport.moved_aside`, `Settled::MovedAsideIncomplete.damaged`); `chess-app` must not forward them to a UI.

### FIND-D4R-09 [Minor][CONFIRMED] The B-R04 grace and the "does not remove the file" rule are not pinned by any test
- Evidence `[RAN]` mutation: W01 (`LOCK_GRACE_TRIES = 0`), W02 (grace never retries), W06 (grace already spent), W03 (`create` removes the file on `WouldBlock` again, the old bug), W04 (`probe` answers `Free` on an open error) all **survive** the whole D4 test set including `recovery_kill` and `journal_race`.
- My own races found no failure: 5 000 in-thread creates against a prober thread running 35.7 million probes: 5 000 `Ok`, 0 `Locked`, 0 other (55 s); 1 000 cross-process creates against a child probing each new file 300 times: 1 000 `Ok`, 0 `Locked` (23 s). But those runs would also pass without the grace, so they do not show it is needed.
- By reading `[OBS writer.rs]`: the grace cannot mask a real holder, because a file that already exists fails `create_new` first and gets `Locked` at once via `probe`; the grace applies only to a file this call just created. It can give a false `Locked` if a scanner or resumer in another process locks the just-created file and holds it longer than about 80 ms (not reproduced; my attempt to win the race did not take the lock). Then the file stays (0 or 42 bytes) and a retry of `create` is `WriteFailed(AlreadyExists)`, never `Locked`; a row-first start recovers it as an empty row (the matrix shows header rewritten at startup).

### FIND-D4R-10 [Minor][CONFIRMED] Test strength: 28 of 71 mutants survive, 21 are real gaps
See section 4. The ones that matter: no test checks the synthesized `Start` fields `rated` and `custom_start` (R04, R05: SDD 3.9.2 "the frozen `rated`, `self_play` and `custom_start`"), `no-replace` of the settle move (F01, CT-ST-089 "no-replace on both moves"), the aside loop's crash window (F12) and its directory sync (F13), the unreadable-header-with-`journal_seq` rebuild (D23), `scan_failed` when `store.id` is missing (D46, R03), the `Parked` classification (D45), and list-name filtering (D37, D38).

### FIND-D4R-11 [Info][CONFIRMED] Answer to FIND-D4V-03 (a `GameEnd` applied in the store plus a leftover `Voided` in the journal)
`[RAN]` matrix `completed_voided_pair`: row `Completed` (rating settled), journal `... GameEnd, Voided`: startup `ClosedLeftover`, journal to `done/`, `Voided` **not** applied, row stays `completed`. That is consistent with D-149 and 3.7.2 ("a `Voided` on a settled completed row is refused") and with D-87 as it applies to unfinished rows: `rv_inprogress_voided_pair` (row in progress, journal `GameEnd, Voided`, nothing applied) gives `Finalized`, status `voided`, ratings 1200/1200 unchanged. Not a defect. Two remarks: the outcome is silent about the disagreement between journal and row, and the pair on a settled row is no longer produced by any current flow, so the case exists only for journals left by older builds.

### FIND-D4R-12 [Info][INF] Other observations
- **D-72 across processes is not enforced.** Order is per process. `rv_two_processes_open_at_once`, 300 rounds, two processes opening one directory together with six pending `GameEnd` journals against one computer Player: 18 rounds in which both processes finalized some games, **0 rounds** where ratings or history order differed from the in-order control, 0 `Failed`. The 80 ms lock grace probably helps here by accident. No guarantee exists.
- **FIND-D4V-04 (parked set not persisted):** by reading and by the existing tests, a poisoned journal parks again at each start and `unpark_game` is in memory only; I did not build a journal that parks (my first attempt, a `Start` naming another player, is accepted by the applier and does not park). `[UNV]` beyond the existing tests.
- Header-only removal for a game with no row (`recover.rs` no-row branch): removes only a file of exactly 42 bytes whose header decodes; it never removes a journal with a record (`norow_with_records` stays). It does not compare the header's game or store with the file, so a foreign or renamed header-only file is also removed; a 0-byte stray file with no row is never removed.
- Startup reads each journal fully twice (`order_key`, then `prepare`), including journals whose lock a live session holds, with no size cap; each held journal costs the 80 ms grace.
- Unreadable header with intact records and nothing in the store (`bad_magic_norows`): `Unreadable`, `Failed` at every start, 3 acknowledged records unapplied. This is the RR-A-1 (a) design (and the SDD's media-fault limit), not a defect.
- Read errors are labelled `WriteFailed` (`io_err`). `UT-ST-038` scans only `api.rs`, `store.rs`, `error.rs`, `datadir.rs`; by reading, `recover.rs`, `files.rs`, `resume.rs` have no integer identity fields and `Display` of every error is a category without path or id.
- Cancellation: `rv_cancel_resume`, 300 rounds of `timeout(0..36 ms)` around `open_for_resume` then restart: the invariants held in all rounds (I did not capture the completed/cancelled split; the test printed it as the last line of a run I read only for pass/fail, so I report it as not recorded).

## 3. Attack matrix `[RAN]` (`rv_matrix`, startup and resume on every state, then a second start)

No journal bytes were lost in any state except the two intended removals (a stale `.rebase` and a 42-byte header-only file of a game with no row). Second start changed nothing in all 33 states. States and results:

| State | Startup | Resume |
|---|---|---|
| clean, applied / unapplied | `Reconciled` | writer, no rebase |
| torn tail, applied (store ahead) | `Rebased`, `StoreAhead`, old bytes in `damaged/` | same |
| torn tail, unapplied | `Reconciled`, tail truncated | writer, `torn = true` |
| last record cut cleanly, store ahead | `Rebased`, `StoreAhead` | same |
| damaged middle, applied / unapplied | `Rebased`, `Middle`, 4 / 1 moves kept | same |
| `UndecodableTail`, `ended_at` 0 and -5 (D-144) | `Rebased`, `UndecodableTail`, refused bytes kept in `damaged/` | same |
| header only, row empty | `Reconciled` | writer, `next_seq 1` |
| header only, row has moves | `Rebased`, `StoreAhead` | same |
| 0-byte file, row empty / row with moves | header rewritten / `Rebased`, `Middle at 0` | same |
| 20-byte file, bad magic, game uuid of another game, row with moves | `Rebased`, original kept in `damaged/` | same |
| bad magic, row empty | `Failed(unreadable)`, file untouched | `Err(Unreadable)` |
| foreign store header / version 2 | `Failed`, file untouched | `Err(ForeignStore)` / `Err(VersionMismatch)` |
| completed + intact terminal | `ClosedLeftover`, to `done/` | writer returned for `close_final` |
| completed + journal cut before `GameEnd` | `MovedAside`, `StoreAhead`, no rebase | `Ok(None)` |
| completed + damaged | `MovedAside` | `Ok(None)` |
| completed, no journal | nothing | `Ok(None)` |
| no row: header only / with records | `RemovedEmpty` / `NoGameRow`, file kept | `NotFound` |
| row with moves, no journal / row empty, no journal | nothing (scan does not visit) | rebased `StoreAhead` / header-only journal, `created_empty` |
| stale `.rebase` + journal | `.rebase` removed | same |
| `.rebase` only, incomplete | `IncompleteRebaseMovedAside`, bytes at `damaged/<uuid>.1.rebase`, game rebuilt | same |
| `.rebase` only, complete | renamed into place | same |
| `done/` and journal different file | `Failed(write failed)` every start | same (FIND-D4R-04) |

D-66 (`db_seq` against the journal, `StoreAhead`) behaved as 3.11.2 says in every row above. D3-10 (whole journal in one `apply_records` call) is in the code at every call site (`recover.rs` intact branch, rebase terminal branch) and the mutants that remove or shorten it (D27, D34) are killed.

## 4. Hand mutation: 71 valid new mutants over `recover.rs`, `files.rs`, `writer.rs`, `resume.rs`, `store/d4.rs`

Runner `scratchpad/rv_mut.py`; one mutant at a time; set: lib, `resume`, `recovery`, `journal`, `journal_faults`, `journal_crash`, `d4_faults`, `d4_verify`, `d4_verify_faults`, `recovery_kill` (default 3 rounds). First 25 mutants ran with `journal_race` too; the survivors of the second batch were re-run with `recovery_kill` included. Baseline green first. 81 patches written: 10 did not compile (unused variable or type) and are not counted: D06, D09, D18, D31, D41, D49, F06, F11, W05, R06. Result: **71 valid, 43 killed, 28 survived.**

| ID | Mutation | Result |
|---|---|---|
| D01 | `lock_open`: file identity check bypassed | KILLED (`lock_open_refuses_a_file_that_was_replaced...`) |
| D02 | identity `&&` -> `\|\|` | KILLED |
| D03 | B-R03 branch without the `file.is_some()` guard | SURVIVED, equivalent except in the race of FIND-D4R-01 |
| D04 | no directory sync after the B-R03 removal at startup | SURVIVED, gap (no sync fault test on this path) |
| D05 | B-R03 `same_file` negated | KILLED |
| D07 | `finished_close` ignored | KILLED |
| D08 | header-only removal for `<= HEADER_LEN` | SURVIVED, equivalent (`decode_header` fails on shorter) |
| D10 | `final_row` only for `Completed` | KILLED |
| D11 | `nothing_stored` with `\|\|` | KILLED |
| D12 | startup and resume swapped for the empty row | KILLED |
| D13 | `created_empty` false | KILLED |
| D14 | `short` for `<= HEADER_LEN` | KILLED |
| D15 | in-place header rewrite also for a final row | SURVIVED, gap |
| D16 | D-66 `>` -> `>=` | KILLED |
| D17 | D-66 `>` -> `<` | KILLED |
| D19 | `last_terminal` always true | KILLED |
| D20 | final branch `next_seq` off by one | SURVIVED (unobserved: the caller only closes) |
| D21 | intact branch `next_seq` off by one | KILLED |
| D22 | `scan_damaged` also true for `StoreAhead` | SURVIVED, equivalent (reconcile is damaged either way) |
| D23 | drop `db_seq == 0` from the Unreadable condition | SURVIVED, **gap** |
| D24 | Unreadable `&&` -> `\|\|` | KILLED |
| D25 | torn-tail truncation skipped | KILLED |
| D26 | `rows_to_delete_from` ignored | KILLED |
| D27 | journal never passed to the applier | KILLED |
| D28 | `Start` never synthesized | KILLED |
| D29 | `journal_last_ply` default 1 | KILLED |
| D30 | `.rebase` not removed after a failed step 2 | KILLED |
| D32 | `had_old` always true | SURVIVED, gap (`moved_aside` for a no-journal rebase) |
| D33 | `moved` negated | KILLED |
| D34 | rebased terminal file never finalized | KILLED |
| D35 | no `StoreAhead` default on the report | SURVIVED, gap |
| D36 | `last_damaged_name` lowest `k` | SURVIVED, **gap** (FIND-D4R-06) |
| D37 | `list_games` accepts non-canonical names | SURVIVED, gap |
| D38 | `list_games` accepts non-files | SURVIVED, gap |
| D39 | order key `-ended_at` | KILLED (CT-ST-084) |
| D40 | order key drops `ended_at` | KILLED |
| D42 | `Held` reported as `Queued` | KILLED |
| D43 | `Terminal(Queued)` reported `Finalized` | KILLED |
| D44 | `NeedsRecord` -> `ClosedLeftover` | KILLED |
| D45 | `GameParked` classification off | SURVIVED, gap |
| D46 | `scan_failed` false | SURVIVED, gap |
| D47 | parked lookup `!=` | KILLED |
| D48 | no-row error for startup instead of resume | KILLED |
| D50 | `torn_tail_dropped` false | KILLED |
| D51 | `apply_plan` result dropped | KILLED |
| F01 | `move_no_replace` as `rename` (replaces) | SURVIVED, **gap** vs CT-ST-089 |
| F02 | no `old_linked` failpoint | KILLED |
| F03 | window (e) `same_file` off | KILLED |
| F04 | no directory sync of `journal/` before `damaged/` | KILLED (`ct_st_089_both_directories...`) |
| F05 | `.rebase` complete without `intact_end == len` | SURVIVED, equivalent (torn or damaged also shows) |
| F07 | complete without the `damaged` test | SURVIVED, equivalent (same reason) |
| F08 | `running_side.is_some()` | KILLED |
| F09 | `Voided` not terminal | KILLED |
| F10 | stale `.rebase` kept | KILLED |
| F12 | aside loop `same_file` off | SURVIVED, gap (crash window of the aside link) |
| F13 | no `damaged/` sync in the aside path | SURVIVED, gap |
| F14 | `close_final` replaces a different file | KILLED |
| F15 | no `close_final.linked` failpoint | KILLED |
| F16 | no `done/` sync | KILLED |
| F17 | `FinishedEarlierAttempt` -> `Moved` | SURVIVED, equivalent (unobserved) |
| F18 | `.rebase` presence test negated | KILLED |
| R01 | `played_at` dropped in the resume load | KILLED |
| R02 | no-row load as `Corrupt` | KILLED |
| R03 | missing `store.id` becomes the nil uuid | SURVIVED, gap |
| R04 | synthesized `Start.custom_start` false | SURVIVED, **gap** |
| R05 | synthesized `Start.rated` true | SURVIVED, **gap** |
| W01 | `LOCK_GRACE_TRIES = 0` | SURVIVED, **gap** (FIND-D4R-09) |
| W02 | grace never retries | SURVIVED, gap |
| W03 | `create` removes the file on `WouldBlock` | SURVIVED, **gap** |
| W04 | `probe` open error -> `Free` | SURVIVED, gap |
| W06 | grace starts exhausted | SURVIVED, gap |

(Counts are from `rv_mut_results.json`: 43 killed, 28 survived, 10 did not compile. D20 appears once in the table.) Equivalent or unobservable: D03, D08, D22, F05, F07, F17 and D20 as far as I can see; the other 21 survivors are gaps.

## 5. Conformance statement

Standard: none claimed. Project rule is SEI CERT by intent only; **no conformance claim is made here**. Language: Rust 1.99.1, edition 2024.

| Category | Result |
|---|---|
| Tool-checked | `rustfmt --check`, `clippy -D warnings` (both feature settings) clean; `forbid(unsafe_code)`; no new dependency. |
| Manually reviewed | the scope files line by line; every `unwrap`, `expect`, index and `as` in them (none outside tests); error `Display` text for paths and ids; the failpoint registry against call sites (all 14 `journal.*` names are registered and wired, `journal.close_final.linked` included). |
| Not checked | `cargo deny` / `npm audit` (no dependency change); Miri; sanitizers; Intel macs; any real power loss (SIGKILL does not drop the page cache, so fsync ordering is shown only by the sync-order tests and by reading); a real `SQLITE_SCHEMA` or similar transient; a volume other than the one here. |
| Deviations | none recorded; I approve none. |

## 6. Gate check

D4 touches the journal interface (3.7.3), the data directory layout (`damaged/<uuid>.<k>.rebase`), a stored/observable outcome set and the open path: G3 is right. Evidence the gate has: this review. Evidence it lacks: no named independent reviewer other than this desk review; Brian's approval. The signature conflict (FIND-D4R-08) needs his decision before the SDD can be amended.

## 7. Conditions before GATE-112 can be approved

1. FIND-D4R-01 closed with a test that fails today without a sleep in the library, or accepted by Brian by name as a risk.
2. FIND-D4R-08: Brian decides the `open_for_resume` signature and `Settled` and friends, and the SDD is amended.
3. FIND-D4R-02, -03, -05, -06 fixed or deferred with an `OQ-` id; FIND-D4R-07 recorded as an NFR-1 risk.
4. FIND-D4R-09 and -10: at least W01/W03 (B-R04), R04/R05 (synthesized `Start`), F01 (no-replace), D23 and D36 get a test that kills them.

## 8. Not run, not analyzed, assumed

- Not run: the FIND-D4R-01 race without an inserted sleep; a false `Locked` from a scanner holding a fresh file (not reproduced); a journal that parks (FIND-D4V-04); power-loss fsync ordering; Intel; sanitizers; Miri; `cargo deny`.
- Not analyzed: the app layer's use of `RecoveryOutcome`; D-78 seed after rebase; the rating code beyond what `rv_two_processes_open_at_once` compared.
- Assumed: macOS APFS on this machine behaves like the target volume for `flock` and `link`; `[UNV]` elsewhere.
- My test files (`rv_d4_matrix.rs`, `rv_d4_files.rs`, `rv_d4_proc.rs`, `rv_d4_extra.rs`) live only in the `-atk` and `-atk2` scratchpad copies and are not in the repository.
