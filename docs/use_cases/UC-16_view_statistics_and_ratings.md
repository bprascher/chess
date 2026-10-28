# UC-16: View Statistics and Ratings

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-16_view_statistics_and_ratings.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants to see results, game history and both Elo ratings over time for any Player, human or computer, so that progress and opponents can be judged.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: accurate numbers by mode, colour and opponent.
- Opponents: their history keeps games against a Player who was reset or archived (PLY-7, UI-5).
- Reset Player: sees only games played after the reset.

**Preconditions:** An active Player exists (UC-2).

**Minimal Guarantees:** No data is changed.

**Success Guarantees:** The User saw the requested statistics, charts and game history.

**Trigger:** The User opens the Statistics screen, or `chess stats [player]`, or the Computer Players screen.

**Main Success Scenario:**

1. User chooses a Player, human or computer, or accepts the active Player.
2. System shows W/L/D totals, by mode, by colour and by opponent or category.
3. System shows average game length in moves and in time.
4. System shows the Strict and Training Elo, current and over time, as a chart for each scale.
5. System shows the Player's game history list.
6. User selects a game to replay or analyse (UC-15).

**Extensions:**

- **1a.** The User opens the Computer Players screen:
  - 1a1. System lists all computer Players with current category, both current ratings and W/L/D (UI-5). Use case resumes at step 1.
- **1b.** The Player is archived:
  - 1b1. Its history stays visible where other Players' records refer to it (UI-5, PLY-4).
- **2a.** The Player has played no games:
  - 2a1. System shows zero counts, "—" for every average, a "no games yet" message, and each Elo chart with its baseline point (UI-2). No error is raised.
- **2b.** The Player's data was reset:
  - 2b1. The Player's own views show only games after the reset; opponents' views and statistics still include all games (PLY-7, UI-5).
- **2c.** A game is self-play or was voided by a reset:
  - 2c1. Self-play and custom-start games are listed in history (the latter marked "custom start") and excluded from aggregates (UI-2). A voided game appears only in the opponent's history, marked "voided by reset", and never in a statistic (UI-5).

**Technology and Data Variations:**
- CLI: the Statistics screen, or tables in plain mode (`chess stats`, `chess players`). GUI: tables and Elo chart with a text or table alternative for screen readers (architecture 6.2, UI-4).
- Elo chart library is deferred (D-21). In the CLI the TUI draws each scale's history as a line chart in character cells with a table of the latest points, and plain mode prints the table with a sparkline (UI-2).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- DAT-3 stored fields feed the numbers.

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
| Open issues | None. UG-16 and UG-18 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | UI-2, UI-5, PLY-4, PLY-7, DAT-3, ELO-1, UI-4 |
| Functions (functional_decomp v0.3) | F-4.5, F-4.4, F-7.3.1, F-1.8.3 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: empty statistics, CLI chart form and custom-start games decided (UI-2, UG-16 and UG-18 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
