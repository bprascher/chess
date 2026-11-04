# Board pass: stored-format and journal decisions (OQ-16)

| Field | Value |
|---|---|
| Document | docs/design/Board-pass-store-journal.md |
| Task | chess-a0r.1.2, independent Review Board pass on D-41 to D-43 and the Addendum A scope (OQ-16, GATE-006, GATE-007) |
| Baseline | B1 (plan v1), repository state at commit 709f0ba plus uncommitted working-tree files |
| Date | 2026-11-04 |
| Stance | **Reviewer only.** Findings, no fixes, no approvals. No architecture, SDD or requirements file was edited |
| Reviewer | `software-engineer` agent, fresh invocation, no context shared with the SDD authors (independence limits in section 2) |
| Report to | Brian Prascher (G-1 gate). This document approves nothing. All dispositions belong to Brian |
| This document's own gate | G0 (new review document, no design effect) |
| Finding prefix | `BP-nnn` (new; no existing ID reused or renumbered) |

Evidence tags: `[OBS file:line]` read in this session; `[RAN]` executed here; `[INF]` inferred from observed text, reasoning stated; `[UNV]` not verified. Severity follows `code-review-conformance` (Blocker, Major, Minor, Note). Confidence: CONFIRMED (document contradiction shown by quotation), LIKELY (strong reading, no code exists to run), SUSPECTED. No code exists, so no finding is `[RAN]` except the D-43 arithmetic check.

## 1. Verdict

**PROCEED WITH CHANGES.** No finding invalidates the direction (journal first, SQLite second, journal wins, one `Fact` to one record kind). But five Major findings (BP-001 to BP-005) are defects in the stored-format batch or in crash recovery, and the first and fifth change journal or schema content. Those are cheap to fix now (nothing is released) and expensive after release (D-41 is one-way at release, architecture.md:2148). **I recommend that no `chess-store` code that touches the journal payloads, `GAME` DDL or recovery starts until Brian has dispositioned BP-001 to BP-005.** Store work that does not depend on them (data directory, open and migrate framework, backups, heartbeat registry) is not affected by any finding; that is Brian's call, not mine (OQ-16 as written blocks all store code).

| Severity | Count | IDs |
|---|---|---|
| Blocker | 0 | |
| Major | 5 | BP-001, BP-002, BP-003, BP-004, BP-005 |
| Minor | 11 | BP-006 to BP-016 |
| Note | 1 | BP-017 |

Per-decision verdicts:

| Decision | Verdict |
|---|---|
| D-41 journal contract and stored-format batch | ACCEPT-WITH-CHANGES (BP-001, BP-005, BP-009, BP-010, BP-016) |
| D-42 `content_hash` | ACCEPT-WITH-CHANGES (BP-006, BP-007, BP-008) |
| D-43 `Personality` / `error_permille` | ACCEPT (BP-016 is an optional DB backstop only) |
| `content_hash` in `GameEnd`, `Abort`, `Voided`, `VoidSpec` | ACCEPT-WITH-CHANGES (BP-008, BP-010) |
| Per-kind payload table (arch 7.6) | ACCEPT-WITH-CHANGES. The four documents agree with each other; the table is incomplete (BP-001, BP-005, BP-009) |
| Seven flags, OQ-22 | See section 5: F1 partly rejected, F4 and F6 changed, F2, F3, F5 accepted, F7 accepted with conditions |
| D-57 calibration owner | ACCEPT-WITH-CONDITIONS (BP-015) |
| D-58 crash recovery | ACCEPT-WITH-CHANGES (BP-002, BP-003, BP-011) |
| OQ-20 | Recommend option A with conditions (section 6, BP-008) |

## 2. Scope, method, independence, limits

**Scope (OQ-16 and GATE-007, architecture.md:2233, 2104):** D-41, D-42, D-43; `content_hash` in `GameEnd`, `Abort`, `Voided` and `VoidSpec`; the per-kind payload table and its seven flags (arch 7.6, OQ-22); D-57; D-58; OQ-20.

**Read:** architecture.md 7.1, 7.2, 7.4, 7.6, 11 (D-23, D-30, D-34, D-35, D-40 to D-43, D-57, D-58), 11.2 door rows, 11.3 GATE-006/007, 14 (OQ-16, OQ-20, OQ-22); SDD-chess-store.md 3.5 to 3.13, 4, 7, 8, 9; SDD-chess-domain.md 3.3, 3.5, 3.9, 3.10, 3.11.1, 3.11.2, 3.11.11; SDD-chess-app.md ALG-APP-01, ALG-APP-09, DD-APP-09, DD-APP-18, DD-APP-19, DD-APP-20, SEQ-APP-08/09, reset and resume rows; SDD-chess-rules.md 3.11.10; SDD-v0.3-joint-review.md section 6; requirements NFR-2, DAT-3, DAT-5, RULE-8, PLY rename text; SWE-001 engagement profile.

**Independence (stated plainly).** I am a fresh invocation with none of the authors' context, and I did not write the architecture or the SDDs. But I am the same agent type (and model family) that the SDDs name as their Engineer stance (SDD-chess-store.md:1050). OQ-16 asks for "a reviewer role distinct from the architect" (architecture.md:2233); that is met. Whether a same-family reviewer is independent enough for GATE-006 tier R2 is Brian's judgment. This is a desk review.

**Lenses used:** durability and data integrity; cross-document consistency; failure modes (FMEA-lite); decision hygiene and reversibility. **Not done:** a lightweight ATAM, a security lens (v1 is local, SWE-001 FIND-016), a performance lens. GATE-006 says R2 asks for four or more lenses and an ATAM (architecture.md:2102); this pass does not claim to satisfy that on its own.

**Not run:** no code exists, so nothing was built or tested. The only execution was the D-43 arithmetic check in section 4.3.

## 3. Scope disposition table

Every OQ-16 scope item, with severity (highest linked finding), evidence tag, verdict and proposed gate tier. Tiers are proposals under SWE-001 section 3 and are not approvals. The stored-format rule of SWE-001 (row "Irreversible work") makes any journal or schema content change G3.

