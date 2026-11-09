# GATE-102: chess-rules (implementation, part A then part B)

| Field | Value |
|---|---|
| Gate | **GATE-102** (numbering from GATE-101 per SWE-001 ASM-S5; condition chess-ckq C-7, readiness-step2-3.md C-7) |
| Tier | **G1** (one crate, architecture 3.2 edges unchanged, 5.7 contracts unchanged) |
| Approver | System Architect, **desk review**, to be requested after the Reviewer pass (tasks chess-a0r.2.5 and chess-a0r.2.9) |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-05 |
| Opened by | Software engineer (task chess-a0r.2.3, part A); part B added by the software engineer (task chess-a0r.2.4) |
| Covers | Part A (chess-a0r.2.3): `types`, `error`, `position`, `fen`, `testing` (perft). Part B (chess-a0r.2.4): `notation` (SAN, `parse_move`), `pgn` (model, writer, reader, line text, `canonical_form`, raw `ECO` tag), part B error types. Added below (section "CHG-102 part B") on 2026-11-06. The ECO table, `build.rs` and `eco::lookup` (DD-RUL-013, IFP-RUL-6) were added in a second pass of part B after the PM ruling; see the part B section |

## CHG-102 Implement chess-rules part A per SDD-chess-rules 0.3.5 DRAFT

