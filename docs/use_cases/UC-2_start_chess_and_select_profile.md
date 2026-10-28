# UC-2: Start Chess and Select Profile

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-2_start_chess_and_select_profile.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to open Chess through the desktop app, the `chess` command or the web page, and continue as one registered Player, so that settings, history and games belong to the right Player.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: reaches the home screen quickly (startup limits in NFR-1), with the right profile.
- Other Players: their data is untouched by another Player's session.
- Project owner: the store is never used by a mismatched version or after a failed integrity check.

**Preconditions:** Chess is installed (UC-1). For the web entry point, the local server is running (UC-4).

**Minimal Guarantees:** The data store is unchanged, unless this launch created it or migrated it after a full backup. A damaged store file is left unmodified (DAT-5). The terminal is restored on exit (CLI-1).

**Success Guarantees:** The chosen Player is the active Player, the Player's settings are loaded, and the home screen is shown.

**Trigger:** The User opens the desktop app, types the application name (`chess`), or opens the page of a running `chess serve` (PLAT-1).

**Main Success Scenario:**

1. User launches an entry point.
2. System checks that the store and any other running front-end belong to its own version (PLAT-5) and that the store passes its integrity check (DAT-5).
3. System shows the list of registered human Players, with the choice to register a new Player.
4. User chooses a Player.
5. System loads the Player's settings and shows the home screen.

**Extensions:**

- **2a.** There is no store (first launch on a clean machine):
  - 2a1. System creates the store, its schema and the computer Player roster, with no network connection (PLAT-3, PLY-4).
  - 2a2. System continues at step 3 with an empty list (3a).
- **2b.** The store or another running front-end has a different version:
  - 2b1. System refuses to use the store and shows a message naming both versions (PLAT-5, NFR-9).
  - *Then:* Use case fails.
- **2c.** The store fails the integrity check:
  - 2c1. System tells the User, offers to restore the most recent backup, and leaves the damaged file unmodified (DAT-5).
  - *Then:* Use case continues as UC-26, then resumes at step 3.
- **2d.** The store has an older schema version:
  - 2d1. System writes a complete backup copy, then upgrades the store (DAT-6).
  - 2d2. System resumes at step 3.
  - 2d3. If the backup cannot be written, System does not upgrade, leaves the store unchanged, shows the cause, and offers retry or exit (DAT-6, NFR-9). If the User exits, use case fails.
- **2e.** The store was written by a newer build:
  - 2e1. System refuses to open it (F-6.9) and shows a message (NFR-9).
  - *Then:* Use case fails.
- **2f.** The entry point is the CLI and the terminal is smaller than 80x24:
  - 2f1. TUI asks the terminal to resize; if 80x24 is not reached within 1 s (EST), it shows an "enlarge the terminal" message and redraws as soon as the size is enough (CLI-1).
  - 2f2. System resumes at step 3.
- **2g.** The newest backup is older than 24 hours and no other front-end is running:
  - 2g1. System writes an automatic backup copy, checks that it opens, and keeps the newest 3 automatic copies (DAT-5, F-6.15). A failure is logged (NFR-10) and never stops the start-up. Use case resumes at step 3.
- **3a.** There are no registered Players:
  - 3a1. Registering is the only choice. User registers a Player (UC-3) and the new Player becomes the active Player (PLY-2).
  - *Then:* Use case continues at step 5.
- **4a.** User chooses to register a new Player:
  - 4a1. User registers a Player (UC-3), who becomes the active Player (PLY-2).
  - *Then:* Use case continues at step 5.
- **\*a.** The User quits at any step:
  - \*a1. System exits and restores the terminal, cursor, echo and main screen (CLI-1).
  - *Then:* Use case ends without an active Player.

**Technology and Data Variations:**
- Entry points: desktop app (Tauri, WKWebView), CLI TUI, CLI plain line-oriented mode (screen readers, pipes, no colour), browser tab via `chess serve` (PLAT-1, CLI-1, UI-4).
- `chess play --player <name>` goes straight into a game (architecture 6.1). Scripted headless runs skip profile selection (PLY-2, UC-27).
- Archived Players are not listed (PLY-2, PLY-4).
- The User can return to profile selection at any time while no game is open in this front-end, to switch the active Player (PLY-2).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- NFR-1: playable within 3 s (CLI 1 s), median of 10 warm starts, on the baseline Mac.
- PLAT-3: first launch and the whole use case work with no Internet.
- PLAT-2, UI-11: supported platforms; English text only.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | NFR-1 (startup to first screen accepting input: 3 s, CLI 1 s). |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Included by none. Extended by UC-3 (register new Player), UC-26 (damaged store). Followed by every other use case. |
| Open issues | None. UG-8 and UG-17 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLAT-1, PLAT-3, PLAT-5, PLY-2, PLY-4, DAT-1, DAT-5, DAT-6, NFR-1, NFR-9, CLI-1, UI-4, UI-11, PLAT-2 |
| Functions (functional_decomp v0.3) | F-8.1, F-8.5, F-6.5, F-6.6, F-6.8, F-6.15, F-6.9, F-1.5, F-6.1, F-6.4, F-7.1.3, F-7.1.5, F-7.3.3, F-7.3.4, F-7.4.3, F-7.4.4, F-7.4.5 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: failed pre-upgrade backup (DAT-6, UG-17), new Player becomes active (PLY-2, UG-8, A-4 closed), switching the active Player. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. Automatic backup (F-6.15) added as extension 2g. |
