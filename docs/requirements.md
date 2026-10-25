# Chess Application — Requirements

**Status legend:** `AGREED` = confirmed by user · `PROPOSED` = suggested by Claude, awaiting decision · `DRAFT` = user's original wording, wording/details still open

**Current state (2026-10-25):**
- Sections 1–10 have been reviewed and marked AGREED by the user. Where an AGREED requirement still contains "proposed" wording, that wording is treated as accepted unless listed under Open Questions.
- All requirements are AGREED, including the CLI terminal-UI details (Q38–Q40 resolved 2026-10-24); no requirements questions remain open.
- Section 11 (Non-Functional) and the Out-of-Scope list are AGREED (2026-10-24).
- Key settled decisions: fully offline operation for desktop, web, and CLI (PLAT-3); local shared data file; local web server; Stockfish; computer Players in five classes with varied parameters and static Elo; only human Elo changes (start 400 / 600 / 1200 / 2200 / 2500 from self-assessment, no cap, K 40 for first 30 games then 32); Training mode (human vs. computer, unrated, untimed, with takeback and in-game analysis) vs. Strict mode (default, rated, clocks, no in-game analysis, post-game analysis only); read-only rewind in both; human-chosen matchup; hot-seat human–human; self-play never rated and excluded from W/L/D stats; clocks in v1 with presets + custom, Strict mode only; game time persisted.
- **Changes 2026-10-24 → 2026-10-25:**
  - CLI amended to a full-screen, ratatui-style terminal UI (TUI): keyboard only, moves typed (no board cursor), no mouse, 80×24 minimum, terminal always restored, plain line-oriented mode for screen readers and pipes, scripted runs headless (CLI-1..4, UI-2, UI-4; Q38–Q40).
  - PLAT-3 strengthened: desktop app, web page, and CLI are fully usable offline in v1, including first launch, with no run-time Internet fetches and explicit offline tests.
  - Architecture inputs resolved (Rust backend, TypeScript UI, Tauri, one shared core); all architecture decisions are AGREED in [architecture.md](architecture.md), including the GUI board (chessground + our keyboard/accessibility layer, which delivers GUI-2 and UI-4).
- Decisions are logged in the Decision Log. Update this file as decisions are made.

Each requirement has: ID, statement, status, and **Verification** (test cases). Overall approach is in [test-strategy.md](test-strategy.md).
Test levels: **U** = unit, **I** = integration, **E** = end-to-end, **M** = manual.

---

## 1. Delivery & Platforms

### PLAT-1 (AGREED) Three ways to run
The Game shall be runnable via (a) a web interface, (b) a command line, (c) a desktop application.
- Invoked by: opening a URL (web), opening an executable (desktop), typing the application name (CLI).
- Desktop packaging (AGREED 2026-10-24): unsigned local build for the user's own Mac; signing/notarization only if distributed later.
- **Verification**
  - E: launch each of the three entry points on a clean machine/profile; each reaches a playable new-game state.
  - E: the same scripted game (list of SAN moves) played in all three ends with identical final position/PGN.
  - M: desktop app launches by double-click on macOS; CLI command is on `PATH` after install.

### PLAT-2 (AGREED) Supported platforms
Desktop app and CLI support macOS (Apple Silicon at minimum). Web supports current Chrome, Safari, Firefox. Linux/Windows out of scope for v1 unless you say otherwise.
- **Verification:** M: smoke test on macOS (arm64); E: web suite on Chromium/WebKit/Firefox.

### PLAT-3 (AGREED; strengthened 2026-10-25) Offline operation
All three entry points — desktop app, web page, and CLI — are fully usable with no Internet connection, in v1. Every feature works offline: registration, all matchups and modes, computer play, clocks, analysis (live and post-game), statistics, themes, settings, import/export, and help.
- Nothing is fetched from the Internet at run time: no CDN scripts, web fonts, remote images, update checks, license checks, or telemetry. The GUI bundle, piece and board themes, fonts, the Stockfish engine (including its neural-network file), and documentation ship with the app.
- The web page is served by the local server (`localhost`), so it needs no Internet; it must also work with Wi-Fi off and no network interface other than loopback.
- First launch after install is also offline: nothing (registration, roster generation, database creation) requires a connection.
- Building from source may download dependencies; running never does.
- **Verification**
  - E: with all network interfaces except loopback disabled, for each of desktop, web (Chromium, WebKit, Firefox), and CLI: first launch on a clean profile → register → full game vs. a computer Player in Strict with a clock → post-game analysis → stats view → PGN export.
  - E: network-capture harness during the above records zero non-loopback connections (shared with NFR-4).
  - U/CI: the built GUI bundle contains no `http(s)://` resource references outside an allow-list (license texts, documentation links that are never fetched); the Content-Security-Policy (`default-src 'self'`) is set by both the local server and the desktop app, so any accidental remote load fails in tests.
  - I: the bundled Stockfish runs with no network and no external network file.

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

