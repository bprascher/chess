# UC-10: Resign Game

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-10_resign_game.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to end a game on purpose by conceding it, which is the only way a human ends a game early.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- Resigning Player: a confirmation step prevents an accidental loss (RULE-2).
- Opponent: receives the win in Elo and W/L/D when the game is rated.
- Project owner: no front-end can skip the confirmation (F-2.5.7).

**Preconditions:** A human–computer or human–human game is in progress or suspended. The matchup is not computer–computer.

**Minimal Guarantees:** If the User cancels, the game continues unchanged, or stays suspended if resign was chosen from the resume prompt.

**Success Guarantees:** The game has the result "loss" for the resigning side with reason resignation. When the game is rated, Elo on the game's scale and W/L/D record the loss. The game's slots are freed, and the game stays in history and can be analysed (RULE-2, GAME-4, ANA-4).

**Trigger:** The User enters `resign` (CLI) or uses the resign control (GUI), or chooses resign when setup or resume offers it.

**Main Success Scenario:**

1. User asks to resign the game.
2. System asks for confirmation.
3. User confirms.
4. System ends the game with a loss for the resigning side and the reason resignation.
5. System finalises the game: statistics, and Elo on the game's scale when the game is rated.
6. System offers full-game analysis (UC-15).

**Extensions:**

- **1a.** The request comes from the resume or setup prompt for a suspended game:
  - 1a1. System frees the slot after the game ends (GAME-4, state machine Suspended to Completed). Use case continues at step 3.
- **2a.** The game is a hot-seat human–human game:
  - 2a1. System asks which side is resigning (RULE-2, CLI-4). This also applies when resign was chosen from the setup or resume offer, where either human at the shared screen may answer (GAME-1).
  - 2a2. User chooses the side. Use case resumes at step 2.
- **3a.** User cancels:
  - 3a1. The game continues unchanged. Use case ends.
- **5a.** The game is unrated (self-play, custom start position):
  - 5a1. System records the loss in the game record only. It changes no rating and counts the game in no W/L/D total or aggregate (ELO-2, UI-2). Use case resumes at step 6.

**Technology and Data Variations:**
- CLI: `resign` command. GUI: resign control. Confirmation in both (CLI-4, RULE-2).
- Training: the loss counts on the Training scale; Strict: on the Strict scale (GAME-4, ELO-2).
- Computers never resign (RULE-2). There is no abort command in human games (CLI-4).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | UC-6, UC-7, UC-9 |
| Extended by / included by | None |
| Open issues | None. UG-7 and UG-18 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | RULE-2, GAME-4, PLY-5, ELO-1, ELO-2, DAT-3, ANA-4, CLI-4, GAME-1, UI-2, UI-12 |
| Functions (functional_decomp v1.0) | F-2.5.3, F-2.5.7, F-2.6.5, F-4.1, F-4.2, F-4.3, F-7.1.2 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: custom-start games out of W/L/D (UG-18 closed), resign from the prompt in a hot-seat game (UG-7 closed), UI-12 added. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
