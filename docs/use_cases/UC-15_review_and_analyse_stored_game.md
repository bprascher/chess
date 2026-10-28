# UC-15: Review and Analyse Stored Game

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-15_review_and_analyse_stored_game.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to step through a finished or imported game with engine evaluation and try alternatives, so that they can learn from it, without affecting any rating.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** Stockfish engine (block `stockfish`, bundled and pinned; driven over UCI)

**Stakeholders and Interests:**
- User: clear flags for blunders, mistakes and inaccuracies.
- All Players: analysis never changes a result, Elo or statistics (ANA-3, RULE-7).
- Reset Player: games from before the reset are hidden from that Player's views (PLY-7).

**Preconditions:** The game is stored and is completed, resigned, watcher-aborted, voided by a reset, or imported. An unfinished game is resumed (UC-9), not analysed (ANA-4).

**Minimal Guarantees:** Stored result, Elo and statistics are unchanged.

**Success Guarantees:** The User stepped through the game with evaluations and explored alternatives. An optional saved branch is stored as a PGN variation.

**Trigger:** The game has just ended and analysis is offered (UC-6, UC-7, UC-8), or the User opens it from the game history list, or opens a FEN position for analysis.

**Main Success Scenario:**

1. User selects a game, from the offer after it ended or from the game history list.
2. System opens the game at its start with engine evaluation for each position, best alternatives, and blunder, mistake and inaccuracy flags by centipawn loss.
3. User steps through the positions.
4. User plays hypothetical moves on a branch, steps back and forward, and branches again; the System updates the evaluation.
5. User leaves the branch and chooses to discard it or save it as a PGN variation.
6. User closes the analysis.

**Extensions:**

- **1a.** The User opens a position from a FEN instead of a game:
  - 1a1. System opens analysis from that position (RULE-8, IF-3), with FEN validation as in UC-5 extension 5a1a. Use case resumes at step 4.
- **1b.** The game is unfinished:
  - 1b1. System offers resume instead (UC-9) (ANA-4). Use case fails.
- **1c.** The game was voided by a reset and the User is the reset Player:
  - 1c1. The game is hidden from that Player's views (PLY-7). It can be opened from the opponent's history. Use case fails.
- **2a.** Background evaluations for some plies are not finished:
  - 2a1. System shows the evaluations stored so far while the background job continues at low priority and resumes after an interruption (DAT-3, F-5.5). A ply without an evaluation shows "evaluating…" and no flag, the view shows how many plies are evaluated out of the total, and values and flags appear as they complete (ANA-3). Use case resumes at step 3.
- **2b.** The engine fails:
  - 2b1. As UC-6 extension 5b (ENG-4). Use case ends.

**Technology and Data Variations:**
- Game kinds that can be opened: completed, resigned, watcher-aborted c–c, voided (marked "voided by reset"), imported (ANA-4).
- Flag thresholds from centipawn loss (EST): inaccuracy 50 to 99, mistake 100 to 299, blunder 300 or more (ANA-3).
- In Strict mode this is the only engine analysis of the game (ANA-3).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- NFR-8 visible feedback; NFR-9 errors; the evaluations are stored with the game (DAT-3).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Entered after UC-6, UC-7, UC-8, UC-10, UC-11, and from UC-16 and UC-23. Extended by none. |
| Open issues | None. UG-16 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | ANA-2, ANA-3, ANA-4, RULE-7, RULE-8, IF-3, DAT-3, PLY-7, ENG-4, NFR-8, UI-2 |
| Functions (functional_decomp v1.0) | F-5.2, F-5.3, F-5.4, F-5.5, F-5.6, F-3.2, F-3.7, F-2.1.3 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: display of evaluations that are not finished decided (ANA-3, UG-16 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
