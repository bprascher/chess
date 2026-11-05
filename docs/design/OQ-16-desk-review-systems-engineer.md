# OQ-16 desk review of Amendment 8: systems engineer (requirements and verification side)

| Field | Value |
|---|---|
| ID | OQ16-DR-SE |
| Task | chess-a0r.16.1, part 2 of 2 (baseline B2) |
| Version / date | 1.0 / 2026-11-04 |
| Stance | **Reviewer** (independent invocation; did not author Amendment 8, its addenda, the SDD alignment or the CR-10 edits) |
| Owner | systems-engineer (reviewer pass) |
| Decides | Brian Prascher decides the gate. This review approves nothing and edits nothing except this file. |
| Skills applied | se-requirements, se-verification, se-rvtm-traceability, se-change-config-management, se-technical-planning-reviews |

Evidence tags: `[OBS file:line]` read in this session; `[RAN]` command executed; `[INF]` inferred; `[UNV]` not verified.

## 1. Verdict

**PASS WITH CONDITIONS.**

The stored-format batch SF-01 to SF-22 is carried into the three SDDs with a named, measurable test for every item (section 4). CR-10 is verification wording only (section 3). There is no FAIL-class defect. But four Major findings must be dispositioned before OQ-16 may close, and the first is a real requirement question that the author-side systems engineer settled by reading, not by decision.

Counts: **Major 4, Minor 10, Info 5.** No Critical.

### Major findings (one line each)

- **M-1** D-78 (resumed-game time totals from `Move.think_ms`) under-counts "time spent by each Player" by an unbounded unfinished turn per stop; the DAT-3 reading in CR-10 is the author's own interpretation, the Board said this option "needs a requirement ruling on DAT-3", and D-63 still says nothing is recomputed from `think_ms`. Brian must decide.
- **M-2** D-78 depends on ASM-10 (do untimed games record `think_ms`), which is still open, yet XC-DOM-A8-1 and XC-APP-A8-3 are marked "Closed". Training games are always untimed, so the loss may be the whole pre-stop time, not one turn.
- **M-3** D-74 (`GameContextProvider`, G2, confirmed by Brian) has no named verification case for its own new behaviour (live snapshot, store fallback, fail closed with the engine untouched). It guards Strict-mode integrity (ANA-1).
- **M-4** D-78 and D-79 are G2 decisions applied to the SDDs on the architect's delegated ruling, not Brian's approval. D-79 (3) is also a stricter reading of Brian-approved D-72 wording. Delegation evidence is a relay recorded only in architecture text.

### Conditions that must hold before OQ-16 may close

| # | Condition | Closes | Owner |
|---|---|---|---|
| C-1 | Brian rules on the DAT-3 / D-78 question: (a) accept the loss explicitly, with the unbounded-turn consequence stated to him, or (b) take option B or C of D-78 (G3, journal change, cheap before release). Whichever he picks is entered in the Decision Log as his decision, and D-63 and SF-01 text is reconciled with D-78 (remove "nothing recomputes from `Move.think_ms`" or qualify it). | M-1 | Brian (G3); systems-engineer to log |
| C-2 | ASM-10 is closed (domain SDD owner states that every live game, timed or untimed, records `think_ms`), or D-78's loss statement is rewritten for untimed games and C-1 covers that wording. XC-DOM-A8-1, XC-APP-A8-3 stay "open pending C-1/C-2", not "Closed". | M-2, m-8 | domain SDD author; architect |
| C-3 | A verification case with measurable pass/fail is named for D-74 in the app SDD (DD-APP-11 verification and the test table): no context for an unknown game or a store error gives `Denied` or `Internal` and the engine spy shows zero queries; live snapshot used for a live game; `get_game` fallback for a non-live game. | M-3 | software-engineer (app SDD) |
| C-4 | Brian ratifies D-78 and D-79 (G2) in the same decision as C-1, or the delegation is evidenced by a citable record (beads id or message) and the SDD Approver rows say "applied under delegation, not approved". | M-4 | Brian; program-manager records |

