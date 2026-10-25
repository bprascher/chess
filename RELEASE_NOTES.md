# Release Notes

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
