# Review: chess-store part D4, the fixes of 2026-11-25 (limited, adversarial, fresh)

| Field | Value |
|---|---|
| Stance | **Reviewer** only, fresh invocation. I did not write the code, the fixes or the first review (`review-chess-store-D4.md`, read for what was found). I fixed nothing, edited nothing in `crates/` or in any other document, committed nothing, closed no bead. |
| Gate | GATE-112 (D4), tier **G3**, status **PENDING (Brian Prascher)**. This review approves nothing. |
| Baseline | B6. Fixes are in `1b0faa9` (merge of `feat/step2-store-d4-fixes`, fix commit `6c90c71`). |
| Snapshot | Copy made **2026-11-25T21:43:55Z** from the working tree (HEAD `1b0faa9` then). Tree hash of `crates/chess-store` excluding `proptest-regressions` (`find . -type f \| sort \| xargs shasum \| shasum`): **`0499d0ac24d1ec30`**. At 22:43Z the live tree was at HEAD `0e4bf6c`; `diff -rq` against the snapshot shows `crates/chess-store/src` identical and only `tests/boundary.rs` (modified) and `tests/tie_break.rs` (new) different. Those are the other engineer's GATE-110 work and are not part of this review. Copies are under the session scratchpad, named `rv-d4f-*`: `-45465-1795643035` (attack copy: my seams and tests), `-clean` (unmodified, all [RAN] results below), `-inv` (attack copy plus a backtrace hook), `-mut`, `-mut2` (mutation). Each has its own `CARGO_TARGET_DIR`. I deleted nothing I did not create and left no background process. |
| Scope | The changes of `6c90c71` in `journal/recover.rs`, `journal/files.rs`, `journal/rebase.rs`, `journal/writer.rs`, `store/d4.rs`, `repo/apply.rs` (`with_busy`), `fault.rs`, and the changed test `recovery.rs::b_r03_a_different_file_in_done...`. Against the engineer's "Fixes 2026-11-25" in `step2-store-D4-evidence.md`. |

## 0. Verdict

| Item | Verdict |
|---|---|
| **GATE-112 (D4), as to these fixes** | **FAIL.** Two Majors of the same family as FIND-D4R-01 are reproduced, and the spurious-error problem the engineer parked as FIND-D4F-02 is worse than described. |

Why FAIL and not PASS WITH CONDITIONS. FIND-D4R-01 was Major because the single-writer rule was broken and an acknowledged record could end up outside the live journal. The fix closes the window the first review demonstrated (a second resumer moving a live, locked journal aside): that part holds, in 1 200 real-process rounds with a live writer and in the deterministic harness. But the new claim mechanism (an exclusive `.rebase`) has a second half that is still open:

- **FIND-D4F-R01 (Major).** A resumer whose `.rebase` was settled away by another resumer can wake, take the lock of a file that now sits in `damaged/`, and be handed a writer on that file while `<uuid>.jnl` is another file. Its acknowledged appends go to `damaged/<uuid>.1.rebase`. Reproduced deterministically with a seam and in real two- and three-process rounds with the same seam. The first half of the interleaving (another process locks and settles the new `.rebase` before its creator locks it) happens **naturally**, with no seam, about 2 times in 1 000 two-process rounds.
- **FIND-D4F-R02 (Major).** The claim protects only against a later `<uuid>.jnl`. It does not protect the plan, which is derived from a row read **before** the claim. A resumer stalled between its row read and its claim can run step 2 and hand out a writer on a game that has been finished and closed meanwhile.

I do not know the natural probability of R01's second half or of R02: both need a process stalled for milliseconds (R01) or for a whole game (R02) at a given instruction. I say so plainly. They are still the invariant the gate exists for ("at most one live writer per game; every acknowledged record stays in `journal/`, `damaged/` or the DB"), and the fix claims the family closed. If Brian accepts R01 and R02 as risks by name, read this as PASS WITH CONDITIONS with R01 and R02 as accepted risks. That is his decision, not mine.