Minor findings (section 2) are recommended before closing but do not block if Brian accepts them as tracked items.

## 2. Findings

Format: `ID | Severity | Rule / characteristic | Evidence | Impact | Recommended action | Owner | Gate tier`.

| ID | Severity | Rule / characteristic | Evidence | Impact | Recommended action | Owner | Tier |
|---|---|---|---|---|---|---|---|
| M-1 | Major | Requirement meaning changed without an explicit user decision (se-requirements: interpretation of an AGREED requirement; se-change-config-management: no silent change). DAT-3 | DAT-3 stores "game time (total elapsed duration, and time spent by each Player; excludes time the app was closed/paused during a saved-and-resumed game)" `[OBS requirements.md:616]`. D-78 sums `Move.think_ms` and states the loss: the unfinished turn of the side to move at each stop and any draw-offer interval `[OBS architecture.md D-78 row; domain SDD 3.11.13]`. The app is open during that turn, so DAT-3's only exclusion (closed or paused) does not cover it. "At most one unfinished turn" bounds the count, not the duration: in an untimed Training game a turn can be hours, and a detach (browser tab closed, D-14) triggers a stop `[INF]`. The Board named this exact option: "(b) ... accept that a final unfinished think is not counted ... this needs a requirement ruling on DAT-3" `[OBS Board-pass-store-journal.md:150]`. D-63 still reads "nothing recomputes them from `Move.think_ms`" and SF-01 says totals "cannot be derived from `Move.think_ms`" `[OBS architecture.md D-63 row; §11.4.2 SF-01]`. CR-10 records the author's reading that DAT-3 "gives no accuracy bound" and does not forbid the loss, and says the total elapsed duration "(start and end time) is not affected" `[OBS requirements.md:1419]`. | Stored per-Player time and `total_active_ms` for any stopped-and-resumed game can be lower than the time actually spent; UI-2 and UI-5 "average game length ... in time" inherit the error. The CR-10 sentence about start and end time does not address `total_active_ms`, which is the DAT-3 "total elapsed duration excluding closed time" and is affected (it is white plus black in the seed). The reading is defensible but it is a deviation from the literal text, chosen by the author-side engineer, so it cannot stand as "no requirement conflict". | Present the unbounded-turn consequence to Brian (not "at most one turn"). Offer: A accept and amend DAT-3 by a CR with the loss written into the requirement; B or C (G3). Reword the CR-10 sentence on elapsed duration. Reconcile D-63 and SF-01 text with D-78. | Brian decides; systems-engineer logs | G3 (B, C); G2 (A, with DAT-3 amendment) |
| M-2 | Major | Open assumption gating a closed item (se-rvtm-traceability: status accounting) | ASM-10 "Open" and "now also gates D-78" `[OBS architecture.md §14 ASM-10; §11.4.12 finding 5]`. "A `Move` without `think_ms` adds 0" `[OBS domain SDD 3.11.13]`. Training is always untimed (RULE-4) `[OBS requirements.md:355]`. XC-DOM-A8-1 "Closed", XC-APP-A8-3 "Closed" `[OBS domain SDD 10; app SDD 11]`. | If untimed games record no `think_ms`, every resumed Training game seeds zero and the real loss is all pre-stop time, far above the stated "one turn" bound. UT-DOM-123 and UT-APP-32 would pass on the rule while the statistic is wrong. | Close ASM-10 by text (domain SDD states `think_ms` is measured for every move from the `TimeSource` regardless of clock) or restate the loss. Mark the two XC rows open until then. | domain SDD author; architect | G1 (text), G2 if the rule changes |
| M-3 | Major | Childless decision: no verification (se-verification: every changed behaviour has a case) | D-74 rows exist `[OBS architecture.md D-74]`; app SDD applies it in 4.1, DD-APP-11 step 1, DD-APP-17, SEQ-APP-04 `[OBS app SDD 0.4.1 change log]`. DD-APP-11 verification names UT-APP-05, UT-APP-06, CT-APP-01, CT-APP-05, CT-APP-13 only `[OBS app SDD:1116]`. A search of the test tables for D-74 or `GameContextProvider` finds no test row `[RAN grep, app SDD lines above 1280: none]`. CT-APP-01 proves the engine spy for an in-progress Strict game with a hand-built analysis command, not the provider's fail-closed branch. | A G2 decision with a safety-relevant property (engine must not be touched when state is unknown) is unverified. `[INF]` UT-APP-05 may cover the Allowed/Denied table but cannot cover `ContextError`. | Add one unit and one core-integration case with numeric pass/fail (zero engine queries on `ContextError`; context equals the last published snapshot for a live game; row-built context for a game live elsewhere; stale-by-one-handler case is safe). | software-engineer (app SDD) | G1 (text) |
| M-4 | Major | Gate tier / approval trace (CLAUDE.md: G2 needs Brian; never self-approve) | D-78 "Tier G2 (two crates, and a reading of DAT-3)", D-79 "Tier G2"; both "ruled by the architect under Brian's delegation ... not Brian's approval of the content" `[OBS architecture.md §11.4.12 Ruling 1, Ruling 2]`. The delegation is a quoted work-order instruction, not a Brian record `[UNV: no beads or message read]`. D-79 (3) tightens D-72's approved wording ("a `fen_after` mismatch that SAN does not explain") and the text itself says "Brian can reverse" `[OBS §11.4.12 Ruling 2, "What is clarified beyond"]`. The three SDDs apply both (domain 0.4.1, app 0.4.1). | The SDDs now carry two G2 rulings as settled. If the delegation does not stand, two crates carry unapproved design. The risk is small (two-way doors) but the SDD Approver text says "Brian may reverse", which is honest; the record of who authorised is thin. | Brian ratifies or reverses D-78 and D-79 together with C-1. Program-manager records the delegation source. | Brian; program-manager | G2 |
| m-1 | Minor | Requirement reading without log entry. NFR-2 | NFR-2: "Where the store disagrees with the moves recorded on the Mac at play time, the moves recorded at play time take precedence" `[OBS requirements.md:1033]`. D-66: a database ahead of a damaged journal tail keeps its moves `[OBS architecture.md D-66; store SDD UT-ST-083]`. | Literal precedence sentence reads against D-66. The primary clause ("no completed move shall be lost") supports D-66, so I read no conflict `[INF]`, but CR-10 does not record the reading. | Add the reading to the Decision Log: store rows are copies of acknowledged records, keeping them honours the first clause. | systems-engineer | G0 |
| m-2 | Minor | Stale cross-reference in an AGREED requirement. DAT-3 (a) | DAT-3 (a): "Architecture D-20 ... puts it in `chess-domain`" `[OBS requirements.md:620]`. D-75 moves the table and `eco::lookup` to `chess-rules` and corrects D-20 `[OBS architecture.md D-75]`. CR-10 says "no requirement text changes" for D-75 `[OBS requirements.md:1419 (a)]`. | The requirement text now points at a corrected decision's old content. | Log it as an editorial amendment (no shall-text change) or accept the stale pointer by note. | systems-engineer | G0 |
| m-3 | Minor | Behaviour change visible to the user; verification absent at requirement level. RULE-8, CR-9.11, UI-2 | SF-02/D-64: a start differing in any of six FEN fields is custom, unrated, marked "custom start" `[OBS architecture.md SF-02]`. So the standard placement with another move counter is a custom start. RULE-8: "A game started from a non-standard position shall always be unrated" `[OBS requirements.md:504]`; CR-9.11: "A FEN equal to the standard start" is standard. The six-field reading is consistent with "equal" and is the stricter reading of "non-standard position". RULE-8 verification has no case for it `[OBS requirements.md:508-509]`; the cases are only in SDDs (UT-DOM-115, UT-APP-29, UT-ST-084). CR-10 leaves it "still open, not decided" with no owner or date `[OBS requirements.md:1419 (3)]` although Brian chose six fields at G-3. | Low risk, but a user-visible rule (unrated, excluded from UI-2 aggregates) rests on SDD text. | Log the reading (no text change) and add a RULE-8 verification line pointing at the four-start table. Give the open item an owner and a date. | systems-engineer | G0 |
| m-4 | Minor | Untracked open item (se-requirements: TBD needs owner and date). NFR-2 / TBD-23, D-71 | CR-10 and RELEASE_NOTES say the `DurabilityUnsupported` refusal as an NFR-2 verification case or a `--data-dir` constraint is "not decided" `[OBS requirements.md:1419 (3); RELEASE_NOTES.md]`. TBD-23 is Closed `[OBS requirements.md:1229]`. No TBD/TBC/OQ entry carries it. | An open decision with no register entry can be lost. NFR-9 (message with next step) is the nearest requirement. | Register as an open question with owner and close-by, or decide: add a verification case under NFR-2 (CT-ST-085, CT-APP-33 already exist). Do not mint a TBD ID without an owner and date. | systems-engineer | G0 (verification case); G3 if a new `--data-dir` constraint |
| m-5 | Minor | Verification strength change recorded as "wording only"; non-measurable criterion. NFR-3, CLI-3 | CR-10 correctly changes no shall-text `[RAN git diff HEAD: 0 shall lines changed]`. But "fuzz" became stable random-input tests, accepted loss "coverage-guided fuzzing finds deeper parser faults" `[OBS architecture.md §11.4.9 residual risk]`. The requirements Decision Log does not state the loss. NFR-3's new cell has no case count, depth or pass criterion for the property tests (only the 1000-game soak is numeric) `[OBS requirements.md:1034]`. | A method downgrade approved at G2 (D-76) is invisible to a reader of the requirements. | Add one sentence on the accepted loss to CR-10, and a number-free pass criterion ("no panic, no illegal state over N generated cases, N set in the SDD") with N owned by the test authors. | systems-engineer | G0 |
| m-6 | Minor | CM: edited baselined document without a revision record | `test-strategy.md` was edited (rows "Property", "Calibration / strength", quality gates, NFR-3 check) `[OBS git diff]` but its header still reads "1.0, Amendment 4, 2026-10-28" and its change list has no Amendment 8 line `[OBS test-strategy.md:5, 17-21]` (also stale for Amendments 5 to 7 before this change, `[INF]`). No new rows for the Amendment 8 behaviours that architecture 11.4.6 told the owners to add (terminal re-check kill test, durable-prefix injection, `ENOTSUP` probe injection, `custom_start` table) `[OBS architecture.md §11.4.5 row 4]`. | Test-strategy cannot be used alone as the definition of done for Amendment 8. | Bump the header or add an Amendment 8 line; add one summary row for the crash-recovery and stored-format tests with SDD test ids. | systems-engineer | G0 |
| m-7 | Minor | Residual fuzz / nightly wording (NFR-3, D-76 consistency) | `SDD-chess-cli.md:690` FZ-CLI-1 level "Fuzz", traced to "test-strategy fuzz", a row that no longer exists; the CLI SDD is not in the L5 edit list `[OBS architecture.md §11.4.10 L5]`. `architecture.md:1842` NFR-3 allocation "fuzz, 1000-game soak". "nightly" without "scheduled" at `architecture.md:226, 249, 334`, `functional_decomp.md:126`, `SDD-chess-cli.md:546`. `cargo-fuzz` remains in `SDD-consistency-review.md:204` (history) and the readiness docs (history). No `cargo-fuzz`, `libfuzzer-sys` or nightly toolchain is required anywhere `[RAN grep, all docs]`. | No contradiction of D-76 in requirements, test-strategy or the three SDDs. The three in-scope sets are consistent. Residual hits are labels. | Relabel FZ-CLI-1 as "Property (`proptest`)" and keep its ID; fix line 1842; D-76 (5) already defines "nightly" as scheduled, so add "scheduled" at the CI-job mentions. | software-engineer; architect | G0 |
| m-8 | Minor | Status accounting | XC-DOM-A8-1, XC-APP-A8-3 "Closed" while the same rows say "the systems engineer still confirms the DAT-3 reading" `[OBS domain SDD 10; app SDD 11]`. XC-ST-A8-2 "Closed here; ASM-9 stays open". D-80 depends on XC-APP-A8-2, still open `[OBS architecture.md §11.4.12 finding 1]`. | A "Closed" label overstates; a reader cannot tell what still gates OQ-16. | Use "Applied, pending confirmation" for rows with a dependent open item. | SDD authors | G0 |
| m-9 | Minor | Provenance wording | RELEASE_NOTES heading "Added or decided: architecture (Amendment 8, approved by Brian at G-2)" covers a bullet list that includes Addendum A (D-75, delegated), Addendum D (D-78 to D-80, delegated) `[OBS RELEASE_NOTES.md]`. The bullets themselves name who ruled. | A skimmer may read D-75 and D-78 to D-80 as Brian-approved. | Change the heading to "Added or decided: architecture (Amendment 8 approved by Brian; later addenda as stated per bullet)". | systems-engineer | G0 |
| m-10 | Minor | Requirement-level verification missing for the DAT-3 time clause | DAT-3 verification (line 629) has no case for per-Player time, total time, or the closed/paused exclusion `[RAN grep of the DAT-3 verification line: no hit for "time spent", "elapsed", "paused"]`. Only SDD tests exist: UT-DOM-122, UT-DOM-123, UT-APP-26, UT-APP-32. | The requirement the whole of M-1 turns on has no requirement-level case, so a change to D-78 would not alter a requirement test. | Add a DAT-3 verification line: a game suspended and resumed N times stores per-Player time equal to the sum of spent time minus the stated loss (or equal, if Brian chooses B/C). | systems-engineer | G0 (text) |
| I-1 | Info | Gate naming | Architecture and CR-10 say "G-2 passed 2026-11-04 (chess-a0r.1.14)" `[OBS architecture.md §11.4.4]`. The work order says Brian decides G-2 for this review. I could not read beads `[UNV]`. | Possible collision of "G-2" (a plan gate) with "G2" (design tier). | Program-manager confirms which gate this review feeds. | program-manager | n.a. |
| I-2 | Info | Currency | RELEASE_NOTES and CR-10 say "No review of Amendment 8 has been held" and "OQ-16 stays open". True now (verified `[OBS]`; the claim does not state that a review took place). They will need an update after this review and its architect-side twin. | none now | Update on closure. | systems-engineer | G0 |
| I-3 | Info | UI-5, UI-2 effects | UI-5: no effect from `custom_start` or an advisory hash on a voided game (a voided game "never in any statistic", `[OBS requirements.md:790]`). UI-2: effect only through the six-field rule (m-3) and the D-78 loss on "average game length in time" (M-1). | none new | none | n.a. | n.a. |
| I-4 | Info | Numbers | I found no new TBD/TBR/TBC identifiers in the requirements or architecture diffs (only the existing TBD-23 cited) `[RAN git diff, grep TBD/TBR/TBC]`. Design values: 65 536-byte record limit (SF-09, rationale `[INF]`, approved by Brian), CRC-32 check value 0xCBF43926 (`[RAN python3 zlib.crc32]` per SF-08, not re-run by me `[UNV]`). `cargo-llvm-cov` 0.9.1 traced to the lookup report, not re-run. | none | none | n.a. | n.a. |
| I-5 | Info | D-80 and requirements | No requirement says a player can resign at any time (searched "resign" with shall) `[RAN grep]`. D-80 (Resign/Abort refused in a degraded session, reached after Detach and a resume) conflicts with no requirement I read. Message wording falls under NFR-9. D-80 still relies on XC-APP-A8-2 (open). | none | The system-architect desk review owns the G1 check. | system-architect | G1 |

