# UC-14: Analyse Position During Training Game

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-14_analyse_position_during_training_game.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants engine help during a Training game: the best moves and what-if lines for the current or an earlier position.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** Stockfish engine (block `stockfish`, bundled and pinned; driven over UCI)

**Stakeholders and Interests:**
- User: sound evaluations without leaving the game.
- Project owner: no engine analysis exists during Strict games, so Strict ratings stay clean (PLY-5, ANA-1).
- Computer opponent: analysis never competes with its own search (ANA-1).

**Preconditions:** A Training game is in progress, and it is the human's turn.

**Minimal Guarantees:** The game, its clocks and its rating status are unchanged. Using analysis does not make a Training game unrated (ANA-1, ELO-2).

**Success Guarantees:** The User has seen the top N moves with evaluation, principal variation and depth, and left analysis. The game continues at the User's turn. A saved branch is stored as a PGN variation.

**Trigger:** The User enters `analyze` (CLI) or uses the analysis control (GUI).

**Main Success Scenario:**

1. User asks for analysis of the current position, or of an earlier position.
2. System asks Stockfish for the top N moves (N from 1 to 5, default 3) and shows the first result within 1 s (EST).
3. System shows each move's evaluation (centipawns or mate), principal variation and depth, and keeps searching until depth 20 or 10 s (EST), unless the User stops it.
4. User explores what-if lines: plays hypothetical moves for both sides, steps back and forward, and branches again, and the System updates the evaluation.
5. User leaves analysis and chooses to discard the branch or save it as a PGN variation.
6. System returns to the game at the User's turn.

**Extensions:**

- **1a.** The game is Strict, or it is the computer's turn:
  - 1a1. Analysis is not offered; the System explains why (PLY-5, ANA-1). Use case fails.
- **2a.** The engine fails:
  - 2a1. As UC-6 extension 5b (ENG-4). Use case ends.
- **3a.** User stops the search early:
  - 3a1. System shows the results so far. Use case resumes at step 4.

**Technology and Data Variations:**
- CLI: `analyze`; GUI: analysis overlay (architecture 6.1 Live analysis overlay). Rewind may select an earlier position (ANA-1, GAME-3).
- After the game, the same what-if tools are available in UC-15.

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- NFR-8: visible feedback within 100 ms while the engine searches.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | UC-6 |
| Extended by / included by | None |
| Open issues | None specific. (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | ANA-1, ANA-2, PLY-5, ELO-2, ENG-4, NFR-8, CLI-4 |
| Functions (functional_decomp v0.3) | F-5.1, F-5.2, F-5.3, F-3.2, F-3.3, F-3.7 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
