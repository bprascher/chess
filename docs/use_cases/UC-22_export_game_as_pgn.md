# UC-22: Export Game as PGN

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-22_export_game_as_pgn.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to save a game as a PGN file, so that it can be kept, shared or opened in another chess program.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: a standard file that other readers open.
- Project owner: identical export from all three front-ends (IF-1).

**Preconditions:** The game is stored and listed in the game history.

**Minimal Guarantees:** The stored game is unchanged. No partial file is reported as success.

**Success Guarantees:** A PGN file in the IF-1 format exists at the chosen place, holding the Seven Tag Roster, SAN movetext, result, custom tags and any saved variations.

**Trigger:** The User chooses to export a game from the game history list (`chess export game <game>` in the CLI).

**Main Success Scenario:**

1. User selects a game in the history list and asks to export it.
2. System asks where to save the file.
3. User gives the destination.
4. System writes the game as PGN encoded as UTF-8.
5. System confirms the export.

**Extensions:**

- **1a.** The game is voided, aborted or unfinished:
  - 1a1. A voided or aborted game is exported with Result "*", Termination "abandoned" and an `EndStatus` tag (IF-1). An unfinished game is exported with Result "*", no Termination tag and no `EndStatus` tag, with the moves completed so far. The game stays unfinished, and a re-import gives a review-only game with no result (IF-1, RULE-6).
- **4a.** The file cannot be written:
  - 4a1. System shows what failed and the next step (NFR-9). Use case fails.

**Technology and Data Variations:**
- Web: the browser's file save. CLI: a path argument. Desktop: a native save dialog. (Mechanism per front-end is not specified in the requirements.)
- Non-standard start: `SetUp` and `FEN` tags. Timed game: `TimeControl`. Custom tag names: TBD-24.
- The PGN deviates from the 1994 standard only in being UTF-8 (IF-1).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Part of the full export in UC-24. |
| Open issues | None. UG-11 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | RULE-6, IF-1, NFR-9, ANA-2 |
| Functions (functional_decomp v0.3) | F-6.10 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: export of an unfinished game decided (IF-1, UG-11 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
