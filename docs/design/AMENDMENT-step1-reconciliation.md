# AMENDMENT: SDD reconciliation after the step-1 code (rules, domain, store, app, architecture)

| Field | Value |
|---|---|
| Document | docs/design/AMENDMENT-step1-reconciliation.md |
| Date | 2026-11-09 |
| Tracker | chess-fwm (C-R2, `SDD-chess-rules`), chess-r5z (C-D1, `SDD-chess-domain`); baseline B4. B3 and B5 wording as in the work order |
| Author and stance | system-architect agent, Architect (author). This is not a review and not an approval |
| Gate | G1 text work (one crate each; the amendments record decisions already made). The decisions themselves were made by Brian Prascher on 2026-11-09 (G-3 passed with conditions; tracker chess-b9j, chess-9mm, chess-7m5, chess-4yc, chess-ebi, chess-0yz). Nothing here is a new G2 or G3 decision |
| Evidence tags | `[OBS]` read, `[RAN]` executed by me, `[INF]` inferred, `[UNV]` not verified |
| Status | (Section 8 added 2026-11-09, task chess-bmx: follow-up decisions; see 8.) Text applied to the five documents in section 1. **Not reviewed.** The Reviewer re-check of `SDD-chess-rules` 0.4 and of the domain 1.0.1 text is PENDING. Baselining `SDD-chess-rules` at v1.0 is Brian Prascher's decision and is PENDING; the label stays 0.4 DRAFT |

## 1 Index of every change

No code, `requirements.md`, `RELEASE_NOTES.md`, `CLAUDE.md`, bead or git state was touched. No Mermaid block was added; two blocks of the rules SDD and none elsewhere were edited.

### 1.1 `docs/design/SDD-chess-rules.md` (0.3.5 DRAFT to 0.4 DRAFT)

| Section | What |
|---|---|
| Header, Approver, Status, Upstream baselines | Version 0.4 DRAFT, date 2026-11-09. Approver cell says what is true: reconciled to code, G1 desk review of the code done, text awaits a Reviewer re-check, baselining pending, no approval claimed. Status says the code exists |
| Change log | Row 0.4 DRAFT |
| 1.2, 1.3, 2.2 | `pgn-reader` removed from scope, decisions row and constraint row; D-90, D-92, D-93, D-95 cited |
| 3.1 (Mermaid) | `pgn-reader` node and edge removed from the context diagram |
| 3.2 | `pgn` row (reader file), `eco` plus `key_derive` row, `testing` row |
| 3.3 (Mermaid) | `PgnGame.warnings` and `ImportWarning` added to the class diagram |
| 3.4 | `pgn-reader` row struck and explained; as-built `cargo tree` / edge-check facts |
| 3.6 | Adapter row without `pgn-reader` |
| 3.7 | IFP-RUL-3 (`explain_illegal` signature, `NotAPromotion`), IFP-RUL-5 (`PgnReader::new`, limits and ceiling, `warnings`, `format_line`/`parse_line` result types, `canonical_form` marker, `CANONICAL_FORM_VERSION` row), IFR-RUL-2, upstream-names paragraph |
| 3.8 | Adapter boundary as built (files that name `shakmaty`) |
| 3.9.2, 3.10 | Note that the tokenizer is in-crate; ceiling 256 and recovery refinements in the state table |
| 3.11.1 | `ParseMode::Pgn` accepted and rejected spellings (A-R4, R8); `Bb4` note (OQ-RUL-007) |
| 3.11.2 | `NotAPromotion` rows; `Other` meaning; stale code-comment note |
| 3.11.4 | `PositionKey` size note |
| 3.11.6 | FEN error shapes, dead upstream arms collapsed, Chess960 back rank with castling `-` accepted, counters saturate |
| 3.11.8 | Rewritten: in-crate tokenizer, reader rules 1 to 8 (tags, boundaries, stray result, Ctrl-Z, ceiling 256, `e.p.`, comments and marks, errors), encoding (whole-game Latin-1, OQ-RUL-006), warnings table, imprecisions, stale code-comment note (A-R4, A-R5, B6) |
| 3.11.9 | ECO build as built: files, data commit, 3865 rows, 0 duplicates, CI checksum limits |
| 3.11.10 | Step 2 as-built note; new steps 6 (illegal-tail marker) and 7 (control-character collapse); Version rule rewritten (`CANONICAL_FORM_VERSION = 1`, `v1` frozen); three golden games with exact bytes; corpus digest (A-R3, B1) |
| 3.12 | Memory amplification, stack and ceiling, depth rows, static data, line length |
| 3.13 (new) | As-built type and error shapes (A-R1) |
| 4 (all `DD-RUL-nnn`) | `Code:` paths made real; notes in DD-RUL-009, 011, 013, 014, 015 (test-support placement), 016 |
| 5 | Error mapping notes; new FM-RUL-009, FM-RUL-010 |
| 7 | As-built results paragraph; UT-RUL-024, UT-RUL-026 extended; new UT-RUL-032 to UT-RUL-034; UT-RUL-031 (4) and (5) rewritten (single source `crates/chess-rules/tests/data/fen_drift_guard.txt`, XC-DOM-A8-5 resolved: en passant row is `parse_error`); CT-RUL-001, CI-RUL-001; "not run" paragraph |
| 8 | Traceability rows extended |
| 9 | DR-RUL-001 reworded; new DR-RUL-013 to DR-RUL-017 |
| 10, 11 | New FIND-RUL-009 to FIND-RUL-011; ASM-RUL-001 and ASM-RUL-004 updated; new ASM-RUL-007 to 009; OQ-RUL-001, OQ-RUL-002, OQ-RUL-005 answered; new OQ-RUL-006 to 008; RSK-RUL-001 updated; new RSK-RUL-004, RSK-RUL-005; not-analyzed list |
| 12 | Approval record updated (no approval claimed); Mermaid validation row |

