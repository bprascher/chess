# Release Notes

## TBR-4 closed, OQ-17 part 1 accepted, rules SDD intent: 2026-11-06

Documentation-only (G0 record keeping). No code exists. Brian Prascher (G2 approver) ruled on 2026-11-06 (his words: "Close TBR-4 as is and I accept your recommendations for the other two decisions above"); logged as **CR-15**. **No requirement statement, pass criterion, verification-case content, decision text or ID changed. Nothing was renumbered.**

### Decisions recorded (CR-15)
- **TBR-4 closed (tracker chess-e64).** The NFR-2 full-sync refusal verification case is accepted as written (mapping to CT-ST-085, CT-ST-001, CT-APP-33). His 2026-11-05 ruling (TBR-4 kept open as a condition of the SDD baseline v1.0) supersedes CR-12 item 5 and FIND-R23-005, which said "accepted before the store SDD is baselined". CR-12 is not amended.
- **OQ-17 part 1 accepted (G2; tracker chess-out).** `docs/engineering/dependency-lookup-step1.md` v0.2 is accepted; no content change, version stays 0.2. The first Cargo.toml entry may now be made, still only via plan task chess-a0r.2.2, which is not started and is not authorised by this entry. Part 2 (steps 2 and 3 crates, chess-a0r.3.2) is unaffected.
- **`SDD-chess-rules` (0.3.5 DRAFT): decided intent** is to baseline it at v1.0 together with the first step-1 code; trigger: the first `chess-rules` code lands and its review passes (tasks chess-a0r.2.3 to chess-a0r.2.5 and chess-a0r.2.9). It is **not baselined now**.

### Phase 1 closed
- **Phase 1 (design closure, `chess-a0r.1`) is closed:** all 55 children are closed, the last being OQ-17 part 1 (`chess-a0r.1.49`). The plan baseline stays B4. No Phase 2 task is started or authorised by this entry; the next task is `chess-a0r.2.2` (Cargo workspace, CI and quality-gate scaffolding), which needs Brian Prascher's go-ahead. The tracker holds 153 issues: 75 closed, 78 open, none in progress.

### Open conditions (changes from the 2026-11-05 table)
| Tracker | Condition | Status |
|---|---|---|
| chess-e64 | TBR-4 | **Closed 2026-11-06 (CR-15)** |
| chess-out | OQ-17 part 1 acceptance | **Closed 2026-11-06 (CR-15)** |
| chess-ddu, chess-bix, chess-2bb, chess-ckq, TBD-36 | as listed 2026-11-05 | unchanged, still open |

### Documents changed
- `docs/requirements.md` (stays 1.0, Amendment 8, now extension 5; header date 2026-11-06): CR-15 Decision Log row and change-history row; dated notes in the NFR-2 Verification cell and the TBR-4 register row (case text unchanged); header.
- `docs/engineering/dependency-lookup-step1.md`: status line only.
- `RELEASE_NOTES.md`: this entry; superseded-by notes in the 2026-11-05 entry.

### Not done, not verified
- Baselined SDDs `SDD-chess-store` (section 2.x matrix row at line ~970 says the mapping "is the text to accept before this SDD is baselined"; change-log 1.0 row lists chess-e64 and chess-out as open) and `SDD-chess-app` (change-log 1.0 row) carry the 2026-11-05 wording. They are history as of their date; not edited, because they are baselined documents. Suggest the software-engineer refresh them with the next SDD revision.
- No test, build or run exists.

## SDD baseline v1.0: 2026-11-05

Documentation-only (G0 record keeping). No code exists. Brian Prascher (G2/G3 approver) ruled on 2026-11-05 (tracker chess-a0r.1.53) accepting the program manager's recommendations; work order CR-4 task T-D (chess-a0r.1.54.4). **No requirement statement, pass criterion, decision text or ID changed. Nothing was renumbered.** The Decision Log entry is **CR-14** in `docs/requirements.md`. This entry supersedes the line "The SDD baseline v1.0 decision (`chess-a0r.1.11`, G2) is NOT made" in the entry below, which is history as of its date.

### Baselined at v1.0 (label change only; no design content changed)
| SDD | Version before | Version now |
|---|---|---|
| `docs/design/SDD-chess-domain.md` | 0.4.6 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-chess-store.md` | 0.4.5 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-chess-engine.md` | 0.3.4 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-chess-app.md` | 0.4.8 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-chess-cli.md` | 0.4.5 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-chess-server.md` | 0.4.2 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-ui.md` | 0.4.5 DRAFT | **1.0 BASELINED** |
| `docs/design/SDD-desktop-tauri.md` | 0.4.3 DRAFT | **1.0 BASELINED** |

Each SDD has a change-log row 1.0, an Approver and Gate-record statement ("baselined at v1.0 by Brian Prascher, G2, 2026-11-05; G1 desk reviews as recorded; open conditions listed") and a baseline row in its review record. The change logs of the CLI, server, UI and Tauri SDDs were re-sorted into version order (FIND-R23-007, mechanical; no row text changed). No other approval is claimed.

**Not baselined: `SDD-chess-rules` (0.3.5 DRAFT).** It was readiness-checked separately and the ruling did not name it. Open question for Brian: baseline it, and when. *(Superseded 2026-11-06, CR-15: intent is to baseline it with the first step-1 code.)*

### Rulings recorded (CR-14)
- OQ-APP-015: option B (D-89); desk review `SDD-G1-rereview-2026-11-05.md` APPROVE WITH CONDITIONS; fix pass applied.
- TBR-5: N >= 10 000 generated cases per NFR-3 property per run accepted as **PROVISIONAL** (runtime measured at step 1). TBR-4 stays open and is carried as a condition *(superseded 2026-11-06: TBR-4 closed, CR-15)*. CR-12 item 5 and FIND-R23-005 said TBR-4 should be accepted before the store SDD baseline; Brian is asked to confirm the reading. CR-12 is not amended.
- Draw-offer interval is not counted (D-78 default stands; the DAT-3 case accepts the shortfall). TBD-36 is deferred to chess-a0r.3.16 and stays open in the register. ASM-9 closes at the baseline (architecture.md is edited by the architect, not in this entry). OQ-17 part 1 reissue waits for Brian's final G2 acceptance *(superseded 2026-11-06: accepted, CR-15)*.

### Open conditions (owner, date)
| Tracker | Condition | Owner | Due |
|---|---|---|---|
| chess-ddu | OQ-APP-016(b), FND-RR2-04 and FND-RR2-06 | software-engineer, architect desk review | before CLI and UI reducers are coded |
| chess-bix | architecture 11.4.15 fact 8 and D-89 untimed-game alignment | architect | with the next architecture amendment, before app code |
| chess-2bb | XC-DOM-A8-5 | software-engineer | step-1 domain code |
| chess-e64 | TBR-4 (NFR-2 full-sync refusal case) | systems-engineer proposes, Brian reviews | before store code, chess-a0r.3.6. **Closed 2026-11-06 (CR-15).** |
| chess-out | OQ-17 part 1: final G2 acceptance of `dependency-lookup-step1.md` v0.2 | Brian Prascher | before the first Cargo.toml entry, chess-a0r.2.2. **Closed 2026-11-06 (CR-15).** |
| chess-ckq | OQ-14 spike scope and GATE-101+ records | software-engineer | first store code; each block's first code task |
| (register) TBD-36 | CLI signal and subcommand exit status (G3) | Brian Prascher | chess-a0r.3.16, before CLI code |

