# UC-23: Import PGN Games

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-23_import_pgn_games.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to bring games from a PGN file into Chess for review and analysis, so that games from elsewhere can be studied.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: every valid game is imported even when others in the file are broken.
- Players and ratings: imported games never change any rating or statistic (RULE-7).

**Preconditions:** A PGN file is available. It is no larger than the size limit (TBD-25).

**Minimal Guarantees:** Games already stored are unchanged. A game that fails to parse is not imported.

**Success Guarantees:** Every valid game in the file is stored as review-only and listed in game history; the User has a report of the games that failed, by index and first error line.

**Trigger:** The User chooses to import a PGN file (`chess import pgn <file>` in the CLI).

**Main Success Scenario:**

1. User selects a PGN file and asks to import it.
2. System reads the file, which may hold many games.
3. System imports each valid game as review-only.
4. System reports the number imported, the number skipped as duplicates, and, for each game that failed to parse, its index and the line of its first error.
5. User opens an imported game for analysis (UC-15).

**Extensions:**

- **2a.** The file is larger than the size limit:
  - 2a1. System rejects it with a message (IF-2, NFR-9). Use case fails.
- **2b.** The file is not readable as PGN:
  - 2b1. System shows a message (NFR-9). Use case fails.
- **3a.** A game is identical to one already imported:
  - 3a1. System raises the collision prompt of DAT-2 with "skip this game" (the default) and "keep both", and "apply to all remaining collisions". "Replace" is not offered, because the contents are identical and a replacement could overwrite a played, rated game (IF-2). A skipped duplicate is not an error. The same applies to a game that repeats an earlier game of the same file. Use case resumes at step 3.
- **3b.** A game fails to parse:
  - 3b1. System skips it and imports the rest (IF-2). Use case resumes at step 3.

**Technology and Data Variations:**
- Accepts tags, SAN movetext, comments, NAGs, variations, and `SetUp`/`FEN` starting positions. Latin-1 or UTF-8 (IF-2).
- Imported variations are kept for what-if analysis (IF-2, ANA-2).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Leads to UC-15. |
| Open issues | None. UG-22 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | RULE-7, IF-2, ELO-2, ANA-4, NFR-9 |
| Functions (functional_decomp v0.3) | F-6.11, F-4.1 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: duplicate-game handling decided (IF-2, UG-22 closed): the DAT-2 prompt offers "skip" and "keep both". |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