## 3. CR-10 check (question 2)

- **Verification-wording only: confirmed `[RAN git diff HEAD -- docs/requirements.md docs/test-strategy.md]`.** Changes in `requirements.md`: header version/date/status cell, one change-history row, one Decision Log row (CR-10), and three verification lines (ENG "E (slow, scheduled...)", CLI-3 "(checked by a stable-toolchain random-input property test...)", NFR-3 cell). No line containing "shall" was added or removed; no requirement number, TBD, TBR or TBC entry changed. `test-strategy.md`: rows "Property", "Calibration / strength", two quality-gate bullets, and the NFR-3 check line; the 90 % gate now names `cargo-llvm-cov` 0.9.1 (a tool name inside a gate, still not requirement text).
- **Accuracy of the text:** it does not claim a review was held ("No review of Amendment 8 has been held, and nothing is implemented"), says OQ-16 "is open with the condition above", and labels D-78 to D-80 as the architect's rulings under delegation that Brian may reverse. Verified. Weaknesses are m-5 (accepted loss not stated), m-9 (RELEASE_NOTES heading) and M-1 (the elapsed-duration sentence).
- **Not verified:** that Brian said what CR-10 attributes to him at G-1, G-2 and for D-74 and D-81 `[UNV: beads chess-a0r.1.14, .1.15, .1.19 not read; bd not run]`. The claims trace only to architecture text.

