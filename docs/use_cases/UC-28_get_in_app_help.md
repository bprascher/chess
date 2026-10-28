# UC-28: Get In-App Help

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-28_get_in_app_help.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to see on any screen what controls, commands and key bindings are available.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** Subfunction

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: help that works offline, on every GUI and TUI screen.
- Screen-reader user: help reachable by keyboard (UI-4).

**Preconditions:** Any GUI or TUI screen is shown.

**Minimal Guarantees:** The screen the User was on is unchanged.

**Success Guarantees:** The User saw that screen's controls, commands and key bindings, and is back on the screen.

**Trigger:** The User presses F1 or enters `help`.

**Main Success Scenario:**

1. User asks for help.
2. System shows the controls, commands and key bindings of the current screen, from content shipped with the app.
3. User closes the help.
4. System returns to the screen.

**Extensions:**

- **2a.** The terminal was left in a bad state after the app was killed (SIGKILL or power loss):
  - 2a1. The help tells the User to run `reset` (CLI-1, UI-9). Use case resumes at step 3.

**Technology and Data Variations:**
- Disallowed commands are shown as unavailable with the reason (CLI-4, PLY-5).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- PLAT-3: help content is bundled; nothing is fetched.

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
| Open issues | None specific to this use case. (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | UI-9, PLAT-3, CLI-1, CLI-4, UI-4 |
| Functions (functional_decomp v1.0) | F-7.3.2 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
