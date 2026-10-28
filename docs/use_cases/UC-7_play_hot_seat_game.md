# UC-7: Play Hot-Seat Game

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-7_play_hot_seat_game.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** Two people on one Mac (or one person playing both sides) want to play a complete Strict game against each other, so that the result rates both Players.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as two human Players who share one screen

**Supporting Actors:** None.

**Stakeholders and Interests:**
- Both human Players: legal play, a running clock that is fair to both, a rating change only when the game is rated.
- Other Players: not affected.

**Preconditions:** Two human Players are registered (or one Player will play both sides). Neither chosen Player has an unfinished Strict game, or it was resolved (UC-5 6a).

**Minimal Guarantees:** Every completed move is stored. An interruption leaves the game unfinished and resumable, with no rating or statistics change (RULE-5, GAME-4).

**Success Guarantees:** The game ended with a stored result and reason. Statistics updated for both Players. Both Players' Strict Elo updated, unless the game is self-play or from a custom start position (ELO-2). The Users have been offered full-game analysis.

**Trigger:** The User chooses to start a human–human game.

**Main Success Scenario:**

1. User sets up a human–human Strict game with two registered Players (includes UC-5).
2. System starts the game and shows the board, move list and clocks.
3. The Player on move makes a move.
4. System checks the move for legality, applies it, stores it durably, and passes the turn and the clock to the other side.
5. The Players repeat steps 3 and 4 until the System detects the end of the game.
6. System records the result and reason and finalises the game in one transaction.
7. System shows the result and offers full-game analysis (UC-15).

**Extensions:**

- **3a.** The move is illegal or stale:
  - 3a1. System rejects it with a reason; game unchanged (RULE-1, NFR-9). Use case resumes at step 3.
- **3b.** A Player offers a draw:
  - 3b1. Offer as UC-11: the offer is made on the offerer's own turn, all clocks stop while it is pending, and the other human accepts or declines (RULE-2). Accepted: use case continues at step 6. Declined: use case resumes at step 3.
- **3c.** A Player resigns:
  - 3c1. Resign as UC-10; the System asks which side resigns. Use case continues at step 6.
- **3d.** Either human asks to rewind:
  - 3d1. Rewind as UC-13. Use case resumes at step 3.
- **3e.** A flag falls:
  - 3e1. System ends the game as a loss for that side, or a draw if the opponent cannot checkmate (RULE-4). Use case continues at step 6.
- **6a.** The same Player filled both sides (self-play) or the game began from a custom position:
  - 6a1. System stores the game flagged, changes no rating, and excludes self-play and custom-start games from W/L/D and aggregates (ELO-2, UI-2). Use case resumes at step 7.
- **\*a.** An interruption occurs, or a reset voids the game:
  - \*a1. As UC-6 extensions *a and *b. Use case ends.

**Technology and Data Variations:**
- Training cannot be selected for human–human (PLY-5): no takeback and no analysis during play. Rewind is available (GAME-3).
- Both Players may be the same registered Player (self-play) (PLY-1, PLY-3).
- Front-end: CLI TUI, plain mode, browser, or desktop; one screen is shared (hot-seat).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- RULE-4 clocks; NFR-2 durability; UI-4/UI-6/UI-8 accessibility as in UC-6.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | UC-5 |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Extended by UC-10, UC-11, UC-13. Leads to UC-15. |
| Open issues | None. UG-6 and UG-7 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLY-1, PLY-3, PLY-5, GAME-1, GAME-3, GAME-4, RULE-1, RULE-2, RULE-4, RULE-5, DAT-3, ELO-1, ELO-2, UI-2, NFR-2, NFR-9 |
| Functions (functional_decomp v0.3) | F-2.2.3, F-2.2.4, F-2.3.1, F-2.4.1, F-2.4.2, F-2.4.3, F-2.5.2, F-2.5.3, F-2.5.6, F-2.6.1, F-2.6.5, F-2.6.6, F-4.1, F-4.2, F-6.2, F-7.2.2 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: draw-offer and resume rules decided (RULE-2, GAME-1; UG-6, UG-7 closed); custom-start games out of W/L/D. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
