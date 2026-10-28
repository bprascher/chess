# UC-12: Take Back Moves

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-12_take_back_moves.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to undo their last move and the computer's reply in a Training game, so that they can try a different move.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: learning by retrying.
- Project owner: takeback never exists in Strict, so Strict ratings are not distorted (PLY-5).

**Preconditions:** A Training game (human–computer) is in progress.

**Minimal Guarantees:** If takeback is not available, the game is unchanged and the User is told why.

**Success Guarantees:** The position and move list are as they were before the User's last move, at the User's turn. The game stays rated on the Training scale (RULE-3, ELO-2).

**Trigger:** The User enters `undo` (CLI) or uses the takeback control (GUI).

**Main Success Scenario:**

1. User asks to take back.
2. System undoes the User's last move and the computer's reply, back to the User's turn, and removes the taken-back moves from the stored move list and the PGN (RULE-3).
3. System shows the earlier position and move list.
4. User may repeat steps 1 to 3 back to the User's first turn: the start position when the User has White, or the position after the computer's first move when the User has Black (RULE-3).

**Extensions:**

- **1a.** The game is Strict:
  - 1a1. Takeback is absent or rejected, with the reason (RULE-3, PLY-5, NFR-9). Use case fails.
- **1b.** The computer is searching:
  - 1b1. System cancels the search and undoes only the User's last move, which gives back the User's turn from before it (RULE-3, architecture 4.3 EngineThinking to AwaitingMove). Use case continues at step 3.
- **2a.** There is nothing to undo:
  - 2a1. System rejects the request with the message "Nothing to take back" and leaves the game unchanged (RULE-3, NFR-9). Use case fails.
- **2b.** The User plays Black and takeback would reach the start:
  - 2b1. The computer's first move is never taken back. Takeback stops at the position after it (RULE-3). Use case ends.

**Technology and Data Variations:**
- CLI `undo`; GUI control. Hidden or shown as unavailable in Strict (CLI-4).

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
| Open issues | None. UG-5 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | RULE-3, PLY-5, CLI-4 |
| Functions (functional_decomp v0.3) | F-2.5.1, F-2.3.1, F-3.6 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: takeback details decided (RULE-3, UG-5 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
