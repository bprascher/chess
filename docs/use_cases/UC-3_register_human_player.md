# UC-3: Register Human Player

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-3_register_human_player.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants a new human Player profile with a unique name and a self-assessed category, so that games, ratings and settings are kept for that Player.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: a profile with a fair starting rating for the stated strength.
- Other Players: names stay unambiguous in lists and statistics (PLY-6).

**Preconditions:** The Chess front-end is running. At launch there may be no active Player yet (UC-2).

**Minimal Guarantees:** No Player record is created when registration fails or is cancelled. Existing Players are unchanged.

**Success Guarantees:** A human Player exists with a permanent UUID, the chosen name and category, default preferences, and both Elo ratings (Strict and Training) at the category baseline (400, 600, 1200, 2200 or 2500).

**Trigger:** The User chooses to register a new Player, at launch (UC-2) or later.

**Main Success Scenario:**

1. User asks to register a new Player.
2. System asks for a name and a self-assessed category (Beginner, Novice, Intermediate, Master, Grand Master).
3. User enters a name and chooses a category.
4. System validates the name.
5. System creates the Player and stores the Player, default preferences and both starting ratings.
6. System confirms the registration and shows the Player in the list. When registering at launch, the System makes the new Player the active Player (PLY-2).

**Extensions:**

- **3a.** User tries to finish without choosing a category:
  - 3a1. System does not complete the registration (PLY-2). Use case resumes at step 3.
- **4a.** The name is empty after trimming, longer than 128 user-perceived characters, contains a control or line-break character, or matches another Player's name (case-sensitive, after NFC normalization):
  - 4a1. System rejects the name and states the reason (PLY-6, NFR-9).
  - 4a2. User enters another name. Use case resumes at step 4.
- **5a.** The data store is not available:
  - 5a1. System creates nothing, leaves the store as it was, and shows what failed and the next step (DAT-7, NFR-9). Use case fails.
- **\*a.** User cancels:
  - \*a1. No Player is created. Use case ends.

**Technology and Data Variations:**
- Names keep their case. Uniqueness is case-sensitive and applies against every Player, human or computer, active or archived: "Brian" and "brian" are different names (PLY-6, amended by CR-4; the architecture's case-insensitive wording in 5.5 is to be corrected by the architect). The name is trimmed and compared after NFC normalization.
- Characters are Unicode extended grapheme clusters, so an accented letter or an emoji counts as one (PLY-6).
- A Player registered in the CLI is visible in the web and desktop front-ends through the shared store (PLY-2).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- NFR-9: the rejection message states what failed and what to do.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | UC-2 |
| Extended by / included by | Extends UC-2 (register new Player). |
| Open issues | None. UG-8, UG-17 and UG-19 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLY-2, PLY-6, ELO-1, UI-1, NFR-9, DAT-1, DAT-7 |
| Functions (functional_decomp v1.0) | F-1.1, F-1.2, F-7.3.3, F-7.3.4 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: name rules aligned with PLY-6 (UG-19 closed), failed write (DAT-7, UG-17), active Player at launch (UG-8). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