| # | Scope item | Severity | Evidence | Verdict | Proposed tier | Findings |
|---|---|---|---|---|---|---|
| S1 | D-41 journal contract and stored-format batch | Major | [OBS arch:1993, 1397] [INF] | ACCEPT-WITH-CHANGES | G3 (content), G2 (recovery behaviour) | BP-001, 002, 003, 005, 009, 010, 016 |
| S2 | D-42 `content_hash` algorithm and canonical form | Minor | [OBS arch:1994; rules:596-606] [INF] | ACCEPT-WITH-CHANGES | G3 if the value format or tag set changes; otherwise G0/G1 text | BP-006, 007 |
| S3 | D-43 `error_permille` and `personality_to_engine` | Note | [OBS arch:1995; SDD-store:275] [RAN] | ACCEPT | G0 (optional DB CHECK is G3) | BP-016 |
| S4 | `content_hash` in `GameEnd`, `Abort`, `Voided`, `VoidSpec` | Minor | [OBS arch:1540, SDD-store:466, 745] [INF] | ACCEPT-WITH-CHANGES | G3 (payload), G1 (text) | BP-008, 009, 010 |
| S5 | Per-kind payload table | Major | [OBS arch:1527-1538; SDD-store:480] | ACCEPT-WITH-CHANGES. The table is consistent across four documents but does not carry the finalize time inputs | G3 | BP-001, 005, 009, 010 |
| S6 | Seven flags (F1 to F7) | Major (F1), Minor (F4, F6) | see section 5 | per flag | per flag | BP-001, 005, 008, 009, 017 |
| S7 | D-57 | Minor | [OBS arch:2009; SDD-app DD-APP-20] [INF] | ACCEPT-WITH-CONDITIONS | G1 (G2 for the new public items already recorded) | BP-015 |
| S8 | D-58 | Major | [OBS arch:1570, 2010; domain:620; app:727-757] [INF] | ACCEPT-WITH-CHANGES | G2 (domain and app behaviour), G1 (text) | BP-002, 003, 011, 012, 013 |
| S9 | OQ-20 | Minor | [OBS arch:2237, 1549] [INF] | Option A with conditions | G1 (conditions), G3 only if option C is chosen | BP-008 |

## 4. Targeted checks

### 4.1 Does the Fact-to-journal round trip hold?

**At the `Fact` level, yes, on the fields compared.** The four tables (arch 7.6 at architecture.md:1527-1538, domain 3.11.1 at SDD-chess-domain.md:601-612, store 3.11.1 at SDD-chess-store.md:734-745, app DD-APP-18 at SDD-chess-app.md:1172-1183) are equal column for column on kinds, `Fact` fields and codec-added fields `[OBS]`. The store table adds a "DB effect" column. I found no `Fact` field without a record field. `Move.mv` to `uci` is lossless because `Move::from_uci` needs no position (SDD-chess-rules.md:275, 311 `[OBS]`; behaviour of the implementation `[UNV]`).

**The claim "FF-3 holds by construction" (arch:1525) is true only at that level and is overstated.** FF-3 proves `record_to_fact(fact_to_record(f)) == f`. It proves nothing about the other direction the design depends on: that **a journal alone can rebuild the stored `GAME` row** at crash replay and at restore (arch:1540, SDD-store:480 "so that a restore can rebuild it from the journal"). That second round trip fails for the finalize time inputs (BP-001) and is undefined for several encodings (BP-005, BP-010). A property test on FF-3 will pass while those gaps remain.

Residual round-trip risks at the `Fact` level, all in BP-010: `StartPosition` has no definition (domain:217 class diagram only); how a standard start is written in `Start.start_fen` (null or the standard FEN) is not stated while the DDL says null means standard (SDD-store:295; arch:1302); `clock_spec` text canonical form; half-null clock pairs; `VoidReason` strings.

### 4.2 Is crash recovery (D-58) consistent with "no completed move lost" (NFR-2)?

NFR-2 text: no completed move lost after kill, crash, OS crash, hard reset, power loss, disconnect (requirements.md:1032). Each scenario below was walked through the documents.

| # | Scenario | Result | Evidence |
|---|---|---|---|
| 1 | Crash before the move record is fsynced | OK. The move was never acknowledged (ack follows fsync, ALG-APP-01 step 5) | [OBS app:842-843] |
| 2 | Crash after fsync, before the DB apply | OK. Replay applies records with `seq > journal_seq` | [OBS arch:1563-1564] |
| 3 | Crash mid-record (torn tail) | OK within the stated threat model; see row 7 for the case where it is not | [OBS SDD-store:755-768] |
| 4 | Crash between the `Move` record and the `Ended` record of the same mating, stalemating or drawing move | **Move kept (NFR-2 met); game outcome lost and the game resumes in a terminal position.** BP-002 | [OBS domain:469, app:841-842] [INF] |
| 5 | Second `append` of a multi-record step fails | **Durable state and in-memory state diverge.** BP-003 | [OBS app:841-843] [INF] |
| 6 | Store busy; backlog lost in a crash | OK. Journal wins | [OBS arch:1561] |
| 7 | Journal tail corrupted or lost while the DB already holds the move | **Reconcile deletes the DB row.** BP-004 | [OBS SDD-store:776] [INF] |
| 8 | Damaged middle record | OK. Longer-run rule, journal wins on disagreement, damaged file kept | [OBS arch:1568; SDD-store:777-778] |
| 9 | Older backup restored; journals replayed | OK. Journal wins; `done/` journals are retained until a verified backup holds the game | [OBS arch:1575-1583; SDD-store:833] |
| 10 | Crash during D-58's own `Suspend`, `Clock`, `SuspendResume` appends | OK as far as I can trace. After `Suspend` alone, the journal is a clean suspension and gets only `SuspendResume`; after `DrawDropped` alone, the next `decide(Suspend)` finds no offer pending `[INF]` | [OBS arch:1570] |
| 11 | `F_FULLFSYNC` unsupported on the data volume | Silent downgrade to `sync_all`. BP-014 | [OBS SDD-store:1028] |
| 12 | Game voided by reset while live in another process | Moves are kept (journalled and applied after the void); hash may be stale. OQ-20, BP-008 | [OBS arch:1419, 1549] |

