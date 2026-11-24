# Decision package: chess-store part D3 (apply path), findings FIND-D3-01 to 11 and F2-01, F2-02, F2-06

Architect stance, 2026-11-23, baseline B6, GATE-111 (G3, OPEN, NOT APPROVED). For **Brian Prascher** where a gate class below says G2 or G3. **Nothing here is decided, reviewed or approved.** Recommendations only; Brian declined delegation. Sources read: [step2-store-D3-evidence.md](step2-store-D3-evidence.md), [review-chess-store-fixes-2.md](review-chess-store-fixes-2.md), [GATE-111](gates/GATE-111-chess-store-D3.md), `crates/chess-store/src/{repo/apply.rs, store/d3.rs, thread.rs, backlog.rs, repo/lifecycle.rs}`, `journal/format.rs` (grep), SDD-chess-store 3.7.2, 3.11.1, 3.11.4, 3.12 and the dated notes of 2026-11-22, architecture D-38, D-40, D-66, D-72, D-87, D-92, D-128, D-129. I did not re-run the test suites. Tags: `[OBS]` read by me, `[RAN]` run by me, `[INF]` inferred, `[UNV]` not verified.

**Door type, once.** Nothing is released and `journal_version` is 1 `[OBS]`. Every item that changes a journal rule is **two-way now and one-way at the first release** (Addendum M note (3): a change of range or meaning bumps `journal_version`, so a later fix needs a version bump and a dual-version reader). Items that change only store code or text are two-way. That is why FIND-D3-01 and FIND-D3-06 should be ruled before the first release, not after.

## Summary

| Item | Recommendation | Gate class | Door | Confidence |
|---|---|---|---|---|
| (a) FIND-D3-01 = F2-04, `ended_at <= 0` | **Option A: the journal format refuses `ended_at <= 0` for `GameEnd`, `Abort` and `Voided` at encode and at decode; the app floors its clock at 1 ms.** Store keeps its refusal as a second line | **G3** (journal range) | One-way at release | Medium-High |
| (b) FIND-D3-02 uncurable batch | **Park the game on the first permanent error too** (as built it parks only on a retry); keep returning the error; list the behaviour in SDD 3.12 | G1 code and text; the observable result is part of (d) G2 | Two-way | Medium |
| (c) FIND-D3-07 `journal_seq` | **The built rule is right: `max(stored, last seq)` for a batch, exact for a plan.** Amend the SDD wording | G3 as a stored-value rule of GATE-111, text G0 | Two-way (code) | High |
| (d) FIND-D3-11 new public API | **Accept the five items**, with `#[non_exhaustive]` added to `BacklogStatus`, and list `backlog_status`, `ApplyOutcome`, `BacklogStatus` in SDD 3.7.2 | **G2** | Two-way before the app consumes them | High |
| (e) FIND-D3-03 `void_game_repair_pair` | **Delete the name; inline its UPDATE into `void_game` after the precondition.** Tests use raw SQL for the settled-and-voided fixture | G1 | Two-way | High |
| (f) FIND-D3-04, 06, 09, 10 | D3-04 accept and document (SA text, app consumer note). D3-06 text form `hh`, `hc`, `cc`, checked against the two kinds at encode and decode (SA text + G3 + engineer). D3-09 the **store** flushes the game's queue before a live terminal command (engineer). D3-10 **D4 and `chess-app` resume** own it (task .3.40, .3.12) | See section (f) | D3-06 one-way at release; others two-way | Medium to High |
| (g) F2-01, F2-02, F2-06 | F2-01 exclude imported games in the statistics SQL. F2-02 `NULLS LAST` plus a test. F2-06 accept the `$TMPDIR` dependency and state it in SDD note 21 | G1, G1 (Brian informed: it amends a ruled order), G2 risk acceptance by Brian | Two-way | High, Medium, Medium-High |

## (a) FIND-D3-01 = FIND-F2-04: the journal accepts an end time the store refuses

