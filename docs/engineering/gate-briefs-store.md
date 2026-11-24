# Gate briefs: chess-store, GATE-108 to GATE-111

| Field | Value |
|---|---|
| Document | docs/engineering/gate-briefs-store.md |
| Author | System architect agent, Architect stance, written after Brian Prascher's rulings of 2026-11-24 (decision bead chess-a0r.3.56). Baseline B6 |
| For | **Brian Prascher**, who asked for the four gates one at a time in this order: GATE-108 (part B including the open path), GATE-109 (D2), GATE-110 (D5), GATE-111 (D3) |
| Status | **Briefing notes only. Nothing here is an approval.** G2 and G3 approvals are Brian's. The recommendations are the architect's opinion and are not a review by the Review Board stance. Brian commits himself; nobody else commits or pushes |
| Gate records | [GATE-108](gates/GATE-108-chess-store-B.md), [GATE-109](gates/GATE-109-chess-store-D2.md), [GATE-110](gates/GATE-110-chess-store-D5.md), [GATE-111](gates/GATE-111-chess-store-D3.md). All four say OPEN, NOT APPROVED |
| Tags | `[RAN]` executed (by whom is named: reviewer agent, engineer agent), `[OBS]` read by me, `[INF]` inferred, `[UNV]` not verified, `ESTIMATE` a number that is a guess |
| Where the rulings are entered | architecture [Addendum N, section 11.4.22](../architecture.md) (D-137 to D-153), store SDD dated notes of 2026-11-24, app SDD 11.5 |

## 0. Order, dependencies and the one-line answers

**Dependency order.** The order 108, 109, 110, 111 is also the safe order. GATE-111 **edits code that GATE-108 and GATE-109 cover**: it moves the bodies of `finalize_game` and `abort_game` into `finalize_in_tx` and `abort_in_tx` (D2, `repo/finalize.rs`); after the rulings of 2026-11-24 it also changes `journal/format.rs` (part B: `ended_at` and `matchup` rules), `repo/lifecycle.rs` (D2: the repair pair is deleted, live void paths check `ended_at > 0`) and `repo/stats.rs` (D5: imported games, NULL start). So:

1. An approval of 108, 109 or 110 covers the code **as reviewed** (reviewer snapshot of `review-chess-store-fixes-2.md`), plus the later edits only where the brief says so.
2. Every file that GATE-111 edits in a part already approved must be listed in GATE-111 again, and the suites of that part must be re-run and green before GATE-111 closes. I recommend Brian approves 108 to 110 with that sentence as a condition.
3. Nothing is released and `journal_version` is 1, so the stored-format rules in 108 and 111 can still change; after the first release they cannot (architecture Addendum M note (3), Addendum N).

**Code state when I wrote this** `[OBS]`, read from the files, not run by me: `journal/format.rs` already has the `ended_at > 0` and `matchup_code` checks in `validate()`; `void_game_repair_pair` is no longer in `src/`; `repo/lifecycle.rs`, `repo/apply.rs` and `journal/format.rs` changed on 2026-11-23 at about 13:16 `[INF from file times]`. There is **no** `unpark` in `src/`, `thread.rs` was last changed earlier, and `repo/stats.rs` has no imported-game filter yet `[INF from file times and grep]`. The engineer is evidently building the rulings; none of that work has evidence or a review yet.