## 4. Coverage table

Verdicts: **C** covered (named test with measurable pass/fail); **C*** covered, with a finding; **G** gap. SDD test ids from `[OBS]` grep of the three SDD test tables.

### SF-01 to SF-22

| ID | Requirement impact | Verification case(s) | Verdict |
|---|---|---|---|
| SF-01 totals in `GameEnd` | DAT-3 time; UI-2, UI-5 average time | UT-ST-082, UT-APP-26 (all `GAME` columns compared), UT-ST-070..073, UT-DOM-122 | C* (M-1, M-2, m-10 for resumed games) |
| SF-02 `custom_start`, six fields | RULE-8, CR-9.11, UI-2 "custom start" mark, DAT-3 ECO | UT-DOM-115, UT-APP-29, UT-ST-084 | C* (m-3) |
| SF-03 `sha256:v1:` marker | IF-2 duplicate detection | CT-APP-18, UT-ST-035 | C |
| SF-04 `Date` is UTC date of `started_at` | IF-1 export `Date` tag `[UNV: IF-1 text not read]` | CT-APP-18 (UTC-midnight game) | C |
| SF-05 `Voided.ended_at` | PLY-7 (void) | UT-ST-085, CT-ST-086, UT-ST-070..073 | C |
| SF-06 `Voided` DB effect, `journal_seq` | none direct | UT-ST-085 | C |
| SF-07 kind codes | none (internal, NFR-2 support) | UT-ST-081, UT-ST-070..073 | C |
| SF-08 CRC-32 | NFR-2 support | UT-ST-081 (check value 0xCBF43926) | C |
| SF-09 65 536 limit | NFR-2 support | UT-ST-081, decoder property test | C |
| SF-10 `VoidReason` strings | DAT-3 termination text unchanged | UT-DOM-121, UT-ST-070..073 | C |
| SF-11 `ClockSpec` text | RULE-4 clock | UT-DOM-121, UT-ST-070..073 | C |
| SF-12 UUID form | none | UT-ST-084, UT-ST-070..073 | C |
| SF-13 null `start_fen` | RULE-8 | UT-DOM-115, UT-APP-29, UT-APP-24 | C |
| SF-14 half-null clocks | RULE-5 | UT-ST-070..073 (decode error), UT-ST-084 (CHECK) | C |
| SF-15 snake_case reasons | DAT-3 termination strings unchanged | UT-DOM-121, UT-APP-24 | C |
| SF-16 no `Start.started_at` | DAT-3 start time | UT-APP-24, UT-APP-29, UT-ST-070..073 | C |
| SF-17 hash CHECK | none | UT-ST-035, UT-APP-33 | C* (ASM-9 open until app text accepted) |
| SF-18 clock-pair CHECK | RULE-5 | UT-ST-084 | C |
| SF-19 profile CHECKs | PLY-4 `[UNV]` | UT-ST-084 | C |
| SF-20 player-id CHECK | none | UT-ST-084 | C |
| SF-21 `Suspend.cause` | NFR-2 diagnosis | UT-DOM-116, UT-APP-31, UT-ST-070..073 | C |
| SF-22 `DrawDropped.by` | RULE-2 | UT-DOM-119, UT-APP-30 | C |

