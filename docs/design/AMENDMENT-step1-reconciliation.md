# AMENDMENT: SDD reconciliation after the step-1 code (rules, domain, store, app, architecture)

| Field | Value |
|---|---|
| Document | docs/design/AMENDMENT-step1-reconciliation.md |
| Date | 2026-11-09 |
| Tracker | chess-fwm (C-R2, `SDD-chess-rules`), chess-r5z (C-D1, `SDD-chess-domain`); baseline B4. B3 and B5 wording as in the work order |
| Author and stance | system-architect agent, Architect (author). This is not a review and not an approval |
| Gate | G1 text work (one crate each; the amendments record decisions already made). The decisions themselves were made by Brian Prascher on 2026-11-09 (G-3 passed with conditions; tracker chess-b9j, chess-9mm, chess-7m5, chess-4yc, chess-ebi, chess-0yz). Nothing here is a new G2 or G3 decision |
| Evidence tags | `[OBS]` read, `[RAN]` executed by me, `[INF]` inferred, `[UNV]` not verified |
| Status | Text applied to the five documents in section 1. **Not reviewed.** The Reviewer re-check of `SDD-chess-rules` 0.4 and of the domain 1.0.1 text is PENDING. Baselining `SDD-chess-rules` at v1.0 is Brian Prascher's decision and is PENDING; the label stays 0.4 DRAFT |

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