### Accepted residual risks
- FIND-R23-015: lightly reviewed text (accepted by Brian).
- The fix-pass text applied after the G1 limited re-review was not independently re-reviewed.
- OQ-APP-016(b) is open, Minor.

### Other documents changed
- `docs/requirements.md` (stays 1.0, Amendment 8, extension 4): CR-14; TBR-5, TBR-4 and TBD-36 register notes (TBD-36 close-by now names chess-a0r.3.16, FIND-R23-011); header and change-history row.
- `docs/engineering/SWE-001-engagement-profile.md`: "Upstream artifacts" row now says eight SDDs are baselined v1.0 and the rules SDD is DRAFT. Its other upstream citations (requirements through CR-12, architecture through D-88) were not refreshed here.
- `README.md` status lead and the Project management paragraph; `CLAUDE.md` document-map line 5 and the Architecture-in-brief SDD sentence.
- `docs/architecture.md`: not edited by this entry (the architect closes ASM-9 and aligns wording in parallel).

### Not done, not verified
- Stale "upstream baselines read" statements in `SDD-chess-domain` (header row, "not re-read for 0.4.5"), `SDD-chess-engine` (header row) and `SDD-chess-store` (section 2.1) were not refreshed: a refresh would claim a re-read that was not done. They need an owner (suggest software-engineer, with chess-bix).
- No test, build or run exists; every claim about code is design text.

## Status wording and record keeping after G-2: 2026-11-05

Documentation-only (G0). No code exists. **This entry records status wording and register edits. It records no new decision, no new gate result and no change to any requirement statement, pass criterion, decision text or ID. Nothing was renumbered.** It closes two items listed as open in the 2026-11-04 entry below: the Decision Log entry for the G-2 closure is now written (CR-13), and most of the stale "OQ-16 is open" wording now carries a dated pointer. That entry stays as history.

### Readiness verdict (not a baseline decision)
- `docs/engineering/readiness-step2-3.md` (new, a fresh reviewer-stance invocation, 2026-11-05) gives **READY-WITH-CONDITIONS for all eight blocks** (`chess-domain`, `chess-store`, `chess-engine`, `chess-app`, `chess-cli`, `chess-server`, `ui`, `desktop-tauri`): 0 Blocker, 5 Major, 10 Minor, conditions dated to the step that needs them. It is findings and questions only and approves nothing.
- **The SDD baseline v1.0 decision (`chess-a0r.1.11`, G2) is NOT made.** It is Brian Prascher's. The SDDs stay DRAFT.

### Changed, per document
- `docs/requirements.md` (stays 1.0, Amendment 8; header date 2026-11-05): Decision Log entry **CR-13** (G-2 passed, OQ-16 closed 2026-11-04; supersedes "OQ-16 stays open" in CR-10, CR-11 and CR-12, whose rows are unchanged; recommended case count N for TBR-5 as a proposal, not binding; ASM-DOM-008 reading confirmed as an interpretation); change-history row and header status extended for CR-13; register: **TBR-5 added** (NFR-3 has no case count N, finding RR-S-3; Brian decides, G2), TBD-36 close-by wording corrected ("before step 5" matched no step of architecture section 13), TBR-4 close-by wording corrected (it stays open; not accepted). No requirement statement changed.
- `docs/architecture.md`: change-log row 2026-11-05 (status-wording pass, no amendment number, no addendum, no decision added). Pointers after the sentences that said OQ-16 is open or the Board pass is required; OQ-22 closed (its condition was met); ASM-9 annotated and left open pending a ruling; section 3.4 Rust and Node notes refreshed (`[RAN 2026-11-05]` on the development Mac: Rust 1.99.1, Node v26.13.0; other machines not checked).
- SDDs (all still DRAFT, G0/G1 wording, not reviewed): `SDD-chess-rules` 0.3.4, `SDD-chess-domain` 0.4.4, `SDD-chess-store` 0.4.4, `SDD-chess-app` 0.4.5, `SDD-chess-engine` 0.3.4, `SDD-chess-cli` 0.4.2, `SDD-chess-server` 0.4.2, `SDD-ui` 0.4.2, `SDD-desktop-tauri` 0.4.2.
- `CLAUDE.md`: the status line on the SDDs and OQ-16 now says OQ-16 closed on 2026-11-04 (Brian Prascher, G-2, chess-a0r.1.47), SDDs are 0.3.x/0.4.x DRAFT, and Brian has not decided the SDD baseline.
- `docs/engineering/SWE-001-engagement-profile.md`: 1.1.1 (2026-11-05), wording update.
- `docs/engineering/readiness-step2-3.md`: new (see above).
- `README.md`: the document table said "Amendment 3" for requirements and "amendment 3" for the architecture. It now says Amendment 8 (and addenda A to F for the architecture).

### Open at this release
- **SDD baseline v1.0 (`chess-a0r.1.11`, G2): Brian's decision, not made.**
- **TBR-5** (NFR-3 case count N), **TBR-4** (NFR-2 full-sync refusal case, until the store SDD baseline) and **TBD-36** (CLI signal exit status, G3) are open for Brian.
- **No `chess-store` code starts until** the `flock`/`F_FULLFSYNC` spike (`chess-a0r.3.1`, OQ-14), the part 2 crate lookups (`chess-a0r.3.2`, OQ-17) and `chess-domain` part B (`chess-a0r.2.7`) close. OQ-16 no longer blocks it.
- The draw-offer time before a stop (`chess-a0r.1.45`, G3), ASM-9, and the post-G-2 minors (`chess-a0r.1.38`).
- Nothing is built. Every test, crash and timing claim is design text, not run.

## Gate G-2 passed and OQ-16 closed: 2026-11-04

Documentation-only. No code exists. Nothing here changes a requirement statement, a decision text or a number. The entries below this one that say OQ-16 is open are dated history; this entry supersedes them.

### Recorded: Brian Prascher's decisions of 2026-11-04 (beads chess-a0r.1.47)
- **D-83 added cases confirmed (G2):** synthesize `Start` from the stored row when the intact journal run lacks it; do not rebase a final or voided game; use no-replace renames; sync both directories.
- **Gate G-2 (`chess-cdb`) passed and OQ-16 closed.** Basis: the stored-format batch is applied (Amendment 8 and addenda B to F); the architect and systems-engineer desk reviews and the limited re-review were each PASS WITH CONDITIONS; the conditions were addressed in text; the closing pass was checked mechanically (ID scan, Mermaid validation, version headers); Brian's word. No reviewer has seen the closing-pass text, and the Board was a single same-family reviewer that ran no ATAM, security or performance lens (see GATE-008).
- **Not decided:** the SDD baseline v1.0 (`chess-a0r.1.11`, G2). The SDDs stay DRAFT (store 0.4.3, app 0.4.4, domain 0.4.3, rules 0.3.3, engine 0.3.3, cli, ui, desktop-tauri and server 0.4.1). The draw-offer time before a stop (`chess-a0r.1.45`, G3) stays as designed, with no stamp on draw facts.

### Changed
- `docs/architecture.md`: status-only edits (G0). The OQ-16 row reads "Closed ... by Brian Prascher, 2026-11-04", with the old text kept as history, and status notes on GATE-006, GATE-007, GATE-008 and D-77 part 2. No decision text or numbering changed.
- Beads: the decision record `chess-a0r.1.47`; `chess-a0r.1.46`, gate `chess-cdb` and CR-1 (`chess-a0r.16`) closed. The plan baseline is B3.