Rows 4, 5 and 7 are the findings. Row 4 is the one I would fix first because it produces a wrong game result, not just a degraded recovery.

### 4.3 D-43 exactness check

The architecture says every permille value 0 to 1000 round-trips through the `f64` conversion (SDD-chess-app.md DD-APP-19; SDD-chess-domain.md UT-DOM-111). Checked with IEEE-754 double arithmetic:

```
python3 -c "bad=[p for p in range(1001) if round((p/1000.0)*1000)!=p]; print(bad); print(len(set(p/1000.0 for p in range(1001))))"
[]
1001
```

`[RAN]` in Python 3.15.2, not in Rust. Rust `f64` division and multiplication follow the same IEEE-754 rules, so the result should carry over, but that is `[UNV]` until `UT-APP-18` runs. All 1001 quotients are distinct. D-43 is sound.

### 4.4 Cross-document consistency of DDL and payloads

| Check | Result |
|---|---|
| Store DDL (SDD-store:255-356) columns against ER (arch:1264-1393), all 14 tables | Equal column for column `[OBS]`. Store adds `INTEGER` epoch ms (documented, SDD-store:388), CHECKs, indexes |
| Payload table in four documents | Equal (4.1) |
| `GameEnd` payload against `FinalizeRequest` | **MISMATCH.** `FinalizeRequest` has `white_time_used_ms`, `black_time_used_ms`, `total_active_ms` (SDD-store:480); `GameEnd` does not (arch:1536). BP-001 |
| `custom_start` | **MISMATCH.** In the `Start` record and in `StatsRow` / `GameFacts` (domain:370, 753); not a `GAME` column (SDD-store:287-314). BP-005 |
| App `void_unreadable(game)` against store `void_unreadable_game` | **MISMATCH.** App calls it without a hash (SDD-app:1014); store says the caller passes `content_hash` (SDD-store:466). BP-008 |
| `Voided` DB effect | **Inconsistent wording.** "none" (SDD-store:745) against "restore re-runs the void" (arch:1540; SDD-store:845). BP-009 |
| "Startup scan never appends" against the `Voided` repair bullet | **Inconsistent.** BP-011 |
| Architecture record framing (no sync marker, arch:1522) against store framing (`A5 5A` sync marker, SDD-store:726) | Refinement, acceptable; the store owns the format |
| `error_permille` in architecture, domain, store, app, engine | Consistent `[OBS]` |
| `MOVE_EVAL` White point of view, `VARIATION` shape | Consistent between architecture and store `[OBS]`; domain and app not compared `[UNV]` |

## 5. The seven flags (arch 7.6, OQ-22)

Flags F1 to F6 are the six items of OQ-22 (architecture.md:2239). F7 is OQ-20.

| Flag | Claim in arch 7.6 | Verdict | Severity | Evidence | Tier |
|---|---|---|---|---|---|
| F1 | Drop `resigning_side?`, "times" and `setup JSON` from `GameEnd` / `Start` | `resigning_side` drop: ACCEPT (loser of `result`, resignation is human-only, not c-c [OBS domain:586]). `setup JSON` drop: ACCEPT. **"times" drop: REJECT.** The finalize transaction needs the three time totals, and they are not derivable from `Move.think_ms` (BP-001) | Major | [OBS arch:1543; SDD-store:480] [INF] | G3 |
| F2 | `Pause`, `Resume`, `Suspend`, `SuspendResume` have empty payloads | ACCEPT. The `Clock` fact that follows carries the clocks. Optional `cause` on `Suspend`: decide now (BP-017) | Note | [OBS arch:1544] | G3 if a field is added, else none |
| F3 | `DrawDropped.cause` has no replay consumer; `by` is checked against the pending offer | ACCEPT. Define what a mismatch does (BP-017) | Note | [OBS arch:1545; domain:614] | G1 |
| F4 | `Start` gains `white_kind`, `black_kind`, `custom_start` | `white_kind`, `black_kind`: ACCEPT. `custom_start`: **CHANGE.** Its definition contradicts RULE-8 and CR-9.11, `StartPosition` is undefined, and the `GAME` table cannot hold it (BP-005) | Major | [OBS arch:1546; requirements:502-507] | G3 and a ruling by Brian |
| F5 | `Start.rated`, `Start.self_play` are redundant but kept, ignored by `record_to_fact` | ACCEPT, provided the rule is written: the frozen value is authoritative for the restored `GAME` row, replay never recomputes it. The store's finalize already reads `g.rated` (SDD-store:791) | Minor, folded into BP-005 | [OBS arch:1547] | G1 |
| F6 | `Voided` has no `ended_at` while `GameEnd` and `Abort` do | CHANGE: add `ended_at` to `Voided` now (BP-009) | Minor | [OBS arch:1548] | G3 |
| F7 | Hash of a game voided while live elsewhere may belong to a shorter move list | ACCEPT with conditions (section 7, BP-008) | Minor | [OBS arch:1549, 2237] | G1 |

## 6. Findings

