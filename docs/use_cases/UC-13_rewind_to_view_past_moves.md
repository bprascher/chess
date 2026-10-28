# UC-13: Rewind to View Past Moves

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-13_rewind_to_view_past_moves.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to look back at earlier positions of the game in play, read-only, and then return to the live position.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: can think about the game's history; the clock keeps running (GAME-3).
- Opponent: nothing in the game record can change while the User is rewound.
- Project owner: rewind is client-side view state and never alters the game state machine.

**Preconditions:** A human Player is in a game in progress (Training or Strict). In a hot-seat game either human may rewind.

**Minimal Guarantees:** The stored game record is unchanged (GAME-3).

**Success Guarantees:** The User viewed earlier positions and is back at the live position, which shows every move made meanwhile.

**Trigger:** The User enters `back` (CLI; also the left arrow) or uses the rewind control (GUI).

**Main Success Scenario:**

1. User steps back to an earlier position.
2. System shows that position read-only, with the clocks still running.
3. User steps forward or back through the positions.
4. User returns to the live position (`live`, End).
5. System shows the live position.

**Extensions:**

- **2a.** The User tries to make a move while rewound:
  - 2a1. System rejects it (GAME-3, NFR-9). Use case resumes at step 3.
- **2b.** The computer's move arrives while the User is rewound:
  - 2b1. The view stays where it is and a "live position has changed" indicator is shown (GAME-3). Use case resumes at step 3.

**Technology and Data Variations:**
- CLI: `back`, `forward`, `live`, arrow keys, End (CLI-4). GUI: equivalent controls (GAME-3).
- Not available to a computer–computer watcher, who can only scroll the move list (GAME-2, Q50).
- Computer Players cannot rewind (GAME-3).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | UC-6, UC-7 |
| Extended by / included by | None |
| Open issues | None specific. (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | GAME-3, GAME-2, CLI-4, NFR-9 |
| Functions (functional_decomp v0.3) | F-2.5.6, F-2.2.3, F-7.4.2, F-7.4.6 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