### PLY-2 (AGREED) Player registration
On first use the Game registers a new Player and stores their preferences in the data store.
- **AGREED 2026-10-24:** registration asks the Player to self-assess as **Beginner, Novice, Intermediate, Master, or Grand Master**; this sets the Player's starting Elo (ELO-1). The self-assessment is stored with the Player.
- **Verification**
  - E: fresh data store → first launch prompts for a name and self-assessment → Player row + default preferences + starting Elo exist.
  - E: registration cannot complete without choosing one of the five levels.
  - E: second launch with same name loads existing Player, does not duplicate.
  - U: name validation (empty, whitespace, duplicate, very long, unicode, case-insensitive uniqueness?).
  - E: a Player registered via CLI is visible in web and desktop (given shared store).

### PLY-3 (AGREED) Player selection / switching
User can choose which registered Player(s) are playing at game start (needed for human–human, and for multiple people on one machine).
- **AGREED 2026-10-24:** in human–human games both sides are registered Players sharing one screen ("hot-seat") for now; both may be the same Player (self-play). Network play is out of scope.
- **Verification:** E: two registered Players selected for h-v-h; each game's stats go to the right Player. E: same Player on both sides is accepted.

### PLY-4 (AGREED) Computer players are first-class Players
Computer opponents are persisted Players with a stable identity, a **class** (Beginner, Novice, Intermediate, Master, Grand Master — same labels as the human self-assessment), and their own engine parameters (see ENG-2).
- **AGREED 2026-10-24:** a computer Player's Elo is **static** and never changes; the Game tracks computer Players' statistics (games, W/L/D, etc.); only human Players' Elo changes.
- **AGREED 2026-10-24 — roster:** 5 computer Players per class (25 total), generated once from a fixed seed with human-readable names. The user can generate more. Computer Players cannot be deleted.
- **AGREED 2026-10-24 — human Player management:** a human Player can be renamed (history follows) and deleted by archiving (hidden; history kept).
- **Verification:** U: roster generation is deterministic for the seed; U: computer Player identity is stable across runs; I: games vs the same computer Player update the same record; I: a computer Player's Elo is identical before and after any number of games.

---

### PLY-5 (AGREED 2026-10-24) Play modes: Training vs. Strict
Every game is played in one of two modes, chosen at game start and fixed for the game:

| | **Training mode** | **Strict mode** |
|---|---|---|
| Takeback | Allowed | Never allowed |
| Analysis during play | Full: best moves, evals, what-if from the current position | **None.** Engine analysis is offered only after the game ends |
| Rewind to view past moves (read-only, human only; GAME-3) | Yes | Yes |
| Clocks | Not available (always untimed) | Available (RULE-4) |
| Elo | Not affected (casual) | Affected (rated, where the opponent is rateable) |
| Post-game full analysis | Yes | Yes |

- After every game (either mode), the user is offered full-game analysis (see ANA-3).
- **Matchup availability (AGREED 2026-10-24):** Training = human vs. computer only. Strict = human–computer, human–human, and computer–computer.
- **Default mode (AGREED 2026-10-24):** Strict.
- **Verification**
  - E: Training mode cannot be selected for human–human or computer–computer, in all three front-ends.
  - E: start game in each mode; mode is stored on the game record and cannot change mid-game.
  - E: Training — takeback works, analysis of current position works, Elo unchanged after game.
  - E: Strict — takeback command/button absent or rejected in all three front-ends; Elo updated after game.
  - E: Strict — no analysis control/command is available during play in any front-end; analysis is offered after the game ends (ANA-3).
  - E: Training — no clock can be selected; Strict — clock presets and custom are selectable.
  - U: policy object (mode → allowed actions) table-tested for every action.

### GAME-1 (AGREED 2026-10-24) New-game setup
In all three front-ends, starting a game collects: matchup (h–h, h–c, c–c), mode (Training/Strict, default Strict), Players for each side (registered humans / computer Players, per PLY-1/3/4), color assignment (White / Black / random), clock (Strict only: preset, custom, or untimed — RULE-4; Training is always untimed), and optionally a starting position (RULE-5). Invalid combinations (e.g., Training for h–h) are not selectable.
- **Verification:** E: each combination produces the expected stored game header; E: invalid combinations are unselectable/rejected in CLI, web, desktop; U: setup validation table-tested.

### GAME-2 (AGREED 2026-10-24) Watching computer–computer games
A human watching a computer–computer game can pause/resume, set move delay (or step one move), and abort. Aborted games record no result and change no stats except history.
- **Verification:** E: pause stops move/clock progress; step advances exactly one move; abort leaves a recoverable record.