### BP-001 [Major][LIKELY] `GameEnd` omits the finalize time inputs; flag F1 drops "times" without a replacement
- **Location:** architecture.md:1536 (`GameEnd` row), 1543 (flag 1), 1482 (finalize row "result + times"); SDD-chess-store.md:480 (`FinalizeRequest`), 797 (finalize UPDATE "times"); SDD-chess-app.md:1065 ("called with the same inputs"); requirements.md:613-614 (DAT-3).
- **Observation `[OBS]`:** `FinalizeRequest` carries `white_time_used_ms`, `black_time_used_ms`, `total_active_ms` and is described as carrying "every input of the transaction so that a restore can rebuild it from the journal". The `GameEnd` payload carries `result`, `reason`, `ended_at`, `eco`, `opening_name`, `analysis_used`, `content_hash` and nothing else. Flag 1 says "times" has no consumer because "per-move times are in `Move`". DAT-3 requires per-Player time spent and total elapsed time, excluding closed-app time, to be stored.
- **Why per-move times do not substitute `[INF]`:** (a) a resignation or flag after a long think has no `Move` record for that think; (b) a Training takeback drops `Move` plies from the fold (SDD-store:772) but the time already spent stays in the live totals; (c) time while a draw offer is pending or the game is paused is excluded from `think_ms` by the clock rules (domain T8, T11) but the definition of `total_active_ms` is not given in any document I read. A rebuild from `Move.think_ms` would produce different numbers from the live game.
- **Impact:** after a crash between `GameEnd` and the finalize commit, or after a restore, the stored times are either missing (NULL, violating DAT-3 and distorting UI-2 average game time) or recomputed differently from live play. No requirement says either is acceptable. The finalize call at crash replay has no source for three of its arguments.
- **Failure scenario:** human resigns after 4 minutes of thought on move 30; app is killed after `GameEnd` is fsynced and before finalize commits. On restart, the scan finalizes from `GameEnd`; `white_time_used_ms` can only be the sum of `think_ms` and misses the 4 minutes.
- **Options:** (a) add `white_time_used_ms`, `black_time_used_ms`, `total_active_ms` to `GameEnd` as assembler-added fields (`FinalizeInputs`), nullable for untimed games only if DAT-3 allows. (b) Define the three totals as pure functions of the `Move`, `Clock`, `Pause`, `Suspend` records and accept that a final unfinished think is not counted; this needs a requirement ruling on DAT-3. I recommend (a): it matches how `ended_at` and `eco` are already handled and removes the derivation question.
- **Verify the fix by:** a test that finalizes a game live and finalizes the same game from its journal alone, and compares all `GAME` columns (UT/CT in `chess-store` and `chess-app`); add the three field names to UT-ST-070 and UT-APP-24.
- **Proposed tier:** G3 (journal payload and `FinalizeInputs`), Brian approves. **Status:** OPEN.

### BP-002 [Major][LIKELY] A crash between the `Move` and `Ended` records leaves a terminal position that is never ended; D-58 resumes it as a live game
- **Location:** SDD-chess-domain.md:469 ("Step with fact Move, plus fact Ended when terminal. The actor journals the facts"), 618-624 (replay: `Ended` is a separate fact; termination is checked only inside `decide` after a move); SDD-chess-app.md:841-842 (one record per fact, appended one at a time); architecture.md:1570 (D-58).
- **Observation `[OBS]`:** a mating, stalemating, repetition, fifty-move or insufficient-material move produces two facts, `Move` then `Ended`. Each becomes its own record with its own `F_FULLFSYNC`. The `Move` record alone is a valid journal. Replay of `Move` without `Ended` leaves `GameState` non-terminal `[INF: evolve(Move) does not end the game; only the Ended fact does, domain:622]`. `replay` then reports `interrupted = true` and D-58 issues `decide(Suspend)` then `decide(SuspendResume)`. Nothing re-checks the termination rules on resume (T1 checks the start position only, domain:571). No test in the three SDDs covers this window (searched `UT-DOM`, `UT-APP`, `CT-ST` tables).
- **Failure scenarios `[INF]`:** (1) Stalemate: the side to move has no legal move; on resume its clock restarts (`SuspendResume`), then the flag fires and the stalemated side **loses on time** instead of drawing. Wrong result, wrong Elo. (2) Checkmate: same clock path, or an engine asked to move in a mated position fails and loops through `EngineError` / retry (T4, T5). (3) Threefold, fifty-move, dead position: the draw is missed for good, because termination is evaluated only after the next move, on a different position.
- **Impact:** wrong game result and Elo, or a stuck game, after a crash in a window that is a few milliseconds wide per terminal move but is exactly where the failpoint `finalize.after_journal_end` style testing would not look (that failpoint names `GameEnd`, not the preceding `Move`). NFR-2 is met for the move, not for the game.
- **Options:** (a) on resume, after replay and before `SuspendResume`, run the termination check on the replayed position and journal the resulting `Ended`; deterministic because position and repetition keys are known. (b) Write all facts of one step in one write with one `F_FULLFSYNC` and a batch commit marker, so the pair is atomic (new framing, a format decision). I recommend (a) because it needs no format change and also repairs every other cause of a missing `Ended`; (b) can be added later only with a version bump.
- **Verify the fix by:** a `FaultPlan` kill between the `Move` and `Ended` appends for each terminal reason (CT-ST-003 extension), asserting that resume ends the game with the correct reason and Elo; a domain unit test that replay of `[..., Move(mate)]` reports a terminal condition.
- **Proposed tier:** G2 (touches `chess-domain` and `chess-app`). **Status:** OPEN.

### BP-003 [Major][LIKELY] Multi-record steps are not atomic on a write error: the first record is durable, the in-memory state reverts
- **Location:** SDD-chess-app.md:841-843 (ALG-APP-01 steps 3 and 4); SDD-chess-store.md:498-501 (`append` takes one record).
- **Observation `[OBS]`:** step 3 builds "one journal record per `Fact`"; step 4 appends "the record" and says "On error: drop the clone, return `JournalWriteFailed`; the clocks of the old state keep running because nothing changed". Steps that return several facts: `Move` + `Ended`; `DrawOffer` + `Clock`; `DrawDropped` + `Suspend` + `Clock` (domain T8, T14).
- **Failure scenario `[INF]`:** the `Move` record is fsynced, the `Ended` append fails (disk full, read-only volume). The session discards the clone, tells the caller nothing changed, and keeps the old state. The journal now holds a move the session denies. The user retries the move (or the engine picks again); a second `Move` for the same ply is appended. At the next replay the second fact does not fit (`ReplayError`), `JournalUnreadable{recovered_moves}` is raised, and everything after the duplicate is recoverable only via the store's rows.
- **Impact:** self-inflicted journal corruption under an I/O error, which is exactly the condition NFR-2 fault injection is meant to survive. Moves after the divergence can be lost.
- **Options:** (a) rule: once any record of a step is durable, the session must adopt the durable prefix (apply the facts that succeeded) or stop accepting commands and re-enter through the resume path; never revert to the old state. (b) Add a batch append (one write, one fsync) to the store API plus a rule for a torn batch (needs a commit marker). I recommend (a); (b) is the G2 interface change and also helps BP-002.
- **Verify the fix by:** `FaultPlan` error injection on the second append of each multi-fact step; assert that journal and session agree afterwards and that a retried command is rejected as stale.
- **Proposed tier:** G1 for (a), inside `chess-app`; G2 for (b). **Status:** OPEN.