*Update 2026-11-24 (architect, after the engineer's section "Rulings 2026-11-24" in `step2-store-D3-evidence.md`).* The paragraph above is stale for D3: the D-145 policy, the D-152 flush, `unpark_game`, `parked_games`, the bounds, the transient list and the F2-01, F2-02, F2-05 and F2-07 items are now **built in the tree** `[OBS by grep and reading, not run by me]`. The engineer reports `cargo test -p chess-store` 440 tests (515 with `test-support`), 5 runs each, 37 hand mutants killed, the kill race binary at 500 rounds, fmt, clippy and the CI scripts clean `[RAN, engineer agent]`; the **fresh re-review of that code does not exist yet** `[OBS: no `review-chess-store-D3-rulings.md`]`. GATE-111 therefore stays "not yet".

| # | Gate | Class | Independent review | Recommendation | Confidence |
|---|---|---|---|---|---|
| 1 | **GATE-108** part B incl. open path | G3 (stored formats), G2 (open path, public items) | Fresh reviewer, 3 passes; last: PASS WITH CONDITIONS, no Major | **Approve with conditions** (section 1.7) | Medium-High |
| 2 | **GATE-109** D2 lifecycle and ratings | G3 (stored values), G2 (public items) | Fresh reviewer, 2 passes; last: PASS WITH CONDITIONS, no Major | **Approve with conditions** (section 2.7) | Medium-High |
| 3 | **GATE-110** D5 statistics, evaluations, variations, jobs | G2 (corrected from G1) | Fresh reviewer, 2 passes; last: PASS WITH CONDITIONS, no Major | **Approve with conditions** (section 3.7) | Medium |
| 4 | **GATE-111** D3 apply path | G3 (stored values, journal format), G2 (public items) | One fresh reviewer pass, on the code **before** the rulings: PASS WITH CONDITIONS, 2 Major. Post-ruling code: built and engineer-tested, **no independent re-review yet** | **Not yet** (section 4.7) | High |

## 1. GATE-108: part B (journal, leases, process registry, open path)

### 1.1 What it covers

`crates/chess-store` part B: the per-game journal (format, writer with `flock`, reader with torn-tail and damage detection, reconcile, rebase), leases and the process registry, F-A03 (the open guard wired to the registry: `Exclusive`, `OtherVersionRunning`), and, from the reworks, the whole **open path**: first launch, interrupted creation (NF-2), an empty database beside data traces (NF-4), `create_fresh`. Class: **G3**, because the journal file format is a stored format frozen at the first release (D-41); **G2** for the open path and the public items. Approver: Brian. Source: GATE-108 header and rework sections.

### 1.2 Key takeaways: what was built

- Journal format exactly as SDD 3.11.1 and architecture 7.6; CRC-32 frames, kind codes, typed strict payloads, tolerant decode of unknown fields (D-131). Writer-assigned `seq`, `F_FULLFSYNC` before an append is acknowledged, a failed append cleans up or poisons the writer (D-82).
- Rebase with no-replace moves by `hard_link` then `remove_file` (D-110), idempotent on repeat (D-141).
- Process registry and leases (2 s, 10 s, 10 s), stale takeover, pid check by `/bin/kill -0` (no `libc`, no `unsafe`).
- Open path rules as rulings stand today: two-class trace rule (D-137), narrowed hot-journal exception decided on a private copy (D-125), `create_fresh` (D-119, D-127).
- Engineer's evidence: `step2-store-B-evidence.md` (77 hand mutants, 74 killed, 3 equivalent or redundant; `[RAN]` engineer).

### 1.3 What the independent reviews found and what was done

| Review | Verdict | Main findings | Disposition |
|---|---|---|---|
| `review-chess-store-B.md` (reviewer 1) | PASS WITH CONDITIONS | B-R01 (an acknowledged record could be dropped silently), B-R02 (CH-4 bound must be in `validate()`), B-R06 (the NF-2 exception opened a real v1 store read-write), B-R03, B-R04, B-R05, B-R07 to R13 minor or info | R01, R02: D-128, D-129. R06: D-125. R03, R05: **moved to D4** by ruling (D-153). R04: status not found `[UNV]` |
| `review-chess-store-rework-1.md` (reviewer 2) | **FAIL for part 1** | RW1-01 (Major): a killed first launch is refused `Missing` (8 to 10 % of kills). RW1-02 (Major): a live concurrent creator refused. RW1-03 to RW1-12 minor | RW1-01: ruled 2026-11-23, D-137. RW1-02: fixed (bounded rescan, then removed by D-137). RW1-03: D-141 |
| `review-chess-store-fixes-2.md` (reviewer 3) | **PASS WITH CONDITIONS, no Major** | Both Majors fixed on every test the reviewer could build. Minor: F2-04, F2-06, F2-07, F2-08; info F2-10 | F2-04: D-144. F2-06: D-151. F2-08: store SDD note 29, entered 2026-11-24. F2-07: open |

### 1.4 Evidence `[RAN, reviewer agent]` unless marked

- 1 300 SIGKILL rounds of a first launch, **0 refusals** (before the fix: 8 to 10 %); 156 failpoint cases at every `open.*` and `sync.*` name, all reopened; 2 400 concurrent-opener rounds (threads and processes), 0 `Missing`, 0 `Corrupt`, 0 lost writes; 80 lost-store cases and 450 concurrent ones with every byte unchanged.
- The real v1 store case (a Player and a hot rollback journal) is left byte-identical; the stale-scratch sweep touches only dead-pid, real, 0700, same-owner directories.
- Rebase idempotence: 0 of 4 000 random journals differ on a second rebase.
- Suites, 5 runs each, 0 failed: 356 default, 418 with `test-support`. clippy `-D warnings` both ways, `rustfmt --check`, coverage lines 96.98 %. Mutation: 70 viable, 66 killed, 4 survived.
- The tree has changed since (D3 added tests; the D3 reviewer counted 413 and 485). I did not re-run anything `[UNV]`.

### 1.5 Residual risks and limits

- **Accepted by Brian's rulings:** R-NF4-1 (a lost store whose only leftover is a WAL beside a truncated database opens as new), R-NF4-4 (an invalid leftover WAL beside a schemaless non-zero database is reset by the first write), no usable `$TMPDIR` refuses a killed first launch with a hot rollback journal (D-151, about 2 in 700 kills `[RAN, reviewer]`).
- **Not run:** a real power cut, a full disk (injected only at the first-launch sync points), a read-only or network volume, a sandboxed or notarized app (whether `/bin/kill` works there), Intel hardware, any sanitizer. The bound `NF2_COPY_CAP` is an `ESTIMATE`.
- Duplicate `seq` at a journal tail is now reported as damage (R-JF-2, D-142 (1)).

### 1.6 Decisions Brian already made that this gate embodies

| Date | Decision bead | Decisions |
|---|---|---|
| 2026-11-04 | chess-a0r.1.47 and the OQ-16 closure | D-41 to D-43 (journal format frozen at release), D-83 (damaged-file moves), D-66, D-72 |
| 2026-11-09 | chess-a0r.3.34 | D-108 to D-115 (including D-110 `hard_link` moves, D-111 workspace lint, no `unsafe`) |
| 2026-11-11 | chess-a0r.3.48 | D-116 to D-124 (`create_fresh` D-119, first-launch rules D-122, D-124) |
| 2026-11-13 to 2026-11-22 | chess-a0r.3.50 | D-125 (narrowed hot-journal exception), D-126 (NF-4), D-127 (`create_fresh` item 1, my reading, he was asked to confirm), D-128 (CH-4), D-129 (CH-5 B), D-130 (CH-11 B), D-131 (tolerant decode), D-132 (part B as built), D-136 (43 s retry) |
| 2026-11-23 | chess-a0r.3.52 | D-137 (two-class trace rule, G2), D-141 (rebase idempotence, G3), D-142 (CH-5 as built, JSON-object payloads, clock text), D-143 (failpoint owners) |
| 2026-11-24 | chess-a0r.3.56 | D-144 (`ended_at` rule, G3), D-146 (`matchup` text, G3), D-151 (TMPDIR), D-153 (B-R03, B-R05 to D4) |

### 1.7 Open items, class and recommendation

| Item | Owner | Class | State |
|---|---|---|---|
| D-144 and D-146 live in `journal/format.rs`, a part B file | Engineer, GATE-111 | G3 | Built in the tree `[OBS]`, **no evidence or review yet**. Approve them under GATE-111, not here |
| F2-07: the committed first-launch kill test runs 120 rounds, the package named 500 | Engineer or Brian | Evidence only | *Update 2026-11-24:* built; the committed first-launch kill test now runs **500 rounds by default** (`CHESS_STORE_KILL_ROUNDS` overrides; engineer's race run: 10 runs, all exit 0 `[RAN, engineer agent]`). The gap is closed in the test, not yet re-reviewed. Original text: open; the reviewer's 1 300 rounds are the evidence; I would accept 120 as the committed default |
| B-R04: `create` can race with `probe` (70 of 3 000 spurious `Locked` in review 1) | Engineer | Minor | Status **not found** `[UNV]`; ask the engineer whether it was closed |
| B-R03, B-R05: `close_final` not crash-idempotent; incomplete `.rebase` installed | D4 | G1 | D4 acceptance lines (D-153) |
| Gate record text: still says "Reviewer pass NOT DONE" and does not list D-144, D-146, D-151 | Engineer / PM | G0 | Stale; fix before Brian closes it |
| Failpoint wiring (D-143) | D1 and D2 follow-ups, D4 | G1 | Not part of the approval |

**Recommendation: approve with conditions.** Class G3 and G2, approver Brian. Conditions: (1) the approval covers the format as of the reviewer's snapshot; the `ended_at` and `matchup` rules are approved under GATE-111; (2) the F2-07 gap is accepted or closed; (3) the engineer states the B-R04 status; (4) the gate record is brought up to date (G0). Why approve: three independent passes, the last with no Major, each Major was fixed and re-tested by a fresh reviewer, and every open item is Minor, ruled, or owned by a later part. Confidence Medium-High; the reason it is not High is that power loss and the sandboxed `/bin/kill` check were never run.

## 2. GATE-109: part D2 (game lifecycle writes and ratings)

### 2.1 What it covers

`finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player`, `check_rating_invariant`, their DTOs and error shapes, and the stored values they write: `GAME.termination` texts `aborted`, `voided by reset`, `voided: unreadable`; reset point rows (`game_id` NULL, `k` 0); the Elo columns of the terminal row; the `analysis_used` latch. Class: **G3** (stored values) and **G2** (public items). Approver: Brian.

### 2.2 Key takeaways: what was built

One transaction applies the rating settlement, the terminal row and the slot, lease and job work together; a failpoint just before each commit leaves every table unchanged and the retry succeeds. The shared void function serves the live reset, the unreadable-game void and the apply path (D-78 (5)). `analysis_used` is a latch (`max(stored, request)`, D-138). Terminal data is validated for shape only (D-139). Evidence file: `step2-store-D2-evidence.md`.

### 2.3 What the independent reviews found and what was done

| Review | Verdict | Findings | Disposition |
|---|---|---|---|
| `review-chess-store-D1-D2-D5.md` (reviewer 2) | PASS WITH CONDITIONS, **one Major that blocked approval** | FIND-D2R-01 (Major): `check_rating_invariant` read without a snapshot and reported violations that did not exist while a finalize committed (25 false in 31 023 checks). D2R-02 (`analysis_used` overwrite), D2R-03 (terminal validation), D2R-05 (void of a settled row) | D2R-01 fixed with one read transaction. D2R-02: latch (D-138). D2R-03: D-139. D2R-05: refused, repair pair then deleted (D-149) |
| `review-chess-store-fixes-2.md` (reviewer 3) | **PASS WITH CONDITIONS, no Major** | F2-04 (journal accepts what the store refuses), F2-05 (`set_analysis_used` flags a finished game), F2-09 (repair pair is a bypass) | F2-04: D-144. F2-09: D-149. **F2-05: no ruling in the relay** `[UNV]` |

### 2.4 Evidence `[RAN, reviewer agent]` unless marked

- 3 675 Elo vectors match an independent implementation.
- After the fix: **31 211 invariant checks beside 500 finalizes, 0 false violations** (before: 25 in 31 023).
- Kills at every `*.before_commit` leave the database unchanged; the latch holds through crash and replay.
- Same suite and tool results as section 1.4. Engineer's own evidence: `step2-store-D2-evidence.md` `[RAN, engineer]`.

### 2.5 Residual risks and limits

- Power-loss behaviour is `[UNV]`. The Elo arithmetic is checked against one independent implementation, not against a second rating library.
- A finalize of a finished game is a no-op by design; ordering of times is not enforced (D-139), so a wall clock that steps back stores an odd but positive `ended_at`.
- `finalize.after_journal_end`, `finalize.after_commit_before_rename` and `reset.after_commit_before_voided_record` are registered failpoints with **no call site yet** (D2 follow-up, D-143), so those three crash windows have no kill test.

### 2.6 Decisions already made that this gate embodies

| Date | Decision bead | Decisions |
|---|---|---|
| 2026-11-04 | OQ-16 and Addenda D to F | D-72 (finalize order), D-78 (5) (one void function), D-87 (`GameEnd` and `Voided`), D-69 |
| 2026-11-11 | chess-a0r.3.48 | D-116 (F-A04: rows with `game_id` NULL are points, `k` 0), D-117 (F-A05 option A: `HasUnfinishedGames{ids}`, `UnlistedUnfinishedGames`) |
| 2026-11-13 to 2026-11-22 | chess-a0r.3.50 | D-135 (stored abort text `aborted`, rating rows ordered by `id`, `VoidSpec.game`, three failpoints) |
| 2026-11-23 | chess-a0r.3.52 | D-138 (latch), D-139 (shape only) |
| 2026-11-24 | chess-a0r.3.56 | D-144 (live void paths check `ended_at > 0`), D-149 (`void_game_repair_pair` deleted) |

### 2.7 Open items, class and recommendation

| Item | Owner | Class | State |
|---|---|---|---|
| GATE-111 edits D2 files: the `finalize_in_tx` and `abort_in_tx` split, void `ended_at` checks, deletion of the repair pair, the queue drain before live terminal commands | Engineer, GATE-111 | G1 inside the crate | The D2 suite must stay green; list the files in GATE-111 |
| F2-05: `set_analysis_used` has no status guard | Engineer | Minor, G1 | *Update 2026-11-24:* **built as the guard I proposed, with one addition**: `set_analysis_used` on a game that is not `in_progress` returns `InvariantViolated`, **unless the flag is already set (then `Ok`)**; it stays a latch for a live game. The engineer's choice, which I recommend Brian accept (it protects a hash and statistics already settled; the app has no legitimate late call). Needs a look in the fresh review. Original text: no ruling `[UNV]`; a small ruling for Brian or a desk review, not a blocker |
| Three unwired failpoints and the registry test | D2 follow-up | G1 | Open (D-143) |
| Gate record: says "Reviewer pass of the rulings NOT DONE" although a fresh pass exists | Engineer / PM | G0 | Stale |

**Recommendation: approve with conditions.** Class G3 and G2. Conditions: (1) GATE-111's edits to D2 files are re-listed there and the D2 tests are re-run green; (2) F2-05 is either guarded or accepted in writing; (3) the three failpoints are wired before the first release (not before approval). Why approve: the only Major was fixed and re-tested by an independent concurrency run, the arithmetic matches an independent implementation, and the stored values are the ones Brian ruled. Confidence Medium-High.

## 3. GATE-110: part D5 (statistics rows, evaluations, variations, job queue)

### 3.1 What it covers

15 new `Store` methods (`stats_rows`, `head_to_head_rows`, `games_for_history`, `upsert_move_eval`, `get_game_analysis`, `save_variation`, `delete_variation`, `list_variations`, the job-queue methods), the types `StatsGameRow`, `StatsSide`, `MoveEvalRow`, `EvalFlag`, `JobState`, `GameAnalysis`, `VariationRecord`, and the errors `VariationAnchor`, `NotOwner`, `VariationTooLarge`. **No schema or stored-format change.** Class: **G2** (the record header was corrected from G1 on 2026-11-23, a G0 record fix: two interface decisions, D-133 and D-134). Approver: Brian.

### 3.2 Key takeaways: what was built

Statistics are rows only (D-44), filtered by structural keys, no aggregate; the store owns `StatsGameRow` so no new crate edge is needed (D-133). A saved variation is identified by `(game, from_ply, pgn)` and saving is idempotent (D-134). The job queue is compare-and-set on `owner` and `state = 'running'` inside `BEGIN IMMEDIATE`, with a 10 s claim lease. `save_variation` refuses a text above 1 MiB (`MAX_VARIATION_PGN_BYTES`, **ESTIMATE**). Evidence: `step2-store-D5-evidence.md`.

### 3.3 What the independent reviews found and what was done

| Review | Verdict | Findings | Disposition |
|---|---|---|---|
| `review-chess-store-D1-D2-D5.md` | PASS WITH CONDITIONS, no defect in the semantics | D5R-01 (no size cap on `pgn`), D5R-02 (plans scan), D5R-03 ("start order" was insertion order), D5R-04 (boundary scanner skips D5 files), D5R-05 (retry of `complete_job`) | D5R-01: cap. D5R-03: order `(started_at, id)` (D-140). D5R-02: no index (D-140). D5R-05: owner retry answers `Ok`. **D5R-04: engineer note says "not in the fix list"; no later closure found** `[UNV]` |
| `review-chess-store-fixes-2.md` | **PASS WITH CONDITIONS, no Major** | F2-01 (latent): an imported game with both seats makes every statistic of that Player `Corrupt`. F2-02: NULL `started_at` sorts first. F2-03: two mutants that drop the `id` tie-break survive | F2-01, F2-02: ruled 2026-11-24 (D-150), **not built yet** `[INF]`. F2-03: tests listed in D-150 |

### 3.4 Evidence `[RAN, reviewer agent]` unless marked

- All 36 D5-area hand mutants killed in the first pass; 4 of the 70 viable mutants of the last pass survived in total (the two `id` tie-break ones are F2-03).
- Stats benchmark, debug build, 10 000 rows: `stats_rows` 52 ms, `games_for_history` 43 ms, head-to-head 5 000 rows 27 ms. Release figures at 50 000 rows (earlier review): `claim_job` 1.86 ms, `save_variation` 10.5 ms, `list_variations` 1.05 ms. The scaled figures in the decision package are `ESTIMATE`.
- Owner idempotence, the 1 MiB cap boundary (bytes, not characters) and the `(started_at, id)` order behave as ruled.

### 3.5 Residual risks and limits

- Variation and job lookups scan; fine for tens of variations, to be revisited at the trigger in D-140 (p95 above 50 ms or more than 50 000 variation rows, `ESTIMATE`).
- The statistics query is one `SELECT` over `GAME`; at 50 000 games a heavy Player's call is about 100 ms by linear scaling `ESTIMATE`, not measured.
- Until D-150 is built an imported game with seats can break a Player's statistics (latent: no import path exists yet).

### 3.6 Decisions already made that this gate embodies

| Date | Decision bead | Decisions |
|---|---|---|
| 2026-10-31 and 2026-11-04 | amendments 7 and 8 | D-44 (rows only, no SQL view), decisions 12 and 13 (variation anchor, CR-9.3, CR-9.4) |
| 2026-11-11 | chess-a0r.3.48 | D-116 (points have `game_id` NULL) |
| 2026-11-13 to 2026-11-22 | chess-a0r.3.50 | D-133 (store-owned `StatsGameRow`), D-134 (variation identity) |
| 2026-11-23 | chess-a0r.3.52 | D-140 (order `(started_at, id)`, `limit` = oldest N, no variation index) |
| 2026-11-24 | chess-a0r.3.56 | D-150 (imported games excluded, NULL start last, tie-break tests) |

### 3.7 Open items, class and recommendation

| Item | Owner | Class | State |
|---|---|---|---|
| F2-01 imported-game filter plus a test with an imported game that has seats | Engineer | G1 | Built per the engineer (2026-11-24 evidence; tests and hand mutants `[RAN, engineer agent]`); not re-reviewed |
| F2-02 `ORDER BY g.started_at IS NULL, g.started_at, g.id` in `list_games`, `games_for_history` and the statistics queries, with an imported-row test | Engineer | G1 (Brian informed: refines D-140) | Built per the engineer (2026-11-24 evidence; `[RAN, engineer agent]`); not re-reviewed |
| F2-03 two tests for the `id` tie-break | Engineer | G1 | Open |
| D5R-04 scanner covers `api_analysis.rs` and `store/d5.rs` | Engineer | Minor | Status `[UNV]` |
| GATE-111 edits D5 files (`stats.rs`, `game.rs` for D-150) | Engineer | G1 | List in GATE-111 |
| Gate record text (Reviewer pass "NOT DONE", system-architect desk review "NOT REQUESTED") | PM | G0 | Stale |

**Recommendation: approve with conditions.** Class G2, approver Brian. Conditions: F2-01, F2-02, F2-03 built and shown by a test that fails on the old code, checked by a desk review (the changes are small SQL and tests); D5R-04 status stated. Why approve: no semantic defect in two independent passes, no stored-format change, and the interface decisions are Brian's own. If Brian prefers to approve code that already includes the three conditions, defer this gate until they land; the cost is small either way. Confidence Medium (the pending SQL edits are unseen).

## 4. GATE-111: part D3 (apply path)

### 4.1 What it covers

`Store::apply_records`, the reconcile-plan applier (`apply_reconcile_plan`, crate-private, first caller is D4), the apply backlog (per-game ordered queue, timer on the store thread, 0.1 s doubling to 5 s, `StoreBacklog` at 30 s), `sync_backlog`, `backlog_status`; and the stored values: `MOVE.ply = journal ply + 1` (D-92), the `GAME` row that a `Start` creates, the `termination` text a `GameEnd` reason maps to, the void fields of a `Voided` record, the `journal_seq` rule. Class: **G3** (stored values) and **G2** (public items). Approver: Brian.

### 4.2 Key takeaways: what was built (before the rulings)

A batch commits atomically (rows, clocks, `journal_seq`, and the D2 finalize, abort or void); a crash on either side of the commit is repaired by applying again. A busy database queues the batch (the journal already holds the record, so play does not wait). The `finalize.rs` change is a move of two function bodies. Evidence: `step2-store-D3-evidence.md`.

*Update 2026-11-24, what is now built after the rulings (engineer's section "Rulings 2026-11-24"; I read it and grepped `thread.rs`, `backlog.rs`, `error.rs`, `api.rs`, `store/d3.rs`, `repo/game.rs`; I did not run anything).* Items where the code differs from, or goes beyond, the wording of D-145 and D-152 (architecture Addendum N, dated note 2026-11-24, has the full table):

| Built behaviour | Versus the ruling text | Class |
|---|---|---|
| `sync_backlog` returns `GameParked{game}` at once when **any** game is parked | D-145 said "a distinct result"; this is that result, and it fires for any parked game, not only the one the caller cares about | G2 |
| A later `apply_records` for a parked game returns `GameParked` and queues nothing | D-145 said "the same error keeps being returned"; the first refusal returns the cause, later calls return `GameParked` | G2 |
| `parked_games() -> Result<Vec<ParkedGame{game, cause: ParkCause, pending_batches, parked_for}>>`; `unpark_game(game) -> Result<Option<usize>>`; `ParkCause` (`InvariantViolated`, `NotFound`, `Corrupt`, `Internal`, `QueueFull`, `Other`) | Names were the architect's proposal; the `Result` wrapper is the common `Store` shape | **G2: names, fields, return type** |
| After `unpark_game` the caller must pass the whole journal in one call: documented in the doc comment, **not enforced** | Matches my reading of D-145 (drop the queue, re-pass the journal); a caller that passes only a later batch skips the dropped records | G2: Brian to accept the unenforced contract |
| `MAX_BATCHES_PER_GAME` 256 and `MAX_PENDING_BATCHES` 4 096, public constants; a game with a queue at a bound is parked (`QueueFull`) and gets `BacklogFull`; a game with **no** queue that meets the total bound gets `BacklogFull` and is **not** parked | Numbers are my `ESTIMATE`; the no-queue case is the engineer's (nothing to hold) | **G2: constants and numbers**; new `BacklogFull` variant |
| Transient (never park): busy, locked, out of memory, interrupt, lock protocol, schema change, plus transient I/O as `Unavailable`; unknown code and panic park | D-145 left the exact list to the engineer for GATE-111; this is that list | **G2: list for Brian** |
| Live `finalize_game`, `abort_game`, `void_unreadable_game`, `reset_player` first drain that game's queue; `Busy` if it cannot drain; `GameParked` if the game is parked; the command then does not run | **Stricter than D-152 and the package** (they said `Busy`); the parked refusal is observable by the app | G1 for the flush, G2 for the new refusal; engineer judgement call |
| `set_analysis_used` refused with `InvariantViolated` on a game not `in_progress`, unless the flag is already set | F2-05; the package left it open | G1; engineer judgement call |
| A waiter forces a pass every `max(100 ms, timeout / 4)`; a new batch behind only parked games restarts the timer; dropped waiters are removed | FIND-D3R-03, -04, -08 | G1 |

**Engineer judgement calls (E-03, E-05).** I could not find the labels E-03 and E-05 in any file `[UNV]`. I read them as the stricter flush and the `set_analysis_used` refusal (with the smaller choices above riding along). **I recommend Brian accept them** (Medium-High): without the flush refusal a terminal command could settle ratings over a hole in the rows, and without the refusal a late analysis flag could change a finished game's settled statistics. Accepting them is a G2 and G1 matter; it is Brian's, not mine.

### 4.3 What the independent reviews found

| Review | Verdict | Findings |
|---|---|---|
| `desk-review-G1-step2-D3.md` (architect desk review) | PASS WITH CONDITIONS (G1): three Major conditions for Brian | DR-01 end-time rule, DR-02 gap after a first-attempt error, DR-03 approval order with GATE-109 |
| `review-chess-store-D3.md` (fresh reviewer, tree hash de8b36fd1596f602) | **PASS WITH CONDITIONS, with 2 open Majors**, 10 Minor, 4 Info | FIND-D3R-01 (Major): a refused first attempt leaves a silent hole, a refused retry parks the game for the life of the process, the queue is unbounded, transient SQLite codes count as permanent. FIND-D3R-02 (Major): `GameEnd` or `Abort` with `ended_at <= 0` refuses the whole batch and the journal can never be applied. Minor: `sync_backlog` can answer `Busy` while the lock is free (D3R-03), a stale timer after a park (D3R-04), stale batches rewrite rows (D3R-05), no `ended_at` check on `Voided` and `Start` (D3R-07), leaked waiters (D3R-08), surviving mutants (D3R-10) |

The reviewer's own words: the earlier reviews used "PASS WITH CONDITIONS" to mean "no Major"; here it does not.

### 4.4 Evidence `[RAN, reviewer agent]` unless marked

- **300 SIGKILL rounds** (100 at `apply.before_commit`, 100 at `apply.after_commit`, 100 random during 12 three-record batches): rows are always a gap-free prefix of whole batches, every acknowledged batch present, 0 failures.
- Ply base `+1` including journal ply 0; half-null clocks and `u64::MAX` values refused with no write; a split D-87 pair refused as expected.
- Suites, 5 runs each: 413 default, 485 with `test-support`, 0 failed; clippy and `rustfmt` clean; coverage of the D3 files 97.8 % to 100 % lines. Mutation: 83 hand mutants, 69 killed, 14 survived (8 equivalent, 6 real test gaps). Engineer's own: 61 mutants, 60 killed `[RAN, engineer]`.
- All of the above is for the code **before** the 2026-11-24 rulings.
- *Update 2026-11-24, engineer's evidence for the code after the rulings `[RAN, engineer agent]`, not re-run by me, not reviewed:* `cargo test -p chess-store` 440 tests, and with `--features test-support` 515, 5 runs each, all exit 0; the kill race binary (500 SIGKILL rounds per run) 10 runs, all exit 0; `cargo fmt --check` and `clippy -D warnings` clean in both feature settings; `node ci/check-crate-edges.mjs` and `CHESS_CI_OFFLINE=1 ci/check.sh` exit 0 (npm audit skipped by the offline flag). Tests were written before the code where the API existed (four format tests failed on the old code); for the backlog policy the API did not exist, so the proof is 37 hand mutants, all killed. Not run: power loss, Intel Mac, the Reviewer pass, D4 behaviour (restart reconcile of a parked game).

### 4.5 Residual risks and limits

- Power loss `[UNV]`; the plan applier is beyond its documented contract until D4 calls it; two `Store`s in one process have independent backlogs (D3R-15); backlog lost on exit is recoverable only by D4, which does not exist (D3R-14).
- The queue bound in D-145 (256 per game, 4 096 in all) is an **ESTIMATE** derived by scaling, not measured. The unpark behaviour and the list of transient SQLite codes are the architect's reading and the engineer's list, both for Brian's G2 review.
- Stale or duplicate batches still rewrite rows (documented; D4 and resume must pass monotonic, whole journals).
- *Update 2026-11-24:* an unpark that is followed by a partial re-pass skips the dropped records and nothing stops it (caller contract only). `sync_backlog` now fails for **every** caller while any game is parked, so an export barrier is blocked by one parked game until unpark or restart (intended; say so to the app owner). A parked game cannot be ended through the store (live finalize, abort, void, reset refused). The restart that is the exit of last resort depends on D4, which is not built. The flush costs one extra attempt per live terminal command when a queue exists `[UNV]`, not measured.

### 4.6 Decisions Brian already made that this gate embodies

| Date | Decision bead | Decisions |
|---|---|---|
| 2026-11-04 to 2026-11-09 | OQ-16 closure, chess-a0r.3.34 and Addendum H | D-66 (StoreAhead), D-72 (finalize order), D-87 (`GameEnd` with `Voided`), D-92 (ply base), D-94 |
| 2026-11-23 | chess-a0r.3.52 | D-138 (latch through apply), D-139 (shape only) |
| **2026-11-24** | **chess-a0r.3.56** | **D-144** `ended_at` refused at encode and decode for `GameEnd`, `Abort`, `Voided` and `Start.wall_ms`, live void paths the same, app floors its clock at 1 ms (G3). **D-145** park on the first permanent error, keep returning the error, inspect and unpark, bounded queue, distinct `sync_backlog` result, transient codes transient (G3). **D-146** `matchup` is `hh`, `hc` or `cc`, checked against the kinds (G3). **D-147** `journal_seq` is the larger of stored and last `seq` (G3 value, G0 text). **D-148** new public surface (G2). **D-149** repair pair deleted (G1). **D-150** F2-01, F2-02 (G1). **D-151** TMPDIR accepted (G2). **D-152** takeback deletes `move_eval`, store flushes before live terminal commands, whole journal in one batch (G1). **D-153** B-R03, B-R05 in D4 (G1) |

### 4.7 Open items, class and recommendation

| Item | Owner | Class | State |
|---|---|---|---|
| D-144 `ended_at` and D-146 `matchup` in `format.rs`; live void checks; repair pair deleted | Engineer | G3 / G1 | Visible in the tree `[OBS]`, no evidence, no review |
| D-145 park on first error, queue bound, `unpark_game` and parked info, distinct `sync_backlog` result, transient classification | Engineer | G3 / G2 | **Built** per the engineer's evidence `[OBS code, RAN by engineer]`, not reviewed; names, return types, bounds and transient list need Brian's G2 look (table in 4.2) |
| D-152 queue drain before live finalize, abort, void, reset | Engineer | G1 | **Built**, stricter than the ruling (`GameParked` as well as `Busy`); not reviewed; engineer judgement call, see 4.2 |
| D-148 `#[non_exhaustive]` on `BacklogStatus`; SDD 3.7.2 rows | Engineer, architect | G2 | SDD text entered 2026-11-24 and amended 2026-11-24 for the built behaviour (store SDD 3.7.2 and 3.12, app SDD 11.5, architecture Addendum N dated note). `BacklogStatus` and the new types are `#[non_exhaustive]` in the code `[OBS]` |
| Tests that fail on today's behaviour: `a_refused_first_attempt_parks_the_game`, `ended_at` refusal at encode and decode, `matchup` mismatch, kill after the drain | Engineer | | Engineer reports them written first where the API existed (four format tests failed on the old code) and 37 hand mutants killed `[RAN, engineer agent]`; I did not see the runs |
| D3R-03 (`sync_backlog` `Busy` with a free lock), D3R-04, D3R-08, D3R-10 survivors | Engineer | Minor | D3R-03, -04, -08 built (waiter pass, timer rule, waiter cleanup) with tests; survivors dispositioned by hand mutants in the engineer's report; the fresh reviewer must confirm |
| F2-09 answer in SDD 3.11.4 | Architect | G0 | Entered 2026-11-24 |
| The gate record: says "Reviewer pass NOT DONE", names no D-144 to D-153 | PM | G0 | Stale; the review exists |

**Recommendation: not yet** (still; updated 2026-11-24). Class G3 and G2, approver Brian. The two Majors are **ruled and now built** (D-145 policy, D-152 flush, with the differences listed in 4.2), and the engineer reports passing tests and 37 killed mutants `[RAN, engineer agent]`, but **no independent look at the post-ruling code exists**: the fresh re-review report `review-chess-store-D3-rulings.md` is not in `docs/engineering/` `[OBS]`. Re-present when: (1) that fresh reviewer has re-checked the changed files (`format.rs`, `lifecycle.rs`, `apply.rs`, `thread.rs`, `backlog.rs`, `store/d3.rs`, `error.rs`, `api.rs`, `repo/game.rs`, `stats.rs`) and the kill rounds at `apply.*`, and the D2, D5 and B suites are re-run green by someone other than the author (GATE-111 edits their files); (2) Brian sees the G2 list: `ParkCause`, `ParkedGame`, `GameParked`, `BacklogFull`, the `unpark_game` return type and the unenforced re-pass contract, the two bound constants, the transient list, `sync_backlog` failing at once for any parked game; (3) Brian accepts or rejects the engineer judgement calls (stricter flush; `set_analysis_used` guard), which I recommend accepting. If Brian wants to approve before (1), the honest reading is "approve the stored values (D-144, D-146, `journal_seq`, the `move_eval` deletion) as built and tested by the engineer, pending the fresh review", which I do not recommend, because the policy that decides whether a game can be stuck is the part with no independent look. Confidence High.

## 5. Across all four gates

- **Never approved by an agent.** GATE-108 to GATE-111 stay Brian's. A desk review by an agent does not stand in for him. The independent-reviewer step for v1 was waived in the gate records (FIND-016); in practice fresh reviewer passes were done for B, D2, D5 and D3, so the records are stale in saying "NOT DONE".
- **Process rule at close.** Brian commits himself (decision bead .3.56). The PM must not commit or push.
- **Items without an owner decision:** B-R04, D5R-04, and the names, bound and transient-code list of D-145. *(2026-11-24: F2-05 and F2-07 are built by the engineer, see 1.7, 2.7 and 4.2; they still need Brian's acceptance of the engineer's choice.)*
- **Failpoints:** `apply.*` are wired and kill-tested; eight registered names still have no call site (owners in architecture 11.4.22).

## 6. Not verified and not analyzed

- I did not run any test, build or tool. Every `[RAN]` figure is from a reviewer or engineer document named in the section; I did not re-check them against the code.
- The reading of the tree (section 0 and 4.7) is by grep and file times; the engineer may have changed more since.
- The gate class of two relayed rulings (D-142 items 2 and 3) and the exact wording of the relayed rulings are not in the bead text beyond what is quoted in architecture 11.4.22 (no verbatim quote exists).
- Power loss, Intel Macs, a sandboxed app, sanitizers, `cargo deny` and the vulnerability scan (outside this task).
- I did not read the front-end SDDs or `chess-app` code (not built).
- `store-rework-specs.md` was not edited (outside my edit list); its section 2.3 and the NF-4 wording are superseded by store SDD note 29.
