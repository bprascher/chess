# Chess

An offline, single-machine chess application for macOS with three front-ends (terminal UI, local web UI, desktop app) on top of one shared Rust core, using Stockfish as the engine.

> **Status: requirements and architecture baselined as v1.0 (2026-10-26), with the architecture amended on 2026-10-27 (reset with unfinished games, Stockfish 19).** No code is implemented yet. This repository holds the requirements, the test strategy, and the architecture, plus the trade studies behind the GUI board choice. Every requirement and every architecture decision is agreed, except the GUI Elo chart library, which is deferred to implementation. What remains open needs measurement or a check during implementation, not a user decision (see the TBD register in [docs/requirements.md](docs/requirements.md) and §14 of [docs/architecture.md](docs/architecture.md)). See [RELEASE_NOTES.md](RELEASE_NOTES.md) for what has been produced so far.

## Planned features

- **Three ways to run**: a full-screen terminal UI (ratatui-style, minimum 80×24, with a plain line-oriented mode for screen readers and scripting), a local web UI started with `chess serve`, and a Tauri desktop app. All three share one data store and one core, and they ship together as one bundle, so they are always the same version.
- **Full FIDE rules**, takeback (Training), PGN import/export (including multi-game files), and optional clocks (presets and custom, Strict only).
- **Players and profiles**: at launch you choose a registered Player or register a new one. Names can be up to 128 characters and are unique, and Players are identified internally by a UUID. Computer Players come in five strength classes, and their engine settings never change.
- **Two play modes**:
  - *Training*: human vs. computer, untimed, with takeback and in-game analysis. It has its own Training Elo.
  - *Strict* (default): clocks, no in-game analysis, and full-game analysis after the game. It has its own Strict Elo.
- **Elo for every Player, human and computer**, on two separate tracks (Training and Strict). Both start at the category baseline: 400 / 600 / 1200 / 2200 / 2500 for Beginner / Novice / Intermediate / Master / Grand Master. Humans pick their category at registration; a computer gets its category when it is created. K = 40 for the first 30 rated games on a track, then 32, counted separately for each track. Computers have a Training Elo too, which changes in Training games. A computer's category label follows its current Strict rating. Computer-vs-computer games between different computers are rated. Self-play and games from a custom start position are never rated.
- **Unfinished games**: games save automatically, and each human Player can have one unfinished game per mode. A computer is limited only in computer-vs-computer games (one unfinished at a time), so you can always play any computer, even one that is in another unfinished game. Crashes, power loss, and a closed browser tab pause the game (the clock freezes at its last recorded value). No completed move is ever lost, and an interruption never counts as a result. A human ends a game early only by resigning, which counts as a loss. Resetting a Player's data voids that Player's unfinished games, which count for nobody (see Statistics). Only a watcher of a computer-vs-computer game can abort it, and that changes nothing.
- **Statistics**: W/L/D totals (Training and Strict), by color and opponent, Elo over time for each track, game history with replay, and average game length in moves and time. A human Player's data can be reset after confirmation: everything except their Elo ratings is cleared from that Player's own views. Nothing is deleted, so their old games stay in their opponents' history and stats. A reset is allowed while the Player has unfinished games: the confirmation lists them, and each one is voided. A voided game has no result and counts for neither player (no Elo, no W/L/D), and it stays in the opponent's history, marked "voided by reset", where it can still be analysed.
- **Analysis**: post-game analysis of every game (including aborted computer-vs-computer games and games voided by a reset), what-if scenarios, and read-only rewind in both modes.
- **Themes and accessibility**: at least 10 board themes and 5 piece sets, and app colour themes including light and dark that follow the system appearance. Every theme meets WCAG AA contrast. Keyboard operation, screen-reader labels, and English-only text in v1. No sound effects in v1.
- **Data**: full-data export as one archive (a PGN of all games plus JSON for Players, ratings, and settings) and a merging import that asks how to handle each conflict.
- **Offline**: the desktop app, web page, and terminal UI are all fully usable with no Internet connection, including first launch. Nothing is fetched at run time and there is no outbound traffic. Network play is a possible future enhancement and is not in v1.

## Platforms

- macOS Tahoe and every later release, as a universal build: any Apple silicon Mac (M1 and newer), and Intel Macs for as long as Apple's macOS supports them.
- Browsers: Safari, Chrome, Firefox, and Edge releases from the last 3 years.
- Terminals: Terminal.app and iTerm2.

## Technology decisions

| Topic | Decision |
|---|---|
| Backend / core | Rust (rules, engine control, persistence, servers, CLI) |
| GUI | TypeScript, React + Vite |
| Desktop shell | Tauri |
| Chess rules | `shakmaty` + `pgn-reader`, wrapped in a `chess-rules` crate |
| Engine | Stockfish 19 (tag `sf_19`), pinned and bundled with the app as one universal arm64 + x86_64 binary, run only inside the Rust process. The build uses the official universal macOS release asset if it contains both architectures, and otherwise builds both from source |
| Storage | SQLite (WAL mode, fully synced), plus a per-game move journal flushed to disk before each move is acknowledged |
| Front-end ↔ core | One serializable protocol: in-process (CLI), Tauri IPC (desktop), localhost WebSocket (web); TypeScript types generated with `ts-rs` |
| Processes | Each entry point embeds the core; they coordinate through the shared data directory. The web server is started with `chess serve`. Its port comes from the `--port` flag, then the config file, then the default |
| Packaging | One `Chess.app` bundle containing the desktop app, the `chess` CLI, and Stockfish; `chess` is put on `PATH` with a symlink |
| License | GPL-3.0-or-later |
| GUI board | `@lichess-org/chessground` for visuals and mouse/touch, plus our own accessible keyboard overlay (ARIA grid) and selection reducer; fallback react-chessboard |

## Documentation

| Document | Contents |
|---|---|
| [docs/requirements.md](docs/requirements.md) | Requirements (baseline v1.0) with verification test cases, TBD register, out-of-scope list, and decision log |
| [docs/test-strategy.md](docs/test-strategy.md) | Test levels, focus areas, determinism, quality gates |
| [docs/architecture.md](docs/architecture.md) | Architecture (baseline v1.0): principles, code organisation, core, protocol, front-ends, persistence and move journal, offline operation, packaging, traceability, decisions and decision log, risks, open questions, next steps |
| [docs/trade-study-gui-board.md](docs/trade-study-gui-board.md) | First-pass trade study of GUI board options (kept for the record) |
| [docs/trade-study-gui-board-refined.md](docs/trade-study-gui-board-refined.md) | Refined trade study, chessground vs. react-chessboard, that decided D-4 |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Changes by release |

## Out of scope for v1

Online or network play (a possible future enhancement), matchmaking, accounts with passwords, chess variants (e.g. 960), opening-book trainers, native mobile apps, Windows/Linux packaging, sound effects, FEN export, and languages other than English.

## Next steps

See section 13 of [docs/architecture.md](docs/architecture.md).