**Problem.** `GameEnd.ended_at`, `Abort.ended_at` and `Voided.ended_at` are plain `i64` in `journal/format.rs` `[OBS review F2-04]`. `finalize_in_tx` and `abort_in_tx` refuse `ended_at <= 0` with `InvariantViolated` (`finalize.rs:144`, `:245` `[OBS]`). `apply_records` runs those bodies, so a journal that holds such a record decodes, replays and then cannot be applied: the batch rolls back, nothing is queued, and the next start meets the same refusal. This is the CH-4 defect class: the writer may write what the consumer refuses.

**Options.**

| Option | What it is | Trade-offs |
|---|---|---|
| **A (recommended)** | `Payload::validate()` (encode) and the decoder refuse `ended_at <= 0` for `GameEnd`, `Abort` and `Voided`. The live store entry points (`void_game`, `void_unreadable_game`) get the same `> 0` check. The app floors its wall clock at 1 ms at one seam, so a real clock never reaches the bound | Same pattern as D-128 (CH-4). One rule for all three terminal paths. The store's own check stays as a second line (unreachable from a decoded record `[INF]`). Cost: one decode rule, two or three tests, a changed test builder. A clock that returns 0 or less now fails the append, so the app must floor it (consequence below) |
| B | Decode refuses, encode does not | Worst case: a legitimate game end is acknowledged, then read back as damage (CH-5). Same defect class as before, moved to the reader. Rejected |
| C | The store accepts: clamp at apply (e.g. to 1) or relax the store check to `>= 0` | Clamp stores a time nobody observed and changes D-72 ordering (finalize ascending by `ended_at`). Relaxing to `>= 0` admits 1970-01-01 as an end time and drops a useful guard against an unset time. Leaves the journal and the store disagreeing on what a valid value is for `Voided`. Rejected |
| D | The app guarantees `> 0`, no format rule | Cheapest, but nothing stops a writer bug or a foreign or damaged journal, and the store cannot tell. The CH-4 ruling already chose the format as the place for such rules. Rejected |

**Recommendation: A.** Confidence Medium-High: it follows the ruled CH-4 pattern, and the one open doubt is whether a failed append for a bad clock is better than a clamp, which I judge it is because a clock at or below zero is a defect worth seeing.

**Door.** One-way at the first release (a range rule of a stored format; Addendum M (3) requires a `journal_version` bump to tighten it later). Nothing is released, so the cost now is the test builder and about three tests.

**Interplay with CH-5 B (D-129).** With A, a CRC-valid last record with `ended_at <= 0` is refused by the decoder and, with no intact frame after it, reported as `UndecodableTail` damage: the journal moves to `journal/damaged/`, the rebase builds from the intact run and the store, and the user is told through `JournalUnreadable`. That is the right treatment for a record that no build of this code writes. A `GameEnd` followed by a `Voided` whose `GameEnd` is refused becomes `Middle` damage the same way. Without encode refusal (Option B) the same path would fire for a record this build itself wrote and acknowledged, which is the case D-129 exists to avoid.

**Interplay with D4 and resume.** No new D4 code: the existing tail-damage path applies. The game then replays as in progress from the intact moves; an acknowledged `GameEnd` that is damaged is lost as a result (a resignation or a time forfeit is not re-derivable from the moves), which is acceptable only because A keeps the writer from producing one. D3 keeps `InvariantViolated` for a `JournalRecord` built in memory by a caller that bypasses the codec.

**Consequences.** D4 and C: none beyond the above. `chess-app` (.3.12): the assembler for `GameEnd`, `Abort` and `Voided` takes `ended_at` from the injected clock with a floor of 1 ms; an encode error of the append is `Internal`, not a user message. Tests: `journal_accepts_ended_at_the_store_refuses` becomes a refusal test; `a_terminal_record_with_a_non_positive_ended_at_is_refused_and_writes_nothing` stays (second line). SDD: dated note at 3.11.1 (G3 text, Brian), D-number from the next free ID in the decision log.

## (b) FIND-D3-02: a batch that no retry cures