### GAME-3 (AGREED 2026-10-24) Rewind to view past moves
During play, a human Player can rewind the game to view past positions, read-only, and then return ("live") to the current state of the game. Available in both modes. Computer Players cannot rewind. No move can be made, and nothing in the game record can be altered, while rewound.
- **AGREED 2026-10-24:** the clock keeps running while rewound (viewing is part of thinking time); if the opponent's computer move arrives while rewound, the view stays where it is with a clear "live position has changed" indicator. In hot-seat human–human games either human may rewind.
- **Verification:** E: rewind k moves then return → position, move list, and clocks identical to never having rewound (apart from elapsed time); E: attempting a move while rewound is rejected; E: stored game record is unchanged by rewinding; E: clock keeps running while rewound; E: computer move arriving while rewound leaves the view in place with the indicator, and "live" shows the new position; E: all of the above in all three front-ends.

---

## 3. Chess Rules & Game Flow

### RULE-1 (AGREED) Full FIDE rules
Legal-move enforcement incl. castling, en passant, promotion (choice of piece), check/checkmate/stalemate, fifty-move rule, threefold repetition, insufficient material.
- **Verification**
  - U: perft test — move-generation node counts match published values for standard positions (start, Kiwipete, etc.) to depth ≥ 4.
  - U: table-driven tests of each special rule and each draw/termination condition (FEN in → expected legal moves / result out).
  - U: illegal moves (into check, pinned piece, castling through check, wrong turn) rejected with a reason.

### RULE-2 (AGREED) Game end actions
Resign, offer/accept draw (human–human), automatic result detection. Result recorded as 1-0, 0-1, ½-½ with termination reason.
- **AGREED 2026-10-24:** a computer never resigns or offers a draw; it accepts a human's draw offer only if its own evaluation is ≤ 0. Threefold repetition and the fifty-move rule end the game automatically (no claim step).
- **Verification:** U/E: each termination path yields correct result + reason in the stored record.

### RULE-3 (AGREED 2026-10-24, including details) Takeback
Takeback is allowed in Training mode only (human vs. computer); never in Strict mode (PLY-5). Proposed: takeback undoes the human's last move and the computer's reply, back to the human's turn; may be repeated to the start of the game.
- **Verification:** U: position/move list after undo equals the earlier state; E: unavailable in Strict mode (see PLY-5).

### RULE-4 (AGREED) Clocks (optional)
Optional time controls (untimed default). Flag on timeout loses unless insufficient material.
- **AGREED 2026-10-24: in v1.**
- **AGREED 2026-10-24:** presets — untimed, 1+0, 3+2, 5+0, 10+5, 15+10, 30+0 — plus a custom option (base time + increment). **Clocks are available in Strict mode only** (any matchup: h–c, h–h, c–c); Training mode is always untimed (decided later than the preset list).
- **Verification:** U: clock logic with a fake time source (increments, pause/resume, flag fall); E: timeout ends game with correct result; E: clock state survives save/resume (RULE-5); E: computer Player respects the clock.

### RULE-5 (AGREED) Save, resume, import/export
Games auto-saved so an interrupted game can be resumed; PGN export for any game; PGN/FEN import.
- **AGREED 2026-10-24:** clocks are paused while the app is closed. A game from a non-standard FEN is always unrated. Imported PGNs are for review/analysis only and never affect Elo or stats.
- **Verification:** E: kill app mid-game → relaunch → resume at same position/clock; U: PGN round-trip (export→import→identical move list); U: malformed PGN/FEN rejected gracefully.

---

## 4. Engine

### ENG-1 (AGREED) Open-source engine
The Game shall use an open-source chess engine for computer play and analysis.
- **AGREED 2026-10-24: Stockfish** (GPLv3, UCI protocol; native binary and WASM build). Consequence: the Game's source and any distributed builds must be GPL-compatible, with Stockfish attribution/source offer.
- **Verification**
  - I: engine process starts, answers `uci`/`isready`, returns legal `bestmove` from standard positions.
  - I: engine failure (crash, missing binary, timeout) surfaces a user-visible error and the game remains recoverable.
  - U: license file/attribution present in distributed artifacts.

