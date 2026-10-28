# Chess Application — Requirements

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/requirements.md | **1.0, Amendment 3** | 2026-10-28 | **Baselined v1.0, 2026-10-26; Amendment 1 (CR-1–CR-3) applied 2026-10-27; Amendment 2 (CR-4) and Amendment 3 (CR-5) applied 2026-10-28** | Systems engineer (Claude), for Brian Prascher | Brian Prascher (user) |

**Baseline statement (v1.0, 2026-10-26).** Every requirement in this document is AGREED with priority Must, or is DELETED. No requirement is PROPOSED, and no detail is TBR. All user questions (Q1–Q79) are answered. What remains open is listed in the TBD/TBR/TBC register, and none of it needs a user decision before implementation starts:
- **Values set during design** by the architect: TBD-18 (screen-reader wording), TBD-21 (exit codes), TBD-24 (custom PGN tags), TBD-25 (PGN import size limit), TBD-26 (pinned Stockfish version), TBD-28 (stale-lease takeover time). The architect has proposed values for most of them (architecture §10).
- **Engineering estimates (EST)** accepted under Q53. Each must be confirmed by measurement on the baseline Mac.
- **One-time source checks** of values accepted from memory: TBC-2 (FIDE Laws), TBC-3 (PGN Standard, including its encoding clause), TBC-4 (WCAG 2.2 contrast criteria), TBC-5 (Nielsen's 0.1 s limit).

**Amendment 1 (2026-10-27).** The baseline statement above is kept as written on 2026-10-26. Three user decisions were applied through change control (Decision Log, 2026-10-27):
- **CR-1:** a reset is allowed while the Player has unfinished games, and voids them (PLY-7). This closes the architect's OQ-8 / G-20.
- **CR-2:** source checks TBC-2–TBC-5 are closed against the published sources. One sub-check remains as TBC-6 (PGN Seven Tag Roster and export-format tag rules).
- **CR-3:** the engine is pinned to Stockfish 19 (`sf_19`, 2026-09-05), closing TBD-26. Two engine facts remain to check (TBC-7, TBC-8), plus the exact GPL version (TBC-9).

No ID was renumbered. Items changed by the amendment are marked "Amended 2026-10-27 (CR-n)".

**Amendment 2 (2026-10-28).** The user delegated the closing of every gap and inconsistency in the use-case issue list (UG-1 to UG-22, assumptions A-4 and A-5) and in the functional-decomposition gap list (FG-1 to FG-12, FA-1 to FA-6) to the systems engineer, to be resolved by judgement without questions. **CR-4** applies those decisions at the requirements level. They are the systems engineer's decisions under that delegation, not user answers, so each one is logged in the Decision Log as an interpretation for the user to correct (Decision Log, 2026-10-28). Items changed are marked "Amended 2026-10-28 (CR-4)". No ID was renumbered and no statement was deleted.
- **New requirements:** PLAT-6 (install, update, uninstall), DAT-7 (failed writes and unreadable journals), UI-12 (confirmation of destructive actions), NFR-10 (local diagnostic log).
- **Amended:** PLAT-1, PLAT-4, PLY-2, PLY-4, PLY-6, GAME-1, GAME-4, RULE-2, RULE-3, RULE-6, ENG-5, DAT-2, DAT-5, DAT-6, UI-2, ANA-3, CLI-4, CLI-5, GUI-4, IF-1, IF-2, IF-5, NFR-5, NFR-6, RISK-1, RISK-2, Out of Scope.
- **Register:** TBD-31, TBD-32, TBC-10 added; TBD-4 note.
- Two behaviours were chosen where the documents conflicted: names are unique case-sensitively and counted in grapheme clusters (PLY-6), and a duplicate PGN game raises the DAT-2 prompt with "skip" and "keep both" (IF-2).

**Amendment 3 (2026-10-28).** A final consistency pass over requirements, architecture (amendment 2), functional decomposition (v0.2), use cases and the handoff, under the same delegation. **CR-5** settles the points the architect raised in architecture §10.4 and OQ-12, and the architect-chosen items that the requirements did not state. Items changed are marked "Amended 2026-10-28 (CR-5)". No ID was renumbered and no statement was deleted.
- **Amended:** DAT-5 (automatic backup, and what a restore cannot recover), DAT-6 (pre-upgrade copies are never pruned), CLI-4 (data commands and flags are design, scripted human play uses plain mode), CLI-5 (exit code 4), RULE-2 (accepted consequence of the hot-seat draw offer), UI-12 (the void offer), PLAT-4 (`--config`, `CHESS_CONFIG` are design).
- **Register:** TBD-33 added (EST); TBD-21 closed with the architect's codes 0 to 4.
- **Not changed:** IF-5 already states that `games.json` carries the engine version of each evaluation; IF-2 already states the duplicate rule that the architecture's `content_hash` implements (the architecture now traces both).

**Amendment 4 (2026-10-28).** The software engineer's readiness check for detailed design ([SRR-SFR-readiness.md](SRR-SFR-readiness.md)) put four questions to the user, who answered them the same day. **CR-6** records the answers. They are user decisions, not interpretations. **No requirement text, ID, verification, margin or budget changed**, so the use cases and the functional decomposition (both v1.0) are unaffected.
- **Criticality and coding standard (CR-6.1):** the software has ordinary criticality (not safety-related, not regulated). SEI CERT is the coding standard, with the standard Rust and TypeScript checks and every warning treated as an error. This is a development constraint, not a product requirement, so it has no requirement ID. It is held in [engineering/SWE-001-engagement-profile.md](engineering/SWE-001-engagement-profile.md) and architecture §8.
- **Design approvals (CR-6.2):** the system-architect agent approves, as a desk review, a design change inside one component. The user approves anything wider.
- **Interpretations confirmed (CR-6.3):** the systems engineer's interpretations that were open to the user's correction (listed in the Decision Log) are confirmed.
- **Intel test hardware (CR-6.4):** no Intel Mac is available and none is expected soon. PLAT-2 is unchanged and its Intel verification can't be run; the consequence is open (readiness FIND-015).

From v1.0 on, no baselined item is edited in place. A change is raised as a change request, its impact is assessed (requirements, interfaces, verification, margins, risks), and it is recorded in the Decision Log and the change history below before the text changes.

**Change history**
| Version | Date | Change |
|---|---|---|
| draft | 2026-10-24 | Initial draft from the user's 15 top-level requirements; sections 1–11 AGREED. |
| draft | 2026-10-25 | CLI amended to a full-screen TUI; PLAT-3 offline operation strengthened. |
| draft | 2026-10-26 | Independent review (29148, INCOSE GtWR); PROPOSED amendments and requirements; decision rounds 1 (Q41–Q55) and 2 (Q56–Q74). |
| **1.0** | 2026-10-26 | Final answers Q75–Q79 applied; every remaining PROPOSED and TBR item accepted (Q75); architecture decisions D-12, D-13, D-16, D-18–D-20, D-22–D-29 accepted; ASM-1 confirmed. Compound requirements split: RULE-5 → RULE-5–RULE-8, ENG-3 → ENG-3 and ENG-5. GAME-4 corrected (computers are never blocked by h–c games). Document baselined. |
| **1.0, Amendment 1** | 2026-10-27 | **CR-1** (reset with unfinished games voids them): PLY-7 statement and verification; notes in GAME-1, GAME-4, RULE-2, RULE-5, DAT-3, ELO-1, ELO-2, UI-2, UI-5, ANA-4, IF-1; TBD-24 updated (`EndStatus`); TBD-30 added (EST). **CR-2** (source checks closed): RULE-1 (FIDE edition, verified citation, dead-position limitation, release-checklist note), IF-1 (PGN §4.1 citation, export format), UI-7 (WCAG 2.2 citation, non-text wording aligned with SC 1.4.11), NFR-8 (Nielsen citation); TBC-2–TBC-5 closed; TBC-6 added. **CR-3** (Stockfish 19): IF-4, ENG-1 note, ENG-2 (no assumed `UCI_Elo` floor); TBD-26 closed; TBC-7–TBC-9 added. Register, Open Questions, and Decision Log updated. Nothing renumbered. |
| **1.0, Amendment 2** | 2026-10-28 | **CR-4** (systems engineer's decisions on the use-case and functional-decomposition gap lists, under the user's delegation): new PLAT-6, DAT-7, UI-12, NFR-10; amended PLAT-1, PLAT-4, PLY-2, PLY-4, PLY-6, GAME-1, GAME-4, RULE-2, RULE-3, RULE-6, ENG-5, DAT-2, DAT-5, DAT-6, UI-2, ANA-3, CLI-4, CLI-5, GUI-4, IF-1, IF-2, IF-5, NFR-5, NFR-6, RISK-1, RISK-2, Out of Scope. Register: TBD-31, TBD-32, TBC-10 added. Decision Log updated. Nothing renumbered. |
| **1.0, Amendment 3** | 2026-10-28 | **CR-5** (consistency pass): DAT-5 gains the automatic backup (24 h, newest 3 kept; TBD-33) and the stated limit of a restore; DAT-6 note; CLI-4, CLI-5 (exit code 4, TBD-21 closed), PLAT-4, RULE-2 and UI-12 notes accepting architect-chosen items. Register: TBD-33 added, TBD-21 closed. Decision Log updated. Nothing renumbered. |
| **1.0, Amendment 4** | 2026-10-28 | **CR-6** (user answers to the readiness check): criticality and coding standard recorded (CR-6.1), design approvers (CR-6.2), earlier interpretations confirmed (CR-6.3), no Intel test hardware (CR-6.4). No requirement text, ID or verification changed. Decision Log updated. Nothing renumbered. |

**Status legend:** `AGREED` = confirmed by user · `PROPOSED` = suggested by Claude, awaiting decision (none remain at v1.0) · `DRAFT` = user's original wording, wording/details still open (none remain at v1.0) · `DELETED` = withdrawn; the ID is kept and never reused · `EST` = engineering estimate by Claude, based on the baseline Mac (Q47); the user accepted these as starting values (Q53), and each must be confirmed by measurement and changed only through the Decision Log · `TBR` = a value or detail Claude proposed that needed the user's review (all accepted by Q75 at v1.0)

**Priority (AGREED 2026-10-26, Q56):** every AGREED requirement is mandatory, so its priority is **Must**. Every item accepted under Q75 is AGREED with priority Must.

**Current state (2026-10-26, baseline v1.0):**
- Sections 1–11 were AGREED by the user on 2026-10-24/25.
- On 2026-10-26 a requirements review added PROPOSED amendments and requirements. The user then answered three rounds of questions the same day:
  - Round 1: Q41–Q55.
  - Round 2: Q56–Q74 and the TUI minimum.
  - Final round: Q75–Q79, the computers' Training Elo, the architecture decisions, and ASM-1.
- The answers are merged into the requirement text, marked "AGREED 2026-10-26 (Qnn)". Where the coordinator interpreted an answer, the Decision Log records the interpretation.
- Items accepted under Q75 are marked "AGREED 2026-10-26 (Q75)". Values Claude had proposed from memory were checked against their published sources on 2026-10-27 (TBC-2–TBC-5, closed by CR-2), and each requirement now cites the verified source.
- Key settled decisions:
  - **Platform:** v1 is fully offline for desktop, web, and CLI (PLAT-3); network play is a planned future enhancement. Targets are macOS Tahoe and later releases, on Intel or any Apple silicon (baseline: M4 with 16 GB), and Safari, Chrome, Firefox, and Edge versions from the last 3 years. Terminal and iTerm2. The CLI and desktop app always ship as one version.
  - **Infrastructure:** one local shared data file; a local web server with a configurable port and no access token (one person per Mac account, ASM-1); Stockfish; English only; no sound effects.
  - **Players:** every Player is identified by a UUID. Names keep their case, are unique (case-sensitive), and are up to 128 user-perceived characters (grapheme clusters, PLY-6). At launch the user picks a profile or registers.
  - **Elo:** every Player, human or computer, has two Elo ratings, Strict and Training, each starting at the Player's category baseline (400 / 600 / 1200 / 2200 / 2500). Both change after rated games on their own scale, for computers as well as humans. K is 40 for the first 30 rated games on each scale (counted separately per scale), then 32; there is no cap. A computer's category label follows its Strict rating, and its engine settings never change.
  - **Modes:** Training is human vs. computer, rated on the Training scale, untimed, with takeback and in-game analysis. Strict is the default: rated on the Strict scale, clocks, no in-game analysis, post-game analysis only.
  - **Games:** each human Player has at most one unfinished game per mode. A computer holds an unfinished-game slot only in a computer–computer (Strict) game, so a computer is never blocked from a human–computer game. Ending a game on purpose means resigning, which is a loss in Elo and W/L/D. An interruption never changes Elo or stats, and the game resumes. Only a computer–computer watcher can abort, and that changes nothing. A reset voids the Player's unfinished games, with no result and no effect on anyone's Elo or stats (amended 2026-10-27, CR-1). Read-only rewind for human Players. Hot-seat human–human. Humans may offer draws in h–h and h–c. Self-play is never rated and is excluded from W/L/D. Clocks are in v1, Strict only. No completed move is lost on any interruption.
  - **Reset:** a reset hides a human Player's past games from that Player's own views only; the games stay in opponents' history and statistics. *(Amended 2026-10-27, CR-1: a reset is allowed while the Player has unfinished games; each is voided, with no result and no effect on anyone's Elo or statistics.)*
- **Changes 2026-10-24 → 2026-10-25:**
  - CLI amended to a full-screen, ratatui-style terminal UI (TUI): keyboard only, moves typed (no board cursor), no mouse, 80×24 minimum, terminal always restored, plain line-oriented mode for screen readers and pipes, scripted runs headless (CLI-1..4, UI-2, UI-4; Q38–Q40).
  - PLAT-3 strengthened: desktop app, web page, and CLI are fully usable offline in v1, including first launch, with no run-time Internet fetches and explicit offline tests.
  - Architecture inputs resolved (Rust backend, TypeScript UI, Tauri, one shared core); all architecture decisions are AGREED in [architecture.md](architecture.md), including the GUI board (chessground + our keyboard/accessibility layer, which delivers GUI-2 and UI-4).
- **Review 2026-10-26:** an independent review against ISO/IEC/IEEE 29148, the INCOSE Guide to Writing Requirements, and architecture decisions D-1–D-11. It added:
  1. Proposed amendments, as bullets headed **Review 2026-10-26 (PROPOSED amendment)**.
  2. New requirements PLAT-4, PLY-6, ENG-4, DAT-4–DAT-6, UI-6–UI-9, ANA-4, CLI-5, GUI-5, NFR-7–NFR-9, and IF-1–IF-5.
  3. A corrected ELO-2 verification, open questions, and a TBD/TBR/TBC register.
- **Decisions 2026-10-26, round 1 (Q41–Q55):**
  - Added: PLY-7, GAME-4, UI-10, UI-11, and section 13 (Risks).
  - Changed: PLAT-2/3/4, PLY-2–6, GAME-1–3, RULE-2/4/5, ENG-2–4, DAT-2/3/5, ELO-1/2, UI-1/3/5/7, ANA-1/3/4, CLI-1/3/4, GUI-2, GUI-5, NFR-1/2/4/8, IF-2/3/4.
- **Decisions 2026-10-26, round 2 (Q56–Q74):**
  - Added: PLAT-5 (one version for CLI and desktop).
  - Training games are now rated on a separate Training Elo scale and count in W/L/D (Q70).
  - The abort rules were replaced: resign is the only deliberate end, and abort exists only for computer–computer watchers (Q60).
  - Changed: PLAT-2, PLAT-4, PLY-2/4/5/6/7, GAME-1/2/4, RULE-2/5, ENG-2, DAT-2/3, ELO-1/2, UI-1/2/3/5/7, ANA-4, CLI-1/4, GUI-2, IF-5, Out of Scope.
- **Decisions 2026-10-26, final round (Q75–Q79) and baseline v1.0:**
  - Added: RULE-6, RULE-7, RULE-8 (split from RULE-5) and ENG-5 (split from ENG-3).
  - Accepted as AGREED, priority Must (Q75): PLAT-4 details, ENG-4, DAT-4, DAT-5, DAT-6, UI-6, UI-8, UI-9, ANA-4, CLI-5, NFR-7, NFR-8, NFR-9, IF-1–IF-5, and every review amendment and TBR detail.
  - Changed: PLAT-1, PLAT-4, PLY-2/4/5/6/7, GAME-1/4, RULE-1/2/3/4/5, ENG-2/3, DAT-2/3/4, ELO-1, UI-1/2/4/5/10, CLI-1, GUI-2, GUI-5, NFR-1/6/7, IF-1/2/5, register.
- Decisions are logged in the Decision Log. From v1.0, changes go through change control (see the baseline statement).

Each requirement has: ID, statement, status, and **Verification** (test cases). Requirements added from 2026-10-26 also carry **Rationale/Source** and **Priority**; every requirement's priority is Must (Q56, Q75). Overall approach is in [test-strategy.md](test-strategy.md).
Test levels: **U** = unit, **I** = integration, **E** = end-to-end, **M** = manual.
Verification method (29148 terms): U, I, and E checks are **Test**. An E check that only observes behaviour, with no measured pass value, is **Demonstration**. M walk-throughs are **Demonstration** or **Inspection**, CI scans and license audits are **Inspection**, and calibration statistics are **Analysis**.

---

## 1. Delivery & Platforms

### PLAT-1 (AGREED; amended 2026-10-26, Q75) Three ways to run
The Game shall be runnable via (a) a web interface, (b) a command line, (c) a desktop application.
- Invoked by: web — running `chess serve`, which starts the local server and opens the page in the default browser, or opening the page's URL while the server runs (PLAT-4); desktop — opening the application; CLI — typing the application name.
  - Original wording (2026-10-24): "opening a URL (web), opening an executable (desktop), typing the application name (CLI)."
  - **AGREED 2026-10-26 (Q75):** the review amendment, which added that the web server must be running first (architecture D-8).
- Desktop packaging (AGREED 2026-10-24): unsigned local build for the user's own Mac; signing/notarization only if distributed later.
  - **Amended 2026-10-28 (CR-4; UG-4):** "unsigned" means no Developer ID signature and no notarization. The build ad-hoc signs `Chess.app` and its executables, because macOS on Apple silicon runs only signed code (fact check TBC-10). A locally built app has no download-quarantine mark and opens normally. An app copied from another Mac or downloaded can be blocked by macOS (Gatekeeper); the user then opens it once, by the steps that NFR-5 documents (PLAT-6). The Game itself adds no check or workaround.
- Install layout (architecture D-18, AGREED 2026-10-26): `Chess.app` is the only artifact. It contains the `chess` CLI and Stockfish, and `chess` is put on `PATH` by a symlink.
- **Verification**
  - E: launch each of the three entry points on a clean machine/profile; each reaches a playable new-game state.
  - E: the same scripted game (list of SAN moves) played in all three ends with identical final position/PGN.
  - M: desktop app launches by double-click on macOS; CLI command is on `PATH` after install.

### PLAT-2 (AGREED; amended 2026-10-26, Q46/Q64) Supported platforms
The desktop app and the CLI shall run on macOS Tahoe (macOS 26) and on every later macOS release that is current at a given app release. They shall run on every Mac that the macOS release supports, Intel or Apple silicon. The web interface shall run, on those Macs, in every major version of Safari, Chrome, Firefox, and Microsoft Edge released in the 3 years before the v1 release date. Older browser versions are not supported. The TUI shall run in the macOS Terminal app and in iTerm2.
- **AGREED 2026-10-26 (Q46, Q64):** Intel or any Apple silicon (M1 and newer; one arm64 build covers all of them), the four browsers, and the two terminals.
  - The 3-year window is measured back from the v1 release date (interpretation, Decision Log).
  - The performance baseline stays an M4 with at least 16 GB RAM (Q47, ENG-3).
- Linux and Windows are out of scope for v1 (Out of Scope list).
- **AGREED 2026-10-26 (Q64, late round 2):** support covers Tahoe and the macOS releases after it.
  - At each app release, the release checklist records the macOS releases that are current that day, from Apple's published release list, and the app is verified on each of them.
  - Intel Macs are supported only on releases that still run on Intel (RISK-1).
- Consequence: Intel support needs x86_64 or universal builds of the `chess` binary, `Chess.app`, and the bundled Stockfish (IF-4). Architecture D-25 (universal arm64 + x86_64 build, including the x86 Stockfish variant) was accepted 2026-10-26.
- Risk: RISK-1 (Tahoe is expected to be the last macOS release that supports Intel).
- **Verification:**
  - Inspection, at each app release: the release checklist lists the current macOS releases (Tahoe and later) and their source.
  - M: smoke test of desktop app and CLI on each listed release, on an Apple silicon Mac. Also on an Intel Mac for each listed release that supports Intel, and on Tahoe on an Intel Mac, an M1 Mac, and an M4 Mac. E: web suite in Safari (WebKit), Chrome, Firefox, and Edge, at the oldest in-window version and at the newest version of each. E: TUI PTY suite. M: TUI check in Terminal and iTerm2.

### PLAT-3 (AGREED; strengthened 2026-10-25; caveat 2026-10-26, Q43) Offline operation
All three entry points — desktop app, web page, and CLI — are fully usable with no Internet connection, in v1. Every feature works offline: registration, all matchups and modes, computer play, clocks, analysis (live and post-game), statistics, themes, settings, import/export, and help.
- **AGREED 2026-10-26 (Q43):** v1 has no network play and makes no non-loopback connection. Network play is a planned future enhancement (Out of Scope for v1). When it is specified, PLAT-3 and NFR-4 will be revised through the Decision Log. Until then this requirement holds without exception.
- Nothing is fetched from the Internet at run time: no CDN scripts, web fonts, remote images, update checks, license checks, or telemetry. The GUI bundle, piece and board themes, fonts, the Stockfish engine (including its neural-network file), and documentation ship with the app.
- The web page is served by the local server (`localhost`), so it needs no Internet; it must also work with Wi-Fi off and no network interface other than loopback.
- First launch after install is also offline: nothing (registration, roster generation, database creation) requires a connection.
- Building from source may download dependencies; running never does.
- **Verification**
  - E: with all network interfaces except loopback disabled, for each of desktop, web (Safari, Chrome, Firefox, Edge), and CLI: first launch on a clean profile → register → full game vs. a computer Player in Strict with a clock → post-game analysis → stats view → PGN export.
  - E: network-capture harness during the above records zero non-loopback connections (shared with NFR-4).
  - U/CI: the built GUI bundle contains no `http(s)://` resource references outside an allow-list (license texts, documentation links that are never fetched); the Content-Security-Policy (`default-src 'self'`) is set by both the local server and the desktop app, so any accidental remote load fails in tests.
  - I: the bundled Stockfish runs with no network and no external network file.

### PLAT-4 (AGREED 2026-10-26, Q55/Q67/Q75) Local web server
The `chess` command shall provide a subcommand that starts the local web server on the port the user selects, prints the page's URL, and opens that URL in the default browser unless the no-open option is given.
- **AGREED 2026-10-26 (Q55, Q67):** the user can set the server port with a command-line option or in a config file. The port setting applies only to `chess serve` and the browser URL. The CLI TUI and the desktop app use no port.
- **AGREED 2026-10-26 (Q75):**
  - The server listens on the loopback interface only (security: NFR-7).
  - The command-line option overrides the config file.
  - If the chosen port is in use, the command exits with a message naming the port and a non-zero status.
- **AGREED 2026-10-26 (architecture D-24 accepted):** the config file is `config.toml` (TOML) in the data directory. When neither the option nor the config file sets a port, the default is 8765. This closes TBD-3 and TBD-29.
- **AGREED 2026-10-26 (ASM-1 confirmed; architecture D-22 accepted):** one person uses each Mac account, so the server needs no access token. Browser-borne requests are stopped by NFR-7. If ASM-1 ever changes (shared Mac accounts, or the app is distributed), PLAT-4 and NFR-7 are reopened through change control.
- **Amended 2026-10-28 (CR-4; UG-15):**
  - **Config errors:** a missing `config.toml` means defaults. A `config.toml` that cannot be read or parsed, or that holds a port outside 1–65535, makes `chess serve` exit with a non-zero status (exit code 1, CLI-5) and a message that names the file, the line, and the reason. It never falls back to defaults silently.
  - **Browser:** if the default browser cannot be opened, the server keeps running and prints the URL with a notice to open it by hand. The exit status is not affected.
  - **Stopping:** the user stops `chess serve` with Ctrl-C (SIGINT) or SIGTERM in its terminal, with no confirmation. The server stops accepting connections, tells connected pages that it has stopped, suspends each open game as an interruption (RULE-5, GAME-4: clock suspended, state recorded, game resumable, no change to Elo or statistics), and exits with status 0. SIGKILL or power loss is an interruption in the same sense (NFR-2).
- **Amended 2026-10-28 (CR-5):** the architecture's `--config <path>` option and `CHESS_CONFIG` variable, which point `chess serve` at another config file, are architect-added for tests. They are not requirements and add no user behaviour beyond the `config.toml` in the data directory.
- **Rationale/Source:** PLAT-1 (web entry point) and DAT-1 (web served locally) never said how the server starts. Architecture D-8 uses `chess serve [--port] [--no-open]`.
- **Priority:** Must.
- **Verification**
  - E: `chess serve` prints an `http://127.0.0.1:<port>/` URL and the page loads in Safari, Chrome, Firefox, and Edge. With `--no-open`, no browser opens.
  - E: a port set by option and a port set in the config file are each used. When both are set, the option wins. When neither is set, the port is 8765.
  - Inspection: the config file is read from `config.toml` in the data directory, as stated in the install and run documentation (NFR-5).
  - I: no connection succeeds on a non-loopback address.
  - E: starting a second server on the same port fails with the message and a non-zero exit status.
  - E (added 2026-10-28, CR-4): a `config.toml` with a syntax error, and one with port 70000, each stop `chess serve` with exit code 1 and a message naming file, line, and reason; no server starts. E: with the browser launcher forced to fail, the server runs and prints the URL and the notice. E: Ctrl-C and SIGTERM stop the server with exit 0, the page shows that the server stopped, and a game open in the page resumes after the server is started again with every move and each clock at its last recorded value.

