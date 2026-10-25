# Chess

An offline, single-machine chess application for macOS with three front-ends (terminal UI, local web UI, desktop app) on top of one shared Rust core, using Stockfish as the engine.

> **Status: design phase.** No code is implemented yet. This repository currently holds the agreed requirements, the test strategy, and a draft architecture. See [RELEASE_NOTES.md](RELEASE_NOTES.md) for what has been produced so far.

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
- **Offline**: no network dependency and no outbound traffic.

## Technology decisions

| Topic | Decision |
|---|---|
| Backend / core | Rust (rules, engine control, persistence, servers, CLI) |
| GUI | TypeScript |
| Desktop shell | Tauri |
| Engine | Stockfish, run only inside the Rust process |
| Storage | SQLite (WAL mode) |
| Front-end ↔ core | One serializable protocol: in-process (CLI), Tauri IPC (desktop), localhost WebSocket (web) |

## Documentation

| Document | Contents |
|---|---|
| [docs/requirements.md](docs/requirements.md) | All requirements (every one AGREED), each with verification test cases, plus out-of-scope list and decision log |
| [docs/test-strategy.md](docs/test-strategy.md) | Test levels, focus areas, determinism, quality gates |
| [docs/architecture.md](docs/architecture.md) | Draft architecture: principles, code organisation, core, protocol, front-ends, persistence, risks, next steps |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Changes by release |

## Out of scope for v1

Online play, matchmaking, accounts with passwords, chess variants (e.g. 960), opening-book trainers, native mobile apps, Windows/Linux packaging.

## Next steps

See section 13 of [docs/architecture.md](docs/architecture.md).
