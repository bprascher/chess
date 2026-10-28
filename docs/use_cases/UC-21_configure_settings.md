# UC-21: Configure Settings

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-21_configure_settings.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to adjust their Player's settings, themes and display options, so that Chess looks and behaves as they like.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: changes take effect at once and persist.
- Accessibility users: high-contrast themes and visible focus are available (UI-3, UI-7, UI-8).

**Preconditions:** An active Player exists (UC-2).

**Minimal Guarantees:** Invalid values are rejected or clamped; stored settings stay valid (UI-1).

**Success Guarantees:** The settings are stored for the Player and apply in every front-end of the same Player.

**Trigger:** The User opens the Settings screen.

**Main Success Scenario:**

1. User opens the settings of the active Player.
2. System shows the settings that apply to the current front-end.
3. User changes one or more settings.
4. System validates and applies each change immediately.
5. System stores the settings for the Player.

**Extensions:**

- **2a.** The front-end is the CLI:
  - 2a1. The settings screen lists no board, piece or GUI colour theme (UI-1); CLI styles (plain, Unicode, ANSI colour, light/dark squares, no-colour) apply instead (UI-3). Use case resumes at step 3.
- **2b.** The front-end is the GUI and the system appearance changes:
  - 2b1. With the default colour theme the GUI follows the system and switches (UI-10).
- **4a.** A value is invalid:
  - 4a1. System rejects or clamps it, with a message (UI-1, NFR-9). Use case resumes at step 3.

**Technology and Data Variations:**
- Settings: default opponent category, board theme, piece theme, GUI colour theme, default colour, legal-move hints, coordinate orientation (flip), maximum engine think time (can only shorten) (UI-1). Coordinates on or off in the GUI (GUI-3).
- At least 10 board themes and 5 piece sets, including high-contrast (UI-3). Every bundled theme meets the contrast ratios (UI-7).
- There is no sound, input-method, confirm-move or language setting in v1 (UI-1, UI-11).

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
| Open issues | None specific. (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | UI-1, UI-3, UI-7, UI-8, UI-10, UI-11, GUI-3, NFR-9 |
| Functions (functional_decomp v0.3) | F-1.7, F-7.2.4, F-7.1.1, F-7.3.4 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