### ENG-2 (AGREED) Difficulty levels
User can configure difficulty by choosing a computer Player (PLY-4). Computer Players come in five **classes** — Beginner, Novice, Intermediate, Master, Grand Master.
- **AGREED 2026-10-24:** members of a class have similar but not identical play parameters, so no two computer Players are exactly alike. Parameters are generated within class bounds (proposed: engine `Skill Level`, depth/movetime limits, `UCI_Elo` where in range, and a small random-error rate), then stored with the Player so they stay stable.
- **AGREED 2026-10-24:** each computer Player's static Elo lies within its class's Elo band (ELO-1 class bands). Grand Master computers span 2500–3000.
- **AGREED 2026-10-24:** Beginner and Novice computer Players (400 / 600 anchors) are below the range of Stockfish's built-in strength limiter, so they use a low skill level, shallow depth, and random errors. All classes are calibrated empirically by playing matches (see verification).
- **Calibration verification:** E (slow): matches against reference opponents / between classes produce measured Elo estimates within a tolerance (to be set) of each class's assigned Elo.
- **Verification**
  - U: generated Players in a class stay within the class's parameter bounds; no two share an identical parameter set.
  - I: generated parameter sets persist and reload unchanged.
  - E (slow): class ordering holds in head-to-head play (e.g., every Master beats every Novice significantly more than half the time).
  - I: each level produces legal moves within its time budget.
  - E (slow, nightly): round-robin matches between levels → higher level wins statistically significantly more (e.g., ≥ 100 games per pairing, SPRT or binomial test) — validates monotonic strength.
  - U: nominal Elo table is monotonic and covers the documented range.

### ENG-3 (AGREED) Responsiveness
Computer move ≤ configured max think time (default ≤ 3 s at top level on a baseline MacBook); UI never blocks while engine thinks; user can cancel.
- **Verification:** I: measured move latency under limit; E: UI remains interactive (can open settings, resign) during engine think.

---

## 5. Persistence & Data

### DAT-1 (AGREED) Central data store
A single central data store persists data across runs: Players, preferences, games, statistics, Elo history.
- **AGREED 2026-10-24:** one local, shared data file on the user's Mac (e.g., SQLite) used by CLI, desktop, and a locally-run web server (`localhost`). No hosted/multi-user service in v1. Specific technology is an architecture-phase choice.
- Consequence: web interface is served locally (not pure-browser storage), so CLI/desktop/web see the same Players, games, and Elo.
- **Verification**
  - I: data written in one run is present after process restart.
  - I: all three clients read/write the same store without corruption (concurrent access test).
  - I: schema migrations upgrade an old-version store without data loss (fixture DBs per version).
  - I: corrupt/missing store → clear error, backup/recovery path, no silent data loss.

### DAT-2 (AGREED) Backup / export of data
User can export all their data (JSON/CSV + PGNs) and import it back.
- **Verification:** I: export→wipe→import yields identical logical contents.

### DAT-3 (AGREED) Per-game statistics
Store for each game (including self-play games, which are flagged — ELO-1/UI-2): players, colors, result + termination reason, start/end time, **game time (total elapsed duration, and time spent by each Player; excludes time the app was closed/paused during a saved-and-resumed game)**, move list (PGN), mode, difficulty/engine level, rated flag, whether analysis was used, Elo before/after, opening (ECO), per-move time, per-move engine evaluation / accuracy (centipawn loss). Per-move evaluation is computed after the game ends, in the background, and never during play.
- **Verification:** E: play known game → stored record matches expected fields; U: record serialization round-trip.

---

## 6. Elo Rating

### ELO-1 (AGREED 2026-10-24) Elo per game, tracked over time
After each completed rated game, the Game computes new Elo for the Player(s) and stores rating history (timestamped, with game reference).
- **AGREED 2026-10-24:** games against the computer are rated against a **fixed nominal Elo per difficulty level**; computer ratings never change. Human-vs-human updates both Players. Only Strict-mode games are rated (PLY-5).
- **AGREED 2026-10-24:** a Player's starting Elo is determined by the self-assessment given at registration (PLY-2): Beginner, Novice, Intermediate, Master, Grand Master.
- **AGREED starting values:** Beginner 400 · Novice 600 · Intermediate 1200 · Master 2200 · Grand Master 2500. Computer classes use the same anchors for their static Elo (ENG-2).
- **AGREED K-factor:** K = 40 for a Player's first 30 rated games, K = 32 afterwards.
- Standard Elo expected score `E = 1/(1+10^((Rb−Ra)/400))` (assumed; standard formula).
- **AGREED 2026-10-24 — class bands:** a rating's class is the highest floor ≤ the rating: Beginner <600 (starts at 400), Novice 600–1199, Intermediate 1200–2199, Master 2200–2499, Grand Master ≥2500.
- **AGREED 2026-10-24:** ratings are stored as real numbers and displayed rounded to the nearest integer.
- **AGREED:** only human Players' Elo changes. Computer Elo is static. Computer-vs-computer games change no rating. No re-assessment/reset of Elo.
- **AGREED 2026-10-24:** there is **no upper or lower cap** on a human's Elo; it is computed and tracked as results dictate. A Grand Master is a Player at **2500 Elo or higher** (starting value 2500).
- **AGREED 2026-10-24:** self-play games (same human on both sides of a human–human game, or same computer Player on both sides of a computer–computer game) change no Elo, are excluded from win/loss/draw statistics, and are kept in game history.
- **Consequence:** in human–human games each Player's change uses their own K (40 vs. 32), so changes are not exactly opposite when one Player has fewer than 30 rated games.
- **Verification**
  - E: registering with each of the five self-assessments yields the configured starting Elo; first history point = starting Elo.
  - U: known-value table (equal ratings win/draw/loss; +200 diff; etc.) matches hand-computed expected results to rounding rule.
  - U: in h-v-h, each Player's delta uses their own K; deltas are exactly opposite when both Ks are equal; sign correct for win/draw/loss.
  - U: K switches from 40 to 32 after exactly 30 rated games (game 30 uses 40, game 31 uses 32).
  - U: unrated games (Training mode), aborted games, computer-vs-computer games, and self-play games do not change rating.
  - U: Elo above 2500 and below the starting value are stored/handled with no clamping.
  - I: rating history is append-only and consistent: final rating = initial + Σ deltas.
  - E: after N scripted games, history query returns N+1 points.