### BP-004 [Major][LIKELY] Reconcile deletes stored `MOVE` rows when the journal tail is dropped as torn, even when the store provably holds later acknowledged records
- **Location:** SDD-chess-store.md:762-768 (scan: "none found, torn tail: drop it silently"), 776 (reconcile row 1: "Journal intact (no damage) ... rows beyond J's end are replaced or deleted. Journal wins"), 781 (clock rule that does use `journal_seq > last_intact_seq`); architecture.md:1567.
- **Observation `[OBS]`:** the scan classifies any bad record with no valid record after it as a torn tail, which is "intact" for reconcile. Reconcile then deletes `MOVE` rows beyond the journal's end. The justification is that a torn record "was never acknowledged" (SDD-store:768). That holds only if the record was torn by the crash that is being recovered. If an acknowledged final record is later damaged or lost (an `F_FULLFSYNC` that a drive did not honour, a journal file restored from an older copy, bit rot on the last sector), the same code path runs, and `GAME.journal_seq` is greater than `last_intact_seq` (the store applied a record the journal no longer proves). For clocks the SDD already trusts the store in exactly that case (line 781); for moves it deletes them.
- **Impact:** a completed move that exists in the database is deleted by recovery, silently. This contradicts NFR-2's "no completed move lost" in the one case where the database still holds it.
- **Fix direction:** treat `db_seq > last_intact_seq` as damage, not as a torn tail: apply the longer-run rule (row 2 of the reconcile table) and report `JournalUnreadable`-style recovery to the user.
- **Verify the fix by:** extend UT-ST-012 with a case "journal truncated by one acknowledged record, store holds it" and assert the store row survives and the user is told; failure injection that truncates the journal after an apply.
- **Proposed tier:** G1 (store-internal algorithm; the architecture sentence at arch:1567 needs one clarifying clause from the SA). **Status:** OPEN.

### BP-005 [Major][CONFIRMED] "Custom start" is defined three ways, contradicts RULE-8, and has no column
- **Location:** architecture.md:1546 (flag 4: "a custom FEN equal to the standard start still makes an unrated game"); requirements.md:502-507 (RULE-8: "a game started from a **non-standard** position shall always be unrated") and 623 (CR-9.11: a FEN equal to the standard start "counts as a standard start, not as a custom start, for ECO"); SDD-chess-app.md:1066 (same CR-9.11 reading); SDD-chess-domain.md:217 (`StartPosition` appears in the class diagram and is never defined), 370 (`rating_track(&GameFacts)` takes "custom start"), 753 (`StatsRow.custom_start`); SDD-chess-store.md:287-314 (no `custom_start` column; `start_fen TEXT` "null = standard", arch:1302).
- **Observation `[OBS]`:** the architecture flag treats a FEN that equals the standard start as custom and unrated. RULE-8 as written, and CR-9.11 for ECO, treat it as standard. The preamble of the flags admits that "no requirement or earlier text justifies" these choices (arch:1542). Statistics and history must filter and mark "custom start" (requirements.md:758, domain:753), so the store must return it. The `GAME` table has `start_fen` only. If an implementer derives custom as `start_fen IS NOT NULL` and the `Start` record writes the standard FEN for a standard game, every game becomes custom; if standard games store NULL, the "FEN equal to standard" case needs a normalisation rule that nobody wrote.
- **Impact:** a stored, one-way-at-release meaning with three readings: Elo eligibility, the statistics predicate and the "custom start" mark in history can disagree. A game could be rated at finalize (using `rated`) and excluded from statistics (using a derived custom flag), or the reverse.
- **Options:** (1) Standard means "position equals the standard start" regardless of how it was entered: `StartPosition::Standard`, `custom_start = false`, `start_fen = NULL`; matches RULE-8 and CR-9.11; define the comparison (full FEN fields, or position and side only). (2) Keep flag 4 as written; this needs a requirements change by the Systems Engineer and Brian. I recommend (1). In both cases add a `custom_start INTEGER NOT NULL CHECK (custom_start IN (0,1))` column, or state the derivation in the DDL comment and test it. Define `StartPosition` in the domain SDD.
- **Verify the fix by:** table tests of `custom_start`, `rated` and the statistics predicate for: standard start, FEN equal to standard, FEN with a different move counter only, genuinely custom FEN.
- **Proposed tier:** G3 (schema and payload semantics), and a requirement-interpretation ruling by Brian; SE to confirm RULE-8. **Status:** OPEN.

### BP-006 [Minor][LIKELY] `content_hash` is an as-of-game-end value; a rename, or an unspecified `Date`, breaks the re-import match that FF-4 promises
- **Location:** SDD-chess-rules.md:598-606 (canonical form includes `White`, `Black`, `Date`, `Event`, `Site`); requirements.md:256 (a human Player can be renamed, "history follows"); architecture.md:1414 (history shows the current name); SDD-chess-app.md:881 (hash computed once at finalize, "a live game and its re-imported export hash alike (FF-4)"); SDD-chess-rules.md:47 (Event, Site, Date content is owned elsewhere).
- **Observation `[OBS]`:** the stored hash is computed when the game ends from the names then. Export writes the current name. After a rename, `hash(export)` differs from the stored hash.
- **Impact `[INF]`:** re-importing one's own export after a rename is not recognised as a duplicate; the user gets a second review-only copy. Harmless in effect, but FF-4 as worded fails, and a test that renames between export and import will fail. Also the source and time zone of `Date` (local or UTC, from `started_at` or `ended_at`) is stated nowhere I read, so the same issue arises if `Date` is derived at export time with a different rule or after a time-zone change.
- **Options:** (a) accept and write it down: the hash is as of game end, FF-4 holds only without a rename, and the `Date` rule is fixed (e.g. UTC date of `started_at`) in the app SDD. (b) Exclude `White`/`Black` from the canonical form (weaker identity, tag set changes, Q-SE-1 would need re-confirmation). (c) Recompute stored hashes on rename (needs a new store write and the canonical form in the app; journals keep the old value). I recommend (a); it costs only text and a test.
- **Proposed tier:** G0/G1 for (a); G3 for (b) or (c). **Status:** OPEN.