No SF item is orphaned (each traces to D-63, D-64, D-68, D-69 or D-70 and to Board BP-001 to BP-017 per architecture 11.4.3) or childless.

### D-63 to D-81

| Decision | Requirement impact | Verification case(s) | Verdict |
|---|---|---|---|
| D-63 | DAT-3 time | UT-ST-082, UT-APP-26 | C* (text contradicts D-78, M-1) |
| D-64 | RULE-8, UI-2 | UT-DOM-115, UT-APP-29, UT-ST-084 | C* (m-3) |
| D-65 (1) terminal re-check, (2) durable prefix | NFR-2, RULE-5 | UT-DOM-117, CT-APP-29, CT-ST-003; UT-APP-28 | C |
| D-66 store ahead of journal | NFR-2 precedence sentence | UT-ST-012, UT-ST-083, CT-APP-30 | C* (m-1) |
| D-67 voided hash advisory | none on UI-5 | UT-ST-086, CT-APP-32, CT-APP-18 (voided excluded) | C |
| D-68 hash scope, `Date`, marker | IF-2 | CT-APP-18 | C |
| D-69 `Voided.ended_at` | PLY-7 | UT-ST-085, CT-ST-086, UT-DOM-120 | C |
| D-70 SF-07..22 | see above | see above | C |
| D-71 refuse volume, `DurabilityUnsupported` | NFR-2 / TBD-23 (open item), NFR-9, CLI-4 `--data-dir` | CT-ST-085, CT-APP-33, CT-ST-001 | C* (m-4) |
| D-72 BP-011 text, BP-012 order, BP-013 SAN | none | BP-011 text only; CT-ST-084, CT-APP-31; UT-DOM-118, UT-APP-30 | C (BP-011 is text, no test needed) |
| D-73 calibration guard | RISK-2 | UT-APP-27, CT-APP-26 | C |
| D-74 `GameContextProvider` | ANA-1, PLY-5 (Strict integrity) | none named | **G** (M-3) |
| D-75 ECO placement | DAT-3 (a) pointer to D-20 | UT-RUL-023, UT-RUL-030 cited by architecture `[UNV: rules SDD not read]` | C* (m-2) |
| D-76 stable-only tools | NFR-3, CLI-3 verification wording | proptest cases across SDDs (FZ-RUL-001..003, FZ-DOM-001, FZ-EN-001, FZ-SRV-1, UT-ST-070..073, DD-APP-11); no case that the policy holds | C* (m-5, m-7); policy has no inspection check `[INF]` |
| D-77 no feature; store block | RISK-2 | UT-APP-05, UT-APP-27; (2) is a schedule rule, no test | C |
| D-78 resumed-game totals | DAT-3 | UT-DOM-122, UT-DOM-123, UT-APP-32 | C* (M-1, M-2, M-4, m-10) |
| D-79 `ReplayItem` / `MoveCheck` | none | UT-DOM-118, UT-APP-30, UT-APP-24, FZ-DOM-001 | C* (M-4) |
| D-80 degraded command set | NFR-2, NFR-9 | UT-APP-28, UT-APP-34 | C (relies on XC-APP-A8-2, I-5) |
| D-81 no nextest | none | policy; `[RAN]` grep finds no nextest mention in test-strategy or SDDs per architecture; not re-run by me | C |