**What is built `[OBS thread.rs 326-347, backlog.rs 17, 45, 144]`.** First attempt with a permanent error (an invalid record, an unknown player, a `Move` for a game with no row): `apply` returns the error and queues nothing, so later batches of the same game still apply. Retry attempt with a permanent error: the game is parked, its queue is kept in order, nothing is dropped, `BacklogStatus.parked_games` counts it, the backlog stays non-empty and `sync_backlog` gives `Busy`. The park is in memory.

**Problem.** The two paths differ. After a first-attempt error the journal holds the record and the database does not, but the next batch of that game applies on top. A later `GameEnd` then finalizes a game with a missing move or clock, silently; the next reconcile (start or resume) repairs only a game whose journal is still open `[INF]`. After a retry error the game stops, which is the safer behaviour.

| Option | Trade-offs |
|---|---|
| **A (recommended): park on the first permanent error as well.** Keep the refused batch at the head of the game's queue, return the error to the caller, apply nothing behind it for that game until the next start | One rule for both paths. Play is unaffected (the journal is first). The database lags for that game until the next start, where reconcile compares the journal with the rows and the game's journal is rebuilt or applied again. Cost: a small change in `thread.rs apply()`, one test |
| B (as built) | Smaller change, but a gap can reach a finalize |
| C: drop the batch and report | Loses the ordering guarantee; a later record applies without its predecessor, as in B, and nothing remains to retry |

**Recommendation: A.** Confidence Medium: I have not weighed how often a permanent error occurs; I expect it only for a defect, so the cost of a lagging game is small and a silent gap is the worse outcome.

**Door.** Two-way (in-memory behaviour, no stored value). **Gate.** G1 inside the crate; the observable result (`Err`, then `Busy`, `parked_games`) belongs to the G2 acceptance in (d).
**Consequences.** `chess-app`: a non-zero `parked_games` is a damage-class condition, not a retry state; the app SDD must say what the user and the log see (today only `StoreBacklog` exists, which is an age, not a parked game). I do not decide the event mapping; it is a G2 item for the app SDD. D4: the startup scan re-applies from the journal, which clears the park (it is in memory); with (a) fixed the known cause of a permanent error is gone. SDD 3.12: add a row "Uncurable batch".

## (c) FIND-D3-07: `journal_seq` after a batch

**Question.** SDD 3.7.2 and 3.11.4 say "set `journal_seq` to the record's `seq`". The code writes `max(stored, last seq)` for a batch and sets the plan's value exactly `[OBS apply.rs 116-121, 464-470]`.

**Which is right.** The built rule, for these reasons.

1. **D-66** treats `db_seq > last_intact_seq` as damage (the store holds a move the journal no longer proves). That works only if `journal_seq` is the highest sequence the store has applied. A stale or replayed batch that lowered it (a restore applying an older run, a retry of an early batch behind a later applied one) would hide a `StoreAhead` after a truncation: store applied to 10, a replay of seq 5 sets 5, the journal is cut to 8, and `5 < 8` reports nothing while the store holds rows of 9 and 10. `[INF]`
2. **D-72** orders startup finalizes by `ended_at`, not by `journal_seq`; it does not constrain the rule.
3. The one place that must lower it is the rebase (SDD 3.11.3 steps 2 and 3, `GAME.journal_seq = N`): the plan applier does that exactly, on purpose, in a transaction of its own. Both rules are consistent with that.
4. A batch of a normal run has `last seq >= stored`, so the two readings give the same value; they differ only for a replay or a stale batch, where "never lower" is the safe side.

**Recommendation.** Keep the code. Amend the SDD text (dated note, no row rewritten): for `apply_records` read "set to the larger of the stored value and the last record's `seq`"; the plan applier sets it exactly. FIND-D3-08 (a stale batch alone moves clocks back) stays Info: the journal wins on clocks.

**Gate.** The value rule appears in GATE-111's list of stored values, so Brian's G3 approval covers it. Text G0. **Door:** two-way (reader and writer code, no byte of a file changes). **Confidence:** High.

## (d) FIND-D3-11: new public surface (G2)