### Open at this release
- **No `chess-store` code starts until its own prerequisites close:** the `flock`/`F_FULLFSYNC` spike (`chess-a0r.3.1`, OQ-14), the part 2 crate lookups (`chess-a0r.3.2`, OQ-17) and `chess-domain` part B (`chess-a0r.2.7`). OQ-16 no longer blocks it.
- **TBR-4** (NFR-2 full-sync refusal case) stays open until the store SDD is baselined.
- **The Decision Log entry** for this closure in `docs/requirements.md` is not yet written.
- **Stale wording:** several documents still say OQ-16 is open as dated history (architecture history rows, earlier release-note entries, `SDD-chess-domain.md`, and the store and app SDD headers).
- **Post-G-2 minors** (`chess-a0r.1.38`, `chess-a0r.1.44`) and the SDD baseline decision (`chess-a0r.1.11`).
- Nothing is built. All test, crash and timing claims are design text, not run.

## CR-3 closing pass: Brian's decisions of 2026-11-04 recorded (CR-12, architecture Addendum F): 2026-11-04

Documentation-only. No code exists. Requirements Decision Log entry: CR-12. Nothing here is reviewed beyond its author, and OQ-16 stays open. No requirement statement, number or TBD status changed.

### Recorded: Brian Prascher's decisions of 2026-11-04
- **D-78 is ratified, option A (OQ-28, G2).** A game that is stopped and resumed may lose up to one turn of time at each stop; it is not required to. The DAT-3 statement is not changed. **D-79 and D-80** were ratified earlier the same day (CR-11).
- **OQ-26, option A (G2):** `NewGame` gains `started_at`, supplied by the app. **OQ-27, option A (G3):** `canonical_form` omits `SetUp` and `FEN` when the `FEN` equals the standard start. **D-83** is rated G2 and approved; D-82 stays G1. The D-71 refusal of a directory that cannot do full sync is an NFR-2 verification case, tracked as TBR-4.

### Changed
- `docs/architecture.md`: Addendum F (D-78 ratified, `NewGame.started_at`, the `canonical_form` omission, re-review findings RR-A-1 to RR-A-5 fixed as text, section 11.4.14 on the draw-offer time).
- `docs/requirements.md` (Amendment 8, extension 2): Decision Log entry CR-12; a DAT-3 time-clause verification case worded as bounds (L <= T <= U) that accepts a shortfall and requires none, including draw-offer time before the last stop, which the design's seed does not count (a design that counts it needs a stamp on draw facts, a Brian decision if wanted); an NFR-2 full-sync refusal case with TBR-4 (mapped to CT-ST-085, CT-ST-001, CT-APP-33); the RULE-8 row 4 now names UT-RUL-031, UT-DOM-115 and UT-APP-29 (also CT-RUL-004, UT-APP-39); an appended correction to CR-11 listing the final SDD versions.
- `docs/design/PROPOSED-D-78-DAT-3-for-Brian.md` 0.2 (decided, option A).
- SDDs, final versions: store 0.4.3, app 0.4.4, domain 0.4.3, rules 0.3.3, engine 0.3.3, cli 0.4.1, ui 0.4.1, desktop-tauri 0.4.1, server 0.4.1 (all DRAFT, G1 text, not reviewed).
- The older Amendment 8 section below now points to the ratification of D-78 to D-80 (RR-S-9).

### Open at this release
- **OQ-16 stays open.** No `chess-store` code starts before it closes (D-77).
- TBR-4 (NFR-2 full-sync refusal case) stays open until the store SDD baseline.

## Brian's rulings of 2026-11-04 and architecture Addendum E recorded: 2026-11-04

Documentation-only. No code exists. Requirements Decision Log entry: CR-11. Nothing here is reviewed beyond its author, and OQ-16 stays open.

### Recorded: Brian Prascher's rulings of 2026-11-04 (beads chess-a0r.1.30)
- **D-79 and D-80 are ratified** by Brian. **D-78 is not ratified.** It awaits his choice between A (accept the loss of the unfinished turn at each stop, amend DAT-3, add a verification case), B (totals in `Suspend`) and C (totals in every `Clock` record). The architect's proposal is docs/design/PROPOSED-D-78-DAT-3-for-Brian.md (recommendation A, confidence Medium). Nothing was applied: DAT-3, its verification and every time-accounting wording are unchanged. Open as OQ-28.
- **A data directory that cannot do full sync is refused** (`DurabilityUnsupported`, D-71), and the command-line program exits with class 3 (store error). Brian accepted this as a G3 decision. No new exit code number.
- Open for Brian, not applied: OQ-26 (how the game row gets the one start time, G2) and OQ-27 (the canonical form of an imported game with a standard `FEN` tag, G3).

### Added or decided: architecture Addendum E (architect, text only)
- Two OQ-16 desk reviews were held on 2026-11-04 (system-architect part and systems-engineer part); both returned PASS WITH CONDITIONS and their conditions are not met. The architect answered the architect-owned findings as D-82 to D-88: what the journal writer does after a failed append, how a damaged journal is rebased, one start time for the game row and the journal (the field is OQ-26), the end time of a game ended by the recovery re-check, how a held session behaves, a `GameEnd` followed by `Voided`, and a start that was never acknowledged. The wording of two journal fitness functions (FF-3, FF-4) is corrected, and ASM-10 is closed on design evidence (the runtime fact is not yet run).
- The store, app and domain SDDs are at 0.4.2 DRAFT. The front-end SDDs are aligned separately (chess-a0r.1.32).

### Changed: requirements (CR-11), test strategy, functional decomposition
- `docs/requirements.md`: Decision Log entry CR-11 and a change-history row; two verification additions (a RULE-8 four-start line; a pass criterion in the NFR-3 verification). **No requirement statement, number or TBD/TBR/TBC status changed.** The entry logs four readings: NFR-2's precedence sentence against D-66 (consistent), the stale D-20 pointer in DAT-3 (a) (accepted by note), the RULE-8 six-field reading, and the accepted loss of coverage-guided fuzzing under D-76.
- `docs/test-strategy.md`: header and Amendment 8 revision line, and the crash-recovery and stored-format tests named in the Integration row.
- `docs/functional_decomp.md`: F-3.9 says "run on a schedule" (D-76); an Amendment 8 note is added to the change log.
- The heading of the Amendment 8 section below no longer reads every later addendum as Brian's approval.

### Open at this release
- **OQ-16 stays open.** The conditions of both desk reviews are not met and the independent re-review has not been held. No `chess-store` code starts before OQ-16 closes (D-77). No gate is recorded as passed by this entry.
- **D-78 and DAT-3 (OQ-28)** wait for Brian. Until then the reading of DAT-3 in CR-10 is the systems engineer's interpretation, neither ratified nor reversed. A DAT-3 verification line for the time clause and the treatment of a pending draw offer in the time totals (XC-DOM-A8-4) wait with it.
- Whether the full-sync refusal is a verification case of NFR-2 or a constraint on `--data-dir` is open (CR-11 item 9); no calendar date is set.
- Nothing is implemented.

## Architecture Amendment 8 and addenda A to D recorded; verification wording follows D-76: 2026-11-04

Documentation-only. No code exists. Requirements Decision Log entry: CR-10. Brian Prascher approved architecture Amendment 8 at gate G-2 on 2026-11-04 and applied his G-1 dispositions of the independent Board pass on the store and journal (docs/design/Board-pass-store-journal.md). Addenda A to D were added the same day.