Orphans: none. Childless: D-74 (M-3). Unverified: the DAT-3 time clause at requirement level (m-10).

## 5. Answers to the five checks

1. **Meaning changes without a log entry.** DAT-3 / D-78: not acceptable as "no conflict" on the author's reading alone (M-1). RULE-8 / CR-9.11 six fields: consistent with the text, reading unlogged, user-visible (m-3). NFR-2 / TBD-23 refusal: no weakening of NFR-2; the open item is untracked (m-4). NFR-2 precedence vs D-66: reading unlogged (m-1). D-75 vs DAT-3 (a): stale pointer (m-2). UI-2: affected only through m-3 and M-1. UI-5: none (I-3).
2. **CR-10 wording-only: yes** (section 3). The text claims no review and keeps OQ-16 open. Gaps: m-5, m-9, M-1 sentence.
3. **Verifiability:** every SF item and every decision except D-74 has a named test with an exact expected result in an SDD (section 4). D-74 is childless (M-3). Requirement-level verification is missing for the DAT-3 time clause and the six-field rule (m-10, m-3).
4. **Invented TBD/TBR/TBC or numbers:** none found (I-4). One untracked open item (m-4).
5. **NFR-3 / D-76 consistency:** requirements.md, test-strategy.md, and the store, domain and app SDD test levels all say stable-only random-input property tests; none names `cargo-fuzz`, `libfuzzer-sys`, `arbitrary` or a nightly toolchain as a requirement `[RAN grep]`. Residual label drift only (m-7).