### 1.2 `docs/design/SDD-chess-domain.md` (1.0 BASELINED to 1.0.1 BASELINED, amendment)

| Section | What |
|---|---|
| Header, Approver, Status, change log | Version 1.0.1; status stays BASELINED; Approver sentence states the amendment is unapproved text recording decisions already made; Decision Log reference **CR-16 (was a placeholder)**; change-log row 1.0.1 |
| 3.10 | New transition T4a (`EngineFailed` in `DrawOffered` when the offer is to a computer) (A-D3 (f)) |
| 3.11.1 | "Field by field" replaced by "every journaled field" with `journal_eq` and what it skips (A-D1); finals clocks not journaled (B5); late `Clock` ignored, `Ended` trusted (A-D2); `Pause` from `Paused`; ply base (B3); transitions without a fact; no pending termination before the first move; `Void` carries the reset Player; SAN-policy sentence for XC-DOM-A8-5 (A-D7) |
| 3.11.5 | Notes (a) to (f) and precedence, with the Systems Engineer's draft cited and not adopted (A-D3) |
| 3.11.6 | Same `PlayerId` on both seats refused (A-D6); `clock_invalid` note; XC-DOM-A8-5 resolved, en passant row `parse_error` (A-D7) |
| 3.11.8, FIND-DOM-004 | `ROSTER_NAMESPACE` accepted (B2); as-built public names (N-05) |
| 3.11.9, UT-DOM-070 | Transposition wording corrected (A-D4, F-D9) |
| 3.11.10, DD-DOM-016, section 5 | `TreeError::TooDeep` and the depth definition (A-D5); arena growth limit |
| IFP-DOM-2 | `journal_eq` row; `Void{reason, reset_player}` |
| Section 7 | UT-DOM-020, 025, 115 reworded; new UT-DOM-125 to UT-DOM-129; traceability rows |
| 10.1, 11 | XC-DOM-A8-5 resolved; ASM-DOM-009; RSK-DOM-006; OQ-DOM-010 to OQ-DOM-015; not-analyzed bullet |

Not changed: the `Code: (pending)` lines of section 4 (stale, see 3).

### 1.3 `docs/design/SDD-chess-store.md` and `docs/design/SDD-chess-app.md` (baselined; minimal)

| File | Section | What |
|---|---|---|
| Store | 3.11.3 | Paragraph "Ply base (amendment 1.0.1, B3, D-92)": journal `Move.ply` is the domain value; the applier writes `MOVE.ply = journal ply + 1`; the fold `J` and `Takeback.to_ply` read accordingly; a test case to add |
| Store | change log | Row 1.0.1 (text note only, no DDL change, no approval claimed) |
| App | DD-APP-18 JournalCodec | Bullet "Ply base": the codec converts nothing |
| App | change log | Row 1.0.1; two related reading notes for the app owner (desk review D9, D15), not decisions |

### 1.4 `docs/architecture.md` (Addendum H, 11.4.16)