### BP-007 [Minor][LIKELY] The `sha256:` prefix versions the algorithm but not the canonical form
- **Location:** architecture.md:1994, 2149 ("softened by the algorithm prefix"); SDD-chess-rules.md:598.
- **Observation `[OBS]`:** a bug fix or change to `canonical_form` (escaping, move-number rule, tag set) produces different bytes under the same `sha256:` prefix. The mitigation in the door table ("a new prefix and a recompute on import") does not work for old stored hashes, because a stored hash cannot be told apart from a new one.
- **Fix direction:** put a canonical-form version in the value now (for example `sha256:v1:<hex>`, exact spelling Brian's), and have the importer compare only values with the same version. Cheap before release, impossible to retrofit cleanly.
- **Proposed tier:** G3 (stored value format). **Status:** OPEN.

### BP-008 [Minor][LIKELY] OQ-20 and its siblings: where the hash of a voided game comes from is unspecified in four cases
- **Location:** architecture.md:1419, 1549, 2237; SDD-chess-store.md:466, 475 (`lease_beat` returns `RowGone{status}` only); SDD-chess-app.md:1014 and the `prepare_void` text in the `SessionRegistry` row.
- **Observation `[OBS]`:** besides OQ-20 (live elsewhere, late moves), the documents leave open: (1) **live in this process:** `prepare_void` only "remembers the handle"; the hash is computed from the store's rows, which exclude any unapplied backlog (SDD-store:857), so the stored hash can belong to a shorter move list in the same way as OQ-20, without any other process involved. (2) **live elsewhere:** the other session must append a `Voided` record with `reset_player` and the committed hash; `RowGone{status}` carries neither. (3) **unreadable game:** `void_unreadable_game` is called with no hash in the app (SDD-app:1014) but the store expects one, and "never null in `Voided`" (arch:1540) leaves no canonical-form input for a game with no readable moves. (4) **`GameEnd` then `Voided`:** a session that ends its game in the window before it sees `RowGone` holds a terminal state; `Voided` is not legal from `Completed` (domain T18), so what it appends is undefined, and a restore of that journal sees `GameEnd` followed by `Voided`.
- **OQ-20 options with recommendation:**

| Option | What it is | For | Against |
|---|---|---|---|
| A | Accept: the voided game's hash is whatever the reset transaction held | No change; a voided game is partial and already outside the dedupe promise for unfinished games (SDD-app ALG-APP-09: unfinished games have no hash, "acceptable") | FF-4 cannot assert equality for voided games; hash and moves can differ |
| B | Recompute when the late moves are applied | Hash always matches moves | Needs a new store write (`set_content_hash`) and a store-to-app callback or a second app write; the journal `Voided` record is append-only and keeps the stale value, so a restore re-introduces it; G2 |
| C | Voided games carry no hash (NULL allowed for `voided`; drop it from `VoidSpec` and `Voided`) | Removes OQ-20, siblings (1) to (3) and the unreadable-game input; smaller payload | Reverses part of ruling 1; a partly played voided game is never a duplicate target (same argument as unfinished games); G3 |
| D | Make the live-elsewhere session recompute and append a corrected hash | | Journal and DB disagree; same flaw as B |

- **Recommendation: option A, with these conditions:** (i) write that a voided game's `content_hash` is advisory and FF-4 and the "second property" in DD-APP-18 exclude `Voided`; (ii) for live-here games, `prepare_void` must `sync_backlog` or take the move list from the session's in-memory `Game`, so (1) closes; (iii) the live-elsewhere session reads `content_hash` and `voided_by` from the `GAME` row (add them to `get_game` or `LeaseBeat`) to build its `Voided` record; (iv) `void_unreadable_game` takes the hash of whatever moves are readable (zero allowed) and the app SDD call is corrected; (v) define `GameEnd` before `Voided`: a `GameEnd` appended in the window is superseded by the already-committed void, the session appends nothing further and the restore skips the pair. If Brian prefers fewer invariants, option C is the clean alternative; B and D are inferior.
- **Proposed tier:** G1 for the conditions; G3 if C is chosen. **Status:** OPEN (OQ-20 stays open until Brian picks).

### BP-009 [Minor][CONFIRMED] `Voided` asymmetries (flag F6) and a done-journal rule that reset-voided games can never satisfy
- **Location:** architecture.md:1548; SDD-chess-store.md:745, 805, 833, 845.
- **Observation `[OBS]`:** `Voided` has no `ended_at`; `GameEnd` and `Abort` do. A `Voided` appended by crash recovery or by the other process carries a later `wall_ms` than the original reset commit time stored in `GAME.ended_at`, so a restore re-running the void from the record writes a different `ended_at`. Second, the done-journal rule deletes a journal only when a verified backup holds the game with `journal_seq` equal to the journal's last sequence (SDD-store:833). `reset_player` takes `VoidSpec{game, content_hash}` and runs before the `Voided` record exists, so it cannot set `journal_seq` to the `Voided` sequence `[INF]`. Journals of reset-voided games would stay in `journal/done/` forever.
- **Recommendation:** add `ended_at` to `Voided` (assembler-added, like the others); define how `journal_seq` of a voided game is advanced (for example the store applies the `Voided` record's sequence in a follow-up `apply_records`, or the deletion rule treats `voided` as final without the sequence test). The word "none" in the store's DB-effect column for `Voided` (SDD-store:745) should say "restore re-runs the void".
- **Proposed tier:** G3 (payload field), G1 (the rest). **Status:** OPEN.

### BP-010 [Minor][LIKELY] Format-defining details are not fixed and must be before release
- **Location:** SDD-chess-store.md:726-745; SDD-chess-domain.md:217, 293-312.
- **Observation `[OBS]`:** not specified in any document read: (1) the numeric `kind: u8` codes for the 14 kinds; (2) the CRC-32 polynomial (IEEE or Castagnoli) and the exact bytes covered; (3) maximum `len`; (4) `VoidReason` variants and their journal strings; (5) canonical `ClockSpec` text (`300+0` or `300`); (6) canonical UUID text (lower-case hyphenated) for payloads, file names and `TEXT UNIQUE` columns, which compare with `BINARY` collation; (7) how a standard start is written in `Start.start_fen` (BP-005); (8) clocks in `Move` and `Clock` are two nullable fields but the `Fact` has one `Option<ClockValues>`; a half-null pair is representable and undefined; (9) `GameEnd.reason` reuses display-style strings with spaces and commas ("time forfeit, opponent cannot mate", domain:293-312) as a stored machine value. (10) `Start` carries `started_at` in the payload and `wall_ms` in the body.
- **Impact:** each is a decision that becomes frozen at release (D-41 is one-way at release). Left to the implementer they become de facto format without review.
- **Proposed tier:** G3 as a batch with BP-001 and BP-005; most are one line each. **Status:** OPEN.

### BP-011 [Minor][CONFIRMED] D-58 text is internally inconsistent and its door class is understated
- **Location:** architecture.md:1570-1571, 2010, 2165; SDD-chess-store.md:905 (`StartupRecovery` "takes each flock non-blocking"); SDD-chess-app.md:1042 (`open_held`).
- **Observation `[OBS]`:** (1) arch:1570 says the startup scan "does not append, because only the holder of a journal's lock appends"; but the scan does take each journal's `flock` (SDD-store:905), so the stated reason does not distinguish scan from session. The real reason is that no `Game` exists to call `decide`. (2) arch:1571 has a recovery bullet that **appends** `Voided` when the status is already voided and the journal lacks it; it does not say who performs it, and it contradicts "the scan never appends". (3) D-58 is classed Two-way (arch:2165), but its records (`Suspend`, `SuspendResume`, an optional `DrawDropped`) become permanent journal history and the alternative it names, an `Interrupted` kind, is "one-way after release" by its own words; the same format-freeze logic as D-41 applies. (4) `open_held` runs the crash-recovery step, which ends with `SuspendResume` and a restarted clock, for a session that has no clock; what the clock fact means for a held game is not stated.
- **Proposed tier:** G1 (text, SA desk review). **Status:** OPEN.

### BP-012 [Minor][LIKELY] Startup journal replay finalizes games in scan order; restore finalizes in end-time order
- **Location:** architecture.md:1579 (restore sorts by `ended_at`), 1583 (startup scan, no order); SDD-chess-store.md:621-622, 845.
- **Observation `[OBS]`:** only restore specifies the order. If two games ended before their finalize committed (store busy at a crash), both are finalized by the scan. When they involve the same computer Player, the second delta depends on the first, so the order changes ratings.
- **Fix direction:** same rule as restore: finalize pending `GameEnd` journals ascending by `ended_at` (ties by `game_uuid`).
- **Proposed tier:** G1. **Status:** OPEN.

### BP-013 [Minor][SUSPECTED] A SAN mismatch during replay is fatal although UCI is authoritative
- **Location:** SDD-chess-domain.md:604, 624; architecture.md:1530.
- **Observation `[OBS]`:** each `Move` record stores `san` and `fen_after`; replay "recomputes the SAN and compares" and stops at a mismatch with `ReplayError`, which becomes `JournalUnreadable{recovered_moves}`.
- **Failure scenario `[INF]`:** a suspended game outlives a dependency bump of the rules library; if the new version formats one SAN differently (suffix, disambiguation) the old journal is "unreadable" from that ply, although the `uci` is intact and the move legal. Whether a given library upgrade changes SAN output is `[UNV]`.
- **Fix direction:** on a SAN mismatch with a legal `uci`, trust `uci`, recompute `san` and `fen_after`, and log; error only on an illegal move or a `fen_after` mismatch that is not explained by SAN.
- **Proposed tier:** G1. **Status:** OPEN.

### BP-014 [Minor][CONFIRMED] A failed `F_FULLFSYNC` silently downgrades the durability promise
- **Location:** SDD-chess-store.md:1028 (ASM-ST-4); architecture.md:1235-1236 (NFR-2 and TBD-23 = 0).
- **Observation `[OBS]`:** on `ENOTSUP` the writer falls back to `sync_all` and logs once. The assumption itself says this "weakens NFR-2 and should be an owner decision". The data directory is user-relocatable (`--data-dir`, `CHESS_DATA_DIR`, D-36), so an external or network volume is a realistic way to hit it.
- **Fix direction:** Brian decides: refuse to open such a directory, or warn visibly in the front-end and record the weakened mode in `OpenReport`.
- **Proposed tier:** G1 for the code; Brian's decision is needed because it trades a Must requirement. **Status:** OPEN.

### BP-015 [Minor][LIKELY] D-57: "refuses the default data directory" is not enough protection for the user's store
- **Location:** architecture.md:2009; SDD-chess-app.md (IFP-APP-04 and DD-APP-20: "refuses the default data directory (R-17)"); architecture.md:1237 (precedence `--data-dir`, `CHESS_DATA_DIR`, default).
- **Observation `[OBS]`:** calibration runs on a full `Core` with a store, a roster and games. A user whose real store lives elsewhere (set by `CHESS_DATA_DIR`, or passed with `--data-dir`) is not protected by a rule that names only the default path, and the first-launch path creates 25 computer Players in whatever store it opens.
- **Recommendation:** accept D-57 on ownership; condition: `open_for_calibration` requires an empty or absent directory, or one carrying a calibration marker it wrote itself, and refuses any directory that already holds `chess.db`, `store.id` or `journal/`. Note (not a finding): consider a `calibration` Cargo feature so the SPRT code is not shipped in the server and desktop binaries; the SDD only guards it from the wire (UT-APP-05).
- **Proposed tier:** G1 for the guard; the new public items are already recorded G2. **Status:** OPEN.

### BP-016 [Minor][LIKELY] The DDL does not enforce several invariants the documents state
- **Location:** SDD-chess-store.md:287-314, 317-322, 270-278; architecture.md:1540, 1322; SDD-chess-domain.md:920.
- **Observation `[OBS]`:** (1) "never null in `GameEnd`, `Abort`, `Voided`" (arch:1540) but the only hash CHECK is `status <> 'in_progress' OR content_hash IS NULL` (line 307); a terminal `completed`, `aborted` or `voided` row with a NULL hash is accepted. (2) `MOVE.white_clock_ms` and `black_clock_ms` may be null independently. (3) `error_multipv >= 2` when `error_permille > 0` is a domain constructor rule (domain:920) with no DB backstop. (4) `white_id` and `black_id` may both be NULL for a non-imported game.
- **Impact:** low, since the single writer is the store thread; the value is that a defect in the app is caught by the DB. Cheap before release.
- **Proposed tier:** G3 (schema, trivial). **Status:** OPEN.

### BP-017 [Note] Two small decisions to take while the format is still open
- **Suspend cause (F2):** D-58 makes `Suspend` mean both a clean detach and "recovered after a crash". With an empty payload and a `deny_unknown_fields` decoder (SDD-store:749) a later `cause` field needs a version bump. Decide now: add an optional `cause` (detach, shutdown, recovered) or record that the loss of diagnosability is accepted.
- **`DrawDropped.by` mismatch (F3):** "replay checks `by` against the pending offer" (arch:1545) does not say what happens on a mismatch. An error would turn an audit field into an unresumable game. Say: log and continue.
- **Proposed tier:** G3 if a field is added; otherwise G0. **Status:** OPEN.

## 7. Items checked and found sound

Listed so the reader knows what was examined, not as praise.

- Journal-first ordering, ack after fsync, single writer per journal, `flock` as the lease (arch:1557-1561; SDD-store:627-654).
- The damaged-middle rule (longest intact run, journal wins on disagreement, damaged file kept) and the pure, idempotent reconcile function (SDD-store:772-783), apart from BP-004.
- Restore on a copy, finalize in end-time order, `done/` retention until a verified backup holds the game, apart from the voided case in BP-009 (arch:1575-1583; SDD-store:833).
- Start order: store transaction, then journal, then acknowledge; a row with no journal is a zero-move game (SDD-store:656).
- D-43 (section 4.3), D-40 UUID identity (journal payloads carry UUIDs only).
- D-58 idempotence across a second crash during recovery (4.2 row 10).
- The four-document payload equality (4.1).

## 8. Proposed dispositions for Brian

These are proposals. Brian decides; nothing here is approved.

| Decision needed | Findings | Recommended |
|---|---|---|
| Add the three finalize time fields to `GameEnd` | BP-001 | Yes, option (a) |
| Rule on custom start and add `custom_start` | BP-005 | Option (1): equal-to-standard is standard; add the column |
| Re-check termination on resume, and fix partial-step failure policy | BP-002, BP-003 | (a) re-check termination; session adopts the durable prefix |
| Treat `db_seq > last_intact_seq` as damage | BP-004 | Yes |
| OQ-20 | BP-008 | Option A with the five conditions; C as the alternative |
| Hash scope: as-of-game-end, `Date` rule, canonical-form version in the value | BP-006, BP-007 | Accept (a); add version marker |
| Add `ended_at` to `Voided`; define `journal_seq` for voided games | BP-009 | Yes |
| Freeze the format details as one batch | BP-010, BP-016, BP-017 | Yes, one G3 batch with BP-001 and BP-005 |
| `F_FULLFSYNC` downgrade policy | BP-014 | Brian chooses refuse or warn |
| Closing OQ-16 | all | Close only after the Major findings are dispositioned and the format batch is applied by the architect/SDD owners |

## 9. Gate record

`GATE-nnn` for this document: G0, no approver. This review does not satisfy GATE-006 or GATE-007 by itself; those say the user's approval does not replace the Board pass and that human sign-off is not claimed (architecture.md:2102, 2104). Whether this single-reviewer desk pass closes OQ-16 is for Brian to decide and record. No approval, acceptance or deviation is claimed here.

## 10. Assumptions and not verified

| ID | Assumption or gap |
|---|---|
| ASM-BP-1 | The SDDs and architecture as of commit 709f0ba and the working tree are the baseline B1 text; I did not diff against a B1 snapshot export |
| ASM-BP-2 | `evolve(Move)` does not itself set a terminal state (domain:622 says only `Ended` and the other terminal facts change state). If the domain implementation ends the game inside `evolve(Move)`, BP-002 narrows to the `Ended` record only; the stale-clock and duplicate-ply parts of BP-003 are unaffected |
| ASM-BP-3 | Every statement about runtime behaviour (replay, reconcile, finalize order) is read from design text; no code exists, so none is `[RAN]` except section 4.3 |
| ASM-BP-4 | SQLite, `rusqlite`, `fcntl(F_FULLFSYNC)` and `flock` behaviour are not verified here. SDD-chess-store.md itself marks several of them `[UNV]` (lines 243, 811, 823, ASM-ST-5, ASM-ST-7) |
| ASM-BP-5 | Not read end to end: SDD-chess-engine, SDD-chess-cli, SDD-chess-server, SDD-ui, SDD-desktop-tauri, test-strategy.md, use cases, the full requirements file, architecture sections outside those listed in section 2 |
| ASM-BP-6 | Not compared: domain and app tables against the architecture for `MOVE_EVAL` and `VARIATION`; engine SDD against D-43 beyond its type definition; FF-3 and FF-4 test definitions in test-strategy.md |
| ASM-BP-7 | The ECO import work (D-59, D-60, D-62) and the roster decisions (D-46) are outside the OQ-16 scope and were not reviewed |
| ASM-BP-8 | No Mermaid diagram is in this document, so the validator was not needed |
