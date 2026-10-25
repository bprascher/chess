# Chess

An offline, single-machine chess application for macOS with three front-ends (terminal UI, local web UI, desktop app) on top of one shared Rust core, using Stockfish as the engine.

> **Status: design phase (2026-10-25).** No code is implemented yet. This repository holds the agreed requirements, the test strategy, and the architecture, with all decisions agreed, plus the trade studies behind the GUI board choice. See [RELEASE_NOTES.md](RELEASE_NOTES.md) for what has been produced so far.

## Planned features

- **Three ways to run**: a full-screen terminal UI (ratatui-style, with a plain line-oriented mode for screen readers and scripting), a local web UI, and a Tauri desktop app. All share one data store and one core.
- **Full FIDE rules**, takeback, save/resume, PGN import/export, optional clocks (presets and custom).
- **Players**: registered humans and computer Players in five strength classes, with static Elo for computers.
- **Two play modes**:
  - *Training*: human vs. computer, unrated and untimed, with takeback and in-game analysis.
  - *Strict* (default): rated, clocks, no in-game analysis; full-game analysis after the game.
- **Elo for humans** only, starting at 400 / 600 / 1200 / 2200 / 2500 from a self-assessment, K = 40 for the first 30 games and 32 after.
- **Statistics**: W/L/D totals, by color and opponent, Elo over time, game history with replay.
- **Analysis**: post-game analysis of every game, what-if scenarios, read-only rewind in both modes.
- **Themes and accessibility**: at least 10 board themes and 5 piece sets (including high-contrast), keyboard operation, screen-reader labels.
- **Offline**: the desktop app, web page, and terminal UI are all fully usable with no Internet connection, including first launch; nothing is fetched at run time and there is no outbound traffic.

## Technology decisions

| Topic | Decision |
|---|---|
| Backend / core | Rust (rules, engine control, persistence, servers, CLI) |
| GUI | TypeScript, React + Vite |
| Desktop shell | Tauri |
| Chess rules | `shakmaty` + `pgn-reader`, wrapped in a `chess-rules` crate |
| Engine | Stockfish, pinned and bundled with the app, run only inside the Rust process |
| Storage | SQLite (WAL mode) |
| Front-end ↔ core | One serializable protocol: in-process (CLI), Tauri IPC (desktop), localhost WebSocket (web); TypeScript types generated with `ts-rs` |
| Processes | Each entry point embeds the core; they coordinate through the shared SQLite file. The web server is started with `chess serve`. |
| License | GPL-3.0-or-later |
| GUI board | `@lichess-org/chessground` for visuals and mouse/touch, plus our own accessible keyboard overlay (ARIA grid) and selection reducer; fallback react-chessboard |

## Documentation

| Document | Contents |
|---|---|
| [docs/requirements.md](docs/requirements.md) | All requirements (every one AGREED), each with verification test cases, plus out-of-scope list and decision log |
| [docs/test-strategy.md](docs/test-strategy.md) | Test levels, focus areas, determinism, quality gates |
| [docs/architecture.md](docs/architecture.md) | Architecture: principles, code organisation, core, protocol, front-ends, persistence, offline operation, decisions and decision log, risks, next steps |
| [docs/trade-study-gui-board.md](docs/trade-study-gui-board.md) | First-pass trade study of GUI board options (kept for the record) |
| [docs/trade-study-gui-board-refined.md](docs/trade-study-gui-board-refined.md) | Refined trade study, chessground vs. react-chessboard, that decided D-4 |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Changes by release |

## Out of scope for v1

Online play, matchmaking, accounts with passwords, chess variants (e.g. 960), opening-book trainers, native mobile apps, Windows/Linux packaging.

## Next steps

See section 13 of [docs/architecture.md](docs/architecture.md).