New public items `[OBS lib.rs, api.rs, store/d3.rs]`: `Store::apply_records`, `Store::sync_backlog`, `Store::backlog_status`, `ApplyOutcome` (`#[non_exhaustive]`: `Applied`, `Queued`), `BacklogStatus` (five public fields, no `#[non_exhaustive]`), and, only with feature `test-support`, `StoreOptions::with_monotonic_clock` and `MonotonicClock`. `StoreOptions` gains one private field; `StoreOptions::new` is the constructor.

**Recommendation: accept all five, with two changes.** (1) Add `#[non_exhaustive]` to `BacklogStatus` before `chess-app` reads it, so a later field does not break a consumer; it derives `Default`, so tests still build one. (2) SDD 3.7.2 gets rows for `backlog_status`, `ApplyOutcome` and `BacklogStatus` (`apply_records` and `sync_backlog` are already listed at lines 483 and 497 `[OBS]`); the `apply_records` row says it returns `Applied` or `Queued` and what an `Err` means. The test-support items stay off the public list and are marked as such. Reason: `backlog_status` is the only way for the app to emit `StoreBacklog` (architecture 4.8, polling), and the five items match D-39 (a concrete handle, no `dyn`). Confidence High. **Gate: G2** (an interface two crates share). **Door:** two-way until `chess-app` code uses it. **Consequence for the app:** `StoreBacklog` is derived by polling `store_backlog`; the app SDD names the poll interval (not set here).

## (e) FIND-D3-03: `void_game_repair_pair`

`void_game` calls it after the settled-ratings precondition `[OBS lifecycle.rs 90-92]`, so it has no *external* caller, and the only other uses are two D2 tests that build a settled-and-voided fixture. D-87 does not need it: a `GameEnd` followed by a `Voided` is skipped before the void, so no rating is settled, and `reset_player` voids only in-progress games.

**Recommendation: delete the name.** Move its `UPDATE` and `release_and_queue` into `void_game` after the precondition; the two tests insert the fixture with SQL. This removes an unguarded bypass with a comment that says "nothing else may call it" and `#[allow(dead_code)]` that no longer holds. Alternative (keep, rename, add a `#[cfg(test)]`): leaves a bypass in the crate. Confidence High. **G1** (inside one crate, D2 file). **Two-way.** F2-09's question (what a voided game that kept settled ratings means) is answered by D3 as built: a `Voided` on a settled row is refused. **Consequence:** with (b) a refused `Voided` parks that game; D4 must treat it as damage.

## (f) Owners of FIND-D3-04, 06, 09, 10

| Finding | Recommendation | Owner | Gate | Door |
|---|---|---|---|---|
| D3-04 takeback deletes `move_eval` rows | **Accept.** The foreign key would refuse the delete otherwise, and an evaluation of a removed ply has no meaning. Add the sentence to the `Takeback` row of SDD 3.11.1 and to the plan applier text | Text: system architect (dated note). Consumer: `chess-app` analysis service must treat evaluations as recomputable after a takeback | G0 text; the deleted rows are stored content, so it is inside GATE-111's G3 | Two-way |
| D3-06 `Start.matchup` text | **The text form is the database's codes `hh`, `hc`, `cc`** (the table CHECK at SDD line 298). Format validation requires it to equal the code derived from `white_kind` and `black_kind` at encode and decode; the store keeps deriving the column from the kinds. The test builder changes from `human-human` to the code | Text: system architect. Code: software engineer (format.rs, test builder). App: assembler writes it from the two kinds | **G3** (journal field text; Brian) | **One-way at release** |
| D3-09 flush before a live terminal command | **The store flushes** the game's queue before `finalize_game`, `abort_game`, `void_unreadable_game` and `reset_player` for that game, on the store thread, and returns `Busy` if it cannot drain. The app's `sync_backlog` before `prepare_void` (CT-APP-32 (a)) stays as it is | Software engineer, `thread.rs` and `store.rs` | G1 | Two-way |
| D3-10 whole journal in one batch | **D4 (restore, startup scan) and `chess-app` resume pass the whole journal, or at least a `GameEnd` with its `Voided`, in one `apply_records` call.** Add it to D4's acceptance and to the app resume text | D4 task .3.40; app part A .3.12 | G1 | Two-way |