What is sound, `[RAN]`: the fix of D4R-01 against the originally reported window; D4R-02 (loser gets `Locked`, mostly); D4R-03; D4R-04 for the states I tried; D4R-06; D4R-12 header check. Gaps and smaller problems follow.

## 1. Commands re-run `[RAN]` (clean copy, snapshot above)

All on the unmodified `-clean` copy. Other work (my attack runs, mutation) shared the machine during these runs; no failure appeared, so no timing flake needs excusing.

| Check | Command | Result |
|---|---|---|
| fmt | `cargo fmt -p chess-store --check` | exit 0, no output |
| clippy, default | `cargo clippy -p chess-store --all-targets --offline -- -D warnings` | exit 0 |
| clippy, test-support | same with `--features test-support` | exit 0 |
| tests default x3 | `cargo test -p chess-store --offline --no-fail-fast` | 3 of 3 exit 0; run 1 **532 passed, 0 failed** (the engineer's number) |
| tests test-support x3 | same with `--features test-support` | 3 of 3 exit 0; run 1 **637 passed, 0 failed** |
| journal_race x5 | `cargo test -p chess-store --features test-support --offline --test journal_race` | 5 of 5 `4 passed; 0 failed` (16.9, 23.5, 25.4, 21.0, 18.6 s) |
| recovery_kill | `CHESS_STORE_KILL_ROUNDS=100 cargo test -p chess-store --features test-support --offline --test recovery_kill -- --test-threads=2` | `7 passed; 0 failed`, 450.6 s |
| no unsafe, no new dependency | `lib.rs` `#![forbid(unsafe_code)]`; `Cargo.toml`/`Cargo.lock` not in the fix diff (`git diff --stat 0b0a47d 1b0faa9`: 7 source files, none a manifest) | yes |
| production cost of the seams | `fault::check` is `#[inline(always)]` returning `Ok(())` without the feature `[OBS fault.rs:541-550]` | none in a release build |

The suite is green. The findings below are what the suite does not see.

## 2. How I attacked it

I added, **in the scratch copy only**, seven seams (`rv.row_read`, `rv.rebase.created`, `rv.before_step2`, `rv.lock_open.opened`, `rv.prepare.between`, `rv.settle.start`, `rv.swap.before_link`) next to the engineer's two, and a test file `rv_f.rs` (in-process deterministic harness `rvf_s1..s4`; a child-process driver with roles "resume" and "scan"; random delays 0 to 200 ms at 0 to 3 seam points per party per round; per-round invariants). Roles and rounds:

| Race | Actors | Rounds | Seams |
|---|---|---|---|
| resume / resume | 2 processes | 300 | random, all 13 names |
| resume / scan | 2 processes (the scan is a full `Store::open`) | 300 | random |
| resume / resume / scan | 3 processes | 300 | random |
| resume / scan / scan | 3 processes | 300 | random |
| **same four, no seams at all** | as above | 300 each | none (only the 0 to 25 ms start offsets) |
| resume / resume, no seams | 2 processes | 1 000 (run once more, with a backtrace hook) | none |
| live writer vs 2 resumers + 1 scan | 4 processes, writer appends 3 records per round | 300 | random |
| closer (`close_final` of a finished game) vs resume + scan | 3 processes, a new game each round | 300 | random |

Invariants per round: at most one writer; the writer's append is visible in `<uuid>.jnl` (not in another inode); `<uuid>.jnl` has link count 1 and scans clean; no `.rebase` left; no `damaged/` file for this game (nothing in these states is damaged); only `Ok` or `Locked` from a resume, no `Failed` from a scan; a journal exists at the end.

## 3. Findings, most severe first

### FIND-D4F-R01 [MAJOR][CONFIRMED with a seam] A creator that was settled away wakes up, locks a file in `damaged/`, and returns a writer on it
- Location: `rebase.rs::write_rebase_file` (`create_new` at the top, `try_lock` after it), `recover.rs::settle_and_read` (a resumer that locks a `.rebase` settles it), `recover.rs::rebase` (the re-check looks only at the name `<uuid>.jnl`), `rebase.rs::swap` (links by **name**: `hard_link(&new, &jnl)`).
- Mechanism `[OBS]`. A creates `<uuid>.jnl.rebase` (0 bytes) and has not yet locked it (the engineer states this gap exists, D4F-01 (b): flock cannot be taken at creation without `libc`). C finds the `.rebase`, opens it and takes its lock, reads the row, and settles it under that lock: the file is 0 bytes, so it is "incomplete" and goes to `damaged/<uuid>.1.rebase`; C drops that lock. C builds its own `.rebase` (a new inode). A wakes: `try_lock` on **its** inode, now in `damaged/`, succeeds; A writes its bytes there and syncs. A's re-check `exists(<uuid>.jnl)` is false if C has not swapped yet. A runs step 2, then `swap` links the name `<uuid>.jnl.rebase`, which is now **C's** file, as `<uuid>.jnl`, and removes the name. A returns `JournalWriter::from_locked(<A's inode in damaged/>, <uuid>.jnl, ...)`. C's swap meets an existing `<uuid>.jnl` and answers `Locked`.
- Evidence `[RAN]` `rvf_s2b_creator_wakes_after_its_file_was_moved_aside` (seam `rv.rebase.created` 800 ms in A, `rv.before_step2` 1 200 ms in C):
  ```
  A: Ok(writer, rebased=true)    C: Err(Locked)
  <uuid>.jnl 835 bytes; damaged/<uuid>.1.rebase 835 bytes
  A writer live? Err("writer appended seq 6 but the live name ends at Some(5)")
  after: <uuid>.jnl 835 bytes; damaged/<uuid>.1.rebase 909 bytes
  ```
  A's acknowledged record (seq 6) is in `damaged/<uuid>.1.rebase`, not in the live journal. Real two- and three-process rounds with the same seam found it three times in about 1 200 seamed rounds: "parent writer not live: writer appended seq 6 but the live name ends at Some(5)" (resume/scan round 96 and 117, resume/resume/scan round 49).
- Natural reach: the first half (C locks and settles A's fresh `.rebase`) occurs without any seam, about 2 in 1 000 unseamed two-process rounds (`damaged/<uuid>.1.rebase` with 0 or 835 bytes left behind, writer still live in those two rounds). The second half needs A descheduled for roughly the time C needs for an open, a flock, a database read and a rename: milliseconds, between two adjacent syscalls. `[UNV]` how often on a loaded machine; I did not reproduce it without the seam.
- Impact: two actors each believe they hold the game; the returned writer appends to a file nothing reads. If the process dies after the journal ack and before the database apply, the move exists only in a file named `...jnl.rebase` under `damaged/`. That is the failure D4R-01 was Major for.
- Close by (not done by me): make the claim unambiguous between create and lock (create under a private name, lock it, then `hard_link` it to `.rebase` without replacing; or compare `fstat` and `stat` identity after `try_lock` and answer `Locked` if the name is not the file), and have `swap` check that the name it links is the inode it holds. Verify with `rvf_s2b` returning `Locked` for A and with the three-process rounds at zero "writer not live".

### FIND-D4F-R02 [MAJOR][CONFIRMED with a seam, low likelihood] The claim does not freshen the row: a stalled resumer rewrites a finished game and returns a writer for it
- Location: `recover.rs::recover_game_once` (row read) and `recover.rs::rebase` (claim, re-check, step 2); `repo/apply.rs::apply_reconcile_plan` has no status or `journal_seq` guard `[OBS apply.rs:470-505]`.
- Mechanism: B reads the row (in progress, 3 moves, no journal), then stalls. A resumes, rebases, plays to the end, applies the `GameEnd`, `close_final` moves the journal to `done/`. B wakes: `create_new(.rebase)` succeeds (A removed its own), the re-check finds no `<uuid>.jnl` (it is in `done/`), B runs step 2 with the **stale plan** and stale `journal_seq`, and links a new `<uuid>.jnl` from the stale rows.
- Evidence `[RAN]` `rvf_s1_stale_row_after_the_game_ended` (seam `rv.row_read` 2 500 ms in B), 3 of 3 runs:
  ```
  DB after A closed: status="completed" journal_seq=6 ... ended_at=Some(9000000)
  B: Ok(writer, rebased=true)
  DB after B:        status="completed" journal_seq=5 ...
  tree: <uuid>.jnl 835 bytes (new), done/<uuid>.jnl 1073 bytes
  ```
  B holds a writer for a completed game; the stored `journal_seq` of a completed game went from 6 to 5 (the plan can also set clocks; here they happened to be equal). The game now has a journal in `journal/` and one in `done/`: `close_final` will refuse with `AlreadyExists` and every start will report `DoneNameCollision` until a person acts.
- Likelihood: B must be stalled between its row read and its claim for the length of another process's whole game (SIGSTOP, a suspended machine). The window in B between the read and the claim is otherwise a `build_rebased` and two fsyncs. I do not claim it happens in ordinary use.
- Close by: re-read the row under the claim (or make step 2 conditional on `status = in_progress` and the `journal_seq` that was read); verify with `rvf_s1` answering `Ok(None)` or `Locked`, no write to the row.

### FIND-D4F-R03 [Minor, upgraded from the engineer's FIND-D4F-02][CONFIRMED, natural] Two resumers can both fail, and the failure is `WriteFailed{NotFound}`, not `Locked`
- Evidence `[RAN]` with **no seam**: `rvf_race_two_resumers`, 400 rounds: 1 violation; 1 000 rounds: 2 violations; (`child: WriteFailed{Io(NotFound)}`, `parent: Locked`, no writer, no journal file; and once the roles swapped). Backtrace (hook in the `-inv` copy, `bt.txt`): `settle_and_read` -> `settle_rebase_files_at` -> `io_err` on `NotFound`. A resumer locked the creator's `.rebase`; the creator lost its `try_lock` and removed the name; the file vanished between `symlink_metadata` and `fs::read` (or the move) **inside** `settle_rebase_files_at`. The engineer's fix tolerates a vanished file only at the later `fs::read(<uuid>.jnl)`.
- The engineer's note says both resumers are "told `Locked`". It is `WriteFailed`, which `chess-app` reports as a disk fault. Nothing is lost (rows and any journal are as they were; a retry works), but the invariant "one loser gets `Locked` and no spurious `WriteFailed`" does not hold, and their 30-round test passes by luck (about 1 in 400 per round).
- The startup scan sees the same family: a scan child ended `Failed("the journal write failed")` in a seamed round (resume/scan/scan round 282).
- Close by: map `NotFound` inside `settle_rebase_files_at` under a lock to "no file" (as `settle_and_read` already does one step later); verify with 2 000 unseamed rounds at zero.

### FIND-D4F-R04 [Minor][CONFIRMED, natural] A benign race writes a false damage record: `damaged/<uuid>.k.rebase` and `IncompleteRebaseMovedAside`
- Evidence `[RAN]`: unseamed two-process rounds 32 and 208 of 1 000 left `damaged/<uuid>.1.rebase` (0 or 835 bytes) with a healthy game; a scanner with no seam reported `RecoveryOutcome::IncompleteRebaseMovedAside` for a game that was only being rebuilt by a resumer (resume/scan round 9, 3-process rounds 76, 88, 98). Nothing is deleted (bytes kept), the game is intact, but the outcome tells the app a rebase was left incomplete and a person finds a stray file. Same root as R01 (a locker settles a creator's fresh, empty `.rebase`). Close with R01.

### FIND-D4F-R05 [Minor][CONFIRMED] A resumer is told `Locked` when nobody holds the game, and it is not retried
- Evidence `[RAN]` `rvf_s3_lock_open_gap_spurious_locked`: B opens the damaged `<uuid>.jnl`, pauses; A resumes fully (the old file goes to `damaged/`, A's writer is dropped). B locks the old inode (free), the identity check sees another file at the name, B answers `Locked`. `recover_game` retries only on `Ok(None)` (`RACE_ATTEMPTS`), not on this `Locked`. Natural cases: unseamed 3-process round 266 (resume `Locked` while two scans finish and hold nothing). `lock_open` is documented to answer `Locked` for this case, so it is a design choice, but the user-visible message ("the game is open elsewhere") is false; the retry would succeed. Not an integrity problem.

### FIND-D4F-R06 [Minor][CONFIRMED] The `journal_seq` witness rejects a complete `.rebase` whenever the row is ahead of the file
- Evidence `[RAN]` `rvg_witness_row_ahead_of_a_complete_rebase`: a journal whose records include `Clock`s gives `journal_seq` 7; the from-store rebuild yields a file whose last `seq` is 5 (Start, three moves, closing Clock) and step 2 sets `journal_seq` to 5. Put the complete file back as `.rebase` with the row at 7 (a crash between step 1 and step 2 of a no-journal rebase): resume moves it to `damaged/<uuid>.1.rebase` as incomplete and rebuilds. Outcome safe (the bytes are kept and the rebuilt file is the same), but a complete file is classified as damage, and `journal_seq` above the rebuilt file's last `seq` is the normal case for any game with `Clock` records.
- Soundness of the witness in the other direction: for a damaged-journal rebase the old `<uuid>.jnl` exists until step 3, so a `.rebase` with no `.jnl` is always after step 2 (row = file's last `seq`, so a cut file is below it and refused). For a store rebuild the file has one `Clock`, at the end, so a cut file never ends at a stopped `Clock`. By reading, the witness is sound where the shape is ambiguous; it is not a complete test where the row is ahead. The engineer's unit sweep (`only_the_whole_rebase_is_complete_at_every_byte_offset`) passed in my runs; I did not write a second sweep. For a **final** row the floor is not applied (`None`); mutant N07 (floor for final rows too) survives, and I see no flow that makes a `.rebase` for a final row `[INF]`.

### FIND-D4F-R07 [Minor][CONFIRMED by mutation] Test gaps left by the fixes
Surviving mutants that are real gaps (section 5): the synthesized `Start.matchup` (N10); the length of the recovery budget (N12: 2 s instead of 10 s is not noticed); a create error of any kind turned into "retry" (N13); no directory sync after the stale-`.rebase` removal and after the install (N27, N28); `resume_row` and `apply_reconcile` not waiting for the game's queued batch (N41, N42: D3-09 at the two new call sites); N03 (the `done/` check without the journal name).

### FIND-D4F-I01 [Info] D4R-07: measured open latency under a held database lock is 10.3 s
`[RAN]` `rvh_budget_three_games_lock_held_long_and_open_latency` (three damaged journals, another connection holds `BEGIN IMMEDIATE` until released): `Store::open` took **10.34 s**; all three games `Failed("the store is busy")`; journals untouched. Later games starve by design once the budget is spent (the third failed at once); each is retried at the next start or by its own resume (no deadline, 5 s wait). The 10 s is an `ESTIMATE` and nothing pins it (N12 survives). With a held lock the user waits about 10 s at every start. Not checked against an NFR number; I did not look for one.

### FIND-D4F-I02 [Info] D4R-03/04: the `done/` name states
`[RAN]` `rvg_done_name_matrix`: `done/<uuid>.jnl` as directory, FIFO, unix socket, symlink to the journal, dangling symlink, and a different regular file, each against an in-progress intact journal, an in-progress damaged journal and a final intact journal (18 cases): startup `DoneNameCollision` in all 18, journal bytes unchanged in all 18, resume succeeded in all 18 (writer; rebased for the damaged one), `close_final` refused in all 18 with `WriteFailed{Io(AlreadyExists)}` and touched nothing. No state I tried leaves a game unresumable. Cost: such a game can never close, and the scan reports `DoneNameCollision` at every start with no escalation. (My first run hung because my own test helper read the FIFO; not a store defect.) Not tried: `done` itself being a regular file; `<uuid>.jnl` or `.rebase` being a directory or a symlink. `lock_open` still uses `fs::metadata` (follows links) while `same_file` was moved to `symlink_metadata`: an inconsistency I did not exercise.

### FIND-D4F-I03 [Info] Public surface
`RecoveryOutcome::DoneNameCollision` is a new variant of an enum that is `#[non_exhaustive]` `[OBS recover.rs:105]`: not a breaking change. The two seams are added to `CT_NAMED_FAILPOINTS` (not in the SDD list; the engineer reported it). `settle_rebase_files` keeps its public signature and is callable without a lock; its doc says the caller holds the lock. FIND-D4R-08 (the `open_for_resume` signature, SDD amendment) is untouched and still needs Brian.

### The changed test
`recovery.rs::b_r03_a_different_file_in_done_is_damage_and_nothing_is_touched`: the assertion changed from `Failed{..}` to `assert_eq!(.., Some(&DoneNameCollision))`; the byte and status assertions are unchanged `[OBS git diff 0b0a47d 1b0faa9]`. It is stronger, not weaker, and it matches the code.

## 4. What held `[RAN]`

| Claim | Result |
|---|---|
| A live, locked journal is never moved or removed by resumers or scanners | 300 rounds, 4 processes, random seams: 0 problems; inode and link count unchanged, writer's 900 appends all in the live file |
| A closer against a resumer and a scanner | 300 rounds, new game each round: 0 problems, journal in `done/` once, nothing left in `journal/` |
| D4R-01 original window | `d4r_01_*` pass; mutants N01, N14, N33, N34, N36, N37b kill |
| No `.rebase` left behind | 0 of about 5 000 seamed and unseamed rounds |
| At most one writer | 0 rounds with two writers in every configuration above, seamed or not (the R01 case is "one writer on the wrong file", not two) |
| Rounds with no winner | unseamed: 2 in 1 000 (two-process resume), 1 in 300 (three-process); each a `Locked` loser against a scanner that had finished, or R03 |

## 5. Hand mutation: 51 patches, 42 valid, 28 killed, 14 survived

Runner `rvm_run.py`/`rvm_run2.py` (scratchpad), one mutant at a time over copies of the clean tree. Test set per mutant: `--lib`, `d4_fixes`, `d4_fixes_faults`, `recovery`, `resume`, `d4_verify`, `d4_verify_faults`, `journal`, `journal_faults`, `d4_faults`, features `test-support`. **Not in the set: `recovery_kill`, `journal_race`, `coverage`** (so a survivor may be killed there; the 100-round `recovery_kill` is a green separate run). 9 patches did not compile and are not counted (N09, N11, N15, N19, N19b, N20, N37, N43, N43b); N09b, N11b, N20b and N37b are their rewrites and compile. N21 was dropped before running. New mutants only: none duplicates one of the engineer's M01..M29 or the first review's D/F/R/W list.

| ID | File | Mutation | Result |
|---|---|---|---|
| N01 | recover | `RACE_ATTEMPTS` 3 to 1 | KILLED `d4r_01_a_resumer_that_meets_a_created_but_free_journal_resumes_on_it` |
| N02 | recover | `DoneCollision` for resume instead of startup | KILLED `d4r_03_close_final_refuses_a_dangling_symlink...` |
| N03 | recover | done check without `exists(&jnl)` | **SURVIVED, gap** |
| N04 | recover | identity check ignores device | SURVIVED, equivalent on one volume |
| N05 | recover | vanished locked file is an error again | KILLED `settle_and_read_treats_a_locked_file_that_vanished_as_no_file` |
| N06 | recover | aside-incomplete keeps the lock handle | SURVIVED, equivalent (the `NotFound` arm drops it) |
| N07 | recover | witness floor also for final rows | SURVIVED, unreachable `[INF]` |
| N08 | recover | `header_names` `&&` to `\|\|` | KILLED `d4r_12_a_header_only_file...` |
| N09b | recover | synthesized `Start` wall time +1 | KILLED `an_unreadable_header_is_rebuilt_from_the_row...` |
| N10 | recover | matchup arguments swapped | **SURVIVED, gap** |
| N11b | recover | wait cap 500 s | KILLED `the_budget_is_passed_on_and_a_spent_one_is_busy` |
| N12 | recover | budget 2 s | **SURVIVED, gap** |
| N13 | recover | any create error becomes "retry" | **SURVIVED, gap** |
| N14 | recover | recheck on `.rebase` and `&& false` (never fires) | KILLED `d4r_01_a_second_resumer_with_no_journal_never_moves...` |
| N16 | recover | `report.rebased` false | KILLED `an_unreadable_header_with_a_journal_seq...` |
| N17 | recover | `Locked` reported `Queued` by the scan | KILLED `a_journal_with_a_live_lock_is_skipped` |
| N18 | recover | `lock_open`: missing file is an error | KILLED `lock_open_hands_back_the_file_it_locked...` |
| N20b | recover | header-only removal without the header check | KILLED `d4r_12_...` |
| N22 | files | `same_file` `&&` to `\|\|` | KILLED `same_file_is_true_only_for_one_regular_file...` |
| N23 | files | `same_file` ignores device | SURVIVED, equivalent on one volume |
| N24 | files | `bytes.len() < HEADER_LEN` for the complete test | SURVIVED, equivalent (no records means not complete anyway) |
| N25 | files | witness `None` means incomplete | KILLED `only_the_whole_rebase_is_complete_at_every_byte_offset` |
| N26 | files | `Abort` not a rebase end | KILLED `a_rebase_is_complete_when_it_ends_in_a_terminal...` |
| N27 | files | no dir sync after stale `.rebase` removal | **SURVIVED, gap** |
| N28 | files | no dir sync after install | **SURVIVED, gap** |
| N29 | files | no syncs after aside of an incomplete rebase | KILLED `the_aside_move_of_an_incomplete_rebase_syncs_the_damaged_directory` |
| N30 | files | aside never finishes an earlier attempt | KILLED `the_aside_move_of_an_incomplete_rebase_finishes_an_earlier_attempt` |
| N31 | rebase | failed rebase write leaves the file | KILLED `a_failed_rebase_file_write_removes_the_rebase` |
| N32 | rebase | no dir sync of the new `.rebase` | KILLED `a_full_swap_makes_four_directory_syncs...` |
| N33 | rebase | refused swap keeps its `.rebase` | KILLED `a_swap_without_a_hold_on_the_journal_refuses...` |
| N34 | rebase | link refusal keeps its `.rebase` | KILLED `d4r_01_the_link_of_the_new_journal_never_replaces...` |
| N35 | rebase | no dir sync after removing `.rebase` | KILLED `a_full_swap_makes_four_directory_syncs...` |
| N36 | rebase | writer `next_seq` off by one | KILLED `d4r_01_a_second_resumer...` |
| N37b | rebase | writer `acked_len` +1 | KILLED `d4r_01_a_second_resumer...` |
| N38 | writer | grace step 0 | KILLED `b_r04_the_grace_lets_create_in_after_a_short_holder` |
| N39 | writer | grace 4 tries | KILLED `b_r04_a_holder_that_outlasts_the_grace_keeps_its_file` |
| N40 | writer | grace one more try | SURVIVED, equivalent in effect (boundary of a 80 ms wait) |
| N41 | d4 | `resume_row` skips the game's flush | **SURVIVED, gap** |
| N42 | d4 | `apply_reconcile` skips the game's flush | **SURVIVED, gap** |
| N44 | recover | finish close only at startup | KILLED `d4r_04_a_resume_finishes_a_close_for_a_final_game` |
| N45 | recover | empty row only for resume | KILLED `b_r05_an_incomplete_rebase_for_a_final_or_an_empty_row...` |
| N46 | recover | never damaged | KILLED `a_final_game_with_a_damaged_journal_is_moved_aside_at_startup` |

Totals from `rvm_results.json` and `rvm_results2.json`: 42 valid (28 killed, 14 survived); survivors: 5 equivalent in effect (N04, N06, N23, N24, N40), 1 unreachable (N07), 8 real gaps (N03, N10, N12, N13, N27, N28, N41, N42). Kill rate 67 %, 78 % of the non-equivalent ones. The fix tests kill most mutants through deterministic seams, which is good. No test drives the R01 and R02 windows, so none of these mutants says anything about them.

## 6. Conformance statement

Standard: none claimed. The project applies SEI CERT by intent only; **no conformance claim is made.** Rust 1.99.1, edition 2024.

| Category | Result |
|---|---|
| Tool-checked | `rustfmt --check`, `clippy -D warnings` (both feature settings) clean; `forbid(unsafe_code)`; no dependency change |
| Manually reviewed | the diff of the six source files line by line; every filesystem step of resume, startup, rebase swap and `close_final` for interleavings (create, link, rename, remove, lock, unlock, fsync points); error mapping of `NotFound` and `AlreadyExists`; the changed test |
| Run as evidence | the races and harnesses in section 2 (scratch, not in the repository) |
| Not checked | `cargo deny`/`npm audit` (no dependency change); Miri; sanitizers; Intel; real power loss (SIGKILL keeps the page cache; fsync ordering is shown only by the sync-order tests); a volume other than this APFS one (flock and `link` semantics `[UNV]` elsewhere); `chess-app`'s use of the outcomes; a second sweep of every byte offset of `rebase_is_complete`; a journal or `.rebase` that is itself a directory or symlink; a database lock held before the process registration; llvm-cov for the new code |
| Deviations | none recorded; I approve none |

## 7. Gate check

D4 stays G3 (stored formats, data directory layout, observable outcomes). Evidence the gate has: this review and the first one. Evidence it lacks: Brian's approval, and a close of R01 and R02 or their acceptance by name. Conditions I would put on an approval:

1. FIND-D4F-R01 and R02 closed with tests that fail today (`rvf_s2b`, `rvf_s1` are ready to be copied in; they use two extra seams that the engineer would add), or accepted by Brian by name with the likelihood statement above.
2. FIND-D4F-R03, R04: closed (they share a root with R01 and one missing `NotFound` mapping), verified by at least 2 000 unseamed two-process rounds at zero violations and the three-process rounds at zero "writer not live".
3. FIND-D4F-R07: the eight gaps get a test or a recorded reason (N41/N42, N10, N13 first).
4. FIND-D4R-08 (signature and SDD amendment) is still open from the first review.

## 8. Not run, not analyzed, assumed

- Not run: R01 without a seam (natural reach unknown); R02 without a seam; recovery_kill with more than 100 rounds; the mutants against `recovery_kill` and `journal_race`; any run on a network or non-APFS volume; sanitizers; Miri.
- Not analyzed: the app layer; D-78 seed after rebase; ratings; the database lock behaviours other than the held `BEGIN IMMEDIATE` in FIND-D4F-I01.
- Assumed: the previous review's R-numbering (D4R) is unchanged; my D4F-Rxx IDs are new and do not reuse the engineer's FIND-D4F-01..04.
- My test files (`rv_f.rs`, `rv_g.rs`, `rv_h.rs`) and the seams live only in the scratchpad copies, under `rv-d4f-*`, and are not in the repository.