- **Goal and driver:** RULE-1 (legal moves, castling, en passant, promotion, check, mate, stalemate), RULE-8 and IF-3 (FEN), NFR-3 (no illegal state), position inputs of RULE-2 and RULE-4; functions F-2.1.3 and F-2.2.1. Design entities DD-RUL-001, 002, 003, 004, 007, 008, 014 (partly: part A variants only), 015.
- **Components touched:** `crates/chess-rules` only (plus `Cargo.lock`, which records the dependency resolution). No other crate, no document, no CI file edited.
- **Interfaces touched:** adds the part A provided interface of IFP-RUL-1, 2 (except `san`), 4 and `PositionKey` of IFP-RUL-6 (key type only). Nothing exists yet that a consumer relies on, so no existing contract changes.
- **Dependencies added:** `shakmaty =0.30.2`, `thiserror =2.0.21` (runtime), `proptest =1.12.0` (dev). All three are in `docs/engineering/dependency-lookup-step1.md` (D-3, D-47). `pgn-reader` is not added (part B). `shakmaty` as a `[build-dependencies]` entry (D-75) is part B.
- **Gate answers:** Q1 no (one crate). Q2 no (first implementation of the SDD cards; no existing interface changes). Q3 no (the crate exists in 3.1). Q4 no (no workspace edge added or changed; `ci/check-crate-edges.mjs` reports 11 edges, all allowed). Q5 no. Q6 no. Q7 no (no ADR contradicted). Q8 no (ordinary criticality, SWE-001). Q9 yes (new observable behaviour of one existing component, architecture intact). Q10 no. **Result G1.**
- **Design summary:** the SDD design as written. `Position` wraps `shakmaty::Chess`; own `Square`, `Move` (UCI-shaped, castling is the king's two-square move), `Fen` (validated newtype with an own six-field syntax check, then upstream legality), exact `PositionKey`, `explain_illegal`. `shakmaty` is imported only by `position`, `fen` and the unit tests of `testing`.
- **Choices inside the SDD's freedom (the SDD signatures are sketches, SDD status row):** `IllegalMove` is a struct `{ reason }`; new `SquareError` for off-board input (DD-RUL-001 asks for a typed error); `FenError` is `TooLong | FieldCount | BadField{field, detail} | Illegal(IllegalPosition)`; `MoveParseError` has only `BadUci` in part A (part B adds variants); `Position::is_dead_by_material` uses the upstream material test after the SDD 3.11.3 table was run against it (ASM-RUL-001 validated). The Reviewer should look at these four.
- **Options considered:** not required below G2. One decision worth the Reviewer's eye: perft tests live in the crate's unit tests, because a self dev-dependency (needed for integration tests to enable feature `test-support`) is a `chess-rules -> chess-rules` edge that `ci/check-crate-edges.mjs` rejects. Alternative: edit the edge check or run CI with `--all-features` (a CI change, not made).
- **Evidence:** docs/engineering/step1-rules-A-evidence.md.
- **Tests added:** UT-RUL-001 to 007, 015, 016, 024, 025, 027 (part), FZ-RUL-002, UT-RUL-005; see the evidence file for the trace.
- **Risk and rollback:** working tree only, nothing committed. Rollback is deleting the new files and reverting `crates/chess-rules/Cargo.toml`, `src/lib.rs` and `Cargo.lock`.
- **Escalation check (G2 triggers):** none found. The SDD did not force an edge or §5.7 change. Two observations for owners are in the evidence file section "Findings" (en passant normalisation for chess-2bb, and the integration-test feature gap).
- **Reviewer findings and dispositions:** none yet. The Reviewer pass (chess-a0r.2.5) is a separate stance and has not happened.

## CHG-102 part B (added 2026-11-06, task chess-a0r.2.4, Engineer stance)

- **Goal and driver:** RULE-6 and RULE-7 (PGN export and import), IF-1, IF-2 (formats), CLI-3 and GUI-2 (c) (typed notation, the uppercase `B` rule), ANA-2 (line text), DAT-3 (raw `ECO` tag), D-42 (`canonical_form`), NFR-3 (no panic on any input), NFR-9 (typed errors); functions F-2.2.2, supports F-5.3, F-6.10, F-6.11. Design entities DD-RUL-005, 006, 009, 010, 011, 012, 016, and the part B variants of DD-RUL-014.
- **Components touched:** `crates/chess-rules` only: new `src/notation.rs`, `src/pgn/{mod,reader}.rs`; edits to `src/{lib,error,position}.rs`; new `tests/{notation,pgn}.rs`; helper additions in `tests/common/mod.rs`. `Cargo.toml` gains the `shakmaty` build-dependency (second pass); `Cargo.lock` is unchanged.
- **Interfaces touched:** adds the part B provided interface of IFP-RUL-1 (`Position::san`), IFP-RUL-3 (`parse_move`), IFP-RUL-5 (PGN services, `eco_tag`, `canonical_form`). Changes one part A type: `MoveParseError` gains variants and loses `Copy` (see F-B1 in the evidence file). No consumer exists yet.
- **Gate answers:** Q1 no. Q2 no (first implementation of the cards; the one part A type changed has no consumer). Q3 no. Q4 no (no edge; `ci/check-crate-edges.mjs` still reports 11 edges). Q5 no. Q6 no. Q7 no. Q8 no. Q9 yes. Q10 no. **Result G1, unchanged.** No G2 trigger was found.
- **Choices inside the SDD's freedom (Reviewer and SA desk review should confirm; listed as F-B1 to F-B9 in the evidence file):** error and type shapes (`MoveParseError` variants and payloads, `PgnErrorKind`, `PgnWriteError`, `San`, `PgnTags`, `PgnDate`, `TimeControl`, `Termination` text); an in-crate tokenizer instead of `pgn-reader` (the SDD's named escape route, OQ-RUL-001); reader leniencies (FEN tag honoured without `SetUp`, a tag section at a line start ends a game that has no terminator).
- **ECO table (second pass, PM ruling that DD-RUL-013 is in scope):** new `third_party/eco/` (Lichess `chess-openings` at commit `3ba6b20d31b2e6125cb0e6a2e792f400717d9d39`, CC0-1.0, retrieved 2026-11-06, SHA-256 in `SHA256SUMS`, source record in `SOURCE.md`), `build.rs`, `build/eco_gen.rs`, `src/key_derive.rs`, `src/eco.rs`, tests `eco` and `eco_build`, `ci/check-eco.mjs` (one new step in `ci/check.sh`). `[build-dependencies] shakmaty = "=0.30.2"` is added to `chess-rules/Cargo.toml` (D-75; same crate and version already admitted, no new crate in `Cargo.lock`, no workspace edge; Q3, Q4 stay no; `ci/check-crate-edges.mjs` still reports 11 edges). Checks that touched shared files: `deny.toml` and the edge script unchanged; `ci/check.sh` +1 step. A third-party data snapshot is now in the repository (D-20 planned this one-time fetch; NFR-6 licence audit applies). Not delivered: UT-RUL-031 row 5 (shared drift-guard table, left for chess-a0r.2.7).
- **Evidence:** docs/engineering/step1-rules-B-evidence.md (sections 1 to 10 first pass, section 11 ECO pass).
- **Risk and rollback:** working tree only, nothing committed. Rollback is deleting the new files and reverting `src/{lib,error,position}.rs` and `tests/common/mod.rs`.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance and has not happened for part B.