**Why the store for D3-09.** The store thread already owns the connection, the backlog and every live command, so one place can keep per-game order; an app rule would have to be remembered at four call sites and `sync_backlog` waits for every game and answers `Busy` for a parked one. Confidence Medium. **Why `hh|hc|cc` for D3-06.** It is the form already in the schema CHECK, so one spelling exists in the whole system; the SDD says `matchup` is in the payload but not its text `[OBS]`, and the store ignores it, which is how a wrong value would otherwise go unnoticed (the CH-2 pattern). The alternative, dropping the field from `Start`, would be an additive change that older readers ignore; I do not recommend it because the SDD and app SDD list the field and the app writes it. Confidence Medium-High. FIND-D3-05 (a restored game has no slot rows until the last pass of SDD 3.11.10) and 12 to 16 are Info and need no ruling.

## (g) F2-01, F2-02, F2-06

**F2-01, imported game with both seats makes statistics `Corrupt` (latent).** The schema allows `status = 'imported'` with seats `[OBS 0001_init.sql, review]`; RULE-7 (requirements.md line 519) says an imported game never changes any statistic. **Recommendation: exclude imported games in the SQL of `stats_rows` and `head_to_head_rows` (`g.status <> 'imported'`), fix the comment at `stats.rs:151`, add a test with an imported game that has seats.** This is the requirement, not a new policy. G1 (D5 file). Two-way. Confidence High. A hard failure of every statistic of a Player is worse than a skipped row; the filter makes the latent case harmless before an import path exists.

**F2-02, NULL `started_at` sorts first.** SQLite puts NULL first, so an imported game without a start is among the "oldest N" of `list_games` and `games_for_history`, and drops out of a windowed history. **Recommendation: `ORDER BY g.started_at IS NULL, g.started_at, g.id` (NULL last), state it in SDD 3.7.2, pin with a test that has an imported row.** Reason: "oldest N" should mean the oldest played games; an imported game has no play time. A reset window (`from`) still excludes it, which is acceptable for a game that has no Player-specific time; the history list "includes everything" only without a window. G1 code, but the `(started_at, id)` order was ruled (FIND-RW2-02), so Brian is told. Two-way. Confidence Medium. Also add the F2-03 tie-break tests (two equal `started_at`, `limit 1`) in the same change.

**F2-06, NF-2 needs a usable `$TMPDIR`.** Without one, a killed first launch that left a hot rollback journal is refused `Unavailable{ReadOnly}` for ever (about 2 in 700 kills `[RAN review]`). **Recommendation: accept the dependency and state it in SDD 3.9.1 note 21 and the risk register, with the cause visible in the error text.** Reasons: macOS gives each user a writable per-user temp directory by default, a scratch copy inside the data directory would write during an open that is trying to be read-only, and the effect needs two unlikely events together. Revisit if a second report appears. **G2 (acceptance of a residual risk against NF-2; Brian)**. Two-way. Confidence Medium-High.

## What I did not analyze

Power-loss behavior and Intel Macs (not testable here). Whether the app should surface `parked_games` as an event or a log line only (app SDD, G2). The performance cost of the per-game flush in (f). `void_unreadable_game` and `reset_player` interplay with a non-empty backlog beyond D3-09. A Reviewer pass on D3 has not happened. No test was run by me; counts are the engineer's `[RAN]` figures in the evidence file.

## For Brian (all are recommendations)

1. **G3:** (a) `ended_at` rule in the journal format; (f) `Start.matchup` text form; (c) `journal_seq` value rule (already inside GATE-111).
2. **G2:** (d) the new public surface; (g) F2-06 residual risk.
3. **Told, not asked (G1):** (b), (e), (f) D3-09 and D3-10, (g) F2-01 and F2-02.
