# Decision package: chess-store rework findings that need a ruling (bead chess-a0r.3.50, baseline B6)

| Field | Value |
|---|---|
| Document | docs/engineering/decision-package-store-rework-findings.md |
| Author | System architect agent, Architect stance, 2026-11-22 |
| For | Brian Prascher (G2 and G3 approver). **Nothing here is decided.** One recommendation per item; Brian rules |
| Sources | [review-chess-store-rework-1.md](review-chess-store-rework-1.md) (FIND-RW1-nn), [review-chess-store-D1-D2-D5.md](review-chess-store-D1-D2-D5.md) (FIND-D2Rnn, D5Rnn, D1Rnn), [step2-store-B-evidence.md](step2-store-B-evidence.md) rework section (FIND-ST-RW-nn), [store-rework-specs.md](store-rework-specs.md), SDD-chess-store 3.9.1 notes 21 to 26 |
| Gate of this document | G0 (text only). The items below carry their own class |
| Evidence tags | `[OBS]` read, `[RAN]` executed by me, `[INF]` inferred, `[UNV]` not verified, `ESTIMATE` a number I chose or scaled |
| Review stance | Architect (synthesis). Not reviewed by the Board stance |

## 0. Recommendations at a glance

| Item | Finding | Recommendation | Gate class | One-way at first release? | Confidence |
|---|---|---|---|---|---|
| (a) | FIND-RW1-01 (Major), also removes the cause of RW1-02 | Split "traces" in two classes. Provenance traces (journals, backups, backup files) refuse as now. The SQLite sidecars `-wal` and `-journal` stop being traces on their own; they are judged by what SQLite can read. Exception kept: a zero-length `chess.db` beside a non-empty `-wal` is still refused | **G2** (narrows D-126; Brian's ruling) | The recognized states are one-way (D-126 reading); the code is two-way | Medium (High on diagnosis) |
| (b) | FIND-D2R-02 | `finalize_game` ORs: stored `analysis_used` = old OR request. A latch that never clears | **G3** (stored value meaning) | Yes | High |
| (c) | FIND-D2R-03 | Store validates shape only (`ended_at > 0`, non-empty `termination`). Order of times, journal sequence and who may abort are not store rules | G1 (inside the crate) plus one note for the app SDD | No | Medium |
| (d1) | FIND-D5R-03 | Order stats and list queries by `(started_at, id)`; `limit` means "oldest N by start". No "newest N" until a consumer needs it | G1 | No | Medium |
| (d2) | FIND-D5R-02 | Do not add `variation(game_id)` now. Revisit on a stated trigger | G3 if added | No (an index is an additive migration) | Medium |
| (e) | FIND-D1R-01 | D1 follow-up: `backup.after_copy`. D2 follow-up: three sites at the entry and tail of `finalize_game` and `reset_player`. D3: `apply.*`. D4: `journal.rebase.db_committed`. C: `restore.*`. Add a registry test that every name is wired or listed as pending with an owner | G1 | No | Medium |
| (f) | FIND-RW1-03 / ST-RW-04 | Make the rebase idempotent: omit the closing `Clock` when the last record is already that same `Clock`. Spec 3.3 (e) stays and gets its exact rule | **G3** (rebase output content) | Yes | High |
| (g) | FIND-ST-RW-01, 02, 03, 05 | Enter the four corrections as dated notes. ST-RW-01, 03: facts. ST-RW-02: goes away if (a) is accepted. ST-RW-05: accept the as-built reading | G0 text; ST-RW-05 is G3 and goes to Brian | ST-RW-05 yes | High (01, 03), Medium (05) |

Two items need a decision before the engineer's current fix of RW1-01/02 is final: (a) changes what the fix should do. See section (a) "Timing".

---

## (a) FIND-RW1-01: a first launch killed during creation is refused as `Missing`

### Facts

- `[OBS]` Reviewer, 2 runs of 120 random SIGKILLs during `Store::open` on a fresh directory: 12 and 9 refusals. Two states:
  1. `chess.db` 4 096 bytes, no tables, `user_version` 0, plus a non-empty `chess.db-wal` (32 bytes = bare header, 4 152, or 24 752 bytes), `-shm`, `store.id`. SQLite sees no schema. Answer `Missing` after 1.03 to 1.08 s (the `WAL_SETTLE` wait).
  2. A zero-length `chess.db` plus a non-empty `chess.db-journal` (7 of 120 in run 1). Answer `Missing`.
- No data ever existed in either state. Before the rework both completed as a first launch (FIND-002, SDD 3.9.1 note 1, spec NF2-1).
- Cause: spec 2.2 items 5 and 6 make any non-empty `-wal` or `-journal` a data trace, and row 5 promises that an interrupted creation completes. Rows 5, 7 and 9 cannot all hold for a creator killed while it switches to WAL or commits the first schema. The code follows the spec (`datadir.rs data_traces`: `wal`, `hot_journal`; `open.rs nf4_verdict`). It is a spec conflict, not a code slip.
- The same cause explains FIND-RW1-02 for the journal: a live creator's rollback journal is read as a trace, and `traces` is read once.

### What the sidecars can and cannot tell

`[RAN]` 2026-11-22 (system python, SQLite 3.54.1; the version bundled in `rusqlite` may differ `[UNV]`): a zero-length `chess.db` with a 512-byte `-journal` (garbage, or with a valid journal magic) reads as 0 tables; a read-only open leaves the journal; a read-write open **deletes** it. SQLite treats a journal beside a zero-page database as junk. FIND-ST-RW-01 already showed the same for a `-wal`.

`[INF]` From SQLite's behaviour and the Reviewer's kill states:

- A creation never leaves a zero-length `chess.db` beside a non-empty `-wal`: SQLite writes page 1 (through a rollback journal) before it creates a WAL. All observed `-wal` states had a non-zero `chess.db`.
- A non-zero `chess.db` that the read-only probe reads as "0 tables, `user_version` 0" already includes every valid committed WAL frame (the probe reads through the WAL). A WAL beside it that adds nothing SQLite can read holds no committed table, so no committed player or game. What remains in it is a half-written first transaction or garbage; neither is user data.
- Real evidence of a lost store is what a creation can never produce: a `journal/<uuid>.jnl` (written and fsynced before each move is acknowledged), `backups/` content, `chess.db.bak*`, `chess.db.tmp-bak-v*`, `chess.db.pre-import-*`, `chess.db.restoring`.

### Options

| Option | What it does | Trade-off |
|---|---|---|
| **A (recommended)** two trace classes | **Class P** (provenance): journal content, non-empty `backups/`, `chess.db.bak*`, `tmp-bak-v*`, `pre-import-*`, `restoring`. Any one with an empty database refuses (D-126 unchanged for these). **Class S** (sidecars): a `-journal` is never a trace of its own; a `-wal` is not a trace when `chess.db` is non-zero and the probe reads no schema. Exception kept: zero-length `chess.db` plus non-empty `-wal` refuses `Missing` before SQLite touches it (creation cannot produce it; SQLite would delete the WAL) | Both killed-creation states complete again. `WAL_SETTLE` (1 s per refusal) and the `traces` re-read race (RW1-02) go away, because the verdict no longer depends on a sidecar that a live creator shows for a moment. Cost: a lost store that has no journal, no backup and no backup file, only a leftover WAL beside a truncated database, opens as new. That is the same class as the already accepted R-NF4-1. If its WAL is not valid, SQLite's first write resets it; nothing we could read is lost `[INF]` |
| B grace period | Keep sidecars as traces unless their mtime is within N seconds of now | Rejected: fragile (clock steps, restored files keep old mtimes), and N is a guess |
| C leave as built | Accept that about 8 to 10 % of kills during the first launch (Reviewer's kill timing, not a field rate) show `StoreMissing`, and the user chooses `create_fresh` | Zero added risk of opening a lost store as new, but it breaks the FIND-002 promise on the very first run after a crash, and RW1-02 still needs its own fix. Valid if Brian prefers the safest refusal over first-run robustness |
| D use `store.id` | Treat an empty database as creation only when `store.id` is absent | Rejected: creation writes `store.id` before the schema, so killed states always have it |

### Recommendation: A

Confidence **Medium**. High that the two states are creation and hold no data. Medium on accepting the new residual case. Reason for not High: it narrows a refusal that Brian chose on 2026-11-22 (D-126), and the residual case cannot be tested against a real lost store.

**Reversibility.** The recognized states are the stored-state contract: after the first release a later build must classify them the same way, so the *state table* is one-way at first release (the D-126 reading). The code is two-way. Nothing is released today.

**Gate class: G2** (observable behaviour of `Store::open`, amends D-126). Brian approves. I cannot.

### Corrected decision table (changes to `store-rework-specs.md` section 2.3; I did not edit the spec file)

Rows 1, 2, 3, 6, 8, 10 stand as written. Row 3 keeps "any other `chess.db*` sibling" for an **absent** `chess.db`. Rows below replace 4, 5, 7, 9 and add 5b, 7b. `P` means a class-P trace.

| # | `chess.db` as SQLite reads it | Traces | `store.id` | Result | Notes |
|---|---|---|---|---|---|
| 4 | empty, **zero-length** | no P; `-journal` of any size ignored; `-wal` zero-length or absent | absent | **first launch** | The `-journal` is SQLite's to delete on open [RAN]. Not a trace |
| 5 | empty, zero-length | as row 4 | valid | **interrupted-creation completion**, `store.id` kept | FIND-002 |
| 5b (new) | empty, **non-zero** (0 tables, `user_version` 0, WAL included in the read) | no P; `-wal` of any size ignored | absent or valid | **first launch** or completion | Covers the killed-creation state 1. The probe already read the WAL |
| 6 | empty | no P | unreadable | `Corrupt` | unchanged |
| 7 | empty (either form) | **one or more P** | any | **refuse `Missing`** | Checked before `store.id` is read; nothing written. Unchanged except that sidecars are no longer in the list |
| 7b (new, was inside the pre-probe guard) | empty, **zero-length** | non-empty `-wal`, whatever else | any | **refuse `Missing`** | As built (`open.rs` guard before the probe). Creation cannot produce it; SQLite would delete the WAL (ST-RW-01) |
| 9 | unreadable: `ReadOnly` (hot rollback journal beside a non-zero database) | | | Section 1 decides; the hot journal itself is **not** a trace. Condition F of section 1 still blocks the exception when a P trace or a non-empty `-wal` exists, and then the answer is `Unavailable{ReadOnly}` | Corrects "a hot journal is a data trace by item 6". Writes nothing |

Spec 2.2, "Data traces": delete items 5 and 6; add a paragraph "SQLite sidecars" with the two rules above. Spec 2.3 "How rows 4, 5 and 7 are told apart": step 2 stays (re-read the probe once) and the settle loop is removed.

### Consequences

| Area | Consequence |
|---|---|
| Part B code (`open.rs`, `datadir.rs`) | `data_traces` splits into `provenance` (class P) and the sidecar sizes; `nf4_verdict` refuses on P only, plus the zero-length-plus-WAL guard; `WAL_SETTLE` and its tests go; `is_interrupted_first_creation` keeps its meaning (F) |
| Tests | New: the Reviewer's `rw1_kill_first_launch_at_random_instant_then_reopen` becomes a committed test with at least 500 kills and **0 refusals**; schemaless non-zero plus 32-byte, 4 152-byte and garbage `-wal` completes; same plus one P trace refuses; zero-length plus `-journal` completes; zero-length plus `-wal` refuses; zero-length plus `-journal` plus `backups/` refuses. Changed: NF4-4 to NF4-7 keep refusing for P traces only; the "non-empty `-wal` alone" sub-case of NF4-7 moves to the 7b form. The FIND-001 race test and a creator held inside the WAL switch (the Reviewer's RW1-02 emulation) must pass |
| D3, D4 | None. They never see the open path |
| C (`create_fresh`) | `database_state` classifier must match the new rows. `create_fresh` item 1 is unaffected: an empty `chess.db` is left in place |
| App | None. Same error `Missing`; reason is logged only |
| SDD | Note 22 lists `-wal` and `-journal` as traces; the dated note 27 (entered) corrects it as a proposal pending Brian |
| Residual risks | R-NF4-1 widens by one case (lost store with only a leftover WAL). New R-NF4-4: an invalid leftover WAL beside a schemaless non-zero database is reset by the first write |

**Timing.** An engineer is fixing RW1-01/02 now. If they implement the spec as written (sidecars stay traces) they will add a creator-aware wait, and that work is discarded if Brian accepts A. Recommend telling them the ruling is pending before they invest in the wait.

---

## (b) FIND-D2R-02: `finalize_game` overwrites `analysis_used`

### Facts

`[OBS]` `finalize.rs:181` writes `i64::from(req.analysis_used)`. Probe: `set_analysis_used(game)` then `finalize_game(analysis_used = false)` leaves `false`. Architecture §1495: the flag is set by the gateway the first time it grants `LiveAnalysis` or `WhatIf` for a Training game (DAT-3). `GameEnd` carries it (architecture 7.6 table, `analysis_used`). No journal record carries it before `GameEnd`. So the stored column is the only record of the grant across a crash.

### Options

| Option | Effect | Trade-off |
|---|---|---|
| **A (recommended)** OR (latch) | `analysis_used = stored OR request` in the finalize update (`MAX(analysis_used, ?)`). `set_analysis_used` is a true latch | A game cannot be un-flagged. No case needs it |
| B overwrite, as built, document it | `set_analysis_used` is only valid if the `GameEnd` will say the same | After a crash the actor is rebuilt from the journal, which has no flag. A `GameEnd` assembled then may say `false` and erase a grant |
| C journal the grant (new record) | Replay carries it | New record kind: stored-format change, `journal_version` bump. Disproportionate |

### Recommendation: A

Confidence **High**. It is the only option under which the flag survives the crash path the design already supports.

- **Gate: G3** (the meaning of a stored value). **Reversibility:** one-way at first release; before release nothing stored depends on it.
- **D2:** one-line change and a test (`set` then `finalize(false)` stays true; `finalize(true)` without a prior set gives true; never goes true to false).
- **D3:** `apply_records` of a `GameEnd` reaches the same function; add the test through apply. A restore from an older backup keeps any flag that is already true.
- **App:** the SDD wording "set_analysis_used (idempotent)" becomes "latch". The gateway keeps calling it at grant time and also puts the accumulated value in `GameEnd`.

---

## (c) FIND-D2R-03: validation of terminal data

### Facts

`[RAN]` (Reviewer probes): `finalize_game` accepts `ended_at = -5` (start 5001) and an empty `termination`; `final_journal_seq` may go backwards; `abort_game` accepts a human against human game. Requirements: only a computer against computer watcher can abort. D-42: the store trusts the caller. Restore replays journal records through these same commands.

### Principle for the recommendation

A store check belongs in the store when it (1) protects a stored invariant that nothing else can protect, and (2) can never make a *legitimate* journal unreplayable. A refused finalize on a finished game strands the game in progress and its rating effect, which is worse than an odd timestamp.

### Recommendation (one per sub-item)

| Sub-item | Where | Rule |
|---|---|---|
| `ended_at <= 0` | **Store** | Refuse with the existing `InvariantViolated`; nothing written. No real clock gives a value at or before the epoch. Engineer lists any existing test that uses a clock at 0; if there are some, use `< 0` and say so |
| empty `termination` | **Store** | Refuse with `InvariantViolated`, same as the empty hash check |
| `ended_at < started_at` | **Not enforced** | Wall clocks step back. Store the value. D-72 orders by `ended_at`, ties by uuid; it tolerates this |
| `final_journal_seq` below the stored `journal_seq` | **Not in finalize.** Reconcile (D3) | This is the StoreAhead and longer-run case (D-66); the journal and the reconcile plan decide, with a damage report |
| `abort_game` on a game that is not computer against computer | **App** (command refused at the source) | Store applies what a journal says: an `Abort` record that exists must replay. D3 apply logs a `warn` when the game is not c-c. D2 gets one pinning test that documents the store accepts it |

Confidence **Medium**: the first two are routine; the placement of the abort rule is a judgement about replay safety.

- **Gate:** G1 for the two store checks (existing error variant, inside the crate, no signature change). The abort placement is a note for the app SDD (no change to it: the rule is already a requirement).
- **Reversibility:** two-way.
- **D3:** add acceptance tests: an `Abort` on a human game applies and logs; a `GameEnd` with a lower `seq` goes through reconcile as damage, not through finalize.

---

## (d) FIND-D5R-03 and FIND-D5R-02: ordering key and the variation index

### (d1) Ordering key

`[OBS]` `stats_rows`, `head_to_head_rows`, `games_for_history` and `list_games` use `ORDER BY g.id` and say "start order". `GAME.id` is insertion order. A game restored from a journal (D3 creates the row from `Start` when absent) or imported gets a later `id` than games that started after it. With `limit` keeping the first N (note 20(a)) the "oldest N" can be the wrong set. No requirement in `requirements.md` defines a "most recent N" view `[OBS grep]`.

| Option | Trade-off |
|---|---|
| Keep `id`, rename the claim to "insertion order, first N inserted" | No code change. A restored game is out of start order. The domain sorts anyway for its own purposes `[INF]` |
| **B (recommended)** `ORDER BY started_at, id` | Matches the documented meaning "oldest N by start". The queries already scan all of `game` (Reviewer's plans), so the sort adds little: ESTIMATE under 5 ms at 10 000 games, not measured. No index needed |
| C add an explicit `StatsOrder {Oldest, Newest}` argument | Needed only when a consumer needs newest N. Public signature change (G2). Defer |

**Recommendation: B**, confidence Medium (the "newest N" need is unknown). **G1.** Two-way. Apply the same key to `list_games` so the two agree; add a test with a restored-style row (later `id`, earlier `started_at`). Consequence for D3: none. App: pass `reset_at` as before; `limit` is "oldest N since the window". Record in the app note that a "latest games" screen needs option C first.

### (d2) `variation(game_id)` index and the other scans

`[RAN]` (Reviewer, release, 50 000 rows): `claim_job` on an empty queue 1.86 ms; `save_variation` 10.5 ms; `list_variations` 1.05 ms; `stats_rows` for a Player with no games 2.41 ms. At 10 000 games `stats_rows` 19.9 to 24.5 ms (BM-ST-03, loaded machine). Variation and job lookups scan; stats scans `game`.

ESTIMATE by linear scaling of the 50 000 figures (not measured): at 10 000 rows `claim_job` about 0.4 ms, `save_variation` about 2 ms, `list_variations` about 0.2 ms; at 50 000 games a heavy Player's `stats_rows` about 100 ms. A person saves tens of variations, not tens of thousands, so the variation table stays far below 10 000 rows `[INF]`.

| Option | Trade-off |
|---|---|
| **A (recommended)** no index now | No stored-format change. Writes on the single store thread cost at most about 10 ms at 50 000 rows |
| B add `variation(game_id, from_ply)` to `0001_init.sql` now | Cheapest moment (nothing released). Needs Brian's G3 approval, a baselined DDL change in SDD 3.5.2, and changes the "DDL unchanged" statement all three gates make. Benefit unmeasurable at real sizes |
| C partial index for `analysis_job(state)` and a stats index | Same cost class; stats cannot use an index for the `OR` over both seats without a query rewrite |

**Recommendation: A**, confidence Medium. **Revisit trigger:** BM-ST-03 or a field log shows `save_variation`, `claim_job` or `stats_rows` p95 above 50 ms at the user's real row counts, or more than 50 000 variation rows. **Reversibility:** an index is additive and is one forward migration (the migrator exists), so it is **not** one-way. Gate if added: G3. Consequences for D3, D4, C, app: none.

---

## (e) FIND-D1R-01: nine registered failpoints with no call site

`[OBS]` registry in `fault.rs` has 43 names, 34 wired. The 9 unwired, and what each marks:

| Name | Point in the sequence | Owner (recommended) | Where |
|---|---|---|---|
| `backup.after_copy` | after the verified copy, before the verify/rename of the backup | **D1 follow-up** (part A code, SDD names it) | `backup.rs`, between the copy and `backup.verify` |
| `finalize.after_journal_end` | the journal `GameEnd` is durable, `finalize_game` has not begun | **D2 follow-up** | entry of `finalize_game`, before `BEGIN IMMEDIATE`. The caller (app live, D3/D4 on recovery) has already appended |
| `finalize.after_commit_before_rename` | finalize committed, the journal has not yet moved to `done/` | **D2 follow-up** | tail of `finalize_game` after `COMMIT`, before it returns. The rename is the caller's step |
| `reset.after_commit_before_voided_record` | reset committed, the holder has not yet appended `Voided` | **D2 follow-up** | tail of `reset_player` after `COMMIT`. SDD 3.11.4: "`reset_player` runs before any `Voided` record exists" |
| `apply.before_commit` | `apply_records` transaction, before commit | **D3** | in `apply_records`, same pattern as `finalize.before_commit` |
| `apply.after_commit` | after commit, before the backlog advances | **D3** | in `apply_records` |
| `journal.rebase.db_committed` | rebase DB step committed, file swap not begun | **D4** | rebase orchestration (allocation row 14); CT-ST-088 |
| `restore.after_replay` | restore replay finished, rename not done | **C** | `RestoreService` |
| `restore.before_rename` | before the restored database replaces the live one | **C** | `RestoreService` |

Reasoning for the three D2 sites: the store cannot place a point *between* two steps that a caller performs, but it can place it at the edge of its own command, and a process killed there has exactly the state the name describes. `[INF]`: if Brian or the D3 design prefers the orchestration to host them, the same names move to D3 and D4 and nothing else changes.

**Recommendation:** assign as above, plus one registry test: every registered name is either wired (a test installs a plan and sees `hits() > 0`) or listed in a `PENDING` table with owner and bead, and the table shrinks as the owners land. Confidence **Medium** (the D2 placement is my reading). **Gate: G1** (test-only feature, inside the crate). **Reversibility:** two-way (names are test-support; `FaultPlan` is feature-gated).

Task lists:

- **D1 follow-up (small):** wire `backup.after_copy`; add the registry test with the `PENDING` list (eight names); add the `!` parse test (FIND-D1R-03); a note in `lib.rs` that default `cargo test` skips the fault suites (FIND-D1R-04).
- **D2 follow-up:** wire three names above; add a kill test per name (the harness exists): kill, reopen, state equals the described one, retry succeeds.
- **D3:** wire `apply.before_commit`, `apply.after_commit`; acceptance includes a kill test at each.
- **D4:** wire `journal.rebase.db_committed`; CT-ST-088 uses it.
- **C:** wire `restore.after_replay`, `restore.before_rename`; CT for kill at each.

---

## (f) FIND-RW1-03 and FIND-ST-RW-04: rebase of a rebased file

### Facts

`[RAN]` Reviewer and engineer: a rebase appends a closing `Clock` whenever the last record is not terminal, including when it is already such a `Clock`. Three rebases of a file ending in `Move`, `Pause` or `Clock` give 4, 5, 6 records. Spec 3.3 test (e) says the second rebase equals the first. Replay is correct either way; each damage recovery grows the file by about 60 bytes.

### Recommendation

Fix the code, keep the spec's intent, and give the spec the exact rule (dated correction entered in the SDD, section "Entered text" below):

> The rebase omits the closing `Clock` when the last record is terminal (CH-11) **or** when it is a `Clock` whose `white_ms`, `black_ms`, `running_side` (null) and `wall_ms` equal the values the rebase would write. Rebasing a rebased file therefore returns the same bytes.

Options: (1) as above (recommended); (2) change test (e) to "the second rebase adds one `Clock` and the third equals the second", which is weaker and still grows once per recovery; (3) accept growth. Confidence **High**.

- **Gate: G3** (the content of rebased records; Brian approved CH-11 B for the terminal case only). **Reversibility:** one-way at first release for the *rule*; before release free.
- **D4:** rebase is called by recovery; idempotence makes a repeated recovery after a crash window byte-stable, which CT-ST-088 ("a second restart changes nothing") needs.
- Also close FIND-RW1-10 in the same change (check that `intact_bytes` and the decoded records agree), since both touch `build_rebased`.

---

## (g) Spec and SDD corrections (FIND-ST-RW-01, 02, 03, 05)

| Finding | Correction | Class | Entered |
|---|---|---|---|
| ST-RW-01 | Spec 2.2 is wrong: a zero-length `chess.db` with a non-empty `-wal` is **never** "schema". SQLite deletes the WAL when it opens the empty file, even read-only [RAN by engineer and Reviewer]. The state is refused before the probe (row 7b). Rows 4 to 7 amended as in (a) | Fact | SDD note 27 |
| ST-RW-02 | `WAL_SETTLE` (1 s) was built where the spec said "re-read once". It costs 1 s per refusal. **If (a) is accepted it is removed**; until then it is an as-built deviation, G1 inside the crate, not accepted by anyone | Pending (a) | SDD note 27 |
| ST-RW-03 | The read-only probe of a WAL database with a `-wal` can create a **non-empty** `-shm` (32 KiB seen), not only a zero-length one. Spec 2.2 and 2.6 say "ignoring zero-length `-wal` and `-shm`": tests ignore every `-shm`, and so does the "nothing written" claim for `-shm` | Fact | SDD note 27 |
| ST-RW-05 | CH-5 B built over the **whole dropped region**: any CRC-valid frame in it is damage, not only the frame at the first bad position. Spec 3.2 reads "the frame at `pos`". The Reviewer judged the as-built reading right: this writer cannot produce a CRC-valid frame in the dropped region, so reporting one is the conservative reading of "any CRC-valid frame the decoder refuses" (his section 3.1). Recommend: **accept as built** and amend spec 3.2. False-positive case: a duplicate `seq` at the tail (R-JF-2) | G3 detail, Brian | SDD note 28 |

I recommend accepting ST-RW-05 as built. Confidence **Medium**: I did not test a power cut, and neither did the Reviewer.

---

## Entered text

Dated notes 27 and 28 in `docs/design/SDD-chess-store.md` (3.9.1 area, after note 26) and one dated paragraph at the end of architecture Addendum M record (a) to (g) as **proposals pending Brian**. No row is rewritten. I did not edit `docs/engineering/store-rework-specs.md` (outside the edit list for this task); the corrected rows for its section 2.3 are in (a) above and can be pasted by whoever owns the file.

## Not analysed, unverified

- No code was built or run for this package. The only run is the SQLite probe in (a) (system python, SQLite 3.54.1, scratchpad), which does not prove the bundled `rusqlite` SQLite behaves the same.
- I did not read `create_fresh`, restore or import code (not built).
- The cost figures in (d) are the Reviewer's measurements or my linear scaling of them, labelled as such.
- Whether the D2 placement of the three finalize and reset failpoints suits the D3 design is my reading of SDD 3.11.4.
- Real lost-store behaviour with a leftover WAL (the residual of A) was not tested.

## Needs a human

Brian Prascher: (a) G2 ruling; (b) G3; (f) G3; (g) ST-RW-05 G3. G1 items (c), (d1), (e) need only a desk review.