## Approval record

| Item | Status |
|---|---|
| Reviewer pass (part A and part B) | NOT DONE |
| System Architect desk review (part A and part B) | NOT REQUESTED yet (after the Reviewer pass) |
| Approved by | nobody |

## Rework note (chess-a0r.2.11, 2026-11-06)

Rework of the adversarial review `review-chess-rules-code.md` (PASS WITH CONDITIONS). Evidence: `step1-rules-rework-evidence.md`. Status of the gate is unchanged: G1, OPEN, **not approved**, nobody has approved anything, no System Architect desk review has happened.

- **Corrected footprint (RF-12).** The "Components touched: `crates/chess-rules` only ... no CI file edited" lines above understate the change. The real footprint of parts A and B plus the ECO pass is: `crates/chess-rules` (src, `build.rs`, `build/`, tests); **shared CI**: `ci/check.sh` (one new step) and the new `ci/check-eco.mjs`; **a third-party data set**: `third_party/eco/` (Lichess `chess-openings`, CC0-1.0, NFR-6 licence audit applies, one-time external fetch); **a build-script dependency** (`[build-dependencies] shakmaty = "=0.30.2"`, reads `../../third_party/eco` outside the crate directory); and an **in-crate PGN tokenizer** that replaces the `pgn-reader` wrapper named in SDD 3.11.8 and DD-RUL-011 (OQ-RUL-001 escape route; the SDD text is stale and needs an amendment by the SDD owner). Whether the CI edit needs Brian Prascher (G2) is the System Architect's call; this note does not decide it.
- **Reworked (RF-01, 02, 03, 06) and added (RF-05 part).** No public signature changed. One additive public item: `CANONICAL_FORM_VERSION: u32 = 1` (re-exported from the crate root; requested for D-68 / OQ-RUL-005 (a)). Behaviour changes inside existing signatures: `PgnLimits::max_variation_depth` is clamped to 256; a stray result word between games and a trailing Ctrl-Z no longer produce items; a blank line between two tag sections ends the first game with a `BadSyntax` error; `explain_illegal` returns `CannotReach` instead of `Other` for a promotion piece on a non-promoting move or a promotion to a king or pawn. Two existing assertions that pinned the `Other` behaviour (`tests/rules.rs`, `ut_rul_015_nfr_9_black_castling_reasons_and_other_cases`) were changed on purpose and say so in a comment.
- **Not changed, owner decision needed.** RF-05 (a) and (b): `canonical_form` is not injective for an illegal mainline tail or for control characters in tag values. The bytes are the stored-format `v1`; no byte was changed (Brian Prascher decides). RF-04, 07, 08, 09 and 10 to 15 were out of scope of this rework.
- **Reviewer re-check of the rework:** not done. SA desk review: not requested.

## Desk review result (chess-a0r.2.9, 2026-11-09, System Architect desk review, G1)

G1 desk review recorded in `docs/engineering/desk-review-G1-step1.md`: **PASS WITH CONDITIONS** (design conformance confirmed; conditions C-R1 to C-R6). Commit blocked until C-R1 (git-ignored `build/eco_gen.rs`). This is a desk review, **not** a Brian Prascher approval; G2/G3 items B1, B3, B4 are for Brian and are not decided.
