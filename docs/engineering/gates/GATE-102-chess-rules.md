# GATE-102: chess-rules (implementation, part A then part B)

| Field | Value |
|---|---|
| Gate | **GATE-102** (numbering from GATE-101 per SWE-001 ASM-S5; condition chess-ckq C-7, readiness-step2-3.md C-7) |
| Tier | **G1** (one crate, architecture 3.2 edges unchanged, 5.7 contracts unchanged) |
| Approver | System Architect, **desk review**, to be requested after the Reviewer pass (tasks chess-a0r.2.5 and chess-a0r.2.9) |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-05 |
| Opened by | Software engineer (task chess-a0r.2.3, part A) |
| Covers | Part A (chess-a0r.2.3): `types`, `error`, `position`, `fen`, `testing` (perft). Part B (chess-a0r.2.4: SAN, `parse_move`, PGN, `canonical_form`, ECO, `build.rs`) is added to this record when it lands |

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

## Approval record

| Item | Status |
|---|---|
| Reviewer pass | NOT DONE |
| System Architect desk review | NOT REQUESTED yet (after the Reviewer pass) |
| Approved by | nobody |