## 6. What I did not read or verify

- Beads (`bd`) records chess-a0r.1.14, .1.15, .1.16 to .1.19: not run, so Brian's approvals and delegation are `[UNV]`, taken from architecture text.
- SDD-chess-rules, -engine, -server, -cli (beyond the fuzz/nightly grep), -ui, -desktop-tauri, SDD-consistency-review, use cases, functional_decomp beyond one grep line.
- Architecture sections outside 7.6, 11, 11.4.x, 9, and §14 ASM rows; no end-to-end read of the 2 662-line file.
- Requirements outside DAT-3, RULE-8, UI-2, UI-5, NFR-2, NFR-3, CLI-3, the ENG verification line, TBD-23, and the history and Decision Log rows. IF-1 and IF-2 text not read, so SF-04 and SF-03 requirement impact is `[UNV]`.
- No test, code or Mermaid diagram was run or validated; nothing exists to run. I did not re-run `zlib.crc32`, the lookup commands, or `mmdc`.
- Board-pass: read the verdict, BP-001, the gate record and the assumptions; did not re-audit BP-002 to BP-017.
- The line numbers cited for `requirements.md` and `architecture.md` are from the 2026-11-04 working tree and will drift.

## 7. Assumptions, open items, questions, next step

- **Assumptions:** A-1 the working tree on branch docs/amendment-8-and-sdd-alignment is baseline B2 text. A-2 "SDD test id with a stated expected result" counts as a verification case for design-level items. A-3 Brian's approvals are as the documents state them (unverified).
- **Open items (TBD/TBR/TBC):** none added by me. Existing: ASM-9, ASM-10, ASM-11, XC-APP-A8-2, XC-APP-A8-4, XC-DOM-A8-3 to -5, and the untracked NFR-2 / D-71 item (m-4).
- **Questions for Brian:** (1) Accept the unbounded unfinished-turn loss in resumed games (D-78), or take option B or C? (2) Ratify D-78 and D-79 as G2? (3) Which gate does this review feed (I-1)?
- **Next step:** Brian decides C-1 and C-4; the app SDD owner adds the D-74 case (C-3); the domain owner closes ASM-10 (C-2); the system-architect desk review reports separately; the systems engineer (author side) then enters the Decision Log updates for m-1 to m-5, m-9, m-10.