### ELO-2 (AGREED 2026-10-24) Rated vs. casual follows play mode
Strict-mode games are rated; Training-mode games are casual and never change Elo. Mode is not mutable mid-game, so no game can switch from rated to casual or vice versa.
- **Verification:** E: Training game (with analysis and takeback) → rating history unchanged; Strict game using restricted analysis → rating updated.

---

## 7. User Interface — Settings, Stats, Themes (GUI + CLI)

### UI-1 (AGREED) Settings
User can configure: default opponent class (pre-selected in new-game setup), board theme, piece theme, side/color, show legal-move hints, sound, coordinate orientation (flip), move-input method defaults, maximum engine think time (can only shorten a computer's think time), confirm-move setting.
- Settings are stored per Player in the data store (PLY-2).
- **Verification**
  - E: change each setting → takes effect immediately and persists after restart.
  - U: invalid values rejected/clamped; defaults applied for new Player.
  - E: settings changed in web are respected in desktop for the same Player.

### UI-2 (AGREED) Statistics views
User can view statistics over time: W/L/D totals, by color, by difficulty/opponent, Elo-over-time chart, game history list with replay, average game length. Self-play games appear in the game history list but are excluded from W/L/D totals and other aggregates.
- CLI: a TUI statistics screen with tables and an Elo-over-time chart; plain mode prints a text table + ASCII sparkline of Elo.
- **Verification**
  - I: seeded store with known games → displayed numbers equal expected aggregates.
  - E: Elo chart point count and last value match stored history.
  - E: empty state (no games) renders gracefully.

### UI-3 (AGREED) Themes
At least 10 board themes and 5 piece sets in GUI (incl. high-contrast). CLI supports ASCII style options (plain, Unicode glyphs, ANSI color, light/dark squares) and a no-color mode.
- **Verification:** E: screenshot/visual-regression per theme (GUI); U: ASCII renderer snapshot tests per style.

### UI-4 (AGREED) Accessibility
GUI keyboard-operable, screen-reader labels for squares/pieces/moves, sufficient color contrast; CLI works with `NO_COLOR` and screen readers via plain mode (CLI-1), since full-screen TUIs are not reliably readable by screen readers.
- **Verification:** automated a11y audit (axe) on web; M: VoiceOver walk-through; U: contrast ratio checks for themes.

### UI-5 (AGREED 2026-10-24) Viewing stats for any Player
The user can view statistics for **any** Player, human or computer: W/L/D totals, by color, by opponent/class, game history with replay, average game length, and Elo (current and, for humans, over time; computers show their static Elo). Self-play games are excluded from aggregates but listed in history (UI-2). A "Computer Players" screen lists all computer Players with class, Elo, and W/L/D.
- **Verification:** I: seeded store → displayed numbers for a chosen human and a chosen computer Player equal expected aggregates; E: computer Player list shows 25 default Players with correct classes; E: archived (deleted) human Players' history remains visible where referenced.

---

## 8. Analysis Mode

### ANA-1 (AGREED 2026-10-24) Analysis during play — Training mode only
A human Player in a **Training** game can invoke analysis at any time to view: top N best moves (default 3) with evaluation (centipawns/mate), principal variation, and depth, for the current position or any earlier position.
- **Strict mode: there is no analysis during play.** Analysis is offered only after the game ends (ANA-3). Humans may rewind to view past moves read-only (GAME-3).
- Analysis is for human Players only; computer–computer games have none during play.
- **Verification**
  - I: for known tactical positions (mate-in-N, free-queen), engine top move equals expected.
  - E: Training — invoking/exiting analysis at move k leaves the real game state unchanged and resumable.
  - E: Strict — no analysis control exists during play in CLI, web, or desktop; the engine is never queried for an in-progress Strict game on behalf of a human (spy on engine calls).
  - E: not available for computer Players' turns.

### ANA-2 (AGREED) What-if scenarios
In analysis (Training mode during play, and post-game in any mode), the user can play hypothetical moves on a branch (both sides), step back/forward, and branch again, with engine evaluation updating; exit discards or optionally saves the branch (PGN variation).
- **Verification**
  - U: variation tree add/navigate/delete operations.
  - E: moves in what-if never alter the stored game; "return to game" restores exact position.
  - E: export of saved branch is valid PGN with variation.

### ANA-3 (AGREED 2026-10-24) Post-game full-game analysis
After every game (any mode), the user is offered analysis of the entire game: step through every position with engine evaluation and best alternatives. Proposed: blunder/mistake/inaccuracy flags by centipawn loss. No restrictions apply once the game is over; it never affects Elo. In Strict mode this is the **only** engine analysis available for the game.
- **Verification:** E: offer appears after checkmate, draw, resignation in each mode; I: known game with planted blunder is flagged; E: running it leaves stored result and Elo untouched.

---

## 9. Command-Line Interface

### CLI-1 (AGREED 2026-10-24, amended) Full-screen terminal UI
The CLI is a full-screen, ratatui-style terminal user interface (TUI). During play it shows the board, drawn with ASCII/Unicode art, in a board panel next to panels for the players and status, the clocks, the move list (SAN), and a command/move input line. The screen updates in place on every state change (move, clock tick, engine thinking, rewind, result) without scrolling.
- **AGREED 2026-10-24 — screens:** the other CLI flows are also TUI screens, navigated by keyboard: registration, new-game setup (GAME-1), play, watching computer–computer games (GAME-2), post-game analysis and what-if (ANA-2/3), statistics (UI-2/5), Computer Players, and settings.
- **AGREED 2026-10-24 — terminal size:** minimum 80×24. Below that, the TUI shows an "enlarge the terminal" message instead of a broken layout. Resizing reflows the layout immediately.
- **AGREED 2026-10-24 — terminal hygiene:** the terminal is always restored (cursor, echo, main screen) on quit, Ctrl-C, panic, or crash. Ctrl-Z suspends and `fg` redraws the screen.
- **AGREED 2026-10-24 — no mouse:** the TUI has no mouse support in v1; it is operated entirely from the keyboard.
- **AGREED 2026-10-24 — plain mode:** a line-oriented mode (`--plain`) prints the board and prompt after each move, as a scrolling transcript. It is selected automatically when stdin/stdout is not a terminal. It serves screen readers (UI-4) and piping. Non-interactive scripted runs (CLI-4) never start the TUI.
- **Verification**
  - U: TUI render snapshots at fixed terminal sizes (start position, mid-game, check, promotion prompt, flipped for Black, rewound with "live position has changed" indicator, game result) for each board style.
  - U: board renderer snapshot tests per style (shared by TUI and plain mode).
  - E: PTY-driven session with a terminal-emulator screen parser asserts the visible screen after each input.
  - E: resize to below 80×24 shows the enlarge message; resizing back restores the layout.
  - E: after `quit`, Ctrl-C, and an injected panic, the terminal is restored (main screen, echo on, cursor visible).
  - E: plain mode via PTY and via pipe shows the board and prompt after each move.

### CLI-2 (AGREED) Coordinates
In the board panel (and in plain mode), the board shows file labels A–H and rank labels 1–8 on all sides needed; orientation flips when Black is at the bottom.
- Note: display uppercase A–H per spec; input accepts either case (Q8).
- **Verification:** U: snapshot includes labels in both orientations.

### CLI-3 (AGREED) Algebraic notation input
Player must enter moves in algebraic notation, typed in the TUI input line (or at the plain-mode prompt); game record kept in algebraic notation (SAN). Invalid input shows an error in the TUI status area and leaves the typed text editable. **AGREED 2026-10-24:** typing is the only way to move in the CLI; there is no keyboard board cursor (unlike GUI-2), so the arrow keys stay free for rewind (CLI-4). Proposed accepted forms: SAN (`e4`, `Nf3`, `exd5`, `O-O`, `e8=Q`) and coordinate form (`e2e4`) as a convenience; case-insensitive file letters; trailing `+`/`#`/`x` optional.
- **Verification**
  - U: parser table tests: valid (all piece types, captures, castling both sides incl. `0-0`, promotion, en passant, disambiguation `Nbd2`, `R1a3`), invalid (garbage, ambiguous, illegal, off-board) with helpful error messages.
  - E: re-prompt on bad input without losing turn; game never crashes on arbitrary input (fuzz).
  - U: move list output equals SAN for the played game.

### CLI-4 (AGREED) Command set
In-game commands distinct from moves, typed in the input line: `help`, `resign`, `draw`, `undo` (Training only), `analyze` (Training during play; any mode post-game), `back` / `forward` / `live` (rewind view, GAME-3), `save`, `flip`, `moves` (list legal), `history`, `quit`. Non-interactive flags for scripting (e.g., `chess --matchup cvc --mode strict --white <computer-player> --black <computer-player> --clock 5+0 --no-color`); scripted runs do not start the TUI.
- **AGREED 2026-10-24 — key bindings:** frequent actions also have keys that can never begin a move, so typing a move is never intercepted: ←/→ = back/forward, End = live, PgUp/PgDn = scroll move list, F1 = help, Tab = switch panel focus, Esc = cancel/clear input. Letters, digits, and `-`/`=` always go to the input line. Commands that the mode policy disallows (PLY-5) are shown as unavailable and their keys do nothing but explain why.
- **AGREED 2026-10-24 — c–c watch keys (GAME-2):** the watch screen has no move input, so plain keys are used: Space = pause/resume, `s` = step one move while paused, `+`/`-` = move delay, `a` = abort (with confirmation).
- **Verification:** E: each command and key binding via PTY; U: command vs. SAN ambiguity (e.g., a move can't collide with a command word); U: no key binding consumes a character that can start a SAN or coordinate move; E: disallowed commands are visibly unavailable in each mode.

---

## 10. Graphical UI (Web + Desktop)

### GUI-1 (AGREED) Common GUI
Web and desktop share one GUI codebase (desktop wraps the same UI).
- **Verification:** E: same E2E suite run against web and desktop shells; any divergence is a failure.

### GUI-2 (AGREED) Move input methods
Player can move by (a) mouse point-and-click (and, proposed, drag-and-drop), (b) keyboard: arrows move cursor, Space selects piece, arrows pick destination, Enter confirms, (c) typing algebraic notation.
- Proposed: Esc cancels selection; legal destinations highlighted; focus indicator visible.
- **Verification**
  - E: same move made by each of the three methods produces identical game state.
  - E: keyboard: select a piece with no legal moves (feedback), cancel with Esc, illegal destination rejected, promotion prompt via keyboard.
  - E: input method mixing mid-move (select by mouse, confirm by Enter) behaves sensibly.
  - E: touch tap-to-move on web (AGREED 2026-10-24; responsive/mobile layout is out of scope).

### GUI-3 (AGREED) Coordinates in GUI
Board shows A–H / 1–8 labels (toggleable in settings, default on).
- **Verification:** E: labels present and correct in both orientations.

### GUI-4 (AGREED) Game feedback
Highlight last move, check, legal moves; move list panel (SAN); captured pieces; game result dialog; engine "thinking" indicator.
- **Verification:** E: assertions on DOM/accessibility tree for each indicator.

---

## 11. Non-Functional (AGREED 2026-10-24)

| ID | Requirement | Verification |
|---|---|---|
| NFR-1 | Startup to playable ≤ 3 s (CLI ≤ 1 s) on baseline MacBook | Benchmark in CI/manual |
| NFR-2 | No data loss on crash/kill at any point (atomic writes, WAL) | Fault-injection: kill during write, verify store integrity |
| NFR-3 | Engine moves always legal; app never enters illegal state | Property-based/fuzz tests; c-v-c soak of 1000 games with no crashes |
| NFR-4 | Privacy: data stays local unless user opts in; no telemetry by default | Network-capture test shows no outbound calls |
| NFR-5 | Documented install + run steps for each of the 3 entry points | M: fresh-machine walkthrough |
| NFR-6 | Open-source license compliance for engine and all deps | License audit in CI |

---

## Open Questions

Architecture inputs resolved 2026-10-24: Rust backend, TypeScript UI, Tauri desktop shell, one shared core (see [architecture.md](architecture.md)).

None open for requirements.

### Resolved
Q1–Q40 (see Decision Log). Removed from scope by decision: Strict-mode analysis during play (Q15).

## Out of Scope (AGREED 2026-10-24, v1)
Online play, matchmaking, accounts with passwords, chess variants (960, etc.), opening-book trainers, mobile native apps, Windows/Linux packaging.

## Decision Log
| Date | Decision | Requirement(s) |
|---|---|---|
| 2026-10-24 | Initial draft created from user's 15 top-level requirements plus Claude's proposals. | all |
| 2026-10-24 | Central data store = one local shared file on the Mac; web served locally (localhost). | DAT-1, PLAT-1, PLAT-3 |
| 2026-10-24 | Engine = Stockfish (GPLv3); app must be GPL-compatible if distributed. | ENG-1 |
| 2026-10-24 | Computer opponents have a fixed nominal Elo per difficulty level; games vs computer are rated. | ELO-1 |
| 2026-10-24 | Training mode is human-vs-computer only; human–human and computer–computer are Strict only. | PLY-1, PLY-5, RULE-3 |
| 2026-10-24 | Game time (total and per-Player) is persisted with game stats. | DAT-3 |
| 2026-10-24 | Starting Elo comes from Player's self-assessment at registration: Beginner, Novice, Intermediate, Master, Grand Master. Numeric values pending. | PLY-2, ELO-1 |
| 2026-10-24 | Starting/anchor Elo: Beginner 400, Novice 600, Intermediate 1200, Master 2200, Grand Master 2500 (user edited ELO-1 directly). K=40 for first 30 games then 32. No Elo re-assessment/reset. No cap on Elo; Grand Master = 2500 or higher. | ELO-1, PLY-2 |
| 2026-10-24 | Self-play (same human, or same computer, on both sides) is unrated, excluded from W/L/D stats, kept in game history. | ELO-1, UI-2, DAT-3 |
| 2026-10-24 | Beginner/Novice computers use low skill, shallow depth, and random errors; all classes calibrated empirically via matches. | ENG-2 |
| 2026-10-24 | Clock presets: untimed, 1+0, 3+2, 5+0, 10+5, 15+10, 30+0, plus custom; clocks allowed in every mode and matchup. | RULE-4 |
| 2026-10-24 | Default play mode is Strict. | PLY-5 |
| 2026-10-24 | Rewind: clock keeps running while rewound; computer move arriving while rewound leaves the view in place with an indicator; either hot-seat human may rewind. GAME-1 and GAME-2 accepted. | GAME-1/2/3 |
| 2026-10-24 | **Strict mode has no analysis during play.** Analysis only after the game ends. Humans may rewind to view past moves read-only and resume at the live position; computers cannot rewind. | PLY-5, ANA-1, ANA-3, GAME-3 |
| 2026-10-24 | Clocks are Strict-mode only; Training is untimed. | RULE-4, GAME-1 |
| 2026-10-24 | Class bands (<600, 600–1199, 1200–2199, 2200–2499, ≥2500); computer Elo within band, Grand Master computers 2500–3000. Ratings stored as reals, displayed rounded. | ELO-1, ENG-2 |
| 2026-10-24 | Roster: 5 computer Players per class (25), fixed seed, human-readable names, user can generate more, cannot be deleted. Human Players: rename; delete = archive. | PLY-4 |
| 2026-10-24 | Computer never resigns/offers draws; accepts draw only if eval ≤ 0; threefold and fifty-move draws are automatic. | RULE-1/2 |
| 2026-10-24 | Custom-FEN games unrated; imported PGNs review-only; clocks pause while the app is closed. | RULE-5, ELO-2 |
| 2026-10-24 | Settings: "difficulty" → default opponent class; "think time" → maximum only. | UI-1 |
| 2026-10-24 | Per-move evaluations computed after game ends, in the background. | DAT-3 |
| 2026-10-24 | Added viewing stats for any Player (human or computer) and a Computer Players screen. | UI-5 |
| 2026-10-24 | NFR-1…6 and Out-of-Scope list accepted; web touch = tap-to-move only (no mobile layout); desktop = unsigned local build. | NFR, GUI-2, PLAT-1 |
| 2026-10-24 | Sections 8–10 reviewed and marked AGREED by user (proposed wording within them treated as accepted unless listed in Open Questions). | ANA, CLI, GUI |
| 2026-10-24 | Human selects matchup and mode at the start of each game; computer–computer is an explicit human-selected Strict option with two chosen computer Players (may be the same one). | PLY-1, PLY-5 |
| 2026-10-24 | Human–human is hot-seat on one screen with registered Players; the same Player may play both sides. | PLY-3 |
| 2026-10-24 | Computer Players: five classes (same labels as self-assessment) with varied-but-similar parameters; static Elo; stats tracked; only human Elo changes; c-v-c changes no Elo. | PLY-4, ENG-2, ELO-1 |
| 2026-10-24 | Clocks / time controls are in v1. | RULE-4 |
| 2026-10-24 | RULE-3 (takeback) and ELO-1 agreed as written. | RULE-3, ELO-1 |
| 2026-10-24 | Two play modes: Training (analysis + takeback, unrated) and Strict (restricted analysis of past positions only, no takeback, rated). Post-game full analysis offered after every game. | PLY-5, RULE-3, ELO-2, ANA-1, ANA-3 |
| 2026-10-24 | CLI amended to a full-screen ratatui-style TUI; plain line-oriented mode kept for screen readers and scripting. TUI details PROPOSED (Q38–Q40). | CLI-1..4, UI-2, UI-4 |
| 2026-10-24 | Q38: TUI details accepted (screens, 80×24 minimum, terminal hygiene, plain mode, key bindings). Q39: no keyboard board cursor in the CLI; moves are typed only. Q40: no mouse support in the TUI. | CLI-1, CLI-3, CLI-4 |
| 2026-10-25 | PLAT-3 strengthened: desktop, web page, and CLI are fully usable offline in v1, including first launch; no run-time Internet fetches of any kind; explicit offline verification. | PLAT-3, NFR-4 |
