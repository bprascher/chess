# UC-17: Rename Human Player

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-17_rename_human_player.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to change a human Player's name while keeping all history, ratings and statistics.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- Player: history follows the permanent UUID, not the name (PLY-6).
- Other Players: names stay unique.

**Preconditions:** The human Player exists and is not archived.

**Minimal Guarantees:** The old name and all records are unchanged.

**Success Guarantees:** The Player has the new name; every game, rating and statistic still belongs to the Player (PLY-4, PLY-6).

**Trigger:** The User chooses to rename a Player in the Players screen.

**Main Success Scenario:**

1. User selects the Player and asks to rename.
2. System asks for the new name.
3. User enters the new name.
4. System validates the name.
5. System stores the new name and shows it.

**Extensions:**

- **1a.** A computer Player is selected:
  - 1a1. System does not offer rename for a computer Player: its name is fixed at creation (PLY-4). Use case fails.
- **4a.** The name is empty after trimming, longer than 128 user-perceived characters, contains a control or line-break character, or matches another Player's name (case-sensitive, after NFC normalization):
  - 4a1. System rejects it and states the reason (PLY-6, NFR-9). Use case resumes at step 3.
- **\*a.** User cancels:
  - \*a1. The name is unchanged. Use case ends.

**Technology and Data Variations:**
- Same validation as UC-3.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | None |
| Open issues | None. UG-9 and UG-19 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLY-4, PLY-6, NFR-9 |
| Functions (functional_decomp v0.3) | F-1.3, F-1.2, F-7.3.4 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: computer Players cannot be renamed (PLY-4, UG-9 closed); name rules aligned with PLY-6 (UG-19 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
