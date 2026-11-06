# SDD G1 desk review, 2026-11-05 (chess-a0r.1.44, chess-a0r.1.38; baseline B3)

| Field | Value |
|---|---|
| Reviewer | system-architect agent, **Review Board stance**, fresh invocation. I did not write the reviewed edits |
| Reviewed | Working-tree diff of 2026-11-05 against HEAD 9b2a43a: `SDD-chess-rules` 0.3.4, `SDD-chess-domain` 0.4.4, `SDD-chess-app` 0.4.5, `SDD-chess-cli` 0.4.2, `SDD-ui` 0.4.2, `SDD-desktop-tauri` 0.4.2, and the version/status-only edits of engine 0.3.4, server 0.4.2, store 0.4.4 |
| Compared with | architecture D-65, D-68, D-79, D-80, D-82, SF-02, SF-03, SF-04, section 4.8 (backpressure), section 5.1 (event list), section 5.5; OQ-16 desk review F-07; requirements NFR-1, NFR-4, NFR-8, NFR-9 |
| Not done | No code exists, nothing was run. No Mermaid block was edited by the SwE, so none was validated (`Mermaid not machine-validated`; not needed). I did not review the architecture, requirements, SWE-001 or CLAUDE.md edits beyond the collateral check in item 5 (they belong to other parts of chess-a0r.1.44) |
| Status | Desk-review recommendation only. **No approval is claimed.** G2 items go to Brian Prascher |

Severities follow architecture-governance: Blocker, Major, Minor, Note. Line numbers are the working-tree lines at review time.

## 1. Verdicts

| # | Item | Verdict |
|---|---|---|
| 1 | rules 0.3.4: Version rule paragraph, `Date` rule, UT-RUL-031 row | **Accept with fixes** (FND-G1-002, FND-G1-003, FND-G1-011) |
| 2 | domain 0.4.4: UT-DOM-118 fixture, UT-DOM-115 table, XC-DOM-A8-5 left open | **Accept with fixes** (FND-G1-001, FND-G1-003, FND-G1-004) |
| 3 | app 0.4.5 OQ-APP-015 and step 5a marker; cli 0.4.2 and ui 0.4.2 pointers | **Accept with fixes** (FND-G1-005, FND-G1-006, FND-G1-007). Ruling in section 4 |
| 4 | New case IDs and `[INF]` mappings | **Accept with fixes** (FND-G1-008, FND-G1-009, FND-G1-010). IDs are sound and not duplicated |
| 5 | No collateral change to a decision, edge or 5.7 contract | **Accept** |

**G1 desk-review verdict: APPROVE WITH CONDITIONS.** No Blocker. Three Major findings (FND-G1-001, 002, 005) must be fixed before these texts are relied on for step 1 test design. The SwE did not invent a rule. The tiering the SwE chose (G0 for wording, G1 for the UT-DOM-118 fixture and the three OQ pointers) is right.

## 2. Findings

### FND-G1-001 | Major | Lens: Quality attributes | domain:733, domain:1110 (UT-DOM-115), rules:830 (UT-RUL-031 row 5)
**Claim.** The new drift-guard table expects the "en passant square" variant to give `Custom` (domain) and to keep both tags (rules). The sentence the SwE left unchanged at domain:733 says the opposite intent: compare "as parsed values (so `KQkq` order or an unreachable en passant square cannot make a standard start look custom)".
**Why it matters.** A variant that differs from the standard FEN "in exactly that field" (standard placement, white to move, ep square `e6`) is an unreachable en passant square. If the `chess-rules` FEN type drops it, the variant becomes `Standard`, the table row is wrong, and the drift guard fails on a correct implementation. If it keeps it, the row is right and domain:733 is wrong. The rows state a definite expected result while the same rows say the normalisation is `[UNV]` (XC-DOM-A8-5). SF-02 itself says "all six fields must equal", with no normalisation wording.
**Required action.** Either (a) mark the en passant and castling-spelling rows of both tables as `expected result pending XC-DOM-A8-5` and state the expected outcome for the other four fields only, or (b) decide the comparison (raw token or parsed value) in step 1 and write it into both rows. Do not leave a definite expectation beside an `[UNV]` that can flip it. Fixing domain:733 vs the table is a G1 text change.

