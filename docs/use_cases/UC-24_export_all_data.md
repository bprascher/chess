# UC-24: Export All Data

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-24_export_all_data.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants a complete backup of everything in the store as one archive, so that it can be restored or moved.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: nothing is left out of the backup (DAT-2).
- Project owner: the format is versioned so later versions can import it (IF-5).

**Preconditions:** The store is available.

**Minimal Guarantees:** The store is unchanged. No partial archive is reported as success.

**Success Guarantees:** One ZIP archive exists, holding `manifest.json` (with the format version), `games.pgn`, `games.json` (analysis results and the stored game fields that PGN does not carry), `players.json`, `ratings.json` and `settings.json` (D-29 as amended by IF-5).

**Trigger:** The User chooses to export all data (`chess export data` in the CLI).

**Main Success Scenario:**

1. User asks to export all data.
2. System asks where to save the archive.
3. User gives the destination.
4. System writes the archive: all games as PGN (with saved variations as PGN variations), the per-game records and analysis results, and Players, ratings with history, and settings as JSON, with a format version.
5. System confirms the export.

**Extensions:**

- **4a.** The archive cannot be written:
  - 4a1. System shows what failed and the next step (NFR-9). Use case fails.
- **4b.** The store holds saved variations and analysis results:
  - 4b1. Analysis results go to `games.json`, and variations travel in `games.pgn` as PGN variations (DAT-2, IF-5, IF-1). Use case resumes at step 5.

**Technology and Data Variations:**
- All data includes computer Players, archived Players and unfinished games (DAT-2).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Inverse of UC-25. |
| Open issues | None. UG-12 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | DAT-2, IF-5, IF-1, NFR-9 |
| Functions (functional_decomp v0.3) | F-6.12 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: archive content decided (DAT-2, IF-5; UG-12 closed): `games.json` holds analysis results and the stored fields PGN does not carry. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