### Added or decided: architecture (Amendment 8 approved by Brian; later addenda as stated per bullet)
- The finalize time totals are carried in the `GameEnd` journal record; `custom_start` is defined once (all six FEN fields compared) and stored; a terminal position is ended, not resumed, after a crash; a failed multi-record step adopts the durable prefix; a database ahead of a damaged journal tail keeps its moves.
- OQ-20 is decided as option A (the hash of a voided game is advisory). The hash scope, the `Date` rule (UTC date of `started_at`) and a version marker in `content_hash` (`sha256:v1:<hex>`) are fixed. `Voided` gains `ended_at`; `Suspend` carries a `cause`; journal reason codes are snake_case; the `Start` record has no `started_at`. One stored-format batch of 22 items (SF-01 to SF-22) with rollback notes. A data directory without `F_FULLFSYNC` is refused.
- New decisions D-63 to D-73, GATE-008, OQ-24, OQ-25, ASM-9 to ASM-11 and Q-A20 (answered).
- Addendum A: the ECO table and `eco::lookup` stay in `chess-rules` and the "deepest named position" rule stays in `chess-domain` (D-75, readiness C-1, decided by the architect under Brian's delegation). The gateway game-state source is D-74 (`GameContextProvider`), proposed by the architect and confirmed by Brian as a G2 decision.
- Addendum B (D-76, approved by Brian): no nightly Rust toolchain is ever used; `cargo-llvm-cov` 0.9.1 is the coverage tool; `cargo-fuzz`, `libfuzzer-sys` and `arbitrary` are not used; illegal-state verification (NFR-3) uses stable-only property tests.
- Addendum C (D-77, ruled by Brian): no `calibration` Cargo feature in v1 (OQ-24 closed); no `chess-store` code of any kind before OQ-16 closes (OQ-25 closed).
- Addendum D (ruled by the architect under Brian's delegation; Brian may reverse D-78 to D-80): the time totals of a game that is suspended and resumed are seeded from the journal's existing `Move.think_ms` values, with no new journal field and with the unfinished turn at each stop not counted (D-78); `Game::replay` takes a record's `san` and `fen_after` beside each `Move` so the SAN check stays in the domain and the `Fact` does not change (D-79); a session whose journal append failed part-way accepts only queries, detach and shutdown, which write nothing, and the player resigns or aborts after resuming (D-80). D-81: `cargo-nextest` is not adopted for now; tests run with plain `cargo test`. **Superseded 2026-11-04:** D-79 and D-80 were ratified by Brian on 2026-11-04 and D-78 by option A (see CR-11 and CR-12 in requirements.md and the closing-pass section above).

### Changed: requirements (Amendment 8, CR-10) and test strategy
- **Verification wording only, per D-76 and D-81:** NFR-3, the CLI-3 verification line and the ENG verification line "E (slow, nightly)" in `docs/requirements.md`; the rows "Property" and "Calibration / strength", the quality gates and the NFR-3 check in `docs/test-strategy.md`. Stable-only random-input property tests replace "fuzz"; "nightly" now says it is a run schedule, never the nightly Rust toolchain; the 90 % line-coverage gate names `cargo-llvm-cov` 0.9.1.
- No requirement statement, number, or TBD/TBR/TBC status changed. DAT-3 is unchanged; CR-10 records the systems engineer's reading that DAT-3 gives no accuracy bound and does not forbid the loss stated in D-78 (an interpretation, which Brian may reverse).

### Open at this release
- **OQ-16 stays open.** It closes only after the stored-format batch is carried into the SDDs (chess-a0r.1.8) and a desk review by the system-architect and the systems engineer, in separate invocations from the author, has passed. No `chess-store` code starts before then (D-77).
- **No review of Amendment 8 has been held.** Brian's approval is the text of the amendment, not a review. Nothing is implemented.
- NFR-2 / TBD-23: whether the refusal of a data directory that cannot do a full sync is a verification case of NFR-2 or a new constraint on `--data-dir` is not decided.


## Project management moves to beads; delivery plan approved: 2026-11-04

Process-only release. No requirement, architecture, SDD or code text changed. No code exists yet.

### Added: project management in beads
- **Delivery plan v1 approved by Brian Prascher on 2026-11-04.** It takes the project from the SDDs at 0.3.1 DRAFT to a verified v1 release and is tracked in beads under program epic `chess-a0r`: five phase epics (P1 design closure, P2 foundation, P3 core services, P4 GUI, desktop and packaging, P5 verification and release), 69 tasks, spikes and decisions, and 10 risk issues. There are 103 dependency edges and no cycles. The plan is a draft of 87 deferred issues in beads until the program manager baselines it; that step is recorded separately.
- **Human gates G-1 to G-6** are all Brian Prascher's: G-1 Review Board findings, G-2 Amendment 8 and OQ-16, G-3 to G-5 exit of architecture steps 1 to 3, G-6 verification, residual risks and release.
- **Decisions recorded with the approval:** Review Board composition as recommended by the program manager (a software engineer in Reviewer stance, in a fresh invocation, separate from the SDD authors); the SDDs are to be baselined; autonomy levels as proposed (P1 and P2 supervised, P3 and P4 review-each, P5 run-to-gate); the later decisions (TBD-36 CLI exit statuses, the D-4 chessground spike outcome, TBD-18 wording, a clean macOS 26 install walkthrough) stay deferred to their tasks.
- **Estimates are not measured.** About 274 hours of effort plus a 20% contingency, with a longest chain of about 137 hours. There is no velocity data behind them.
- `.beads/issues.jsonl` (the passive export of the tracker) is now tracked. `AGENTS.md` and the beads block in `CLAUDE.md` were refreshed by the beads tooling.

### Open at this release
- No SDD is approved. The independent Review Board pass on D-41 to D-43 and Addendum A (OQ-16) has not been held and still blocks all `chess-store` code.
- The development Mac's Rust toolchain (1.89.0) is older than the baseline (1.99.1) and must be updated before architecture step 1. Node is at the baseline 26.13.0. The open items listed in the previous release are unchanged.

## Amendment 7, addendum C: PGN import stays two steps: 2026-11-01

Documentation-only. Brian Prascher ruled on OQ-23 and on the findings of `docs/design/SDD-v0.4-consistency-pass.md`.

### Changed: architecture
- **D-62, §5.8, OQ-23 closed:** duplicates and the ECO mismatch question stay two steps. A PGN `ImportConflicts` reply holds one kind of entry: duplicates come first with no `import_id`, then the ECO question with an `import_id`. `ResolveImport{import_id, eco}` has no `on_duplicate`; the held state keeps the policy of the creating `ImportPgn`. The archive preview is one stateless list that holds conflicts and `eco_mismatch` entries together. No protocol shape changes; the wording that allowed both kinds in one PGN reply is withdrawn.

### Changed: SDDs (all nine, G0)
- New review documents: `docs/design/SDD-v0.2-joint-review.md`, `SDD-v0.3-joint-review.md`, `SDD-v0.4-consistency-pass.md` (findings and dispositions) and `SDD-SE-desk-answers.md`.
- Version label unified to 0.3.1 DRAFT; Status rows of the CLI, server, UI and Tauri SDDs updated; D-60, D-61 and D-62 cited. App IFP-APP-03 `eco_mismatch` entry corrected to `moves_san` (no `tag_name?`); stale "architecture is being updated" wording removed; OQ-CLI-006 and OQ-UI-005 closed. Nothing is approved.

## Triage decisions and desk answers resolved, Amendment 7: 2026-10-31

Documentation-only release. No application code exists yet. The user (Brian Prascher) resolved the system-architect's 13 triage decisions and the systems engineer's 26 desk questions. Requirements Decision Log entry: CR-9 (CR-9.1 to CR-9.13; CR-9.8 to CR-9.13 added later the same day, see below).

### Changed: requirements (Amendment 7, CR-9)
- **ANA-1, ANA-3:** evaluations are shown and stored from White's point of view; centipawn loss stays from the mover's point of view (CR-9.3).
- **ANA-2:** a what-if branch started from the final position can't be saved; the save is refused with a message. This limits "optionally saves the branch" (CR-9.4).
- **DAT-2, IF-5:** the browser and desktop front-ends have an archive import size limit, stated as an estimate (TBD-35, about 24 MiB, not measured), with a message that names the limit, the file size and the CLI alternative. The CLI imports by path with no limit. PGN import stays 10 MB (TBD-25) (CR-9.5).
- **RULE-4, CLI-5:** notes pointing to the new register entries.
- **Register:** TBD-34 (custom clock limits, EST), TBD-35 (archive import size limit, EST) and TBD-36 (exit status of TUI/plain-mode signals, non-headless subcommands and extra setup flags; open, deferred) added. TBD-18 and TBD-25 noted.
- **Interpretations confirmed** with no text change: Q-SE-1, 2, 3, 4, 5, 12, 13, 18, 19, 22, 25. Q-SE-6, 7, 10, 11, 14, 15, 16, 21, 23, 24, 26 recorded as already answered by existing requirements (CR-9.2).

### Added to Amendment 7: rulings on the SDD v0.2 joint review (CR-9.8 to CR-9.13)
- **DAT-3 (CR-9.8):** an imported PGN that is a custom start (`SetUp`/`FEN`) and carries an `ECO` tag keeps the file's code, with no name from the table. A custom start without a tag stays unclassified. A sub-bullet and one verification case were added; no statement was deleted. Q-SE-24 in docs/design/SDD-SE-desk-answers.md is marked superseded (text kept).
- **ID policy (CR-9.9):** the one-time ID normalisation in the rules, domain and engine SDDs (zero-padding, `R-EN-n` to `DR-EN-nnn`, `EI-EN-02` to `EI-EN-002`) is an accepted exception to "do not renumber". Each SDD keeps its own finding prefix (`FIND-` or `UF-`). CLAUDE.md itself was not edited.
- **Recorded by reference (CR-9.10):** `content_hash` in the `GameEnd`, `Abort` and `Voided` journal payloads; the domain `Fact` carries the `DrawDropped` and state-change payloads; the store returns the highest stored `roster_index` and the app calls `first_k_after`; `chess-app` owns the calibration run; the architect settles the `ParseMove` error shape and the global event envelope. The rulings are applied by the other agents; this release records them only.
- **docs/design/SDD-v0.2-joint-review.md:** disposition section added marking the 7 items for Brian as DECIDED. Findings unchanged.
- **DAT-3 (CR-9.11):** a custom start that later reaches a named position is classified by it (code and name from the table), and a FEN equal to the standard start counts as a standard start. Two verification cases added. This removes the "standard start only" restriction in domain 3.11.9 (`EcoTable::observe`), rules FIND-RUL-7 / `eco_for_import` and app CT-APP-18; those owners must follow.
- **DAT-3, IF-2 (CR-9.12):** when an imported PGN's `ECO` tag does not match the table's classification of the position (any import, not only custom starts), the app flags it and asks the player to keep the PGN's ECO or the position classification. This supersedes "the file's code is kept" only for a mismatch; agreement, "table finds nothing" and "no tag" are unchanged. Verification cases added.
- **DAT-3, IF-2, CLI-5 (CR-9.13):** the three open ECO questions are closed. Interactive imports ask per game with "apply to all remaining"; non-interactive imports (CLI plain mode, headless) need `keep-pgn` or `keep-table`, else exit 1 with the mismatches listed and nothing written; keeping the PGN's ECO stores an empty name; the choice is not remembered in v1. Architecture D-59 (two-step import flow) accepted as proposed, which also confirms that in an archive only games without stored ECO in `games.json` are asked. DAT-3 verification gets the expected results and a non-interactive case.
- Downstream, not edited: use cases and test cases for import; exact option spelling (CLI design, G3).

### Added to Amendment 7 on 2026-11-01: corrections to the ECO import rulings (CR-9.14)
- **DAT-3, IF-2, CLI-5 (CR-9.14):** plain mode **asks** the ECO mismatch question when stdin is a terminal. Only runs with stdin not a terminal (piped or scripted) and headless runs need an explicit option, else they exit 1, list the mismatches and write nothing. This corrects the CR-9.13 and CLI-5 wording that called "CLI plain mode" non-interactive.
- CLI option values are hyphenated: `--eco keep-pgn` or `--eco keep-table` (protocol values `keep_pgn` and `keep_table`; the CLI maps).
- A kept PGN ECO is stored normalised (trimmed, upper-cased) if well-formed (A00 to E99); a malformed tag is treated as no tag; the stored name is empty.
- The start position of a custom start (ply 0) is looked up in the ECO table too. The `eco_mismatch` entry carries an additive SAN `moves` text. Archive imports are stateless and `CancelImport` is for PGN imports only. The pending-import idle time-out is a 10 minute estimate (design value, not a requirement).
- Verification cases added for plain mode at a terminal, plain mode with stdin not a terminal, ply 0, malformed tag and normalisation. Decision Log entry CR-9.14. docs/design/SDD-SE-desk-answers.md notes that OQ-DOM-009 is answered; docs/design/SDD-v0.3-joint-review.md gains a short Disposition section (findings unchanged).
- Downstream, not edited: architecture D-59 and the domain, rules, app, CLI and UI SDDs, use cases and test cases must follow.

### Changed: architecture and design
- **Architecture amendments AMD-1 to AMD-14 applied** to docs/architecture.md (the approved decisions: `chess-app -> chess-rules` edge and front-ends on `chess-app` only, concrete `Store`, UUID identities, the stored-format batch, `Query::ParseMove`, domain-computed statistics, pinned `UCI_Elo` range, feature-gated `ts-rs` and `GameCommand`, file transfer, protocol additions, dependencies in principle, White's-point-of-view scores, variation tree). The architect owns that text; see architecture.md for its change log.
- **SDDs updated to v0.2** to follow the approved decisions and the desk answers.
- **docs/design/SDD-architect-triage.md:** approver recorded as APPROVED by Brian Prascher, 2026-10-31, decisions 1 to 13, with modifications to 9 (archive size limit as an estimate), 11 (single-instance mechanism dropped; not a requirement) and 13 (no `player_id`; saving a what-if branch from the final position is refused). The recommendation text is kept.
- **docs/design/SDD-SE-desk-answers.md:** approver recorded as resolved.
- **docs/functional_decomp.md:** amendment note (change log) for F-8.1, F-8.2 and F-3.7. No function ID, name, allocation or trace changed; the document stays v1.0 (CR-9.7).

### Open at this release
- **TBD-36:** exit status for the TUI and plain mode on SIGTERM and SIGHUP, for the non-headless subcommands, and any extra setup flags. The user deferred it (public CLI surface, G3). It must be decided before the CLI front-end (step 5).
- **TBD-35** is an estimate that is not measured. **TBD-34** is an estimate of limits. **TBD-18** sample wording still awaits the user's review.
- Dependency versions are not looked up. Each crate needs a lookup before it enters architecture 3.4.
- No SDD has had a reviewer pass or desk review.
- Intel Mac verification remains not run (CR-7.6).

## Detailed design drafts (SDDs) and Amendments 5 and 6: 2026-10-30

Documentation-only release. No application code exists yet.

### Added
- **docs/design/**: nine IEEE 1016 Software Design Descriptions, DRAFT v0.1, approver pending, one per software block: `chess-rules`, `chess-domain`, `chess-store`, `chess-engine`, `chess-app`, `chess-server`, `chess-cli`, `ui`, `desktop-tauri`. Each traces its design entities to requirements, functions and decisions, and lists upstream findings.
- **docs/design/SDD-consistency-review.md**: cross-SDD review. 30 mismatches (15 Major, 15 Minor), consolidated edge, dependency and ownership tables, and a de-duplicated register of 52 upstream findings. It made 14 mechanical edits to seven SDDs.
- **docs/design/SDD-architect-triage.md**: system-architect triage of every mismatch and finding, 12 cross-cutting rulings, 10 draft ADRs (D-38 to D-47), 14 proposed architecture amendments, and 13 decisions recommended for the user. These are recommendations only. None is approved, and architecture.md is not yet amended.

### Changed: requirements and architecture (Amendments 5 and 6, CR-7 and CR-8)
- User decisions on the readiness findings: whole-integer Elo ratings (ELO-1), PGN rules checked against the standard, latest stable versions of all components (architecture §3.4), CERT informs best practice for Rust and TypeScript, best-effort verification on unavailable platforms, and no security requirement beyond best practice and avoiding known CVEs.
- ECO opening table is the Lichess `chess-openings` snapshot; games store the ECO code and opening name (DAT-3).
- Engagement profile, readiness check and test strategy updated to match.

### Open at this release
- The 13 decisions in the triage await the user, and 26 questions await the systems engineer. Several affect stored formats (journal kinds, nullable fields, `content_hash`) and must be settled before any store code.
- Dependency versions listed in the SDDs are not looked up, and library behaviour marked unverified has not been checked.
- No SDD has had a reviewer pass or desk review.

## Design baseline v1.0, Amendment 4: 2026-10-28

Documentation-only release. No application code exists yet.

### Added
- **docs/engineering/SWE-001-engagement-profile.md**: criticality, coding standard, design approvers, installed toolchain and assumptions.
- **docs/SRR-SFR-readiness.md**: the readiness check for detailed design. Run 1: NOT-READY (one blocker, no criticality or coding standard). Run 2, after the user's answers: **READY-WITH-CONDITIONS** for step 1 (`chess-rules`, `chess-domain`), with 12 findings open (7 Major, 5 Minor) and owners and closing steps for each. The use cases and functional decomposition baselined as v1.0 earlier the same day are the SFR inputs (entry below).

### Changed: user decisions (requirements Amendment 4, CR-6)
- The software has **ordinary criticality** (not safety-related, not regulated). **SEI CERT** is the coding standard, with the standard Rust and TypeScript checks and every warning treated as an error. No requirement text changed.
- Design changes inside one component are approved by the system-architect agent (desk review); anything wider by the user.
- The systems engineer's earlier interpretations are confirmed (PLY-4, RULE-2, CLI-4, IF-2, PLY-7, RULE-1, UI-7), and architecture Q-A18 is answered.
- No Intel Mac is available for testing. PLAT-2 is unchanged and its Intel verification can't be run (open, FIND-015).

### Open at this release
- As before, plus the open readiness findings (FIND-004 to FIND-008, FIND-010 to FIND-016), in particular how CERT applies to Rust and TypeScript (FIND-014) and the unverifiable Intel requirement (FIND-015).

## Design baseline v1.0: use cases and functional decomposition, 2026-10-28

Documentation-only release. No application code exists yet.

### Changed
- **docs/use_cases/** (README and UC-1 to UC-29) and **docs/functional_decomp.md** are baselined as **v1.0**, approved by the user. Content is unchanged from v0.3; only status, approver, version references and change logs changed. Further changes go through change control.

## Design baseline v1.0, Amendments 2 and 3: 2026-10-28

Documentation-only release. No application code exists yet. Under the user's delegation (no questions), the systems engineer and the architect closed every gap in the use-case set and the functional decomposition, then ran a consistency pass over all documents. The decisions are the systems engineer's and the architect's judgement and are open to the user's correction (requirements Decision Log, CR-4 and CR-5).

### Added
- **docs/use_cases/** (README and UC-1 to UC-29, v0.3): Cockburn use cases with a use-case diagram, five activity diagrams, requirement and function traces, and a coverage table (82 of 82 requirement and risk IDs). UC-29 covers the build and release enabling system.
- **docs/functional_decomp.md** (v0.3): 124 functions, 104 leaves, allocated to the architecture's named blocks (§2.2), with a requirement-to-function matrix.
- **docs/handoff_se_to_architect.md**: the systems engineer's change list for the architect (closed; the disposition is in architecture §10.4).

### Changed: requirements (Amendment 2, CR-4; Amendment 3, CR-5)
- **New requirements:** PLAT-6 (install, update, uninstall; `chess install-cli`), DAT-7 (failed writes, unreadable journals), UI-12 (confirmation of destructive actions), NFR-10 (local diagnostic log).
- **Rules settled:** names are unique case-sensitively and counted in grapheme clusters; takeback stops at the human's first turn; draw offers stop all clocks; "move now" is `movenow`, F2 and a GUI button; the resume list covers either side of a hot-seat game; computer Players cannot be renamed or archived; scripted runs are computer-vs-computer, stored and rated, with `--data-dir`; `chess serve` fails fast on a bad config and stops cleanly on Ctrl-C; the full-data archive gains `games.json` with the analysis results; duplicate PGN games raise a skip or keep-both prompt.
- **Data damage:** a failed pre-upgrade backup stops the upgrade; every store-changing action is all-or-nothing; an unreadable journal resumes from its longest intact run, or the game is voided as "voided: unreadable"; a restore replays every retained journal. **Amendment 3** adds the automatic backup (at start-up when the newest is older than 24 h, newest 3 kept; TBD-33) and states the limit of a restore (a game of a Player created after the backup keeps its journal but is not recovered).
- **Register:** TBD-31 to TBD-33 and TBC-10 added; TBD-21 closed (exit codes 0 to 4).

### Changed: architecture (amendment 2 and 3)
- Decisions D-31 to D-37, risks R-19 and R-20, a build-and-release context view (§2.1), named allocation blocks (§2.2), and the handoff disposition (§10.4). Amendment 3 is text-only: it records the systems engineer's acceptance of the architect's pushback.

### Open at this release
- As for Amendment 1, plus: macOS facts behind `install-cli` and ad-hoc signing (TBC-10, ASM-5), checked at packaging time.

## Design baseline v1.0, Amendment 1: 2026-10-27

Documentation-only release. No application code exists yet.

### Changed: user decisions
- **Reset with an unfinished game is allowed (PLY-7, D-30).** The unfinished game is *voided*: it has no result and no effect on either side's Elo, rated-game count or W/L/D, and it frees the Player's unfinished-game slot. It stays in the opponent's history marked "voided by reset", can be analysed, and is hidden from the reset Player's own views. A game live in another front-end stops at its next lease heartbeat (about 2 s, an estimate). The reset confirmation names the games it will void. Exported PGN uses `Result "*"`, `Termination "abandoned"` and the custom tag `EndStatus`. D-26 is revised, G-20 and OQ-8 are closed, and the data model, state machine, protocol, error table and journal gain a `Voided` state. The interpretation of how a voided game is shown is recorded for the user to correct (Q-A18).
- **Stockfish 19 is the pinned engine (D-9, IF-4, TBD-26):** tag `sf_19`, released 2026-09-05. The build prefers the official `stockfish-macos-universal` asset, accepted only if `lipo -info` shows both arm64 and x86_64, with a from-source fallback. The `UCI_Elo` floor is no longer hard-coded: the range is read from the engine at run time. Any later Stockfish release needs a decision-log entry and a calibration re-run.

### Changed: source checks closed against the primary texts
- **FIDE Laws of Chess (TBC-2):** the 2023 edition (in force from 2023-01-01) is current. Threefold repetition (Art. 9.2) and the fifty-move rule (Art. 9.3) are claims; fivefold repetition and the 75-move rule (Art. 9.6) are automatic. The app's automatic draw at threefold and fifty-move is stated as a deliberate deviation. Dead positions other than insufficient material are not detected (limitation against Art. 5.2.2). A re-check for a new edition is now a release-checklist item.
- **PGN standard (TBC-3):** revised 1994.03.12; section 4.1 specifies ISO 8859/1, so the UTF-8 export is a stated deviation. Export and import formats are named as the standard names them. The Seven Tag Roster text and the `Termination` value `abandoned` are still to be checked (new TBC-6).
- **WCAG 2.2 (TBC-4):** W3C Recommendation of 2024-12-12. SC 1.4.3 and 1.4.11 values (4.5:1 and 3:1) and the ratio formula are confirmed. UI-7 keeps 4.5:1 for all text, which is stricter than the standard.
- **Nielsen (TBC-5):** the 0.1 s limit is from "Response Times: The 3 Important Limits" (Chapter 5 of *Usability Engineering*, 1993), which cites Miller (1968) and Card et al. (1991). NFR-8's citation is corrected.

### Open at this release
- Could not be checked, because downloading and running the binary was blocked: the macOS Stockfish asset's architectures and the `UCI_Elo` range (TBC-7, TBC-8, ASM-3), and the GPL version (TBC-9). All close at the first engine build-script run.
- Mermaid diagrams are hand-checked only (no Node.js on the build machine).

## Design baseline v1.0: 2026-10-26

Documentation-only release. No application code exists yet. **Requirements and architecture are baselined.** Every requirement is AGREED at priority Must; every architecture decision is AGREED except D-21 (deferred).

### Changed: requirements (docs/requirements.md)
- **Reviewed to ISO/IEC/IEEE 29148 and INCOSE rules.** Fixes to agreed requirements were made as amendments with the old wording kept. No IDs were renumbered or deleted.
- **Added:** PLAT-4 (`chess serve`), PLAT-5 (one version for CLI and desktop), PLY-6 (names), PLY-7 (reset), ENG-4 (engine failure), ENG-5 (cancel), DAT-4 to DAT-6 (one front-end per live game, store integrity, backup before upgrade), GAME-4 (unfinished games), UI-6 to UI-11 (screen-reader announcements, contrast, focus, offline help, themes, English only), ANA-4, CLI-5, NFR-7 to NFR-9, RULE-6 to RULE-8 (PGN export, PGN import, FEN start positions, split from RULE-5), and IF-1 to IF-5 (PGN, FEN, bundled Stockfish, export archive).
- **User decisions (Q41 to Q79) applied:**
  - **Ratings:** every Player, human or computer, has a Strict and a Training Elo, starting at the category baseline. Ratings change by the Elo formula, with K = 40 for the first 30 rated games, counted per scale. A computer's category label follows its Strict rating. No recalibration after release.
  - **Ending games:** a crash or closed tab suspends a game with no effect. Humans can only resign, which is a loss. Abort exists only for a computer-vs-computer watcher and has no effect on stats.
  - **Draws:** humans can offer draws in human-human and human-computer games. The computer never offers one.
  - **Unfinished games:** one per mode per human Player. Computers are limited only in computer-vs-computer games and are never blocked in human-vs-computer games.
  - **Players:** UUID identity; unique, case-sensitive names of up to 128 characters; a reset keeps Elo and hides history from the reset Player only.
  - **Platform:** macOS Tahoe and later; Intel and Apple silicon; Safari, Chrome, Firefox and Edge from the last 3 years; Terminal and iTerm2. v1 is fully offline, with network play a possible future enhancement.
  - **Interface:** no sound and no confirm-move setting in v1; themes follow the system; WCAG 2.2 AA contrast; English only; the TUI minimum is 80x24 with a resize request.
  - **Data:** no completed move is ever lost, and clocks restore to the last recorded value. Imports merge, asking on collisions. The full-data export is one archive of PGN and JSON.
  - **Server:** the port is set by CLI option or config file.
- **Register:** a TBD/TBR/TBC register with owners and close-by steps, and a Decision Log.

### Changed: architecture (docs/architecture.md)
- **New sections:** concurrency model (D-13); error catalogue of 16 codes; protocol reliability (reconnect, idempotency, version check); internal interface contracts; data ownership; crash-safe move journal (D-23); one version everywhere (D-16); deployment and packaging (D-18); accessibility details; per-requirement traceability with orphan checks.
- **Decisions:** D-12 to D-29 recorded. D-12 (SQLite) and D-16 are one-way for data, and the user accepted the consequences. ASM-1 (one person per Mac account, so no access token on the local server) is confirmed.
- **Fixes to the design:** the engine now replies only after an accepted move; post-game analysis goes to the analysis service; every engine move is checked for legality; timeouts and tolerances are set from the M4 / 16 GB baseline.
- **Added risks:** R-9 to R-18, including Tahoe being the last macOS release to support Intel.

### Changed: other documents
- **docs/test-strategy.md** rewritten to match: two Elo scales, resign and interruption rules, per-Player unfinished-game limit, merge import, export archive, themes, no sound, no-lost-moves fault injection, the macOS release checklist, and isolated data stores for every test run.
- **README.md** rewritten to describe the baselined design.

## Design baseline v0.4: 2026-10-25

Documentation-only release. No application code exists yet. **All architecture decisions are now agreed.**

### Added
- **docs/trade-study-gui-board.md** (first pass, kept for the record). Four GUI board options scored against a requirements gate and ten weighted criteria, using research from the published source of chessground 10.5.0 and react-chessboard 5.12.1:
  - A: own React + SVG board with dnd-kit (91.0);
  - B: chessground as is (56.4, fails the gate);
  - C: chessground + our keyboard/accessibility layer (68.2);
  - D: react-chessboard + that layer (72.4).

  After it, the user ruled out building our own board for v1.
- **docs/trade-study-gui-board-refined.md**: refined study of C vs. D, with twelve criteria focused on how well each library hosts our keyboard/accessibility layer.
  - New design ideas: a transparent ARIA-grid overlay aligned over the visual board, instead of a hidden copy; and controlled selection for chessground.
  - D split into D1 (patched) and D2 (run-time workaround).
  - Scores: C 88.8, D1 87.4, D2 78.2, with sensitivity analysis showing C and D1 effectively tied.
  - Recommendation: C, decided by tie-breakers (mature pointer interaction; risk that sits in our own code; no library modification), gated by a 2-day spike.

### Changed
**docs/architecture.md**
- **D-4 AGREED:** `@lichess-org/chessground` for visuals and mouse/touch, plus our shared layer: a `BoardModel` reducer owning the one selection state, a transparent `AccessibleOverlay` ARIA grid (keyboard focus, screen-reader labels, visible focus on the real squares), a live region, a promotion picker, and a Reconciler that keeps chessground's selection in step. Fallback: react-chessboard + the same layer, patched (D1).
- §11.1 rewritten as the decided board design, with the spike's exit criteria; §6.2 GUI section and diagram updated; repository layout updated for `ui/src/board/`.
- New risk R-8 (selection drift between chessground and our reducer) with mitigations.
- §13 next steps reordered: core crates, then the CLI, then the GUI starting with the board spike.
- Header status: all decisions D-1–D-11 AGREED. Decision-log entries added.

**docs/test-strategy.md**
- New focus area **GUI board**: selection-sync sequences across mouse, drag, touch, keyboard, and typed input; overlay alignment; axe on the `aria-hidden` board; rejected pointer moves revert. Header updated.

**docs/requirements.md**
- Header notes that all architecture decisions are agreed, including the board that delivers GUI-2 and UI-4. No requirement changed.

**README.md**
- Status, GUI board technology row, and documentation index (both trade studies) updated.

## Design baseline v0.3: 2026-10-25

Documentation-only release. No application code exists yet.

### Changed: architecture decisions recorded
**docs/architecture.md**
- Section 11 renamed "Decisions". D-1–D-3 and D-5–D-11 are AGREED:
  - D-1: the desktop app links the core in-process and talks to it over Tauri IPC.
  - D-2: each entry point embeds the core; they coordinate through the SQLite file.
  - D-3: chess rules wrap `shakmaty` + `pgn-reader`.
  - D-5: React + TypeScript + Vite for the GUI.
  - D-6: full-screen ratatui TUI (agreed in v0.2).
  - D-7: Rust → TypeScript types with `ts-rs`.
  - D-8: the web server is started by the user with `chess serve`.
  - D-9: Stockfish pinned, downloaded and checksummed at build time, bundled with the app.
  - D-10: project license GPL-3.0-or-later.
  - D-11: legal-move lookups for what-if branches go through the core.
- **D-4 (GUI board) remains open.** New §11.1 compares three options in detail: our own React + SVG board (recommended), chessground used directly, and chessground with our own keyboard/accessibility layer.
- New §11.2 decision log.
- New §8.1 **Offline operation**: every run-time dependency and how it is met locally, enforcement through Content-Security-Policy and a CI bundle scan, offline E2E tests, and the build-vs-run distinction.
- §6.1 records that the TUI is keyboard only: moves typed, no board cursor, mouse capture never enabled.
- Header rewritten with current status and a change summary.

### Changed: offline operation is an explicit v1 requirement
**docs/requirements.md**
- **PLAT-3** strengthened: the desktop app, web page, and CLI are fully usable with no Internet connection, including first launch. Nothing is fetched at run time (no CDN scripts, web fonts, update checks, license checks, or telemetry). Building from source may download dependencies; running never does. New verification: offline E2E for each entry point with non-loopback networking blocked, a network-capture check, a CI scan of the GUI bundle for remote resources, a Content-Security-Policy in both the local server and the desktop app, and a check that the bundled Stockfish runs with no network.
- **CLI-1/3/4**: TUI details (Q38), typed-only moves (Q39), and no mouse (Q40) marked AGREED.
- Header "Current state" updated to 2026-10-25 with a change summary. Decision Log entries added.

**docs/test-strategy.md**
- Title no longer says "technology-neutral"; it points to architecture.md §9 for tools.
- Header status updated to 2026-10-25 with a change summary.
- Non-functional checks add offline E2E for desktop, web, and CLI, plus the GUI bundle scan (PLAT-3).

**README.md**
- Status, offline feature description, technology decisions table, and documentation index updated.

## Design baseline v0.2: 2026-10-24

Documentation-only release. No application code exists yet.

### Added
- **README.md**: project overview, planned features, technology decisions, and a documentation index.
- **docs/architecture.md** (new, 773 lines, DRAFT): architectural principles, system context and deployment, code organisation, the shared `chess-app` core, the front-end/core protocol, the three front-ends, persistence, cross-cutting concerns, a testing architecture mapped onto the test strategy, component-level requirements traceability, open decisions, risks, and next steps.
  - Settled inputs: Rust backend, TypeScript GUI, Tauri desktop shell, one shared core for CLI/web/desktop.
  - Key principles: the Rust core owns every rule and mode-policy decision; pure logic is separated from I/O (fake engine, fake clock, in-memory store for tests); Stockfish is never exposed to a front-end and all engine requests pass one policy gateway; every state change is durable in SQLite (WAL) before it is acknowledged.
- **RELEASE_NOTES.md**: this file.

### Changed: CLI is now a full-screen terminal UI
The CLI was previously specified as a scrolling ASCII board. It is now a full-screen, ratatui-style TUI (Decision Log, 2026-10-24; Q38–Q40 resolved).

**docs/requirements.md**
- **CLI-1** rewritten as "Full-screen terminal UI". The board, players/status, clocks, SAN move list, and input line are shown in panels that update in place. New agreed details:
  - Screens: registration, new-game setup, play, computer–computer watch, post-game analysis and what-if, statistics, Computer Players, and settings, all keyboard-navigated.
  - Minimum terminal size 80×24, with an "enlarge the terminal" message below that and immediate reflow on resize.
  - Terminal always restored (cursor, echo, main screen) on quit, Ctrl-C, panic, or crash; Ctrl-Z suspends and `fg` redraws.
  - No mouse support in v1.
  - Plain mode (`--plain`): line-oriented transcript, chosen automatically when stdin/stdout is not a terminal; serves screen readers and piping. Scripted runs never start the TUI.
- **CLI-2**: coordinate labels apply to the board panel and plain mode.
- **CLI-3**: moves are typed only (no keyboard board cursor, Q39), so arrow keys stay free for rewind; invalid input shows an error in the status area and leaves the text editable.
- **CLI-4**: added key bindings that can never begin a move (←/→ back/forward, End live, PgUp/PgDn scroll moves, F1 help, Tab switch panel, Esc cancel). Letters, digits, `-`, and `=` always go to the input line. Commands disallowed by the play mode are shown as unavailable with an explanation. Added watch-screen keys for computer–computer games: Space pause/resume, `s` step, `+`/`-` delay, `a` abort (with confirmation).
- **UI-2**: statistics are a TUI screen with tables and an Elo-over-time chart; plain mode prints a text table and ASCII sparkline.
- **UI-4**: screen-reader support in the CLI is provided through plain mode, since full-screen TUIs are not reliably readable by screen readers.
- Header status, Open Questions, and Resolved (now Q1–Q40) updated. The architecture inputs are recorded as resolved. Two Decision Log entries were added.

**docs/test-strategy.md**
- Unit level adds board renderer and TUI screen snapshots at fixed terminal sizes.
- End-to-end level adds PTY-driven TUI sessions (screen parsed by a terminal emulator) and plain-mode transcripts.
- Notation and input: TUI key bindings must never swallow move characters.
- New focus area, **Terminal UI**: layout at minimum size and on resize; terminal restored after quit, Ctrl-C, and panic; plain mode selected when not a TTY; screen-reader use via plain mode.

### Status at this release
- All requirements (sections 1–11) and the out-of-scope list are AGREED; no requirements questions are open.
- The architecture is a DRAFT for iteration. Nothing is implemented.

## Initial commit: 2026-10-24

- Repository created and published to GitHub.
- Added `.gitignore`, `docs/requirements.md`, and `docs/test-strategy.md`.