### FND-G1-002 | Major | Lens: Evolution | rules:613 (Version rule), rules:825 (UT-RUL-026), rules:830 (UT-RUL-031)
**Claim.** The Version rule says "UT-RUL-026 and UT-RUL-031 pin the bytes of `v1`: a byte change that leaves them green is a defect of the test." Neither row pins bytes. UT-RUL-026 checks inclusion and exclusion of tags and equality of two outputs; UT-RUL-031 row (1) compares two outputs with each other. A change to the escaping, the movetext joiner or the tag order can keep both green.
**Why it matters.** D-68 is one-way at release (score 6). The only guard against changing `canonical_form` without bumping `v1` is a byte-exact test, and the crate exposes no version constant (rules:613 says so). The SDD claims a guard that the rows do not provide.
**Required action.** Add a golden-bytes case (snapshot level is already in use, section 8 levels line) for two or three fixed games, and say in the row that changing the golden requires the `v2` bump in ALG-APP-09 and a decision-log line. Or reword the sentence so it does not claim a guard that is not there.

### FND-G1-003 | Minor | Lens: Simplicity | rules:830, domain:733, domain:1110, app:1320
**Claim.** The drift-guard table is described three ways: "one four-start table" (UT-RUL-031 lead-in), "the four BP-005 starts plus one variant per field" (domain:733), "the same seven-row table" (UT-DOM-115, UT-RUL-031 row 5). Four plus six is ten, not seven. UT-RUL-031 rows (1) to (4) are not the BP-005 starts. Also, "shared" has no named single source: two crates, two test files, no fixture file or test-support item is named, so the "shared" table can itself drift.
**Required action.** Define the table once (seven rows: the standard FEN and six one-field variants), say that UT-APP-29 uses a subset, and name where the data lives (for example one data file included by both crates' tests, or an item of `chess-rules` behind its existing test-support; a dev-dependency adds no §3.2 edge `[INF]`, `chess-domain` already depends on `chess-rules`). Row (5) keeps different expected results per crate (`Custom` vs tags kept); say that only the inputs are shared.

### FND-G1-004 | Minor | Lens: Data | domain:1113 (UT-DOM-118)
**Claim.** The F-07 fixture feeds a double pawn push with `fen_after` once with the ep square and once with `-`, and expects both equal to the computed position. That holds only when no enemy pawn can capture en passant. If one can, the two spellings are different positions (repetition, `PositionKey`) and `-` is wrong.
**Required action.** State the position: for example 1.e4 from the standard start (no black pawn on d4 or f4, so `e3` and `-` are the same position), plus one case where a capture is possible and `-` must give `FenMismatch` or the SwE records the chosen behaviour. The row correctly keeps XC-DOM-A8-5 open and does not say which spelling the writer emits. F-07 stays open until the fixture exists as code; the 0.4.4 change-log row says so.

### FND-G1-005 | Major | Lens: Quality attributes | app:856, app:1507 (OQ-APP-015), architecture 4.8 and 5.5
**Claim.** Whichever option is chosen, the snapshot or events a front-end receives in `JournalDegraded` do not say that moves are refused and the clock is held. The architecture says front-ends "render controls from `allowed_actions`" (architecture section 4.x, about line 720) and D-80 says every other command is answered `JournalWriteFailed`. Nothing says `allowed_actions` is reduced in `JournalDegraded`, or what `running_side` the snapshot carries. DD-CLI-014 says the local clock display stops and "a later snapshot replaces it", which would restart the countdown if the snapshot still names a running side.
**Required action.** In the ruling text (section 4) the session publishes `allowed_actions` limited to the D-80 set (Detach) and a clock marked not running. The SwE must check how `GameSnapshot.clocks` carries the running side `[UNV]`. If either cannot be expressed without a new field, stop: that is a protocol change and G2 (Brian).

### FND-G1-006 | Minor | Lens: Operability | cli:558 (DD-CLI-014 state rule), ui:540 (DD-UI-015)
**Claim.** The CLI and UI infer `Degraded` when "the game snapshot changed although the command failed". The reply to the command and the broadcast event travel on different paths (reply channel vs `broadcast` stream and forwarder task). Their arrival order is not specified, so "changed although failed" can be evaluated before the change arrives.
**Required action.** When the SwE applies the OQ-APP-015 ruling, make the rule order-independent: record the failure, then evaluate when either the reply or the snapshot arrives, comparing against the board as it was at the command. Text only, G1.

### FND-G1-007 | Minor | Lens: Quality attributes | app:1507 (OQ-APP-015 text)
**Claim.** (a) The question is framed as "adopting a durable prefix". `JournalDegraded` is also entered with no adopted fact (a poisoned writer after a single-record step, step 4, D-82 (iv)); front-ends need the same news then. (b) Option C is described as leaving "a second subscriber ... stale". It is worse: an engine move or a flag fall has no replying front-end, so nobody receives `JournalWriteFailed` and nobody would know to call `GetGame`.
**Required action.** Reword the question to "on entering `JournalDegraded`" and add the internal-origin case to option C when the question is closed.

### FND-G1-008 | Minor | Lens: Quality attributes | cli:748 (NFR-9 trace row)
**Claim.** The row says UT-CLI-08 "every catalogue code has a message and exit row". UT-CLI-08 (cli:689) asserts `exit_code` rows only; no message check is stated there. The NFR-9 requirement is "every protocol error code has a message". The row also omits the degraded-notice cases (UT-CLI-017, 018, E2E-CLI-23) that carry the newest NFR-9 text.
**Required action.** Reword to "exit row"; name the case that checks the message per code (or allocate one), and add the degraded-notice cases.

### FND-G1-009 | Minor | Lens: Quality attributes | tauri:481, tauri:482 (CT-TAU-023, CT-TAU-024), ui:710 (CT-UI-020)
**Claim.** The three new cases omit the pass criterion of the requirement they trace to. NFR-1: median of 10 warm starts from launch to first screen accepting input, 3 s desktop. NFR-8: inject inputs during a maximum-length search, 95th-percentile input-to-render at most 100 ms (ESTIMATE), UI stays interactive. CT-TAU-023 cites "the 3 s budget of section 3.7", but 3.7 only mentions it; the budget is stated in the time-limits line (tauri:447). CT-UI-020 delays every reply but does not run a search; CT-TAU-024 measures "not held up" with no threshold.
**Required action.** Add the criterion and the correct anchor to each row, or mark each `criterion = requirements NFR-n verification column` explicitly. The rows say "no further procedure defined here", which is honest, so this is Minor.

### FND-G1-010 | Minor | Lens: Quality attributes | ui:741, ui:744, ui:695-702
**Claim.** (a) The task brief and the SwE's reasoning mention GAME-3; the change is to GUI-3 (coordinates). The **GAME-3 (rewind) row still reads "UT-UI-09, E2E"** with a bare "E2E" that names no case, the same defect FIND-R23-013 fixed elsewhere. (b) The `[INF]` mappings are one-way: UT-UI-05, E2E-UI-09 and E2E-UI-01 do not list GUI-3 or PLAT-2 in their own trace column, so the reverse trace is missing. (c) E2E-UI-01 runs in all four browsers, which supports PLAT-2: sound.
**Required action.** Name a case for GAME-3 or register the gap; add the requirement IDs to the case rows. The CLI `[INF]` mapping for NFR-9 is sound apart from FND-G1-008.

### FND-G1-011 | Note | Lens: Simplicity | rules:613, rules:10 and domain:10 vs app:14
**Claim.** (a) The Version rule adds an open question ("whether a version constant is added is a G1 question for the code step") with no OQ ID; next free is OQ-RUL-005 (OQ-RUL-001 to 004 exist). (b) rules and domain cite "SWE-001 v1.1" while app 0.4.5 cites v1.1.1. (c) The Version rule's sentence "the editor ... must bump the version" is a process rule derived from D-68 (c); it is fair, but it is the SwE's wording, not D-68's. No new rule is invented (item 1 below).

## 3. Item notes

**Item 1 (rules).** Compared line by line with D-68 (b), (c), (d), SF-03 and SF-04.
- `sha256:v1:<64 lowercase hex>`, `v1` versions `canonical_form`, not the algorithm: matches SF-03.
- Importer compares only the same version; another version never matches and is not an error, owned by `chess-app` (ALG-APP-09 states the same, app:895): matches D-68 (c).
- Change after release is a new version; before release stays `v1` (ASM-6): matches D-68 row and the door table.
- `Date` is the UTC date of `started_at`, `YYYY.MM.DD`, the `GAME_TAG` value for imports: matches SF-04. The rules crate has no clock, so `chess-app` fills the field: consistent with ALG-APP-09 `Date` rule.
- Steps 1 to 5 exist and the "steps 1 to 5" scope is right. The first-release bump discipline is the only addition (FND-G1-002 on its enforcement).
- The 0.3.2 history row is left as written and marked superseded: correct (decision-records rule 1, append-only).

**Item 2 (domain).** UT-DOM-118 text adds only the fixture F-07 asked for and does not choose the normalisation: correct and honest. XC-DOM-A8-5 is left open with the owner and F-07 reference: correct. UT-DOM-115 and the 3.11.6 paragraph disagree with each other (FND-G1-001, 003).

**Item 4 (IDs).** CT-TAU-023, CT-TAU-024, CT-UI-019, CT-UI-020, E2E-UI-15 each occur only in their own SDD, only in the case table, the trace row and the change log (3 hits each). They are the next free numbers (CT-TAU-022, CT-UI-018, E2E-UI-14 were the previous highs; no CT-TAU-025, CT-UI-021 or E2E-UI-16 exists). Each traces to the requirement that had no case. No renumbering. E2E-UI-15 matches the DD-UI-8 Watch row (controls, no rewind, scrolling). CT-UI-019 combines three screens in one component-plus-E2E case; acceptable, split later if it grows. CT-UI-020 overlaps CT-UI-016 only in topic (CT-UI-016 is real `ParseMove` latency); not a duplicate. `[INF]` tags are used as the evidence rules require. CLI NFR-9, UI GUI-3 and PLAT-2 mappings: sound with FND-G1-008 and FND-G1-010.

**Item 5 (collateral).** `git diff` of architecture.md contains no changed D-, SF- or FF- table row (checked with a row-prefix filter); changes are status wording and the 2026-11-05 log row (part SA, not reviewed in depth here). The SDD diffs touch version, date, status, upstream-read lines, trace rows, case rows, open-question rows and change-log rows. engine, server and store changes are wording only. No §3.2 edge, no §5.7 contract, no protocol event or code, no stored format changed. App 0.4.5 step 5a gains one "Open" sentence and no behaviour. The "Not approved, not desk-reviewed" labels are kept. Out of this review: requirements CR-13, SWE-001 1.1.1, CLAUDE.md (other parts of the work order).

## 4. [Architect] Ruling recommendation on OQ-APP-015

*This section changes stance. It is a recommendation for the software-engineer to apply. It is the reviewer's recommendation, written by an invocation that also reviewed the text, and it has had no separate review; record it in the architecture decision log as an architect ruling "not reviewed by anyone but its author", as D-82 was.*

**Question.** What does a session publish to subscribers on entering `JournalDegraded` (after adopting a durable prefix, step 5a, or after a poisoned writer, step 4)?

**Recommendation: option B, extended to every entry into `JournalDegraded`.** On entering `JournalDegraded` the session publishes **one `GameSnapshot` to the current subscribers**, once, and publishes nothing for the discarded facts. Confidence: **Medium-High** (reasoning from the documents; no prototype).

Rule text for the SwE to write at step 5a (and referenced from step 4 and 5b):
1. Order: apply the durable prefix, hand its records to the store applier in order (step 6), set `JournalDegraded`, then publish the snapshot, then answer the failed command with `JournalWriteFailed{cause}`. Then run the 5b terminal re-check. If the re-check journals `Ended`, the normal finalize path publishes `GameEnded` after the snapshot.
2. The snapshot carries the adopted state, the next `seq` of the subscription (as the lag snapshot does, IFP-APP-02), `allowed_actions` limited to the D-80 set, and the clocks as held (FND-G1-005).
3. It goes to current subscribers only. A later subscriber gets the usual first-item snapshot. With no subscriber nothing is sent.
4. A single-record step with a poisoned writer (step 4) enters the same state and publishes the same snapshot, so front-ends learn the state at once instead of at the next failed command.

**Options compared.**

| Option | For | Against |
|---|---|---|
| A: events of the adopted facts, as step 5 | Uses events for their stated meaning (zero protocol-text change); per-move announcements (UI-6) work | Needs a fact-to-event map for every possible prefix (`Move`, `DrawOffer`, `DrawDropped`, `Suspend`, `Clock`); `Ended` goes through finalize and carries `elo_changes`, so it is not a plain map; nothing to publish for the poisoned single-record entry; the reducer result after a partial event run is not guaranteed equal to the state |
| B: one `GameSnapshot` | State-based, one message, independent of the prefix; same mechanism as the lag rule (architecture 4.8, IFP-APP-02) and reconnect, which every front-end must already handle as "replace state"; also covers the entry with no adopted fact | A mid-stream snapshot has a third trigger (subscribe, lag, degraded entry); per-move announcement for the adopted move is lost (the degraded notice covers it); depends on FND-G1-005 |
| C: nothing, re-read with `GetGame` | No new behaviour | **Rejected.** Engine results and flag falls have no replying front-end, so nobody learns of the failure; other subscribers stay stale; it breaks the CLI and UI rule that infers `Degraded` from a changed snapshot |

Option B was preferred over A mainly because of the poisoned-writer entry and the partial-prefix mapping; A is the fallback if a front-end SDD turns out to treat a mid-stream snapshot as an error.

**Gate tier: G1** (inside `chess-app`; no event kind, error code, protocol field, edge, §5.7 contract or stored format changes: `GameSnapshot` already exists and is already sent mid-stream by the lag rule `[INF]`). Informal score: blast 1, reversibility 0, novelty 0, regulatory 0, cost 0, data 0, coupling 1 = 2 (R0 by the architecture-governance scale; the project gate G1 applies). Door: two-way. **Escalate to G2 (Brian) if** the snapshot cannot express "moves refused" and "clock held" without a new field, or if any front-end SDD states that a snapshot arrives only on subscribe. Revisit trigger: users meet the degraded state often, or a front-end cannot render it from the snapshot alone; escape route: option A, text and code inside `chess-app`.

**Consequential edits for the SwE (text only, after this ruling is recorded):**
- app: step 5a "Open" sentence becomes the rule above; steps 4 and 5b reference it; OQ-APP-015 becomes RESOLVED; UT-APP-28 and UT-APP-35 gain: one snapshot to current subscribers, none with no subscribers, contiguous `seq`, `allowed_actions` limited, no event for discarded facts, store hand-off before publish, snapshot before the failed reply, poisoned single-record case publishes.
- cli DD-CLI-014 and ui DD-UI-015: OQ-CLI-007 and OQ-UI-006 become RESOLVED; make the `Degraded` rule order-independent (FND-G1-006); the accepted "looks like `Failed` until the next command" note for a poisoned single-record step can be dropped.
- architecture: one D-number entry (next free is D-89 `[INF]`, check) in section 11 and the §11.4 ruling list, by the architect, not the SwE. I did not edit the architecture.

## 5. Dispositions the SwE can take without Brian

All findings above are G0 or G1 text. None needs Brian, except the escalation condition in section 4. No sign-off is claimed here; the G1 desk-review verdict is the reviewer's recommendation and Brian does not need to approve G1.

## 6. Not analyzed

Requirements CR-13, SWE-001 1.1.1, CLAUDE.md and the architecture wording pass (other parts of chess-a0r.1.44); the store, engine, server SDD bodies (diff is wording only); whether `shakmaty` or the future `chess-rules` FEN type normalises ep squares (`[UNV]`, step 1); how `GameSnapshot.clocks` carries the running side (`[UNV]`, FND-G1-005).
