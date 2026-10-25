# Release Notes

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
