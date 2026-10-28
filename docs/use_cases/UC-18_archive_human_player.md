# UC-18: Archive Human Player

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-18_archive_human_player.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to remove a human Player from the lists (delete by archiving) while keeping the Player's history.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- Archived Player: history is kept (PLY-4).
- Opponents: their history and statistics are unchanged.
- User: a confirmation prevents an accidental archive (Q52).

**Preconditions:** The human Player exists. Computer Players cannot be deleted (PLY-4).

**Minimal Guarantees:** If cancelled, the Player stays active.

**Success Guarantees:** The Player is hidden from the profile list and Player pickers; all records are kept; the name stays reserved (PLY-6).

**Trigger:** The User chooses to archive a Player in the Players screen.

**Main Success Scenario:**

1. User selects the Player and asks to archive.
2. System asks for confirmation.
3. User confirms.
4. System archives the Player and hides it from lists.

**Extensions:**

- **1a.** The selected Player is the active Player of this front-end:
  - 1a1. System does not offer archive; the User first switches to another profile (PLY-4, PLY-2). Use case fails.
- **1b.** The selected Player has an unfinished game in either mode:
  - 1b1. System refuses, names each such game, and tells the User to finish or resign it (PLY-4, NFR-9). Nothing changes. Use case fails.
- **3a.** User cancels:
  - 3a1. The Player stays active. Use case ends.

**Technology and Data Variations:**
- There is no un-archive in v1 (PLY-4, Out of Scope). The name stays reserved (PLY-6).

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
| Open issues | None. UG-9 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLY-4, PLY-6, PLY-2, UI-5, UI-12, GAME-4 |
| Functions (functional_decomp v1.0) | F-1.4, F-2.5.7 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: archive guards and no un-archive decided (PLY-4, UG-9 and FG-8 c closed); UI-12 added. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
