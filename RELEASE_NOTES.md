# Release Notes

## Design baseline v1.0, Amendment 4: 2026-10-28

Documentation-only release. No application code exists yet.

### Added
- **docs/engineering/SWE-001-engagement-profile.md**: criticality, coding standard, design approvers, installed toolchain and assumptions.
- **docs/SRR-SFR-readiness.md**: the readiness check for detailed design. Run 1: NOT-READY (one blocker, no criticality or coding standard). Run 2, after the user's answers: **READY-WITH-CONDITIONS** for step 1 (`chess-rules`, `chess-domain`), with 12 findings open (7 Major, 5 Minor) and owners and closing steps for each. The use cases and functional decomposition baselined as v1.0 earlier the same day are the SFR inputs (entry below).

### Changed: user decisions (requirements Amendment 4, CR-6)
- The software has **ordinary criticality** (not safety-related, not regulated). **SEI CERT** is the coding standard, with the standard Rust and TypeScript checks and every warning treated as an error. No requirement text changed.
- Design changes inside one component are approved by the system-architect agent (desk review); anything wider by the user.
- The systems engineer's earlier interpretations are confirmed (PLY-4, RULE-2, CLI-4, IF-2, PLY-7, RULE-1, UI-7), and architecture Q-A18 is answered.
- No Intel Mac is available for testing. PLAT-2 is unchanged and its Intel verification can't be run (open, FIND-015).

### Open at this release
- As before, plus the open readiness findings (FIND-004 to FIND-008, FIND-010 to FIND-016), in particular how CERT applies to Rust and TypeScript (FIND-014) and the unverifiable Intel requirement (FIND-015).

## Design baseline v1.0: use cases and functional decomposition, 2026-10-28

Documentation-only release. No application code exists yet.