| Section | What |
|---|---|
| Header list | Addendum H bullet |
| 3.2 | Note after the component diagram: the `pgn-reader` box is history (diagram not edited) |
| 3.3, 3.4, 5.7 | Notes inside the `chess-rules` row, the `shakmaty` / `pgn-reader` row, the PGN/FEN I/O row |
| 7.6 | Bullet "Ply base" (D-92) and finals clocks (D-94) |
| 11 | Decisions **D-90** (canonical form v1), **D-91** (`ROSTER_NAMESPACE`), **D-92** (ply base), **D-93** (in-crate PGN tokenizer), **D-94** (finals clocks), **D-95** (import warnings). The highest earlier decision was D-89 `[OBS grep]`; the numbers continue it. Each marked "Decided by Brian Prascher 2026-11-09 (G-3 passed with conditions, tracker chess-b9j, chess-9mm, chess-7m5, chess-4yc, chess-ebi, chess-0yz)" |
| 11.2 | Log row 2026-11-09 |
| 11.3 | GATE-013 (R1 by score 7; the author's own score) and six door rows |
| 11.4.1, 11.4.16 | One ruling row; the Addendum H section (B-to-D mapping, stale mentions, escalation check, what is not decided) |
| 14 | OQ-29 (per-payload Latin-1) |

Mapping of Brian's items to D-numbers (B1 to B6 as named in `desk-review-G1-step1.md` section 7 and the work order): B1 = D-90, B2 = D-91, B3 = D-92, B4 = D-93, B5 = D-94, B6 = D-95. **The numbering is the architect's.** The tracker identifiers are cited as one group; I did not assert which bead belongs to which decision.

## 2 FOR BRIAN

*Section 8.4 (2026-11-09 follow-up, task chess-bmx) supersedes this list: items 1, 2, 4, 5, 6, 7, 8 and 9 are decided, item 3 is done (rules SDD 1.0). The text below is history.*

1. **OQ-29 / OQ-RUL-006 (chess-9o7), not decided:** decode non-UTF-8 PGN per payload or per whole game. It changes the `White` string, hence the `canonical_form` and `content_hash`, of an import from a mixed file. The cheapest moment is before the first stored hash. Class G3. The reader warns and decodes the whole game as before.
2. **After the B1 freeze, what does a byte change before the first release mean?** D-68 (d) and ASM-6 say a change before release stays `v1`. The freeze with golden bytes suggests any change is a conscious decision. The SDD text says only that a change to the pinned bytes needs your decision and is `v2` after release; please confirm or correct (G3, text).
3. **Baselining `SDD-chess-rules` at v1.0** is yours, after a Reviewer re-check of 0.4. I did not set it.
4. **Policy readings not confirmed by the Systems Engineer** (their draft `docs/engineering/se-confirmation-policy-readings.md`, SE-CONF-POLICY-1): OQ-DOM-014 (`SelectClock` allowed in an in-progress Strict game; the SE does not confirm it and recommends a setup-time check only; G1 inside the crate, G2 if architecture 4.4 or the protocol changes) and OQ-DOM-015 (Training `WhatIf` allowed while a draw offer to the computer is pending). I wrote the as-built behaviour and the open questions and adopted none of the SE's recommendations.
5. **Please confirm the six tracker identifiers** may be cited as one group on all six decisions.
6. **GATE-013** (R1, score 7) is my own score and is not approved. R1 asks for an ADR for a one-way door; I followed the repository's practice (D rows plus the door table, as for D-68) and wrote no ADR file for D-90 and D-91. Say if you want ADR files.

7. **SE R-1 (`SelectClock`)**: take `SelectClock` out of the in-game policy and keep it a setup-time check only? The SE does not confirm the as-built reading (OQ-DOM-014). G1 inside `chess-domain` and its text; G2 if architecture 4.4 or the protocol changes. Not adopted.
8. **SE R-2 (Training `WhatIf` in `DrawOffered`)**: restrict it to `AwaitingMove` with a human to move, as for `LiveAnalysis`? A requirement-intent call on ANA-1/ANA-2 (OQ-DOM-015). Not adopted.
9. **PD-2 (response-specific denial reason)**: add a new reason for a wrong draw answerer, or keep `state_not_allowed`? The SE recommends keeping it; a new reason touches the protocol (G2) (OQ-DOM-011). Not adopted.

Nothing was found that needs a new G2 or G3 decision beyond the above; I stopped at none.

## 3 Left out, and why

| Item | Why |
|---|---|
| `docs/requirements.md` Decision Log, `RELEASE_NOTES.md` | Not mine; drafts are in sections 4 and 5 |
| `CLAUDE.md` ("None of their Code: paths is real yet"; rules SDD "0.3.5 DRAFT") | Forbidden to edit; stale after this amendment (desk review N-06). Owner: systems engineer or program manager at the baseline |
| `Code:` lines of `SDD-chess-domain` section 4 | The work order asked for A-D1 to A-D7 and the conditions wording; the paths are a separate edit of a baselined document. Marked in the status cell |
| Per-payload Latin-1 | Not decided (FOR BRIAN 1) |
| Adoption of SE-CONF-POLICY-1 R-1 to R-3 | Not decided; would change the policy (F-1) or architecture 4.4 |
| New reason code for a wrong draw answerer (PD-2) | Touches the protocol; G2; asked as OQ-DOM-011 |
| `SDD-chess-store` DDL, `SDD-chess-app` protocol text | B3 is a text note; no DDL, record or protocol change was needed |
| Other baselined SDDs (engine, cli, server, ui, desktop-tauri) | No finding of the desk review required text there; `[UNV]` for UI "final clock" displays (ASM-DOM-009) |
| Code defects found while reconciling | *Fixed since by the software engineer (re-check RC-08, RC-09; SDD notes removed in the re-check addendum).* Reported at the time, not fixed by me: (a) the doc comment of `ImportWarning` (`crates/chess-rules/src/pgn/mod.rs:17-20`) says between-game comments are not reported, while the code and `tests/warnings.rs` report them on the next game; (b) the doc comment of `Position::explain_illegal` (`crates/chess-rules/src/position.rs:255-257`) still says `CannotReach` for a promotion piece on a non-promoting move, the code returns `NotAPromotion`. Both are G0 comment fixes for the software engineer |

## 4 Proposed Decision Log entry (for the systems engineer to enter in `docs/requirements.md`; not entered by me)

Number: **CR-16** (confirmed free and entered in `docs/requirements.md` 2026-11-09 by the systems engineer; the entry as entered differs in wording from this draft).

> | 2026-11-09 | **CR-16 (Amendment 8, extension 6; Brian Prascher's decisions of 2026-11-09, G-3 passed with conditions; tracker chess-b9j, chess-9mm, chess-7m5, chess-4yc, chess-ebi, chess-0yz; step-1 conditions of the G1 desk review `docs/engineering/desk-review-G1-step1.md`; G0 record keeping):** Decision Log entry CR-16 records six decisions made by Brian Prascher on 2026-11-09 and their text in the architecture and the SDDs. (1) **B1 (G3):** `canonical_form` `v1` is frozen: golden bytes (two standard-start games and one custom-start game), `CANONICAL_FORM_VERSION = 1`, a mainline move that is illegal in the replay appends a deterministic marker `{illegal <ply> <uci> ...}` before the result (it cannot occur in legal output and changes no byte of a legal game), and the collapse of control characters in tag values to a space is accepted and documented (architecture D-90). (2) **B2 (G3):** `ROSTER_NAMESPACE` = `aa98d4a0-fa10-585f-a6a6-268551d362c9` is accepted (D-91). (3) **B3 (G2/G3):** the journal stores the domain ply value as is (`Fact::Move.ply` = number of plies already played, zero-based) and the store applier writes `MOVE.ply` = journal ply + 1 (D-92). (4) **B4 (G1, flagged for G2):** the in-crate PGN tokenizer replaces the `pgn-reader` wrapper (D-3's escape route; `pgn-reader` is not a dependency) (D-93); `SDD-chess-rules` is to be baselined at v1.0 after the reconciliation amendment (now 0.4 DRAFT) and a Reviewer re-check, which is a separate decision of Brian Prascher's and is not made by this entry. (5) **B5 (G1):** the final clocks of a finished game are not journaled (no `Clock` fact before `Ended`), and `journal_eq` documents what it skips (D-94). (6) **B6 (G1):** the PGN reader reports what it drops or changes in a warnings list on each imported game (D-95). **Not decided:** whether non-UTF-8 PGN text is decoded per payload instead of per whole game (OQ-29, tracker chess-9o7). **No requirement statement, pass criterion, verification-case content or ID changed. Nothing was renumbered.** The Systems Engineer still owns the confirmation of the domain policy readings (domain SDD OQ-DOM-010 to OQ-DOM-015) and of the IF-2 import-fidelity intent (rules SDD OQ-RUL-007). The text is in architecture Addendum H (11.4.16), SDD-chess-rules 0.4 DRAFT, SDD-chess-domain 1.0.1 (amendment), and the ply note of SDD-chess-store and SDD-chess-app 1.0.1. |

Optional change-history row (same style as extensions 4 and 5): "**1.0, Amendment 8, extension 6** | 2026-11-09 | **CR-16** (Brian Prascher's decisions of 2026-11-09, trackers as above; G0 record keeping): Decision Log entry CR-16; no requirement text changed."

## 5 Proposed RELEASE_NOTES.md entry (for the systems engineer; not entered by me)

> ## Step-1 SDD reconciliation and Brian's decisions of 2026-11-09
>
> Documentation-only (G0 record keeping for the decisions; G1 text for the SDD amendments). Brian Prascher (G2/G3 approver) decided on 2026-11-09 at gate G-3 (passed with conditions); logged as **CR-16**. **No requirement statement, pass criterion, verification-case content or ID changed. Nothing was renumbered.**
>
> ### Decisions recorded (CR-16; architecture D-90 to D-95)
> - **`canonical_form` v1 frozen** (D-90): golden bytes including a custom start, `CANONICAL_FORM_VERSION = 1`, a marker for an illegal mainline tail, control-character collapse accepted.
> - **`ROSTER_NAMESPACE` accepted** (D-91).
> - **Journal ply base** (D-92): the journal stores the domain value; the store applier writes `MOVE.ply` = journal ply + 1.
> - **In-crate PGN tokenizer** (D-93): `pgn-reader` is not a dependency. `SDD-chess-rules` is reconciled to the code as **0.4 DRAFT**; it is **not baselined**. Baselining at v1.0 follows a Reviewer re-check and is Brian Prascher's decision.
> - **Finals clocks of a finished game are not journaled** (D-94).
> - **Import warnings list** (D-95). Per-payload Latin-1 decoding is **not decided** (OQ-29, tracker chess-9o7).
>
> ### Documents changed
> - `docs/architecture.md` (Addendum H, 11.4.16; D-90 to D-95, GATE-013, OQ-29; stale `pgn-reader` mentions corrected by note).
> - `docs/design/SDD-chess-rules.md` (0.3.5 DRAFT to 0.4 DRAFT).
> - `docs/design/SDD-chess-domain.md` (1.0 BASELINED to 1.0.1 amendment; status stays BASELINED; unreviewed).
> - `docs/design/SDD-chess-store.md`, `docs/design/SDD-chess-app.md` (1.0.1 notes on the ply base only).
> - `docs/design/AMENDMENT-step1-reconciliation.md` (index, FOR BRIAN list, these drafts).
> - `docs/requirements.md` (CR-16 Decision Log row) and `RELEASE_NOTES.md`: by the systems engineer.
>
> ### Open
> | Item | Owner |
> |---|---|
> | OQ-29 per-payload Latin-1 | Brian Prascher |
> | Reviewer re-check of SDD-chess-rules 0.4 and of the domain 1.0.1 text | Reviewer |
> | Baseline of SDD-chess-rules at v1.0 | Brian Prascher |
> | Domain policy readings: OQ-DOM-010 to OQ-DOM-015 (the Systems Engineer's draft does not confirm the `SelectClock` reading) | Systems Engineer, Brian |
> | `CLAUDE.md` statements "None of their Code: paths is real yet" and "rules SDD still 0.3.5 DRAFT" are stale | Systems engineer / program manager |
>
> ### Not done, not verified
> - Test, coverage and mutation figures in the amended SDDs are cited from the engineering evidence files; they were not re-run for this amendment. Intel Macs: not run.
> - The PGN Standard text was not re-read (accepted spellings, blank-line rule, `e.p.`): `[UNV]`.
> - No approval of the amended SDD text is claimed.

## 6 Verification of this amendment

| Check | Result |
|---|---|
| Mermaid, `validate-mermaid.mjs` | rules 7/7 parsed, domain 8/8, architecture 14/14 `[RAN 2026-11-09]` |
| Mermaid, `mmdc -p .mermaid/puppeteer.json` on the two edited rules blocks (3.1, 3.3) | both rendered to SVG without error `[RAN 2026-11-09]`; other blocks not rendered with `mmdc` |
| Highest earlier D-number | D-89 `[OBS grep of docs/architecture.md]` |
| Facts quoted from the code (golden bytes, marker text, enum variants, signatures, ceiling, `ROSTER_NAMESPACE`, file lists) | Read in the code and tests `[OBS]` |
| Test, coverage and mutation figures | Cited from the evidence files, not re-run `[UNV by this edit]` |
| Table column counts of the rows I added | Checked by script; no mismatch except one pre-existing row of the rules change log that lacks its closing pipe (0.3.5) |
| `git ls-files crates/chess-rules/build` | `eco_gen.rs` is tracked (desk review condition C-R1 is met) `[OBS 2026-11-09]` |

## 7 Re-check addendum (tracker chess-921, 2026-11-09)

Inputs: [review-step1-recheck.md](../engineering/review-step1-recheck.md) (Reviewer, PASS WITH CONDITIONS), the "Recheck fixes" sections of the rules and domain conditions evidence, and `se-confirmation-policy-readings.md`. G1 text only. Nothing approved.

| Item | Where | What |
|---|---|---|
| RC-14 / CND-4 | Rules SDD 3.11.2, 3.11.8, 3.11.10, 3.13, change log | Checked that the marker, `NotAPromotion`, `SyntaxReason`/`TagReason`, `ImportWarning`, `PgnGame.warnings`, `e.p.` and the RF-03 blank-line rule are carried as built (they were, from 0.4); added the RC-10 consequences (`[Event]` blank line `[White]` is now an error; a tag-only game at end of input is still an `Ok` empty game). Still pending: the Reviewer's re-check of the SDD text |
| RC-01 | Rules SDD 3.11.2, UT-RUL-035 | `NotAPromotion` also for a castling move with a promotion letter; the stale code-comment note removed (comment fixed, RC-08) |
| RC-03, RC-13 | Rules SDD 3.11.8, 3.12 (next to RF-10 and TBD-25), 3.13, UT-RUL-035 | `MAX_IMPORT_WARNINGS_PER_GAME` = 256 (`ESTIMATE`) and `ImportWarning::WarningsTruncated { dropped }`; the cap applies to all kinds, so `Latin1Fallback` can be dropped and only counted; one warning per typed tag, later tag replaces |
| RC-09 | Rules SDD 3.11.10 step 6, UT-RUL-032 | Invariant reworded: the movetext line of a legal game never contains `{`; tag values may; a value cannot forge the marker |
| CND-3 / RC-02 | Domain SDD 3.11.1, UT-DOM-125 | For a finished game `journal_eq` does not compare remaining times even if a journaled `Clock` fact precedes `Ended`; it compares the clock spec |
| RC-12 | Domain SDD 3.11.1, OQ-DOM-017 | Late `Clock` is a silent no-op; a warning is an open option for the architect |
| SE confirmation | Domain SDD 3.11.5 notes, OQ-DOM-011, 013, 016, 017 | Wording changes (a), (b3), (e), (f), F-3 "unfinished" versus `InProgress`, and the PD-1/2/4 recommendations recorded as notes and open questions; R-1 and R-2 not adopted |
| CND-5 / RC-05 | Domain SDD | The SE confirmation stays open for the items above; the independent table does not replace it (disclosed in UT-DOM-129) |

Files touched: `docs/design/SDD-chess-rules.md` (stays 0.4 DRAFT, new change-log row), `docs/design/SDD-chess-domain.md` (stays 1.0.1), this file. Mermaid `validate-mermaid.mjs`: rules 7/7, domain 8/8 `[RAN]`; no diagram was edited in this round. Test and coverage figures of the re-check are cited from the evidence files (rules coverage 97.70 %, domain 98.90 %), not re-run.


## 8 Follow-up of 2026-11-09 (task chess-bmx, baseline B4): per-payload Latin-1, pre-release `v1`, policy decisions

| Field | Value |
|---|---|
| Date | 2026-11-09 |
| Author and stance | system-architect agent, Architect (author). Not a review, not an approval |
| Gate | G1 text work (one crate each). The decisions were made by Brian Prascher on 2026-11-09, typed in the session and relayed by the coordinator (not a new G2 or G3 decision by me). Class of the decisions: G3 for D-96 and D-97 (stored-format effect), G1 for D-98 to D-100 |
| Inputs | [step1-rules-conditions-evidence.md](../engineering/step1-rules-conditions-evidence.md) ("Latin-1 per-payload"), [step1-domain-conditions-evidence.md](../engineering/step1-domain-conditions-evidence.md) ("R-1 / R-2 / PD-2"), [se-confirmation-policy-readings.md](../engineering/se-confirmation-policy-readings.md) |
| Status | Text applied to the four documents of 8.1. **Not reviewed.** The code (task chess-n3h for rules, chess-3ey for domain) has no system-architect desk review yet. No approval is claimed |

### 8.1 Index of every change

No code, `requirements.md`, `RELEASE_NOTES.md`, `CLAUDE.md`, `README.md`, bead or git state was touched. No Mermaid block was added or edited.

| File | What |
|---|---|
| `docs/architecture.md` (Addendum I, 11.4.17) | Header bullet. Dated notes under 4.4 (after the `allowed_actions` paragraph) and 4.5 (after the gateway paragraph); **no table row or enum text rewritten**. New decisions **D-96** (per-payload Latin-1; closes OQ-29), **D-97** (a pre-release byte change of `canonical_form` stays `v1`; D-68 (d), ASM-6), **D-98** (`SelectClock` out of the in-game policy; `check_setup` owns the clock rules), **D-99** (Training `WhatIf` live only in `AwaitingMove` with a human to move), **D-100** (`RespondDraw` by a wrong answerer keeps `state_not_allowed`). Notes in the D-95 row ("OQ-29 closed by D-96") and the GATE-013 line (no ADR files for D-90 and D-91, Brian's decision). 11.2 log row, **GATE-014** (R1, score 6, author's own), five door rows (D-96 one-way at release; D-97 to D-100 two-way), a 11.4.1 row, section 11.4.17, OQ-29 marked closed in section 14. The highest earlier D-number was D-95 `[OBS grep]` |
| `docs/design/SDD-chess-rules.md` (1.0 to **1.0.1 BASELINED**, amendment) | Header (Version, Date, Status, Approver sentence), change-log row 1.0.1, 3.11.8 *Encoding* rewritten (per payload), warnings table (`Latin1Fallback` once per fallback payload), cap paragraph and order line, 3.11.10 version rule (D-97), 3.13 `ImportWarning` row, DD-RUL-011 purpose, UT-RUL-020 and UT-RUL-033 wording, new **UT-RUL-036** (`tests/latin1_per_payload.rs`), FIND-RUL-010, OQ-RUL-006 closed, section 12 decisions row. Status stays BASELINED. Not changed: the section 12 "Reviewer pass" and "Baseline" rows |
| `docs/design/SDD-chess-domain.md` (1.0.1 to **1.0.2 BASELINED**, amendment) | Header (Version, Date, Status, Approver sentence), change-log row 1.0.2, `DeniedReason` note (IFP-DOM-1), 3.11.5 table rows (`LiveAnalysis`, `WhatIf`, `SelectClock`, `RespondDraw`, `MakeMove`, last row) and notes (c), (d) replaced, new (d2) and (d3), (e), (f), precedence (PD-4), the SE bullet replaced by "applied 1.0.2", F-3 terms; UT-DOM-022 note, new **UT-DOM-130**, traceability row; OQ-DOM-010 to OQ-DOM-016 closed or applied, new OQ-DOM-018 (OQ-DOM-017 stays open). Not changed: `Code:` lines of section 4 |
| `docs/design/AMENDMENT-step1-reconciliation.md` | This section 8; one pointer line at the top of section 2 |
| `SDD-chess-store`, `SDD-chess-app` and the other baselined SDDs | Not changed (see 8.5) |

Mapping of Brian's decisions to D-numbers: (1) Latin-1 = D-96; (2) pre-release byte change = D-97; (3) R-1 = D-98, R-2 = D-99, PD-2 = D-100; (4) no ADR files for D-90 and D-91: recorded in GATE-013 and GATE-014, no D-number; (5) the six decision beads cited as one group: done in Addendum I and below. **The numbering is the architect's.**

### 8.2 Proposed Decision Log entry (for the systems engineer; not entered by me)

**Proposal: extension 7 with its own entry, number CR-17** (the highest entry in `docs/requirements.md` is CR-16 `[OBS grep 2026-11-09]`; the systems engineer confirms that CR-17 is free). Reason: CR-16 is dated and logged as it was decided, and its "Not decided" sentence (OQ-29) is history that should not be rewritten; amendments add. The only edit to the CR-16 entry that I propose is optional and one line at its end: "*Follow-up 2026-11-09: see CR-17 (OQ-29 decided, pre-release `v1` rule, policy decisions).*" If the systems engineer prefers to amend CR-16 in place, the text below can be appended to it as "Follow-up of 2026-11-09", with the same content; I recommend against it, with Medium confidence, because it blurs which decisions were made at gate G-3 and which afterwards.

> | 2026-11-09 | **CR-17 (Amendment 8, extension 7; Brian Prascher's follow-up decisions of 2026-11-09, typed in the session and relayed by the coordinator; the six decision beads of CR-16 (chess-b9j, chess-9mm, chess-7m5, chess-4yc, chess-ebi, chess-0yz) are cited as one group by his decision; task chess-bmx; G0 record keeping):** Decision Log entry CR-17 records follow-up decisions of Brian Prascher and their text in the architecture and two SDDs. (1) **OQ-29 / OQ-RUL-006 closed, option B (G3):** non-UTF-8 PGN text is decoded per payload: each tag value and each comment is read as UTF-8 if valid, else that payload alone is read as Latin-1; `ImportWarning::Latin1Fallback` is recorded once per payload that fell back (architecture D-96). All-UTF-8 and all-Latin-1 files give the same bytes and hashes as before; only a file that mixes encodings reads differently. (2) **A byte change to `canonical_form` before the first release still counts as `v1` (G3):** D-68 (d) and ASM-6 stand; each such change still needs Brian's decision; after the first release it is `v2` (D-97). (3) **Mode policy (G1, inside `chess-domain`):** R-1, a clock may not be changed after a game has started: `SelectClock` is removed from the in-game actions and `allowed_actions` and answers `state_not_allowed`; `check_setup` is the single owner of the clock rules (D-98). R-2, Training `WhatIf` on a live game only in `AwaitingMove` with a human to move; finished games unconditional (D-99). PD-2, `RespondDraw` by a wrong side or actor keeps `state_not_allowed`, no new denial reason (D-100). (4) No ADR files are written for D-90 and D-91. **Not decided:** whether `DeniedReason::TrainingUntimed`, which `Policy::check` no longer produces, is retired from the enum (a protocol change, G2). **No requirement statement, pass criterion, verification-case content or ID changed. Nothing was renumbered.** The Systems Engineer has not recorded a confirmation of the domain wording items of SE-CONF-POLICY-1 (domain SDD OQ-DOM-018). The text is in architecture Addendum I (11.4.17), SDD-chess-rules 1.0.1 and SDD-chess-domain 1.0.2. |

Optional change-history row (same style as extension 6): "**1.0, Amendment 8, extension 7** | 2026-11-09 | **CR-17** (Brian Prascher's follow-up decisions of 2026-11-09; G0 record keeping): Decision Log entry CR-17; no requirement text changed."

### 8.3 Proposed RELEASE_NOTES.md entry (for the systems engineer; not entered by me)

> ## Follow-up decisions of 2026-11-09: per-payload Latin-1, pre-release `v1`, mode-policy changes (CR-17)
>
> Documentation for the decisions (G0 record keeping) and G1 text amendments of two SDDs; code changes by the software engineer are listed in the evidence files, not here. Brian Prascher decided on 2026-11-09, typed in the session; logged as **CR-17**. **No requirement statement, pass criterion, verification-case content or ID changed. Nothing was renumbered.** The six decision beads of CR-16 are cited as one group by his decision.
>
> ### Decisions recorded (architecture D-96 to D-100, Addendum I)
> - **Latin-1 per payload (D-96; closes OQ-29):** each PGN tag value and comment is decoded on its own (valid UTF-8 kept, else Latin-1). `ImportWarning::Latin1Fallback` is now once per fallback payload. Output for all-UTF-8 and all-Latin-1 files is unchanged (differential over 300 000 generated files, 0 differences in 200 227 pure files, `[RAN by the software engineer]`); only a file that mixes encodings reads differently.
> - **A byte change to `canonical_form` before the first release stays `v1` (D-97).**
> - **A clock cannot change after a game starts (D-98):** `SelectClock` is not an in-game action; `check_setup` owns the clock rules.
> - **Training what-if on a live game only on the human's turn in `AwaitingMove` (D-99);** finished games unconditional.
> - **`RespondDraw` by a wrong answerer keeps `state_not_allowed` (D-100).**
> - No ADR files for D-90 and D-91.
>
> ### Documents changed
> - `docs/architecture.md` (Addendum I, 11.4.17; D-96 to D-100, GATE-014; notes under 4.4 and 4.5; OQ-29 closed).
> - `docs/design/SDD-chess-rules.md` (1.0 to 1.0.1; status stays BASELINED; amendment text unreviewed).
> - `docs/design/SDD-chess-domain.md` (1.0.1 to 1.0.2; status stays BASELINED; amendment text unreviewed).
> - `docs/design/AMENDMENT-step1-reconciliation.md` (section 8).
> - `docs/requirements.md` (CR-17 row) and `RELEASE_NOTES.md`: by the systems engineer.
>
> ### Open
> | Item | Owner |
> |---|---|
> | Retire `DeniedReason::TrainingUntimed` from the enum (no longer produced by `Policy::check`; protocol change) | Brian Prascher (G2) |
> | Confirm that architecture 4.4 and 4.5 need no table-text change (architect's reading: none; dated notes only) | Brian Prascher |
> | System-architect desk review of the per-payload code (chess-rules) and the policy code (chess-domain) | System architect, then Brian for any G2 finding |
> | Systems Engineer confirmation of the applied domain wording (OQ-DOM-018) | Systems Engineer |
> | Independent Reviewer text review of the rules SDD: not requested; open note only | Brian Prascher decides whether to ask |
> | `CLAUDE.md` and `README.md` statements that depend on these changes, if any | Systems engineer / program manager |
>
> ### Not done, not verified
> - Test, coverage, mutation and differential figures are cited from the evidence files, not re-run for this entry. Intel Macs: not run.
> - No approval of the amended SDD text is claimed.

### 8.4 FOR BRIAN (only what is still open)

1. **Retire `DeniedReason::TrainingUntimed`?** `Policy::check` no longer produces it after D-98; `SetupReason::TrainingUntimed` is unchanged and is the only user-visible `training_untimed` now. Options: (a) keep the dead variant (no protocol change; a reader of the wire enum sees a reason that never occurs); (b) retire it (a protocol enum change, G2, cheap now because no consumer exists `[UNV: chess-app and chess-server do not exist yet]` and nothing is released). Recommendation: (b), at the time `chess-app` first generates the TypeScript types, so the enum is right before the first consumer; Medium confidence (a dead code is harmless, so the urgency is low). Not done by me.
2. **Reviewer text review of the rules SDD: not requested** (open note only, as instructed). The rules SDD text, including amendment 1.0.1, has had no independent Reviewer text review.
3. **Is Addendum I's reading of architecture 4.4 right (new)?** I read D-98 and D-99 as G1 inside `chess-domain` plus dated notes, because no row of the 4.4 table changes meaning (clock selection is a setup capability; "human only" analysis includes the narrower "`AwaitingMove`, human to move"), and no edge, command, reason code or stored format changes. Please confirm, or say that the 4.4 table text should be edited (that is a G2 change to a baselined table). I did not edit the table, the `EnginePurpose` comment, or the "in progress" wording of 4.4 and 4.5 (the term "unfinished" is proposed in the domain SDD only).

Pending but not for Brian: the system-architect desk review of the code of chess-n3h and chess-3ey (G1); the Systems Engineer's confirmation of the applied wording (OQ-DOM-018); the confirmation of CR-17 as the next free number (systems engineer).

Nothing else was found that needs a new G2 or G3 decision. I stopped at none.

### 8.5 Left out, and why

| Item | Why |
|---|---|
| `docs/requirements.md`, `RELEASE_NOTES.md`, `CLAUDE.md`, `README.md` | Not mine; drafts in 8.2 and 8.3. CLAUDE.md's statements about the Latin-1 question or the policy readings, if any, are the systems engineer's to check `[UNV]` |
| ADR files for D-90 and D-91 | Brian decided that none is written (recorded under GATE-013) |
| Retiring `DeniedReason::TrainingUntimed` | G2, protocol (FOR BRIAN 1) |
| Rewrite of the 4.4 and 4.5 table and prose ("in progress", the `EnginePurpose` comment) | A baselined table; notes only (FOR BRIAN 3) |
| `SDD-chess-store`, `SDD-chess-app`, `SDD-ui`, engine, cli, server, desktop-tauri | No text depends on `SelectClock`, per-game Latin-1 or the changed `WhatIf` reading `[OBS grep of docs/ for SelectClock, training_untimed, TrainingUntimed: hits only in engineering files, the domain SDD and one line of the app SDD (the degraded-session `allowed_actions` item, which lists `SelectClock` among the kinds left out: still true, now moot)]`. `[UNV]` for `SDD-ui` and `test-strategy.md`: a control or test that relies on what-if in `DrawOffered` was not searched for by phrase |
| `Code:` lines of `SDD-chess-domain` section 4 | Unchanged by this task (stale, see section 3) |
| Reviewer text review of the rules SDD | Not requested (FOR BRIAN 2) |
| `docs/engineering/*` evidence files | Owned by the software engineer; read only |
| `desk-review-G1-step1.md` A-D3 rulings (D3) | History; the 1.0.2 text supersedes reading (d) |

### 8.6 Verification of section 8

| Check | Result |
|---|---|
| Mermaid, `validate-mermaid.mjs` | architecture 14/14, rules 7/7, domain 8/8 parsed `[RAN 2026-11-09]`; no diagram edited |
| Highest earlier D-number and CR number | D-95 (grep of `architecture.md`), CR-16 (grep of `requirements.md`) `[OBS]` |
| Table row column counts of the rows I added | Checked by awk; one row (UT-DOM-130) had an extra pipe and was fixed |
| Facts quoted from the code (policy behaviour, warning shape, test names, figures) | Taken from the two evidence files `[OBS]`; **not checked against the code by me, and no test, coverage or mutation run was repeated** `[UNV by this edit]` |
| That the code matches the new SDD text | `[UNV]`: no code was read by me; the desk review of the code is PENDING |
| That the requirements and test-strategy do not contradict D-98 to D-100 | `[UNV]`: not re-read; SE-CONF-POLICY-1 cites GAME-1, RULE-4, ANA-1, ANA-4 as consistent |
