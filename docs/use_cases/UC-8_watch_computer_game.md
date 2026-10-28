# UC-8: Watch Computer Game

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-8_watch_computer_game.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to watch two computer Players play each other, at a pace they control, so that they can see how the computers play and rate them against each other.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a watcher (not a Player in the game)

**Supporting Actors:** Stockfish engine (block `stockfish`, bundled and pinned; driven over UCI)

**Stakeholders and Interests:**
- User: control of pace; no accidental destruction of the game.
- Both computer Players: ratings and statistics change only if the game is rated and finishes (ELO-2, GAME-2).
- Project owner: unattended play never crashes the app (NFR-3).

**Preconditions:** An active human Player exists (UC-2). Neither chosen computer Player has an unfinished computer–computer game.

**Minimal Guarantees:** Every completed move is stored. An interruption leaves the game unfinished and resumable. An abort changes no rating or statistic of either computer (GAME-2, GAME-4).

**Success Guarantees:** The game ended by a rule or a clock, with a stored result. Statistics updated. If the two computers are different, both Strict Elo ratings changed (ELO-2). The User has been offered analysis.

**Trigger:** The User explicitly chooses the computer–computer option.

**Main Success Scenario:**

1. User sets up a computer–computer game, choosing two computer Players (includes UC-5).
2. System starts the game and shows the board, move list and clocks, with no move input.
3. System asks Stockfish for each computer's move in turn, applies it, stores it, and shows it after the move delay (default 1 s).
4. User scrolls the move list while the game runs.
5. System repeats steps 3 and 4 until it detects the end of the game.
6. System records the result and reason and finalises the game (statistics; Strict Elo for both computers when they differ).
7. System offers full-game analysis (UC-15).

**Extensions:**

- **3a.** User pauses:
  - 3a1. System freezes the moves and both clocks (GAME-2).
  - 3a2. User resumes, or steps exactly one move, or changes the delay (0 to 10 s in 0.5 s steps). Use case resumes at step 3.
- **3b.** User aborts:
  - 3b1. System asks for confirmation (CLI-4: key `a`).
  - 3b2. On confirmation the game ends with no result and no rating or statistics change for either computer, is kept in game history marked aborted, and can be analysed (GAME-2, ANA-4). Use case ends.
  - 3b3. If the User cancels the abort, use case resumes at step 3.
- **3c.** A computer's flag falls:
  - 3c1. System records the result per RULE-4. Use case continues at step 6.
- **3d.** The engine fails:
  - 3d1. As UC-6 extension 5b: the game stays resumable and the System offers retry or exit (ENG-4). Use case ends or resumes at step 3.
- **6a.** The same computer Player was chosen for both sides:
  - 6a1. System stores the game as self-play: no rating change, excluded from W/L/D (ELO-1, UI-2). Use case resumes at step 7.
- **\*a.** The User closes the app or tab, or the app is interrupted:
  - \*a1. System suspends the game: unfinished, clocks frozen, resumable by UC-9 (RULE-5, GAME-4). Use case ends.

**Technology and Data Variations:**
- The watcher cannot rewind during play; after the game, UC-15 is available (GAME-2).
- CLI watch keys: Space pause/resume, `s` step, `+`/`-` delay, `a` abort, PgUp/PgDn scroll (CLI-4). GUI: equivalent controls.
- The active profile's maximum think time applies to both computers (UI-1, Q69).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- PLY-1: computer–computer is available only by explicit choice and is always Strict.
- NFR-3: a soak of 1000 computer–computer games on an isolated store completes without crashes (verification of the requirement, run by the project, not a User step).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | UC-5 |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Leads to UC-15. Resumed by UC-9. |
| Open issues | None. UG-3 and UG-13 closed 2026-10-28 (CR-4). "Move now" is not offered in a computer–computer game; the watcher pauses (ENG-5). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLY-1, GAME-1, GAME-2, GAME-4, RULE-4, ELO-2, ENG-3, ENG-4, ANA-3, ANA-4, UI-1, CLI-4, NFR-3, ENG-5, UI-12 |
| Functions (functional_decomp v1.0) | F-2.5.4, F-2.5.5, F-2.4.1, F-2.4.3, F-2.2.3, F-2.6.1, F-2.6.5, F-3.4, F-3.7, F-4.1, F-4.2 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: move-now is unavailable here (UG-3 closed); headless games and ratings answered in UC-27 (UG-13 closed); UI-12 added to the trace. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
