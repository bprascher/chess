# UC-19: Generate More Computer Players

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-19_generate_more_computer_players.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants more computer opponents in a chosen starting category, in addition to the 25 generated at first launch.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: more opponents of the strength wanted.
- All Players: engine parameters of every computer stay fixed once created (PLY-4, ENG-2).
- Project owner: a computer's category label follows its Strict rating after creation (PLY-4).

**Preconditions:** The roster exists (5 computer Players per category, 25 in all, generated once from a fixed seed at first launch).

**Minimal Guarantees:** No computer Player is created if the request fails.

**Success Guarantees:** New computer Players exist with human-readable names, engine parameters within their category's bounds fixed for life, and both ratings at the category baseline (400, 600, 1200, 2200 or 2500).

**Trigger:** The User chooses to generate more computer Players on the Computer Players screen.

**Main Success Scenario:**

1. User asks to generate more computer Players.
2. System asks for the starting category and the number of Players to create, 1 to 25, default 5 (PLY-4, TBD-31).
3. User chooses the category and the number.
4. System creates that many Players, with names from the roster's name generator, and stores them with their fixed engine parameters and both starting ratings.
5. System shows them in the Computer Players list.

**Extensions:**

- **4a.** A generated name equals an existing Player's name (human or computer, active or archived):
  - 4a1. The generator skips that name and takes the next one, so the User is never asked to resolve a collision (PLY-4, PLY-6). Use case resumes at step 4.
- **3a.** The number is outside 1 to 25:
  - 3a1. System rejects it with the reason (PLY-4, NFR-9). Use case resumes at step 3.
- **\*a.** User cancels:
  - \*a1. Nothing is created. Use case ends.

**Technology and Data Variations:**
- One request creates N Players, all in the chosen category; N is 1 to 25, default 5 (PLY-4, TBD-31). Engine parameter sets are unique across all computer Players (ENG-2).

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
| Open issues | None. UG-10 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLY-4, ENG-2, ELO-1, PLY-6 |
| Functions (functional_decomp v0.3) | F-1.6, F-4.4 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: number of Players per request and name collisions decided (PLY-4, UG-10 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
