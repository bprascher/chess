# UC-4: Start Local Web Server

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-4_start_local_web_server.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants the local web server running on a chosen port and the page opened in a browser, so that Chess can be played in a browser tab.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as operator of the web front-end

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: the page opens with one command, offline.
- User (security): only this Mac and only the page's own origin can reach the server (NFR-7, ASM-1).
- Other web page in the browser: must not be able to drive games (NFR-7).

**Preconditions:** Chess is installed (UC-1). No other process uses the chosen port.

**Minimal Guarantees:** No server is left running on a failed start, and no non-loopback connection is ever made (NFR-4). The data store is unchanged.

**Success Guarantees:** A server listens on the loopback interface only, the GUI bundle is served from it, the URL is printed, and the page is open in the default browser unless `--no-open` was given.

**Trigger:** The User types `chess serve`, with optional `--port <n>` and `--no-open`.

**Main Success Scenario:**

1. User runs `chess serve`.
2. System selects the port: the command-line option, otherwise the `config.toml` setting, otherwise 8765.
3. System binds the loopback address on that port and starts serving the embedded GUI bundle.
4. System prints the page URL, `http://127.0.0.1:<port>/`.
5. System opens the URL in the default browser.
6. User continues in the browser tab (UC-2).

**Extensions:**

- **1a.** User gives `--no-open`:
  - 1a1. System skips step 5. Use case resumes at step 6.
- **2a.** `config.toml` cannot be read or has an invalid port:
  - 2a1. System exits with status 1 and a message naming the file, the line and the reason. It does not fall back to defaults (PLAT-4, CLI-5, NFR-9). Use case fails; nothing is bound. A missing `config.toml` is not an error: defaults apply.
- **3a.** The chosen port is already in use:
  - 3a1. System exits with a message naming the port and a non-zero status (PLAT-4).
  - *Then:* Use case fails; nothing is bound.
- **5a.** The default browser cannot be opened:
  - 5a1. System keeps the server running, and prints the URL again with a notice to open it by hand (PLAT-4). The exit status is not affected. Use case resumes at step 6.
- **\*b.** The User stops the server with Ctrl-C or SIGTERM in its terminal:
  - \*b1. System stops accepting connections, tells connected pages that the server stopped, suspends each open game as an interruption (clock suspended, state recorded, resumable, no change to Elo or statistics) and exits with status 0, with no confirmation (PLAT-4, RULE-5, GAME-4). Use case ends.
- **\*a.** A request arrives whose Host or Origin is not the server's own loopback origin:
  - \*a1. System rejects it (NFR-7). The use case is not affected.

**Technology and Data Variations:**
- Port: option overrides config file; default 8765 (PLAT-4, D-24).
- Browsers: current-3-year versions of Safari, Chrome, Firefox and Edge (PLAT-2).
- Closing the tab suspends the game's clock and state, even while the server keeps running (RULE-5, D-14). The User stops the server as in extension \*b.

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- PLAT-3, NFR-4: nothing remote is loaded; Content-Security-Policy `default-src 'self'`.
- NFR-1: ready within 3 s measured from running `chess serve`.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | NFR-1 (3 s, measured from `chess serve` to the first screen accepting input). |
| Channels to actors | Terminal (command), browser tab. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | Precedes UC-2 for the web entry point. |
| Open issues | None. UG-15 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLAT-1, PLAT-4, PLAT-3, PLAT-5, NFR-4, NFR-7, NFR-1, NFR-9 |
| Functions (functional_decomp v1.0) | F-8.2, F-8.3, F-8.4, F-7.4.3, F-7.4.5, F-8.1, F-8.12 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: config errors, browser failure and stopping the server decided (PLAT-4, UG-15 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