### PLAT-5 (AGREED 2026-10-26, Q71) One version for all front-ends
The CLI (including `chess serve`) and the desktop app shall be installed and updated together as one application version. Before using the data store, each front-end shall check that the store and any other running front-end belong to its own version. On a mismatch it shall refuse to use the store and show a message naming both versions and telling the user to install the matching version.
- **Rationale/Source:** Q71. The CLI and desktop app must never run at different versions against one store (DAT-1). The web UI is served by `chess serve`, so it always has the CLI's version. Architecture D-16 (one bundle, mismatch refused) and D-18 (install layout) were accepted 2026-10-26.
- Upgrading the store after installing a new version is DAT-6.
- **Priority:** Must.
- **Verification:** Inspection: the install and update instructions (NFR-5) install both together. I: a store marked with another version is refused with the message, and its bytes are unchanged. I: with the CLI at version N running, starting a desktop app at version N+1 is refused with the message.

### PLAT-6 (AGREED 2026-10-28, CR-4; systems engineer's decision, closes UG-4 and FG-8 (a), (b)) Install, update, and uninstall
The Game shall be installed by placing `Chess.app` in a folder the user chooses (default `/Applications`) and running `chess install-cli`, shall be updated by replacing `Chess.app`, and shall be uninstalled by running `chess install-cli --remove` and deleting `Chess.app`. Installing, updating, and uninstalling shall never change or delete the user's data.
- **(a) Source of the bundle.** `Chess.app` is built locally from source by the user (PLAT-1). v1 has no installer package, download service, or app store.
- **(b) Command on `PATH`.** `chess install-cli`, run from the bundle's `chess` binary, shall create a symbolic link named `chess` to that binary in `/usr/local/bin` when that folder exists and is writable, and otherwise in `~/.local/bin` (created if absent). It shall print the link's path and, when that folder is not on `PATH`, the exact line to add to the shell profile. Run again, it replaces a link that it created earlier. It shall refuse to replace any other file or link with that name. If it cannot create the link, it shall say why, change nothing, and leave the CLI usable by its full path (NFR-9).
- **(c) Update.** The user quits every running front-end and any `chess serve`, then replaces `Chess.app` with the new version. If the bundle path is unchanged, the link keeps working; otherwise the user runs `chess install-cli` again. The first launch of the new version upgrades the store after a backup (DAT-6), and PLAT-5 refuses a version mismatch. The Game makes no update check (PLAT-3).
- **(d) Uninstall.** `chess install-cli --remove` removes only a link that it created. The user then deletes `Chess.app`. The data directory stays. NFR-5 documents its location, says that deleting it removes every Player, game, and rating, and advises a full-data export (DAT-2) first. The Game has no command that deletes the user's data.
- **(e) Unsupported macOS.** The bundle declares macOS 26 as its minimum system version, so macOS itself refuses to open it on an earlier release. The Game adds no check of its own.
- **Rationale/Source:** UG-4 and functional_decomp FG-8 (a), (b): NFR-5 and PLAT-5 require install and update to keep one version, and D-18 puts `chess` on `PATH` by a symlink, but no step, failure case, or retirement was defined.
- **Priority:** Must.
- **Verification:** M (clean macOS 26 Mac): copy the bundle to a folder, run `chess install-cli`, and start the desktop app and the CLI. M: `chess install-cli` run twice leaves one link. M: with a regular file named `chess` at the target, it refuses and leaves the file. M: with `/usr/local/bin` absent or not writable, it uses `~/.local/bin` and prints the profile line. M: update from version N to N+1 keeps every Player and game (DAT-6 backup written). M: `chess install-cli --remove` removes the link, and the data directory is untouched. M: a copy of the app that carries the quarantine mark opens by the documented steps. Inspection: the bundle's Info.plist declares macOS 26 as the minimum, and NFR-5 documents the Gatekeeper steps, the data directory, and the deletion warning.

---

## 2. Players & Game Modes

### PLY-1 (AGREED) Player types and modes
Each Player is human or computer. Supported matchups: human–human, human–computer, computer–computer.
- **Constraint (AGREED 2026-10-24):** Training mode supports human–computer only. Human–human and computer–computer games are always Strict mode (see PLY-5).
- **Mode selection (AGREED 2026-10-24):** a human selects the matchup and mode at the start of every game. Computer–computer is a special option that a human must explicitly choose (always Strict); the human then selects the two computer Players, which may be the **same** computer Player playing against itself.
- **Self-play (AGREED 2026-10-24):** in human–human games, the same registered human Player may occupy both sides (playing against themself).
- **Additional verification**
  - E: computer–computer can only be started by a human choosing it explicitly, and requires two computer Player selections.
  - E: same computer Player selected for both sides runs to completion; stats recorded once, correctly attributed.
  - E: same human Player selected for both sides of a human–human game works.
- **Verification**
  - E: for each of 3 modes, play to a terminal result (checkmate scripted for humans; engine games run to completion).
  - U: game controller requests moves from the correct player type per turn.
  - E: computer–computer game completes without any user input and is recorded.

### PLY-2 (AGREED; amended 2026-10-26, Q48/Q70) Player registration and profile selection
At launch, the Game shall let the user pick a profile from the list of registered human Players, or register a new human Player. The user can also register a new Player at any later time.
- **AGREED 2026-10-26 (Q48):** profile choice at launch, as above. This replaces the 2026-10-24 wording "On first use the Game registers a new Player", which covered only the first Player.
- Registration stores the Player and default preferences in the data store.
- **AGREED 2026-10-24:** registration asks the Player to self-assess as **Beginner, Novice, Intermediate, Master, or Grand Master**, and the category is stored with the Player. **Amended 2026-10-26 (Q70):** the category sets both of the Player's starting Elo ratings, Strict and Training (ELO-1).
- With an empty store, the list is empty, so registering is the only choice.
- Archived Players are hidden (PLY-4), so they are not in the list.
- Name and identity rules: PLY-6.
- **AGREED 2026-10-26 (Q75):** scripted headless runs (CLI-4) skip profile selection.
- **Amended 2026-10-28 (CR-4; UG-8, closes assumption A-4):** the **active Player** is the Player whose settings, statistics, and views the front-end shows. Picking a profile at launch, or registering a new Player at launch, makes that Player the active Player. Registering a new Player later does not change the active Player. The new Player can be chosen as a side in new-game setup (PLY-3), or by switching profile. The user can switch the active Player at any time while no game is open in that front-end, by returning to profile selection.
- **Verification**
  - E: fresh data store → first launch prompts for a name and category → Player row, default preferences, and both starting Elo ratings exist.
  - E: registration cannot complete without choosing one of the five categories.
  - E: a later launch shows the profile list. Picking a profile loads that Player without creating a duplicate, and "register new" adds a Player. Archived Players are not listed.
  - U: name validation per PLY-6.
  - E: a Player registered via CLI is visible in web and desktop (given shared store).
  - E: a headless scripted run (CLI-4) completes without showing profile selection.
  - E (added 2026-10-28, CR-4): registering at launch makes the new Player active in each front-end. Registering later from the main screen leaves the active Player unchanged, and the new Player is selectable in new-game setup. Returning to profile selection and picking another profile switches the active Player.

### PLY-3 (AGREED) Player selection / switching
User can choose which registered Player(s) are playing at game start (needed for human–human, and for multiple people on one machine).
- **AGREED 2026-10-24:** in human–human games both sides are registered Players sharing one screen ("hot-seat") for now; both may be the same Player (self-play). Network play is out of scope for v1 and is a planned future enhancement (Q43).
- **Verification:** E: two registered Players selected for h-v-h; each game's stats go to the right Player. E: same Player on both sides is accepted.

### PLY-4 (AGREED; amended 2026-10-26, Q41/Q52/Q57/Q72/Q75/Q79) Computer players are first-class Players
Computer opponents are persisted Players. Each has a stable identity, a **category** (Beginner, Novice, Intermediate, Master, Grand Master — the same five as the human self-assessment), and its own engine parameters (see ENG-2).
- **AGREED 2026-10-26 (Q41, Q70):** when a computer Player is first created, it is put in a category and starts at that category's baseline on both Elo scales (ELO-1). Its ratings then change after rated games, the same way as a human's. This replaces the 2026-10-24 rules that a computer Player's Elo is static and that only human Players' Elo changes.
- **AGREED 2026-10-26 (final round):** a computer Player's Training rating changes after each rated Training game it plays, and its Strict rating after each rated Strict game.
- **AGREED 2026-10-26 (Q57, Q79):** a computer Player's category label follows its current **Strict** rating, using the ELO-1 class bands. Its Training rating never changes the label.
  - The opponent picker and the default-opponent-category setting (UI-1) use the current label.
- **AGREED 2026-10-26 (Q57, Q72):** a computer Player's engine parameters are fixed when it is created and never change. There is no recalibration after release.
- The Game tracks computer Players' statistics (games, W/L/D, etc.) (AGREED 2026-10-24).
- **AGREED 2026-10-24 — roster:** 5 computer Players per category (25 total), generated once from a fixed seed with human-readable names. The user can generate more, choosing the new Players' starting category (AGREED 2026-10-26, Q75). Computer Players cannot be deleted or reset (Q62).
- **AGREED 2026-10-24 — human Player management:** a human Player can be renamed (history follows) and deleted by archiving (hidden; history kept).
  - **AGREED 2026-10-26 (Q52):** archiving always asks for confirmation.
  - Resetting a human Player's data: PLY-7.
- **Amended 2026-10-28 (CR-4; UG-9, UG-10, FG-8 (c)):**
  - **Computer Players** cannot be renamed, archived, deleted, or reset. Their name, identity, and engine parameters are fixed when they are created.
  - **Archiving a human Player** is refused while that Player has any unfinished game, in either mode, as a side of a human–human or human–computer game. The message names each such game and tells the user to finish or resign it. So an archived Player never holds an unfinished-game slot (GAME-4), and no game of an archived Player has to be recovered. The active Player of a front-end cannot archive itself; the user first switches to another profile (PLY-2). There is no un-archive in v1 (Out of Scope). An archived Player's name stays taken (PLY-6), and its history stays visible wherever other Players' history refers to it (UI-5).
  - **Generating more computer Players:** one request creates N Players, all in the category the user chooses. N is chosen by the user from 1 to 25, default 5 (TBD-31, EST). Names come from the roster's name generator, continuing its fixed-seed sequence. The generator skips every name already used by any Player, human or computer, active or archived, so a generated name never collides and the user is never asked to resolve one. Engine parameters follow ENG-2: no two computer Players, including the original 25, share an identical parameter set.
- **Verification:**
  - U: roster generation is deterministic for the seed. U: computer Player identity is stable across runs.
  - I: games vs. the same computer Player update the same record. I: a new computer Player's first rating-history point on each scale equals its category baseline. Its ratings change after rated games as ELO-1 computes, and its engine parameters are byte-identical before and after any number of games.
  - U: when a computer's Strict rating crosses a band floor, its displayed category changes. When only its Training rating crosses a band floor, its displayed category does not change.
  - I: a rated Training game against a computer changes that computer's Training rating and leaves its Strict rating unchanged.
  - E: generating more computer Players asks for the starting category, and each new Player starts at that category's baseline on both scales.
  - E: archiving asks for confirmation, and cancelling leaves the Player active.
  - E (added 2026-10-28, CR-4): archiving a Player who has an unfinished game is refused with a message that names the game, and nothing changes; after the game is resigned, archiving works. No rename, archive, or reset action exists for a computer Player. The active Player has no archive action on itself. E: generating N Players (N = 1, 5, 25) creates N Players in the chosen category with unique names and unique parameter sets, and N = 0 or 26 is rejected. U: with a human named like the next generated name, the generator skips it.

---

### PLY-5 (AGREED 2026-10-24; amended 2026-10-26, Q49/Q70) Play modes: Training vs. Strict
Every game is played in one of two modes, chosen at game start and fixed for the game:

| | **Training mode** | **Strict mode** |
|---|---|---|
| Takeback | Allowed | Never allowed |
| Analysis during play | Full: best moves, evals, what-if from the current position | **None.** Engine analysis is offered only after the game ends |
| Rewind to view past moves (read-only, human only; GAME-3) | Yes | Yes |
| Clocks | Not available (always untimed) | Available (RULE-4) |
| Elo (ELO-1, ELO-2) | Rated on the **Training** Elo scale (AGREED 2026-10-26, Q70) | Rated on the **Strict** Elo scale |
| Counts in W/L/D statistics | Yes (AGREED 2026-10-26, Q70) | Yes |
| Post-game full analysis | Yes | Yes |
| Unfinished games (GAME-4) | At most one per human Player. Computers are never limited | At most one per human Player. A computer: at most one c–c game; h–c games never count |

- This replaces the 2026-10-24 rule "Training: Elo not affected (casual)".
- After every game (either mode), the user is offered full-game analysis (see ANA-3).
- **Matchup availability (AGREED 2026-10-24):** Training = human vs. computer only. Strict = human–computer, human–human, and computer–computer.
- **Default mode (AGREED 2026-10-24):** Strict.
- **Verification**
  - E: Training mode cannot be selected for human–human or computer–computer, in all three front-ends.
  - E: start game in each mode; mode is stored on the game record and cannot change mid-game.
  - E: Training — takeback works, analysis of the current position works, and after the game only the Training ratings change.
  - E: Strict — takeback command/button absent or rejected in all three front-ends; after the game only the Strict ratings change.
  - E: Strict — no analysis control/command is available during play in any front-end; analysis is offered after the game ends (ANA-3).
  - E: Training — no clock can be selected; Strict — clock presets and custom are selectable.
  - U: policy object (mode → allowed actions) table-tested for every action.

### PLY-6 (AGREED 2026-10-26, Q48/Q63) Player identity and name rules
Each Player shall have a permanent unique identifier (a UUID) that is independent of its name, so renaming never changes which records belong to the Player. At registration and on rename, the Game shall reject a human Player name that is empty after trimming whitespace, is longer than 128 user-perceived characters, contains a control or line-break character, or matches another Player's name (compared case-sensitively, after Unicode NFC normalization).
- **AGREED 2026-10-26 (Q63):** Players are identified internally by UUID, not by name. Names are stored and shown with their case kept.
- **AGREED 2026-10-26 (Q48):** a name has up to 128 Unicode characters and is unique.
  - Interpretations (Decision Log): uniqueness is case-sensitive, so "Brian" and "brian" are different names. "Unique" applies against every Player, human or computer, active or archived.