### Changed
- **docs/use_cases/** (README and UC-1 to UC-29) and **docs/functional_decomp.md** are baselined as **v1.0**, approved by the user. Content is unchanged from v0.3; only status, approver, version references and change logs changed. Further changes go through change control.

## Design baseline v1.0, Amendments 2 and 3: 2026-10-28

Documentation-only release. No application code exists yet. Under the user's delegation (no questions), the systems engineer and the architect closed every gap in the use-case set and the functional decomposition, then ran a consistency pass over all documents. The decisions are the systems engineer's and the architect's judgement and are open to the user's correction (requirements Decision Log, CR-4 and CR-5).

### Added
- **docs/use_cases/** (README and UC-1 to UC-29, v0.3): Cockburn use cases with a use-case diagram, five activity diagrams, requirement and function traces, and a coverage table (82 of 82 requirement and risk IDs). UC-29 covers the build and release enabling system.
- **docs/functional_decomp.md** (v0.3): 124 functions, 104 leaves, allocated to the architecture's named blocks (§2.2), with a requirement-to-function matrix.
- **docs/handoff_se_to_architect.md**: the systems engineer's change list for the architect (closed; the disposition is in architecture §10.4).

### Changed: requirements (Amendment 2, CR-4; Amendment 3, CR-5)
- **New requirements:** PLAT-6 (install, update, uninstall; `chess install-cli`), DAT-7 (failed writes, unreadable journals), UI-12 (confirmation of destructive actions), NFR-10 (local diagnostic log).
- **Rules settled:** names are unique case-sensitively and counted in grapheme clusters; takeback stops at the human's first turn; draw offers stop all clocks; "move now" is `movenow`, F2 and a GUI button; the resume list covers either side of a hot-seat game; computer Players cannot be renamed or archived; scripted runs are computer-vs-computer, stored and rated, with `--data-dir`; `chess serve` fails fast on a bad config and stops cleanly on Ctrl-C; the full-data archive gains `games.json` with the analysis results; duplicate PGN games raise a skip or keep-both prompt.
- **Data damage:** a failed pre-upgrade backup stops the upgrade; every store-changing action is all-or-nothing; an unreadable journal resumes from its longest intact run, or the game is voided as "voided: unreadable"; a restore replays every retained journal. **Amendment 3** adds the automatic backup (at start-up when the newest is older than 24 h, newest 3 kept; TBD-33) and states the limit of a restore (a game of a Player created after the backup keeps its journal but is not recovered).
- **Register:** TBD-31 to TBD-33 and TBC-10 added; TBD-21 closed (exit codes 0 to 4).

### Changed: architecture (amendment 2 and 3)
- Decisions D-31 to D-37, risks R-19 and R-20, a build-and-release context view (§2.1), named allocation blocks (§2.2), and the handoff disposition (§10.4). Amendment 3 is text-only: it records the systems engineer's acceptance of the architect's pushback.

### Open at this release
- As for Amendment 1, plus: macOS facts behind `install-cli` and ad-hoc signing (TBC-10, ASM-5), checked at packaging time.

## Design baseline v1.0, Amendment 1: 2026-10-27

Documentation-only release. No application code exists yet.

### Changed: user decisions
- **Reset with an unfinished game is allowed (PLY-7, D-30).** The unfinished game is *voided*: it has no result and no effect on either side's Elo, rated-game count or W/L/D, and it frees the Player's unfinished-game slot. It stays in the opponent's history marked "voided by reset", can be analysed, and is hidden from the reset Player's own views. A game live in another front-end stops at its next lease heartbeat (about 2 s, an estimate). The reset confirmation names the games it will void. Exported PGN uses `Result "*"`, `Termination "abandoned"` and the custom tag `EndStatus`. D-26 is revised, G-20 and OQ-8 are closed, and the data model, state machine, protocol, error table and journal gain a `Voided` state. The interpretation of how a voided game is shown is recorded for the user to correct (Q-A18).
- **Stockfish 19 is the pinned engine (D-9, IF-4, TBD-26):** tag `sf_19`, released 2026-09-05. The build prefers the official `stockfish-macos-universal` asset, accepted only if `lipo -info` shows both arm64 and x86_64, with a from-source fallback. The `UCI_Elo` floor is no longer hard-coded: the range is read from the engine at run time. Any later Stockfish release needs a decision-log entry and a calibration re-run.

### Changed: source checks closed against the primary texts
- **FIDE Laws of Chess (TBC-2):** the 2023 edition (in force from 2023-01-01) is current. Threefold repetition (Art. 9.2) and the fifty-move rule (Art. 9.3) are claims; fivefold repetition and the 75-move rule (Art. 9.6) are automatic. The app's automatic draw at threefold and fifty-move is stated as a deliberate deviation. Dead positions other than insufficient material are not detected (limitation against Art. 5.2.2). A re-check for a new edition is now a release-checklist item.
- **PGN standard (TBC-3):** revised 1994.03.12; section 4.1 specifies ISO 8859/1, so the UTF-8 export is a stated deviation. Export and import formats are named as the standard names them. The Seven Tag Roster text and the `Termination` value `abandoned` are still to be checked (new TBC-6).
- **WCAG 2.2 (TBC-4):** W3C Recommendation of 2024-12-12. SC 1.4.3 and 1.4.11 values (4.5:1 and 3:1) and the ratio formula are confirmed. UI-7 keeps 4.5:1 for all text, which is stricter than the standard.
- **Nielsen (TBC-5):** the 0.1 s limit is from "Response Times: The 3 Important Limits" (Chapter 5 of *Usability Engineering*, 1993), which cites Miller (1968) and Card et al. (1991). NFR-8's citation is corrected.

### Open at this release
- Could not be checked, because downloading and running the binary was blocked: the macOS Stockfish asset's architectures and the `UCI_Elo` range (TBC-7, TBC-8, ASM-3), and the GPL version (TBC-9). All close at the first engine build-script run.
- Mermaid diagrams are hand-checked only (no Node.js on the build machine).

## Design baseline v1.0: 2026-10-26

Documentation-only release. No application code exists yet. **Requirements and architecture are baselined.** Every requirement is AGREED at priority Must; every architecture decision is AGREED except D-21 (deferred).

### Changed: requirements (docs/requirements.md)
- **Reviewed to ISO/IEC/IEEE 29148 and INCOSE rules.** Fixes to agreed requirements were made as amendments with the old wording kept. No IDs were renumbered or deleted.
- **Added:** PLAT-4 (`chess serve`), PLAT-5 (one version for CLI and desktop), PLY-6 (names), PLY-7 (reset), ENG-4 (engine failure), ENG-5 (cancel), DAT-4 to DAT-6 (one front-end per live game, store integrity, backup before upgrade), GAME-4 (unfinished games), UI-6 to UI-11 (screen-reader announcements, contrast, focus, offline help, themes, English only), ANA-4, CLI-5, NFR-7 to NFR-9, RULE-6 to RULE-8 (PGN export, PGN import, FEN start positions, split from RULE-5), and IF-1 to IF-5 (PGN, FEN, bundled Stockfish, export archive).
- **User decisions (Q41 to Q79) applied:**
  - **Ratings:** every Player, human or computer, has a Strict and a Training Elo, starting at the category baseline. Ratings change by the Elo formula, with K = 40 for the first 30 rated games, counted per scale. A computer's category label follows its Strict rating. No recalibration after release.
  - **Ending games:** a crash or closed tab suspends a game with no effect. Humans can only resign, which is a loss. Abort exists only for a computer-vs-computer watcher and has no effect on stats.
  - **Draws:** humans can offer draws in human-human and human-computer games. The computer never offers one.
  - **Unfinished games:** one per mode per human Player. Computers are limited only in computer-vs-computer games and are never blocked in human-vs-computer games.
  - **Players:** UUID identity; unique, case-sensitive names of up to 128 characters; a reset keeps Elo and hides history from the reset Player only.
  - **Platform:** macOS Tahoe and later; Intel and Apple silicon; Safari, Chrome, Firefox and Edge from the last 3 years; Terminal and iTerm2. v1 is fully offline, with network play a possible future enhancement.
  - **Interface:** no sound and no confirm-move setting in v1; themes follow the system; WCAG 2.2 AA contrast; English only; the TUI minimum is 80x24 with a resize request.
  - **Data:** no completed move is ever lost, and clocks restore to the last recorded value. Imports merge, asking on collisions. The full-data export is one archive of PGN and JSON.
  - **Server:** the port is set by CLI option or config file.
- **Register:** a TBD/TBR/TBC register with owners and close-by steps, and a Decision Log.

### Changed: architecture (docs/architecture.md)
- **New sections:** concurrency model (D-13); error catalogue of 16 codes; protocol reliability (reconnect, idempotency, version check); internal interface contracts; data ownership; crash-safe move journal (D-23); one version everywhere (D-16); deployment and packaging (D-18); accessibility details; per-requirement traceability with orphan checks.
- **Decisions:** D-12 to D-29 recorded. D-12 (SQLite) and D-16 are one-way for data, and the user accepted the consequences. ASM-1 (one person per Mac account, so no access token on the local server) is confirmed.
- **Fixes to the design:** the engine now replies only after an accepted move; post-game analysis goes to the analysis service; every engine move is checked for legality; timeouts and tolerances are set from the M4 / 16 GB baseline.
- **Added risks:** R-9 to R-18, including Tahoe being the last macOS release to support Intel.

### Changed: other documents
- **docs/test-strategy.md** rewritten to match: two Elo scales, resign and interruption rules, per-Player unfinished-game limit, merge import, export archive, themes, no sound, no-lost-moves fault injection, the macOS release checklist, and isolated data stores for every test run.
- **README.md** rewritten to describe the baselined design.

## Design baseline v0.4: 2026-10-25

Documentation-only release. No application code exists yet. **All architecture decisions are now agreed.**

### Added
- **docs/trade-study-gui-board.md** (first pass, kept for the record). Four GUI board options scored against a requirements gate and ten weighted criteria, using research from the published source of chessground 10.5.0 and react-chessboard 5.12.1:
  - A: own React + SVG board with dnd-kit (91.0);
  - B: chessground as is (56.4, fails the gate);
  - C: chessground + our keyboard/accessibility layer (68.2);
  - D: react-chessboard + that layer (72.4).

  After it, the user ruled out building our own board for v1.
- **docs/trade-study-gui-board-refined.md**: refined study of C vs. D, with twelve criteria focused on how well each library hosts our keyboard/accessibility layer.
  - New design ideas: a transparent ARIA-grid overlay aligned over the visual board, instead of a hidden copy; and controlled selection for chessground.
  - D split into D1 (patched) and D2 (run-time workaround).
  - Scores: C 88.8, D1 87.4, D2 78.2, with sensitivity analysis showing C and D1 effectively tied.
  - Recommendation: C, decided by tie-breakers (mature pointer interaction; risk that sits in our own code; no library modification), gated by a 2-day spike.

### Changed
**docs/architecture.md**
- **D-4 AGREED:** `@lichess-org/chessground` for visuals and mouse/touch, plus our shared layer: a `BoardModel` reducer owning the one selection state, a transparent `AccessibleOverlay` ARIA grid (keyboard focus, screen-reader labels, visible focus on the real squares), a live region, a promotion picker, and a Reconciler that keeps chessground's selection in step. Fallback: react-chessboard + the same layer, patched (D1).
- §11.1 rewritten as the decided board design, with the spike's exit criteria; §6.2 GUI section and diagram updated; repository layout updated for `ui/src/board/`.
- New risk R-8 (selection drift between chessground and our reducer) with mitigations.
- §13 next steps reordered: core crates, then the CLI, then the GUI starting with the board spike.
- Header status: all decisions D-1–D-11 AGREED. Decision-log entries added.

**docs/test-strategy.md**
- New focus area **GUI board**: selection-sync sequences across mouse, drag, touch, keyboard, and typed input; overlay alignment; axe on the `aria-hidden` board; rejected pointer moves revert. Header updated.

**docs/requirements.md**
- Header notes that all architecture decisions are agreed, including the board that delivers GUI-2 and UI-4. No requirement changed.

**README.md**
- Status, GUI board technology row, and documentation index (both trade studies) updated.

## Design baseline v0.3: 2026-10-25

Documentation-only release. No application code exists yet.

### Changed: architecture decisions recorded
**docs/architecture.md**
- Section 11 renamed "Decisions". D-1–D-3 and D-5–D-11 are AGREED:
  - D-1: the desktop app links the core in-process and talks to it over Tauri IPC.
  - D-2: each entry point embeds the core; they coordinate through the SQLite file.
  - D-3: chess rules wrap `shakmaty` + `pgn-reader`.
  - D-5: React + TypeScript + Vite for the GUI.
  - D-6: full-screen ratatui TUI (agreed in v0.2).
  - D-7: Rust → TypeScript types with `ts-rs`.
  - D-8: the web server is started by the user with `chess serve`.
  - D-9: Stockfish pinned, downloaded and checksummed at build time, bundled with the app.
  - D-10: project license GPL-3.0-or-later.
  - D-11: legal-move lookups for what-if branches go through the core.
- **D-4 (GUI board) remains open.** New §11.1 compares three options in detail: our own React + SVG board (recommended), chessground used directly, and chessground with our own keyboard/accessibility layer.
- New §11.2 decision log.
- New §8.1 **Offline operation**: every run-time dependency and how it is met locally, enforcement through Content-Security-Policy and a CI bundle scan, offline E2E tests, and the build-vs-run distinction.
- §6.1 records that the TUI is keyboard only: moves typed, no board cursor, mouse capture never enabled.
- Header rewritten with current status and a change summary.

### Changed: offline operation is an explicit v1 requirement
**docs/requirements.md**
- **PLAT-3** strengthened: the desktop app, web page, and CLI are fully usable with no Internet connection, including first launch. Nothing is fetched at run time (no CDN scripts, web fonts, update checks, license checks, or telemetry). Building from source may download dependencies; running never does. New verification: offline E2E for each entry point with non-loopback networking blocked, a network-capture check, a CI scan of the GUI bundle for remote resources, a Content-Security-Policy in both the local server and the desktop app, and a check that the bundled Stockfish runs with no network.
- **CLI-1/3/4**: TUI details (Q38), typed-only moves (Q39), and no mouse (Q40) marked AGREED.
- Header "Current state" updated to 2026-10-25 with a change summary. Decision Log entries added.

**docs/test-strategy.md**
- Title no longer says "technology-neutral"; it points to architecture.md §9 for tools.
- Header status updated to 2026-10-25 with a change summary.
- Non-functional checks add offline E2E for desktop, web, and CLI, plus the GUI bundle scan (PLAT-3).

**README.md**
- Status, offline feature description, technology decisions table, and documentation index updated.

## Design baseline v0.2: 2026-10-24

Documentation-only release. No application code exists yet.

### Added
- **README.md**: project overview, planned features, technology decisions, and a documentation index.
- **docs/architecture.md** (new, 773 lines, DRAFT): architectural principles, system context and deployment, code organisation, the shared `chess-app` core, the front-end/core protocol, the three front-ends, persistence, cross-cutting concerns, a testing architecture mapped onto the test strategy, component-level requirements traceability, open decisions, risks, and next steps.
  - Settled inputs: Rust backend, TypeScript GUI, Tauri desktop shell, one shared core for CLI/web/desktop.
  - Key principles: the Rust core owns every rule and mode-policy decision; pure logic is separated from I/O (fake engine, fake clock, in-memory store for tests); Stockfish is never exposed to a front-end and all engine requests pass one policy gateway; every state change is durable in SQLite (WAL) before it is acknowledged.
- **RELEASE_NOTES.md**: this file.

### Changed: CLI is now a full-screen terminal UI
The CLI was previously specified as a scrolling ASCII board. It is now a full-screen, ratatui-style TUI (Decision Log, 2026-10-24; Q38–Q40 resolved).

**docs/requirements.md**
- **CLI-1** rewritten as "Full-screen terminal UI". The board, players/status, clocks, SAN move list, and input line are shown in panels that update in place. New agreed details:
  - Screens: registration, new-game setup, play, computer–computer watch, post-game analysis and what-if, statistics, Computer Players, and settings, all keyboard-navigated.
  - Minimum terminal size 80×24, with an "enlarge the terminal" message below that and immediate reflow on resize.
  - Terminal always restored (cursor, echo, main screen) on quit, Ctrl-C, panic, or crash; Ctrl-Z suspends and `fg` redraws.
  - No mouse support in v1.
  - Plain mode (`--plain`): line-oriented transcript, chosen automatically when stdin/stdout is not a terminal; serves screen readers and piping. Scripted runs never start the TUI.
- **CLI-2**: coordinate labels apply to the board panel and plain mode.
- **CLI-3**: moves are typed only (no keyboard board cursor, Q39), so arrow keys stay free for rewind; invalid input shows an error in the status area and leaves the text editable.
- **CLI-4**: added key bindings that can never begin a move (←/→ back/forward, End live, PgUp/PgDn scroll moves, F1 help, Tab switch panel, Esc cancel). Letters, digits, `-`, and `=` always go to the input line. Commands disallowed by the play mode are shown as unavailable with an explanation. Added watch-screen keys for computer–computer games: Space pause/resume, `s` step, `+`/`-` delay, `a` abort (with confirmation).
- **UI-2**: statistics are a TUI screen with tables and an Elo-over-time chart; plain mode prints a text table and ASCII sparkline.
- **UI-4**: screen-reader support in the CLI is provided through plain mode, since full-screen TUIs are not reliably readable by screen readers.
- Header status, Open Questions, and Resolved (now Q1–Q40) updated. The architecture inputs are recorded as resolved. Two Decision Log entries were added.

**docs/test-strategy.md**
- Unit level adds board renderer and TUI screen snapshots at fixed terminal sizes.
- End-to-end level adds PTY-driven TUI sessions (screen parsed by a terminal emulator) and plain-mode transcripts.
- Notation and input: TUI key bindings must never swallow move characters.
- New focus area, **Terminal UI**: layout at minimum size and on resize; terminal restored after quit, Ctrl-C, and panic; plain mode selected when not a TTY; screen-reader use via plain mode.

### Status at this release
- All requirements (sections 1–11) and the out-of-scope list are AGREED; no requirements questions are open.
- The architecture is a DRAFT for iteration. Nothing is implemented.

## Initial commit: 2026-10-24

- Repository created and published to GitHub.
- Added `.gitignore`, `docs/requirements.md`, and `docs/test-strategy.md`.