- **AGREED 2026-10-26 (Q75):** characters are counted as user-perceived characters (Unicode extended grapheme clusters, as defined in Unicode Standard Annex #29), so an accented letter or an emoji counts as one.
- **Rationale/Source:** names identify Players in profile selection, hot-seat selection, and statistics, so duplicates would be ambiguous. The UUID keeps history attached through renames (PLY-4) and data import (DAT-2).
- The rejection message states the reason (NFR-9).
- **Amended 2026-10-28 (CR-4; closes FG-9 and UG-19, settles TBD-4):**
  - **Length unit:** user-perceived characters, meaning Unicode extended grapheme clusters (UAX #29), as the Q75 bullet and TBD-4 already stated. The 2026-10-26 statement's "128 Unicode characters" was imprecise and is replaced.
  - **Case:** uniqueness is case-sensitive ("Brian" and "brian" are different names), against every Player, human or computer, active or archived. Architecture §5.5 (`ValidationFailed`: "duplicate case-insensitively among active Players") contradicts this and is corrected by the architect through change control.
  - **Normalization:** the name is trimmed of leading and trailing whitespace before any check and before it is stored. Names are compared after NFC normalization, so two names that look the same but are built differently (one code point "é" against "e" plus a combining accent) are the same name. No other folding is applied (not case, not compatibility forms).
  - **Characters:** control characters (Unicode category Cc) and line or paragraph separators (Zl, Zp) are rejected, because they break the TUI and PGN tag lines.
  - The message says which rule failed: empty, too long, not-allowed character, or already used (it says "already used by an archived Player" when that is the case).
- **Priority:** Must.
- **Verification:** U: table test, for register and for rename, of empty, whitespace-only, 128-character, 129-character, Unicode (combining marks, emoji), exact-duplicate (rejected), and case-variant names ("Brian" vs. "brian", accepted). The displayed name keeps its case. I: renaming keeps the UUID and every game, rating, and statistic. E: the rejection reason is shown in each front-end. *(Added 2026-10-28, CR-4:)* U: a name of 128 user-perceived characters built from 256 code points (combining marks) is accepted and one of 129 is rejected; a name with a tab or newline is rejected; "  Brian " is stored as "Brian"; "Zoë" in NFC and in NFD forms count as the same name (second rejected); a name equal to an archived Player's or a computer Player's name is rejected.

### PLY-7 (AGREED 2026-10-26, Q48/Q62/Q76) Reset a human Player's data
The Game shall let the user reset a human Player's data, and shall carry out the reset only after the user confirms it. A reset clears everything about the Player, as seen in that Player's own views, except its Elo ratings, Strict and Training.
- **AGREED 2026-10-26 (Q62):**
  - A reset clears everything except the Player's Elo ratings. This replaces the round-1 open scope.
  - Computer Players cannot be reset.
  - The 2026-10-24 rule "No re-assessment/reset of Elo" (ELO-1) still holds: a reset never changes either rating.
- **AGREED 2026-10-26 (Q76):** the reset Player's past games stay in their opponents' game history and statistics, unchanged. They are hidden only from the reset Player's own views: that Player's game history, statistics, W/L/D, and saved variations show only games played after the reset.
  - So a reset hides the Player's games; it does not delete them. Architecture D-26 (a `reset_at` point, accepted 2026-10-26) does this.
- Interpretations (Decision Log):
  - "Elo ratings" means each scale's current value and its rated-game count (which sets K, ELO-1, counted per scale); these are kept.
  - Preferences return to their defaults. Rating-history points from before the reset are hidden from the Player's own Elo charts.
  - A new history point at each current value is written on each scale, so the Player's charts and ELO-1's "final = initial + Σ deltas" check start again from it.
- **Amended 2026-10-27 (CR-1, user decision closing OQ-8 / G-20): reset with unfinished games.** The Game shall allow a reset while the Player has unfinished games. When the Player has any, the confirmation prompt shall state that each of them will be voided. On a confirmed reset, the Game shall void each of the Player's unfinished games.
  - User's words: "allow it; the unfinished game does not count against either player."
  - A **voided** game has no result. It changes no Elo rating, rated-game count, W/L/D total, or other statistic of any Player in it: the reset Player, a human opponent, or a computer Player. Voiding frees the reset Player's unfinished-game slot in that game's mode (GAME-4).
  - Voiding is not a resignation, so the resignation-is-a-loss rule (RULE-2, GAME-4, Q60) does not apply.
  - Interpretations (Decision Log, 2026-10-27; for the user to correct):
    - A voided game stays in the opponent's game history, marked "voided by reset", with no result. It can be analysed, like a watcher-aborted c–c game (ANA-4). It is hidden from the reset Player's own views (Q76), because it started before the reset.
    - A voided game is no longer unfinished, so it also frees the human opponent's slot in that mode, and it can never be resumed.
    - A voided c–c game cannot occur: only human Players can be reset (Q62), and humans never play in c–c games.
    - In a self-play h–h game (the reset Player on both sides), the voided game is hidden entirely, because both sides are the reset Player.
    - A game that is open for play in a front-end process (DAT-4) is voided too. That front-end stops the game within 2 s (EST, TBD-30; the architecture's lease heartbeat) and tells its user that the game was voided by a reset. A move completed there in that short window is still stored (NFR-2), and the game stays voided (architecture D-30).
- This replaces the architect's rule "a reset is refused while the Player has an unfinished game" (architecture OQ-8, G-20, D-26), which no requirement stated.
- **Rationale/Source:** Q48, Q62, Q76; user decision 2026-10-27 (CR-1).
- **Priority:** Must.
- **Verification:** E: reset asks for confirmation, and cancelling changes nothing. I: after a confirmed reset, both current ratings and rated-game counts are unchanged; the Player's own history, statistics, and Elo charts show no game from before the reset; preferences are back to their defaults; the next rated game on each scale uses the kept K for that scale. I: after the reset, each opponent's (human or computer) game history, W/L/D, and ratings still include every game against the reset Player, unchanged. U: no reset action exists for computer Players.
  - **Added 2026-10-27 (CR-1):**
  - E: with an unfinished game, the reset prompt states that the game will be voided. Cancelling leaves the game unfinished and resumable, with nothing changed.
  - I: reset a Player who has an unfinished Strict h–h game. Afterwards the opponent's Strict and Training ratings, rated-game counts, and W/L/D are unchanged; the game appears in the opponent's history marked "voided by reset", with no result, and can be opened in analysis (ANA-4); the game does not appear in the reset Player's views; the opponent can start a new Strict game.
  - I: reset a Player who has an unfinished h–c game (one case in Training, one in Strict). Afterwards the computer Player's ratings on both scales, rated-game counts, and W/L/D are unchanged.
  - E: immediately after a reset that voided an unfinished game, the reset Player starts a new game in that mode, and setup offers no resume or resign.
  - I: a Player with an unfinished game in each mode is reset; both games are voided.
  - U: voiding records no result and no loss (it is not a resignation), and its termination reason is "voided by reset".
  - I: while one of the Player's unfinished games is open in another front-end process, a confirmed reset voids it; within 2 s (EST) that front-end stops the game and shows that it was voided by a reset; every completed move is stored, and the game is not resumable.

### GAME-1 (AGREED 2026-10-24) New-game setup
In all three front-ends, starting a game collects: matchup (h–h, h–c, c–c), mode (Training/Strict, default Strict), Players for each side (registered humans / computer Players, per PLY-1/3/4), color assignment (White / Black / random), clock (Strict only: preset, custom, or untimed — RULE-4; Training is always untimed), and optionally a starting position (RULE-8). Invalid combinations (e.g., Training for h–h) are not selectable.
- **AGREED 2026-10-26 (Q49, Q60, Q61, Q77):** setup cannot start a game in a mode where a chosen human Player already has an unfinished game, or a computer–computer game where a chosen computer Player already has an unfinished computer–computer game (GAME-4). Instead it offers to resume that game, or to resign it (a human's game), or to abort it (a computer–computer game). A computer Player can always be chosen for a human–computer game.
- **Amended 2026-10-27 (CR-1):** a reset of the human Player (PLY-7) also frees the slot, by voiding the unfinished game. Reset is not one of setup's choices.
- **Amended 2026-10-28 (CR-4; UG-7):**
  - **Resume scope:** the resume offer in setup and the list of games to resume at launch show every unfinished game in which the active Player is a Player on either side (including the second side of a hot-seat game; a self-play game is listed once), and every unfinished computer–computer game, because computers belong to no profile.
  - **Resigning from the offer:** the offer to resign a blocking game works on the game that is listed. For a hot-seat human–human game, setup asks which side resigns, as RULE-2 does, and then asks for the confirmation. Either human at the shared screen may answer. A self-play game is resigned with no effect on Elo or W/L/D, because it is unrated.
- **Verification:** E: each combination produces the expected stored game header; E: invalid combinations are unselectable/rejected in CLI, web, desktop; U: setup validation table-tested; E: with an unfinished game for a chosen human Player in the chosen mode, setup offers resume, resign, or abort, and starts nothing else; E: a computer Player with unfinished games (h–c and c–c) can still be chosen as the opponent in a new h–c game. *(Added 2026-10-28, CR-4:)* E: the resume list for the second side of a hot-seat game shows that game; an unfinished c–c game shows for every active Player; a self-play game shows once. E: resigning a hot-seat game from the setup offer asks which side resigns and then for confirmation, and that side gets the loss.

### GAME-2 (AGREED; amended 2026-10-26, Q49/Q50/Q59) Watching computer–computer games
A human watching a computer–computer game can pause/resume, set the move delay (or step one move), scroll the move list, and abort.
- **AGREED 2026-10-26 (Q50):** during play the watcher can only scroll the move list. There is no rewind of the board. After the game ends, the human can open the full-game analysis (ANA-3).
- Move delay: 0–10 s in 0.5 s steps, default 1 s (EST).
- **AGREED 2026-10-26 (Q59):**
  - Abort asks for confirmation (CLI-4).
  - An aborted c–c game changes neither computer's Elo or statistics.
  - It stays in game history, marked aborted, and can be analysed (ANA-4).
  - Abort exists only here; human games end on purpose only by resigning (RULE-2, Q60).
- **Verification:** E: pause stops move/clock progress. Step advances exactly one move. The watch screen has no rewind control, and the move list scrolls. Abort asks for confirmation and leaves a record marked aborted, with both computers' ratings and statistics unchanged. Analysis is offered after the game ends.

### GAME-3 (AGREED 2026-10-24) Rewind to view past moves
During play, a human Player can rewind the game to view past positions, read-only, and then return ("live") to the current state of the game. Available in both modes. Computer Players cannot rewind. No move can be made, and nothing in the game record can be altered, while rewound.
- **AGREED 2026-10-24:** the clock keeps running while rewound (viewing is part of thinking time); if the opponent's computer move arrives while rewound, the view stays where it is with a clear "live position has changed" indicator. In hot-seat human–human games either human may rewind.
- **AGREED 2026-10-26 (Q50):** a human watching a computer–computer game is not a Player in it and cannot rewind (GAME-2).
- **Verification:** E: rewind k moves then return → position, move list, and clocks identical to never having rewound (apart from elapsed time); E: attempting a move while rewound is rejected; E: stored game record is unchanged by rewinding; E: clock keeps running while rewound; E: computer move arriving while rewound leaves the view in place with the indicator, and "live" shows the new position; E: all of the above in all three front-ends.

### GAME-4 (AGREED 2026-10-26, Q49/Q60/Q61/Q77) Unfinished games, interruptions, and deliberate ends
Each human Player shall have at most one unfinished game per play mode (one Training, one Strict). While a human Player has an unfinished game in a mode, no other game in that mode can start with that Player until the unfinished game is finished, resigned, or voided by a reset of one of its human Players (PLY-7; amended 2026-10-27, CR-1).

Each computer Player shall have at most one unfinished computer–computer game. While it has one, no other computer–computer game can start with that computer Player until the unfinished one is finished or aborted by the watcher. Human–computer games never count toward a computer Player's limit, so a computer Player can always be chosen for a new human–computer game, whatever unfinished games it has.
- **AGREED 2026-10-26 (Q61):** the limit is per Player, not per app.
  - Computer–computer games count toward the Strict limit (c–c games are always Strict). Interpretation: they count for each computer Player in the game.
- **AGREED 2026-10-26 (Q77):** computers are never blocked by unfinished human–computer games. Any human Player can always play any computer Player, and several humans can each have an unfinished game against the same computer at the same time. The one-unfinished-game-per-mode limit applies to each human Player, and to a computer Player only in c–c games.
  - This replaces the round-2 interpretation that the limit also applied to a computer Player in an unfinished h–c game. That interpretation was wrong and is withdrawn.
  - Architecture D-27 (accepted 2026-10-26) enforces this: humans hold a slot in every matchup; each computer holds a Strict slot only in c–c.
- **AGREED 2026-10-26 (Q49, Q60):**
  - **Interruptions:** an interruption the Player did not choose leaves the game unfinished. Examples are a crash, a failure, power loss, a closed app or tab, or quitting. The game is kept and resumes on restart (RULE-5), and no Player's Elo or statistics change.
  - **Resignation:** a deliberate end by a Player is a resignation (RULE-2). It counts as a loss for that Player, in Elo on the game's scale and in W/L/D.
  - **No abort in human games:** human games have no abort; the only deliberate end is resigning. Abort exists only for a computer–computer watcher (GAME-2). Interpretation, Decision Log. *(Amended 2026-10-27, CR-1: a reset also takes a game out of play, by voiding it with no result; see below.)*
  - **Analysis:** resigned and aborted games stay in history and can be analysed (ANA-4).
  - **Voiding by reset (amended 2026-10-27, CR-1):** a reset of a human Player voids that Player's unfinished games (PLY-7). A voided game has no result and changes no Player's Elo or statistics. It is not a resignation and not an interruption.
  - This replaces the round-1 rule "aborting a Strict game counts as a loss for Elo" and "abort is available in every matchup".
- **Amended 2026-10-28 (CR-4):** a human Player who has an unfinished game cannot be archived (PLY-4), so an archived Player never holds a slot. The games a front-end offers to resume are listed in GAME-1.
- **Rationale/Source:** Q49, Q60, Q61; RULE-5; GAME-2.
- **Priority:** Must.
- **Verification:**
  - E: a human Player with an unfinished Strict game cannot start a second Strict game, and setup offers resume or resign, while a Training game can still start; and the reverse.
  - E: a different human can start a Strict game while the first human's Strict game is unfinished, as long as no human in the new game has an unfinished Strict game.
  - E: two different humans each start an unfinished h–c game, in the same mode, against the same computer Player; both games start and both resume correctly.
  - E: a computer Player in an unfinished c–c game cannot be chosen for a new c–c game (setup offers resume or abort), and can still be chosen for a new h–c game in either mode.
  - E: a computer Player with any number of unfinished h–c games can still be chosen for a new c–c game.
  - I: kill the app mid-game, then relaunch → the game resumes, and no rating or statistic changed.
  - U: no abort command exists in h–c or h–h games.
  - I (added 2026-10-27, CR-1): after a reset voids a human Player's unfinished game, that Player's slot in that mode is free, and so is a human opponent's.

---

## 3. Chess Rules & Game Flow

### RULE-1 (AGREED; amended 2026-10-26, Q75) Full FIDE rules
The Game shall enforce the FIDE Laws of Chess (edition effective 1 January 2023, approved by the FIDE General Assembly on 7 August 2022) for legal moves and game termination, with the deviations stated in RULE-2 and the limitation stated in (c). This covers castling, en passant, promotion (choice of piece), check, checkmate, stalemate, the fifty-move rule, threefold repetition, and insufficient material.
- Original wording (2026-10-24): "Legal-move enforcement incl. castling, en passant, promotion (choice of piece), check/checkmate/stalemate, fifty-move rule, threefold repetition, insufficient material."
- **AGREED 2026-10-26 (Q75), review amendments (a)–(c):**
  - (a) The statement above, with a subject and "shall".
  - (b) **Deliberate deviation from FIDE:** under the FIDE Laws, threefold repetition and the fifty-move rule are draws a player must claim, and only fivefold repetition and the 75-move rule end the game automatically. The Game ends the game automatically at threefold repetition and at the fifty-move rule, with no claim step (RULE-2).
  - (c) FIDE's draw rule is "dead position", of which insufficient material is one case. The Game detects the dead-position draw only for insufficient material (positions where neither side can checkmate by any legal sequence of moves because of the material left).
- **Amended 2026-10-27 (CR-2, TBC-2 closed): verified source.** FIDE Laws of Chess, effective 1 January 2023, https://handbook.fide.com/chapter/E012023. Checked against the text:
  - Art. 5.2.2: a dead position ("neither player can checkmate the opponent's king with any series of legal moves") ends the game automatically.
  - Art. 9.2 (threefold repetition) and Art. 9.3 (fifty-move rule) give a draw only "upon a correct claim by a player having the move".
  - Art. 9.6: fivefold repetition and the 75-move rule are automatic draws. Under Art. 9.6.2, a checkmate on the final move takes precedence.
  - So (b) is correct: the automatic draw at threefold repetition and at the fifty-move rule is a deliberate deviation from Art. 9.2 and 9.3.
  - **(c) is a limitation relative to Art. 5.2.2.** Dead positions that are not insufficient material (for example, a fully blocked pawn structure) are not detected. Such a game continues until another rule ends it, for example the automatic threefold-repetition or fifty-move draw, a draw agreed on offer, or resignation.
  - Interpretation (Claude, Decision Log 2026-10-27; for the user to correct): when the move that completes the fifty-move count gives checkmate, the checkmate stands, as Art. 9.6.2 rules for the 75-move rule. A position cannot repeat a third time with a checkmating move, so threefold repetition needs no such rule.
- **Release checklist (not a requirement):** the FIDE Council on 23 August 2026 agreed to keep discussing a draft revision of the Laws. At each app release, the release checklist records whether a newer FIDE Laws edition is in force. If one is, the change is raised as a change request against RULE-1, RULE-2, and RULE-4.
- **Verification**
  - U: perft test — move-generation node counts match published values for standard positions (start, Kiwipete, etc.) to depth ≥ 4.
  - U: table-driven tests of each special rule and each draw/termination condition (FEN in → expected legal moves / result out), including automatic ends at threefold repetition and at the fifty-move rule, and an insufficient-material draw.
  - U: illegal moves (into check, pinned piece, castling through check, wrong turn) rejected with a reason.
  - U (added 2026-10-27, CR-2): a move that completes the fifty-move count and gives checkmate ends the game as checkmate. A blocked-pawn dead position without insufficient material is not declared drawn.
  - Inspection, at each app release: the release checklist records whether a newer FIDE Laws edition is in force (release-checklist item above). *(The one-time source check TBC-2 was closed on 2026-10-27; see the verified source above.)*

### RULE-2 (AGREED; amended 2026-10-26, Q42/Q52/Q60) Game end actions
Resign, draw offers, automatic result detection. Result recorded as 1-0, 0-1, ½-½ with termination reason.
- **AGREED 2026-10-26 (Q42):** a human may offer a draw in human–human and human–computer games, in either mode.
  - In human–human games the other human accepts or declines.
  - A computer never offers a draw.
  - This replaces the 2026-10-24 wording "offer/accept draw (human–human)".
- **AGREED 2026-10-24 (unchanged):** a computer never resigns. It accepts a human's draw offer only if its own evaluation, from its own side, is ≤ 0. Threefold repetition and the fifty-move rule end the game automatically (no claim step). This is a deliberate deviation from the FIDE Laws, Art. 9.2 and 9.3 (RULE-1 (b); verified 2026-10-27, CR-2).
- **AGREED 2026-10-26 (Q52, Q60):** resigning is the only way for a Player to end a game on purpose.
  - **Amended 2026-10-27 (CR-1):** resigning is the only deliberate end that gives a result. A reset (PLY-7) voids the Player's unfinished games instead: a voided game has no result and is not a resignation.
  - It always asks for confirmation.
  - It counts as a loss for the resigning Player in Elo (on the game's scale) and in W/L/D (GAME-4).
  - In a hot-seat human–human game, the Game asks which side is resigning (interpretation, Decision Log).
- **Amended 2026-10-28 (CR-4; UG-6): draw-offer rules.**
  - **When:** a human can offer a draw only on their own turn, before moving, and not while a search or another offer is pending. In a hot-seat game, the human to move is the offerer.
  - **Offer to a human:** it stays pending until that human accepts or declines. There is no time-out. No move can be made while it is pending.
  - **Offer to a computer:** the computer answers within the ENG-3 maximum think time, by its own evaluation (accept only if its evaluation from its own side is ≤ 0). The thinking indicator shows meanwhile. The offerer cannot withdraw the offer. If the engine fails before it answers, the offer is dropped and ENG-4 applies.
  - **Repeat offers:** after a decline, the same side can offer again only after completing another move. There is no other limit.
  - **Clocks:** every clock stops while an offer is pending and restarts, with the same side to move, when the offer is declined.
  - **Acceptance:** the game ends 1/2–1/2 with termination reason "draw agreed".
  - **Amended 2026-10-28 (CR-5; architecture R-20, accepted consequence):** in a hot-seat game, an offer that the other human leaves unanswered stops both clocks with no time-out, so a player can pause a timed game that way. This is the rule as stated (offer to a human: no time-out, clocks stopped). No requirement changes; the two people share one screen, and the other human can answer at once. A suspend (RULE-5) keeps the offer to a human pending and drops an offer to a computer.
- **Verification:** U/E: each termination path yields correct result + reason in the stored record. E: with a fake engine, a human's draw offer in an h–c game is accepted at computer eval ≤ 0 and declined at eval > 0. U: no draw offer ever originates from a computer. E: resign asks for confirmation, and cancelling continues the game. E: in hot-seat h–h, resign asks which side resigns, and that side gets the loss in Elo and W/L/D. *(Added 2026-10-28, CR-4:)* E: a draw offer is refused on the opponent's turn, during a search, and during a pending offer. E: an offer to a human blocks moves until answered, and both clocks stay stopped while it is pending. E: an offer to a computer is answered within the ENG-3 maximum; after a decline, a second offer is refused until the offerer has moved. E: acceptance gives 1/2–1/2 "draw agreed". I: with a fake engine that fails, the pending offer is dropped and the engine-failure message appears.

### RULE-3 (AGREED 2026-10-24, including details) Takeback
Takeback is allowed in Training mode only (human vs. computer); never in Strict mode (PLY-5). Takeback undoes the human's last move and the computer's reply, back to the human's turn; may be repeated to the start of the game (detail AGREED 2026-10-24 with the requirement).
- **Amended 2026-10-28 (CR-4; UG-5):**
  - **Earliest position:** the first position in the game at which it was the human's turn: the start position when the human has White (from a custom start, the position given), or the position after the computer's first move when the human has Black. The computer's first move is never taken back.
  - **During a search:** if the computer is still thinking after the human's move, takeback cancels the search and undoes only that human move, returning the human's turn from before it. Takeback after the computer's reply undoes both moves, as stated above.
  - **Record:** taken-back moves are removed from the game. The stored move list, the PGN export, and the per-move times and evaluations contain only the moves in force. The game continues from the position reached.
  - **Nothing to undo:** the command is rejected with the message "Nothing to take back" (NFR-9), and the game is unchanged.
  - **Rating:** takeback leaves the game rated on the Training scale (ELO-2).
- **Verification:** U: position/move list after undo equals the earlier state; E: unavailable in Strict mode (see PLY-5). *(Added 2026-10-28, CR-4:)* E: as Black, the first takeback after one pair of moves returns to the position after the computer's first move, and the next takeback is rejected with "Nothing to take back". E: takeback during a search cancels it and removes one move. E: after takeback and a different move, the stored move list and PGN hold only the moves in force.

### RULE-4 (AGREED; amended 2026-10-26, Q75) Clocks (optional)
Optional time controls (untimed default). The side whose flag falls loses, unless the opponent cannot checkmate by any legal sequence of moves, in which case the game is drawn.
- Original wording (2026-10-24): "Flag on timeout loses unless insufficient material." **AGREED 2026-10-26 (Q75):** review amendment (a), which says whose material counts (aligned with RULE-1 (c)).
- **AGREED 2026-10-24: in v1.**
- **AGREED 2026-10-24:** presets — untimed, 1+0, 3+2, 5+0, 10+5, 15+10, 30+0 — plus a custom option (base time + increment). **Clocks are available in Strict mode only** (any matchup: h–c, h–h, c–c); Training mode is always untimed (decided later than the preset list).
- **AGREED 2026-10-26 (Q53):** the clock keeps running while the engine is unavailable (ENG-4).
- **Verification:** U: clock logic with a fake time source (increments, pause/resume, flag fall); E: timeout ends game with correct result, including a draw when the flag falls and the opponent has only a bare king; E: clock state survives save/resume (RULE-5); E: a computer Player does not lose on time in 20 games at each preset (EST, Q53; TBD-6), and each of its moves takes no longer than its clock-derived budget.
  - **AGREED 2026-10-26 (Q75):** review amendment (b), which replaced the 2026-10-24 check "computer Player respects the clock" with the pass criterion above.

### RULE-5 (AGREED; amended 2026-10-26, Q49/Q54/Q66; split Q75) Auto-save and resume
The Game shall save every game automatically as it is played, so that an unfinished game can be resumed.
- Original wording (2026-10-24): "Games auto-saved so an unfinished game can be resumed; PGN export for any game; PGN/FEN import." **AGREED 2026-10-26 (Q75):** the compound statement is split. RULE-5 keeps auto-save and resume; PGN export is RULE-6, PGN import is RULE-7, and FEN input is RULE-8. File formats are IF-1–IF-3.
- **AGREED 2026-10-24:** clocks are paused while the app is closed. The 2026-10-24 rules "a game from a non-standard FEN is always unrated" and "imported PGNs are for review/analysis only and never affect Elo or stats" moved with the split to RULE-8 and RULE-7.
- **AGREED 2026-10-26 (Q54):** after any interruption, the resumed game shall contain every move completed before the interruption, and each clock shall be restored to its last recorded value. Interruptions include normal close, disconnect, crash, process kill, hard reset, and power loss. An interruption never changes Elo or statistics (GAME-4).
- **AGREED 2026-10-26 (Q66; architecture D-14):** when the browser tab closes or disconnects, the game's clock is suspended and the game's state is recorded, as when the app closes. This applies even if `chess serve` keeps running.
- Clock recording frequency (EST): at every completed move, and at least once per second while a clock runs. "Last recorded value" only limits the time lost or regained by an interruption if values are recorded often. This matters in rated games, where a forced interruption should not buy thinking time.
- One unfinished game per mode per human Player (a computer only in c–c games): GAME-4. Durability: NFR-2.
- **Amended 2026-10-27 (CR-1):** a game voided by a reset (PLY-7) is no longer unfinished and is never resumed. Its completed moves stay stored.
- Durability of moves is designed by architecture D-23 (per-game move journal, accepted 2026-10-26).
- **Verification:** E: kill the app mid-game (for web, also kill the server) → relaunch → the game resumes at the same position with every completed move, and each clock equals its last recorded value. E: closing the browser tab suspends the clock; reopening resumes from the recorded value. *(The PGN round-trip and malformed-input checks moved to RULE-6–RULE-8 with the split.)*

### RULE-6 (AGREED 2026-10-26, Q75; split from RULE-5) PGN export of a game
The Game shall let the user export any stored game as a PGN file in the IF-1 format.
- **Amended 2026-10-28 (CR-4; UG-11):** "any stored game" includes an unfinished game, a resigned, aborted, voided, imported, or custom-start game. IF-1 gives each kind's tags.
- **Rationale/Source:** RULE-5 (2026-10-24, "PGN export for any game"), split at baseline v1.0 (Q75).
- **Priority:** Must.
- **Verification:** U: PGN round-trip (export → import → identical move list). E: export works from the game history list in CLI, web, and desktop.

### RULE-7 (AGREED 2026-10-26, Q75; split from RULE-5) PGN import
The Game shall let the user import games from a PGN file in the IF-2 format. Imported games shall be for review and analysis only: they never change any Elo rating or statistic.
- **AGREED 2026-10-24 (moved from RULE-5):** imported PGNs are for review/analysis only and never affect Elo or stats.
- **Rationale/Source:** RULE-5 (2026-10-24, "PGN/FEN import"), split at baseline v1.0 (Q75).
- **Priority:** Must.
- **Verification:** U: malformed PGN is rejected with a message, and valid games in the same file are still imported (IF-2). I: importing games changes no rating, rating history, or W/L/D statistic.

### RULE-8 (AGREED 2026-10-26, Q75; split from RULE-5) FEN starting positions
The Game shall let the user start a game, or open an analysis, from a position given as FEN in the IF-3 format. A game started from a non-standard position shall always be unrated.
- **AGREED 2026-10-24 (moved from RULE-5):** a game from a non-standard FEN is always unrated.
- **Rationale/Source:** RULE-5 (2026-10-24, "PGN/FEN import"), split at baseline v1.0 (Q75); GAME-1 (optional starting position).
- **Priority:** Must.
- **Verification:** U: malformed or illegal FEN is rejected with a message (IF-3). E: a game started from a custom FEN plays to a result and changes no rating on either scale.

---

## 4. Engine

### ENG-1 (AGREED) Open-source engine
The Game shall use an open-source chess engine for computer play and analysis.
- **AGREED 2026-10-24: Stockfish** (GPLv3, UCI protocol; native binary and WASM build). Consequence: the Game's source and any distributed builds must be GPL-compatible, with Stockfish attribution/source offer.
- **Verification**
  - I: engine process starts, answers `uci`/`isready`, returns legal `bestmove` from standard positions.
  - I: engine failure (crash, missing binary, timeout) surfaces a user-visible error and the game remains recoverable.
  - U: license file/attribution present in distributed artifacts.
- **Review 2026-10-26 (note):**
  - The WASM build is not used. Stockfish runs only as a bundled native child process of the Rust core (architecture §2, D-9). The pinned-engine constraint is now IF-4.
  - The engine-failure behaviour in the verification is now requirement ENG-4.
  - The project license is GPL-3.0-or-later (D-10).
- **Amended 2026-10-27 (CR-3):** the pinned release is Stockfish 19 (IF-4). Its release notes say "GPL"; the "GPLv3" in the 2026-10-24 wording is confirmed from the shipped `COPYING` file (TBC-9), and the license audit (NFR-6) records it.

### ENG-2 (AGREED; amended 2026-10-26, Q41/Q53/Q57/Q72) Difficulty levels
User can configure difficulty by choosing a computer Player (PLY-4). Computer Players come in five **categories** — Beginner, Novice, Intermediate, Master, Grand Master.
- **AGREED 2026-10-24:** members of a category have similar but not identical play parameters, so no two computer Players are exactly alike. Parameters are generated within category bounds (engine `Skill Level`, depth/movetime limits, `UCI_Elo` where in range, and a small random-error rate; accepted with the requirement on 2026-10-24), then stored with the Player. The criterion is that no two Players share an identical parameter set.
- **AGREED 2026-10-26 (Q57, Q72):** a computer Player's parameters never change after it is created. Calibration happens before release only; there is no recalibration after release.
- **AGREED 2026-10-26 (Q41, Q70):** each computer Player starts at its category's baseline Elo (400 / 600 / 1200 / 2200 / 2500) on both scales. Its ratings then change after rated games (ELO-1). This replaces "each computer Player's static Elo lies within its class's Elo band; Grand Master computers span 2500–3000" (2026-10-24).
- **AGREED 2026-10-24:** Beginner and Novice computer Players (400 / 600 baselines) are below the range of Stockfish's built-in strength limiter, so they use a low skill level, shallow depth, and random errors. All categories are calibrated empirically by playing matches before release (see verification).
  - **Amended 2026-10-27 (CR-3):** no `UCI_Elo` floor value is assumed. Any category whose baseline is below the lowest `UCI_Elo` value that the pinned Stockfish 19 accepts (TBC-8) uses the low-skill method above, and `UCI_Elo` is used only for values inside that engine's reported range. The 2026-10-24 statement expects Beginner and Novice to be in this group; TBC-8 confirms which categories are.
  - Calibration is run against the pinned release (IF-4). A later Stockfish release is pinned only after a Decision Log entry and a new calibration run.
- **Calibration criteria (EST, Q53):**
  - Each category's parameter bounds are tuned so its measured strength is within ±150 Elo of its baseline.
  - Each higher category scores more than 50 % against each lower one, at one-sided significance p < 0.05, over ≥ 100 games per pairing.
  - Calibration runs on an isolated store, never the user's (RISK-2).
- **Calibration verification:** E (slow, pre-release): matches against reference opponents and between categories give measured Elo estimates within ±150 (EST) of each category's baseline.
- **Verification**
  - U: generated Players in a category stay within the category's parameter bounds; no two share an identical parameter set.
  - I (added 2026-10-27, CR-3): every generated `UCI_Elo` value lies inside the range that the pinned engine reports for the `UCI_Elo` option in its `uci` reply (TBC-8).
  - I: generated parameter sets persist and reload unchanged, including after rated games.
  - E (slow, nightly): round-robin matches between categories (≥ 100 games per pairing, SPRT or binomial test). Each higher category scores more than 50 % at p < 0.05 (EST), which shows strength rises with category.
  - I: each level produces legal moves within its time budget.
  - U: the category baselines increase strictly (400 < 600 < 1200 < 2200 < 2500).

### ENG-3 (AGREED; amended 2026-10-26, Q47/Q53; split Q75) Computer think time
Each computer move shall take no longer than the configured maximum think time (default ≤ 3 s at the top category on the baseline Mac).
- Original wording (2026-10-24): "Computer move ≤ configured max think time (default ≤ 3 s at the top category on the baseline Mac); UI never blocks while engine thinks (quantified in NFR-8); user can cancel." **AGREED 2026-10-26 (Q75):** the compound statement is split. ENG-3 keeps the think-time limit; the non-blocking UI is NFR-8; cancel is ENG-5.
- **AGREED 2026-10-26 (Q47):** the baseline Mac is an Apple M4 with at least 16 GB RAM running macOS Tahoe. NFR-1 and NFR-8 use the same baseline.
- **AGREED 2026-10-26 (Q69, interpretation):** whose maximum-think-time setting (UI-1) applies:
  - In c–c games, the setting of the active (watching) profile applies to both computers.
  - In h–h games no computer moves, so the setting affects play not at all. It applies only to engine analysis, where relevant (post-game analysis, ANA-3), as an upper limit on search time per position.
  - In h–c games, the human Player's setting applies (UI-1, 2026-10-24).
- **Verification:** I: c–c game with the watching profile's maximum set below the computers' own limits → no computer move exceeds it; I: h–h post-game analysis per position stops at the active profile's maximum when it is below the ANA-1 limit; I: measured move latency under limit. *(The checks "UI remains interactive during engine think" and "cancel → move within 200 ms" moved to NFR-8 and ENG-5 with the split.)*

### ENG-4 (AGREED 2026-10-26, Q53/Q75) Engine failure handling
If the engine crashes, cannot be started, does not answer a readiness check within 5 s (EST), or does not return a move within 2 s after the move's time limit (EST), then the Game shall keep the game resumable with every move already made preserved, and shall show a message that names the failure and offers retry or exit.
- **AGREED 2026-10-26 (Q53):** the clock keeps running while the engine is unavailable.
- An engine failure is an interruption, not a deliberate end: it never changes Elo or statistics (GAME-4, Q60).
- **AGREED 2026-10-26 (Q75):** the failure-handling statement above. Its timeouts are EST (TBD-13).
- **Rationale/Source:** the ENG-1 verification ("engine failure … surfaces a user-visible error and the game remains recoverable") had no matching statement. Architecture §4.3 (`EngineError`, retry) and §4.7 (`EngineUnavailable`, restart) assume this behaviour.
- **Priority:** Must.
- **Verification:** I: with a fake engine that crashes, hangs, or is missing, the error event is raised within the stated times. Retry restarts the engine and play continues, the stored game is unchanged, and the clock kept running. E: the message and the retry/exit choices appear in the TUI, plain mode, web, and desktop.

### ENG-5 (AGREED 2026-10-26, Q53/Q75; split from ENG-3) Cancel a computer's search
When the user cancels a computer Player's search, the Game shall have that computer play the best move found so far within 200 ms (EST, TBD-12).
- **Amended 2026-10-28 (CR-4; UG-3): the "move now" controls.**
  - TUI and plain mode: the command `movenow` (CLI-4). The TUI also binds key F2, which can never begin a move.
  - GUI (web and desktop): a "Move now" button in the engine thinking indicator (GUI-4), reachable with Tab and operated with Enter or Space, and key F2.
  - The control is available only while a computer is thinking in a human–computer game. In a computer–computer game it is shown as unavailable, and the watcher uses Pause (GAME-2). Outside those times it is shown as unavailable (CLI-4).
  - It affects only the computer's move search. Stopping an analysis search is part of analysis (ANA-1).
- **Rationale/Source:** ENG-3 (2026-10-24, "user can cancel"); value EST under Q53; split at baseline v1.0 (Q75).
- **Priority:** Must.
- **Verification:** I: cancel during a long search → a legal move is played within 200 ms (EST) on the baseline Mac. E: the cancel control or key exists in each front-end while a computer is thinking. *(Added 2026-10-28, CR-4:)* E: `movenow`, F2, and the "Move now" button each cause the move within 200 ms (EST) in the front-ends that have them; each is shown as unavailable on the human's turn and in a c–c game, and pressing it then does nothing but explain why.

---

## 5. Persistence & Data

### DAT-1 (AGREED) Central data store
A single central data store persists data across runs: Players, preferences, games, statistics, Elo history.
- **AGREED 2026-10-24:** one local, shared data file on the user's Mac (e.g., SQLite) used by CLI, desktop, and a locally-run web server (`localhost`). No hosted/multi-user service in v1. Specific technology is an architecture-phase choice.
- Consequence: web interface is served locally (not pure-browser storage), so CLI/desktop/web see the same Players, games, and Elo.
- Version mismatch between front-ends: PLAT-5 (AGREED 2026-10-26, Q71).
- **Verification**
  - I: data written in one run is present after process restart.
  - I: all three clients read/write the same store without corruption (concurrent access test).
  - I: schema migrations upgrade an old-version store without data loss (fixture DBs per version).
  - I: corrupt/missing store → clear error, backup/recovery path, no silent data loss.
- **Review 2026-10-26 (note):**
  - The technology is now decided: SQLite in WAL mode (architecture §7.1, D-12, accepted 2026-10-26).
  - The corrupt-store, missing-store, and migration behaviour in the verification is now stated as DAT-5 and DAT-6.
  - Use of one live game from several processes is now DAT-4.

### DAT-2 (AGREED; amended 2026-10-26, Q51/Q74) Backup / export of data
User can export all their data and import it back (format: IF-5).
- **AGREED 2026-10-26 (Q51):** import merges with the existing data. When an imported record collides with an existing one, the Game asks the user whether to replace the existing record, or to keep both as distinct records.
  - **AGREED 2026-10-26 (Q75):** a collision is an imported record with the same identity as an existing one, meaning the same Player UUID (PLY-6) or the same game ID.
  - **AGREED 2026-10-26 (Q75):** when keeping two Players, the imported one gets a new UUID, and the user gives it a new name if its name is taken (PLY-6).
  - **AGREED 2026-10-26 (Q75):** the prompt offers "apply to all remaining collisions".
  - **AGREED 2026-10-26 (Q75):** cancelling an import leaves the store unchanged.
  - Merge mechanics: architecture D-17 (preview, then one exclusive transaction after a backup).
- **AGREED 2026-10-26 (Q74):** the export is one archive holding a PGN file of all games plus JSON for Players, ratings, and settings (IF-5). This replaces "JSON/CSV + PGNs".
- **AGREED 2026-10-26 (Q75):** "all their data" means everything in the store, including computer Players, settings, saved variations, and analysis results.
- **Amended 2026-10-28 (CR-4; UG-12, UG-11): what "all data" holds, and unfinished games.**
  - The archive holds: every game as PGN, with saved what-if variations as PGN variations (IF-1); every game's stored fields that PGN tags and comments do not carry, and its analysis results: per-move evaluations and centipawn loss (`games.json`, IF-5); every Player (human, computer, and archived) with its engine parameters; both rating scales with their history; and settings.
  - **Unfinished games** are exported and, on import, restored as unfinished. If restoring one would give a Player a second unfinished game in a mode, or a computer a second unfinished c–c game, that game is imported as a review-only game with no result (as in RULE-7), and the import preview says so. A stored game that is open for play in a front-end (DAT-4) is never replaced by import: the import reports it and keeps the stored game.
- **Verification:** I: export→wipe→import yields identical logical contents. I: importing into a non-empty store with no collisions yields the union of both. I: with collisions, "replace" and "keep both" each give the expected records, and "apply to all" applies the choice to every remaining collision. I: cancelling an import leaves the store unchanged. *(Added 2026-10-28, CR-4:)* I: the round trip returns the same per-move evaluations, centipawn loss, saved variations, per-move times, archived Players, and an unfinished game with its clocks. I: an archive whose unfinished game would conflict with a stored unfinished game imports that game as review-only, and the preview names it. I: replacing a game that is open in another front-end is refused and reported.

### DAT-3 (AGREED; amended 2026-10-26, Q41/Q70/Q75) Per-game statistics
Store for each game (including self-play games, which are flagged — ELO-1/UI-2): players, colors, result + termination reason, start/end time, **game time (total elapsed duration, and time spent by each Player; excludes time the app was closed/paused during a saved-and-resumed game)**, move list (PGN), mode, for each computer side the computer Player's ID and the engine parameters it used, rating scale (Strict, Training, or unrated), whether analysis was used, Elo before/after on that scale, opening (ECO), per-move time, per-move engine evaluation / accuracy (centipawn loss). Per-move evaluation is computed after the game ends, in the background, and never during play.
- **AGREED 2026-10-26 (Q41, Q70):** "Elo before/after" applies to both sides, computer Players included, on the game's scale. "Rated flag" became "rating scale", because Training games are now rated on their own scale.
- Termination reasons include resignation and, for c–c games, aborted (GAME-4).
  - **Amended 2026-10-27 (CR-1):** they also include "voided by reset" (PLY-7). A voided game stores no result, and its Elo before/after fields are empty, because it is unrated.
- **Accuracy (EST, Q53):** stored as per-move centipawn loss and each side's average centipawn loss for the game. There is no separate "accuracy %" in v1.
- **AGREED 2026-10-26 (Q75), review amendments:**
  - (a) The ECO classification table used for "opening (ECO)" is bundled with the app, so classification works offline (PLAT-3). Architecture D-20 (accepted 2026-10-26) puts it in `chess-domain`, and its data license is checked under NFR-6.
  - (c) "Difficulty/engine level" (2026-10-24 wording) is replaced in the statement by the computer Player's ID and the engine parameters it used, because the level duplicated the computer Player reference (PLY-4).
- **Verification:** E: play known game → stored record matches expected fields, including the ECO code of a known opening and the computer Player's ID and parameters; U: record serialization round-trip; E (offline, PLAT-3): ECO classification works with networking disabled.

### DAT-4 (AGREED 2026-10-26, Q75) One front-end per live game
While an in-progress game is open in one front-end process, the Game shall refuse to open that game for play in any other front-end process, and the refusal shall name the front-end that has it open.
- **Rationale/Source:** DAT-1 lets the CLI, desktop, and web share one store at the same time. Two front-ends making moves in the same game would corrupt it. Architecture D-2, D-23 (the journal lock is the lease, accepted 2026-10-26), and §7.3 assume this behaviour (game leases; "this game is open in the CLI").
- If the owning process has stopped, the game becomes resumable elsewhere at once (TBD-28 = 0 s: the OS releases the journal lock when the owner stops; architecture §7.3).
- **AGREED 2026-10-26 (Q75):** live cross-process refresh of other screens, such as stats in another front-end, is not required in v1. Those screens show changes when reopened (architecture §7.3).
- **Priority:** Must.
- **Verification:** I: two processes open one game; the second is refused and the owner is named. After the owner is killed, the second can resume at once (TBD-28 = 0 s). E: the same check with CLI, web, and desktop.

### DAT-5 (AGREED 2026-10-26, Q51/Q75) Store integrity check and recovery
On start-up, the Game shall check the integrity of the data store. If the check fails, it shall tell the user, offer to restore the most recent backup, and leave the damaged file unmodified.
- **AGREED 2026-10-26 (Q51):** if a store existed before and is now missing, the Game alerts the user and asks whether to start with a new empty store or import records from a file the user chooses (DAT-2). On first launch, a new store is created without asking (PLAT-3).
- **Amended 2026-10-28 (CR-4; UG-21):** after a restore, the Game shall replay every move journal that is still held, whether the game was unfinished or finished after the backup was taken, over the restored store (the journal wins, NFR-2; architecture §7.1, §7.6). A game that finished after the backup is finalized again, in order of its end time, so that its Elo changes and statistics are applied once. The restore message states how many games and moves were recovered from journals. Journals are kept until the next backup has run (architecture §7.6), so every game that ended after the latest backup is recoverable.
- **Amended 2026-10-28 (CR-5; architecture D-35, OQ-12, R-19; accepts the architect's proposal):**
  - **Automatic backup.** DAT-5 offers "the most recent backup", so the Game shall make sure one exists. At start-up, when no other front-end process is running and the newest backup is older than 24 hours (TBD-33, EST), the Game shall write a complete backup copy of the store and check that the copy opens. It keeps the newest 3 automatic backups and deletes older automatic ones. Backups written before a schema upgrade (DAT-6) or before an import (DAT-2) are never deleted by this rule. The backups are local files in the data directory (PLAT-3, NFR-4). A failed automatic backup never stops the Game: it is logged (NFR-10) and tried again at the next start-up.
  - **What a restore cannot recover.** A move journal holds the moves of a game, not the registration, rename, archive, reset, settings change, generated Players or import done after the backup. Those actions are lost with the damaged store if they came after the latest backup, which is at most about 24 hours of non-game changes. A game that finished after the backup and involves a Player who does not exist in the restored store cannot be finalised. Its journal is kept, never deleted, and the restore message names the game. v1 does not recover such a game automatically, and does not journal registration. Every other retained journal is replayed as stated above.
- **Rationale/Source:** the DAT-1 verification ("corrupt/missing store → clear error, backup/recovery path, no silent data loss") had no matching statement. Architecture §7.1 assumes the integrity check (`quick_check`, restore offer).
- **AGREED 2026-10-26 (Q75):** the integrity check and restore offer in the statement.
- **Priority:** Must.
- **Verification:** I: a fixture corrupt file produces the message and the restore offer, and the file's bytes are unchanged afterwards. E: delete the store after first use → relaunch → the alert offers "start fresh" and "import from file", and each choice works. *(Added 2026-10-28, CR-4:)* I: restore a backup taken before 3 moves of an unfinished game and before a game that then finished: afterwards the unfinished game has all its moves and clocks, the finished game is present once, and each rating moved once for it. *(Added 2026-10-28, CR-5:)* I: start-up with a newest backup older than 24 h writes a new backup that opens, and with a newer one writes nothing; after 4 automatic backups only the newest 3 remain, and a pre-upgrade or pre-import copy is still present; an unwritable backup folder does not stop start-up. I: restore a backup taken before Player P was registered, then a game of P that finished after it: the restore completes, the game is not in the store, its journal is still on disk, and the message names it.

### DAT-6 (AGREED 2026-10-26, Q75) Backup before schema upgrade
Before upgrading the data store to a new schema version, the Game shall write a complete backup copy of the store.
- **Amended 2026-10-28 (CR-4; UG-17):** if the backup cannot be written (disk full, no permission), the Game shall not upgrade the store. The store file stays as it was. The front-end shall show a message that names the cause and offers retry or exit. It does not open the store at the old version for use by a newer front-end (PLAT-5).
- **Amended 2026-10-28 (CR-5):** the copy written before an upgrade is kept, and the automatic-backup rule that deletes old copies (DAT-5) never deletes it.
- **Rationale/Source:** DAT-1 verification ("schema migrations upgrade an old-version store without data loss"); architecture §7.1 (`VACUUM INTO chess.db.bak-v<N>`). Runs after a matching-version install (PLAT-5).
- **Priority:** Must.
- **Verification:** I: migrating from each fixture version leaves a backup that opens at the old version with identical logical contents. *(Added 2026-10-28, CR-4:)* I: with the backup folder unwritable, the migration does not start, the store bytes are unchanged, and the message offers retry or exit; retry after the folder is fixed completes the migration.

### DAT-7 (AGREED 2026-10-28, CR-4; systems engineer's decision, closes UG-17) Failed writes and unreadable journals
Each user action that changes the data store (register, rename, archive, change settings, generate computer Players, reset, import, restore) shall either complete in full or leave the store as it was, and when it fails, the front-end shall show a message that names the failure and the user's next step (NFR-9). When the move journal of an unfinished game cannot be read in full at resume, the Game shall resume the game from the longest intact run of moves that the journal or the store holds, and tell the user how many moves were recovered. If neither holds a readable game, the Game shall offer to void it: the game then has no result, termination reason "voided: unreadable", and no effect on any Elo or statistic.
- **Rationale/Source:** UG-17 (failed registration write, unreadable journal at resume). NFR-2 and RULE-5 promise that no completed move is lost; this states what happens when a file is damaged anyway. A torn last journal record is dropped by design (architecture §7.6), and that case is not an error.
- **Priority:** Must.
- **Verification:** I: fault injection (write failure, disk full, store locked) in each listed action leaves the store identical to its state before the action, with the message shown. I: a journal truncated mid-record resumes with every acknowledged move and no message about damage. I: a journal with a damaged record in the middle resumes from the longest intact run, from the journal or the store, whichever is longer, and reports the number of moves. I: with neither readable, the void offer appears; accepting it records "voided: unreadable" with no rating or statistic change and frees the slot.

---

## 6. Elo Rating

### ELO-1 (AGREED; amended 2026-10-26, Q41/Q58/Q70/Q78/Q79) Elo per game, tracked over time
Every Player, human or computer, shall have two Elo ratings: a **Strict** rating and a **Training** rating. After each completed rated game, the Game computes a new rating on the game's scale for each Player in the game, and stores a rating-history entry (scale, timestamp, game reference). Which games are rated, and on which scale, is set by ELO-2.
- **AGREED 2026-10-26 (Q41):** every Player is put in one of five categories when created. A human chooses the category at registration (PLY-2); a computer Player gets one when it is first created (PLY-4). Each Player's ratings start at the category's baseline Elo, and the first history point on each scale is that baseline. After each rated game, each Player's rating goes up or down by the Elo formula, against the opponent's current rating on the same scale.
  - This replaces the 2026-10-24 rules that games against the computer are rated against a fixed nominal Elo per difficulty level, that computer ratings never change, and that only human Players' Elo changes.
- **AGREED 2026-10-26 (Q70):** Training games are rated on the separate Training scale.
  - Interpretation: the Training rating starts at the same category baseline as the Strict rating.
  - The two scales never affect each other.
  - **AGREED 2026-10-26 (final round):** computer Players have a Training rating too, and it changes after every rated Training game they play, by the same formula as a human's. In a Training game both the human's and the computer's Training ratings change.
  - Storage of the two scales: architecture D-28 (accepted 2026-10-26).
- **AGREED baselines:** Beginner 400 · Novice 600 · Intermediate 1200 · Master 2200 · Grand Master 2500, for humans and computers alike, on both scales.
- **AGREED K-factor (2026-10-24; extended 2026-10-26, Q58):** K = 40 for a Player's first 30 rated games, K = 32 afterwards. The same rule applies to computer Players. **AGREED 2026-10-26 (Q78):** the 30-game count is kept separately for each scale: a Player's first 30 rated Strict games use K = 40 on the Strict scale, and its first 30 rated Training games use K = 40 on the Training scale, whatever the count on the other scale.
- Standard Elo expected score `E = 1/(1+10^((Rb−Ra)/400))` (assumed; standard formula).
- **AGREED 2026-10-24 — class bands:** a rating's class is the highest floor ≤ the rating: Beginner <600 (starts at 400), Novice 600–1199, Intermediate 1200–2199, Master 2200–2499, Grand Master ≥2500. A computer Player's displayed category follows its Strict rating (Q57, AGREED 2026-10-26 Q79).
- **AGREED 2026-10-24:** ratings are stored as real numbers and displayed rounded to the nearest integer.
- **AGREED 2026-10-24:** there is **no upper or lower cap** on Elo; it is computed and tracked as results dictate. A Grand Master is a Player at **2500 Elo or higher** (starting value 2500). This applies to both scales and to computer Players.
- **AGREED 2026-10-24:** self-play games (same human on both sides of a human–human game, or same computer Player on both sides of a computer–computer game) change no Elo, are excluded from win/loss/draw statistics, and are kept in game history.
- **AGREED 2026-10-24, reaffirmed 2026-10-26 (Q62):** no re-assessment or reset of Elo. A data reset (PLY-7) keeps both ratings.
- **Consequence:** each Player's change uses their own K (40 vs. 32), so changes are not exactly opposite when one Player has fewer than 30 rated games.
- **Verification**
  - E: registering with each of the five categories yields the configured starting rating on both scales; first history point on each scale = baseline. I: each generated computer Player's first history point on each scale = its category baseline.
  - U: known-value table (equal ratings win/draw/loss; +200 diff; etc.) matches hand-computed expected results to rounding rule.
  - U: each Player's delta uses their own K; deltas are exactly opposite when both Ks are equal; sign correct for win/draw/loss. Covers h–h, h–c (human and computer both change), and c–c between different computers.
  - U: K switches from 40 to 32 after exactly 30 rated games on a scale (game 30 uses 40, game 31 uses 32), for humans and computers, and the two scales' counts are independent: a Player with 31 rated Strict games and 0 rated Training games uses K = 32 on Strict and K = 40 on Training.
  - U: a Training game changes only Training ratings; a Strict game changes only Strict ratings.
  - U: unrated games (self-play, custom start position, imported, watcher-aborted c–c, voided by reset) and interruptions do not change any rating. A resignation is rated as a loss for the resigning side.
  - U: ratings above 2500 and below the starting value are stored/handled with no clamping.
  - I: rating history is append-only and consistent per scale: final rating = initial + Σ deltas (after a reset, initial = the reset point, PLY-7).
  - E: after N scripted rated games on one scale, history query for that scale returns N+1 points.

### ELO-2 (AGREED; amended 2026-10-26, Q41/Q59/Q60/Q70) Rating scale follows play mode
Strict-mode games are rated on the Strict scale, and Training-mode games on the Training scale. These games are unrated on both scales:
- self-play games (ELO-1);
- games from a non-standard starting position (RULE-8);
- imported games (RULE-7);
- computer–computer games aborted by the watcher (GAME-2);
- games voided by a reset (PLY-7; added 2026-10-27, CR-1).

Mode is not mutable mid-game, so no game can switch scales.
- **AGREED 2026-10-26 (Q70):** Training games are rated on their own scale. This replaces the 2026-10-24 rule "Training-mode games are casual and never change Elo".
- **AGREED 2026-10-26 (Q59):** a computer–computer game between two different computer Players is rated, and both computers' Strict ratings change. This replaces the 2026-10-24 rule "computer-vs-computer games change no rating".
- **AGREED 2026-10-26 (Q60):**
  - A resignation is a rated loss for the resigning Player.
  - An interruption is not an end of the game and changes no rating (GAME-4).
  - This replaces the round-1 rule "aborting a rated Strict game counts as a loss".
- **Verification:** E: Training game (with analysis and takeback) → only the Training ratings of the human and the computer change; both Strict ratings are unchanged. E: rateable Strict h–c game (standard start, not self-play) → both Players' Strict ratings change. E: c–c game between different computers → both Strict ratings change; the same game aborted by the watcher → no change. E: custom-FEN, self-play, and imported games → no change on either scale. E: resigned rated game → loss recorded for the resigning side. E: game interrupted by a kill and then resumed → no change until it ends. E (added 2026-10-27, CR-1): a game voided by a reset → no change on either scale for any Player in it (cases in PLY-7). *(The 2026-10-24 check "Strict game using restricted analysis → rating updated" was removed 2026-10-26, because Strict has no in-game analysis (ANA-1).)*

---

## 7. User Interface — Settings, Stats, Themes (GUI + CLI)

### UI-1 (AGREED; amended 2026-10-26, Q52/Q65 and sound removal) Settings
User can configure:
- default opponent category (pre-selected in new-game setup; matched against computers' current labels, which follow their Strict ratings, PLY-4)
- board theme
- piece theme
- GUI color theme (UI-10)
- side/color (the default color pre-selected in new-game setup)
- show legal-move hints
- coordinate orientation (flip)
- maximum engine think time (can only shorten a computer's think time)

- Settings are stored per Player in the data store (PLY-2).
- **AGREED 2026-10-26 (Q52) — move input:** every move-input method of a front-end is available at all times during play (GUI-2, CLI-3). There is no "default input method" setting; this replaces "move-input method defaults".
- **AGREED 2026-10-26 (Q65):** there is no confirm-move setting; when a move commits is fixed by GUI-2 and CLI-3. This replaces "confirm-move setting".
- **AGREED 2026-10-26 (round 2):** v1 has no sound effects, so there is no sound setting (Out of Scope). This replaces "sound" (2026-10-24) and the round-1 sound toggle (Q52).
- **AGREED 2026-10-26 (Q75):** "side/color" is the default color pre-selected in new-game setup (GAME-1).
- **AGREED 2026-10-26 (Q75):** the CLI uses only the settings that apply to it. Board, piece, and color themes are GUI-only; CLI styles are in UI-3.
- **AGREED 2026-10-26 (Q69, interpretation):** in c–c games the active (watching) profile's maximum think time applies. In h–h games it applies only to engine analysis (ENG-3).
- **Verification**
  - E: change each setting → takes effect immediately and persists after restart.
  - U: invalid values rejected/clamped; defaults applied for new Player.
  - E: settings changed in web are respected in desktop for the same Player.
  - E: every move-input method works in the same game without changing any setting. No input-method, confirm-move, or sound setting exists.
  - E: the side/color setting pre-selects the color in new-game setup. The CLI settings screen lists no board, piece, or GUI color theme.

### UI-2 (AGREED; amended 2026-10-26, Q70/Q73/Q75/Q76) Statistics views
User can view statistics over time: W/L/D totals, by color, by mode (Training, Strict), by difficulty/opponent, Elo-over-time charts for both scales, game history list with replay, and average game length. Self-play games appear in the game history list but are excluded from W/L/D totals and other aggregates.
- **AGREED 2026-10-26 (Q70):** Training games count in W/L/D totals. **AGREED 2026-10-26 (Q75):** the by-mode breakdown shows Training and Strict separately.
- **AGREED 2026-10-26 (Q76):** after a reset (PLY-7), the reset Player's own statistics show only games played after the reset. Opponents' statistics keep every game against that Player.
- **AGREED 2026-10-26 (Q73):** average game length is shown in both moves and time.
- Resignations count as losses (GAME-4). Watcher-aborted c–c games and unfinished games are not counted in aggregates, but resigned and aborted games are listed in history.
  - **Amended 2026-10-27 (CR-1):** games voided by a reset (PLY-7) are not counted in any aggregate. They are listed in the opponent's history, marked "voided by reset", and are hidden from the reset Player's views.
- CLI: a TUI statistics screen with tables and Elo-over-time charts; plain mode prints a text table + ASCII sparkline of each scale's Elo.
- **Amended 2026-10-28 (CR-4; UG-18, UG-16):**
  - **Custom-start games** (RULE-8) are excluded from W/L/D totals and from every aggregate (by color, by opponent, average length), like self-play. They are listed in history, marked "custom start". The reason is the one that excludes them from Elo (ELO-2): the position is not balanced. A resignation in such a game is recorded in the game record only.
  - **No data:** for a Player or a breakdown row with no games, counts show 0 and averages show "—" (never 0 or NaN). Each Elo chart shows at least its baseline point.
  - **Elo charts in the CLI:** the TUI draws each scale's history as a line chart in character cells (block or braille glyphs, with an ASCII fallback under `NO_COLOR`/plain-glyph style), next to a table of the latest points (date, rating, game). Plain mode prints the table and the sparkline. The GUI chart has a text or table alternative (UI-4). The chart library is architecture D-21.
- **Verification**
  - I: seeded store with known games (Training, Strict, self-play, resigned, aborted c–c, unfinished, voided by reset) → displayed numbers equal expected aggregates, including average length in moves and in time.
  - E: each Elo chart's point count and last value match the stored history for its scale.
  - E: empty state (no games) shows zero totals and a "no games yet" message, with no error.
  - I: seeded store with a reset Player → that Player's views exclude games from before the reset, and an opponent's views include them.
  - I (added 2026-10-28, CR-4): a seeded custom-start game appears in history marked "custom start" and in no total or average. E: a Player with no games shows 0 totals, "—" averages, and a baseline Elo point, in the TUI, plain mode, web, and desktop.
- **AGREED 2026-10-26 (Q75):** review amendment giving the empty-state check its criterion (the 2026-10-24 wording was "renders gracefully").

### UI-3 (AGREED) Themes
At least 10 board themes and 5 piece sets in GUI (incl. high-contrast). CLI supports ASCII style options (plain, Unicode glyphs, ANSI color, light/dark squares) and a no-color mode.
- **AGREED 2026-10-26 (Q45, Q68):** every GUI theme meets the UI-7 contrast values. The high-contrast theme needs nothing beyond WCAG AA.
- GUI color themes (light, dark, and others, following the system): UI-10.
- **Verification:** E: screenshot/visual-regression per theme (GUI); U: ASCII renderer snapshot tests per style.

### UI-4 (AGREED; amended 2026-10-26, Q75) Accessibility
GUI keyboard-operable, screen-reader labels for squares/pieces/moves, sufficient color contrast; CLI works with `NO_COLOR` and screen readers via plain mode (CLI-1), since full-screen TUIs are not reliably readable by screen readers.
- "Sufficient color contrast" is quantified in UI-7 (AGREED 2026-10-26, Q45).
- **AGREED 2026-10-26 (Q75):** UI-4 is the parent of its measurable parts, which are separate requirements: UI-6 (screen-reader announcements), UI-7 (contrast), and UI-8 (visible focus). The keyboard model is GUI-2 (b). UI-4's "keyboard-operable" and "sufficient color contrast" clauses are verified through GUI-2 (b), UI-7, and UI-8; the screen-reader labels and the CLI `NO_COLOR`/plain-mode clause are verified here. Architecture D-4 delivers these through the ARIA-grid overlay, live region, and `BoardModel`.
- **Verification:** automated a11y audit (axe) on web and on desktop (Tauri) (AGREED 2026-10-26, Q75); M: VoiceOver walk-through; U: contrast ratio checks for themes; E: with `NO_COLOR` set, CLI output has no color escape sequences.

### UI-5 (AGREED; amended 2026-10-26, Q41/Q57/Q70/Q73) Viewing stats for any Player
The user can view statistics for **any** Player, human or computer:
- W/L/D totals, including Training games, by mode, by color, and by opponent/category;
- game history with replay;
- average game length, in moves and in time;
- both Elo ratings (Strict and Training), current and over time.

Self-play games are excluded from aggregates but listed in history (UI-2). A "Computer Players" screen lists all computer Players with current category, both current ratings, and W/L/D.
- **AGREED 2026-10-26:** computer ratings change (Q41) and are shown over time like a human's, on both scales. A computer's category label follows its Strict rating (Q57, Q79). Training games count, and the Training rating is shown, for computers as well as humans (Q70, final round). Game length is shown both ways (Q73). This replaces "for humans, over time; computers show their static Elo".
- **AGREED 2026-10-26 (Q76):** a reset Player's past games still appear in the statistics and history of the Players they played (PLY-7).
  - **Amended 2026-10-27 (CR-1):** a game voided by the reset appears only in the opponent's history, marked "voided by reset", and never in any statistic. This applies to computer opponents too.
- **Verification:** I: seeded store → displayed numbers for a chosen human and a chosen computer Player equal expected aggregates. E: the computer Player list shows the 25 default Players with categories matching their current Strict ratings. E: each Elo chart matches the rating history for its scale. E: archived (deleted) human Players' history remains visible where referenced.

### UI-6 (AGREED 2026-10-26, Q75) Screen-reader announcements in the GUI
When either side moves, check is given, the game ends, or a move is rejected, the GUI shall announce the event to an active screen reader without moving keyboard focus.
- **Rationale/Source:** UI-4 (screen-reader labels for moves). A blind Player must learn the opponent's move without exploring the board. Architecture D-4 and §8 assume this (`LiveRegion`, aria-live).
- Announcement wording (SAN, or spoken form such as "knight to f3") is TBD-18. It is a wording choice, not a behaviour change, and is set in design at step 3.
- **Priority:** Must.
- **Verification:** E: Playwright asserts the live-region text after each event type, in web and desktop. M: VoiceOver speaks each event in Safari and in the desktop app while focus stays on the board.

### UI-7 (AGREED 2026-10-26, Q45/Q68) Color contrast
In every bundled GUI theme (board, piece, and color themes), the GUI shall give all text a contrast ratio of at least 4.5:1 against its background. The visual information needed to identify user-interface components and their states (including move and check highlights and focus indicators), and the graphical objects needed to understand the content (including pieces), shall have at least 3:1 against the adjacent colors. Ratios are computed as defined in WCAG 2.2: (L1 + 0.05) / (L2 + 0.05), where L1 is the relative luminance of the lighter color and L2 that of the darker.
- Original non-text wording (2026-10-26): "Pieces, move and check highlights, and focus indicators shall have at least 3:1 against the adjacent colors." **Amended 2026-10-27 (CR-2):** worded as SC 1.4.11 states its scope; the original items are kept as named examples. The original ratio wording was "Ratios are computed as defined in WCAG 2.2"; the formula is now stated.
- **AGREED 2026-10-26 (Q45):** WCAG 2.2 AA contrast values.
- **AGREED 2026-10-26 (Q68):** nothing beyond WCAG AA is required, including for the high-contrast theme. Interpretation: no minimum contrast between light and dark squares.
- **Rationale/Source:** UI-4's "sufficient color contrast" was not measurable. **Amended 2026-10-27 (CR-2, TBC-4 closed): verified source.** WCAG 2.2, W3C Recommendation, 12 December 2024, https://www.w3.org/TR/WCAG22/. Checked against the text:
  - SC 1.4.3 Contrast (Minimum), Level AA: text at least 4.5:1; large-scale text (at least 18 point, or 14 point bold) at least 3:1.
  - SC 1.4.11 Non-text Contrast, Level AA: user-interface components and their states, and graphical objects required to understand the content, at least 3:1 against adjacent colors.
  - The contrast ratio is (L1 + 0.05) / (L2 + 0.05).
- **Stricter than SC 1.4.3 (decision 2026-10-27, CR-2):** UI-7 keeps its AGREED wording, so 4.5:1 applies to all text, including large-scale text and text in inactive controls. UI-7 does not use the 3:1 allowance for large-scale text. This keeps one threshold for every text check. Q68 says nothing beyond AA is *required*; if the user wants exactly AA, the large-text allowance can be added by a one-line change request.
- **Priority:** Must.
- **Verification:** U: each theme's colors are computed against these thresholds (extends the existing UI-4 check). E: the axe color-contrast rule is clean on every screen in every color theme. U (amended 2026-10-27, CR-2): the contrast-ratio function implements (L1 + 0.05) / (L2 + 0.05) with the relative-luminance definition of WCAG 2.2. The test author reads that definition when writing the test and checks known pairs: black on white gives 21:1, and identical colors give 1:1. *(The one-time source check TBC-4 was closed on 2026-10-27. The relative-luminance definition was not read in that check, so this unit test carries that sub-check.)*

### UI-8 (AGREED 2026-10-26, Q75) Visible keyboard focus
While any GUI control or board square has keyboard focus, the GUI shall show a focus indicator on it that meets the UI-7 non-text contrast ratio.
- **Rationale/Source:** GUI-2 ("focus indicator visible"). Architecture D-4 assumes it (overlay focus ring on the real squares).
- **Priority:** Must.
- **Verification:** E: tab through every screen and arrow across all 64 squares in both orientations. Each focused element differs visibly from its unfocused screenshot, and the ring lies within the focused square's bounds.

### UI-9 (AGREED 2026-10-26, Q75) In-app help
On every GUI and TUI screen, the Game shall give access to help that lists that screen's controls, commands, and key bindings, using content shipped with the app.
- **Rationale/Source:** PLAT-3 lists help as an offline feature and CLI-4 has `help`/F1, but no requirement said what help contains or where it can be reached. Architecture §6.1, §6.2, and §8.1 assume a help screen everywhere.
- In plain mode, the `help` command prints the same content as text.
- Help tells the user to run `reset` if the terminal is left in a bad state after a forced kill (CLI-1, AGREED 2026-10-26, Q75).
- **Priority:** Must.
- **Verification:** E: from each screen in each front-end, help opens and lists that screen's controls. The offline E2E (PLAT-3) opens help. U: help content covers every CLI-4 command and key binding (checked against the KeyMap).

### UI-10 (AGREED 2026-10-26, Q52) GUI color themes
The GUI shall offer several color themes, including a light theme and a dark theme. By default it shall follow the system appearance (light or dark) and switch when the system appearance changes.
- **Rationale/Source:** Q52.
- **AGREED 2026-10-26 (Q75):** the user can pick a specific theme in settings (UI-1) instead of following the system.
- Every color theme meets UI-7.
- **Priority:** Must.
- **Verification:** E: switching macOS appearance between light and dark switches the GUI in web and desktop without a reload. E: choosing a specific theme overrides the system appearance. U: UI-7 checks pass for each color theme.

### UI-11 (AGREED 2026-10-26, Q52) Language
All user-facing text in v1 shall be in English. Move notation uses the English piece letters (K, Q, R, B, N).
- **Rationale/Source:** Q52. Other languages are out of scope for v1.
- **Priority:** Must.
- **Verification:** Inspection: all UI strings, help, and messages are English; SAN uses English piece letters.

### UI-12 (AGREED 2026-10-28, CR-4; derived from functional_decomp F-2.5.7) Confirmation of destructive actions
The Game shall ask the user to confirm before it carries out any of these actions: resigning a game, aborting a computer–computer game, archiving a Player, resetting a Player's data, applying an import (after its preview), restoring a backup, and quitting the TUI with Ctrl-C. Cancelling, or pressing Esc in a dialog, shall change nothing.
- **Rationale/Source:** the confirmations were stated one by one inside RULE-2, GAME-2, PLY-4, PLY-7, DAT-2, DAT-5, and CLI-1 (Q48, Q52, Q59, Q60). The architecture implements one `ConfirmationRequired` mechanism (§5.5). This statement keeps them in one place; it adds no new confirmation.
- Dialogs name what will happen, including, for a reset, each unfinished game that will be voided (PLY-7).
- **Amended 2026-10-28 (CR-5; architecture §10.4 H-20):** the offer to void a game whose journal and store cannot be read (DAT-7) is not an eighth action in this list. The offer is already an explicit choice with its own dialog, which states that the game gets no result and no effect on any rating, so a second confirmation would add nothing. Cancelling that dialog leaves the game as it is.
- **Priority:** Must.
- **Verification:** E: for each listed action, in each front-end that offers it, the confirmation appears; cancelling and Esc leave the store and the game unchanged.

---

## 8. Analysis Mode

### ANA-1 (AGREED; amended 2026-10-26, Q53) Analysis during play — Training mode only
During the human's turn, a human Player in a **Training** game can invoke analysis to view the top N best moves with evaluation (centipawns/mate), principal variation, and depth, for the current position or any earlier position.
- **Values (EST, Q53):**
  - N is 1–5, default 3.
  - Each request shows a first result within 1 s, then keeps searching until depth 20 or 10 s per position, whichever comes first, unless the user stops it.
- **Human's turn only (Q53, chosen by Claude):** this keeps the AGREED verification "not available for computer Players' turns". It also means analysis never competes with the computer's own search. It replaces "at any time" in the 2026-10-24 statement.
- Using analysis does not make a Training game unrated (ELO-2, Q70).
- **Strict mode: there is no analysis during play.** Analysis is offered only after the game ends (ANA-3). Humans may rewind to view past moves read-only (GAME-3).
- Analysis is for human Players only; computer–computer games have none during play.
- **Verification**
  - I: for known tactical positions (mate-in-N, free-queen), engine top move equals expected.
  - E: Training — invoking/exiting analysis at move k leaves the real game state unchanged and resumable.
  - E: Strict — no analysis control exists during play in CLI, web, or desktop; the engine is never queried for an in-progress Strict game on behalf of a human (spy on engine calls).
  - E: not available for computer Players' turns.
  - I: first result within 1 s and stop at depth 20 or 10 s on the baseline Mac (EST).

### ANA-2 (AGREED) What-if scenarios
In analysis (Training mode during play, and post-game in any mode), the user can play hypothetical moves on a branch (both sides), step back/forward, and branch again, with engine evaluation updating; exit discards or optionally saves the branch (PGN variation).
- **Verification**
  - U: variation tree add/navigate/delete operations.
  - E: moves in what-if never alter the stored game; "return to game" restores exact position.
  - E: export of saved branch is valid PGN with variation.

### ANA-3 (AGREED; amended 2026-10-26, Q53) Post-game full-game analysis
After every game (any mode), the user is offered analysis of the entire game: step through every position with engine evaluation and best alternatives, with blunder/mistake/inaccuracy flags by centipawn loss. No restrictions apply once the game is over; it never affects Elo. In Strict mode this is the **only** engine analysis available for the game.
- **Flag thresholds (EST, Q53):** centipawn loss from the mover's point of view.
  - Inaccuracy: 50–99.
  - Mistake: 100–299.
  - Blunder: ≥ 300, or a move that throws away a forced mate or allows one.
- **Verification:** E: offer appears after checkmate, draw, resignation, flag fall, and watcher abort (c–c) in each mode. I: a known game with a planted 300+ centipawn blunder is flagged as a blunder. E: running it leaves stored result and Elo untouched.
- Opening analysis later from game history: ANA-4.
- **Amended 2026-10-28 (CR-4; UG-16): evaluation not finished.** Per-move evaluations are computed in the background (DAT-3), so some plies may have none when the user opens the analysis. A ply without an evaluation shows "evaluating…" in place of its value, and no flag. The view shows how many plies are evaluated out of the total. Values and flags appear as they complete. The user can step through the game meanwhile, and the evaluation continues after an interruption (NFR-2). *Verification added:* E: open the analysis of a game whose evaluation is half done: the plies without values show "evaluating…", the count is shown, and flags appear as the plies finish.

### ANA-4 (AGREED 2026-10-26, Q49/Q60/Q75) Analysis of stored games
In every front-end, the user shall be able to open full-game analysis (ANA-3) and what-if (ANA-2) from the game history list for any completed, resigned, watcher-aborted, voided, or imported game.
- **AGREED 2026-10-26 (Q49, Q60):** resigned games and watcher-aborted c–c games can be analysed. Unfinished games are resumed, not analysed (GAME-4).
- **Amended 2026-10-27 (CR-1, interpretation):** a game voided by a reset (PLY-7) can be analysed from the opponent's history, like a watcher-aborted c–c game.
- **AGREED 2026-10-26 (Q75):** opening analysis from the game history list, as stated.
- **Rationale/Source:** RULE-7 ("imported PGNs are for review/analysis only"; RULE-5 before the split) and UI-2 ("game history list with replay") imply it, but ANA-3 only covers the offer made right after a game. Architecture §6.1 assumes it (Game history → replay / analysis / PGN export).
- Analysis never changes Elo or statistics (ANA-3, RULE-7).
- **Priority:** Must.
- **Verification:** E: in CLI, web, and desktop, open analysis from history for a completed Strict game, a Training game, a resigned game, a watcher-aborted c–c game, a game voided by a reset (from the opponent's history; added 2026-10-27, CR-1), and an imported PGN. I: stored result, Elo, and statistics are unchanged afterwards.

---

## 9. Command-Line Interface

### CLI-1 (AGREED 2026-10-24, amended; amended 2026-10-26, Q52/Q55 and round 2) Full-screen terminal UI
The CLI is a full-screen, ratatui-style terminal user interface (TUI). During play it shows the board, drawn with ASCII/Unicode art, in a board panel next to panels for the players and status, the clocks, the move list (SAN), and a command/move input line. The screen updates in place on every state change (move, clock tick, engine thinking, rewind, result) without scrolling.
- **AGREED 2026-10-24 — screens:** the other CLI flows are also TUI screens, navigated by keyboard: profile selection and registration (PLY-2), new-game setup (GAME-1), play, watching computer–computer games (GAME-2), post-game analysis and what-if (ANA-2/3), statistics (UI-2/5), Computer Players, and settings.
- **AGREED terminal size (2026-10-24; amended 2026-10-26, Q55; fixed in round 2):**
  - The minimum terminal size is fixed at 80×24, and every TUI screen shall fit legibly in 80×24.
  - If the terminal is smaller at start-up or after a resize, the TUI asks the terminal to resize its window to 80×24.
  - If the terminal has not reached 80×24 within 1 s (EST), for example because it ignores the request, the TUI shows an "enlarge the terminal to 80×24" message instead of a broken layout, and redraws as soon as the size is enough.
  - Resizing reflows the layout immediately.
  - **AGREED 2026-10-26 (Q75):** the original window size is restored on exit if the TUI changed it.
- **AGREED 2026-10-24, amended 2026-10-26 (Q75) — terminal hygiene:** the terminal is always restored (cursor, echo, main screen) on quit, Ctrl-C, panic, SIGTERM, and SIGHUP. Ctrl-Z suspends and `fg` redraws the screen.
  - Original wording (2026-10-24): "on quit, Ctrl-C, panic, or crash". "Crash" was replaced because no program can restore the terminal after SIGKILL or power loss. For those cases, help (UI-9) tells the user to run `reset`.
- **AGREED 2026-10-26 (Q52):** Ctrl-C asks for confirmation before quitting. The terminal is restored when the TUI quits. Quitting suspends an unfinished game; it is not a resignation (GAME-4).
- **AGREED 2026-10-24 — no mouse:** the TUI has no mouse support in v1; it is operated entirely from the keyboard.
- **AGREED 2026-10-24 — plain mode:** a line-oriented mode (`--plain`) prints the board and prompt after each move, as a scrolling transcript. It is selected automatically when stdin/stdout is not a terminal. It serves screen readers (UI-4) and piping. Non-interactive scripted runs (CLI-4) never start the TUI.
- **Verification**
  - U: TUI render snapshots at fixed terminal sizes (start position, mid-game, check, promotion prompt, flipped for Black, rewound with "live position has changed" indicator, game result) for each board style. Every screen renders without truncation at 80×24.
  - U: board renderer snapshot tests per style (shared by TUI and plain mode).
  - E: PTY-driven session with a terminal-emulator screen parser asserts the visible screen after each input.
  - E: started below 80×24 in a terminal that honours the resize request, the window grows to 80×24. In a PTY that ignores the request, the enlarge message appears within 1 s, and resizing back restores the layout. M: resize behaviour checked in Terminal and iTerm2.
  - E: after `quit`, a confirmed Ctrl-C, an injected panic, SIGTERM, and SIGHUP, the terminal is restored (main screen, echo on, cursor visible), and the unfinished game resumes on relaunch. An unconfirmed Ctrl-C leaves the game running.
  - E: when the TUI resized the window at start-up, the original size is restored on exit (in a PTY that reports window size).
  - U: the help content includes the `reset` advice for a forced kill.
  - E: plain mode via PTY and via pipe shows the board and prompt after each move.

### CLI-2 (AGREED) Coordinates
In the board panel (and in plain mode), the board shows file labels A–H and rank labels 1–8 on all sides needed; orientation flips when Black is at the bottom.
- Note: display uppercase A–H per spec; input accepts either case (Q8), subject to the bishop rule in CLI-3 (Q44).
- **Verification:** U: snapshot includes labels in both orientations.

### CLI-3 (AGREED; amended 2026-10-26, Q44/Q65) Algebraic notation input
Player must enter moves in algebraic notation, typed in the TUI input line (or at the plain-mode prompt); game record kept in algebraic notation (SAN). Invalid input shows an error in the TUI status area and leaves the typed text editable.
- **AGREED 2026-10-24:** typing is the only way to move in the CLI. There is no keyboard board cursor (unlike GUI-2), so the arrow keys stay free for rewind (CLI-4).
- **AGREED 2026-10-26 (Q65):** a typed move commits when the user presses Enter. There is no further confirmation step.
- **Accepted forms:**
  - SAN: `e4`, `Nf3`, `exd5`, `O-O`, `e8=Q`.
  - Coordinate form, as a convenience: `e2e4`.
  - File letters in either case.
  - Trailing `+`/`#`/`x` optional.
- **AGREED 2026-10-26 (Q44) — bishop/b-file rule:** piece letters are recognised only in uppercase.
  - A leading lowercase `b` is always the b-file.
  - A leading uppercase `B` means a bishop when a bishop move matches; otherwise it is read as the b-file.
  - The same rule applies to typed input in the GUI (GUI-2 (c)).
- **Verification**
  - U: parser table tests: valid (all piece types, captures, castling both sides incl. `0-0`, promotion, en passant, disambiguation `Nbd2`, `R1a3`), invalid (garbage, ambiguous, illegal, off-board) with helpful error messages.
  - U: positions where a bishop capture and a b-pawn capture on the same square are both legal: `bxc3` plays the pawn capture and `Bxc3` the bishop capture. In a position where only the pawn capture is legal, `Bxc3` plays the pawn capture.
  - E: re-prompt on bad input without losing turn; game never crashes on arbitrary input (fuzz).
  - E: a valid move is applied on Enter, with no confirmation prompt.
  - U: move list output equals SAN for the played game.

### CLI-4 (AGREED; amended 2026-10-26, Q52/Q60) Command set
In-game commands distinct from moves, typed in the input line: `help`, `resign`, `draw`, `undo` (Training only), `movenow` (while a computer is thinking in a human–computer game), `analyze` (Training during play; any mode post-game), `back` / `forward` / `live` (rewind view, GAME-3), `save`, `flip`, `moves` (list legal), `history`, `quit`. Non-interactive flags for scripting (e.g., `chess --matchup cvc --mode strict --white <computer-player> --black <computer-player> --clock 5+0 --no-color`); scripted runs do not start the TUI. How the CLI chooses between headless, plain, and TUI is architecture D-19 (accepted 2026-10-26).
- **AGREED 2026-10-26 (Q52, Q60):**
  - `resign` asks for confirmation. In hot-seat h–h it also asks which side is resigning (RULE-2).
  - There is no `abort` command in human games. The round-1 `abort` command is withdrawn (interpretation, Decision Log).
  - `quit` suspends the game; it does not resign (GAME-4).
- **AGREED 2026-10-24 — key bindings:** frequent actions also have keys that can never begin a move, so typing a move is never intercepted: ←/→ = back/forward, End = live, PgUp/PgDn = scroll move list, F1 = help, Tab = switch panel focus, Esc = cancel/clear input. Letters, digits, and `-`/`=` always go to the input line. Commands that the mode policy disallows (PLY-5) are shown as unavailable and their keys do nothing but explain why.
- **AGREED 2026-10-24 — c–c watch keys (GAME-2):** the watch screen has no move input, so plain keys are used: Space = pause/resume, `s` = step one move while paused, `+`/`-` = move delay, `a` = abort (with confirmation; changes no Elo or stats, Q59). PgUp/PgDn scroll the move list; there is no rewind (Q50).
- **Amended 2026-10-28 (CR-4; UG-3, UG-13, UG-14):**
  - **`movenow`** and key **F2** play the computer's best move found so far (ENG-5). Both are shown as unavailable when no computer is thinking in a human–computer game.
  - **`save`** changes nothing, because every move is saved as it is played (RULE-5). It prints "Game saved" with the number of moves stored, after confirming that the last move is durably written.
  - **Scripted runs** (non-interactive flags): (a) computer–computer only. A scripted run that names a human Player is a usage error (exit code 1); a human side needs input that a script cannot give. (b) Always Strict. (c) The game is stored in the data store like any c–c game: it uses each computer's c–c slot, is rated by ELO-2, and counts in W/L/D. (d) `--data-dir <path>` selects another data directory (created with the 25-Player roster if absent). It overrides the `CHESS_DATA_DIR` environment variable, which overrides the default directory. Tests, calibration, and soak runs use it (RISK-2). (e) If a chosen computer already has an unfinished c–c game, the run is a setup error (exit code 1, CLI-5); it does not offer to resume. (f) A run stopped by a signal leaves an unfinished game that can be resumed as a watched game (RULE-5), and exits non-zero.
- **Amended 2026-10-28 (CR-5; architect's pushback 3 accepted):**
  - **Scripted games with a human side.** A scripted run that has a human Player is a usage error, as above. A scripted human game, such as PLAT-1's list of SAN moves for a human–computer or hot-seat game, is played in the plain line-oriented mode with the moves piped to standard input (CLI-1; architecture §6.1), not in the headless mode.
  - **Data commands.** The commands that export and import (`chess export game`, `chess export data`, `chess import pgn`, `chess import data`), the listing command `chess games` (it shows the numbers that `chess export game` accepts), and the options `--on-duplicate skip|keep` (PGN import, IF-2) and `--yes` (answers the import confirmation of UI-12 for scripts) are design choices of architecture §5.7 and D-36 that realise IF-1, IF-2, DAT-2 and UC-22 to UC-25. They add no requirement. Their names are public once released.
- **Verification:** E: each command and key binding via PTY; U: command vs. SAN ambiguity (e.g., a move can't collide with a command word); U: no key binding consumes a character that can start a SAN or coordinate move; E: disallowed commands are visibly unavailable in each mode; E: `resign` asks for confirmation (and for the side in hot-seat), and cancelling continues the game; E: typing `abort` in a human game is rejected as not a command or move. *(Added 2026-10-28, CR-4:)* E: `save` prints the stored move count and changes nothing. E: a scripted run with a human Player exits 1 with nothing on stdout. E: a scripted c–c run with `--data-dir` pointing at a temporary directory leaves the default store unchanged, and the same run without it changes the computers' Strict ratings in the default store. E: a second scripted run with a computer that is in an unfinished c–c game exits 1.

### CLI-5 (AGREED 2026-10-26, Q75) Headless run output and exit status
When the CLI runs non-interactively from scripting flags (CLI-4), it shall write the finished game as PGN (IF-1) to standard output. It shall exit with status 0 when the game reached a result, and with a non-zero status when the setup was invalid or the game could not be completed.
- **Rationale/Source:** CLI-4 (scripted runs) and PLAT-1 (the same scripted game in all three front-ends) need a stable contract for scripts and tests. Architecture §6.1 assumes it (headless mode "prints the PGN and exits with a status code").
- Error messages go to standard error. Whether each failure class (invalid flags, engine failure, store error) gets its own code is TBD-21 (closed 2026-10-28, CR-5, accepting the architect's codes: 0 result, 1 usage or setup error, 2 engine unavailable, 3 store error, 4 stopped by a signal; architecture §6.1, D-36).
- **Priority:** Must.
- **Amended 2026-10-28 (CR-5):** a run stopped by SIGINT or SIGTERM leaves an unfinished game (CLI-4) and exits with code 4.
- **Amended 2026-10-28 (CR-4):** a usage or setup error includes a human Player named in a scripted run and a computer already in an unfinished c–c game (CLI-4). The data directory can be chosen with `--data-dir` (CLI-4).
- **Verification:** E: a headless c–c run writes one PGN game to stdout whose result matches the stored record, and exits 0. Invalid flags give a non-zero exit, nothing on stdout, and a message on stderr. *(Added 2026-10-28, CR-5:)* E: each failure class gives its code (1 usage or setup, 2 engine unavailable, 3 store error), and a run stopped by SIGTERM exits 4 with an unfinished game stored.

---

## 10. Graphical UI (Web + Desktop)

### GUI-1 (AGREED) Common GUI
Web and desktop share one GUI codebase (desktop wraps the same UI).
- **Verification:** E: same E2E suite run against web and desktop shells; any divergence is a failure.

### GUI-2 (AGREED; amended 2026-10-26, Q44/Q52/Q65/Q75) Move input methods
Player can move by (a) mouse point-and-click and drag-and-drop, (b) keyboard: arrows move cursor, Space selects piece, arrows pick destination, Enter confirms, (c) typing algebraic notation, (d) touch on web: tap a piece, then tap its destination.
- Original wording (2026-10-24): "(a) mouse point-and-click (and, proposed, drag-and-drop), (b) keyboard …, (c) typing algebraic notation." Drag-and-drop was accepted with sections 8–10 on 2026-10-24. **AGREED 2026-10-26 (Q75):** review amendment (a) added (d), which the 2026-10-24 verification already required.
- **AGREED 2026-10-26 (Q52):** all of these methods are available at all times during play. The user can switch methods at any time, even partway through a move. There is no default-method setting (UI-1).
- **AGREED 2026-10-26 (Q65) — when a move commits:** there is no confirm-move step.
  - A mouse or touch move commits when the piece is dropped: on release for drag, or on the destination click or tap for click-to-move or tap-to-move.
  - A typed SAN move commits on Enter.
  - A keyboard-overlay move commits when the destination square is selected (Enter on the destination), as an interpretation (Decision Log).
  - A promotion commits when the promotion piece is chosen.
- **AGREED 2026-10-26 (Q44):** typed input uses the CLI-3 parser and its bishop/b-file rule.
- AGREED 2026-10-24 (accepted with sections 8–10): Esc cancels selection; legal destinations highlighted; focus indicator visible (quantified in UI-8).
- **Verification**
  - E: same move made by each of the three methods produces identical game state.
  - E: each method commits at the stated moment and at no other: drag release, destination click/tap, Enter for SAN, Enter on the destination for keyboard, and the promotion choice.
  - E: keyboard: select a piece with no legal moves (feedback), cancel with Esc, illegal destination rejected, promotion prompt via keyboard.
  - E: input method mixing mid-move (select by mouse, confirm by Enter): the selection made by one method is the selection seen by the others, and the move completes. This matches the D-4 spike criterion 1 (architecture §11.1). **AGREED 2026-10-26 (Q75):** review amendment (b), which replaced the 2026-10-24 criterion "behaves sensibly".
  - E: touch tap-to-move on web (AGREED 2026-10-24).

### GUI-3 (AGREED) Coordinates in GUI
Board shows A–H / 1–8 labels (toggleable in settings, default on).
- **Verification:** E: labels present and correct in both orientations.

### GUI-4 (AGREED) Game feedback
Highlight last move, check, legal moves; move list panel (SAN); captured pieces; game result dialog; engine "thinking" indicator.
- **Amended 2026-10-28 (CR-4; UG-3):** while a computer is thinking in a human–computer game, the indicator includes a "Move now" button (ENG-5), labelled for screen readers, reachable by Tab, and operated with Enter or Space or the key F2.
- **Verification:** E: assertions on DOM/accessibility tree for each indicator, including the "Move now" button while the computer thinks and its absence otherwise.

### GUI-5 (AGREED 2026-10-26, Q55) Scalable layout
The GUI shall have no minimum window size. At every browser viewport or desktop window size, it shall scale its layout to the space it is given. The board stays square, fully visible, and aligned with its keyboard overlay, every control remains reachable, and the page never scrolls horizontally.
- **AGREED 2026-10-26 (Q55):** no minimum size; the GUI scales cleanly to whatever space the host gives it. This replaces the PROPOSED "Minimum window size".
- This supersedes "responsive/mobile layout is out of scope" (GUI-2 verification, 2026-10-24) for layout scaling. Phones and tablets are still not supported platforms (PLAT-2).
- **AGREED 2026-10-26 (Q75):** at small sizes, side panels such as the move list may scroll vertically.
- **Priority:** Must.
- **Verification:** E: Playwright at 480×320, 1024×768, 1440×900, 2560×1440, and 5120×2880 CSS pixels (EST test set), in both board orientations. At each size the board is square and fully visible, the overlay is aligned (D-4 spike criterion 3), every control is reachable, and there is no horizontal scroll. Live resizing between sizes needs no reload.

---

## 11. Non-Functional (AGREED 2026-10-24)

| ID | Requirement | Verification |
|---|---|---|
| NFR-1 (AGREED; amended 2026-10-26, Q47/Q75) | Startup to playable ≤ 3 s (CLI ≤ 1 s) on the baseline Mac (Apple M4, ≥ 16 GB RAM, macOS Tahoe; AGREED 2026-10-26, Q47). Startup is measured from launching the command or app (for web, running `chess serve`) to the first screen accepting input, as a warm start, taking the median of 10 runs (AGREED 2026-10-26, Q75). | Benchmark in CI/manual: median of 10 warm starts per entry point, from launch to the first screen accepting input |
| NFR-2 (AGREED; amended 2026-10-26, Q54) | No completed move shall be lost after any interruption: process kill, crash, OS crash, hard reset, power loss, disconnect, or closed browser tab. After restart, each clock is restored to its last recorded value (RULE-5). If the shared data store is unavailable when a move is completed, the move shall still be kept and written to the store once it becomes available. Where the store disagrees with the moves recorded on the Mac at play time, the moves recorded at play time take precedence. *(Replaces the 2026-10-24 wording "No data loss on crash/kill at any point (atomic writes, WAL)", which named a design.)* | Fault injection at every write step (kill, simulated power loss, store locked or unavailable, tab closed) → after restart every completed move is present, each clock equals its last recorded value, and the store passes an integrity check |
| NFR-3 | Engine moves always legal; app never enters illegal state | Property-based/fuzz tests; c-v-c soak of 1000 games with no crashes (on an isolated store, RISK-2) |
| NFR-4 (AGREED; amended 2026-10-26, Q43) | Privacy: in v1 the Game makes no network connection other than loopback, and has no telemetry, update check, or other outbound feature. Data stays on the Mac. Network play is a planned future enhancement (PLAT-3). *(Replaces "data stays local unless user opts in; no telemetry by default".)* | Network-capture test shows no outbound calls |
| NFR-5 (amended 2026-10-28, CR-4) | Documented install + run steps for each of the 3 entry points. The documentation shall cover: install, update, and uninstall (PLAT-6); the steps to open an app that macOS blocks (PLAT-1); the data directory and the warning that deleting it removes all data; the `config.toml` keys and their errors, and how to stop `chess serve` (PLAT-4); and `reset` for a terminal left in a bad state (CLI-1) | M: fresh-machine walkthrough; install and update keep the CLI and desktop app at one version (PLAT-5). Inspection: the documents contain each listed item |
| NFR-6 (AGREED; amended 2026-10-26, Q75) | Open-source license compliance for the engine, all code dependencies, and all bundled assets (fonts, piece sets, board themes, the ECO table); the project license is GPL-3.0-or-later (D-10). *(2026-10-24 wording: "Open-source license compliance for engine and all deps".)* | License audit in CI, covering code dependencies and every bundled asset. The audit records the Stockfish GPL version read from its shipped `COPYING` file (added 2026-10-27, CR-3, TBC-9). The audit is a step of the release procedure (use case UC-29; added 2026-10-28, CR-4) |
| NFR-7 (AGREED 2026-10-26, Q75) | Security: the local web server shall reject every HTTP and WebSocket request whose `Host` or `Origin` is not the server's own loopback origin. *Rationale:* without this, any web page open in the browser, or a DNS-rebinding host name, could drive games and read data over localhost (architecture R-5, §5.2). Loopback-only binding is PLAT-4. No access token is required, because ASM-1 (one person per Mac account) was confirmed on 2026-10-26 (architecture D-22, accepted). Priority Must. | I: requests with a foreign `Host`/`Origin`, including a rebinding host name, are rejected. E: a page from another origin cannot open the WebSocket. |
| NFR-8 (AGREED 2026-10-26, Q75; value EST, Q53) | Responsiveness: while an engine search or background analysis runs, every front-end shall show visible feedback to a key press, click, or tap within 100 ms (EST) on the baseline Mac. *Rationale:* "UI never blocks" (ENG-3 before the split) is not measurable. 0.1 s is the limit for the user to feel that the system reacts instantaneously (Jakob Nielsen, "Response Times: The 3 Important Limits", an excerpt from Chapter 5 of *Usability Engineering*, 1993, https://www.nngroup.com/articles/response-times-3-important-limits/; the article credits Miller (1968) and Card et al. (1991) as the origin of the limits). *Amended 2026-10-27 (CR-2, TBC-5 closed):* citation verified; the 100 ms value stays an EST (TBR-2). Priority Must. | I/E: inject inputs during a maximum-length search; 95th-percentile input-to-render latency ≤ 100 ms. E: during engine think the UI stays interactive (settings open, resign works) (moved from ENG-3). |
| NFR-9 (AGREED 2026-10-26, Q75) | Error reporting: when the Game rejects a user action or an operation fails, the front-end shall show a message that states what failed and, where one exists, the user's next step. Cases include an illegal or stale move, a disallowed command, an invalid import file, a version mismatch (PLAT-5), and a store or engine error. *Rationale:* error handling appeared only in scattered verification lines (CLI-3, ENG-1, DAT-1, RULE-5). Architecture §8 maps error codes to messages. Priority Must. | U: every protocol error code has a message. E: each listed error path in each front-end shows its message and the game continues. Fuzz (CLI-3, NFR-3). |
| NFR-10 (AGREED 2026-10-28, CR-4; derived from functional_decomp F-6.14) | Diagnostic log: the Game shall write a local diagnostic log of errors and significant events (start-up, engine start and failure, store errors, migrations, imports, restores) to a file in the data directory. The log shall never be sent over a network (NFR-4). It shall hold Player and game identifiers but no move lists and no Player names. It shall rotate at 10 MB with 3 files kept (TBD-32, EST). *Rationale:* a failure the user cannot fix (NFR-9) needs a record for later diagnosis, and the architecture already assumes one (§5.5 correlation ids, §10.2). Priority Must. | I: a forced engine failure and a forced store error each add a log line with an identifier and without names or moves. I: the log rotates at the size limit. Inspection: the network-capture test (NFR-4) shows no log upload. |

- **AGREED 2026-10-26 (Q75):** the review amendments to NFR-1 (start and end points of startup) and NFR-6 (bundled assets) are merged into the table above.
- NFR-2 durability is designed by architecture D-23 (move journal, accepted 2026-10-26).

---

## 12. External Interfaces (AGREED 2026-10-26, Q75)

Added by the 2026-10-26 review. RULE-5–RULE-8, DAT-2, and ENG-1 name PGN/FEN files, the full-data export, and the engine, but do not specify them. The front-end ↔ core protocol is internal and is specified in architecture §5, not here.

### IF-1 (AGREED 2026-10-26, Q75) PGN export
The Game shall export each game as PGN that conforms to the export format of the PGN Standard (Steven J. Edwards, coordinator, "Portable Game Notation Specification and Implementation Guide", revised 1994.03.12), except that the file is encoded as UTF-8. The export includes the Seven Tag Roster, SAN movetext, and the result.
- **Amended 2026-10-27 (CR-2, TBC-3 closed): verified source.** Copy read at https://phh.readthedocs.io/en/latest/_static/PGN_Reference.txt. Checked against the text:
  - Section 4.1: "PGN data is represented using a subset of the eight bit ISO 8859/1 (Latin 1) character set". The UTF-8 encoding is therefore a deliberate deviation from section 4.1.
  - The standard defines an import format (flexible, may be prepared by hand) and an export format (strict, generated by programs). IF-1 output uses the export format ("export format" added to the statement on 2026-10-27, matching the deterministic output that PLAT-1 needs). IF-2 input follows the import format.
  - Not yet checked: the Seven Tag Roster text and the export-format tag spacing rules (TBC-6).
- **Amended 2026-10-27 (CR-1, interpretation):** a game voided by a reset (PLY-7) is exported with `Result "*"`, `Termination "abandoned"`, and the custom tag `EndStatus "voided"` (architecture §5.7, D-30; tag name in TBD-24). A watcher-aborted c–c game is exported the same way with `EndStatus "aborted"`. The standard tag values are checked under TBC-6.
- Tags: a non-standard start uses `SetUp` and `FEN`, a timed game uses `TimeControl`, and the termination reason uses `Termination`. Mode, rating scale, and Player category go in custom tags, with names TBD-24.
- Saved what-if branches are exported as PGN variations (ANA-2).
- **Amended 2026-10-28 (CR-4; UG-11):** an **unfinished game** is exported with `Result "*"`, no `Termination` tag, and no `EndStatus` tag, and with the moves completed so far. Exporting does not change the game. Importing such a PGN (IF-2) gives a review-only game with no result (RULE-7). It is never resumable.
- **AGREED 2026-10-26 (Q75):** the 1994 PGN Standard is the reference, and the encoding is UTF-8, a deliberate deviation. The 1994 standard specifies a subset of ISO 8859/1 (Latin 1) (section 4.1, verified 2026-10-27), which cannot hold every Unicode Player name (PLY-6). This closes TBR-3.
- **Rationale/Source:** RULE-6 ("PGN export for any game"; RULE-5 before the split), ANA-2, DAT-2, CLI-5. PLAT-1 compares PGN across front-ends, so the export must be deterministic. PGN is the de facto game format (Q74).
- **Priority:** Must.
- **Verification:** U: exported PGN of fixture games re-imports (IF-2) with identical moves and tags. U: the export of one game is byte-identical from CLI, web, and desktop. U: a Player name with non-Latin-1 characters round-trips as UTF-8. Inspection: a sample opens in an independent PGN reader, chosen by the test owner and named in the test report. Inspection (once, step 1): the Seven Tag Roster, the tag names used, the export-format tag spacing rules, the `*` result token, and the `Termination "abandoned"` value match the PGN Standard text (TBC-6). U (added 2026-10-27, CR-1): a voided game and a watcher-aborted game export with `*` and the matching `EndStatus`, and the full-data import restores their status. *(Added 2026-10-28, CR-4:)* U: an unfinished game exports with `*` and no `Termination` or `EndStatus` tag, the game stays unfinished, and the exported file imports as a review-only game with no result. *(The encoding clause was verified on 2026-10-27, closing TBC-3.)*

### IF-2 (AGREED 2026-10-26, Q51/Q75) PGN import
The Game shall import every valid game in a PGN file, including files that hold many games. For each game that fails to parse, it shall report the game's index in the file and the line of its first error, and still import the remaining valid games.
- **AGREED 2026-10-26 (Q51):** multi-game PGN files are in scope.
- Content accepted: tags, SAN movetext, comments, NAGs, variations, and `SetUp`/`FEN` starting positions.
- **Amended 2026-10-27 (CR-2):** the importer accepts the PGN Standard's import format (the flexible, hand-preparable form; see IF-1), in Latin-1 (section 4.1) or UTF-8.
- **AGREED 2026-10-26 (Q75):** variations are kept for analysis. A game identical to one already imported (same tags and moves) is a collision and is handled by the DAT-2 prompt.
- **Amended 2026-10-28 (CR-4; closes UG-22):** a **duplicate** is an imported game whose tags and moves are identical to a game in the store, or to an earlier game in the same file. The collision prompt of DAT-2 for a PGN import offers **"skip this game"** (the default) and **"keep both"**, with "apply to all remaining collisions". It does not offer "replace": the contents are identical, so nothing would change, and a replacement could overwrite a played game that is rated. A skipped duplicate is not an error. The result report gives the number imported, the number skipped as duplicates, and the failures with index and line. (The functional model's "skip duplicates" is therefore one of the two choices, not the only behaviour.)
- A file larger than TBD-25 is rejected with a message (NFR-9).
- Imported games are review-only: never rated, never counted in statistics (RULE-7), and listed in game history (ANA-4).
- **Rationale/Source:** RULE-7 ("PGN import"; RULE-5 before the split, including "malformed PGN/FEN rejected gracefully"), architecture §5.1 (`ImportPgn`).
- **Priority:** Must.
- **Verification:** U: fixture files (multi-game, variations, comments, NAGs, Latin-1 and UTF-8, malformed at a known line) import with the expected game count and error positions, and kept variations open in what-if (ANA-2). I: re-importing a file already imported raises the DAT-2 collision prompt. *(Added 2026-10-28, CR-4:)* I: choosing "skip" imports none of the duplicates and the report counts them; "keep both" adds copies; "apply to all" applies to the remaining duplicates; a duplicate of a played, rated game is never able to replace it; a file holding the same game twice raises one prompt for the second copy. Fuzz: arbitrary bytes never crash the importer.

### IF-3 (AGREED 2026-10-26, Q53/Q75) FEN input
The Game shall accept a position as a six-field FEN string, and shall reject, with a message naming the problem, a FEN that is malformed or describes an illegal position. Illegal positions include a wrong number of kings, a pawn on the first or last rank, and the side not to move being in check.
- Uses: new-game setup from a position (GAME-1; unrated per RULE-8) and position import for analysis.
- **AGREED 2026-10-26 (Q53):** FEN export is not in v1 (Out of Scope).
- **Rationale/Source:** RULE-8 (RULE-5 before the split), GAME-1; architecture §3.3 (`Fen`) and §5.1 (`ImportFen`).
- **Priority:** Must.
- **Verification:** U: a table of valid FENs (including castling and en-passant fields) and invalid FENs (each syntax error and each illegal-position case) gives the expected acceptance or message.

### IF-4 (AGREED 2026-10-26, Q75) Engine
All computer play and analysis in the Game shall use one pinned Stockfish release, Stockfish 19 (tag `sf_19`, published 2026-09-05), bundled with every entry point for both Intel (x86_64) and Apple silicon (arm64) (PLAT-2), and controlled through the UCI protocol.
- Original wording (2026-10-26): "one pinned Stockfish release (version TBD-26)". **Amended 2026-10-27 (CR-3, user decision "use the latest Stockfish as the baseline"; TBD-26 closed as a version choice):**
  - Source: GitHub release API, https://github.com/official-stockfish/Stockfish/releases/tag/sf_19 (not a pre-release). Release assets include `stockfish-macos-universal.tar.gz`. The release notes say Stockfish moved to "universal binaries" that detect CPU features at run time. That means CPU-feature dispatch; it does not by itself show that the macOS asset holds both arm64 and x86_64 code.
  - License: the release notes say "licensed under the GPL license". That matches the project's GPL-3.0-or-later (D-10). The exact GPL version is read from the shipped `COPYING` file (TBC-9).
  - Not verified (the binary could not be downloaded and run): the architectures in the macOS asset (TBC-7), and the `UCI_Elo` range, including its floor (TBC-8).
  - **Any later Stockfish release** needs a Decision Log entry and a new calibration run (ENG-2) before it is pinned, because computer parameters never change after creation (Q57, Q72).
- This is a constraint, not a free design choice. It follows from ENG-1 (AGREED: Stockfish, UCI), PLAT-3 (bundled, no run-time download), PLAT-2 (Intel and Apple silicon targets), and architecture D-9 (pinned, checksummed at build time).
- **Rationale/Source:** without a pinned version, the strength calibration (ENG-2) and tactics fixtures cannot be reproduced. Since computer parameters never change after creation (Q57/Q72), changing the engine version in a later release would also change computer strength. Install location and engine discovery: architecture D-18 and D-25 (accepted 2026-10-26).
- **Priority:** Must.
- **Verification:** I: the engine's `id name` reply reports the pinned version (Stockfish 19) on both architectures. I: the build fails if the downloaded archive's checksum differs. Inspection: the CLI install and `Chess.app` bundle the same version. Inspection (step 2, TBC-7): `lipo -archs` (or equivalent) on the bundled `stockfish` lists both `arm64` and `x86_64`.

### IF-5 (AGREED 2026-10-26, Q74/Q75) Full-data export format
The full-data export (DAT-2) shall be one ZIP archive file holding:
- one PGN file with all games (IF-1);
- JSON files for Players (with UUIDs), ratings (both scales, with history), settings, and per-game records (the stored fields that PGN tags and comments do not carry, and the analysis results; `games.json`, added 2026-10-28, CR-4);
- a format version.

Import shall accept every earlier released format version, and reject a newer one with a message. Import merges, with the collision prompt of DAT-2 (AGREED, Q51).
- **AGREED 2026-10-26 (Q74):** the most common format among chess apps.
  - Interpretation (Decision Log): games as PGN, the de facto standard that Lichess and Chess.com both export; Players, ratings, and settings, which PGN cannot hold, as JSON; everything in one archive.
  - **AGREED 2026-10-26 (Q75):** the archive is a ZIP file, and the version rules above (accept every earlier released format version, reject a newer one).
  - File names and contents: architecture D-29 (accepted 2026-10-26): `manifest.json`, `games.pgn`, `players.json`, `ratings.json`, `settings.json`.
  - **Amended 2026-10-28 (CR-4; UG-12):** D-29 gains `games.json`, keyed by game ID. It holds, for each game, the stored fields that neither PGN tags nor PGN comments carry (for example Elo before and after on the game's scale, whether analysis was used, and the parameters the computer Players used), and the analysis results: per-move evaluations, centipawn loss, depth, and engine version (DAT-3, ANA-3). Clocks and per-move times keep travelling as PGN comments, and saved variations stay in `games.pgn` as PGN variations (architecture §5.7). No format version has been released, so this is format version 1 as first released; an archive with no `games.json` is still accepted, and its games get no analysis results.
- **Rationale/Source:** DAT-2; DAT-1 (schema migrations) shows the store will change between versions, so exports will too.
- **Priority:** Must.
- **Verification:** I: export→wipe→import round-trip (DAT-2) for fixture stores at each released version. I: the export is a valid ZIP file. I: the archive's PGN file opens in an independent PGN reader. I (added 2026-10-28, CR-4): `games.json` holds an entry for every game, and an archive without it imports. I: a file with a newer format version is rejected with a message. Inspection: the format document matches the exporter's schema.

---

## 13. Risks (requirements level)

| ID | Risk | Source | Handling |
|---|---|---|---|
| RISK-1 | macOS Tahoe is expected to be the last macOS release that supports Intel Macs. PLAT-2 requires support for Tahoe and later releases. If later releases drop Intel, Intel Macs stay supported only on Tahoe, and the Intel build must be kept and tested on Tahoe alongside newer releases. | Q46, Q64 (user) | At each app release, record the current macOS releases and which run on Intel (PLAT-2 checklist). Keep Intel builds (IF-4) in CI while any supported release runs on Intel. *(Added 2026-10-28, CR-4: this handling is a step of the release procedure, use case UC-29.)* |
| RISK-2 | Computer Players' ratings now change after rated games on both scales, including c–c games (Q41, Q59, Q70), while their engine parameters never change (Q57, Q72). If users run many rated c–c games, or many games against one computer (several at once are allowed, Q77), then that computer's rating and category label can drift away from the strength set at calibration. The Elo that humans gain or lose against it then becomes less meaningful. | Q41, Q57, Q59, Q70, Q72, Q77 | Accepted by the user's decisions; ratings self-correct with results. Run tests, calibration, and soak runs (NFR-3, ENG-2) on an isolated store, so they never change the user's ratings. *(Added 2026-10-28, CR-4: the mechanism is `--data-dir` or `CHESS_DATA_DIR` (CLI-4). A scripted run without it is a real c–c game and changes the ratings of the computers in it.)* |

---

## Open Questions

Architecture inputs resolved 2026-10-24: Rust backend, TypeScript UI, Tauri desktop shell, one shared core (see [architecture.md](architecture.md)).

**At baseline v1.0 (2026-10-26) no user question is open.** New questions raised after the baseline are added here with the next free number (Q80 onward) and answered through change control.

**Amendment 1 (2026-10-27):** no new numbered question. The user is asked to confirm or correct the interpretations recorded in the Decision Log on 2026-10-27 (CR-1 voided-game handling; CR-2 checkmate on the fifty-move-completing move, and 4.5:1 for all text in UI-7).

**Amendment 2 (2026-10-28):** no new numbered question. The user is asked to confirm or correct the interpretations recorded in the Decision Log on 2026-10-28 (CR-4), in particular: PLY-4 (no un-archive; archiving refused while a game is unfinished), RULE-2 (draw-offer rules), CLI-4 (scripted runs are stored and rated unless `--data-dir` is used), and IF-2 (duplicate import offers "skip" and "keep both"). *Confirmed by the user on 2026-10-28 (CR-6.3).*

### Resolved
- Q1–Q40: see Decision Log. Removed from scope by decision: Strict-mode analysis during play (Q15).
- Q41–Q55: answered 2026-10-26, round 1.
- Q56–Q74: answered 2026-10-26, round 2. Q64's post-Tahoe part and Q69 were answered late in round 2.
- Q75–Q79: answered 2026-10-26, final round (Decision Log):

| Q | Requirement(s) | Question (as asked) | Answer |
|---|---|---|---|
| Q75 | see list below | Do you accept all of the items below as written? Each one accepted becomes AGREED with priority Must. | **Yes, all.** Each listed item is AGREED 2026-10-26 with priority Must. Values cited from memory carry a one-time source check (TBC-2–TBC-5). *(All four closed 2026-10-27, CR-2.)* |
| Q76 | PLY-7, UI-2, UI-5 | After you reset a Player, should that Player's past games stay in their opponents' history and stats? | **Yes.** They stay in opponents' history and stats, and are hidden only from the reset Player's own views. |
| Q77 | GAME-4, GAME-1, PLY-5 | Because the one-unfinished-game limit applies to computer Players too, nobody can start a game in that mode against a computer that already has an unfinished game. Is that OK? | **No.** Computers are never blocked by unfinished h–c games; any Player can always play any computer. The limit applies to each human Player, and to computers only in c–c (Strict) games. |
| Q78 | ELO-1 | Is the "first 30 rated games at K = 40" counted separately for Strict and for Training? | **Yes.** |
| Q79 | PLY-4, ELO-1, UI-5 | Should a computer's category label follow its Strict rating? | **Yes, Strict.** |

**Items accepted under Q75** (formerly PROPOSED or TBR; now AGREED, Must):
- **Whole requirements:** DAT-4, DAT-6, UI-6, UI-8, UI-9, CLI-5, NFR-7, NFR-8, NFR-9, IF-1, IF-3, IF-4.
- **Parts of requirements:**
  - PLAT-4: loopback only; the option overrides the config file; a port in use gives a message and a non-zero exit.
  - ENG-4: the failure-handling statement.
  - DAT-5: the integrity check and restore offer.
  - ANA-4: opening analysis from history.
  - IF-2: everything except multi-game files (already AGREED).
  - IF-5: the version rules and the ZIP format.
- **Amendments:**
  - PLAT-1: `chess serve` wording.
  - RULE-1 (a)–(c): "shall" wording, the FIDE deviation note, and insufficient material only.
  - RULE-4 (a): whose material counts at flag fall.
  - RULE-4 (b): the computer-clock pass criterion.
  - RULE-5 and ENG-3: compound statements split (new RULE-6, RULE-7, RULE-8, ENG-5).
  - DAT-3 (a): bundle an ECO table.
  - DAT-3 (c): store the computer Player's ID and parameters.
  - CLI-1: crash wording, and restoring the window size on exit.
  - GUI-2 (a): touch in the statement.
  - GUI-2 (b): pass criterion for mixing input methods.
  - UI-2: empty-state criterion.
  - UI-4: split the statement, and run axe on desktop too.
  - NFR-1: startup start and end points.
  - NFR-6: include bundled assets.
- **TBR details:**
  - PLY-2: headless runs skip profile selection.
  - PLY-4: the user picks the category for newly generated computers.
  - PLY-6: names counted in user-perceived characters.
  - DAT-2: collision identity; a new UUID and name when keeping both; "apply to all"; the scope of "all data"; cancelling leaves the store unchanged.
  - DAT-4: no live cross-process refresh.
  - UI-1: side/color meaning; settings that apply to the CLI.
  - UI-2: by-mode breakdown.
  - UI-10: the user may pick a theme instead of following the system.
  - GUI-5: side panels scroll vertically at small sizes.
  - IF-1: UTF-8 (TBR-3).
  - IF-2: variations kept; duplicate games treated as collisions.
- **Accepted, with a one-time source check:** TBC-2 (FIDE Laws edition and deviation wording), TBC-3 (PGN Standard 1994 and its encoding clause), TBC-4 (WCAG 2.2 contrast criteria), TBC-5 (Nielsen's 0.1 s limit). *(All four closed 2026-10-27, CR-2; TBC-6 remains for the PGN tag rules.)*

### TBD / TBR / TBC register (baseline v1.0, 2026-10-26; Amendment 1, 2026-10-27)
TBD = value unknown · TBR = value proposed, needs review · TBC = to confirm · EST = engineering estimate accepted under Q53. "Close by" refers to the implementation steps in [architecture.md §13](architecture.md): step 1 = `chess-rules` + `chess-domain`; step 2 = store, engine, app, CLI; step 3 = GUI and Tauri shell.

Status values: **Closed** = settled, with the source · **EST** = accepted estimate, to be confirmed by the named measurement · **Open (design)** = a value the owner sets during design; it needs no user decision unless it changes user-visible behaviour · **Source check** = accepted value whose source text is checked once · **Fact check** = a fact about a chosen component, checked by the owner at the step shown.

| ID | Item | Requirement | Owner | Close by | Status |
|---|---|---|---|---|---|
| TBD-1 | Minimum macOS version | PLAT-2 | User | — | **Closed** (Q46): macOS Tahoe |
| TBD-2 | Supported browser versions | PLAT-2 | User | — | **Closed** (Q64): versions from the 3 years before v1 release |
| TBD-3 | Default web server port | PLAT-4 | Architect | — | **Closed** (D-24 accepted 2026-10-26): 8765 |
| TBD-4 | Name length and characters | PLY-6 | User | — | **Closed** (Q48, Q75): 128 user-perceived characters (grapheme clusters). The PLY-6 statement was aligned to this on 2026-10-28 (CR-4) |
| TBD-5 | c–c move-delay range and default | GAME-2 | Test owner | Step 2 (watch-screen E2E) | EST: 0–10 s, 0.5 s steps, default 1 s |
| TBD-6 | Games per preset for computer-clock test | RULE-4 | Test owner | Step 2 (first clock run) | EST: 20 |
| TBD-7 | Clock loss after a hard kill | RULE-5 | User | — | **Closed** (Q54): last recorded value; recording EST every move and ≥ 1 per s |
| TBD-8 | Calibration tolerance | ENG-2 | Test owner | Step 2 (first calibration run) | EST: ±150 Elo |
| TBD-9 | Significance level for category ordering | ENG-2 | Test owner | Step 2 (first calibration run) | EST: one-sided p < 0.05, ≥ 100 games |
| TBD-10 | Lowest Elo for Beginner computer Players | ENG-2 | User | — | **Closed** (Q41): starts at 400, no cap |
| TBD-11 | Baseline Mac | ENG-3, NFR-1, NFR-8 | User | — | **Closed** (Q47): M4, ≥ 16 GB RAM, macOS Tahoe |
| TBD-12 | Cancel → best-so-far move | ENG-5 | Test owner | Step 2 (engine benchmark) | EST: 200 ms |
| TBD-13 | Engine no-reply timeouts | ENG-4 | Architect | Step 2 (engine benchmark) | EST: 5 s readiness; 2 s past move limit |
| TBD-14 | "Accuracy" definition | DAT-3 | User | — | **Closed** (Q53): average centipawn loss; no accuracy % in v1. The definition is settled; it needs no measurement |
| TBD-15 | Confirm-move behaviour | UI-1, GUI-2 | User | — | **Closed** (Q65): no setting; commit points in GUI-2/CLI-3 |
| TBD-16 | Sound events | UI-1 | User | — | **Closed** (round 2): no sound in v1 |
| TBD-17 | Unit of "average game length" | UI-2 | User | — | **Closed** (Q73): moves and time |
| TBD-18 | Screen-reader announcement wording | UI-6 | Architect, with the user's review of the sample wording | Step 3 (before the VoiceOver walk-through) | Open (design). UI-6 itself is AGREED (Q75) |
| TBD-19 | Range of N (analysis lines) and search limit | ANA-1 | Test owner | Step 2 (analysis benchmark) | EST: N 1–5, default 3; first result ≤ 1 s; depth 20 or 10 s |
| TBD-20 | Blunder / mistake / inaccuracy thresholds | ANA-3 | Test owner | Step 2 (fixture games) | EST: 50 / 100 / 300 cp |
| TBD-21 | Headless exit codes per failure class | CLI-5 | Architect | — | **Closed** (CR-5, 2026-10-28): 0 result, 1 usage or setup error, 2 engine unavailable, 3 store error, 4 stopped by a signal (architecture §6.1, D-36). The runner is built at step 2 |
| TBD-22 | GUI minimum window size | GUI-5 | User | — | **Closed** (Q55): no minimum; GUI scales |
| TBD-23 | Move loss on power loss / OS crash | NFR-2 | User | — | **Closed** (Q54): no completed move lost |
| TBD-24 | Custom PGN tag names | IF-1 | Architect | Step 1 (PGN exporter) | Open (design). Architect's proposal: `Mode`, `Rated`, `WhiteClass`, `BlackClass` (architecture §5.7); `EndStatus` (`aborted` or `voided`, only for games that ended without a result) added 2026-10-27 (D-30, CR-1) |
| TBD-25 | Maximum PGN import file size | IF-2 | Architect | Step 2 (importer) | Open (design). Architect's proposal: 10 MB, EST (architecture §5.7) |
| TBD-26 | Pinned Stockfish version | IF-4 | Architect (D-9) | — | **CLOSED 2026-10-27** (CR-3, user: "use the latest Stockfish as the baseline"), as a version choice: Stockfish 19, tag `sf_19`, published 2026-09-05, not a pre-release. Source: https://github.com/official-stockfish/Stockfish/releases/tag/sf_19 (GitHub release API). A later release needs a Decision Log entry and a new calibration run (ENG-2). Open facts: TBC-7, TBC-8, TBC-9 |
| TBD-27 | Full-data export layout | IF-5 | User | — | **Closed** (Q74, Q75, D-29): one ZIP archive, PGN + JSON |
| TBD-28 | Stale-lease takeover time | DAT-4 | Architect | Step 2 (lease integration test) | **Closed** (architecture §10.1, D-23): 0 s. The journal lock is released by the OS when the owner exits or crashes, so another process can take over at once |
| TBD-29 | Server config file location and format | PLAT-4 | Architect | — | **Closed** (D-24 accepted 2026-10-26): `config.toml` (TOML) in the data directory |
| TBD-30 | Time for another front-end to stop a game voided by a reset | PLY-7 | Architect | Step 2 (lease integration test) | EST (added 2026-10-27, CR-1): 2 s, the lease heartbeat of architecture D-30 |
| TBD-31 | Largest number of computer Players created by one request | PLY-4 | Architect | — | EST (added 2026-10-28, CR-4): 1 to 25, default 5 |
| TBD-32 | Diagnostic log size and rotation | NFR-10 | Architect | Step 2 (store and app) | EST (added 2026-10-28, CR-4): 10 MB per file, 3 files |
| TBD-33 | Automatic backup cadence and retention | DAT-5 | Architect | Step 2 (store; backup-size and restore-time check) | EST (added 2026-10-28, CR-5; architecture D-35): start-up backup when the newest is older than 24 h; newest 3 automatic backups kept |
| TBR-1 | WCAG 2.2 AA contrast values | UI-7 | User | — | **Closed** (Q45, Q68): accepted; nothing beyond AA. Source verified 2026-10-27 (TBC-4 closed) |
| TBR-2 | 100 ms input feedback | NFR-8 | Test owner | Step 2 (latency test) | EST (Q53); NFR-8 AGREED (Q75). Citation verified 2026-10-27 (TBC-5 closed); the value is still confirmed by the latency measurement |
| TBR-3 | UTF-8 PGN encoding | IF-1 | User | — | **Closed** (Q75): UTF-8 accepted |
| TBC-1 | Supported terminals | PLAT-2 | User | — | **Closed** (Q46): Terminal and iTerm2 |
| TBC-2 | FIDE Laws edition and deviation wording | RULE-1, RULE-2, RULE-4 | User (check the current FIDE Laws) | — | **CLOSED 2026-10-27** (CR-2). Edition: FIDE Laws of Chess effective 1 January 2023 (approved by the FIDE General Assembly on 7 August 2022), https://handbook.fide.com/chapter/E012023. Verified: Art. 5.2.2 dead position ends the game automatically; Art. 9.2 threefold repetition and Art. 9.3 fifty-move rule are draws only "upon a correct claim by a player having the move"; Art. 9.6 fivefold repetition and the 75-move rule are automatic (9.6.2: checkmate on the final move takes precedence). RULE-1 (b) confirmed; RULE-1 (c) recorded as a limitation. The FIDE Council (23 August 2026) is discussing a draft revision: re-checked at each release (release checklist) |
| TBC-3 | PGN Standard (1994) as the reference, and its ISO 8859-1 encoding clause | IF-1, IF-2 | User | — | **CLOSED 2026-10-27** (CR-2). "Portable Game Notation Specification and Implementation Guide", revised 1994.03.12, coordinator Steven J. Edwards; copy at https://phh.readthedocs.io/en/latest/_static/PGN_Reference.txt. Verified: §4.1 "a subset of the eight bit ISO 8859/1 (Latin 1) character set" (the UTF-8 deviation stands); import format (flexible) and export format (strict) are defined. Remaining part: TBC-6 |
| TBC-4 | WCAG 2.2 success criteria 1.4.3 and 1.4.11: numbers, 4.5:1 and 3:1 values, ratio formula | UI-7, UI-8 | User | — | **CLOSED 2026-10-27** (CR-2). WCAG 2.2, W3C Recommendation 12 December 2024, https://www.w3.org/TR/WCAG22/. Verified: SC 1.4.3 (AA) text ≥ 4.5:1, large-scale text (≥ 18 pt, or 14 pt bold) ≥ 3:1; SC 1.4.11 (AA) UI components and states, and required graphical objects, ≥ 3:1 against adjacent colors; ratio (L1 + 0.05) / (L2 + 0.05). The relative-luminance definition was not read; it is checked in the UI-7 unit test of the ratio function |
| TBC-5 | Nielsen (*Usability Engineering*, 1993) as the source of the 0.1 s limit | NFR-8 | User | — | **CLOSED 2026-10-27** (CR-2). "Response Times: The 3 Important Limits", excerpt from Chapter 5 of *Usability Engineering* (1993), https://www.nngroup.com/articles/response-times-3-important-limits/; cites Miller (1968) and Card et al. (1991). Verified: 0.1 s is the limit for the user to feel the system reacts instantaneously. The 100 ms value stays EST (TBR-2) |
| TBC-6 | PGN Standard: Seven Tag Roster text, export-format tag spacing rules, the `*` result token, and the `Termination` value `abandoned` | IF-1 | Test owner | Step 1 (before the PGN exporter) | **Fact check** (opened 2026-10-27, CR-2). Read the named parts of the TBC-3 source and confirm the IF-1 export matches them |
| TBC-7 | Architectures in the Stockfish 19 macOS asset (`stockfish-macos-universal.tar.gz`): arm64 and x86_64 both present, or separate builds needed | IF-4, PLAT-2 | Architect | Step 2 (engine build script) | **Fact check** (opened 2026-10-27, CR-3; same fact as architecture ASM-3). Not verified: the binary could not be downloaded and run. "Universal binaries" in the release notes means run-time CPU-feature dispatch, which does not prove both architectures |
| TBC-8 | `UCI_Elo` range of Stockfish 19, including its floor | ENG-2 | Architect | Step 2 (engine build script, before the first calibration run) | **Fact check** (opened 2026-10-27, CR-3; same fact as architecture ASM-3). Read from the `option name UCI_Elo` line of the pinned engine's `uci` reply. No floor value is assumed in the requirements |
| TBC-9 | Exact GPL version of Stockfish 19 | ENG-1, NFR-6 | Architect | Step 2 (engine build script) | **Fact check** (opened 2026-10-27, CR-3). The release notes say "GPL"; read the version from the shipped `COPYING` file and confirm it is compatible with GPL-3.0-or-later (D-10) |
| TBC-10 | macOS facts behind PLAT-1 and PLAT-6: (a) arm64 executables need at least an ad-hoc signature; (b) a locally built app that was never downloaded carries no quarantine mark; (c) the steps to open a blocked app on macOS 26; (d) `/usr/local/bin` and `~/.local/bin` behaviour for `chess install-cli` | PLAT-1, PLAT-6, NFR-5 | Architect | Step 3 (packaging; the M walkthrough on macOS 26) | **Fact check** (opened 2026-10-28, CR-4). Stated from general knowledge of macOS; not verified on a macOS 26 machine |

## Out of Scope (AGREED 2026-10-24, v1)
Online play, matchmaking, accounts with passwords, chess variants (960, etc.), opening-book trainers, mobile native apps, Windows/Linux packaging.
- **Added 2026-10-26:**
  - Network play in v1. It is a planned future enhancement (Q43).
  - FEN export (Q53).
  - Languages other than English (Q52).
  - Sound effects (round 2).
  - Recalibrating computer Players after release (Q72).
- **Added 2026-10-28 (CR-4):**
  - Un-archiving a human Player (PLY-4).
  - An installer package, a download service, an update check, and automatic updates (PLAT-6, PLAT-3).
  - A command that deletes the user's data (PLAT-6).

## Decision Log
| Date | Decision | Requirement(s) |
|---|---|---|
| 2026-10-24 | Initial draft created from user's 15 top-level requirements plus Claude's proposals. | all |
| 2026-10-24 | Central data store = one local shared file on the Mac; web served locally (localhost). | DAT-1, PLAT-1, PLAT-3 |
| 2026-10-24 | Engine = Stockfish (GPLv3); app must be GPL-compatible if distributed. | ENG-1 |
| 2026-10-24 | Computer opponents have a fixed nominal Elo per difficulty level; games vs computer are rated. *(Superseded 2026-10-26 by Q41: computer Players start at their category baseline and their Elo changes.)* | ELO-1 |
| 2026-10-24 | Training mode is human-vs-computer only; human–human and computer–computer are Strict only. | PLY-1, PLY-5, RULE-3 |
| 2026-10-24 | Game time (total and per-Player) is persisted with game stats. | DAT-3 |
| 2026-10-24 | Starting Elo comes from Player's self-assessment at registration: Beginner, Novice, Intermediate, Master, Grand Master. Numeric values pending. | PLY-2, ELO-1 |
| 2026-10-24 | Starting/anchor Elo: Beginner 400, Novice 600, Intermediate 1200, Master 2200, Grand Master 2500 (user edited ELO-1 directly). K=40 for first 30 games then 32. No Elo re-assessment/reset. No cap on Elo; Grand Master = 2500 or higher. *(2026-10-26: "no Elo reset" reaffirmed by Q62; a data reset keeps both ratings.)* | ELO-1, PLY-2 |
| 2026-10-24 | Self-play (same human, or same computer, on both sides) is unrated, excluded from W/L/D stats, kept in game history. | ELO-1, UI-2, DAT-3 |
| 2026-10-24 | Beginner/Novice computers use low skill, shallow depth, and random errors; all classes calibrated empirically via matches. | ENG-2 |
| 2026-10-24 | Clock presets: untimed, 1+0, 3+2, 5+0, 10+5, 15+10, 30+0, plus custom; clocks allowed in every mode and matchup. *(Superseded on 2026-10-24 by "Clocks are Strict-mode only; Training is untimed" below.)* | RULE-4 |
| 2026-10-24 | Default play mode is Strict. | PLY-5 |
| 2026-10-24 | Rewind: clock keeps running while rewound; computer move arriving while rewound leaves the view in place with an indicator; either hot-seat human may rewind. GAME-1 and GAME-2 accepted. | GAME-1/2/3 |
| 2026-10-24 | **Strict mode has no analysis during play.** Analysis only after the game ends. Humans may rewind to view past moves read-only and resume at the live position; computers cannot rewind. | PLY-5, ANA-1, ANA-3, GAME-3 |
| 2026-10-24 | Clocks are Strict-mode only; Training is untimed. | RULE-4, GAME-1 |
| 2026-10-24 | Class bands (<600, 600–1199, 1200–2199, 2200–2499, ≥2500); computer Elo within band, Grand Master computers 2500–3000. Ratings stored as reals, displayed rounded. *(Superseded in part 2026-10-26 by Q41: computer Players start at the category baseline, and the 2500–3000 Grand Master computer range no longer applies. Bands now also set computers' displayed category, Q57.)* | ELO-1, ENG-2 |
| 2026-10-24 | Roster: 5 computer Players per class (25), fixed seed, human-readable names, user can generate more, cannot be deleted. Human Players: rename; delete = archive. | PLY-4 |
| 2026-10-24 | Computer never resigns/offers draws; accepts draw only if eval ≤ 0; threefold and fifty-move draws are automatic. | RULE-1/2 |
| 2026-10-24 | Custom-FEN games unrated; imported PGNs review-only; clocks pause while the app is closed. | RULE-5, ELO-2 |
| 2026-10-24 | Settings: "difficulty" → default opponent class; "think time" → maximum only. | UI-1 |
| 2026-10-24 | Per-move evaluations computed after game ends, in the background. | DAT-3 |
| 2026-10-24 | Added viewing stats for any Player (human or computer) and a Computer Players screen. | UI-5 |
| 2026-10-24 | NFR-1…6 and Out-of-Scope list accepted; web touch = tap-to-move only (no mobile layout); desktop = unsigned local build. *(2026-10-26: "no mobile layout" superseded for layout scaling by GUI-5, Q55.)* | NFR, GUI-2, PLAT-1 |
| 2026-10-24 | Sections 8–10 reviewed and marked AGREED by user (proposed wording within them treated as accepted unless listed in Open Questions). | ANA, CLI, GUI |
| 2026-10-24 | Human selects matchup and mode at the start of each game; computer–computer is an explicit human-selected Strict option with two chosen computer Players (may be the same one). | PLY-1, PLY-5 |
| 2026-10-24 | Human–human is hot-seat on one screen with registered Players; the same Player may play both sides. | PLY-3 |
| 2026-10-24 | Computer Players: five classes (same labels as self-assessment) with varied-but-similar parameters; static Elo; stats tracked; only human Elo changes; c-v-c changes no Elo. *(Superseded in part 2026-10-26: computer Elo changes (Q41); c–c games between different computers are rated (Q59).)* | PLY-4, ENG-2, ELO-1 |
| 2026-10-24 | Clocks / time controls are in v1. | RULE-4 |
| 2026-10-24 | RULE-3 (takeback) and ELO-1 agreed as written. | RULE-3, ELO-1 |
| 2026-10-24 | Two play modes: Training (analysis + takeback, unrated) and Strict (restricted analysis of past positions only, no takeback, rated). Post-game full analysis offered after every game. *(Superseded in part: "restricted analysis of past positions" was replaced by "Strict mode has no analysis during play" above; "Training unrated" was replaced 2026-10-26 by the separate Training Elo scale, Q70.)* | PLY-5, RULE-3, ELO-2, ANA-1, ANA-3 |
| 2026-10-24 | CLI amended to a full-screen ratatui-style TUI; plain line-oriented mode kept for screen readers and scripting. TUI details PROPOSED (Q38–Q40). | CLI-1..4, UI-2, UI-4 |
| 2026-10-24 | Q38: TUI details accepted (screens, 80×24 minimum, terminal hygiene, plain mode, key bindings). Q39: no keyboard board cursor in the CLI; moves are typed only. Q40: no mouse support in the TUI. | CLI-1, CLI-3, CLI-4 |
| 2026-10-25 | PLAT-3 strengthened: desktop, web page, and CLI are fully usable offline in v1, including first launch; no run-time Internet fetches of any kind; explicit offline verification. | PLAT-3, NFR-4 |
| 2026-10-26 | Requirements review against ISO/IEC/IEEE 29148, the INCOSE GtWR, and architecture D-1–D-11. PROPOSED amendments and new PROPOSED requirements (PLAT-4, PLY-6, ENG-4, DAT-4–6, UI-6–9, ANA-4, CLI-5, GUI-5, NFR-7–9, IF-1–5), a corrected ELO-2 verification, Q41–Q56, and a TBD register. Records the review, not a decision. | see header |
| 2026-10-26 | **User decisions, round 1 (Q41–Q55):** | see each Q |
| | **Q41:** every Player, human or computer, is put in one of five categories (Beginner, Novice, Intermediate, Master, Grand Master) when created, and starts at that category's baseline Elo. After each rated Strict game, Elo goes up or down by the Elo formula. This replaces the static/nominal computer Elo, the within-band computer Elo (GM 2500–3000), and "only human Elo changes". *(Round 2: K-factor confirmed by Q58; c–c rating confirmed by Q59.)* | ELO-1, ELO-2, PLY-4, ENG-2, UI-5, DAT-3 |
| | **Q42:** humans may offer draws in h–h and h–c; the computer never offers one. The existing AGREED rule stays: the computer accepts only at its own eval ≤ 0. | RULE-2 |
| | **Q43:** v1 has no network play and stays fully offline. Network play is recorded as a planned future enhancement. NFR-4's opt-in clause is dropped. | PLAT-3, NFR-4, PLY-3, Out of Scope |
| | **Q44:** the CLI-3 bishop/b-file rule is accepted. | CLI-3, CLI-2, GUI-2 |
| | **Q45:** WCAG 2.2 AA contrast values are accepted (UI-7 AGREED; every theme meets them). | UI-7, UI-3, UI-4 |
| | **Q46:** targets are macOS Tahoe on Intel or Apple M4 or newer; Safari, Chrome, Firefox, Edge; Terminal and iTerm2. RISK-1 recorded (Tahoe is expected to be the last macOS release for Intel). *(Round 2, Q64: widened to any Apple silicon.)* | PLAT-2, PLAT-3, IF-4, RISK-1 |
| | **Q47:** the baseline Mac is an M4 with at least 16 GB RAM, running macOS Tahoe. | ENG-3, NFR-1, NFR-8 |
| | **Q48:** names are up to 128 Unicode characters and unique. A Player's data and statistics can be reset after confirmation (new PLY-7). At launch the user picks a profile or registers a new user. *(Round 2: reset scope set by Q62; identity by Q63.)* | PLY-2, PLY-6, PLY-7, ELO-1 |
| | **Q49:** unfinished games save automatically. At most one unfinished game per mode; it must be finished or aborted before a new game in that mode. Aborting a Strict game counts as a loss for Elo. Aborted games can be analysed. (New GAME-4.) *(Round 2: the abort rules are superseded by Q60 (resign is the only deliberate end; abort only for c–c watchers); the limit is per Player (Q61).)* | GAME-4, GAME-1, GAME-2, RULE-5, ELO-2, ANA-4, CLI-4, PLY-5 |
| | **Q50:** a c–c watcher can only scroll the move list (no rewind) and can see the analysis after the game. | GAME-2, GAME-3, CLI-4 |
| | **Q51:** import merges. On collision the user chooses: replace, or keep both as distinct records. Multi-game PGN files are in scope. If a previous store is missing, the user is alerted and chooses to start fresh or import from a file. | DAT-2, DAT-5, IF-2, IF-5 |
| | **Q52:** all of a UI's move-input methods are available during play (no default-method setting). "Confirm move" stays open, reworded (Q65). Sound toggles game sound effects. The GUI has several color themes, including light and dark, and follows the system theme (new UI-10). v1 is English only (new UI-11). Ctrl-C asks before quitting. Resign and archive always ask for confirmation. *(Round 2: sound removed from v1 entirely; Q65 closed with no confirm-move setting.)* | UI-1, UI-10, UI-11, GUI-2, CLI-1, CLI-4, RULE-2, PLY-4 |
| | **Q53:** Claude set engineering estimates (EST) for engine and analysis values on the M4/16 GB baseline (ENG-2 calibration, ENG-3 cancel, ENG-4 timeouts, ANA-1 N and search limits, ANA-3 thresholds, DAT-3 accuracy, NFR-8, GAME-2 move delay, RULE-4 test size). The clock keeps running while the engine is unavailable. FEN export is not in v1. Analysis stays human-turn only (Claude's choice, keeping the AGREED ANA-1 verification). | ENG-2, ENG-3, ENG-4, ANA-1, ANA-3, DAT-3, NFR-8, GAME-2, RULE-4, IF-3 |
| | **Q54:** after any disconnect, hard reset, closure, or crash, each clock is restored to its last recorded value and no completed move is lost. Moves recorded at play time take precedence over the shared store when they disagree. Written without naming a design. | RULE-5, NFR-2 |
| | **Q55:** the server port is user-configurable by CLI option or config file. The user said "backend database port"; the SQLite store has no port, so this was read as the local web server port (PLAT-4, D-8). The GUI has no minimum size and scales to any space (GUI-5 replaced). The TUI minimum fits all content (80×24); when the terminal is smaller, the TUI asks it to resize, and shows an "enlarge" message if it does not. *(Round 2: Q67 confirmed the port applies only to `chess serve` and the browser URL; the TUI minimum is fixed at 80×24.)* | PLAT-4, GUI-5, CLI-1 |
| 2026-10-26 | New open questions Q57–Q75 from applying Q41–Q55, plus the architect's gaps G-4 (Q69), G-6 (Q70), G-9 (Q71), and G-10 (Q72). | see Open Questions |
| 2026-10-26 | **User decisions, round 2 (Q56–Q68, Q70–Q74, TUI minimum, sound).** "Interpretation" marks the coordinator's reading of an answer; each one is recorded as an interpretation, not as the user's words. | see each Q |
| | **Q56:** every AGREED requirement is a mandatory "shall". *Interpretation:* priority = Must for all AGREED requirements. No rationale is requested. | all |
| | **Q57:** a computer's category label follows its current rating, and its engine settings stay fixed. *Interpretation (Claude):* the label follows the Strict rating (confirm with Q79). *(Confirmed 2026-10-26 by Q79.)* | PLY-4, ENG-2, ELO-1, UI-5 |
| | **Q58:** computers use the same K-factor as humans (40 for the first 30 rated games, then 32). *Interpretation (Claude):* the 30-game count is kept per scale (confirm with Q78). *(Confirmed 2026-10-26 by Q78.)* | ELO-1 |
| | **Q59:** c–c games between different computers change both ratings. If the watcher aborts a c–c game, neither computer's Elo or stats change. | ELO-2, GAME-2, CLI-4 |
| | **Q60:** an interruption the player didn't choose (crash, failure, power loss, and so on) is not an abort; the game stays cached and resumes, and no stats or Elo change. A deliberate end by a player is a resignation and counts as a loss for that player in Elo and W/L/D. *Interpretations:* in hot-seat h–h the app asks which side resigns; human games have no separate abort command (only resign); abort stays only for a watcher ending a c–c game; resigned and c–c-aborted games can be analysed. Replaces the round-1 "aborted Strict game = loss" rule. | GAME-4, ELO-2, RULE-2, ANA-4, CLI-4, GAME-1 |
| | **Q61:** one unfinished game per mode per Player, not per app. c–c games count toward the Strict limit. *Interpretation:* they count for each computer Player involved. *Interpretation (Claude):* the per-Player limit also applies to computers in h–c games (confirm with Q77). *(Withdrawn 2026-10-26 by Q77: computers hold an unfinished-game slot only in c–c games, and are never blocked from h–c games.)* | GAME-4, GAME-1 |
| | **Q62:** a reset clears everything except the Player's Elo ratings (Training and Strict). Computers can't be reset. The 2026-10-24 "no Elo reset" rule still holds. *Interpretation (Claude):* the current value and rated-game count on each scale are kept, and a new history point is written; whether the Player's past games stay in opponents' records is Q76. *(Settled 2026-10-26 by Q76: they stay; they are hidden only from the reset Player's own views.)* | PLY-7, PLY-4, ELO-1 |
| | **Q63:** a Player is identified internally by a UUID, not by name. Names are shown with their case kept. *Interpretation:* uniqueness is case-sensitive ("Brian" and "brian" are different), and uniqueness still applies (Q48), against every Player. | PLY-6, DAT-2 |
| | **Q64:** include M1–M3 Macs, since one arm64 build covers all Apple silicon and Tahoe supports M1 and newer, so the platform is macOS Tahoe on Intel or any Apple silicon. The M4/16 GB baseline stays. Browsers: Safari, Chrome, Firefox, and Edge versions released in the last 3 years, and no older. *Interpretation (Claude):* the 3 years are counted back from the v1 release date. Post-Tahoe macOS support is still open. | PLAT-2, PLAT-3, PLAT-4 |
| | **Q65:** there is no "confirm move" setting. A mouse or touch move commits when the piece is dropped; a SAN move commits on Enter. *Interpretation:* a keyboard-overlay move commits when the destination square is selected. | UI-1, GUI-2, CLI-3 |
| | **Q66 (architecture D-14):** when the browser tab closes, the clock is suspended and the game is recorded. | RULE-5, NFR-2 |
| | **Q67:** the port setting applies only to `chess serve` and the browser URL. | PLAT-4 |
| | **Q68:** the high-contrast theme needs nothing beyond WCAG AA. *Interpretation (Claude):* no minimum light/dark square contrast. | UI-3, UI-7 |
| | **Q70 (G-6):** Training games count in W/L/D and have their own Training Elo, tracked separately from Strict Elo. *Interpretation:* Training Elo starts at the same category baseline. *Interpretation (Claude):* both sides of a Training game, human and computer, are rated on the Training scale *(confirmed by the user 2026-10-26, final round)*. Replaces "Training games are casual and never change Elo". | ELO-1, ELO-2, PLY-2, PLY-4, PLY-5, UI-2, UI-5, DAT-3 |
| | **Q71 (G-9):** the CLI and desktop app must never be different versions; they ship and update together, and a mismatch is detected and refused with a message (new PLAT-5). | PLAT-5, DAT-1, NFR-5, NFR-9 |
| | **Q72 (G-10):** no recalibration of computer Players after release. | ENG-2, PLY-4, Out of Scope |
| | **Q73:** average game length is measured in both moves and time. | UI-2, UI-5 |
| | **Q74:** use the most common format among chess apps. *Interpretation:* games export as PGN, the de facto standard that Lichess and Chess.com both export; the full-data export is one archive with a PGN of all games plus JSON for Players, ratings, and settings. The ZIP archive format is a further TBR. *(Accepted 2026-10-26 by Q75.)* | IF-5, DAT-2, IF-1 |
| | **TUI minimum:** fixed at 80×24. | CLI-1 |
| | **Sound:** removed from v1 entirely; the sound setting is removed and sound effects are added to Out of Scope. | UI-1, PLAT-3, NFR-6, Out of Scope |
| | **Q64 (late answer):** the app must support macOS Tahoe and the releases after it. Written as "Tahoe and every later release current at an app release". The macOS releases current at each app release are checked and recorded, then verified (PLAT-2). | PLAT-2, RISK-1 |
| | **Q69 (G-4, late answer):** in a c–c game, the active (watching) profile's maximum-think-time setting applies. In h–h games the computer plays no moves, so the setting applies only to engine analysis, if relevant. *Interpretation:* this is recorded as an interpretation. "Engine analysis" in h–h means post-game analysis (Strict has none during play), where the setting caps search time per position. | ENG-3, UI-1 |
| | Still open after round 2: Q75 (remaining PROPOSED and TBR items, listed under Open Questions), and new Q76–Q79 raised by the interpretations above. | see Open Questions |
| 2026-10-26 | **User decisions, final round (Q75–Q79, computer Training Elo, architecture decisions, ASM-1) and baseline v1.0.** "Interpretation" marks the coordinator's or Claude's reading of an answer. | see each Q |
| | **Q75:** every item still marked PROPOSED or TBR is accepted. Each becomes AGREED 2026-10-26 with priority Must (Q56 rule). Values Claude proposed from memory (FIDE Laws, PGN Standard and its encoding, WCAG 2.2 criteria, Nielsen's 0.1 s limit) are accepted, and each gets a one-time source check (TBC-2–TBC-5). *Interpretation (Claude):* accepting the "split the compound statement" amendments means splitting them now, at the baseline: RULE-5 → RULE-5 (auto-save and resume), RULE-6 (PGN export), RULE-7 (PGN import), RULE-8 (FEN starting positions); ENG-3 → ENG-3 (think time) and ENG-5 (cancel), with the non-blocking clause carried by NFR-8. UI-4 becomes the parent of UI-6, UI-7, UI-8, and GUI-2 (b). TBR-3 closed. | PLAT-1, PLAT-4, PLY-2, PLY-4, PLY-6, RULE-1, RULE-3, RULE-4, RULE-5–RULE-8, ENG-2–ENG-5, DAT-2–DAT-6, UI-1, UI-2, UI-4, UI-6, UI-8, UI-9, UI-10, ANA-4, CLI-1, CLI-5, GUI-2, GUI-5, NFR-1, NFR-6–NFR-9, IF-1–IF-5 |
| | **Q76:** after a Player is reset, their old games stay in opponents' history and statistics. They are hidden only from the reset Player's own views. | PLY-7, UI-2, UI-5 |
| | **Q77:** computers are not blocked by unfinished games. Human–computer games never use up a computer's unfinished-game slot, so any Player can always play any computer. The one-unfinished-game-per-mode limit applies to each human Player, and to computers only in c–c games (Strict). This corrects GAME-4, whose round-2 interpretation said the opposite. | GAME-4, GAME-1, PLY-5, RISK-2 |
| | **Computer Training Elo (final round):** computers also have a Training Elo that changes in Training games. This confirms the Q70 interpretation. | ELO-1, PLY-4, UI-5 |
| | **Q78:** the K = 40 rule for the first 30 rated games is counted separately for Strict and for Training. | ELO-1, PLY-7 |
| | **Q79:** a computer's category label follows its Strict rating. | PLY-4, ELO-1, UI-1, UI-5 |
| | **Architecture decisions:** the user accepted every decision the architect had PROPOSED: D-12, D-13, D-16, D-18, D-19, D-20, D-22–D-29. Requirements cite them where relevant. D-24 closes TBD-3 (default port 8765) and TBD-29 (`config.toml` in the data directory). | PLAT-1, PLAT-2, PLAT-4, PLAT-5, GAME-4, RULE-5, DAT-1–DAT-4, ELO-1, PLY-7, CLI-4, NFR-2, NFR-7, IF-4, IF-5 |
| | **ASM-1 confirmed:** one person uses each Mac account, so the local web server needs no access token (D-22). If this changes, PLAT-4 and NFR-7 are reopened. | PLAT-4, NFR-7 |
| 2026-10-26 | **Baseline v1.0.** Every requirement is AGREED (priority Must) or DELETED; no PROPOSED or TBR item remains; no user question is open. Remaining open items are design values (TBD-18, TBD-21, TBD-24–TBD-26, TBD-28), EST values awaiting measurement, and one-time source checks (TBC-2–TBC-5). Later changes follow change control. | all |
| 2026-10-27 | **Amendment 1 to baseline v1.0 (CR-1–CR-3).** Three user decisions applied through change control. The baseline statement is kept; no ID renumbered. Impact: requirements and verification listed per CR below; interfaces IF-1, IF-4; no margin or budget affected; risks unchanged (RISK-2 still applies to computer ratings). | see each CR |
| | **CR-1 (user decision, closes the architect's OQ-8 / G-20):** "allow it; the unfinished game does not count against either player." A reset is allowed while the Player has unfinished games, and the confirmation prompt says they will be voided. A voided game has no result and no effect on any Player's Elo, rated-game count, W/L/D, or statistics (the reset Player, a human opponent, or a computer), and it frees the reset Player's unfinished-game slot. It is not a resignation, so the Q60 loss rule does not apply. Replaces the architect's rule that refused a reset while a game was unfinished (architecture D-26, OQ-8). | PLY-7, GAME-1, GAME-4, RULE-5, DAT-3, ELO-1, ELO-2, UI-2, UI-5, ANA-4, IF-1 |
| | **CR-1 interpretations (Claude; for the user to correct):** (1) The voided game stays in the opponent's game history, marked "voided by reset", with no result; it can be analysed, like a watcher-aborted c–c game (ANA-4); it is hidden from the reset Player's own views (Q76). (2) A voided c–c game cannot occur, because only humans can be reset and humans never play in c–c games. (3) A voided game is no longer unfinished, so it also frees a human opponent's slot, and it can never be resumed. (4) A voided self-play game is hidden entirely. (5) A game that is open for play in another front-end process is voided too; that front-end stops it and tells its user (architecture D-30). (6) A voided game is exported as PGN with `Result "*"`, `Termination "abandoned"`, and the custom tag `EndStatus "voided"` (architecture §5.7, D-30; TBD-24; standard tag values checked under TBC-6). These match the architect's interpretation Q-A18. | PLY-7, GAME-4, ANA-4, DAT-4, IF-1 |
| | **CR-2 (user asked for the source checks to be made against the web):** TBC-2, TBC-3, TBC-4, and TBC-5 closed with the verified sources recorded in the register. Findings applied: RULE-1 names the FIDE Laws edition effective 1 January 2023, confirms (b) as a deliberate deviation from Art. 9.2 and 9.3, and states (c) as a limitation relative to Art. 5.2.2; IF-1 cites PGN §4.1 and names the export format, and IF-2 the import format; UI-7's non-text clause is worded as SC 1.4.11 scopes it, and the ratio formula is stated; NFR-8 cites Nielsen's "Response Times: The 3 Important Limits". The "from memory" caveats were removed from the requirement text and verification. New TBC-6 (PGN Seven Tag Roster, export-format tag spacing, `*` token; test owner, step 1). Release-checklist item added (not a requirement change): at each release, check whether a newer FIDE Laws edition is in force (the FIDE Council on 23 August 2026 agreed to keep discussing a draft revision). | RULE-1, RULE-2, IF-1, IF-2, UI-7, NFR-8, register |
| | **CR-2 decisions and interpretations (Claude; for the user to correct):** (1) UI-7 keeps its AGREED wording, so 4.5:1 applies to all text, which is stricter than SC 1.4.3 (it allows 3:1 for large-scale text). Q68 said nothing beyond AA is required; adding the large-text allowance is a one-line change if the user prefers exact AA. (2) The relative-luminance definition, not read in the check, is carried by the UI-7 unit test instead of a register item. (3) When the move that completes the fifty-move count gives checkmate, the checkmate stands, as FIDE Art. 9.6.2 rules for the 75-move rule. | UI-7, RULE-1 |
| | **CR-3 (user decision: "use the latest Stockfish as the baseline"):** IF-4 pins Stockfish 19 (tag `sf_19`, published 2026-09-05, not a pre-release; GitHub release API). TBD-26 closed as a version choice. A later release needs a Decision Log entry and a new calibration run (ENG-2) before it is pinned. ENG-2 no longer assumes a `UCI_Elo` floor value. Not verified, because the binary could not be downloaded and run: the macOS asset's architectures (TBC-7) and the `UCI_Elo` range (TBC-8); the exact GPL version is read from the shipped `COPYING` file (TBC-9). Owner of all three: architect, step 2. | IF-4, ENG-1, ENG-2, NFR-6, register |
| 2026-10-28 | **Amendment 2 to baseline v1.0 (CR-4).** The user delegated the closing of all use-case gaps (UG-1–UG-22, A-4, A-5) and functional-decomposition gaps (FG-1–FG-12, FA-1–FA-6) to the systems engineer, with no questions to the user. Each decision below is the systems engineer's judgement ("Interpretation (Claude)"), recorded for the user to correct. No ID renumbered; four requirements added. Impact: requirements and verification listed per decision; interfaces IF-1, IF-2, IF-5, CLI-4; architecture items for the architect are in docs/handoff_se_to_architect.md; no margin or budget affected; RISK-2 handling clarified. | see each decision |
| | **CR-4.1 (UG-19, FG-9, TBD-4):** names are unique case-sensitively against every Player (active or archived, human or computer); length is counted in grapheme clusters; names are trimmed and compared after NFC; control and line-break characters are rejected. Architecture §5.5 is wrong and is corrected by the architect. | PLY-6 |
| | **CR-4.2 (UG-22):** a duplicate PGN game raises the DAT-2 prompt with "skip" (default) and "keep both"; "replace" is not offered, because the contents are identical and a replacement could overwrite a rated played game. The functional model's F-6.11 is aligned by the architect. | IF-2, DAT-2 |
| | **CR-4.3 (UG-12, UG-11):** the full-data archive gains `games.json` for the stored per-game fields that PGN tags and comments do not carry and for analysis results; variations stay PGN variations; unfinished games are exported and restored as unfinished unless that breaks the slot limit; an unfinished game exports with `Result "*"` and no `EndStatus`. | DAT-2, IF-5, IF-1, RULE-6 |
| | **CR-4.4 (UG-3):** "move now" is the command `movenow`, key F2 in the TUI and GUI, and a "Move now" button in the GUI thinking indicator; available only while a computer thinks in an h–c game. | ENG-5, CLI-4, GUI-4 |
| | **CR-4.5 (UG-4, FG-8 a, b):** new PLAT-6: the app is built locally; `chess install-cli` links `chess` into `/usr/local/bin` or `~/.local/bin`; update replaces the bundle; uninstall removes the link and the bundle and keeps the data; macOS 26 is declared as the minimum; the Gatekeeper step is documentation, not product behaviour. The build ad-hoc signs (TBC-10). | PLAT-6, PLAT-1, NFR-5 |
| | **CR-4.6 (UG-5):** takeback stops at the human's first turn (after the computer's first move when the human has Black); during a search it undoes one move; taken-back moves leave the record; nothing to undo gives the message "Nothing to take back". | RULE-3 |
| | **CR-4.7 (UG-6):** a draw is offered only on one's own turn; a pending offer blocks moves and stops all clocks; a computer answers within its think-time maximum; after a decline the same side must move before offering again. | RULE-2 |
| | **CR-4.8 (UG-7):** the resume list holds every unfinished game the active Player plays on either side, plus all unfinished c–c games; resigning a hot-seat game from the offer asks which side resigns. | GAME-1 |
| | **CR-4.9 (UG-8, A-4):** registering at launch makes the new Player active; registering later does not; the active Player can be switched when no game is open. | PLY-2 |
| | **CR-4.10 (UG-9, FG-8 c):** computer Players cannot be renamed; a human with an unfinished game cannot be archived; the active Player cannot archive itself; there is no un-archive in v1. | PLY-4, GAME-4 |
| | **CR-4.11 (UG-10):** a generation request creates 1 to 25 Players (default 5) in the chosen category; the name generator skips used names, so no collision prompt exists. | PLY-4 |
| | **CR-4.12 (UG-13, A-5):** scripted runs are c–c only and Strict; they are stored and rated like any c–c game; `--data-dir` (or `CHESS_DATA_DIR`) isolates them. ELO-2 stays unchanged. | CLI-4, CLI-5, RISK-2 |
| | **CR-4.13 (UG-14):** `save` is defined as a confirmation that prints the stored move count; it changes nothing. | CLI-4 |
| | **CR-4.14 (UG-15):** an invalid `config.toml` stops `chess serve` with exit 1 and a message; a browser that fails to open leaves the server running with the URL printed; Ctrl-C or SIGTERM stops the server with exit 0 and suspends open games. | PLAT-4 |
| | **CR-4.15 (UG-16, UG-18):** custom-start games are excluded from W/L/D and aggregates, as self-play is; empty statistics show 0 and "—"; unfinished evaluations show "evaluating…" with a count; the CLI draws the Elo charts in character cells with a table. | UI-2, ANA-3 |
| | **CR-4.16 (UG-17, UG-21):** a failed pre-upgrade backup stops the upgrade; store-changing actions are all-or-nothing (new DAT-7); an unreadable journal resumes from the longest intact run, or the game can be voided as "voided: unreadable"; after a restore, all retained journals are replayed and finished games are finalized again in order. | DAT-5, DAT-6, DAT-7 |
| | **CR-4.17 (UG-1, UG-2, FG-10, FG-1):** NFR-6 and RISK-1 are verified by a release procedure, which a new enabling-system use case (UC-29) describes. The build, calibration, and release functions (F-3.9, F-8.6–F-8.10) stay in the functional model as flagged life-cycle functions. | NFR-6, RISK-1, PLAT-2 |
| | **CR-4.18 (FG-7):** of the ten derived functions, F-2.5.7 becomes UI-12 and F-6.14 becomes NFR-10; F-6.9 is covered by PLAT-5 and DAT-6; F-6.3 and F-6.4 by NFR-2 and DAT-4; F-2.1.4, F-3.9, F-7.2.6, F-7.4.2, and F-7.4.6 are accepted as derived design functions that need no requirement of their own. | UI-12, NFR-10 |
| | **CR-4.19 (FG-6):** no performance budget is allocated to functions for NFR-1 and NFR-8; the system-level measurements in their verification stand, and the architect may allocate budgets in design (architecture §4.8). | NFR-1, NFR-8 |
| | **CR-4.20 (UG-20, FA-2, FA-4):** the use-case set keeps one external actor `User` with roles, and `stockfish` as a supporting actor; a Release engineer role is added for the enabling system (UC-29). | — |
| 2026-10-28 | **Amendment 3 to baseline v1.0 (CR-5).** Final consistency pass under the user's delegation (no questions). Impact: requirements DAT-5, DAT-6, CLI-4, CLI-5, RULE-2, UI-12, PLAT-4 and the register; no ID added or renumbered; architecture, functional decomposition (new F-6.15) and use cases aligned; no margin or budget affected. Each decision is the systems engineer's judgement for the user to correct. | see each CR |
| | **CR-5.1 (architect's pushback 1, OQ-12, R-19; FG-14):** accepted. The automatic backup (start-up, newest older than 24 h, newest 3 kept, TBD-33) is stated in DAT-5, because DAT-5 and DAT-6 assume a recent backup exists. Registration and the other non-game actions are not journalled: the cost (a journal for Player data) is not justified for at most about 24 hours of such changes, so DAT-5 states the limit instead. A game that cannot be finalised keeps its journal in `journal/unrestored/` and is named in the restore message. | DAT-5, DAT-6 |
| | **CR-5.2 (pushback 3, architect-chosen items):** scripted runs stay computer–computer only; scripted human games use plain mode with piped input (CLI-4). Exit code 4 (stopped by a signal) accepted; TBD-21 closed. The data-command names, `chess games`, `--on-duplicate` and `--yes` are accepted as design (D-36), not requirements. `--config` and `CHESS_CONFIG` are accepted as test-only. | CLI-4, CLI-5, PLAT-4 |
| | **CR-5.3 (R-20):** the indefinite clock stop of an unanswered hot-seat draw offer is accepted as the stated rule; no change. | RULE-2 |
| | **CR-5.4 (H-20 SE note):** the offer to void an unreadable game is not a UI-12 action, because the offer is itself the explicit choice. | UI-12, DAT-7 |
| 2026-10-28 | **Amendment 4 to baseline v1.0 (CR-6).** The user's answers to the readiness check for detailed design (SRR-SFR-readiness.md). Impact: no requirement text, ID, verification, margin or budget changed; the answers set the project's criticality, coding standard and approvers (docs/engineering/SWE-001), confirm earlier interpretations, and record that no Intel test hardware exists. Use cases and functional decomposition (v1.0) unaffected. These are user decisions. | — |
| | **CR-6.1 (FIND-001):** the user said: "ordinary, not safety or regulated; use CERT as the coding standard and use standard Rust and TS checks with warnings treated as errors". The software has ordinary criticality. SEI CERT is the coding standard with the standard Rust and TypeScript checks and every warning an error. A development constraint, so no requirement ID; the tools and scope are assumptions of SWE-001 (ASM-S1 to ASM-S4). Not answered: whether the loopback server (NFR-7) is security-critical for design-gate purposes (readiness FIND-016). Open: how CERT applies to Rust and TypeScript (FIND-014). | — |
| | **CR-6.2 (FIND-002):** the user said: "yes, SA agent approves changes inside a component, I approve anything else". The system-architect agent approves one-component changes as a desk review (recorded as such); the user approves everything wider. | — |
| | **CR-6.3 (FIND-003):** the user answered "yes" to whether approving the use cases also confirms the systems engineer's interpretations. Confirmed: PLY-4 (no un-archive; archiving refused while a game is unfinished), RULE-2 (draw-offer rules), CLI-4 (scripted runs are stored and rated unless `--data-dir` is used), IF-2 (duplicate import offers "skip" and "keep both"), and the 2026-10-27 interpretations: CR-1 void handling (PLY-7), checkmate on the move that completes the fifty-move count (RULE-1), and 4.5:1 for all text (UI-7). Architecture Q-A18 (a voided game's visibility) is answered in the same way (D-30). The other CR-4 and CR-5 decisions were approved with the use cases and the functional decomposition that carry them. | PLY-4, PLY-7, RULE-1, RULE-2, UI-7, CLI-4, IF-2 |
| | **CR-6.4 (FIND-009):** the user said there is no Intel Mac and "we won't be able to test that requirement anytime soon". PLAT-2 is **not changed**, and neither are IF-4, RISK-1 or architecture D-25. Its Intel verification (a smoke test on an Intel Mac, Intel builds passing the suite) can't be performed. Whether the verification is waived as an accepted risk, the requirement is relaxed, or hardware is found later is **not decided** (readiness FIND-015). | PLAT-2, IF-4, RISK-1 |
