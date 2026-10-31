# Chess Application — Architecture

**Status: Baselined v1.0, 2026-10-26.** Every decision is AGREED except D-21 (DEFERRED, two-way, decided at implementation). ASM-1 is CONFIRMED. All three rounds of the user's answers are applied, and every question this document raised for the user is answered (§14). Nothing here is implemented yet.

**Amended 2026-10-27 (v1.0 amendment 1, user decisions; nothing renumbered):** a reset is allowed while the Player has unfinished games, and voids them (D-26, D-30, OQ-8 and G-20 closed). Stockfish 19 (`sf_19`) is the pinned engine (D-9, TBD-26 closed). One new non-blocking question for the user: Q-A18 (the architect's interpretation of how a voided game is shown, with a working default). See the change log below and §11.2.

**Amended 2026-10-28 (v1.0 amendment 2; requirements Amendment 2 / CR-4 and [the SE handoff](handoff_se_to_architect.md); nothing renumbered):** the architecture now implements the systems engineer's decisions CR-4.1 to CR-4.20. New decisions D-31 to D-37; D-17, D-18, D-19, D-21, D-24 and D-29 amended; risks R-19 and R-20; ASM-5 and OQ-12 added. In short: a **build-and-release view** with the Release engineer and the blocks that carry the release-time functions (§2.1, D-31); a **named allocation-block list** that makes the allocation usable below "chess-app core" and names the Tauri shell (§2.2, D-32); name rules corrected (§5.5, §7.2); `games.json` in the full-data archive (§5.7); duplicate-PGN prompt without "replace" (D-34); takeback, draw-offer and move-now rules (D-33); headless, `--data-dir`, `chess serve` errors and shutdown, `chess install-cli`, and distinct export and import command forms (D-36); journal and restore recovery with "voided: unreadable" and a backup cadence (D-35); a CLI Elo chart in character cells (D-37). Where the architecture pushes back on the systems engineer, the reason is in §10.4. See the change log below.

**Amended 2026-10-28 (v1.0 amendment 3; requirements Amendment 3 / CR-5; nothing renumbered; no decision added):** the systems engineer's consistency pass answered the points this document raised in §10.4. The **automatic backup** (24 h, newest 3 kept, TBD-33) is now stated in requirements DAT-5, so D-35 is driven by a requirement. The **restore limit** (R-19, OQ-12) is stated in DAT-5 too, and OQ-12 is closed; the journal is kept in `journal/unrestored/` and no automatic recovery is offered in v1. Exit code 4, `chess games`, `--on-duplicate`, `--yes` and the `chess export`/`chess import` forms are accepted as design (CLI-4, CLI-5; TBD-21 closed). `--config` and `CHESS_CONFIG` are accepted as test-only. The R-20 draw-offer clock stop is accepted. The confirmation wording in §4.1 and the UI-12 note on the void offer are aligned with the functional decomposition (F-2.5.7 is on `chess-app core`). Details are in §10.4 (last rows) and §14.

**Amended 2026-10-28 (v1.0 amendment 4; requirements Amendment 4 / CR-6; nothing renumbered; no decision added):** the user answered the readiness check ([SRR-SFR-readiness.md](SRR-SFR-readiness.md)). The software has ordinary criticality (not safety-related, not regulated), and SEI CERT is the coding standard with the standard Rust and TypeScript checks and every warning treated as an error (a new cross-cutting row in §8, a new row in §10.2; the source is a user decision, not a requirement). Q-A18 is answered: the reading of a voided game stands. Design changes inside one component are approved by the system-architect agent as a desk review, and anything wider by the user ([engineering/SWE-001-engagement-profile.md](engineering/SWE-001-engagement-profile.md)). No Intel Mac is available for testing and none is expected soon: PLAT-2, D-25 and R-13 are unchanged and the consequence is open (readiness FIND-015).

**Amended 2026-10-30 (v1.0 amendment 6; requirements CR-8; no decision added):** the ECO table source is chosen (G-15 closed, D-20 amended): the Lichess `chess-openings` dataset (CC0-1.0), a pinned snapshot in `third_party/eco/`, stored as code and name, matched by position (deepest named position). See D-20, §8.1 and §10.3.

**Amended 2026-10-30 (v1.0 amendment 5; requirements Amendment 5 / CR-7; nothing renumbered; no decision added):** the user answered the open readiness findings ([SRR-SFR-readiness.md](SRR-SFR-readiness.md), run 3). **Ratings are whole integers** and the Elo update is fully specified (ELO-1, §7.2: the rating columns change from `real` to `int`; nothing is released, so there is no migration). **The PGN export now follows the standard's export format** (§5.7; TBC-6 and TBD-24 closed). **Baseline versions** of every language and open-source component are recorded in the new §3.4. The NFR-8 budget allocation in §4.8 is a Should. The coding-standard row (§8) now reads "CERT informs best practice", the security row (§8) reflects a local-only v1 with CVE scanning, and the platform matrix (§9) is best effort for hardware that is not available. D-25 and R-13 are unchanged.

**What remains open at baseline** (none of these needs a user decision; each has an owner and a closing step in §14):
- D-4 board spike, 2 days, the first GUI task (§11.1). Owner: GUI developer.
- Measurements on the M4 baseline: the §4.8 timeouts and the journal flush cost (ASM-2), and NFR-8 input latency. Owner: core developer, implementation step 2.
- Two facts about the pinned Stockfish 19 that could not be checked without downloading and running it (ASM-3, narrowed 2026-10-27): whether the official `stockfish-macos-universal` asset contains both arm64 and x86_64 slices, and its `UCI_Elo` floor and range. Owner: architect. The first run of the engine build script (step 2) closes it.
- Terminal resize behaviour in Terminal.app and iTerm2 (ASM-4, R-16). Owner: CLI developer, step 2.
- The exact browserslist query for "the last 3 years" (§8.2). Owner: GUI developer, step 3.
- TBD-18: the wording of the screen-reader announcements; the events are fixed in §8.3. Owner: GUI developer, step 3.
- Tauri's sidecar naming rule for a universal target (OQ-11, split out of ASM-3 on 2026-10-27), checked against the pinned Tauri version when packaging is set up (§8.2). Owner: architect, step 3.
- D-21 (Elo chart library for the GUI), DEFERRED. The CLI chart is decided (D-37).
- Amendment 2 additions: ASM-5 (macOS facts behind `install-cli` and ad-hoc signing, requirements TBC-10; owner architect, step 3) and OQ-12 (what a restore cannot recover; closed by requirements Amendment 3, CR-5.1).

**Diagrams:** 14 Mermaid diagrams, **machine-validated on 2026-10-28** with `validate-mermaid.mjs` from the architecture-governance skill (mermaid 12.x, Node 26): 14 of 14 parsed OK. The 13 earlier diagrams were hand-checked only before this date. The amendment-2 changes are the §2 context diagram (Tauri shell block, engine labels), the new §2.1 build-and-release diagram, the §3.2 dependency graph, the §4.3 statechart, the §6.1 screens diagram and the §7.2 ER diagram. A parse pass proves syntax, not readability. Re-run `node <architecture-governance>/scripts/validate-mermaid.mjs docs/architecture.md` after any diagram edit.

**Changes 2026-10-24 → 2026-10-26:**
- First draft created from the requirements and the user's stack choices.
- CLI changed to a full-screen ratatui TUI, keyboard only with typed moves and no mouse, plus plain and headless modes (§6.1, D-6).
- Offline operation for desktop, web, and CLI made explicit, with enforcement and tests (§8.1).
- Section 11 decisions recorded (§11.2).
- D-4 decided after two trade studies ([first pass](trade-study-gui-board.md), [refined](trade-study-gui-board-refined.md)): chessground + shared keyboard/accessibility overlay, with react-chessboard (patched) as the fallback (§6.2, §11.1).
- 2026-10-26: all diagrams reviewed against the requirements and decisions. Added: offline boundary, bundled Stockfish, CLI modes (§2); third-party dependencies (§3.2); Policy and event stream (§4.1); step, engine error, and draw rule (§4.3); board revert path (§5.3); full screen lists (§6.1, §6.2); clock checkpoint and `GAME_TAG` (§7.2); fallback adapter (§11.1).
- 2026-10-26, architecture review (traceability and completeness):
  - New sections: concurrency and threading model with timeouts (§4.8); error catalogue, protocol reliability (reconnect, idempotency, versioning), and internal interface contracts including PGN/FEN I/O (§5.5–§5.7); data ownership and transaction boundaries, and schema version skew (§7.4, §7.5); deployment and packaging (§8.2); accessibility details (§8.3); open items (§14).
  - Expanded: Policy table (Resign, CancelEngine), engine-move legality check, and `Calibration` scope (§4.4–§4.7); state machine (takeback during an engine search, pause freezes clocks, D-14/D-15 hooks; §4.3); protocol commands and events (§5.1); §5.3 (the engine reply happens only after an accepted move); §5.4 (post-game analysis goes through `AnalysisService`); CLI run-mode rule (§6.1); GUI settings effects (§6.2); `APP_SETTING` and notes on imported games (§7.2); `ui/` row and requirement IDs in §3.3.
  - §10 rewritten as a per-requirement allocation, with reverse (orphan) checks and requirement gaps. The systems engineer's PROPOSED requirements of the same day (PLAT-4, PLY-6, ENG-4, DAT-4–6, UI-6–9, ANA-4, CLI-5, GUI-5, NFR-7–9, IF-1–5) are allocated provisionally. Values are proposed for the TBDs assigned to the architect, and gaps are linked to their Q41–Q56.
  - §11: decisions D-12–D-22 added; consequences and revisit triggers for every decision (§11.3); study option "D1" renamed so it can't be mistaken for decision D-1. §12: risks R-9–R-12 added, R-2 mitigation corrected.
  - Marked as dependent on undecided requirements, without resolving them: Q41 (computer Elo), Q42 (draw offers), Q54/TBD-7/TBD-23 (durability tolerance; the `synchronous` pragma and checkpoint interval are placeholders), and the Ctrl-C confirmation. ECO table and sound assets added to the §8.1 offline table. TBD-1 input given (§10.1).
- 2026-10-26, user answers Q41–Q55 applied:
  - Dynamic Elo for computers with fixed engine parameters (Q41); humans offer draws, computers never do (Q42); room for future network play (§8.4, Q43).
  - Universal Intel + arm64 build, Tahoe minimum, M4 baseline, supported browsers and terminals (§8.2, Q46/Q47).
  - Profile chooser, 128-character unique names, confirmed reset (Q48); one unfinished game per mode (Q49). *(Superseded the same day: humans can only resign, which is a loss; abort exists only for a c–c watcher and has no effect on stats.)*
  - Merge import and missing-store flow (D-17, Q51); themes that follow the system, sound toggle, confirmations, English only (Q52); clock keeps running during engine failure (D-15, Q53).
  - Move journal so no completed move is ever lost (§7.6, D-23, Q54); `synchronous=FULL` + `fullfsync`.
  - Port precedence and config file (D-24), fluid GUI, TUI auto-resize (Q55).
  - New risks R-13–R-16, decisions D-23–D-27, gaps G-16–G-18, and follow-up questions Q-A1–Q-A12 (§14).
- 2026-10-26, second round of user answers applied:
  - Computer category follows its rating, the K-factor is shared, and c–c games are rated. Two Elo tracks (Training, Strict) for every Player (D-28), with Training games in W/L/D.
  - Interruptions are never aborts. Humans end games only by Resign (h–h asks which side). Abort is for the c–c watcher only, with no effect. State machine and Policy table rewritten.
  - One unfinished game per Player per mode (`UNFINISHED_SLOT`, D-27). Reset clears everything except Elo, humans only (D-26). Names unique case-sensitively, identity by UUID.
  - No confirm-move and no sound; move commit rules stated (§6.2). A tab close suspends immediately (D-14).
  - Platform: Tahoe and later on M1+, Intel while Apple supports it, browsers from the last 3 years. Max-think time comes from the active profile (§4.6). One bundle with the CLI inside (D-16 simplified, D-18 revised). Port scope settled. TUI fixed at 80×24 with a resize attempt.
  - Average length in moves and time. Export is one zip archive (D-29). High contrast = WCAG AA.
  - Traceability now covers GAME-4, PLY-7, UI-10, UI-11, RISK-1/2. A leftover duplicate timeout table in §4.8 was removed. R-17 added.
  - Mermaid not machine-validated: no Node.js was available. Diagrams were hand-checked against the governance syntax rules.
- 2026-10-26, final answers applied and **baselined as v1.0**:
  - Reset (D-26): the Player's old games stay in opponents' history and stats. They are hidden only from views of the reset Player, and a reset point starts that Player's rating history again (§7.2, §7.4).
  - Unfinished games (D-27): a human–computer game never uses a computer's slot, so any Player can always play any computer. Slots are held by each human Player, and by computers only in c–c games (Strict). Policy table, §7.2, §7.4, and §5.5 revised.
  - Computers have a Training Elo that changes in Training games. The K = 40 count is kept per track (`strict_rated_games`, `training_rated_games`), and a computer's category label follows its Strict Elo.
  - All PROPOSED decisions are AGREED: D-12, D-13, D-16–D-20, D-22–D-29. D-12 and D-16 are one-way for data, and the user accepted that (§11.3). D-21 stays DEFERRED. ASM-1 is CONFIRMED.
  - All PROPOSED requirements and TBR items in requirements.md are accepted (Q75), so §10.1 is final, not provisional. The architect-owned TBD values are set (§10.1). The §4.8 engine timeouts are aligned with the accepted EST values (TBD-12, TBD-13).
  - Open items closed or given an owner and a closing step (§10.3, §14). The c–c resume prompt gains Abort (§4.3). Diagrams hand-checked again; the `render(&state…)` label lost its `&`, and a `;` in a §5.3 message was escaped as `#59;`, and a `;` in a §4.3 transition label became a comma.

**Changes 2026-10-27 (v1.0 amendment 1; the baseline statement above stands, and no ID was renumbered):**
- **Reset with unfinished games (user: "allow it; the unfinished game does not count against either player").** `ResetPlayerData` no longer fails with `UnfinishedGameExists`. Its confirmation lists the unfinished games that will be voided, and the reset transaction **voids** each of them: new terminal state `Voided` (§4.3), new `GAME.status = 'voided'` (§7.2), new event `GameVoided` (§5.1), no Elo, W/L/D, or rating-history effect for anyone, and every `UNFINISHED_SLOT` row of the game released. The journal is closed, not deleted (§7.6). PGN export of voided and aborted games is now stated (§5.7, TBD-24 gains `EndStatus`). New reset sequence diagram (§7.2). D-26 revised, D-30 added; OQ-8 and G-20 resolved; `UnfinishedGameExists` stays for game setup only (§5.5). The architect's interpretation of the voided game's visibility is recorded as D-30 for the user to correct.
- **Stockfish 19 pinned (user: "use the latest Stockfish").** Stockfish 19, tag `sf_19`, published 2026-09-05, not a pre-release (verified 2026-10-27 from the GitHub release API). TBD-26 closed as a version choice. D-9 revised. The §8.2 build plan now prefers the official `stockfish-macos-universal` asset, accepted only if `lipo -info` shows both arm64 and x86_64, with the previous build-and-`lipo` plan as the fallback. ASM-3 narrowed to two unverified facts. §4.6 now reads the `UCI_Elo` range from the engine instead of assuming a floor. Any later Stockfish version needs a decision-log entry and a calibration re-run (§11.3).
**Changes 2026-10-28 (v1.0 amendment 2; the baseline statement above stands, and no ID was renumbered).** Source: requirements Amendment 2 (CR-4.1 to CR-4.20) and the handoff items H-01 to H-23. The disposition of every item is in §10.4.
- **Names (H-01, PLY-6).** Case-sensitive uniqueness against every Player (human or computer, active or archived), 128 grapheme clusters, trim, NFC, control characters rejected, message names the rule. §5.5 corrected (it said case-insensitive among active Players, which was wrong). `unicode-segmentation` and `unicode-normalization` added to §3.2.
- **Archive and import (H-02, H-03, H-04).** `games.json` joins the full-data archive and carries the analysis results (the "recomputed after import" sentence is struck). An archive without it still imports. PGN duplicates raise a prompt with **skip (default)** and **keep both**, never replace. An unfinished game exports as `Result "*"` with no `Termination` and no `EndStatus`. Full-data import restores an unfinished game as unfinished unless a slot would be doubled (D-34).
- **In-play rules (H-05 to H-08, D-33).** `movenow`, F2 and a "Move now" button; takeback stops at the human's first turn and removes taken-back moves; all clocks stop in `DrawOffered`; the resume list covers either side of a hot-seat game plus every unfinished c–c game.
- **Players (H-09 to H-11).** Active-Player rules; computer Players cannot be renamed, archived, deleted or reset; a Player with an unfinished game cannot be archived; no un-archive; one request generates 1 to 25 computer Players (default 5), skipping used names; roster UUIDs are derived from the seed.
- **CLI and server (H-12, H-13, H-19, D-36).** Headless is c–c only, stored and rated, with `--data-dir`; `chess serve` fails fast on a bad `config.toml`, survives a browser that will not open, and stops cleanly on SIGINT or SIGTERM; `chess install-cli` and `--remove`; ad-hoc signing; `chess export game|data` and `chess import pgn|data` (UC-22 vs UC-24, UC-23 vs UC-25).
- **Statistics (H-14, H-17).** Empty states, "evaluating…", custom-start games excluded from aggregates, a CLI Elo chart (D-37).
- **Data damage (H-15, H-16, D-35).** Failed pre-upgrade backup stops the upgrade; every store-changing command is all-or-nothing; unreadable journals resume from the longest intact run or are voided as "voided: unreadable"; a restore replays every retained journal (including `journal/done/`) and re-finalises games in end-time order. The architecture adds an **automatic backup cadence**, because without one "journals are kept until the next backup" had no meaning (D-35).
- **Enabling system and allocation (H-18, FG-1 to FG-3, D-31, D-32).** New §2.1 (build and release view) and §2.2 (allocation blocks, including `Tauri shell and IPC adapter`). §2 is stated as run-time only.
- **Log and confirmations (H-20).** The log holds identifiers only and rotates at 10 MB with 3 files; one `ConfirmationRequired` mechanism covers six of the seven UI-12 actions; the seventh, the TUI Ctrl-C quit, changes no data and uses the same dialog widget in the TUI.
- **Requirements (H-21, H-22).** PLAT-6, DAT-7, UI-12, NFR-10 allocated; 26 amended requirements updated in §10.1; TBD-31, TBD-32 and TBC-10 added. No per-function performance budget is allocated (CR-4.19).

**Anchors:** [requirements.md](requirements.md) and [test-strategy.md](test-strategy.md). The systems engineer is baselining requirements.md on 2026-10-26: every PROPOSED requirement and TBR item is accepted (Q75) and Q76–Q79 are answered, so every requirement is AGREED and mandatory (Must). §10 allocates every requirement ID in it.
**Status legend:** `DECIDED (delegated)` = decided by the architect on 2026-10-28 under the user's instruction to resolve gaps by judgement without questions; the user may correct it, and each such entry says what a correction touches · `AGREED` = confirmed by user · `CONFIRMED` = an assumption the user confirmed · `DEFERRED` = two-way door, decided at implementation time. Earlier drafts also used `PROPOSED` (suggested by Claude, awaiting decision) and `OPEN` (the user must choose). At baseline v1.0 no decision has either status; the words survive only in the change log and decision log, which record history.

### Inputs from the user (AGREED 2026-10-24)
| Topic | Decision |
|---|---|
| Language | Rust for the backend (all chess logic, engine control, persistence, servers, CLI). TypeScript for the GUI. |
| Desktop shell | Tauri. |
| Shared core | One core, shared by the CLI, web, and desktop front-ends. |
| CLI style | Full-screen, ratatui-style terminal UI (requirements CLI-1, amended). Keyboard only: moves are typed (no board cursor), and there is no mouse support. |

---

## 1. Architectural principles

1. **One Rust core owns every rule and decision.** Move legality, mode policy (Training/Strict), clocks, Elo, roster generation, game lifecycle, and persistence live in Rust. Front-ends render state and send intents; they never decide what is allowed. This is what makes "same scripted game in all three front-ends gives identical PGN" (PLAT-1) and "each front-end exposes exactly what the policy allows" (test-strategy principle 5) achievable.
2. **Functional core, imperative shell.** Pure crates (rules, domain) take data and return data. I/O — Stockfish processes, SQLite, wall-clock time, sockets, terminals — sits at the edges behind traits, so tests can swap in fakes (fake engine, fake clock, in-memory store).
3. **The engine is never exposed to a front-end.** Stockfish runs only inside the Rust process. Every engine request goes through one gateway that checks the request against the mode policy. Strict-mode integrity (ANA-1) is therefore enforced in one place, and a spy on that gateway proves it.
4. **One protocol, thin transports.** The core's API is a set of serializable commands, queries, and events. The CLI calls it in-process. The desktop app reaches it over Tauri IPC. The web UI reaches it over a localhost WebSocket. The TypeScript types are generated from the Rust types, so the two languages cannot drift apart.
5. **Every state change is durable before it is acknowledged.** Each move, clock checkpoint, and game end is appended to the game's local move journal and flushed to disk (`F_FULLFSYNC`) before the event is broadcast. SQLite is updated from the journal as soon as it is available. If the two disagree, the journal wins (§7.6, NFR-2, RULE-5, Q54).

---

## 2. System context and deployment

This section is the **run-time view only** (D-31). It shows the system while a person uses it. The build, packaging and release environment is a second context view in §2.1, and §2.2 names the blocks that requirements and functions are allocated to.

Everything runs on the user's Mac. The desktop app, the web page, and the CLI all work with no Internet connection, including first launch (PLAT-3, §8.1), and nothing makes outbound connections (NFR-4).

```mermaid
flowchart LR
    user(("User"))
    internet(["Internet"])

    subgraph mac["User's Mac — fully offline at run time (PLAT-3), no outbound traffic (NFR-4)"]
        direction LR
        subgraph p1["Process: chess (one binary on PATH)"]
            cli["CLI front-end<br/>ratatui TUI · plain mode · headless"]
            core1["chess-app core<br/>(in-process)"]
            cli --> core1
        end

        subgraph p2["Process: chess serve (same binary, started by the user)"]
            srv["axum HTTP + WebSocket<br/>127.0.0.1 only · Host/Origin checks · CSP<br/>serves embedded GUI bundle"]
            core2["chess-app core<br/>(in-process)"]
            srv --> core2
        end

        browser["Browser tab<br/>Safari · Chrome · Firefox · Edge<br/>TypeScript GUI"]

        subgraph p3["Process: Chess.app (Tauri desktop, ad-hoc signed local build)"]
            webview["WKWebView<br/>same TypeScript GUI (bundled)"]
            tauriShell["Tauri shell and IPC adapter<br/>desktop/src-tauri"]
            core3["chess-app core<br/>(in-process)"]
            webview -- "Tauri IPC" --> tauriShell
            tauriShell --> core3
        end

        db[("chess.db + journals<br/>SQLite, WAL<br/>~/Library/Application Support/Chess/<br/>journal locks + job queue coordinate processes")]
        sf1["stockfish<br/>(bundled, pinned)"]
        sf2["stockfish<br/>(bundled, pinned)"]
        sf3["stockfish<br/>(bundled, pinned,<br/>started as Tauri sidecar)"]

        core1 --> db
        core2 --> db
        core3 --> db
        core1 -. "UCI over stdin/stdout" .-> sf1
        core2 -. "UCI" .-> sf2
        core3 -. "UCI" .-> sf3
    end

    user --> cli
    user --> browser
    user --> webview
    browser -- "HTTP + WebSocket<br/>loopback only" --> srv
    mac -. "✗ no run-time connections" .- internet
```

Key points:
- **Three processes, one library.** Each entry point is its own OS process that links the same `chess-app` crate. There is no shared daemon. They coordinate only through the data directory: the SQLite file and the per-game journal locks (§7.3, §7.6).
- **One `chess` binary** provides both the CLI (`chess`) and the web server (`chess serve`, which prints the URL and opens it in the default browser). It ships inside `Chess.app` and is put on `PATH` by `chess install-cli`, which makes a symlink (PLAT-6, §8.2), so there is one thing to install and one version (PLAT-1, NFR-5, D-16).
- **Three engine boxes, one block.** The diagram draws `stockfish` once per process because each process starts its own child. It is the same bundled binary. Requirements and functions allocate to the one block named `stockfish`. In the desktop process Tauri starts it as a sidecar, which changes how it is launched and nothing else (FG-2).
- **The desktop adapter is a block.** `Tauri shell and IPC adapter` (`desktop/src-tauri`) sits between the webview and the core. It maps one `invoke("core", envelope)` call onto the protocol dispatcher and forwards events. It holds no rules (principle 1). Functions that carry the protocol over Tauri IPC are allocated to it, not to the process boundary (FG-2, D-32).
- **Stockfish runs as a native child process.** It is pinned to a specific release (Stockfish 19, `sf_19`, D-9), shipped for both arm64 and x86_64 (a universal build, Q46), and bundled once inside `Chess.app`, next to the `chess` CLI binary that also lives in the bundle (§8.2, D-16). The WASM build is not needed, because the engine always runs in Rust, never in the browser. Install locations and how each process finds its engine are covered in §8.2 (D-18).
- **Trust boundary.** The only boundary inside the Mac is loopback. Any local process can reach the `chess serve` port, and the browser is treated as untrusted input to the core (§5.2, R-5, D-22). Everything else stays inside one OS user account, used by one person (ASM-1, CONFIRMED).

### 2.1 Build and release view (enabling system, D-31)

**Question this diagram answers:** which blocks and which external parties produce a release of Chess, and where do the release-time functions (build, engine acquisition, licence audit, platform matrix, install documentation) live? The run-time view in §2 has no block for any of them, and UC-29 is the use case for this scope (FG-1, FG-10, H-18).

```mermaid
flowchart LR
    rel(("Release engineer<br/>(project owner in the build role)"))
    sfsrc(["Stockfish release source<br/>(build time only)"])
    subgraph envb["Build and CI environment (build time, may use the Internet)"]
        direction TB
        tp["third_party/stockfish/<br/>pinned tag, fetch script, SHA-256,<br/>COPYING, source offer"]
        pkg["packaging/<br/>universal build, bundle config,<br/>ad-hoc signing"]
        chk["CI checks<br/>cargo-deny, npm licence checker,<br/>lipo test, offline E2E, nightly calibrate"]
        doc["Release checklist and install docs<br/>README install section,<br/>platform-matrix record"]
    end
    app[["Chess.app (.dmg)<br/>the run-time system of section 2"]]
    user(("User"))
    rel --> doc
    rel --> pkg
    sfsrc -. "download once, checksum" .-> tp
    tp --> pkg
    pkg --> app
    chk -. "audits and tests" .-> app
    chk -. "audits dependencies of" .-> pkg
    doc -. "records the results of" .-> chk
    app -- "installed by the User (UC-1)" --> user
```

| Element | Kind | Role | Functions allocated (functional_decomp) |
|---|---|---|---|
| `Release engineer` | External actor | The project owner in the build role. The same person as the User under ASM-1, acting in a different role. Starts the release procedure, records the platform matrix, signs off the checklist (the sign-off is the owner's, never claimed here) | none: outside the boundary |
| `Stockfish release source` | Supporting external entity | The published `sf_19` release and its checksum. Reached at build time only. Running the app never uses it (PLAT-3, NFR-4) | none |
| `Build and CI environment` | Supporting external entity (boundary) | Runs builds, audits and tests. May use the Internet (§8.1, "Build vs. run") | none: it hosts the blocks below |
| `packaging/` | Block | `Chess.app` build (universal, Tauri target), `Info.plist` minimum system version, ad-hoc signing of the app and its executables, `.dmg`, licence notices | F-8.6 |
| `third_party/stockfish/` | Block | Pinned engine source or asset, `fetch.sh` with the SHA-256, the `lipo -info` and `uci` checks, `COPYING`, source offer | F-8.7 |
| `CI checks` | Block | `cargo-deny`, npm licence checker, the `lipo -info` packaging test, bundle scan, offline E2E, and the nightly `chess calibrate` job | F-8.9, F-3.9 |
| `Release checklist and install docs` | Block | `release/CHECKLIST.md` (platform matrix, FIDE Laws edition check, engine facts, licence result) and the README install, update and uninstall text (PLAT-6, NFR-5) | F-8.8, F-8.10 |

Notes:
- **Calibration (F-3.9).** The function is the nightly job, so it is allocated to `CI checks`. The code it runs is the `chess calibrate` subcommand inside `CLI front-end`. That subcommand is dev tooling, is not reachable through the protocol (§4.5), and refuses to run on the default data directory (R-17).
- **Ownership.** One person plays every role in v1 (ASM-1). The roles are still separate so that a release does not depend on the person remembering what to check: the checklist is the control, not memory (UC-29, RISK-1, NFR-6).
- **Out of scope here:** hosting or distribution, an installer package, notarization, and automatic updates (requirements Out of Scope, PLAT-6).

### 2.2 Allocation blocks (normative names, D-32)

**Why this section exists.** The §2 diagram shows `chess-app core` as one box, so an allocation to it cannot tell the services apart (FG-3). The §4.1 diagram already shows the services. This table gives every block that functions and requirements may be allocated to one exact name. An allocation cell in functional_decomp uses these names verbatim. A name of the form `chess-app core :: X` means block X inside the core, in whichever process runs the core.

| Allocation name | Kind | Crate or path | Runs in | Owner role |
|---|---|---|---|---|
| `User`, `Internet` | External | — | outside the Mac | — |
| `CLI front-end` | Front-end block | `chess-cli` (TUI, plain, headless, `serve`, `install-cli`, `calibrate`, `export`, `import`) | Process: chess; Process: chess serve | CLI developer |
| `axum HTTP + WebSocket` | Transport block | `chess-server` | Process: chess serve | Core developer |
| `Browser tab` | Front-end block | `ui/` | the browser | GUI developer |
| `WKWebView` | Front-end block | `ui/` | Process: Chess.app | GUI developer |
| `Tauri shell and IPC adapter` | Transport block | `desktop/src-tauri` | Process: Chess.app | GUI developer |
| `chess-app core` | Block (whole) | `chess-app` and the crates it links | every process | Core developer |
| `chess-app core :: Protocol dispatcher` | Core part | `chess-app::protocol`, event stream | every process | Core developer |
| `chess-app core :: PlayerService` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: GameSetupService` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: SessionRegistry` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: GameSession` | Core part | `chess-app` (one actor per live game) | every process | Core developer |
| `chess-app core :: Policy` | Core part (pure) | `chess-domain` | every process | Core developer |
| `chess-app core :: Game domain` | Core part (pure) | `chess-domain` (`Game`, `Clock`, `EloCalc`, `RosterGen`, `VariationTree`, `EcoTable`, `BoardGrid`) | every process | Core developer |
| `chess-app core :: chess-rules` | Core part (pure) | `chess-rules` (moves, SAN, FEN, PGN) | every process | Core developer |
| `chess-app core :: EngineGateway` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: chess-engine` | Core part | `chess-engine` (UCI driver, pool, personalities) | every process | Core developer |
| `chess-app core :: AnalysisService` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: AnalysisWorker` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: StatsService` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: ImportExportService` | Core part | `chess-app` | every process | Core developer |
| `chess-app core :: chess-store` | Core part | `chess-store` (store thread, `MoveJournal`, migrations, backups) | every process | Core developer |
| `chess-app core :: Diagnostic log` | Core part | `chess-app` (tracing setup, rotation) | every process | Core developer |
| `chess.db + journals` | Data store | `~/Library/Application Support/Chess/` | the data directory | Core developer |
| `stockfish` | Supporting block | `third_party/stockfish/` build output | child of each process | Core developer |
| `Process: chess (one binary on PATH)`, `Process: chess serve (same binary, started by the user)`, `Process: Chess.app (Tauri desktop, ad-hoc signed local build)` | Process boundaries | — | used for process-level functions only | — |
| `packaging/`, `third_party/stockfish/`, `CI checks`, `Release checklist and install docs` | Enabling-system blocks | §2.1 | build time | Release engineer |

The `User's Mac` boundary is used only for the top function F-0. A cell that names two or three front-end blocks (for example `Browser tab; WKWebView`) means one behaviour realised once per front-end codebase (FG-4); no split of responsibility is implied.

---

## 3. Code organisation

### 3.1 Repository layout (baseline; a two-way door that may change during implementation)

```
chess/
├── Cargo.toml                 # Rust workspace
├── crates/
│   ├── chess-rules/           # board, move gen, SAN/UCI parse+format, FEN, PGN       (pure)
│   ├── chess-domain/          # game state machine, policy, clock, Elo, roster,       (pure)
│   │                          #   variation tree, stats aggregation, ASCII renderer,
│   │                          #   ECO classifier over a bundled opening table (D-20)
│   ├── chess-engine/          # Engine trait, UCI driver, Stockfish process pool,
│   │                          #   computer personalities, FakeEngine
│   ├── chess-store/           # SQLite schema, migrations, repositories, export/import
│   ├── chess-app/             # THE shared core: services, game session actors,
│   │                          #   engine gateway, protocol types (→ TS via ts-rs)
│   ├── chess-server/          # axum: static GUI assets + WebSocket protocol endpoint
│   └── chess-cli/             # bin "chess": ratatui TUI, plain mode, flags, `serve`
├── desktop/
│   └── src-tauri/             # Tauri shell: links chess-app, exposes commands/events
├── ui/                        # TypeScript GUI (Vite + React), shared by web & desktop
│   └── src/
│       ├── protocol/          # generated TS types (do not edit)
│       ├── transport/         # CoreClient interface + ws.ts + tauri.ts
│       ├── board/             # Board contract, BoardModel reducer, AccessibleOverlay,
│       │                      #   chessground adapter + Reconciler, PromotionPicker
│       ├── screens/           # register, home, setup, play, watch, analysis, history,
│       │                      #   stats, computer players, players, import/export, settings, help
│       └── themes/            # ≥10 board themes, ≥5 piece sets
├── e2e/
│   ├── scenarios/             # front-end-neutral scenario files (setup + moves + expected)
│   ├── cli/                   # PTY runner (vt100 screen parsing for the TUI)
│   └── gui/                   # Playwright runner (web + desktop)
├── third_party/stockfish/     # pinned version, build script, LICENSE, source offer
├── packaging/                 # Chess.app/.dmg build, Info.plist, ad-hoc signing (§8.2). `install-cli` is a subcommand of `chess`, not a script
├── ci/                        # cargo-deny config, npm licence check, lipo/bundle tests, nightly calibrate job (§2.1)
├── release/                   # CHECKLIST.md: platform matrix, FIDE Laws edition, engine facts, licence result (§2.1)
└── docs/
```

### 3.2 Crate dependency graph

Arrows point from a crate to the crates it depends on. Pure crates have no I/O dependencies.

```mermaid
flowchart TD
    cli["chess-cli<br/>(bin: chess)"]
    server["chess-server"]
    tauri["desktop/src-tauri"]
    app["chess-app<br/>shared core + protocol"]
    engine["chess-engine"]
    store["chess-store"]
    domain["chess-domain<br/>(pure)"]
    rules["chess-rules<br/>(pure)"]
    ui["ui/ (TypeScript)"]

    cli --> server
    cli --> app
    server --> app
    tauri --> app
    app --> engine
    app --> store
    app --> domain
    engine --> rules
    store --> domain
    domain --> rules

    app -. "ts-rs generates<br/>protocol types (D-7)" .-> ui
    server -. "embeds built assets" .-> ui
    tauri -. "bundles built assets" .-> ui

    subgraph ext["Key third-party dependencies"]
        direction LR
        shak["shakmaty + pgn-reader<br/>(D-3)"]
        tui["ratatui + crossterm<br/>(D-6)"]
        axum["axum + rust-embed"]
        sql["rusqlite (bundled SQLite)"]
        tokio["tokio"]
        uni["unicode-segmentation +<br/>unicode-normalization<br/>(PLY-6)"]
        tomlLib["toml<br/>(config.toml, line numbers)"]
        tauriLib["Tauri"]
        sfbin["Stockfish binary<br/>(pinned, bundled, D-9)"]
        react["React + Vite<br/>(D-5)"]
        cgLib["@lichess-org/chessground<br/>(D-4)"]
    end

    rules -.-> shak
    cli -.-> tui
    server -.-> axum
    store -.-> sql
    app -.-> tokio
    app -.-> uni
    cli -.-> tomlLib
    tauri -.-> tauriLib
    engine -.-> sfbin
    ui -.-> react
    ui -.-> cgLib
```

### 3.3 Crate responsibilities

| Crate | Responsibility | Key types | Requirements |
|---|---|---|---|
| `chess-rules` | Positions, legal-move generation, check/mate/stalemate, draw detection, SAN and coordinate parsing (lenient: either case, optional `+ # x`, `0-0`), SAN formatting, FEN, PGN with variations. Wraps `shakmaty` + `pgn-reader` (GPL-3.0, already perft-verified) rather than writing move generation from scratch (D-3, AGREED). | `Position`, `Move`, `San`, `Fen`, `PgnGame` | RULE-1, CLI-3, RULE-6/7/8, IF-1..3 |
| `chess-domain` | Game state machine; `Policy` (mode × matchup × state → allowed actions); `Clock` with an injected `TimeSource`; Elo calculator (K 40→32, no clamp, exclusions); class bands; seeded roster and personality generator; variation tree; stats aggregation; ECO classification of a move sequence against a bundled opening table (D-20); terminal-independent board renderer that produces a grid of styled cells (glyph, square shade, highlight) for each style (plain, Unicode, ANSI, no-color), used by both the TUI and plain mode. | `Game`, `GameSetup`, `Policy`, `Action`, `Clock`, `EloCalc`, `RosterGen`, `VariationTree`, `EcoTable`, `BoardGrid`, `BoardStyle` | PLY-1/5, GAME-1/3, RULE-2/3/4, ELO-1/2, PLY-4, ENG-2, ANA-2, DAT-3 (ECO), UI-2/5, CLI-1/2, UI-3 |
| `chess-engine` | `Engine` trait; UCI driver over async stdin/stdout; process pool; timeouts, crash detection, restart; engine binary discovery (D-18); `Personality` → UCI options and search limits, plus seeded "random error" move selection; `FakeEngine` for tests. | `Engine`, `UciEngine`, `EnginePool`, `SearchLimits`, `Personality`, `FakeEngine` | ENG-1/2/3, NFR-3 |
| `chess-store` | SQLite access (`rusqlite`, bundled SQLite), WAL, busy timeout, versioned migrations with a pre-migration backup, schema-version guard (D-16), repositories, game leases, analysis job queue, bulk row read/write for export and import (formats are owned by `ImportExportService`), `MoveJournal` (append + `F_FULLFSYNC`, `flock`, replay; §7.6). Runs on its own blocking thread (D-13). | `Store`, `PlayerRepo`, `GameRepo`, `RatingRepo`, `JobQueue`, `Lease` | DAT-1/2/3, RULE-5, NFR-2 |
| `chess-app` | **The shared core.** Name validation (PLY-6: trim, NFC, grapheme-cluster count, control-character check, using `unicode-segmentation` and `unicode-normalization`). The one `ConfirmationRequired` mechanism (UI-12). The diagnostic log writer (NFR-10). Application services that combine domain, store, and engine. One actor per live game. `EngineGateway` (the only path to the engine, policy-checked). Background analysis worker. The protocol: `Command`, `Query`, `Event`, `Snapshot` DTOs and the error catalogue (§5.5). | `Core`, `GameSession`, `EngineGateway`, `AnalysisWorker`, `Command`, `Event`, `GameSnapshot` | PLY-1..5, GAME-1..4, RULE-2..8, ENG-3/5, DAT-2/3, ELO-1/2, UI-1/2/5, ANA-1..3 (allocation in §10) |
| `chess-server` | axum app bound to `127.0.0.1`: serves the embedded GUI build, one WebSocket endpoint carrying the protocol, Host/Origin checks, CSP header, port selection. | `serve(core, port)` | PLAT-1, PLAT-3, PLAT-4, DAT-1, NFR-4, NFR-7 |
| `chess-cli` | `chess` binary: argument parsing (`clap`); `install-cli` and `serve` lifecycle (config errors, signals); `export` and `import` subcommands; full-screen TUI (`ratatui` + `crossterm`) with screens, widgets, and an async event loop over the core; plain line-oriented mode; command/move/key disambiguation; scripted headless mode; `serve` subcommand. | `TuiApp`, `Screen`, `BoardWidget`, `InputLine`, `KeyMap`, `PlainRunner`, `InputParser` | CLI-1..4, UI-2/4, PLAT-1 |
| `desktop/src-tauri` | The block `Tauri shell and IPC adapter` (§2.2): creates the `Core`, maps protocol commands onto Tauri commands, forwards events with `emit`, bundles the Stockfish sidecar, sets the Tauri CSP. | — | PLAT-1, PLAT-3, GUI-1 |
| `ui/` (TypeScript) | One GUI bundle for web and desktop: screens, `Board` component (D-4), typed SAN input, client state (snapshot, view cursor, local clock countdown), themes, `CoreClient` transports. Holds no rules. Controls come from `allowed_actions`. | `CoreClient`, `BoardProps`, `BoardModel`, `AccessibleOverlay`, `Screen`s | GUI-1..4, UI-1..5, GAME-3 (view cursor), PLAT-2 |
| `packaging/` | `.app`/`.dmg` build, bundle metadata (`LSMinimumSystemVersion`), ad-hoc signing of the app and its executables, licence notices (§8.2). Part of the enabling system (§2.1). | — | PLAT-1/2/6, NFR-5 |
| `ci/`, `release/`, `third_party/` | CI checks, release checklist and install docs, pinned engine (§2.1). Enabling system, not shipped | — | NFR-5, NFR-6, IF-4, RISK-1 |


### 3.4 Baseline versions (CR-7.3, FIND-006)

The user's rule: use the latest version of every open-source component and language as of the baselined requirements. "Baselined" is taken as the date of the last baseline amendment, **2026-10-28**. Each version below is the highest stable (not pre-release, not yanked) release published on or before that date, read on 2026-10-30 from crates.io, the npm registry, the Rust release channel (`channel-rust-stable.toml`), nodejs.org and the Stockfish GitHub releases `[RAN]`. `Cargo.lock` and `package-lock.json` record the exact resolved set at implementation; a dependency is raised later only through a normal change, with a CVE scan (§8). A version that needs a newer toolchain than the one installed is a reason to update the toolchain, not to pick an older version.

| Component | Baseline version | Published | Notes |
|---|---|---|---|
| Rust | **1.99.1**, edition **2024** | 2026-10-13 | Workspace `rust-version = "1.99"`. `shakmaty` and `pgn-reader` require 1.97, Tauri 1.90, `rusqlite` 1.89. **The development Mac has 1.89.0 and must be updated first** (`rustup update stable`) |
| Node.js | **26.13.0** (LTS "Lithium") | 2026-10-28 | The development Mac has 26.12.1 (a Current release); update to 26.13.0 |
| TypeScript | **7.0.2** | 2026-07-08 | Strict mode (SWE-001 ASM-S1) |
| `shakmaty` / `pgn-reader` | **0.30.2** / **0.29.1** | 2026-10-07 | D-3. Step-1 dependencies; pre-1.0 versions, so the minor version is pinned and API drift is absorbed inside `chess-rules` |
| `tokio` | **1.54.1** | 2026-10-21 | D-13 |
| `ratatui` / `crossterm` | **0.30.2** / **0.29.0** | 2026-06-19 / 2025-04-05 | D-6 |
| `rusqlite` (bundled SQLite) | **0.41.1** (`libsqlite3-sys` 0.39.1) | 2026-10-28 | D-12. The SQLite release inside `libsqlite3-sys` 0.39.1 was not read `[UNV]`; the CVE scan covers it |
| `axum` | **0.8.9** | 2026-04-14 | Web server; `rust-embed` **8.13.0** |
| Tauri (crate) / `@tauri-apps/api` / `@tauri-apps/cli` | **2.12.5** / **2.12.4** / **2.12.4** | 2026-10-21 / 10-14 / 10-21 | Tauri 2 (the 3.x line is alpha). Fixes the sidecar, IPC and CSP mechanics of §6.3 |
| `ts-rs` | **12.0.1** | 2026-01-31 | D-7 |
| `unicode-segmentation` / `unicode-normalization` | **1.13.3** / **0.1.26** | 2026-06-01 / 2026-10-16 | PLY-6 |
| `toml` / `tracing` / `tracing-appender` / `thiserror` / `serde` | **1.1.8** / **0.1.44** / **0.2.5** / **2.0.21** / **1.0.229** | 2026-10-09 … | Config, logging, errors |
| React / `react-dom` | **19.3.1** | 2026-10-14 | D-5 |
| Vite / `@vitejs/plugin-react` | **8.3.5** / **6.1.2** | 2026-10-21 / 10-05 | D-5. Needs Node 20.19+ or 22.12+ |
| `@lichess-org/chessground` | **10.5.0** | 2026-10-20 | D-4 candidate, subject to the board spike (OQ-9). The unscoped `chessground` package on npm is old (9.2.1, 2025-05-15) and is not used |
| Test and lint tools | `proptest` **1.12.0**, `insta` **1.49.0**, `cargo-mutants` **27.1.0**, `cargo-fuzz` **0.13.2**, `cargo-deny` **0.20.2**, `cargo-audit` **0.22.2**, `expectrl` **0.9.0**, `vt100` **0.18.1**; Vitest **5.0.5**, Playwright **1.64.1**, `@axe-core/playwright` **4.14.0**, ESLint **10.13.0** with `typescript-eslint` **8.72.1**, Prettier **3.9.10** | 2026-05 to 2026-10 | §9. Their presence on the development Mac was not checked |
| Stockfish | **19** (`sf_19`) | 2026-09-05 | D-9, IF-4 (CR-3) |

Not in the table because they do not affect the design: other transitive crates and packages, which are whatever the two lock files resolve.

---

## 4. The shared core (`chess-app`)

### 4.1 Internal structure

```mermaid
flowchart TB
    subgraph fe["Front-end adapters"]
        direction LR
        cliA["CLI adapter<br/>TUI · plain · headless<br/>(direct calls)"]
        wsA["WebSocket adapter<br/>(chess-server)"]
        tauriA["Tauri adapter<br/>(src-tauri)"]
    end

    subgraph core["chess-app :: Core"]
        api["Protocol dispatcher<br/>Command / Query → Result<br/>Event stream (per-game subscription)"]
        players["PlayerService<br/>register (self-assessment → Elo), name rules, rename,<br/>archive, active Player, preferences, computer roster"]
        setup["GameSetupService<br/>validate GameSetup"]
        sessions["SessionRegistry<br/>one GameSession actor per live game<br/>clocks · leases · allowed_actions · resume list"]
        analysis["AnalysisService<br/>Training live analysis,<br/>post-game analysis, what-if"]
        stats["StatsService<br/>aggregates (self-play excluded),<br/>Elo history, any Player"]
        io["ImportExportService<br/>PGN / FEN (review only) / full-data backup"]
        gateway["EngineGateway<br/>purpose-tagged, policy-checked, audited"]
        worker["AnalysisWorker<br/>background per-move evals<br/>(finished games only)"]
    end

    subgraph pure["chess-domain (pure)"]
        direction LR
        policy["Policy<br/>mode × matchup × state → allowed actions"]
        game["Game state machine<br/>Clock · EloCalc · RosterGen"]
    end

    subgraph edges["Edges (traits, swappable in tests)"]
        direction LR
        eng["Engine pool<br/>(Stockfish | Fake)"]
        st["Store<br/>(SQLite file | in-memory)"]
        clk["TimeSource<br/>(system | fake)"]
        rng["Rng<br/>(seeded)"]
    end

    cliA --> api
    wsA --> api
    tauriA --> api
    api -. "events" .-> cliA
    api -. "events" .-> wsA
    api -. "events" .-> tauriA
    api --> players
    api --> setup
    api --> sessions
    api --> analysis
    api --> stats
    api --> io
    setup --> policy
    sessions --> policy
    sessions --> game
    gateway --> policy
    sessions --> gateway
    analysis --> gateway
    worker --> gateway
    gateway --> eng
    players --> st
    sessions --> st
    stats --> st
    io --> st
    worker --> st
    sessions --> clk
    sessions --> rng
```

**Allocation names.** The boxes in this diagram are the normative sub-blocks of `chess-app core` (§2.2). A function or requirement allocated to `chess-app core :: PlayerService` is the service's job, whichever process runs it. The three front-end adapters are `CLI front-end`, `axum HTTP + WebSocket` and `Tauri shell and IPC adapter` (D-32).

**Cross-cutting duties of the core** (they have no box of their own because every service uses them):
- **Confirmation.** A command that needs a user decision returns `ConfirmationRequired` until it carries `confirmed: true` (§5.5). The services that own the actions listed in UI-12 are the only callers (D-33). The check is one shared helper in `chess-app`, not a box of its own, so the functional decomposition allocates F-2.5.7 to `chess-app core`, not to the Protocol dispatcher; the dispatcher only routes the command.
- **All-or-nothing.** Each store-changing command is one store-thread transaction or, for restore and migration, a copy-then-rename. A failure leaves the store as it was and returns a catalogue error whose message states the next step (DAT-7, NFR-9, D-35).
- **Diagnostic log.** `chess-app core :: Diagnostic log` writes identifiers and error detail, never names or move lists (NFR-10, §10.2).

### 4.2 Game sessions as actors

Each live game is one `GameSession`: a tokio task that owns that game's `chess_domain::Game` value. Commands reach it over an `mpsc` channel, and it publishes `Event`s on a `broadcast` channel. Every front-end — including two hot-seat humans on one screen — subscribes to the same stream.

- Commands are handled one at a time, so a game cannot race with itself: a human move, a flag falling, and an engine reply can never interleave.
- **Optimistic concurrency guards rewind (GAME-3).** Every move command carries the `ply` the client believes is current. If a client is rewound, or a computer move landed meanwhile, the `ply` does not match and the command is rejected with `StalePosition`. Rewind itself is purely client-side view state: the snapshot includes the FEN and SAN of every ply, so stepping back needs no engine and no server round trip, and it cannot touch the record.
- **Computer turns** are spawned as cancellable engine tasks whose results return to the actor as messages. The actor stays responsive while the engine thinks: a human can resign, a watcher can pause or abort, and a flag can fall (ENG-5, NFR-8, GAME-2).
- **Clocks** are domain values driven by an injected `TimeSource`. The actor arms a single timer for the active side's flag deadline. **Every state that waits for a decision stops both clocks**: `Paused` and `DrawOffered` (RULE-2, GAME-2). The `Clock` value then has `running_side = None`, the flag timer is disarmed, and the next `Clock` journal record carries the stopped values. Clients receive `{white_ms, black_ms, running_side, as_of}` and count down locally, so there is no stream of tick events. The core alone decides when a flag falls.

### 4.3 Game state machine

```mermaid
stateDiagram-v2
    [*] --> Setup
    Setup --> InProgress: StartGame (setup validated by Policy)

    state InProgress {
        [*] --> AwaitingMove
        AwaitingMove --> EngineThinking: side to move is computer
        EngineThinking --> AwaitingMove: engine move applied (CancelEngine = play best so far)
        EngineThinking --> EngineError: engine crash / timeout / missing binary
        EngineError --> EngineThinking: retry (engine restarted)
        AwaitingMove --> AwaitingMove: human move applied (ply checked)
        AwaitingMove --> AwaitingMove: Takeback (Training only), undoes the human move and the computer reply
        EngineThinking --> AwaitingMove: Takeback (Training only), search cancelled and the human move undone
        AwaitingMove --> DrawOffered: OfferDraw (own turn, none pending, not blocked after a decline), all clocks stop
        DrawOffered --> AwaitingMove: declined, clocks restart with the same side to move
        DrawOffered --> AwaitingMove: engine failure drops the offer to a computer (ENG-4)
        AwaitingMove --> Paused: Pause (computer–computer watcher)
        EngineThinking --> Paused: Pause (engine search cancelled)
        Paused --> EngineThinking: Step (exactly one move)
        Paused --> AwaitingMove: Resume
    }

    note left of InProgress
        Paused and DrawOffered freeze both
        clocks (GAME-2, RULE-2). EngineError does
        not: the side to move keeps its clock
        running (D-15, Q53). An offer to a human
        survives a suspend and resume. An offer to
        a computer is dropped by a suspend.
    end note

    note right of InProgress
        Rewind is client-side view state only:
        it never changes this machine, and clocks
        keep running while a human is rewound.
    end note

    InProgress --> Suspended: app closed / crash / power loss / tab closed or disconnected (D-14)<br/>clocks frozen at last recorded value (Q54). Not an abort: no Elo or stats effect
    Suspended --> InProgress: ResumeGame (journal lock acquired, journal replayed)

    InProgress --> Completed: mate · stalemate · threefold · 50-move ·<br/>insufficient material · flag · draw agreed (computer accepts only if eval ≤ 0) ·<br/>Resign (confirmed, h–h asks which side resigns)
    Suspended --> Completed: Resign from the resume prompt (human games, frees the slot)
    InProgress --> Aborted: Abort (c–c watcher only, confirmed)
    Suspended --> Aborted: Abort from the resume prompt (c–c only, confirmed)
    InProgress --> Voided: confirmed reset of a human Player in this game (D-26, D-30)
    Suspended --> Voided: confirmed reset of a human Player in this game (D-26, D-30)
    Suspended --> Voided: user voids a game whose journal and store are unreadable (DAT-7, D-35)

    Completed --> Finalized: one transaction: result + times + W/L/D,<br/>Elo on the game's track (Training or Strict) for both sides,<br/>unless self-play or custom FEN
    Finalized --> [*]
    Aborted --> [*]
    Voided --> [*]

    note right of Aborted
        c–c only. No result, no Elo, no stats
        for either computer. Kept in history
        and can be analysed.
    end note

    note right of Voided
        h–h and h–c only (only humans are reset).
        No result, no Elo, no W/L/D, no rating
        history point for any side. Every slot
        of the game is released. Kept in the
        opponent history and can be analysed.
        Not a resignation (D-30). An unreadable
        game (DAT-7) is voided by the user with
        termination voided: unreadable.
    end note

    note right of Finalized
        Post-game analysis (ANA-3) and the
        background per-move evaluations (DAT-3)
        start only after this point.
    end note
```

### 4.4 Mode policy as a single table

`Policy::check(action, &GameContext) -> Result<(), Denied>` is a pure function and the single source of truth. `GameContext` holds the mode, matchup, game state, whose turn it is, whether that side is human or computer, and whether the game is still in progress.

| Action | Training (h–c) | Strict (h–c, h–h, c–c) |
|---|---|---|
| Takeback | ✅ to the human's turn. Stops at the human's first turn. During a search, undoes one move; after the computer's reply, undoes both (D-33). Moves taken back leave the record. Nothing to undo: `NothingToTakeBack` | ❌ |
| Live analysis / what-if on the current game | ✅ human only | ❌ until the game ends |
| Rewind (client view) | ✅ human | ✅ human (either hot-seat human); ❌ c–c has no human player, but watchers may scroll the move list |
| Clock selection | ❌ always untimed | ✅ presets + custom |
| Offer draw (humans only; a computer never offers, Q42). Only on the offerer's own turn, never during a search, never while an offer is pending, and not again after a decline until the offerer has moved (D-33). All clocks stop while pending | ✅ to the computer, which accepts only if its eval ≤ 0. The decision runs within the ENG-3 maximum think time and cannot be withdrawn | ✅ h–h: to the other human, no time-out, moves blocked until answered. h–c: to the computer (eval ≤ 0 rule). ❌ c–c |
| Pause / step / delay | ❌ | c–c only |
| Resign (always confirmed) — the only way a human ends a game early. Also from the setup or resume offer (`Suspended` games) | ✅ human: counts as a loss for Training Elo and W/L/D | ✅ h–c, h–h: loss for Strict Elo and W/L/D. In h–h the front-end asks which side resigns, then confirms, from the game and from the resume offer. A self-play resignation has no Elo or W/L/D effect (self-play is unrated). Resigning a custom-start game is recorded in the game record only (H-17). ❌ c–c |
| Abort (always confirmed) | ❌ | c–c watcher only. Neither computer's Elo or stats change |
| Start a new game while a slot-holding Player has an unfinished game in this mode (D-27) | ❌ `UnfinishedGameExists`: resume it, or resign it. Only the human holds a Training slot. The computer is never blocked | ❌ same. h–h: each human holds a slot (one for self-play). h–c: only the human; the computer's slot is neither used nor checked, so any Player can always play any computer. c–c: each computer holds its Strict slot, and the setup offers resume or abort |
| CancelEngine ("move now", ENG-5). Front-end forms: command `movenow`, key F2, GUI "Move now" button (D-33) | ✅ only in `EngineThinking` in an h–c game | ✅ only in `EngineThinking` in an h–c game; ❌ c–c (use Pause); shown unavailable in every other state |
| Elo track | **Training Elo** for both sides, human and computer (computers have a Training Elo too) | **Strict Elo** for both sides, including c–c between different computers. Never for self-play or a custom FEN start |
| Counts in W/L/D | ✅ (filterable by mode) | ✅. Self-play is excluded in both modes |
| Post-game analysis | ✅ | ✅ |

**In-play action rules (D-33).** The rules below are the architecture's reading of requirements RULE-2, RULE-3, ENG-5, GAME-1 and GAME-4 (CR-4.4, CR-4.6, CR-4.7, CR-4.8).
- **Takeback.** The earliest position reachable is the human's first turn: the start position when the human has White; the position after the computer's first move when the human has Black; from a custom FEN, the first position at which it was the human's turn. In `EngineThinking` the search is cancelled and exactly one move, the human's, is undone, so it is the human's turn again. After the computer's reply, both moves are undone. When there is nothing to undo (the earliest position, or the computer is making its opening move), the command returns `NothingToTakeBack` ("Nothing to take back"). **Taken-back moves are removed** from the `MOVE` rows, so from the stored move list, the exported PGN, the per-move times and any evaluation; the append-only journal keeps the `Takeback{to_ply}` record, so replay reaches the same state. A takeback does not change the game's rating track: it stays rated on the Training scale. `analysis_used` is unaffected.
- **Draw offer.** Allowed only on the offerer's own turn, not during a search, and not while an offer is pending. After a decline the same side may offer again only after it has completed another move (`Game` holds `offer_blocked_for: Option<Side>`, cleared when that side moves). While an offer is pending all clocks are stopped, and on a decline they restart with the same side to move. An offer to a human has no time-out and blocks moves until it is answered. An offer to a computer starts `ComputerDrawDecision`, which must finish within the ENG-3 maximum think time, shows the thinking indicator, and cannot be withdrawn. An engine failure drops the offer (ENG-4). Acceptance ends the game 1/2-1/2 with termination "draw agreed". Rejected attempts return `Denied` with the reasons `draw_not_own_turn`, `draw_engine_busy`, `draw_offer_pending`, `draw_repeat_blocked`, `draw_not_available` (c–c, or a Strict-only or human-only rule).
- **Move now.** `CancelEngine` is accepted only in `EngineThinking` of an h–c game. The command word is `movenow`, the key is F2 (TUI and GUI), and the GUI thinking indicator carries a "Move now" button that is keyboard reachable, has an ARIA label, and activates on Enter or Space. The core sends UCI `stop`, plays the best move found so far, and the move must land within 200 ms (TBD-12, §4.8). Elsewhere the control is shown unavailable.
- **Resume list.** `ListUnfinishedGames{player_id}` returns every unfinished game in which the active Player plays either side (the second side of a hot-seat game included; self-play appears once) **plus every unfinished c–c game**. It reads `GAME` for `status = 'in_progress'`, not the slot table, so a game held by a live session in another process is listed and marked "open in the CLI" (`GameLeased`). Resigning from the offer in a hot-seat game asks which side resigns, then confirms.

Every `GameSnapshot` includes `allowed_actions`, computed from this table. Front-ends render controls from that list and never re-derive the rules themselves. The table drives (a) table-driven unit tests over the full mode × matchup × state product, and (b) per-front-end E2E checks that the visible controls match `allowed_actions` exactly.

### 4.5 Engine gateway and Strict-mode integrity

Every engine call passes through `EngineGateway::request(purpose, …)`, where:

```rust
enum EnginePurpose {
    ComputerMove { game_id, player_id },          // a computer side choosing its move
    ComputerDrawDecision { game_id, player_id },  // the "eval ≤ 0" check (RULE-2); result never shown
    LiveAnalysis { game_id },                     // Training only
    WhatIf { game_id },                           // Training live, or any finished game
    PostGameAnalysis { game_id },                 // finished games only
    Calibration,                                  // dev/nightly tooling
}
```

The gateway looks up the game's state and mode and calls `Policy::check` before it touches the engine. `LiveAnalysis` or `WhatIf` on an in-progress Strict game returns `Denied`, even if a modified client sends it directly over the WebSocket. Every call is recorded in an in-memory audit log, which is the "spy on engine calls" required by ANA-1's verification. `Calibration` cannot be sent through the protocol. Only the `chess calibrate` dev subcommand builds it, and it never names a user game.

The gateway also validates engine output. Every `bestmove`, including a seeded "error" pick, is checked for legality with `chess-rules` before the session applies it. An illegal or unparsable move counts as an engine failure (§4.7), so the game can never enter an illegal state (NFR-3).

### 4.6 Computer Players

- **Roster generation** (pure, `chess-domain`): `RosterGen::new(seed)` produces 5 Players per class. Each gets a human-readable name and a `Personality { skill_level, depth_limit, movetime_ms, uci_elo: Option, error_rate, error_multipv }` drawn within the class bounds, plus **two starting ratings, Strict Elo and Training Elo, both equal to its class baseline** (Beginner 400, Novice 600, Intermediate 1200, Master 2200, Grand Master 2500). That is the same rule as a human's self-assessment (Q41). From then on both ratings are **dynamic**: every rated game updates both sides on that game's track by the Elo formula, with the same K-factor rule as humans: 40 for the first 30 rated games **on that track**, then 32. The two tracks count separately (`strict_rated_games`, `training_rated_games`; Q78). The change is written to `RATING_HISTORY`. Its **engine parameters stay fixed** forever, so its rating converges toward its real strength. The **category label follows the current Strict Elo** (Q79) through the ELO-1 bands, never the Training Elo, while the engine settings stay those of the class it was created in. Generation rejects duplicate parameter sets across all computers (ENG-2). The result is persisted once and never regenerated (PLY-4, ENG-2). **One request creates N Players, 1 to 25, default 5 (TBD-31), in the chosen category** (`GenerateComputerPlayers{category, count}`). Each Player is a position in the seed's stream: `roster_index` fixes its personality and its UUID, which is derived deterministically as a UUIDv5 of `(roster_seed, roster_index)` (D-34), so an archive from another install matches by identity and DAT-2 prompts only for real conflicts. The **name** is the first unused candidate from that position's candidate names: the generator skips any name already used by any Player, human or computer, active or archived, so no collision prompt exists and no request can fail on a name. All N are inserted in one transaction or none (DAT-7).
- **Playing a move:** limits = min(personality movetime, the active profile's max-think-time preference (UI-1), a clock-derived budget). Classes whose target strength lies inside the engine's `UCI_Elo` range use `UCI_LimitStrength`. The range is **never hard-coded**: `chess-engine` reads the `min` and `max` of `option name UCI_Elo type spin …` from the `uci` handshake of the pinned binary (Stockfish 19, D-9), and passes it as data to `RosterGen` (still pure) and to `chess calibrate`. An engine integration test asserts that every stored `COMPUTER_PROFILE.uci_elo` lies inside the range the bundled binary reports. Earlier releases had a floor of 1320; Stockfish 19's floor is not yet verified (ASM-3). If the range changes, calibration is re-run before release. Beginner and Novice use a low `Skill Level`, shallow depth, and a random error: with probability `error_rate`, the core picks a non-best line from a MultiPV search. The RNG is seeded from `(game_id, ply)`, so replays are reproducible.
- **Calibration** is a separate `chess calibrate` dev subcommand (nightly CI). It plays matches between classes and reference settings and reports measured Elo and SPRT results against the class baselines, on the baseline machine: M4, 16 GB, macOS Tahoe (Q47). Because ratings are now dynamic (Q41), calibration is used to set the generator bounds before release and as a regression check. It never rewrites a released Player's parameters (R-2). Strength depends on `Threads`, `Hash`, and hardware, so computer-Player instances use fixed `Threads`/`Hash` values. Those values are recorded with each calibration run. Changing them means recalibrating.
- **Whose max-think time applies (UI-1, user 2026-10-26).** It is always the **active profile's** setting, meaning the Player chosen in the profile chooser for that front-end session. In h–c it caps the computer's moves. In c–c it caps both computers' moves for the watching profile. In h–h there are no engine moves, so it caps only the engine analysis that profile requests: post-game analysis, deeper lines, and what-if. It can only shorten a limit, never lengthen it. The session passes the active profile to `EngineGateway` with each request.

### 4.7 Engine process management

| Use | Engine instances | Priority |
|---|---|---|
| Computer side(s) of a live game | One per computer side. Self-play c–c (same Player on both sides) gets two instances with identical options, so neither side shares the other's hash table. | Normal |
| Training live analysis / what-if | One shared analysis instance per process, `MultiPV 3` | Normal |
| Post-game analysis + background per-move evals | The same analysis instance; the background worker yields to interactive requests | Low (`nice`) |

Instances start lazily, which keeps CLI startup under 1 s (NFR-1). They are health-checked with `isready`, killed on timeout, and restarted transparently. If the binary is missing or crashes, the core emits `EngineUnavailable`, leaves the game in a resumable state (ENG-1), and lets the user retry (`RetryEngine`) or leave (the game is suspended). Engine failures are: process exit, no `uciok`/`readyok` within the handshake timeout, no `bestmove` within the search deadline (§4.8), or an illegal or unparsable move (§4.5). While the engine is unavailable, the side to move's clock keeps running (D-15, decided by Q53). A computer that can't recover in time can lose on time.

### 4.8 Concurrency and threading model (D-13)

Nothing slow ever runs on a UI thread: the browser main thread, the WKWebView main thread, or the TUI's `select!` loop. Engine searches run in a separate OS process. SQLite calls run on a dedicated store thread. The UI only sends commands and receives events.

| Execution context | Runs | Must not |
|---|---|---|
| tokio multi-thread runtime, one per process | Protocol dispatcher, `GameSession` actors, engine I/O tasks (async stdin/stdout), `AnalysisWorker`, axum (web), Tauri command handlers (desktop), TUI event loop (CLI) | Block. `rusqlite` is synchronous, so no SQLite call runs on a runtime worker |
| Store thread, one per process | Owns the single write connection. Requests arrive over a channel, and every transaction in §7.4 runs here. Read queries (stats, history) use a small pool of read connections through `spawn_blocking`. WAL lets readers run during a write | Hold a transaction across an `await` or an engine call |
| Stockfish child processes | Searches (§4.7) | Be reached by anything but `chess-engine` |
| `GameSession` actor, one tokio task per live game | All state changes of its game, one message at a time (§4.2) | Wait on the engine inline. Results come back as messages |
| GUI main thread (JS) | Rendering, `BoardModel` reducer, local clock countdown | Compute rules. Legal moves arrive in the snapshot, and what-if moves use `LegalMoves` (D-11) |
| TUI loop (`tokio::select!`) | Key, tick, signal, and core-event handling, plus `render` | Wait on a core future inline. Effects are spawned, and results come back as `Msg`s |

**Ordering and durability.** The actor applies a command to a copy of its `Game`, then appends the resulting record to the game's move journal and flushes it with `F_FULLFSYNC` (§7.6). Only after that does it swap in the new state and broadcast the event (principle 5). Writing to SQLite happens afterwards, on the store thread, so a busy or locked database never delays a move. If the journal write fails (disk full or read-only), the old state stays and the command returns `JournalWriteFailed` (§5.5).

**Backpressure.** Event channels are bounded `broadcast` channels. A subscriber that lags (`RecvError::Lagged`) is sent a fresh `GameSnapshot` instead of the missed events, so a slow WebSocket client can never stall a game.

**Timeouts and periods.** Values are `ESTIMATE`s for the baseline machine (M4, 16 GB, macOS Tahoe; Q47, Q53), to be confirmed by the engine integration benchmarks. Tests set them through config. Intel Macs use the same wall-clock values, so they get less search in the same time (R-14).

**Budget allocation is a Should (CR-7.4, FIND-007).** Splitting the NFR-8 100 ms budget across flush, core, transport and render, and setting a memory ceiling for the engine pool, are Should items of the `chess-store` and `GameSession` design. The design proceeds without them; the NFR-8 latency test (Must) still measures the whole path. They are done if the ASM-2 measurement shows the flush is a large share of the budget.

| Item | Value | On expiry |
|---|---|---|
| UCI handshake (`uci`→`uciok`, `isready`→`readyok`) (ENG-4, TBD-13) | 5 s (the accepted EST in ENG-4). Startup is expected to be well under 1 s on the baseline | Kill and restart once, then `EngineUnavailable` |
| Search deadline (ENG-4, TBD-13) | at the search's own limit + 500 ms, send `stop` | No `bestmove` by the limit + 2 s (the accepted EST in ENG-4) → kill, engine failure |
| `CancelEngine` → move played (ENG-5, TBD-12) | ≤ 200 ms (the accepted EST). Design target 100 ms (`stop` → `bestmove`) | Treated as a search-deadline failure |
| Computer move, top class (ENG-3) | ≤ 3 s by default, capped by the clock budget and the max-think preference | — |
| Computer draw decision (`ComputerDrawDecision`, RULE-2) | within the same ENG-3 maximum think time as a move. All clocks are stopped while it runs | Engine failure drops the offer (ENG-4). A suspend drops it |
| Web server stop (SIGINT or SIGTERM, PLAT-4) | ≤ 5 s `ESTIMATE` to stop accepting, notify pages, suspend open games (journal flush each), then exit 0 | Exit anyway: every acknowledged move is already durable, so nothing is lost |
| Training live analysis (ANA-1) | MultiPV 3, streamed, refined until the user leaves, capped at 10 s per position | Shows the deepest result so far |
| Background per-move evaluation (DAT-3) | 200 ms per ply at low priority (about 30 s for an 80-move game) | Job resumes from the last ply (§7.4) |
| On-demand deeper line / what-if (ANA-2/3) | 2 s per request | Shows the best line so far |
| Journal flush (`F_FULLFSYNC`) per record | a few ms per move. Measured in the NFR-8 latency test | `JournalWriteFailed` |
| Journal clock checkpoint (RULE-5, Q54, TBD-7) | every 1 s while a clock runs, plus on every move, pause, and suspend | After a crash, the clock is restored to the last checkpoint: at most 1 s is lost |
| Journal → SQLite apply | after every journal record, retried with backoff (0.1 s → 5 s) while the DB is busy or locked | Play continues. The backlog is shown in the status line if it exceeds 30 s |
| SQLite `busy_timeout` | 5 s (§7.1) | `StoreBusy` for non-game writes. Game writes are retried from the journal |
| Game lock (journal `flock`) and lease row heartbeat (DAT-4, TBD-28) | the lock is released by the OS when the owner exits or crashes. Heartbeat every 2 s, used only for display | Another process can take over immediately after a crash (§7.3) |
| Client command timeout (GUI) | 10 s | Show "core not responding", resubscribe (§5.6) |
| Last front-end detaches (tab closed, WebSocket dropped; D-14) | immediately, no grace | `Suspend` is journalled and the clocks freeze at the recorded value. When a client resubscribes, the game resumes |

---

## 5. Protocol (front-end ↔ core)

One set of Rust types in `chess-app::protocol`, `serde`-serialized, with TypeScript types exported by `ts-rs` into `ui/src/protocol/`. A CI check fails if the generated files are stale.

### 5.1 Shape

| Kind | Examples |
|---|---|
| **Command** (changes state, returns `Result`) | `RegisterPlayer`, `RenamePlayer`, `ArchivePlayer`, `UpdatePreferences`, `GenerateComputerPlayers{category, count}`, `StartGame{game_uuid, GameSetup}`, `ResumeGame`, `DetachGame`, `MakeMove{game_id, ply, mv}`, `Takeback{ply}`, `OfferDraw`, `RespondDraw{accept}`, `Resign{side, confirmed}` (h–h: the side chosen in the prompt), `Pause`, `Resume`, `Step`, `SetMoveDelay`, `Abort{confirmed}` (c–c watcher only), `CancelEngine` ("move now", ENG-5), `RetryEngine`, `VoidUnreadableGame{game_id}` (offered when a journal and the store are both unreadable, DAT-7; the offer dialog is the confirmation, so no extra step is added to UI-12), `ImportPgn{file, on_duplicate}`, `ImportFen`, `PreviewDataImport{file}` → conflicts, `ApplyDataImport{plan, confirmed}`, `CreateFreshStore`, `RestoreBackup{confirmed}`, `SaveVariation`, `DeleteVariation`, `SelectProfile{player_id}`, `ArchivePlayer{confirmed}`, `ResetPlayerData{player_id, confirmed, void_game_ids}` (humans only. Unconfirmed, it returns `ConfirmationRequired` listing the unfinished games that will be voided, and the confirmed command echoes that list back as `void_game_ids`, §7.2) |
| **Query** (no change to the record) | `ListUnfinishedGames{player_id}` (the resume list, §4.4), `ListPlayers{kind, include_archived}`, `GetPreferences{player_id}`, `GetPlayerStats{player_id}`, `GetRatingHistory`, `ListGames{filter}`, `GetGame`, `GetGameAnalysis{game_id}` (stored `MOVE_EVAL` rows, job state, and `evaluated` and `total` ply counts so a client can show "n of N plies evaluated"), `ExportPgn`, `ExportData`, `LegalMoves{fen}`, `Analyze{purpose, fen, multipv}` (policy-checked; may set `analysis_used`) |
| **Event** (pushed, per game subscription) | `GameSnapshot` (full state on subscribe), `MoveApplied{ply, san, uci, fen, clocks, allowed_actions}`, `EngineThinking{side}`, `ClockSync`, `TakebackApplied{to_ply}`, `DrawOffered`, `DrawDeclined`, `PauseChanged{paused}`, `GameEnded{result, reason, elo_changes}`, `GameVoided{game_id, reason, reset_player_id?}` (the game was voided by a reset of one of its human Players, D-30, or by the user after an unreadable journal, `reason = unreadable`, DAT-7; the client leaves the game screen, shows why, and sends no more commands for it), `RatingsUpdated`, `GameSuspended`, `StoreBacklog{records}`, `AnalysisProgress`, `EngineUnavailable`, `EngineRecovered`, `LeaseLost`, `ServerStopping` (the web server is stopping; the page shows "server stopped" and does not retry, D-36) |

`GameSnapshot` carries everything a client needs to render and rewind without asking the server again: setup, players, every ply (SAN, UCI, FEN, timestamp, clock after the move), the current ply's legal moves (for highlighting and keyboard input), clocks, state, and `allowed_actions`.

### 5.2 Transports

| Front-end | Transport | Notes |
|---|---|---|
| CLI | In-process: `core.execute(cmd).await`, `core.subscribe(game_id)` | No serialization. |
| Web | One WebSocket at `ws://127.0.0.1:<port>/api`, JSON frames `{id, kind, body}` | Bound to loopback only. Rejects any `Host`/`Origin` other than `127.0.0.1:<port>` / `localhost:<port>` (blocks DNS rebinding and drive-by pages). |
| Desktop | Tauri `invoke("core", envelope)` + `listen("core-event")` | Same JSON envelope as the web, so the TS client differs only in its send/receive functions. No TCP port opened. |

In the UI, one `CoreClient` interface hides the difference:

```ts
interface CoreClient {
  execute<C extends Command>(cmd: C): Promise<ResultOf<C>>;
  query<Q extends Query>(q: Q): Promise<ResultOf<Q>>;
  subscribe(gameId: GameId, onEvent: (e: Event) => void): Unsubscribe;
}
// ui/src/transport/ws.ts    → WebSocketCoreClient
// ui/src/transport/tauri.ts → TauriCoreClient   (chosen at startup: window.__TAURI__ ?)
```

### 5.3 Example: human move then computer reply (Strict, timed)

```mermaid
sequenceDiagram
    autonumber
    actor H as Human (GUI)
    participant B as Board (chessground + BoardModel)
    participant UI as TS GUI (screen + client state)
    participant T as Transport (WS or Tauri)
    participant S as GameSession actor
    participant P as Policy + Domain (pure)
    participant J as Move journal (fsync)
    participant DB as Store (SQLite)
    participant G as EngineGateway
    participant SF as Stockfish

    H->>B: drag or click e2→e4 / keyboard on overlay / types "e4"
    B->>UI: onMove(e2, e4)
    UI->>T: MakeMove{game, ply: 12, mv: "e2e4"}
    T->>S: command
    S->>P: check(MakeMove, ply) + apply(move, now)
    alt legal and ply current
        P-->>S: new Game state, clocks updated (+increment)
        S->>J: append MoveRecord{seq, ply, uci, clocks} + F_FULLFSYNC
        J-->>S: durable
        S-)DB: apply record (async, retried while busy#59; §7.6)
        S-->>T: Event MoveApplied{ply 13, san "e4", fen, clocks, allowed_actions}
        T-->>UI: event → re-render
        UI-->>B: BoardProps{fen, lastMove, interactive: false}
    else IllegalMove or StalePosition
        S-->>T: error {code, message}
        T-->>UI: show message
        UI-->>B: BoardProps{fen unchanged} → chessground reverts the piece
        Note over B,S: No engine request follows a rejected move
    end
    opt move was accepted and Black is a computer
        S-->>T: Event EngineThinking{black}
        S->>G: request(ComputerMove, fen, limits from personality ∧ clock ∧ max-think)
        G->>P: check(purpose, game context) ✓
        G->>SF: position fen … / go wtime … movetime …
        Note over UI,S: Actor stays responsive: Resign, flag fall,<br/>or pause can arrive and cancel the search
        SF-->>G: bestmove e7e5 (or a seeded "error" pick from MultiPV)
        G->>P: legal? (NFR-3, §4.5)
        G-->>S: engine result message
        S->>P: apply(move, now)
        S->>J: append MoveRecord + F_FULLFSYNC
        S-)DB: apply record (async)
        S-->>T: Event MoveApplied{ply 14, ...}
        T-->>UI: event → re-render
        UI-->>B: BoardProps{fen, lastMove, interactive: true} → animated move
    end
```

### 5.4 Example: game end, rating, and background analysis

```mermaid
sequenceDiagram
    autonumber
    participant S as GameSession actor
    participant P as Domain (pure)
    participant J as Move journal
    participant DB as Store
    participant W as AnalysisWorker
    participant G as EngineGateway
    participant A as AnalysisService
    participant UI as Front-end

    S->>P: apply(move) → Terminal(Checkmate, 1-0)
    S->>J: append GameEnd record + F_FULLFSYNC (the result is now durable)
    S->>DB: finalize(game) on the store thread
    Note over DB,P: One BEGIN IMMEDIATE transaction: re-read both Players' current Elo<br/>(a computer may have finished another game in another process),<br/>EloCalc per side on the game's track (each Player's own K on that track: 40 for the first 30 rated games, then 32),<br/>write result + reason + times + rating_history for both sides, enqueue analysis_job
    DB->>P: EloCalc(current ratings, K, rated flag)
    P-->>DB: deltas on the game's track (Training or Strict), or none (self-play / custom FEN)
    DB-->>S: committed, elo_changes
    S-->>UI: GameEnded{result, reason, elo_changes, offer_analysis: true}
    Note over S,UI: If the DB is busy, GameEnded is sent with elo_changes pending,<br/>and RatingsUpdated follows once the journal record is applied
    Note over W,G: Per-move evaluation runs only after the game ends,<br/>never during play (DAT-3, ANA-1)
    W->>DB: claim job (lease)
    loop each ply, low priority
        W->>G: request(PostGameAnalysis, fen)
        G-->>W: eval, best line
        W->>DB: upsert move_eval (cp, mate, best, cp_loss, flag)
    end
    W->>DB: mark job done (accuracy is derived from MOVE_EVAL by query)
    UI->>A: (optional) open post-game analysis → GetGameAnalysis reads stored evals,<br/>Analyze / LegalMoves for deeper lines and what-if on demand
    A->>G: request(PostGameAnalysis or WhatIf, fen) — allowed, game is finished
```

### 5.5 Error catalogue

Every `Result` error crosses the protocol as `{code, message, details}`. `code` is a stable string from this table. Front-ends branch on `code` and show `message`, and new codes may be added without breaking clients (§5.6). Rust crates use `thiserror` and are mapped to these codes in `chess-app`. No internal error text (paths, SQL) reaches `message`. Internal detail goes to the log.

| Code | Raised by | Meaning | Client behaviour | Retry? |
|---|---|---|---|---|
| `ConfirmationRequired{action, voids}` | services | A destructive command (resign, c–c abort, archive, reset player data, apply an import after its preview, restore a backup) was sent without `confirmed: true` (Q48, Q52, UI-12). The core enforces this, so no front-end can skip the prompt. The seventh UI-12 action, quitting the TUI with Ctrl-C, changes no data and is confirmed by the TUI's own dialog, which is the same dialog widget as the others. For a reset, `voids` lists the Player's unfinished games (opponent, mode, moves played) that the reset will void (D-30). It is also returned when, at commit time, the Player has an unfinished game that is not in the confirmed reset's `void_game_ids`, so a game the user never saw listed is never voided. A listed game that ended normally in the meantime is simply not voided | Show the confirmation dialog. For a reset, the dialog says that each listed game will be voided: no result, and it counts for neither player | After confirming |
| `UnfinishedGameExists{game_id, mode, player_id}` | `GameSetupService` | A slot-holding Player in the new game (a human, or a computer in a c–c game) already has an unfinished game in that mode (GAME-4, D-27). Since 2026-10-27 it is **not** a reset outcome: a reset voids unfinished games instead (D-30) | Offer Resume, or Resign (human games) or Abort (c–c), each confirmed | After resume, resign, or abort |
| `JournalWriteFailed{cause}` | `GameSession` | The move journal can't be written (disk full, read-only) | The move isn't made. Show the cause. The clock keeps running | User-initiated |
| `StoreMissing` | `chess-store` on open | The data dir shows an earlier store existed (marker or backups), but `chess.db` is gone (Q51) | Choose: start fresh, or import from a file | — |
| `ImportConflicts{list}` | `ImportExportService` | Preview found Players or games that already exist (Q51) | Full-data import: for each one, choose Replace, or Keep both as distinct identities. **PGN import:** a duplicate game (identical tags and moves to a stored game or an earlier game in the same file) offers only **Skip this game (default)** and **Keep both**, plus "apply to all remaining collisions". Replace is not offered (D-34) | After choosing |
| `IllegalMove{reason}` | `GameSession` / `chess-rules` | Move not legal in the current position | Show reason, keep input editable (CLI-3), revert the piece (GUI) | No |
| `StalePosition{live_ply}` | `GameSession` | `ply` ≠ live ply (rewound, or a move landed meanwhile) | Show the live-position badge, refresh from the snapshot | After refresh |
| `Denied{reason}` | `Policy` (via services and gateway) | Action not allowed in this mode, matchup, or state. Reasons include the draw-offer set (§4.4), `computer_player` (rename, archive or reset of a computer Player), `active_player` (archiving the Player that this front-end session has selected), and `voided` | Should not happen for a correct client (controls come from `allowed_actions`). Show reason | No |
| `ValidationFailed{field, reason}` | `PlayerService`, `GameSetupService`, preferences | Bad name, bad setup or setting. A name is trimmed of leading and trailing whitespace, then NFC-normalised (no other folding). The rule that failed is named in `reason` and in the message: `empty`, `too_long` (over 128 user-perceived characters, i.e. extended grapheme clusters, UAX #29), `character_not_allowed` (control characters, Unicode category Cc, and line or paragraph separators, Zl and Zp), or `already_used`. `already_used` is **case-sensitive against every Player**, human or computer, active or archived, and the message says "by an archived Player" when so (PLY-6, CR-4.1) | Highlight the field and show the rule | After fixing input |
| `ParseError{kind: Pgn\|Fen, game_index, line, col, reason}` | `ImportExportService` | Malformed PGN/FEN (RULE-7, RULE-8, IF-2, IF-3) | List errors per game. The other games in the file still import | No |
| `NotFound{entity, id}` | services | Unknown Player, game, or variation | Refresh the list | No |
| `GameLeased{owner_kind}` | `SessionRegistry` | The game is live in another process ("open in the CLI") | Show the owner, offer read-only view | When the owner releases it or its lease goes stale |
| `LeaseLost` (event) | `GameSession` | This process lost the lease (another process took it over after a stall) | Drop to read-only, explain | No |
| `EngineUnavailable{cause}` | `chess-engine` / gateway | Binary missing, crash, timeout, illegal output | Offer Retry / Leave (ENG-1) | `RetryEngine` |
| `StoreBusy` | `chess-store` | `busy_timeout` exceeded | Transparent retry once in the core, then show | Yes, once |
| `StoreUnavailable{cause}` | `chess-store` | I/O error, disk full, read-only file. **Every store-changing command** (register, rename, archive, preferences, generate, reset, import, restore) is all-or-nothing and, on any failure, returns this or `StoreBusy` with a message naming the failure and the next step (DAT-7, NFR-9) | For non-game writes: show. A live game keeps playing from the journal, and the backlog is applied later (§7.6) | User-initiated |
| `StoreCorrupt` | `chess-store` on open | `quick_check` failed (§7.1) | Restore-from-backup flow | — |
| `SchemaTooNew{db_version, app_version}` | `chess-store` on open | The database was migrated by a newer build (D-16) | Tell the user to update this entry point. Nothing is written | — |
| `Busy{reason}` | `ImportExportService` | Import or restore needs exclusive access, but another process is live (§7.4) | Ask the user to close the other entry points | Yes |
| `ProtocolMismatch{server, client}` | WebSocket adapter | A browser tab from an older build (D-16) | Hard reload | After reload |
| `NothingToTakeBack` | `GameSession` | Takeback at the earliest reachable position (§4.4). Message: "Nothing to take back" | Show the message, change nothing | No |
| `PlayerHasUnfinishedGames{game_ids}` | `PlayerService` | `ArchivePlayer` on a Player with an unfinished game (PLY-4, GAME-4). The message names the games | Offer Resume or Resign for each. Reset is the only action that voids games (D-30) | After resume or resign |
| `BackupFailed{cause}` | `chess-store` on upgrade | The pre-upgrade backup could not be written (DAT-6, D-35). Nothing was migrated | Show the cause. Offer Retry or Exit. A newer front-end never opens an old-version store without the upgrade | User-initiated |
| `JournalUnreadable{game_id, recovered_moves}` | `SessionRegistry` at resume | The journal could not be read in full (DAT-7). `recovered_moves` is the length of the longest intact run held by the journal or the store. Resume continues from it and the client reports the count. When neither is readable the client offers `VoidUnreadableGame` | Show the count, or offer the void | No |
| `ConfigInvalid{file, line, reason}` | `chess-cli` (`serve`) | `config.toml` is unreadable, unparsable, or `server.port` is outside 1 to 65535 (PLAT-4). Never a silent fallback | Print the message, exit 1 | After fixing the file |
| `Internal{id}` | any | Bug. Logged with a correlation id | Generic message with the id | No |

### 5.6 Protocol reliability, idempotency, and versioning

- **Envelope.** `{id, kind, body}`. `id` is a client-chosen request id, echoed in the response. The web and desktop transports have a per-request client timeout (§4.8).
- **Idempotency.**
  - `MakeMove`, `Takeback{ply}`, `Step`, `OfferDraw`, `RespondDraw`, `CancelEngine`, `VoidUnreadableGame`, and `Resign` are guarded by the game state. A duplicate after a reconnect gets `StalePosition` or `Denied` and changes nothing. A command for a voided game gets `Denied{reason: voided}`.
  - A duplicate confirmed `ResetPlayerData` is harmless: the games it listed are already voided, so it voids nothing, adds a second reset point at the same rating, and moves `reset_at` past no game, which changes no view. Pending requests are never replayed blindly anyway (see Reconnect).
  - `StartGame` carries a client-generated `game_uuid`. A repeat with the same uuid returns the existing game instead of creating a second one.
  - Imports are all-or-nothing per game and dedupe on a content hash, so re-importing the same PGN reports "already imported".
  - `RegisterPlayer` relies on unique names.
- **Reconnect (web).** If the WebSocket drops, the client reconnects with backoff (0.5, 1, 2, 4 s, capped), resubscribes, and treats the new `GameSnapshot` as authoritative. Pending requests fail with a "connection lost" message and are never replayed blindly. While disconnected, inputs are disabled and a banner is shown. If the session gets no subscriber back, D-14 decides what the game does.
- **Versioning.** The bundle and the core always come from the same build: the bundle is embedded in `chess` or bundled in `Chess.app`. The handshake sends `protocol_version` (the build version) both ways. A mismatch, such as a stale tab after an upgrade, returns `ProtocolMismatch` and asks for a reload. Within one version, unknown event kinds are ignored by clients, which keeps tests tolerant of additive changes. There is no external API, so no deprecation policy is needed beyond this.

### 5.7 Internal interface contracts

| Interface | Provider → consumer | Style | Contract essentials | Failure behaviour |
|---|---|---|---|---|
| Protocol (`Command`/`Query`/`Event`) | `chess-app` → CLI, `chess-server`, `src-tauri`, `ui/` | Async request/response plus a per-game event stream | §5.1–§5.6. TS types generated (D-7). `allowed_actions` in every snapshot | §5.5 codes, §5.6 reconnect |
| `Engine` trait | `chess-engine` → `EngineGateway` | Async, cancellable | `search(position, SearchLimits, multipv) → SearchResult{best, pv, score, depth}`, `stop()`, `health()`. Only the gateway holds an `Engine` | Typed `EngineError` → §4.7 |
| `Store` trait and repositories | `chess-store` → `chess-app` services | Calls to the store thread over a channel (D-13) | One method per transaction in §7.4. No SQL outside `chess-store` | `StoreBusy` / `StoreUnavailable` / `SchemaTooNew` |
| `TimeSource`, `Rng` | edges → `chess-domain`, `GameSession` | Sync, injected | Monotonic time for clocks, wall time only for timestamps. `Rng` seeded from `(game_id, ply)` or the roster seed | Not applicable |
| PGN/FEN I/O | `chess-rules` (wrapping `pgn-reader`/`shakmaty`) → `ImportExportService`, `GameSetupService`, `AnalysisService` | Pure functions | **Parse:** FEN must be a legal position, or it's rejected with a reason. PGN is streamed game by game, and every SAN is validated by replay. **Export (IF-1), checked against the PGN Standard on 2026-10-30 (TBC-6 and TBD-24 closed):** the Seven Tag Roster first, in the standard order (`Event`, `Site`, `Date`, `Round`, `White`, `Black`, `Result`; unknown values `?`, `Round "-"`, `Date` as `YYYY.MM.DD`), then every other tag in **ASCII order of the tag name** (`BlackClass`, `BlackUUID`, `EndStatus`, `FEN`, `GameUUID`, `Mode`, `Rated`, `SetUp`, `Termination`, `TimeControl`, `WhiteClass`, `WhiteUUID`; the UUID tags only in the archive). `SetUp "1"` and `FEN` for a custom start, `TimeControl` (`seconds` or `seconds+increment`) when timed. One tag pair per line, one space between name and value, a blank line after the tags, `\"` and `\\` escaped in values. Movetext in canonical SAN with `1.` and `1...` numbers, a single space between tokens, lines under 80 characters, and a final token equal to `Result`. `Termination` uses the standard's values: `normal` for checkmate, stalemate, resignation and draws; `time forfeit` for a loss on the clock; `abandoned` for voided and aborted games; none for an unfinished game. The writer is one function in `chess-rules` so every front-end gets the same bytes. The custom tags are `Mode`, `Rated`, `WhiteClass`, `BlackClass`, `EndStatus`. `Mode` also names the rating scale. **Games that ended without a result** (watcher-aborted c–c, and voided by a reset, D-30) are exported the same way: `[Result "*"]` with the movetext ending in `*`, `[Termination "abandoned"]` (a standard PGN termination value, so other tools read it as a game that stopped without a result rather than one still in progress), and the custom tag `[EndStatus "aborted"]` or `[EndStatus "voided"]`, which tells the two apart and lets the full-data import restore `GAME.status`. `EndStatus` is written only for those two statuses. **An unfinished game** (`in_progress`, suspended or not) is exported with `[Result "*"]`, the movetext ending in `*`, **no `Termination` tag and no `EndStatus` tag**, and the moves completed so far (RULE-6, IF-1). Exporting changes nothing in the game. Importing that file gives a review-only game (`status = 'imported'`, no result) that can never be resumed. Only the full-data import restores an unfinished game as unfinished (below). Also imported `GAME_TAG`s, saved variations from `VARIATION`, UTF-8, and deterministic output (byte-identical from every front-end). **Import (IF-2):** multi-game files; UTF-8, falling back to Latin-1 per game; comments, NAGs, and variations kept; files over 10 MB rejected (TBD-25, `ESTIMATE`). **Duplicates (CR-4.2):** a game is a duplicate when its tags and moves are identical to a stored game or to an earlier game in the same file, found by a `content_hash` over the canonical tags and SAN movetext. The collision prompt offers **Skip this game (default)** and **Keep both**, plus "apply to all remaining collisions". "Replace" is not offered, because the contents are identical and a replacement could overwrite a rated, played game. A skipped duplicate is not a `ParseError`; the report counts imported, skipped and failed. The importer can tell a played game from an imported one by `GAME.status` and `matchup`, but it never needs to, since nothing is replaced. **FEN (IF-3):** six fields, legality checked. **Full-data export (IF-5, DAT-2):** one zip archive (D-29) holding `manifest.json` (`format_version`, build version, export time), `games.pgn` (every game, with custom tags `GameUUID`, `WhiteUUID`, `BlackUUID` and `[%clk]`/`[%emt]` comments so clocks and think times round-trip), `players.json` (including `reset_at`), `ratings.json` (both tracks, full history, both rated-game counters), `settings.json` (preferences, `APP_SETTING`), and **`games.json`**. `games.json` is keyed by `GameUUID` and holds, per game, the stored fields that neither PGN tags nor PGN comments carry: `status`, `mode`, `rated`, `self_play`, `matchup`, `clock_spec`, the clock checkpoints of an unfinished game, `*_elo_before` and `*_elo_after` on the game's scale, `analysis_used`, the engine parameters each computer side used (a copy of its `COMPUTER_PROFILE` values), `termination` text, `voided_by` as a Player UUID, and `content_hash`; and **the analysis results**: per ply `cp`, `mate`, `best_uci`, `cp_loss`, `flag`, `depth` and `engine_version` (a new `MOVE_EVAL` column, e.g. "Stockfish 19"). Clocks and think times stay in PGN `[%clk]` and `[%emt]` comments, and saved variations stay PGN variations. The archive also carries archived Players and unfinished games. **Import accepts an archive without `games.json`** (older or hand-made): the stored fields then come from the PGN custom tags and defaults, there are no analysis results, an unfinished game takes its clocks from its last `[%clk]` comments, and the background evaluation queues every finished game (F-5.5). **`format_version` stays 1**, because nothing has been released; this is an amendment of D-29, not a new format. Import accepts older format versions and rejects newer ones. **Unfinished games on import (CR-4.3):** an unfinished game in the archive is restored as unfinished, with its slot, unless that would give a Player a second unfinished game in a mode (or a computer a second c–c game). Then it is imported review-only with no result, and the preview says so. A game that is live in a front-end of the importing process is never replaced: the import reports it and keeps the stored game. (Another live process refuses the whole import with `Busy`, §7.4.) FEN export is not in v1 (IF-3). Round-trip: `parse(export(g))` gives the same move list (RULE-6, RULE-7) | `ParseError` with game index, line, and column. One bad game never aborts the rest of a file |
| CLI data commands (D-36) | `chess-cli` → user, scripts | Command line | **Single game (UC-22, RULE-6):** `chess export game <game> [--output <file>]`, where `<game>` is a game UUID or the number shown in `chess games`; the PGN goes to the file, or to standard output without `--output`. **Full data (UC-24, DAT-2):** `chess export data --output <file.zip>`. **PGN import (UC-23, RULE-7):** `chess import pgn <file> [--on-duplicate skip\|keep]`. **Full-data import (UC-25, DAT-2):** `chess import data <file.zip> [--yes]` (preview first, then the confirmation; `--yes` answers it for scripts). The bare `chess export` and `chess import` print the four forms and exit 1. The TUI and plain-mode Import/export screens offer the same four actions | Exit 1 usage, 3 store error; messages on standard error |
| FEN uses | — | — | **Start position** in `GameSetup` (GAME-1, unrated) vs. **Import FEN** on the Import screen, which opens an analysis board and creates no game ("review only") | — |
| `BoardGrid` renderer | `chess-domain` → TUI `BoardWidget`, `PlainRunner` | Pure | Grid of `{glyph, shade, highlight}` per `BoardStyle` and orientation (CLI-2, UI-3) | Not applicable |
| `Board` contract (`BoardProps`/`onMove`) | `ui/board` → GUI screens | React props | §11.1. Screens never touch chessground | Rejected move → `fen` re-set, piece reverts |
| `CoreClient` | `ui/transport` → GUI client state | Promise plus subscription | §5.2. Two implementations with identical behaviour (GUI-1) | §5.6 |

---

## 6. Front-ends

### 6.1 CLI (`chess`): ratatui terminal UI

The CLI is a full-screen terminal UI built with **`ratatui`** (rendering) and **`crossterm`** (terminal I/O, keys, resize). It is **keyboard only** (CLI-1/3, AGREED): moves are typed in algebraic notation, there is no board cursor, and mouse capture is never enabled — which also leaves the terminal's normal text selection and copy working. It links the core in-process like the other front-ends and follows the same rules as the GUI: it renders `GameSnapshot`s, shows only the controls in `allowed_actions`, keeps rewind as local view state, and counts clocks down locally from `ClockSync`.

#### Run modes

| Mode | Selected when | Behaviour |
|---|---|---|
| **TUI** (default) | stdin and stdout are terminals and no scripting flags are given | Alternate screen, raw mode, full-screen layout, live clocks and spinner |
| **Plain** | `--plain`, or stdin/stdout is not a terminal | Line-oriented transcript: board, last move, clocks, and prompt printed after each change. For screen readers (UI-4) and pipes. |
| **Headless** | scripting flags (`--matchup cvc --white … --black … --clock 5+0`) | **Computer–computer, Strict only** (CR-4.12). No UI. Runs to completion, prints the PGN, and exits with a status code. The game is stored and rated like any c–c game (D-36). Drives the c–c scenario tests |

**Selection rule (D-19).** Rules are checked in order:
1. `--headless`, or top-level setup flags with no subcommand (`chess --matchup cvc --white … --black …`, as in CLI-4), selects headless. Headless needs a fully specified setup of two computer Players. **Naming a human Player, or a matchup other than `cvc`, is a usage error: exit 1, nothing on stdout, a message on stderr.** A computer that already has an unfinished c–c game is a setup error (exit 1): a headless run never offers resume (D-27). *(Earlier text let a human side read moves from stdin in headless mode. That is withdrawn: scripted human play uses plain mode, below.)*
2. `--plain`, or stdin/stdout not a TTY, selects plain.
3. Otherwise TUI. `chess play [setup flags]` opens the TUI pre-filled with the setup.

**Scripted human play** (the h–c and h–h scenario tests, PLAT-1) runs in plain mode with moves piped to standard input: `chess play --plain --player <name> [setup flags] < moves.txt`. It uses the same `InputParser` and the same stored games as the interactive modes.

**Data directory (CLI-4, D-36).** The global option `--data-dir <path>` selects another data directory. It **overrides `CHESS_DATA_DIR`**, which overrides the default `~/Library/Application Support/Chess/` (precedence stated again in §7.1). A directory that does not exist is created (mode `0700`) and given the seeded roster, as on a first launch. Scripted tests and calibration use it, so they never touch the user's ratings (RISK-2, R-17).

Exit codes (CLI-5, TBD-21): 0 game reached a result, 1 usage or setup error, 2 engine unavailable, 3 store error, **4 stopped by a signal** (architect-chosen, D-36; accepted by CR-5). A headless run stopped by SIGINT or SIGTERM leaves an **unfinished** game, which the journal already holds (D-14, §7.6), prints no PGN, and exits 4. PGN goes to stdout, and messages go to stderr. **`save` (CLI-4)** is a confirmation: it makes a round trip through the game's session actor (so it sees every earlier command), checks that the last move is durable, and prints "Game saved" with the stored move count. It changes nothing. **`movenow`** (ENG-5) is the in-game command for "move now" (F2 does the same, §4.4).

**Plain mode and screen readers.** Plain mode never prints clock ticks. It prints the clocks with each move and as warnings at fixed thresholds (e.g., 60 s, 10 s), so a screen reader isn't flooded. Opponent moves are printed as they arrive, even while the user is typing.

All three modes share `InputParser` (command words, SAN, and coordinate moves) and the board renderer in `chess-domain`. That renderer produces a terminal-independent `BoardGrid`: per square, a glyph, a light/dark shade, and highlights (last move, check, rewound). The TUI's `BoardWidget` maps that grid to ratatui styles. Plain mode prints it as text lines. One snapshot suite therefore covers both modes (CLI-1/2, UI-3).

#### Play screen layout (80×24 minimum)

```text
┌ Chess ─ Strict · 5+0 · Rated ───────────────────────────────────── F1 help ┐
│    A  B  C  D  E  F  G  H        Black  Viktor Rook (Master 2310)   04:51  │
│ 8  ♜  ♞  ♝  ♛  ♚  ♝  ·  ♜  8     ⠿ thinking…  (F2 move now)                │
│ 7  ♟  ♟  ♟  ♟  ·  ♟  ♟  ♟  7     ──────────────────────────────────────    │
│ 6  ·  ·  ·  ·  ·  ♞  ·  ·  6      1. e4      e5                            │
│ 5  ·  ·  ·  ·  ♘  ·  ·  ·  5      2. Nf3     Nf6                           │
│ 4  ·  ·  ·  ·  ♙  ·  ·  ·  4      3. Nxe5    …                             │
│ 3  ·  ·  ·  ·  ·  ·  ·  ·  3                                               │
│ 2  ♙  ♙  ♙  ♙  ·  ♙  ♙  ♙  2     ──────────────────────────────────────    │
│ 1  ♖  ♘  ♗  ♕  ♔  ♗  ·  ♖  1     White  Brian (1243)                04:38  │
│    A  B  C  D  E  F  G  H                                                  │
├────────────────────────────────────────────────────────────────────────────┤
│ Last: Nxe5 (White) · ←/→ rewind · End live · Tab focus · Esc clear         │
│ > _                                                                        │
└────────────────────────────────────────────────────────────────────────────┘
```

- The board panel sits on the left; the right panel holds the opponent, clocks, engine-thinking indicator, scrollable move list, and the bottom player. `flip` (or Black at the bottom) turns the board and its A–H/1–8 labels (CLI-2).
- The status line shows the last move, errors (e.g., "Illegal: Nf3 — no knight can reach f3"), the "live position has changed" badge while rewound (GAME-3), and context-sensitive key hints built from `allowed_actions`.
- The input line has editing, Up/Down history, and paste.
- **Minimum size (CLI-1, Q55): fixed at 80×24.** Every screen is designed to fit 80×24. If the terminal is smaller:
  1. At startup or on resize, the TUI asks the terminal to resize itself to 80×24 with the xterm window-manipulation sequence `CSI 8 ; 24 ; 80 t`. Terminal.app and iTerm2 are expected to honour it; this is checked in both (R-16, and iTerm2 has a setting that can block it). It records the original size first (`CSI 18 t`) and restores it on exit as part of terminal hygiene.
  2. If no resize event of at least 80×24 arrives within 300 ms (`ESTIMATE`), the screen is replaced by a "please enlarge the terminal to 80×24" message. Inside tmux or screen the sequence is ignored, so the message appears.
  3. The TUI never asks again until the user resizes, so it doesn't fight a user who shrinks the window on purpose.

#### Screens

```mermaid
flowchart LR
    pick["Choose profile<br/>(every launch: registered Players or Register new, Q48)"] --> home["Home menu"]
    pick --> reg["Register<br/>(name ≤ 128 user-perceived characters + self-assessment)"] --> home
    home --> setup["New Game<br/>matchup · mode · players ·<br/>colour · clock (Strict only) · start FEN"]
    setup --> play["Play<br/>(h–c, h–h hot-seat)<br/>rewind · takeback (Training)"]
    setup --> watch["Watch<br/>(c–c: pause · step · delay · abort)"]
    home --> resume["Resume game<br/>(your games, either side,<br/>and every c–c game)"] --> play
    play --> result["Result dialog"]
    watch --> result
    result --> post["Post-game analysis<br/>eval per move · flags · what-if"]
    play -- "analyze (Training only)" --> live["Live analysis / what-if<br/>overlay"]
    live --> play
    home --> stats["Statistics<br/>any Player · tables +<br/>Elo chart in character cells"]
    home --> roster["Computer Players<br/>class · Elo · W/L/D ·<br/>generate 1 to 25"]
    home --> games["Game history<br/>→ replay / analysis / PGN export"]
    home --> data["Import / export<br/>PGN · FEN (review only) · full-data backup"]
    home --> settings["Settings"]
    home --> players["Players<br/>switch active · rename · archive"]
    home --> help["Help (F1, any screen)"]
```

Each screen is a `Screen` implementation with its own state, key handling, and `render`. Forms (setup, settings, registration) use focusable fields where Tab/Shift-Tab moves and Enter confirms. Fields the policy disallows (e.g., Training for h–h, a clock in Training) are not offered (GAME-1). Statistics use ratatui's `Table`, `Chart`, and `Sparkline` widgets (UI-2, UI-5).

**Active Player (PLY-2, CR-4.9).** Picking a profile, or registering a Player at launch, makes that Player the active Player of this front-end session (`SelectProfile`). Registering a Player later (from the Players screen) does not change the active Player. The active Player can be switched at any time when no game is open in this front-end, by returning to the profile chooser. A front-end session has exactly one active Player; two front-ends may differ.

**Players screen (PLY-4, CR-4.10).** Rename and archive are offered for human Players only, and are not offered for computer Players. A human with any unfinished game cannot be archived (`PlayerHasUnfinishedGames` names the games); the active Player cannot archive itself. There is **no un-archive in v1**, and nothing is deleted. An archived Player's name stays taken (PLY-6).

**Empty and partial states (UI-2, ANA-3, CR-4.15).**
- With no games, statistics show zero counts, "—" for every average, the text "no games yet", and each Elo chart shows its baseline point.
- Analysis of a game whose background evaluation has not finished shows "evaluating…" in place of each missing value, raises no blunder flag for it, shows "n of N plies evaluated" from the `evaluated` and `total` counts, and fills in values as `AnalysisProgress` events arrive.
- Games started from a custom position are excluded from W/L/D and from every aggregate, as self-play is, and are listed in the game history marked "custom start" (H-17).

**CLI Elo chart (D-37).** The TUI draws each Elo series with ratatui's `Chart` using a Braille marker, with a table of the latest points beside it. The ASCII style (and any terminal without Unicode) draws the same series on a character grid of `*` and `.` from a small pure renderer, so the snapshot tests cover both. Plain mode prints the table and a one-line sparkline (block glyphs, or the ASCII ramp `_.-^` in the ASCII style) per series. The GUI chart library stays deferred (D-21).

#### Event loop (Elm-style)

```mermaid
flowchart LR
    keys["crossterm EventStream<br/>keys · resize · paste<br/>(no mouse events)"] --> msg
    coreev["Core event subscription<br/>MoveApplied · ClockSync · GameEnded · …"] --> msg
    tick["Tick timer ~10 Hz<br/>clock display · spinner"] --> msg
    sig["signal-hook<br/>SIGTSTP / SIGCONT (Ctrl-Z, fg)"] --> msg
    msg(["Msg"]) --> update["update(state, msg)<br/>→ new state + effects"]
    update --> state["TuiState<br/>screen stack · snapshot · view cursor ·<br/>input buffer · focus · status"]
    update --> effects["Effects<br/>core.execute / core.query"]
    effects --> core["chess-app Core<br/>(in-process)"]
    core --> coreev
    state --> view["render(state, frame)<br/>pure, ratatui widgets<br/>(resize request, then enlarge message below 80×24)"]
    view --> term["Terminal<br/>(alternate screen, restored on quit,<br/>Ctrl-C, panic)"]
```

- **One `tokio::select!` loop** over terminal events, core events, and a tick. There are no blocking input threads, so engine moves, flag falls, and clock updates appear while the user is typing.
- **`update` is separate from `render`.** `update` can be unit-tested with synthetic `Msg`s and no terminal. `render` draws into a ratatui `TestBackend` for snapshot tests at fixed sizes.
- **Key routing (`KeyMap`, CLI-4).** Keys that can never start a move are bindings: ←/→ back/forward, End live, PgUp/PgDn move list, F1 help, **F2 move now** (only while a computer thinks in an h–c game, otherwise shown unavailable), Tab focus, Esc clear/cancel. F1 and F2 are in the help screen generated from `KeyMap` (UI-9), and the `KeyMap` test checks that both are bound. On a Mac laptop F-keys may need Fn, so the command `movenow` always works as well. Every printable character (letters, digits, `-`, `=`) goes to the input line. On Enter, `InputParser` resolves the text to a command word or a move. A unit test checks that no binding consumes a character that can begin SAN or coordinate notation. Because there is no board cursor, the arrow keys are reserved for rewind. The Watch screen has no move input, so it uses plain keys (Space, `s`, `+`/`-`, `a`).
- **Terminal hygiene.** A guard object enters raw mode and the alternate screen and restores both on `Drop`. A panic hook restores the terminal before printing the panic. Ctrl-C arrives as a key in raw mode and always asks for confirmation before quitting (Q52). Every move is already journalled (§7.6), so quitting is always safe. Resign (in hot-seat h–h, after asking which side resigns), c–c abort, archive, "reset player data", applying an import (after its preview) and restoring a backup also always ask, and the core refuses them without `confirmed: true` (§5.5). The Ctrl-C quit dialog is the TUI's own and uses the same dialog widget (UI-12). Cancel and Esc change nothing. Ctrl-Z restores the terminal, suspends via `SIGTSTP`, and redraws on `SIGCONT` (`signal-hook`).
- **Styles.** `BoardStyle` = plain ASCII, Unicode glyphs, ANSI colour, light/dark squares. With `NO_COLOR` or `--no-color`, no colour is emitted. The plain ASCII style also switches panel borders to an ASCII border set.
- **Startup (NFR-1).** The first frame is drawn before the engine starts; engines start lazily.

#### Subcommands

`chess` (profile chooser, then TUI home), `chess play [--player <name>] [setup flags]` (straight into a game), `chess serve [--port <n>] [--no-open] [--config <path>]` (port precedence in §8.2), `chess install-cli [--remove]` (§8.2), `chess stats [player]` / `chess players` (open the TUI screen, or print tables in plain mode), `chess games` (lists stored games with the numbers `export game` accepts), `chess export game <game>` and `chess export data` (UC-22 and UC-24), `chess import pgn <file>` and `chess import data <file.zip>` (UC-23 and UC-25) with the exact forms in §5.7, `chess calibrate` (dev). The global option `--data-dir <path>` applies to every subcommand. `install-cli` and `--help` run before the core is created, so they never open the store or start the engine.

#### `chess serve` lifecycle (PLAT-4, D-36)

- **Configuration.** A missing `config.toml` means defaults. A file that is unreadable, not valid TOML, or whose `server.port` is outside 1 to 65535 stops the command with exit 1 and a message naming the file, the line and the reason (`ConfigInvalid`). There is no silent fallback to a default. Unknown keys are kept and warned about, as before. `--config <path>` and `CHESS_CONFIG` are **architect-added** (tests use them; the requirements do not ask for them) and are kept.
- **Port in use.** Exit 1 with a message naming the port, and the URL of the other `chess serve` when `PROCESS_HEARTBEAT` shows one (§8.2).
- **Browser will not open.** The server keeps running, the URL is printed with a notice that the browser could not be opened, and the exit status is not affected.
- **Stop.** Ctrl-C (SIGINT) or SIGTERM, with no confirmation (the server has no user at the keyboard to ask). The server stops accepting connections, sends `ServerStopping` to every connected page (the page shows "server stopped" and does not retry), suspends each open game as an interruption (D-14: `Suspend` journalled, clocks frozen), and exits 0. Stopping takes at most 5 s (`ESTIMATE`, §4.8); every acknowledged move is already durable, so the limit loses nothing.

### 6.2 GUI (web and desktop, one codebase)

```mermaid
flowchart TB
    subgraph ui["ui/ (TypeScript, React + Vite) — one bundle for web and desktop, all assets local (CSP: self)"]
        screens["Screens<br/>Register · Home · New Game · Play · Watch (c–c) ·<br/>Analysis · Game history · Stats (any Player) ·<br/>Computer Players · Players · Import/Export · Settings · Help"]
        board["Board component (D-4)<br/>chessground (visuals, mouse/touch, aria-hidden)<br/>+ AccessibleOverlay (ARIA grid, keyboard)<br/>+ BoardModel reducer (one selection state)"]
        san["Typed SAN input<br/>+ PromotionPicker"]
        controls["Controls rendered from<br/>allowed_actions (Policy)"]
        store["Client state<br/>game snapshot + local view cursor (rewind)<br/>+ local clock countdown"]
        themes["Themes<br/>≥10 board CSS · ≥5 audited piece sets"]
        client["CoreClient interface"]
        ws["WebSocketCoreClient"]
        tc["TauriCoreClient"]
        gen["protocol/ (generated from Rust by ts-rs)"]
        screens --> board
        screens --> san
        screens --> controls
        screens --> store
        board --> themes
        store --> client
        client --> ws
        client --> tc
        ws -.-> gen
        tc -.-> gen
    end
    ws -- "ws://127.0.0.1" --> server["chess-server"]
    tc -- "Tauri IPC" --> tauri["src-tauri"]
```

- **Framework:** React + TypeScript + Vite (AGREED, D-5): the largest ecosystem, mature testing tools (Testing Library, Playwright, axe), and good accessibility support.
- **Board (AGREED, D-4):** `@lichess-org/chessground` (pinned 10.x, GPL-3.0-or-later) renders the board and handles mouse, touch, and animation, wrapped by our own React component. Our shared layer adds the keyboard model (GUI-2) and screen-reader support (UI-4): a transparent ARIA-grid overlay and a `BoardModel` reducer that owns the one selection state. Full design in §11.1. Themes are chessground CSS classes for boards plus license-audited SVG piece sets; contrast ratios are checked by a unit test.
- **Rewind** is a local view cursor over the snapshot's ply list. While rewound, move input is disabled. The "live position has changed" badge appears when a `MoveApplied` arrives with the cursor behind live (GAME-3).
- **Elo chart:** a small charting dependency or hand-built SVG (D-21, DEFERRED). Either way it has a text or table alternative for screen readers (UI-4).
- **Settings take effect in the client (UI-1, GUI-3).** `PREFERENCES` drive:
  - `BoardProps`: theme, piece set, orientation, coordinates on/off (GUI-3, default on), legal-move hints;
  - app colour theme (UI-10): System (default, follows `prefers-color-scheme` and switches live when macOS appearance changes), Light, Dark, or another bundled theme. It is applied as CSS custom properties on the root element. Every theme, the high-contrast one included, meets WCAG AA (UI-7), and nothing more is required of the high-contrast theme;
  - the default opponent class and colour in New Game.
  Changes are saved with `UpdatePreferences` and apply immediately. The other entry points pick them up the next time they read preferences.
- **Move commit rules (GUI-2, Q52).** All input methods are active during play, and there is no confirm-move setting and no "default input method" setting. A move commits:
  - mouse or touch: when the piece is dropped on a legal square (click-click: on the second click);
  - keyboard overlay: when the destination square is selected with Enter/Space;
  - typed SAN: on Enter.
  A promotion opens the `PromotionPicker` first, and the move commits when a piece is chosen.
- **No sound in v1** (Q52). There are no sound assets, no sound setting, and no terminal bell.
- **Language.** English only in v1 (Q52). UI strings still live in one module per front-end, which keeps the help content (UI-9) and messages (NFR-9) in one place.
- **Layout (Q55).** There is no minimum window size. The layout is fluid (CSS grid with `clamp()` sizing), and the board is a square sized from its container. A `ResizeObserver` triggers chessground's `redrawAll()`, which keeps the `AccessibleOverlay` aligned at every size. Below a narrow breakpoint, panels stack under the board instead of beside it.
- **Profiles (Q48).** Each GUI session starts at the profile chooser: registered Players, or Register new. The chosen Player is session state (`SelectProfile`) and becomes the **active Player**: picking or registering at launch sets it, registering later does not change it, and it can be switched whenever no game is open in this front-end (§6.1). Two tabs or apps can use different profiles. `APP_SETTING.last_active_player` only pre-selects the list.
- **Game feedback (GUI-4).** Last move, check, and legal destinations come from `BoardProps`. The move list comes from the snapshot's plies. Captured pieces are derived in the client from the snapshot (material difference at the viewed ply). The result dialog is shown on `GameEnded`, and the thinking indicator on `EngineThinking`. While a computer thinks in an h–c game the indicator carries a **"Move now" button** (keyboard reachable, ARIA label "Move now", activated by Enter or Space, and bound to F2); it sends `CancelEngine` and is not rendered as available in any other state or matchup (ENG-5, GUI-4).
- **Connection loss (web).** §5.6: banner, inputs disabled, then reconnect and resnapshot.

### 6.3 Desktop (Tauri)

- `src-tauri` creates the `Core` at startup, using the same data-directory resolution as the CLI. It registers one `core` command and forwards events.
- It bundles the Stockfish binary as a Tauri **sidecar**, the `chess` CLI binary beside it (symlinked onto `PATH`, §8.2), and the license notices. There is one bundle, so CLI and desktop are always the same version (D-16).
- Packaging: a universal (arm64 + x86_64) `.app`/`.dmg` (PLAT-1, Q46), **ad-hoc signed** (no Developer ID, no notarization; Apple silicon will not run unsigned code), built with `cargo tauri build --target universal-apple-darwin`. See §8.2.
- Single instance: a second launch focuses the existing window instead of starting another core. Closing the window quits the app, which suspends any live game (RULE-5). All of this is the block `Tauri shell and IPC adapter` (§2.2).

---

## 7. Persistence

### 7.1 Technology

- **SQLite** through `rusqlite` with the `bundled` feature (no system dependency). Pragmas: `journal_mode=WAL`, `synchronous=FULL`, `fullfsync=ON` and `checkpoint_fullfsync=ON` (on macOS a plain `fsync` does not flush the drive's cache, so this makes SQLite use `F_FULLFSYNC`), `foreign_keys=ON`, `busy_timeout=5000`.
- **Durability (NFR-2, RULE-5, Q54: decided).** No completed move is ever lost, whatever happens: app crash, kill, disconnect, OS crash, or power loss. Every move is written to the game's move journal and flushed with `F_FULLFSYNC` before it is acknowledged. The journal is then applied to SQLite, whose own commits are also fully synced (§7.6). After any interruption, the clock comes back at the last recorded value: the last move or 1 s checkpoint, so at most 1 s of clock is lost (TBD-7 proposed: 1 s). TBD-23 (moves lost on power loss) is 0. The flush costs a few ms per move (`ESTIMATE`, measured by the NFR-8 latency test), which is negligible at chess move rates.
- **Location and precedence (D-36):** `~/Library/Application Support/Chess/chess.db`. Precedence: (1) the `--data-dir <path>` option, (2) `CHESS_DATA_DIR`, (3) the default. A directory that does not exist is created with the roster, as on a first launch. Every test and `chess calibrate` isolate state this way.
- **Migrations:** numbered SQL files tracked with `PRAGMA user_version`. Before any migration, the file is copied with `VACUUM INTO chess.db.bak-v<N>`. **If that backup cannot be written the upgrade does not start** (DAT-6, D-35): the store is left unchanged, `BackupFailed{cause}` shows the cause, and the user may retry or exit. A newer front-end never opens an old-version store without the upgrade. Fixture databases for each released schema version are kept in the repo for migration tests (DAT-1).
- **Corruption:** on open, `PRAGMA quick_check`. If it fails, the app explains the problem, offers to restore the latest backup, and never overwrites the file silently (DAT-5). A restore is all-or-nothing and replays **every retained journal**, including those in `journal/done/` for games that finished after the backup was taken; the journal wins on any disagreement, and each such game is finalised again in order of its end time so its Elo change and statistics apply once. Details in §7.6 and D-35.
- **Automatic backups (D-35; stated in DAT-5 by CR-5.1).** Requirements DAT-5 and DAT-6 assume a "most recent backup" exists, but only the pre-upgrade and pre-import copies were defined. The store therefore also writes `backups/chess-<UTC date>.db` with `VACUUM INTO`, followed by `quick_check` of the copy, at start-up when the newest backup is older than 24 h (TBD-33, `ESTIMATE`) and no other process is live. The newest 3 automatic copies are kept, and the pre-upgrade and pre-import copies are never pruned by this rule. Requirements DAT-5 (Amendment 3, CR-5.1) now states the cadence and the retention; a failed automatic backup is logged and never stops start-up (function F-6.15).
- **Missing store (Q51).** When a store is created, a `store.id` file (store uuid, creation time) is written next to it. On open, if `chess.db` is absent:
  - nothing else in the data dir (no `store.id`, backups, or journals) means a genuine first launch. The store is created silently (PLAT-3) and the profile chooser opens on Register.
  - any trace of an earlier store returns `StoreMissing`. The user chooses **start fresh** (`CreateFreshStore`, keeping the traces aside in `missing-<date>/`) or **import from a file** (a full-data export or a backup).

### 7.2 Data model (baseline v1.0)

```mermaid
erDiagram
    PLAYER ||--o| PREFERENCES : has
    PLAYER ||--o| COMPUTER_PROFILE : "if computer"
    PLAYER ||--o{ RATING_HISTORY : "every Player (Q41)"
    PLAYER ||--o{ GAME : "plays white"
    PLAYER ||--o{ GAME : "plays black"
    GAME ||--o{ MOVE : contains
    GAME ||--o{ VARIATION : "saved what-ifs"
    GAME ||--o| ANALYSIS_JOB : "queued after end"
    GAME ||--o| GAME_LEASE : "while live"
    GAME ||--o{ UNFINISHED_SLOT : "occupies until finished"
    PLAYER ||--o{ UNFINISHED_SLOT : "at most one per mode"
    GAME ||--o{ GAME_TAG : "PGN headers"
    MOVE ||--o| MOVE_EVAL : "computed post-game"
    RATING_HISTORY }o--o| GAME : "caused by"

    PLAYER {
        int id PK
        text uuid "stable identity for import merge (Q51)"
        text kind "human | computer"
        text name "UNIQUE case-sensitive across every Player, NFC, trimmed, max 128 grapheme clusters, case kept (PLY-6)"
        text start_category "self-assessed (human) or creation class (computer)"
        int strict_elo "dynamic for every Player (Q41)"
        int training_elo "separate track, same baseline (ELO-2)"
        int strict_rated_games "Strict K counter: 40 for games 1-30, then 32"
        int training_rated_games "Training K counter, counted separately"
        datetime reset_at "humans only: hides older games from this Player views"
        bool archived
        datetime created_at
    }
    COMPUTER_PROFILE {
        int player_id PK, FK
        int roster_seed
        int roster_index
        int skill_level
        int depth_limit
        int movetime_ms
        int uci_elo "nullable"
        real error_rate
        int error_multipv
    }
    PREFERENCES {
        int player_id PK, FK
        text json "versioned settings blob (UI-1)"
    }
    GAME {
        int id PK
        text uuid
        int white_id FK "null for imported PGN"
        int black_id FK "null for imported PGN"
        text matchup "hh | hc | cc"
        text mode "training | strict"
        bool rated
        bool self_play
        text start_fen "null = standard"
        text clock_spec "null | base+inc (Strict only)"
        int white_clock_ms "checkpoint for resume (RULE-5)"
        int black_clock_ms "checkpoint for resume (RULE-5)"
        text status "in_progress | completed | aborted | voided | imported"
        text result "1-0 | 0-1 | 1/2-1/2, null if aborted or voided"
        text termination "voided by reset (D-30) or voided: unreadable (DAT-7), and the normal reasons"
        int voided_by FK "the reset Player, null for voided: unreadable"
        text eco "code of the deepest named position (D-20)"
        text opening_name "name from the ECO table (D-20)"
        datetime started_at
        datetime ended_at
        int white_time_used_ms
        int black_time_used_ms
        int total_active_ms
        bool analysis_used
        int white_elo_before
        int white_elo_after
        int black_elo_before
        int black_elo_after
        text content_hash "hash of canonical tags + moves, set for every game: PGN duplicate check and import dedupe (§5.6, D-34)"
        int journal_seq "last journal record applied (§7.6)"
    }
    MOVE {
        int game_id PK, FK
        int ply PK
        text san
        text uci
        text fen_after
        int think_ms
        int white_clock_ms
        int black_clock_ms
        datetime played_at
    }
    MOVE_EVAL {
        int game_id PK, FK
        int ply PK, FK
        int cp "nullable"
        int mate "nullable"
        text best_uci
        int cp_loss
        text flag "blunder | mistake | inaccuracy | null"
        int depth
        text engine_version "e.g. Stockfish 19 (games.json, ANA-3)"
    }
    RATING_HISTORY {
        int id PK
        int player_id FK
        text track "strict | training"
        int game_id FK "null for an initial or reset point"
        int rating_before
        int rating_after
        int k
        datetime at
    }
    VARIATION {
        int id PK
        int game_id FK
        int from_ply
        text pgn "PGN variation text"
    }
    ANALYSIS_JOB {
        int game_id PK, FK
        text state "queued | running | done | failed"
        text owner "process id"
        datetime heartbeat_at
    }
    GAME_TAG {
        int game_id PK, FK
        text name PK
        text value
    }
    GAME_LEASE {
        int game_id PK, FK
        text owner "process kind + pid"
        datetime heartbeat_at
    }
    UNFINISHED_SLOT {
        int player_id PK, FK
        text mode PK "training | strict"
        int game_id FK
    }
    APP_SETTING {
        text setting_key PK "e.g. last_active_player, roster_seed"
        text value
    }
    PROCESS_HEARTBEAT {
        text owner PK "process kind + pid"
        int build_version
        datetime heartbeat_at
    }
```

Notes:
- **The `MOVE` rows are the source of truth.** The game state is rebuilt by replaying them from `start_fen`. PGN is produced on export, not stored redundantly. Imported PGN headers go in `GAME_TAG`; imported games may have no registered Players, so `white_id`/`black_id` are nullable for them.
- **Imported games** (`status = 'imported'`) have null `matchup`/`mode`, `rated = 0`, null `think_ms`, and no analysis job. Post-game analysis runs on demand. They are never counted in stats or Elo (RULE-7).
- **`APP_SETTING`** holds machine-wide values: the last active Player (PLY-3) and the roster seed (PLY-4). Per-Player settings stay in `PREFERENCES` (UI-1). **`PROCESS_HEARTBEAT`** lists live entry points and their build versions. It is used by D-16 and the exclusive operations in §7.4.
- **`analysis_used`** is set by the gateway the first time it grants `LiveAnalysis` or `WhatIf` for a Training game (DAT-3).
- **Stats are SQL queries/views** filtered with `self_play = 0 AND status = 'completed' AND start_fen IS NULL` for aggregates (custom-start games are excluded as self-play is, UI-2), while history lists include everything, with custom-start games marked "custom start" (UI-2, UI-5). A resignation in a custom-start game is recorded in the game record only. Computer Players' W/L/D use the same queries.
- **Two Elo tracks for every Player (Q41, ELO-1/2).** Each Player, human or computer, has a **Strict Elo** and a **Training Elo**, both starting at the same category baseline. A finished game updates the track of its mode (`GAME.mode`) for both sides, unless it is self-play or started from a custom FEN (`rated = 0`). That includes c–c games between different computers. `GAME.*_elo_before/after` hold the values on that game's track. **Ratings, `k` and the history values are whole integers (CR-7.1):** `EloCalc` computes `Δ = round(K × (S − E))` in `f64`, rounding half away from zero, and stores `R + Δ`, so the history sums exactly. Each track has its own rated-game counter (`strict_rated_games`, `training_rated_games`), which sets that track's K: 40 for its first 30 rated games, then 32 (Q78). Because two processes may finish games against the same computer Player at the same time, finalize re-reads both current ratings inside its `BEGIN IMMEDIATE` transaction (§5.4).
- **Rating history is append-only, one series per track.** The initial row for each track is written when the Player is created. An integration test asserts `elo == initial + Σ deltas` per Player and per track (ELO-1). The displayed category of a computer follows its current Strict Elo (§4.6, Q79).
- **One unfinished game per mode, per Player (GAME-4, D-27; user 2026-10-26).** `StartGame` inserts one `UNFINISHED_SLOT` row, keyed by `(player_id, mode)`, for each slot-holding Player in the game:
  - h–h (Strict): each human, or one row for self-play;
  - h–c (Training or Strict): the human only. A human–computer game never uses or checks the computer's slot, so any Player can always play any computer, even one that has an unfinished game;
  - c–c (Strict): each computer, or one row for self-play.
  The slot limit therefore applies to every human Player, and to a computer only in c–c games. A computer with an unfinished c–c game can't start another c–c game until that one ends or is aborted, but humans can still play it. The primary key makes a second unfinished game impossible across all processes, and the error is `UnfinishedGameExists`. Rows are deleted in the finalize, abort, or reset (void) transaction. `StartGame` is therefore a synchronous DB transaction: it fails with `StoreBusy` if the DB is locked, which is acceptable because no clock is running yet.
- **Ending a game early (GAME-4, RULE-2).** An interruption the player didn't choose (crash, engine or power failure, closed tab) is never an abort. The game stays `in_progress` in the journal and DB, and resumes with no Elo or stats effect. A human ends a game early only by **Resign** (confirmed; h–h asks which side). That is a normal result and counts as a loss for Elo on the game's track and for W/L/D. **Abort** exists only for a watcher ending a c–c game: `status = 'aborted'`, no result, no Elo and no stats for either computer. It stays in history and can be analysed (ANA-4). The only other way a game ends without a result is a **void** by a reset (`status = 'voided'`, D-30, below).
- **Player reset (PLY-7, D-26; user 2026-10-26, amended 2026-10-27).** Humans only; computers can't be reset. `ResetPlayerData{confirmed, void_game_ids}` runs one transaction: it sets `reset_at`, restores default `PREFERENCES`, appends a reset point to `RATING_HISTORY` on each track (`game_id` null, `rating_before` = `rating_after` = the current value), and voids every unfinished game of the Player (below). Both current ratings and both rated-game counters are kept, so the next rated game uses the kept K. Nothing is deleted.
  - **Views of the reset Player** show only what came after `reset_at`: statistics, W/L/D, game history, saved variations, and the rating history and chart. That applies in every front-end and in the "stats for any Player" views (UI-5). The ELO-1 check restarts at the reset point: current = reset point + Σ later deltas.
  - **Opponents' views are unchanged** (Q76): the Player's earlier games stay in every opponent's game history, statistics, W/L/D, and head-to-head, computers included, under the Player's current name.
  - The full-data export still contains every game and `reset_at` (DAT-2), so an import shows the same views.
  - **Unfinished games are voided by the reset (D-30; user 2026-10-27, closes OQ-8).** *Superseded rule (baseline v1.0): a reset was refused with `UnfinishedGameExists` while the Player had an unfinished game.* Now the reset is allowed, and the unfinished game does not count against either player:
    - **Confirmation.** The unconfirmed command returns `ConfirmationRequired{action: reset, voids}`, listing each unfinished game (opponent, mode, moves played). Every front-end's dialog says that these games will be voided: no result, no rating change, not counted for either player. The confirmed command echoes the list as `void_game_ids`. If at commit time the Player has an unfinished game that is not in that list, nothing is written and the confirmation is asked again (§5.5).
    - **Inside the reset transaction**, for each unfinished game of the Player: `GAME.status = 'voided'`, `result` null, `termination = 'voided by reset'`, `voided_by` = the reset Player, `ended_at` = the reset time. No Elo change, no `*_rated_games` change, no W/L/D, and no `RATING_HISTORY` row for any side: the reset Player, a human opponent, or a computer. Every `UNFINISHED_SLOT` row that points to the game is deleted, whichever Player holds it (the opponent human in h–h, both rows of a self-play game), and its `GAME_LEASE` row is deleted. An `ANALYSIS_JOB` is enqueued as for an aborted game, so the opponent can analyse it.
    - **Journal and lock.** Before the transaction, `PlayerService` tries the game's journal `flock`. If it gets the lock (the game is suspended), it first replays unapplied journal records, so every completed move is in `MOVE` (NFR-2). After the commit it appends a `Voided` record with `F_FULLFSYNC`, closes the journal, and moves it to `journal/done/` (the normal lifecycle of a final game, §7.6). The journal is not deleted by the reset. If the game is live in this process, `PlayerService` sends `Void` to its `GameSession` through the `SessionRegistry` after the commit. If it is live in another process, that process's `GameSession` finds out at its next lease heartbeat (its `GAME_LEASE` row is gone, at most 2 s, `ESTIMATE`) or at its next journal apply, whichever comes first. Either way the session cancels any engine search, appends `Voided`, closes its journal, releases its `flock`, emits `GameVoided`, and stops. A move made in that short window is still journalled and applied to `MOVE` (no completed move is lost), but the status stays `voided`. A `GameEnd` in that window is skipped by finalize, because the status is already final (§7.4).
    - **Which record wins.** The journal wins on moves and clocks (§7.6). The void is a store decision made in the reset transaction, so `status = 'voided'` is final and no journal replay changes it.
    - **Visibility (architect's interpretation, D-30, for the user to correct).** The voided game stays in the opponent's game history, marked "voided by reset", with no result, and can be analysed (ANA-4). It is hidden from the reset Player's own views by the `reset_at` rule (Q76), because it started before `reset_at`. It is not a resignation, so the Q60 loss rule does not apply. It is never counted in W/L/D or Elo for anyone: stats aggregates already filter on `status = 'completed'`.

Reset with unfinished games (D-30). Question this diagram answers: how does a reset void a suspended game and a game that is live in another process, without losing a move or touching any rating?

```mermaid
sequenceDiagram
    autonumber
    actor U as User (front-end A)
    participant PS as PlayerService (process A)
    participant DB as Store thread + chess.db
    participant J as Journal of suspended game G1
    participant S2 as GameSession of G2 (process B)
    participant UI2 as Front-end B
    U->>PS: ResetPlayerData{player, confirmed: false}
    PS->>DB: list unfinished games of the Player
    DB-->>PS: G1 (suspended), G2 (live in process B)
    PS-->>U: ConfirmationRequired{reset, voids: G1, G2}
    U->>PS: ResetPlayerData{player, confirmed: true, void_game_ids: G1, G2}
    PS->>J: flock G1 (free), replay unapplied records
    PS->>DB: BEGIN IMMEDIATE
    alt an unfinished game is missing from void_game_ids
        PS->>DB: ROLLBACK
        PS->>J: unlock G1 (it stays suspended and resumable)
        PS-->>U: ConfirmationRequired with the current list
    else every unfinished game was listed
        PS->>DB: reset_at, default preferences, reset point per track
        PS->>DB: G1 and G2 voided, their slots and leases deleted, analysis jobs queued
        PS->>DB: COMMIT (no Elo, no W/L/D, no rating history for any game)
        PS->>J: append Voided + F_FULLFSYNC, close, move to journal/done, unlock
        PS-->>U: reset done, G1 and G2 voided
        Note over DB,S2: At the next lease heartbeat (2 s, ESTIMATE) or journal apply
        S2->>DB: heartbeat G2 lease
        DB-->>S2: lease row gone, status voided
        S2->>S2: cancel search, append Voided, close journal, unlock
        S2-->>UI2: Event GameVoided{G2, reset player}
    end
```
- **Names (PLY-6, CR-4.1).** A name is trimmed of leading and trailing whitespace and then NFC-normalised, and nothing else is folded. It is at most 128 user-perceived characters (extended grapheme clusters, UAX #29), contains no control character (category Cc) and no line or paragraph separator (Zl, Zp), and is non-empty. It is unique **case-sensitively** against **every** Player, human or computer, active or archived ("Anna" and "anna" are different names), and an archived Player's name stays taken. The `UNIQUE` index is on the stored (trimmed, NFC) value. Case is kept. The failed rule is named in the message (`empty`, `too_long`, `character_not_allowed`, `already_used`, and "by an archived Player" when so). Players are identified by `uuid`, never by name.
- **Void as unreadable (DAT-7, D-35).** `VoidUnreadableGame` is the second way a game becomes `voided`: `status = 'voided'`, `result` null, `termination = 'voided: unreadable'`, `voided_by` null, `ended_at` = the void time. No Elo, counter, W/L/D or rating-history change for anyone, and every `UNFINISHED_SLOT` row of the game is deleted in the same transaction, so the slot is freed. The game stays in history marked "voided: unreadable".
- **Average game length (UI-2).** Reported both in moves (full moves = ⌈plies / 2⌉) and in time (`total_active_ms`), from the same filtered queries.
- **Time accounting (DAT-3):** `think_ms` per move and the per-side totals come from the session's `TimeSource` while the game is live. Time while the app is closed is never counted, because a suspended game has no running clock. The session checkpoints both clocks into the journal every 1 s while a clock runs, and on every move, pause, and suspend (§7.6). The checkpoints are applied to `GAME.white_clock_ms`/`black_clock_ms`, so a resumed game restarts from the last recorded remaining time, including the side to move's partial thinking time (RULE-5, Q54). After any crash, disconnect, or reset, at most one checkpoint interval (1 s) is lost.

### 7.3 Multi-process coordination

The CLI, web server, and desktop app can all be open at once against the same file.

- **Writes** are short transactions, and WAL plus `busy_timeout` serialise them safely across processes (concurrent-access integration test for DAT-1).
- **Live games** are owned by exactly one process: the one holding an exclusive `flock` on the game's journal file (§7.6). The OS releases the lock the instant the owner exits or crashes, so another process can resume immediately (DAT-4; TBD-28 = 0 s: no wait after a crash). The `GAME_LEASE` row (owner kind, pid, heartbeat every 2 s) is only for display: opening the same game elsewhere shows "this game is open in the CLI". A hung but still-running owner keeps the lock, and the message tells the user which process to quit.
- **The background analysis queue** uses the same lease idea, so whichever process is running picks up pending jobs, and jobs survive restarts.
- Other screens (stats, player lists) re-query when they are opened. No cross-process push is needed in v1.

### 7.4 Data ownership and transaction boundaries

One writer per entity, and every invariant is enforced inside one SQLite transaction on the store thread (D-13).

| Entity | System of record (writer) | Readers | Transaction boundary and invariant |
|---|---|---|---|
| `PLAYER`, `PREFERENCES` | `PlayerService` | all services, front-ends via queries | **Register:** `PLAYER` + default `PREFERENCES` + an initial `RATING_HISTORY` row per track (Strict and Training), all-or-nothing (PLY-2, ELO-1). `name` UNIQUE, case-sensitive (PLY-6). **Reset (confirmed, humans only):** `reset_at` + default `PREFERENCES` + one reset point per track in `RATING_HISTORY` + the void of each unfinished game of the Player (`GAME.status = 'voided'`, every `UNFINISHED_SLOT` and `GAME_LEASE` row of the game deleted, analysis job queued, no rating or W/L/D effect for any side), one `BEGIN IMMEDIATE` transaction. Ratings and K counters untouched (PLY-7, D-26, D-30). For the void, `PlayerService` is a second writer of `GAME.status` and `UNFINISHED_SLOT` beside `GameSession`. The `BEGIN IMMEDIATE` serialisation and the "skip if already final" rule keep the two from conflicting (§7.2). `PlayerService` writes only initial and reset points to `RATING_HISTORY`; every game delta is written by finalize. **Rename and archive** apply to human Players only (`Denied{computer_player}`). Archive is refused while the Player has any unfinished game (`PlayerHasUnfinishedGames`) and is refused for the session's own active Player (`Denied{active_player}`). There is no un-archive in v1. **Register, rename, archive, preferences:** one transaction each, all-or-nothing (DAT-7) |
| `COMPUTER_PROFILE` | `PlayerService` (via `RosterGen`) | `EngineGateway`, stats | Roster batch of N Players (1 to 25) inserted in one transaction, together with each computer Player's two baseline rating rows. Computer Players are never renamed, archived, deleted or reset. Profile rows are never updated after insert: engine parameters are fixed (PLY-4, Q41) |
| `GAME` (header), `MOVE`, `UNFINISHED_SLOT` | `GameSession` (live), `GameSetupService` (start), `ImportExportService` (imported) | stats, analysis, export | **Start:** `GAME` + `Start` journal record + one `UNFINISHED_SLOT` per slot-holding Player (every human; computers only in c–c, D-27), in one synchronous transaction (GAME-4). **Move:** journal record + `F_FULLFSYNC` first (the durability point). Then `MOVE` row + `GAME` clock + `journal_seq`, one transaction per applied batch (NFR-2, §7.6). **Takeback:** a journal record, then delete the trailing `MOVE` rows in one transaction |
| `GAME` result, `RATING_HISTORY`, `PLAYER.*_elo` / `*_rated_games`, `ANALYSIS_JOB` | `GameSession` finalize | stats | **Finalize** (completed, including resignation): `GameEnd` journal record first. Then one `BEGIN IMMEDIATE` transaction: re-read both Players' ratings on the game's track (Training or Strict), compute result + times + Elo deltas for both sides on that track + history rows, delete the `UNFINISHED_SLOT` rows, enqueue the analysis job (ELO-1/2, §5.4). **c–c abort:** status, slot deletion, and job, with no ratings. Both are idempotent on replay: skipped if the status is already final (`completed`, `aborted`, or `voided`) |
| `MOVE_EVAL`, `ANALYSIS_JOB` state | `AnalysisWorker` | `AnalysisService`, stats | One upsert per ply. A partial run resumes from the last ply |
| `VARIATION` | `AnalysisService` | export | Per save or delete |
| `GAME_TAG`, imported `GAME`/`MOVE` | `ImportExportService` | export, history | One transaction per imported game |
| Journal files (`journal/<game_uuid>.jnl`) | the `GameSession` holding the file's `flock` | recovery in any process | Append-only, one writer. Truncated only after its records are applied and the game is final (§7.6) |
| `GAME_LEASE`, `PROCESS_HEARTBEAT` | `SessionRegistry`, `Core` startup | all processes | Display and liveness only. Compare-and-set on `heartbeat_at` |
| Whole database (`ImportData`, `RestoreBackup`, migrations) | `ImportExportService` / `chess-store` | — | **Exclusive:** refused with `Busy` if `PROCESS_HEARTBEAT` shows another live process. Pending journals are replayed first. Runs under `BEGIN EXCLUSIVE` after a `VACUUM INTO` backup. Data import merges (D-17, see below) |

**Data import merges (D-17, decided by Q51).**
1. `PreviewDataImport` matches Players by `uuid` (computers by `(roster_seed, roster_index)`) and games by `uuid`. A different Player with an identical name is also a conflict, because names are unique. It returns `ImportConflicts`.
2. For each conflict the user picks **Replace**: the imported record, with its rating history, replaces the local one. Or **Keep both**: the imported record gets a new `uuid` and a suffixed name such as "Brian (imported 2)", so it stays a distinct identity. Non-conflicting records are added.
3. `ApplyDataImport{plan, confirmed}` is confirmed after the preview (UI-12) and writes everything in one transaction, then re-checks the Elo invariant per Player.
4. If any step fails, nothing is written and the backup is untouched.
5. **Unfinished games** in the archive are restored as unfinished (with their `UNFINISHED_SLOT` rows and the clocks from `games.json`, or the last `[%clk]`) unless that would give a Player a second unfinished game in a mode, or a computer a second c–c game. In that case the game is imported **review-only with no result**, and the preview lists it. A game that is live in a front-end of the importing process is never replaced: it is reported and the stored game is kept.
6. **PGN import** (`ImportPgn`) is not a merge: duplicates raise the Skip (default) or Keep both prompt of D-34, and nothing is ever replaced.

Multi-game PGN files are supported by `ImportPgn` (IF-2).

A suspended game is `in_progress` with no journal lock held, so the stored status needs no separate `suspended` value.

Cross-process consistency is eventual only for screens that are open in another process: they re-query when opened (§7.3). Live game state is never shared, because the lease gives each live game one owner.

### 7.5 One version everywhere (D-16; one-way for data, accepted by the user)

The CLI and the desktop app must never be different versions. They ship and update together as one bundle (§8.2), so v1 has **no** machinery for mixed versions working together. It only detects a mismatch and refuses, with a message.
- **One build version** is compiled into the bundle and covers `chess`, `Chess.app`, the GUI bundle, the journal format, and the schema version.
- **Against the database.** On open, a `user_version` newer than the build's schema returns `SchemaTooNew`, writes nothing, and says "this data was written by a newer Chess. Update the app." An older `user_version` is a normal upgrade: the build migrates after a `VACUUM INTO` backup (DAT-6), but only when no other process is live. Before migrating, it replays all pending journals.
- **Against running processes.** Every process records its build version in `PROCESS_HEARTBEAT`. If a live process of a different version is found, which can only happen in the middle of an upgrade, startup is refused with "Chess version X is still running (CLI / web server / desktop). Quit it first."
- **Against a running server or a stale tab.** A second `chess serve` of another version is refused the same way. A browser tab from another build gets `ProtocolMismatch` and reloads (§5.6).
- **Journals.** The header's `journal_version` must equal the build's. A different value is refused with the same message.
- **One-way for data (accepted by the user, 2026-10-26).** Once a newer build has migrated `chess.db`, no older build can open it. The only way back is the pre-migration backup `chess.db.bak-v<N>`, with pending journals replayed on top, or a full-data export and import (§11.3).
- Tests: an older build opens a newer DB (`SchemaTooNew`); a heartbeat from another version is refused; migration replays journals first (DAT-1).

### 7.6 Move journal: the local cache that always wins (Q54, D-23)

The user's rule (Q54) has three parts:
- every move is cached locally and written to the database when the database is available;
- after any disconnect, hard reset, close, or crash, the clock is restored to the last recorded clock time;
- if the cache and the database disagree, the local cache wins.

**Files.** One append-only file per live game, `<data dir>/journal/<game_uuid>.jnl`. It starts with a header (`journal_version`, `game_uuid`, `store.id`). Each record is `{seq, kind, payload, crc32}`, length-prefixed. The kinds are:
- `Start{setup}`, `Move{ply, uci, san, think_ms, clocks_after}`, `Takeback{to_ply}`;
- `Clock{white_ms, black_ms, running_side}`: every 1 s while a clock runs, and on pause and suspend;
- `DrawOffer`/`DrawDeclined`, `Pause`/`Resume`, `Suspend`, `GameEnd{result, reason}` (including resignation and its side), `Abort` (c–c only), `Voided{reason, reset_player_id?}` (written after the reset transaction commits, D-30, §7.2, or after `VoidUnreadableGame`, DAT-7). `DrawOffer` is followed by a `Clock` record with both clocks stopped (RULE-2).

**Write path.**
1. The `GameSession` holds an exclusive `flock` on its journal for as long as it owns the game. That lock *is* the lease (§7.3).
2. For each state change it appends the record and calls `fcntl(F_FULLFSYNC)`. Only then is the event broadcast, the move acknowledged, and the clock switched.
3. The record is passed to the store thread. In one transaction, the store thread writes the corresponding rows and `GAME.journal_seq = seq`.
4. If the DB is busy or locked (another process's write, an import, a migration) or unavailable, the store thread retries with backoff while play continues. The backlog is reported with `StoreBacklog`.

**Recovery (resume, takeover, startup).** Any process that acquires a game's `flock` first replays the journal:
- records with `seq > GAME.journal_seq` are applied in order;
- if a DB row differs from the journal for the same ply, for example after `RestoreBackup` brought back an older DB, the DB rows from that ply on are replaced by the journal's (**the cache wins**);
- the clock is restored from the latest `Move` or `Clock` record;
- a torn final record (bad CRC or short length) is dropped **silently**. It was never acknowledged, so no completed move is lost, and the user is told nothing;
- **a damaged record in the middle** (bad CRC with intact records after it, or an unreadable header) is not a torn tail. The journal is read up to the first damaged record, and the number of moves in that **intact run** is compared with the moves held by the store (`MOVE` rows, which are valid by replay). The game resumes from the **longer** of the two runs when they agree on their common plies. If they disagree at some ply and the journal is intact up to there, the journal wins (the cache wins, Q54). The core returns `JournalUnreadable{game_id, recovered_moves}` and the client tells the user how many moves were recovered. The damaged journal file is kept aside as `journal/damaged/` and never deleted. Clocks come from the last intact `Move` or `Clock` record, or from the `GAME` checkpoint when that is later;
- **if neither the journal nor the store holds a readable game** (no intact run of even the start record, and no `MOVE` rows to replay from `start_fen`), the core offers `VoidUnreadableGame`. Accepting it voids the game with termination "voided: unreadable" and frees the slot (§7.2, D-35);
- if `GAME.status` is already `voided` but the journal has no `Voided` record (a crash between the reset commit and the journal write), the moves are applied, `Voided` is appended, and the journal is closed. The game is never resumed (D-30).

**Lifecycle.** After `GameEnd`/`Abort` and a successful finalize, or after `Voided` (whose store transaction is the reset itself, or the unreadable-game void), the journal is renamed to `journal/done/`. A `done/` journal is deleted only after **an automatic or manual backup written after the game's end time has passed `quick_check`** (§7.1, D-35). Because of the automatic backup cadence that happens within a day or so of the next start-up; without it `done/` would grow until the next migration or import.

**Restore (DAT-5, CR-4.16, D-35).** `RestoreBackup{confirmed}` is all-or-nothing and works on a copy:
1. Acquire exclusivity (`Busy` if another process is live, §7.4).
2. Copy the chosen backup to `chess.db.restoring`, and open it.
3. Replay **every retained journal** over the copy: those for unfinished games, and those in `journal/done/` for games that finished after the backup. A journal is matched to its game by `game_uuid`. The journal wins on any disagreement (rule above). A game whose Players both exist in the copy is applied; a game that is already final in the copy with the same `journal_seq` is skipped, so the replay is idempotent.
4. **Finalise again, in order of end time.** The games that finished after the backup are finalised one by one in ascending `ended_at`, with the rated flag, track and K counters recomputed from the copy's current ratings. Each game's Elo change and statistics are thus applied once, in the order they originally happened.
5. `fsync`, then rename the damaged `chess.db` aside (never deleted) and rename the copy into place. A failure at any step leaves the original untouched.
6. The result message states the games and moves recovered from journals.

*Limit (R-19, OQ-12).* A journal records only what happens inside a game. A game that involves a Player created after the backup, or an action that is not in a journal (register, rename, reset, import), cannot be reconstructed. Such a game's journal is kept in `journal/unrestored/` and the message names it. v1 offers no automatic recovery of it (a re-created Player has a new UUID, so the journal cannot be matched to it). The limit is stated in requirements DAT-5 (CR-5.1), which accepts it and does not require registration to be journalled. Startup scans for journals with no live lock and replays them, which recovers games interrupted by an OS crash or power loss.

```mermaid
sequenceDiagram
    autonumber
    participant S as GameSession (new owner)
    participant J as Journal file (one per game)
    participant ST as Store thread
    participant DB as chess.db
    S->>J: flock(EXCLUSIVE) — free, because the previous owner crashed
    S->>J: read records, verify CRCs, drop torn tail
    S->>ST: replay(records with seq > journal_seq)
    ST->>DB: BEGIN IMMEDIATE
    alt DB rows agree with the journal
        ST->>DB: insert missing MOVE rows, clocks, journal_seq
    else DB rows differ (e.g., older backup restored)
        ST->>DB: replace rows from the first differing ply (journal wins)
    end
    ST->>DB: COMMIT (fullfsync)
    ST-->>S: applied up to seq N
    S->>S: rebuild Game from moves, set clocks from last Move/Clock record
    S-->>S: ready: ResumeGame continues
```

**Why a journal instead of only SQLite with `synchronous=FULL`?** FULL alone would also make each commit durable. But a move would then wait on the shared database, which another process may lock (busy timeout, import, migration), and Q54 explicitly asks for a local cache that survives the database being unavailable and that wins on disagreement. The journal costs one extra `F_FULLFSYNC` per move. Alternative: keep only SQLite with FULL, and make moves fail with `StoreBusy` when the DB is locked. That is simpler, but it doesn't meet Q54's "written when available".

**Browser and desktop.** The cache lives in the core process, on the Mac's disk. The browser holds no authoritative state. After a WebSocket drop, the page reloads the snapshot (§5.6). A dropped tab suspends the game at once and freezes its clocks (D-14).

---

## 8. Cross-cutting concerns

| Concern | Approach | Requirements |
|---|---|---|
| Error handling | Rust: `thiserror` per crate, mapped at the protocol boundary to `{code, message, details}` (catalogue in §5.5). Front-ends show `message` and branch on `code`. State changes only after the journal flush (§4.8, §7.6), so every error leaves the game as it was and resumable. | CLI-3, ENG-1, RULE-5, NFR-2 |
| Logging | `tracing` to `<data dir>/logs/chess.log`, rotating at **10 MB with 3 files kept** (TBD-32, `ESTIMATE`; a size-triggered writer is needed because a time-based appender alone does not meet NFR-10). A correlation id per request (`Internal{id}`). It holds identifiers (game and Player ids, error codes, durations) and **no Player names and no move lists**. Never uploaded; off-device telemetry: none. | NFR-10, NFR-4, NFR-9 |
| Security | Data dir created `0700` and `chess.db` `0600`. Web: loopback bind, Host/Origin allow-list, CSP, no CORS (R-5, D-22). The core validates every protocol message and never trusts client-computed state (principle 1). One person per Mac account is confirmed (ASM-1, CONFIRMED), so the loopback server has no access token (D-22). **v1 runs only locally on the user's Mac and has no security-criticality requirement beyond security best practice and avoiding known CVEs (CR-7.7):** `cargo-deny advisories` (or `cargo-audit`) and `npm audit` run in CI and on each release, and a known vulnerability in a used dependency blocks the release unless the Decision Log records why it is not reachable. | NFR-4, NFR-7, PLAT-3 |
| Offline | No run-time Internet access anywhere; everything bundled; CSP enforces it. Details in §8.1. | PLAT-3 |
| Privacy / network | The web server binds to loopback only. A test runs a full session under a network-capture harness and asserts zero outbound connections. | NFR-4 |
| Determinism | `TimeSource` and `Rng` are injected everywhere. Engine tests use `Threads=1`, a fixed hash size, and fixed depth. | test-strategy |
| Licensing | Project licensed **GPL-3.0-or-later** (needed for distributed builds that bundle Stockfish; compatible with `shakmaty`). `cargo-deny` and an npm license checker in CI. Stockfish `COPYING` + source offer shipped in every artifact. | ENG-1, NFR-6 |
| Performance | Engines start lazily. SQLite opens in a few ms. The GUI is a static bundle, embedded in the server binary with `rust-embed`. Benchmarks are run in CI for startup and engine latency. | NFR-1, ENG-3 |
| Accessibility | GUI: ARIA grid board, live region announcing moves, focus ring, axe in E2E. CLI: the full-screen TUI is not reliably screen-reader friendly, so plain mode (`--plain`, automatic off-TTY) is the accessible path; `NO_COLOR` honoured in both. Details in §8.3. | UI-4, GUI-2 |
| Packaging and install | One build version for every artifact, `chess install-cli` for the command, and an ad-hoc signed `.app`. Details in §8.2. | PLAT-1/2/6, NFR-5 |
| Coding standard and static checks | Ordinary criticality: not safety-related and not regulated (user, 2026-10-28). SEI CERT is the coding standard, with the standard compiler and linter checks of Rust and TypeScript and every warning treated as an error. CERT has no Rust or TypeScript rule set, so CERT *informs* best practice there: reviews apply the intent of the matching CERT rules and report findings, with no conformance claim (CR-7.5). The tools, the mapping, the scope and the approvers are in [engineering/SWE-001-engagement-profile.md](engineering/SWE-001-engagement-profile.md) | none: a project constraint (§10.2) |

---

### 8.1 Offline operation (PLAT-3)

All three entry points must be fully usable with no Internet connection, from first launch. The architecture has no component that needs the network at run time. The table below lists every run-time dependency and how it is satisfied locally.

| Run-time dependency | How it is satisfied offline |
|---|---|
| Chess rules, game logic, Elo, clocks | Compiled into the Rust binaries (`chess`, `Chess.app`). |
| Engine (Stockfish) | Native universal binary bundled inside `Chess.app`, next to the bundled `chess` CLI (D-9, §8.2). Official Stockfish builds embed their NNUE network file, so no network file is downloaded. The pinned build is verified to start with networking disabled. |
| Data store | Local SQLite file in `~/Library/Application Support/Chess/`. Created on first launch; roster generated from a fixed seed. No account or server. |
| GUI code, styles, fonts | Vite builds a self-contained bundle: all JS/CSS, fonts (self-hosted, no Google Fonts), board themes, and SVG piece sets. Embedded into the `chess` binary (`rust-embed`) for the web, and bundled into `Chess.app` for desktop. No CDN references. |
| Web page transport | `chess serve` serves the bundle and the WebSocket on `127.0.0.1`. Works with Wi-Fi off, because loopback never leaves the machine. |
| Desktop transport | Tauri IPC in-process (D-1); no socket at all. |
| ECO opening table (DAT-3) | Bundled data file compiled into `chess-domain` (D-20): the five Lichess `chess-openings` files (`a.tsv` to `e.tsv`, CC0-1.0) copied to `third_party/eco/`. Never downloaded at run time or at build time. Its license is checked under NFR-6 |
| Help / documentation | Rendered from files shipped with the app (TUI help screen, GUI help page). External links (e.g., Stockfish source offer) are shown as text and never fetched automatically. |
| Updates, telemetry, crash reporting, license checks | None. The Tauri updater plugin is not included. |
| macOS launch checks | The desktop app is a locally built, **ad-hoc signed** build (PLAT-1; "unsigned" in the requirements means no Developer ID and no notarization). Built on the same Mac, it carries no quarantine attribute (TBC-10, unverified), so Gatekeeper's online check doesn't apply. A copy that was downloaded or copied from another Mac can be blocked; the documented first-open steps handle that, and the Game adds no workaround (PLAT-6). |

**Enforcement, so offline can't silently regress:**
- **Content-Security-Policy** `default-src 'self'; connect-src 'self' ws://127.0.0.1:*` (web) and the equivalent Tauri `csp` config (desktop). Any accidental remote script, font, or image fails to load, and the E2E suite catches it.
- **Bundle scan in CI:** fails if the built GUI contains `http(s)://` resource references outside an allow-list (license texts, documentation links shown as text).
- **Offline E2E:** the PLAT-3 scenario (clean profile → register → timed game vs. a computer → post-game analysis → stats → export) runs for desktop, web (three browsers), and CLI with non-loopback networking blocked, under the NFR-4 network-capture harness.

**Build vs. run:** building from source needs the Internet once (crates, npm packages, the pinned Stockfish download in D-9). Running never does. If fully offline builds are wanted later, `cargo vendor`, an npm offline mirror, and a cached Stockfish archive cover that without architectural change.

### 8.2 Deployment and packaging (D-18; one bundle per D-16)

| Artifact | Contents | Installed to | Engine lookup |
|---|---|---|---|
| `Chess.app`, the only artifact | Tauri app, GUI bundle, the `chess` CLI binary (web server included) in `Contents/MacOS/`, one universal pinned `stockfish` next to it, `LICENSES/`, Stockfish source offer | `.dmg` → `/Applications`, or any folder the user chooses (ad-hoc signed, universal) | `<exe dir>/stockfish`, in the same bundle for both entry points. `CHESS_STOCKFISH` overrides it for dev and tests only. **Never `PATH`**, so the pinned version (D-9) is always the one used |
| `chess` on `PATH` | A symlink, not a copy, named `chess`, pointing at the bundle's `Contents/MacOS/chess` | Created by `chess install-cli` (PLAT-6, below) in `/usr/local/bin` if it exists and is writable, otherwise `~/.local/bin` | — |
| Data | `chess.db`, journals, backups, logs, `config.toml` | `~/Library/Application Support/Chess/` (or `CHESS_DATA_DIR`), shared by all entry points | — |

- **`chess install-cli` and `--remove` (PLAT-6, D-18 amended, CR-4.5).** A subcommand of the bundled `chess` binary, so no separate script exists (the earlier `packaging/install-cli.sh` and the app menu item are dropped: one way, one test). It runs before the core is created and never touches the data directory.
  - It resolves its own executable path (following symlinks), and creates a symlink `chess` to it in `/usr/local/bin` when that folder exists and is writable; otherwise in `~/.local/bin`, created if absent. It prints the link's path and, if that folder is not on `PATH`, the exact line to add to the shell profile.
  - Run again, it replaces a link that it created earlier (it recognises its own link by a target inside a `Chess.app/Contents/MacOS/chess` path). It **refuses to replace any other file or link** with that name.
  - On any failure it says why, changes nothing, and tells the user the CLI still runs by its full path (NFR-9).
  - `--remove` removes only a link it created, in either folder. It never deletes the bundle or the data.
  - **Update** = quit every front-end and `chess serve`, replace `Chess.app`; if the bundle path is unchanged the link keeps working, otherwise run `chess install-cli` again. The first launch of the new version upgrades the store after a backup (DAT-6, D-16). **Uninstall** = `chess install-cli --remove`, then delete `Chess.app`; the data directory stays (the README says where it is, that deleting it removes every Player, game and rating, and to export first).
  - The facts behind this (that `/usr/local/bin` exists and is writable on a stock macOS 26 Mac, how a blocked app is opened) are from general knowledge and **not verified**: ASM-5 (requirements TBC-10), closed by the M walkthrough at step 3. The design does not depend on them, because the fallback folder and the failure message cover the unwanted cases.
- **Signing and minimum system.** The bundle's `Info.plist` declares `LSMinimumSystemVersion` 26.0 (Tauri `bundle.macOS.minimumSystemVersion`), so macOS itself refuses earlier releases and the Game adds no check (PLAT-6 (e)). The build **ad-hoc signs** `Chess.app` and each executable inside it (`chess`, `stockfish`) with identity `-`, inside-out, **after** `lipo` has joined the slices (joining invalidates earlier signatures). A CI test runs `codesign --verify --deep --strict` on the bundle.
- **One bundle, one version (D-16).** Replacing `Chess.app` updates the CLI, web server, desktop app, and engine together, because the symlink always points into the current bundle. A separately copied `chess` binary is not a supported install.
- **Platform (PLAT-2, Q46, Q47, user answer 2026-10-26).**
  - **Target:** macOS Tahoe **and every later release**, on any Apple silicon Mac (M1 and newer), and on Intel Macs for as long as Apple's macOS supports them (R-13). The minimum deployment target stays macOS 26.0 (Tahoe): `MACOSX_DEPLOYMENT_TARGET=26.0` for `chess`, and Tauri `bundle.macOS.minimumSystemVersion = "26.0"` (which writes `LSMinimumSystemVersion`) for `Chess.app`. There is no upper bound.
  - **What "and later" implies:**
    - builds use the current Xcode SDK;
    - each new macOS release is tested before support is claimed: smoke test, engine integration, offline E2E, and the desktop E2E against the new WKWebView, starting on the developer betas;
    - pinned dependencies with OS contact (Tauri, WKWebView behaviour, crossterm/terminal behaviour, Stockfish builds) are re-checked and bumped when a release breaks them;
    - deprecated APIs are tracked in CI warnings;
    - the platform matrix (§9) gains a column per supported macOS release.
  - **Build:** `chess` is built for `aarch64-apple-darwin` and `x86_64-apple-darwin` and joined with `lipo`. The arm64 slice covers M1 and newer. `Chess.app` uses the universal Tauri target.
  - **Stockfish (revised 2026-10-27): Stockfish 19, tag `sf_19`** (D-9). Stockfish 19 ships "universal binaries" that detect the CPU's features at run time, so there is no longer a choice between AVX2, AVX-512, and other variants (release notes, checked 2026-10-27). The build script `third_party/stockfish/fetch.sh` follows this plan:
    1. **Preferred path: the official `stockfish-macos-universal.tar.gz` asset** of the `sf_19` release. The script downloads it once, checks it against the SHA-256 recorded in the script (written when the pin was made), extracts it, and runs `lipo -info` on the binary. It accepts the asset **only if both `arm64` and `x86_64` are listed**. Then it runs the binary once with `uci` and records the `UCI_Elo` range it reports (§4.6). That first run closes ASM-3. With run-time CPU dispatch, the single x86_64 slice covers every Intel Mac that Tahoe supports, with no AVX2 build and no run-time file choice. This is confirmed by the engine integration tests on the Intel Mac of the platform matrix (§9).
    2. **Fallback, used only if `lipo -info` does not show both architectures:** the previous plan. Build Stockfish 19 from the checksummed `sf_19` source archive for `arm64` and for `x86_64`, and join the two with `lipo` into one universal `stockfish`. If the source build offers the same run-time CPU dispatch as the release binaries (checked in its build documentation when the fallback is needed), one x86_64 build is enough. If not, the old conditional plan applies: an AVX2 build checked against Apple's Tahoe-compatible Intel models, and, if one variant can't cover them all, two x86 files chosen at run time from `sysctl hw.optional.avx2_0`.
    - **CI test:** a packaging test runs `lipo -info` on the bundled `Chess.app/Contents/MacOS/stockfish` and fails unless it lists both `arm64` and `x86_64`. It also checks the engine's `id name` line names Stockfish 19, so a different binary can't slip in.
    - **License:** the Stockfish 19 release notes say it is "licensed under the GPL license". The build script reads the exact GPL version from the shipped `COPYING` file (requirements TBC-9) and the license audit records it. `COPYING` and the source offer ship in every artifact (§8, NFR-6).
    - **Network:** Stockfish 19 uses the new SFNNv16 network architecture. The existing offline test (§8.1) checks that the bundled binary starts and searches with networking disabled, which proves its network file is embedded.
    - For the sidecar, follow Tauri's naming rule for universal targets (OQ-11, to verify against the pinned Tauri version).
  - **Performance baseline:** M4, 16 GB, Tahoe (NFR-1, ENG-3, NFR-8, calibration, the §4.8 timeouts). M1–M3 and Intel Macs are supported but slower (R-14).
  - **Browsers:** Safari, Chrome, Firefox, and Edge releases from the last 3 years. Vite's JS and CSS build target is derived from a browserslist query for releases of those four browsers from the last 3 years. The exact query is checked with `npx browserslist` when the build is set up. Playwright tests the current engines; the oldest supported version of each is smoke-tested manually per release.
  - **Terminals:** Terminal.app and iTerm2.
- **Install docs (NFR-5)** live in the README: build prerequisites (needs the Internet once, §8.1), `make release`, opening the `.dmg` or copying the app, `chess install-cli`, `chess serve`, update (replace the app), uninstall (`chess install-cli --remove`, then delete the app), the first-open steps for a blocked copy, and the data directory with its deletion warning (PLAT-6, NFR-5). The text lives in `Release checklist and install docs` (§2.1).
- **Port and config file (PLAT-4, Q55, TBD-3).** The port setting applies only to `chess serve` and the browser URL. The CLI play modes run the core in-process and the desktop app uses IPC (D-1), so neither uses a port.
  - Precedence: (1) the `--port <n>` flag, then (2) `server.port` in `config.toml`, then (3) the default, 8765 (TBD-3).
  - The config file is `~/Library/Application Support/Chess/config.toml`. `--config <path>` or `CHESS_CONFIG` points elsewhere, which tests use. These two overrides are **architect-added**: the requirements do not ask for them. It is created only when the user saves a setting, and unknown keys are kept and warned about. A missing file means defaults; an unreadable or invalid file, or a port outside 1 to 65535, stops `chess serve` with exit 1 and a message naming file, line and reason (PLAT-4, §6.1).
  - The page connects back to the origin it was served from. A fixed port keeps that origin stable for the Host/Origin allow-list (NFR-7).
  - If the port is taken, `chess serve` exits non-zero with a message naming the port. If the holder is another `chess serve` (found in `PROCESS_HEARTBEAT`), the message prints that server's URL. This matches PLAT-4.
- **No auto-update** (§8.1). Upgrading means replacing `Chess.app`.

### 8.3 Accessibility details (UI-4, GUI-2)

| Area | Mechanism | Verification hook |
|---|---|---|
| Keyboard | Every screen is reachable and operable by keyboard (Tab order, Enter/Space, Esc closes dialogs). The board uses `AccessibleOverlay` (§11.1), and the SAN field is always available | Playwright keyboard-only scenario per screen |
| Screen reader (GUI) | Squares labelled "e4, white knight". The live region announces own move confirmed, opponent move, check, draw offer, game end, and clock warnings at fixed thresholds (no per-second announcements). The Elo chart has a table alternative. Dialogs move focus in and return it on close | axe, scripted VoiceOver walk-through (M) |
| Screen reader (CLI) | Plain mode, as in §6.1 (no tick output) | PTY transcript tests |
| Contrast | High-contrast board theme and piece set. Unit test checks theme contrast ratios against WCAG 2.2 AA (4.5:1 text, 3:1 non-text; UI-7) | U: contrast test per theme |
| Motion | `prefers-reduced-motion` turns off chessground animation (`animation.enabled = false`) | E: emulated media query |
| App themes | Light, Dark, and System (follows `prefers-color-scheme`) plus other bundled themes (Q52). All are checked against UI-7 | Contrast unit test per theme; screenshots in both schemes |
| Confirmations | Resign (h–h: which side), c–c abort, archive, reset player data, applying an import (after its preview), restoring a backup, and TUI quit (Ctrl-C) always ask (Q52, UI-12). Cancel and Esc change nothing. The reset dialog lists each unfinished game it will void and says it counts for neither player (D-30). Dialogs are focus-trapped and announced, and Esc cancels | E: keyboard-only confirm and cancel |
| Colour independence | Last move, check, and selection use outline or pattern as well as colour. TUI highlights use reverse video or markers when `NO_COLOR` is set | Screenshot and snapshot tests |

### 8.4 Room for network play later (Q43, not built in v1)

v1 stays fully offline (PLAT-3, NFR-4): there is no listener beyond loopback and no outbound connection. Network play is a planned future enhancement, and these existing choices keep it cheap to add without building any of it now:
- **The core is already authoritative and remote-ready.** All rules, clocks, and the Policy run in the core (principle 1), and front-ends speak one serializable protocol (§5). A remote opponent would be one more front-end adapter that sends commands (`MakeMove{ply}` with the optimistic check) and receives events.
- **Identity.** Players and games carry `uuid`s (§7.2, Q51), so records can later be matched across machines.
- **A new seat type.** A "remote human" side can be added to `GameSetup` and the Policy table as a new matchup, without changing the existing rows.
- **The transport seam** is the `chess-server` WebSocket adapter (§5.2). The future work is TLS, authentication, NAT traversal or a relay, and a server-side clock authority between two cores.
- **What would have to change, and is deliberately absent now:** binding beyond loopback, CSP `connect-src` beyond self, and any outbound traffic. These need a requirements change (PLAT-3, NFR-4) and a new decision record. The offline enforcement in §8.1 would then become per-feature, not global.

## 9. Testing architecture (mapping test-strategy onto this design)

| Test level | Where | Tools |
|---|---|---|
| Unit | `chess-rules`, `chess-domain` (perft, SAN tables, Policy table, Elo vectors, clock with fake time, roster determinism, variation tree, `BoardGrid` snapshots); `chess-cli` (TUI `update` with synthetic messages, `render` into ratatui `TestBackend` at 80×24 and larger, key-map vs. SAN collision test) | `cargo test`, `proptest`, `insta` (snapshots), `cargo-mutants` (Elo, Policy), `cargo-deny`/`cargo-audit` and `npm audit` for known CVEs (CI) |
| Property / fuzz | random legal playouts; `parse(format(m)) == m`; arbitrary CLI input; protocol fuzzing at the WebSocket | `proptest`, `cargo-fuzz` |
| Core integration | `chess-app` with `FakeEngine` + in-memory or temp-file store: full game flows, Strict gateway spy, rewind/stale-ply rejection, lease takeover, crash-recovery (kill child process mid-write); failure injection for each §5.5 code (fake engine returning an illegal move, hanging, or crashing; store busy, full, or read-only; `SchemaTooNew` from a newer fixture DB); lagged event subscriber gets a snapshot; duplicate `StartGame{game_uuid}` and resent `MakeMove` are idempotent; slow fake store doesn't stall the clock timer (R-11); reset with unfinished games (D-30): a suspended game, a game live in the same process, and a game live in a second process are each voided, every slot is released, no rating, counter, W/L/D, or rating-history row changes for any side, a game started after the confirmation prompt causes a new `ConfirmationRequired`, and a kill between the reset commit and the `Voided` journal record recovers to `voided` | `tokio::test`, temp dirs |
| Engine integration | real Stockfish (the pinned Stockfish 19): UCI handshake, `id name` names the pinned version, the reported `UCI_Elo` range contains every stored `COMPUTER_PROFILE.uci_elo` (§4.6), legal `bestmove`, tactics fixtures, crash/timeout recovery, latency | gated by `--features stockfish-tests` |
| CLI E2E | scenario files replayed through a PTY: TUI screens asserted by parsing the terminal output into a virtual screen; resize; terminal restored after quit/Ctrl-C/panic; plain mode as a transcript and via pipe (scripted human play); headless c–c mode: usage errors for a human Player or a busy computer, `--data-dir` isolation, stored and rated result, exit 4 on a signal; `chess serve` config errors, browser-open failure and clean stop; `chess install-cli` twice, over a regular file, with `/usr/local/bin` absent or read-only, and `--remove` | `expectrl` + `vt100` (Rust) |
| GUI E2E (web) | same scenario files through the browser | Playwright: WebKit (for Safari), the `chrome` and `msedge` channels, Firefox; axe; screenshot diffs per theme, including light and dark app themes and emulated `prefers-color-scheme`; fluid-resize checks |
| Packaging | `lipo -info` on the bundled `stockfish` lists both `arm64` and `x86_64` (§8.2); also a step of the engine build script. `codesign --verify --deep --strict` passes. `Info.plist` declares `LSMinimumSystemVersion` 26.0 | CI script |
| Amendment 2 behaviours | Core integration and PTY/Playwright: name rules (case, grapheme clusters, NFC, control characters, archived collision); takeback bounds and record removal; draw offer (clocks stop, repeat-offer block, engine failure drops it); `movenow`/F2/button only in `EngineThinking` h–c; resume list scope; import duplicates (skip, keep both, no replace); `games.json` round trip and an archive without it; unfinished-game import with and without a slot clash; custom-start exclusion; empty statistics; damaged-journal resume and "voided: unreadable"; failed pre-upgrade backup; fault injection in each store-changing command (store identical afterwards); restore with a finished-after-backup game (each rating moves once, in end-time order); confirmation and Esc for each UI-12 action | `tokio::test`, `expectrl`, Playwright |
| Platform matrix | arm64 (the M4 baseline, plus one older M-series Mac if available) and x86_64, on Tahoe and on each later macOS release, starting at its developer beta: smoke test, engine integration, offline E2E, desktop E2E. **Best effort (CR-7.6):** a platform without available hardware is not run and is reported as "not run: hardware unavailable"; today that is every Intel Mac and every Mac model other than the development Mac. Running Intel would need a physical Intel Mac for as long as Intel is supported (R-13), and is done if one becomes available | manual + scripted |
| GUI E2E (desktop) | same GUI suite against the Tauri app — **see risk R-1** | Playwright WebKit against a Tauri dev build plus a scripted smoke test of the real `.app` |
| Calibration / soak | `chess calibrate`; 1000-game c–c soak | nightly CI |

The same scenario format (`e2e/scenarios/*.yaml`: setup, inputs, expected PGN/result/visible controls) feeds the CLI and GUI runners, which gives "one scenario, three front-ends" (test-strategy principle 3, PLAT-1). Test names carry requirement IDs for the traceability matrix.

---

## 10. Requirements traceability (component view)

### 10.1 Allocation: every requirement to one primary component

The **primary** component is accountable for the requirement. Supporting components take part. Verification hooks point to §9 and to the requirement's own verification list.

| Req | Primary component | Supporting | Design refs |
|---|---|---|---|
| PLAT-1 | `packaging/` (three entry points, one build) | `chess-cli` (+ `serve`), `chess-server`, `desktop/src-tauri` **Amd 2:** `packaging/` ad-hoc signs the app and its executables (ASM-5); the build is local (D-31). | §2, §6.3, §8.2 |
| PLAT-2 | `packaging/` (universal build, Tahoe minimum) | `ui/` (Safari, Chrome, Firefox, Edge), TUI on Terminal.app and iTerm2 **Amd 2:** `Release checklist and install docs` holds the platform matrix (§2.1). | §8.2, §9 |
| PLAT-3 | Offline enforcement (CSP, bundle scan, offline E2E) | every artifact. Bundled engine, fonts, themes, ECO table | §8.1 |
| PLY-1 | `chess-domain::GameSetup` | `GameSetupService`, setup screens, CLI flags | §4.4 |
| PLY-2 | `PlayerService` (register; profile chooser at launch) | `EloCalc` start values on both tracks, `PREFERENCES`, Register screens **Amd 2:** Active-Player rules (§6.1). | §6.1, §6.2, §7.4 |
| PLY-3 | Profile chooser at launch in every front-end (`SelectProfile`, Q48) | `GameSetupService` (side assignment), `APP_SETTING.last_active_player` (pre-selection only) | §6.1, §6.2, §7.2 |
| PLY-4 | `chess-domain::RosterGen` | `PlayerService`, `COMPUTER_PROFILE`, Computer Players screens **Amd 2:** Computer Players cannot be renamed, archived, deleted or reset. N Players per request, used names skipped, roster UUIDs from the seed (§4.6, D-34). Archive refused with unfinished games, no un-archive (§7.4). | §4.6 |
| PLY-5 | `chess-domain::Policy` | `allowed_actions` in every front-end, `EngineGateway` | §4.4, §4.5 |
| GAME-1 | `GameSetupService` | `Policy`, New Game screens (TUI, GUI), CLI flags **Amd 2:** Resume list (§4.4). Resign from the offer asks which side in h–h. | §4.4, §6.1 |
| GAME-2 | `GameSession` (pause, step, delay, watcher-only abort) | Watch screens | §4.3, §4.4 |
| GAME-3 | Front-end view cursor (TUI `TuiState`, GUI client state) | `GameSession` `ply` check (`StalePosition`) | §4.2, §6.2 |
| GAME-4 | `GameSetupService` + `UNFINISHED_SLOT` (one unfinished game per mode for each human, and for each computer in c–c only; D-27) | Resign as the only early end for humans, c–c watcher abort, `UnfinishedGameExists` (setup only), resume prompt, slots released by a reset void (D-30) **Amd 2:** `PlayerHasUnfinishedGames` for archive. `VoidUnreadableGame` frees the slot (§7.2). | §4.3, §4.4, §7.2 |
| RULE-1 | `chess-rules` | — | D-3 |
| RULE-2 | `chess-domain::Game` (termination) | `GameSession`, `EngineGateway(ComputerDrawDecision)` **Amd 2:** Draw-offer rules, clocks stop in `DrawOffered`, `ComputerDrawDecision` within the ENG-3 maximum (§4.4, D-33). | §4.3, §4.5 |
| RULE-3 | `chess-domain::Game` (takeback) | `Policy`, `GameSession` **Amd 2:** Takeback bounds, record removal, `NothingToTakeBack` (§4.4, D-33). | §4.3, §7.4 |
| RULE-4 | `chess-domain::Clock` | `GameSession` flag timer, client countdown, `SearchLimits` budget | §4.2 |
| RULE-5 | Move journal (§7.6) + `GameSession` (auto-save and resume) | `chess-store` (applies the journal), `SessionRegistry` (journal lock) | §7.2, §7.3, §7.6, D-23 |
| RULE-6 | `ImportExportService` (PGN export of any stored game) | `chess-rules` PGN writer (IF-1), history screens **Amd 2:** Unfinished-game export: `Result "*"`, no `Termination`, no `EndStatus` (§5.7). | §5.7 |
| RULE-7 | `ImportExportService` (PGN import, review only) | `chess-rules` PGN reader (IF-2); imported games are unrated (`status = 'imported'`) | §5.7, §7.2 |
| RULE-8 | `GameSetupService` (FEN start position, always unrated) | `chess-rules` FEN parser (IF-3), Import screen's analysis board | §5.7, §7.2 |
| ENG-1 | `chess-engine` | `packaging/` (bundled engine), `EngineGateway` failure handling | §4.7, §8.2 |
| ENG-2 | `chess-domain::RosterGen` + `Personality` | `chess-engine` (UCI options), `chess calibrate` | §4.6 |
| ENG-3 | `chess-engine::SearchLimits` (min of personality, the active profile's max think, and the clock budget) | `GameSession`, `EngineGateway` | §4.6, §4.8 |
| ENG-5 | `GameSession` (`CancelEngine` → UCI `stop` → best move so far, ≤ 200 ms) | `chess-engine`, every front-end's cancel control or key **Amd 2:** `movenow`, F2 and the GUI "Move now" button; only in `EngineThinking` of an h–c game (§4.4, D-33). | §4.4, §4.8 |
| DAT-1 | `chess-store` | leases, migrations, D-16 | §7 |
| DAT-2 | `ImportExportService` (merge import, zip export) | `chess-store` exclusive ops **Amd 2:** `games.json` with analysis results, unfinished-game restore rule, PGN duplicate prompt without replace (§5.7, §7.4, D-34). | §5.7, §7.4, D-17 |
| DAT-3 | `chess-store` schema | `GameSession` time accounting, `AnalysisWorker`, ECO classifier (D-20) | §7.2, §5.4 |
| ELO-1 | `chess-domain::EloCalc` (both sides, human and computer, Q41; K per track from that track's rated-game counter, Q78) | finalize transaction re-reads current ratings, `RATING_HISTORY` | §5.4, §7.2, §7.4 |
| ELO-2 | `chess-domain::Policy` + `EloCalc` (Elo track = game mode: Training Elo or Strict Elo) | `PLAYER.*_elo`, `RATING_HISTORY.track` | §4.4, §7.2 |
| UI-1 | `PlayerService` (`PREFERENCES`) | GUI/TUI settings screens, gateway (active profile's max think, §4.6). No sound or confirm-move settings | §6.2, §4.6 |
| UI-2 | `StatsService` (W/L/D including Training; average length in moves and time) | Stats screens (TUI `Chart`, GUI chart D-21) **Amd 2:** Empty states, custom-start exclusion, CLI Elo chart (§6.1, D-37). | §7.2 |
| UI-3 | `ui/themes` (GUI), `chess-domain::BoardStyle` (CLI) | contrast test | §6.2 |
| UI-4 | `ui/board` `AccessibleOverlay` + `LiveRegion` (GUI), `PlainRunner` (CLI) | §8.3 | §11.1 |
| UI-5 | `StatsService` | Computer Players screen, with an Elo chart for computers too (Q41). Category label from the Strict Elo (Q79) | §4.6, §7.2 |
| ANA-1 | `EngineGateway` (+ `Policy`) | `AnalysisService`, audit log spy | §4.5 |
| ANA-2 | `AnalysisService` + `VariationTree` | `LegalMoves` (D-11), `VARIATION` | §5.1 |
| ANA-3 | `AnalysisService` | `AnalysisWorker`, `MOVE_EVAL` **Amd 2:** "evaluating…" placeholder and the n-of-N count (§6.1, `GetGameAnalysis`). | §5.4 |
| CLI-1 | `chess-cli` (`TuiApp`, screens; 80×24 with resize attempt) | `PlainRunner`, terminal guard | §6.1 |
| CLI-2 | `chess-domain::BoardGrid` | `BoardWidget` | §6.1 |
| CLI-3 | `chess-cli::InputParser` | `chess-rules` SAN parsing | §6.1 |
| CLI-4 | `chess-cli::KeyMap` + `clap` flags | headless runner, D-19 **Amd 2:** `movenow`, `save` (prints the stored move count), headless c–c only, `--data-dir`, F2 in `KeyMap` (§6.1, D-36). | §6.1 |
| GUI-1 | `ui/` (one bundle) | `CoreClient` with two transports, R-1 | §5.2, §6.2 |
| GUI-2 | `ui/board` (`BoardModel`, overlay, chessground adapter; commit on drop, Enter, or destination select) | SAN input, `PromotionPicker` | §6.2, §11.1 |
| GUI-3 | `ui/board` (coordinates option) | `PREFERENCES` | §6.2 |
| GUI-4 | `ui/` Play screen | `EngineThinking` / `GameEnded` events **Amd 2:** "Move now" button in the thinking indicator (§6.2). | §6.2 |
| NFR-1 | Lazy engine start + first frame before engine | embedded bundle, startup benchmark | §4.7, §6.1 |
| NFR-2 | `chess-store` (WAL, transactions in §7.4) | durable-before-ack (§4.8) | §7 |
| NFR-3 | `EngineGateway` legality check + `chess-rules` | fuzz, 1000-game soak | §4.5 |
| NFR-4 | `chess-server` loopback + no telemetry | network-capture harness | §8, §8.1 |
| NFR-5 | `packaging/` + README install docs | — **Amd 2:** Install, update, uninstall text, first-open steps and the data-directory warning live in `Release checklist and install docs` (§2.1, §8.2). | §8.2 |
| NFR-6 | CI license audit (`cargo-deny`, npm checker) | `third_party/`, notices in every artifact **Amd 2:** Allocated to the `CI checks` block (§2.1). | §8 |

Every requirement in requirements.md at its 2026-10-26 baseline has a primary component (checked by script against the requirement headings and tables; re-checked by script on 2026-10-27 against requirements.md v1.0 Amendment 1: all 78 requirement and RISK IDs have an allocation row, and Amendment 1 added no new requirement ID; re-checked by script on 2026-10-28 against Amendment 2: all **82** requirement and RISK IDs have a row, the 78 plus PLAT-6, DAT-7, UI-12 and NFR-10, and the 26 amended requirements carry an "Amd 2" note): the original requirements, and the baseline splits RULE-6–RULE-8 (from RULE-5) and ENG-5 (from ENG-3), in the table above, plus every other ID added on 2026-10-26 in the table below. Every AGREED requirement is a mandatory "shall" (user, 2026-10-26), so none of them may be left without an architectural home or deferred.

**Requirements added by the systems engineer on 2026-10-26.** All of them are AGREED at baseline (Q75 accepted every PROPOSED requirement and TBR item), so these allocations are final:

| Req | Primary component | Supporting | Design refs |
|---|---|---|---|
| PLAT-4 | `chess-cli serve` → `chess-server` | `packaging/` **Amd 2:** `chess-cli` serve lifecycle: `ConfigInvalid`, browser-open failure, SIGINT/SIGTERM stop with `ServerStopping` (§6.1, D-36). | §8.2, D-8 |
| PLAT-5 | `packaging/` (one `Chess.app` bundle with `chess` and Stockfish) | `chess-store` (version stamp), `chess-server` / gateway (version handshake) | §7.5, §8.2, D-16, D-18 |
| PLY-6 | `PlayerService` (`ValidationFailed`; trim, NFC, ≤ 128 grapheme clusters, no control characters, unique case-sensitively against every Player, active or archived) | Register/rename screens | §5.5, §7.2, §7.4 |
| PLY-7 | `PlayerService::reset` (`reset_at`, reset points, confirmed, humans only, voids the Player's unfinished games; D-26, D-30) | `ConfirmationRequired{voids}` (lists the games to be voided); `GameSession` (`Void`, `GameVoided`); stats and history queries for the reset Player filter by `reset_at`, opponents' queries don't (Q76) | §4.3, §5.1, §5.5, §7.2, §7.4, §7.6 |
| ENG-4 | `chess-engine` + `EngineGateway` failure path | `GameSession` (`EngineError`), D-15 | §4.7, §4.8 |
| DAT-4 | `SessionRegistry` (journal `flock`, `GAME_LEASE` for display) | `GameLeased` error | §7.3, §7.6 |
| DAT-5 | `chess-store` (`quick_check`, restore flow) | `RestoreBackup` **Amd 2:** Restore replays every retained journal and re-finalises in end-time order. Automatic backups at start-up when the newest is older than 24 h, newest 3 kept (§7.1, D-35, TBD-33; DAT-5 Amendment 3). Restore limit: R-19. | §7.1 |
| DAT-6 | `chess-store` migrations (`VACUUM INTO` backup) | D-16 **Amd 2:** A failed pre-upgrade backup stops the upgrade: `BackupFailed` (§7.1). | §7.1, §7.5 |
| UI-6 | `ui/board` `LiveRegion` | §8.3 announcement list | §8.3 |
| UI-7 | `ui/themes` + contrast unit test (WCAG AA for every theme, high contrast included) | axe | §6.2, §8.3 |
| UI-8 | `AccessibleOverlay` focus ring + global focus styles | — | §11.1, §8.3 |
| UI-9 | Help screens (TUI, GUI), content bundled. CLI help is generated from `KeyMap`, so it can't drift | plain-mode `help` | §6.1, §8.1 |
| UI-10 | `ui/themes` app colour themes (System / Light / Dark / others; live switch on appearance change) | `PREFERENCES` | §6.2, §8.3 |
| UI-11 | English-only string modules per front-end | help content (UI-9) | §6.2 |
| ANA-4 | `AnalysisService` (from game history) | `GetGameAnalysis`, history screens | §5.1 |
| CLI-5 | `chess-cli` headless runner | exit codes (§6.1, TBD-21 closed, codes 0 to 4) **Amd 2:** Usage errors for a human Player or a busy computer. Exit 4 on a signal (accepted by CR-5). `--data-dir` (§6.1). | §6.1 |
| GUI-5 | `ui/` fluid layout. No minimum size (Q55) | `ResizeObserver` → chessground `redrawAll` | §6.2, §11.1 |
| NFR-7 | `chess-server` Host/Origin allow-list | D-22 | §5.2 |
| NFR-8 | Concurrency model (D-13): no blocking on UI threads | `CancelEngine`, latency test | §4.8 |
| NFR-9 | Error catalogue (`code` → message) | every front-end | §5.5 |
| IF-1, IF-2, IF-3 | `chess-rules` PGN/FEN + `ImportExportService` | **Amd 2:** IF-1 unfinished-game export. IF-2 duplicate prompt with skip and keep both, never replace | §5.7 |
| IF-4 | `chess-engine` + `third_party/stockfish` (D-9: Stockfish 19, `sf_19`) | `packaging/` (`lipo -info` check) | §4.6, §8.2 |
| IF-5 | `ImportExportService` (format version) | D-17 **Amd 2:** Archive gains `games.json` (§5.7, D-29 amended). | §5.7, §7.4 |
| RISK-1 (requirements risk) | Mirrored by architecture risk R-13 (Tahoe is the last Intel macOS) | D-25 | §12 |
| RISK-2 (requirements risk) | Mirrored by architecture risk R-17 (computer rating drift). Test and calibration stores are isolated | `--data-dir` and `CHESS_DATA_DIR`, `chess calibrate`. **Amd 2:** scripted runs are stored and rated, so isolation is by `--data-dir` (§6.1) | §4.6, §12 |
| PLAT-6 (added 2026-10-28, CR-4) | `CLI front-end` (`chess install-cli`, `--remove`) | `packaging/` (`LSMinimumSystemVersion`, ad-hoc signing), `Release checklist and install docs` (update, uninstall, first-open steps, data warning). ASM-5 | §8.2, D-18, D-31 |
| DAT-7 (added 2026-10-28, CR-4) | `chess-app core :: chess-store` (all-or-nothing transactions, journal recovery) | `PlayerService`, `ImportExportService`, `SessionRegistry` (`JournalUnreadable`, `VoidUnreadableGame`), every front-end (message and next step, NFR-9) | §5.5, §7.2, §7.6, D-35 |
| UI-12 (added 2026-10-28, CR-4) | `chess-app core` (`ConfirmationRequired`, one mechanism) | every front-end's dialog, including the TUI Ctrl-C quit dialog | §5.5, §6.1, §8.3 |
| NFR-10 (added 2026-10-28, CR-4) | `chess-app core :: Diagnostic log` | `tracing`, size-triggered rotation (TBD-32) | §8, §10.2 |

**Values for the TBDs that requirements.md assigns to the architect (set at baseline v1.0):**
- TBD-3: default port 8765 (§8.2).
- TBD-13: follows the accepted EST in ENG-4: 5 s for a readiness reply, and 2 s past the move's time limit (§4.8).
- TBD-21: exit codes 0 result reached, 1 usage or setup error, 2 engine unavailable, 3 store error, 4 stopped by a signal (§6.1, D-36). Closed by requirements Amendment 3 (CR-5).
- TBD-24: custom tags `Mode`, `Rated`, `WhiteClass`, `BlackClass`, plus `EndStatus` (`aborted` | `voided`, only for games that ended without a result; added 2026-10-27, D-30) (§5.7).
- TBD-25: 10 MB (§5.7, `ESTIMATE`).
- TBD-28: 0 s. The journal `flock` is released by the OS when the owner exits or crashes, so another process can take over at once (§4.8, §7.3).
- TBD-29: `config.toml` (TOML) in the data dir, overridable with `--config` or `CHESS_CONFIG` (§8.2, D-24).
- TBD-26: **Stockfish 19, tag `sf_19`** (published 2026-09-05, not a pre-release; https://github.com/official-stockfish/Stockfish/releases/tag/sf_19, checked 2026-10-27). Closed as a version choice (user 2026-10-27: "use the latest Stockfish"). Two facts about it are still checked at the first engine build-script run (ASM-3, step 2). A later version needs a decision-log entry and a calibration re-run (D-9, §11.3).
- TBD-18: **still open.** The events to announce are fixed in §8.3; the exact wording is written with the GUI string module in step 3 and checked in the scripted VoiceOver walk-through (owner: GUI developer).
- TBD-31 (added 2026-10-28): computer Players per generation request: 1 to 25, default 5 (§4.6).
- TBD-32 (added 2026-10-28): diagnostic log rotation: 10 MB per file, 3 files (§8).
- TBD-33 (added by requirements Amendment 3, CR-5.1; from D-35, `ESTIMATE`): automatic backup when the newest is older than 24 h, newest 3 kept (§7.1).
- TBC-10: carried as ASM-5 (§14).
- Closed by the user and met by the design: TBD-1 (Tahoe minimum, §8.2), TBD-7 and TBD-23 (last recorded clock value, no completed move lost: §7.1, §7.6), TBD-12 (≤ 200 ms EST, §4.8).

### 10.2 Reverse check: elements without a driving requirement

| Element | Driver | Verdict |
|---|---|---|
| `chess calibrate`, `EnginePurpose::Calibration` | ENG-2 calibration verification (dev tooling, not shipped behaviour) | Keep. Not reachable through the protocol (§4.5) |
| `tracing` log file | NFR-10 (CR-4.18), correlation ids (§5.5) | Keep. Local only (NFR-4); identifiers only |
| Automatic backups (D-35) | DAT-5 (Amendment 3, CR-5.1: cadence and retention stated); also serves DAT-6 | Keep. Driven by a requirement since CR-5 |
| `--config`, `CHESS_CONFIG` (D-24) | none. Architect-added for tests; accepted as test-only in PLAT-4 (CR-5) | Keep, recorded (H-13) |
| Exit code 4 (D-36) | CLI-5 and TBD-21 (closed with codes 0 to 4 by CR-5) | Keep |
| `chess games`, `--on-duplicate`, `--yes`, the `export`/`import` command forms (D-36) | CLI-4 note (CR-5): design choices realising IF-1, IF-2, DAT-2 and UC-22 to UC-25 | Keep. Names become public after release |
| `MOVE_EVAL.engine_version` | IF-5 (`games.json` carries the engine version of each evaluation), DAT-3, ANA-3, D-9 (a later engine needs a decision) | Keep |
| `GAME.uuid`, `content_hash` | `GAME.uuid`: DAT-2 (export/import identity) and §5.6 idempotency. `content_hash`: IF-2 (a duplicate has identical tags and moves) and DAT-2 | Keep |
| `PROCESS_HEARTBEAT`, `SchemaTooNew` | derived from DAT-1 (concurrent access, migrations) and D-16 | Keep. Requirements gap G-9 asks for an explicit requirement |
| `APP_SETTING` | PLY-3 (switching), PLY-4 (seed) | Keep |
| `Internal{id}` correlation ids | derived | Keep |
| Coding standard and static checks (§8) | User decisions of 2026-10-28 and 2026-10-30 (requirements Decision Log, CR-6.1, CR-7.5); not a product requirement | Keep. It governs how the code is written and reviewed, not what the product does |

There are no orphan components. The enabling-system blocks of §2.1 (`packaging/`, `third_party/stockfish/`, `CI checks`, `Release checklist and install docs`) are driven by PLAT-1, PLAT-2, PLAT-6, IF-4, NFR-5, NFR-6, RISK-1 and ENG-2 (UC-29).

### 10.3 Requirement gaps found by this review (for the systems engineer)

The last column links each gap to the item the systems engineer opened in requirements.md on 2026-10-26, where one exists. "new" means the gap is not covered there yet. All three rounds of the user's answers (2026-10-26) are reflected below.

| ID | Gap | Affects | Status at baseline v1.0 (2026-10-26) | Systems-engineer item |
|---|---|---|---|---|
| G-1 | Web "app closed" undefined | RULE-5, RULE-4 | **Resolved:** a closed or disconnected tab suspends the game at once (D-14) | Q66 |
| G-2 | Clock during engine failure | ENG-1, RULE-4 | **Resolved:** keeps running (D-15) | ENG-4 |
| G-3 | Full-data import semantics | DAT-2 | **Resolved:** merge (D-17). Export is one archive (§5.7) | IF-5 |
| G-4 | Whose max-think-time setting applies in c–c and h–h | UI-1, ENG-3 | **Resolved:** the active profile's setting. c–c: caps both computers. h–h: caps engine analysis only (§4.6) | Q69 |
| G-5 | CLI scope of UI-1 settings | UI-1, CLI-1 | **Resolved:** no sound and no confirm-move in v1. All inputs are always active | Q65 |
| G-6 | Training games in W/L/D | UI-2 | **Resolved:** counted, with a separate Training Elo | ELO-2 |
| G-7 | Meaning of "user can cancel" | ENG-3 | **Resolved:** the ENG-3 amendment was accepted (Q75). `CancelEngine` = move now, within 200 ms (TBD-12) | ENG-3 amendment (c) |
| G-8 | Accessibility not quantified | UI-4 | **Resolved:** WCAG AA for every theme, high contrast included (UI-7) | Q68 |
| G-9 | Mixed builds sharing one DB | DAT-1 | **Resolved:** never allowed. Detected and refused (D-16) | — |
| G-10 | Recalibration after release | ENG-2 | Resolved (Q41) | — |
| G-11 | Baseline machine | NFR-1, ENG-3 | Resolved: M4, 16 GB, Tahoe (performance baseline only) | — |
| G-12 | Name rules | PLY-6 | **Resolved:** ≤ 128 user-perceived characters (grapheme clusters, CR-4.1), unique case-sensitively, case kept. Identity is the UUID | Q63 |
| G-13 | Stale clock log row | RULE-4 | Done | — |
| G-14 | No way to leave a game | GAME-4 | **Resolved:** Resign (counts as a loss). Abort is c–c watcher only | Q60 |
| G-19 | Does an h–c game use the computer's unfinished-game slot? | GAME-4 | **Resolved:** no. Slots are held by humans, and by computers only in c–c (D-27) | Q77 |
| G-20 | Reset while the Player has an unfinished game | PLY-7, GAME-4 | **Resolved 2026-10-27 (user):** allowed. Each unfinished game is voided and counts against neither player (D-30, §7.2). The systems engineer adds the rule to PLY-7 | user decision 2026-10-27 |
| G-15 | ECO data source and license | DAT-3, PLAT-3, NFR-6 | **Resolved 2026-10-30 (user, CR-8):** the Lichess `chess-openings` dataset, CC0-1.0, copied into `third_party/eco/` with its source commit and SHA-256 and a notice file | DAT-3 amendment (a), D-20 |
| G-16 | Computer Elo vs. the old ELO-1 text | ELO-1, PLY-4, UI-5 | **Resolved:** dynamic on two tracks (Training too, for computers), c–c rated, K counted per track, label follows the Strict Elo | Q57–Q59, Q78, Q79 |
| G-17 | Abort-as-loss rules | GAME-4 | **Resolved:** replaced by Resign; the c–c watcher abort has no effect | Q60 |
| G-18 | Reset scope | PLY-7 | **Resolved:** everything except Elo, humans only. Old games are hidden only from the reset Player's views and stay in opponents' history and stats (D-26) | Q62, Q76 |

### 10.4 Disposition of the systems engineer's handoff (docs/handoff_se_to_architect.md)

Pushback is recorded with its reason. "Done" means the architecture text, tables and diagrams now carry the item.

| Item | Disposition | Where / reason |
|---|---|---|
| H-01 | Done | §5.5, §7.2 names bullet, §3.2 libraries |
| H-02 | Done. **Pushback (partial):** accepted as written, but `games.json` needs a `MOVE_EVAL.engine_version` column (IF-5 already lists the engine version, so the column is required, not extra), and recompute-after-import stays the fallback for an archive without it | §5.7, §7.2 |
| H-03 | Done. **Accepted with a note:** the importer can tell a played game from an imported one, but with no "replace" it never needs to. Duplicate detection uses `content_hash` on every game, not `import_hash` on imports only | §5.7, §7.4, D-34 |
| H-04 | Done | §5.7 |
| H-05 | Done | §4.4, §5.1, §6.1, §6.2 |
| H-06 | Done | §4.3, §4.4, §5.5 |
| H-07 | Done (R-20 accepted by CR-5.3). **Accepted with a note:** all clocks stop fits the §4.2 clock model (`running_side = None`). A suspend drops an offer to a computer and keeps an offer to a human. Risk R-20: in hot-seat, an unanswered offer stops both clocks indefinitely | §4.2, §4.3, §4.4, R-20 |
| H-08 | Done | §4.4, §5.1 |
| H-09 | Done | §6.1, §6.2 |
| H-10 | Done | §5.5, §7.4, §6.1 |
| H-11 | Done. Roster UUIDs are derived (UUIDv5 of seed and index); the name is the first unused candidate at that index | §4.6, D-34 |
| H-12 | Done. **Pushback (consequence):** headless being c–c only withdraws the earlier "human side reads stdin" mode, so scripted h–c and h–h scenarios run through plain mode with piped input. Exit code 4 added for a signal. Distinct export and import forms in §5.7. **Systems engineer, CR-5.2: pushback 3 accepted;** CLI-4 now says scripted human games use plain mode with piped input, and CLI-5 and TBD-21 list exit code 4 | §6.1, §5.7, §7.1, D-36 |
| H-13 | Done. `--config` and `CHESS_CONFIG` kept as architect-added (accepted as test-only, CR-5.2) | §6.1, §8.2, D-36 |
| H-14 | Done. CLI chart decided (D-37). D-21 stays deferred for the GUI | §6.1, D-37 |
| H-15 | Done. **Pushback (addition):** "all-or-nothing" for restore needs copy-then-rename, so restore works on a copy | §5.5, §7.1, §7.5, §7.6, D-35 |
| H-16 | Done. **Pushback (two points).** (1) The requirement says journals are kept "until the next backup has run", but only pre-upgrade and pre-import backups existed, so `done/` would grow until a migration. The architecture adds an automatic backup cadence (D-35, TBD-33). (2) A journal cannot recreate a Player created, renamed or reset after the backup, so such games are kept in `journal/unrestored/` and named in the message (OQ-12, R-19). Re-finalising is idempotent and ordered by end time; it is order-dependent on the ratings in the restored copy. **Systems engineer, CR-5.1: both points accepted.** DAT-5 now states the automatic backup and the limit; OQ-12 is closed | §7.6, D-35, R-19, OQ-12 |
| H-17 | Done | §6.1, §7.2 |
| H-18 | Done | §2, §2.1, §2.2, D-31 |
| H-19 | Done. The `install-cli.sh` script and the app menu item are dropped in favour of one subcommand. Facts unverified (ASM-5) | §8.2, D-18 |
| H-20 | Done. The Ctrl-C quit dialog is local to the TUI (no data changes), the other six go through `ConfirmationRequired`. `VoidUnreadableGame` is a core-offered action whose dialog is its confirmation; UI-12's list does not include it. **Systems engineer, CR-5.4: accepted;** UI-12 says the offer is itself the explicit choice | §5.5, §8, §8.3 |
| H-21 | Done | §10.1, §14 |
| H-22 | Done: no per-function budget is allocated (CR-4.19); §4.8 values stay design values | §4.8 |
| H-23 | See functional_decomp.md v0.2 | functional_decomp.md |
| SE FG-2, FG-3 | Closed by §2.2 (named blocks, `Tauri shell and IPC adapter`, one `stockfish` block) | §2, §2.2, D-32 |
| SE FG-14, OQ-12 | Closed by requirements Amendment 3 (CR-5.1): F-6.15 is the automatic backup function | §7.1, D-35 |
| SE F-2.5.7 allocation | The shared confirmation check is a cross-cutting duty of `chess-app core` (§4.1), so F-2.5.7 is allocated to `chess-app core`, not to the Protocol dispatcher | §4.1 |

---

## 11. Decisions

| ID | Decision | Recommendation | Alternatives / trade-off | Status |
|---|---|---|---|---|
| D-1 | How the desktop app reaches the core | Link `chess-app` in-process; Tauri IPC with the same envelope as the WebSocket | Desktop starts the embedded axum server and loads `http://127.0.0.1:<port>`: one transport everywhere and identical E2E paths, but it opens a port and is less idiomatic Tauri | AGREED 2026-10-25 |
| D-2 | Process topology | Each entry point embeds the core; coordination through SQLite leases | A single background daemon (launchd) that all front-ends talk to: one engine pool and live cross-process updates, but more moving parts and a daemon lifecycle to manage | AGREED 2026-10-25 |
| D-3 | Move generation | Wrap `shakmaty` + `pgn-reader` (GPL-3.0, mature, fast, perft-verified) | Write our own in `chess-rules`: full control and a learning exercise, but weeks of work and the risk of subtle rule bugs; perft tests still apply either way | AGREED 2026-10-25 |
| D-4 | GUI board | `@lichess-org/chessground` + our shared keyboard/accessibility layer (overlay ARIA grid + `BoardModel` reducer); own board ruled out for v1 | Fallback: react-chessboard + same layer, patched (study option D1, not decision D-1). Studies: [first pass](trade-study-gui-board.md), [refined](trade-study-gui-board-refined.md) | AGREED 2026-10-25 |
| D-5 | UI framework | React + TypeScript + Vite | Svelte or SolidJS: smaller and faster, with a smaller testing/a11y ecosystem | AGREED 2026-10-25 |
| D-6 | CLI style | Full-screen TUI with `ratatui` + `crossterm`, Elm-style update/render; plain mode for screen readers and pipes; headless for scripts | — (decided by the user; requirements CLI-1 amended) | AGREED 2026-10-24 |
| D-7 | Rust ↔ TS type sharing | `ts-rs` | `specta` / `tauri-specta`: tighter Tauri integration, but less useful on the WebSocket path | AGREED 2026-10-25 |
| D-8 | Web server lifecycle | `chess serve` started by the user (prints and opens URL) | A launchd agent that is always running, so the URL "just works" | AGREED 2026-10-25 |
| D-9 | Stockfish acquisition | Pin one release; a build script downloads and checksums it at build time and bundles it (run time stays offline, §8.1). **Pinned release (2026-10-27): Stockfish 19, tag `sf_19`.** Preferred: the official `stockfish-macos-universal.tar.gz` asset, accepted only if `lipo -info` shows arm64 and x86_64. Fallback: build arm64 and x86_64 from the `sf_19` source and join them with `lipo` (§8.2) | Require `brew install stockfish` and find it on `PATH`: simpler builds, worse install story. Stay on an earlier release: no reason, as the user asked for the latest | AGREED 2026-10-25. Version AGREED 2026-10-27 (user: "use the latest Stockfish") |
| D-10 | Project license | GPL-3.0-or-later | Required only if builds are distributed; local-only use doesn't trigger GPL obligations, but choosing now avoids relicensing | AGREED 2026-10-25 |
| D-11 | Legal-move lookups for what-if branches | Ask the core (`LegalMoves{fen}` query; localhost round trip ≪ 1 ms, `ESTIMATE`) | Compile `chess-rules` to WASM for the UI: no server round trip, but a second build target and logic in two places | AGREED 2026-10-25 |
| D-12 | Data store technology | SQLite via `rusqlite` (bundled), WAL, one file shared by all entry points (§7.1) | A store per process synced through files: no shared-file locking, but conflicts. An embedded KV store (e.g., sled/redb): no SQL for stats, weaker multi-process support | AGREED 2026-10-26 (user, final round). One-way for data; the user accepted the consequences (§11.3) |
| D-13 | Concurrency model | tokio runtime, one actor per live game, SQLite on a dedicated store thread, engines as child processes (§4.8) | Async SQLite crate (`sqlx`): fewer threads, but a second SQLite binding and less control over transactions. Global mutex around the store: simpler, but blocks the runtime | AGREED 2026-10-26 (user, final round) |
| D-14 | Live game with no front-end attached (tab closed, WebSocket dropped, c–c with no watcher) | **Decided:** suspend immediately. `Suspend` is journalled, clocks freeze at the recorded value, and the game resumes when a client resubscribes | Grace period; keep running | AGREED 2026-10-26 (user) |
| D-15 | Clocks while the engine has failed (`EngineError`) | **B (decided):** the side to move's clock keeps running. A computer that can't recover in time can lose on time | A: freeze both clocks. C: adjudicate | AGREED 2026-10-26 (user, Q53) |
| D-16 | Version consistency | CLI and desktop ship and update together as one bundle (§8.2). Any mismatch with the DB, a running process, a server, or a tab is detected and refused with a message. No mixed-version support (§7.5) | Backward-compatible mixed versions (rejected by the user) | AGREED 2026-10-26 (user, final round). One-way for data; the user accepted the consequences (§7.5, §11.3) |
| D-17 | Full-data import (DAT-2) | **Merge (decided):** preview the conflicts, then the user picks Replace or Keep both (distinct identity) for each one. One exclusive transaction after a backup (§7.4) | Replace all; import only into an empty store | AGREED 2026-10-26 (user, Q51; merge mechanics accepted in the final round) *Amended 2026-10-28 (D-34, CR-4.2, CR-4.3): the PGN-import duplicate prompt offers Skip (default) and Keep both, never Replace. An unfinished game in an archive is restored as unfinished when no slot would be doubled.* |
| D-18 | Install layout and engine discovery | `Chess.app` is the only artifact. It contains `chess` and `stockfish`, and `chess` is symlinked onto `PATH`. The engine is found next to the executable, never on `PATH` (§8.2) | Separate CLI install in `~/.local` (dropped: it allowed version skew); Homebrew formula | AGREED 2026-10-26 (user, final round) *Amended 2026-10-28 (CR-4.5, PLAT-6): `chess install-cli` and `--remove` replace the install script. The link goes in `/usr/local/bin` if it exists and is writable, else `~/.local/bin`. The bundle declares macOS 26 as `LSMinimumSystemVersion`. The build ad-hoc signs the app and its executables. Update replaces the bundle. Uninstall keeps the data (§8.2). Facts unverified: ASM-5 (TBC-10).* |
| D-19 | CLI run-mode selection | Ordered rule: headless if `--headless` or top-level setup flags; plain if `--plain` or no TTY; otherwise TUI (§6.1) | Explicit subcommands only (`chess headless …`): unambiguous, but changes the CLI-4 example syntax | AGREED 2026-10-26 (user, final round) *Amended 2026-10-28 (D-36): headless is computer–computer only and Strict. Scripted human play uses plain mode with piped input.* |
| D-20 | ECO classification | Bundled opening table in `chess-domain`. **Source (G-15, 2026-10-30):** the Lichess `chess-openings` dataset (CC0-1.0), five TSV files with columns `eco`, `name`, `pgn`. A pinned snapshot lives in `third_party/eco/`; its commit and SHA-256 are recorded there when it is taken at the start of step 1. A `chess-rules` build step replays each `pgn` line with shakmaty and keys the table by position (EPD, without move numbers), so a transposition matches. **Rule:** after each move, look the position up; the **deepest named position reached** gives the game's code and name; no named position gives none. A game stores both (`GAME.eco`, `GAME.opening_name`). For an imported PGN, an `ECO` tag in the file is kept and the name comes from the table. Never fetched at run time or at build time | Leave ECO empty except for imported PGN `ECO` tags: no data dependency, but DAT-3 isn't met. A hand-written table, or another dataset with a less clear license | AGREED 2026-10-26 (user, final round); source AGREED 2026-10-30 (user, CR-8) |
| D-21 | GUI Elo chart | A small charting dependency or hand-built SVG | — | DEFERRED (two-way, at implementation) *2026-10-28: this entry is the GUI chart only. The CLI chart is decided in D-37.* |
| D-22 | Web endpoint authentication | No per-launch token in v1. Host/Origin checks stop browser-borne attacks, and a local process could read `chess.db` directly anyway | Per-launch token in the URL: stops other local processes from driving the core, at the cost of bookmarkable URLs | AGREED 2026-10-26 (user, final round). ASM-1 CONFIRMED |
| D-23 | How "no completed move is ever lost" is met (Q54) | A per-game append-only move journal, flushed with `F_FULLFSYNC` before each acknowledgement. It is applied to SQLite when available, and the journal wins on disagreement. Its `flock` is the live-game lease (§7.6) | SQLite only with `synchronous=FULL`: simpler, but moves block or fail when another process holds the DB, and there is no local cache to prefer (Q54 asks for one) | AGREED 2026-10-26 (user, final round) (the rule itself via Q54) |
| D-24 | Config file and port resolution (Q55) | `config.toml` in the data dir. Precedence: CLI flag, then config file, then default 8765 | Environment variable only; a `~/.config/chess/` (XDG) location | AGREED 2026-10-26 (user: precedence Q55; location and default port in the final round) *Amended 2026-10-28 (D-36, CR-4.14): an invalid config stops `chess serve` with exit 1. `--config` and `CHESS_CONFIG` are architect-added. `--data-dir` overrides `CHESS_DATA_DIR` (§7.1).* |
| D-25 | Target platform and build | macOS Tahoe and later, on Apple silicon (M1+) and on Intel while Apple supports it. Universal arm64 + x86_64 `Chess.app` (with `chess` and Stockfish). M4/16 GB is the performance baseline. Browsers: the last 3 years of Safari, Chrome, Firefox, Edge | arm64 only | AGREED 2026-10-26 (user). x86 Stockfish variant accepted. 2026-10-27: with Stockfish 19's run-time CPU dispatch, one x86_64 slice is expected to cover every Intel Mac; whether the official macOS asset holds both slices is checked under ASM-3 |
| D-26 | Player reset (PLY-7) | Humans only. *Revised 2026-10-27: allowed with unfinished games, which the reset voids (D-30); the baseline rule "only with no unfinished game" is superseded.* Clears everything except both Elo ratings and both rated-game counters. It sets a `reset_at` epoch, restores default preferences, and writes a reset point on each rating track. Views of the reset Player show only games and rating points after `reset_at`. Opponents' history and stats keep the old games (§7.2) | Delete the Player's games (also removes them from opponents' history and stats) | AGREED 2026-10-26 (user, final round) (Q76: old games stay in opponents' views). Revised 2026-10-27 (user: "allow it; the unfinished game does not count against either player") |
| D-27 | Where and for whom "one unfinished game per mode per Player" is enforced (GAME-4) | `UNFINISHED_SLOT(player_id, mode)` primary key, written in the `StartGame` transaction. Every human Player holds a slot in each game it plays. A computer holds a Strict slot only in c–c games. A human–computer game never uses or checks the computer's slot, so any Player can always play any computer | A service-level check (racy across processes). Computers also blocked by h–c games (rejected by the user: nobody could play a computer that has an unfinished game) | AGREED 2026-10-26 (user, final round) (Q77: h–c never uses the computer's slot) |
| D-28 | Two Elo tracks (ELO-2, user 2026-10-26) | `strict_elo` / `training_elo` and one rated-game counter per track (`strict_rated_games`, `training_rated_games`) for every Player, human and computer. `RATING_HISTORY.track`. The game's mode selects the track, and each track's counter sets its K (Q78). A computer's label follows the Strict Elo (Q79) | One shared Elo, with Training excluded (the earlier rule) | AGREED 2026-10-26 (user, final round) |
| D-29 | Full-data export format (DAT-2, IF-5) | One zip: `manifest.json`, `games.pgn` with UUID and clock tags, `players.json`, `ratings.json`, `settings.json` (§5.7) | A folder; a single JSON | AGREED 2026-10-26 (user, final round) *Amended 2026-10-28 (D-34, CR-4.3): the archive gains `games.json` (stored per-game fields and analysis results). `format_version` stays 1 because nothing has been released.* |
| D-30 | What a reset does to the Player's unfinished games (PLY-7, GAME-4; closes OQ-8) | **Void them in the reset transaction.** The confirmation lists them. Each gets `status = 'voided'` (terminal state `Voided`, §4.3), no result, no Elo, counter, W/L/D, or rating-history change for any side, and every `UNFINISHED_SLOT` row of the game is released. The journal is closed, not deleted. A live session gets `GameVoided` and stops. **Interpretation (architect, for the user to correct, Q-A18):** the voided game stays in the opponent's history marked "voided by reset", with no result, and can be analysed. It is hidden from the reset Player's views by `reset_at` (Q76). It is not a resignation, so the Q60 loss rule doesn't apply. Exported as `Result "*"`, `Termination "abandoned"`, `EndStatus "voided"` (§5.7) | (a) Refuse the reset while a game is unfinished (the baseline rule; rejected by the user). (b) Treat each unfinished game as resigned by the reset Player (rejected: the user said it counts against neither player). (c) Delete the unfinished game (rejected: it would remove it from the opponent's history, against Q76) | AGREED 2026-10-27 (user decision). Visibility details are an interpretation (Q-A18) |
| D-31 | How the enabling system (build, release, install docs) appears in the architecture (H-18, FG-1, FG-10) | **§2 stays a run-time view. §2.1 is a second context view** with the external entity `Release engineer` (the project owner in the build role), the supporting entities `Stockfish release source` (build time only) and `Build and CI environment`, and four blocks: `packaging/`, `third_party/stockfish/`, `CI checks`, `Release checklist and install docs`. F-8.6 to F-8.10 and F-3.9 are allocated to them. The functions stay in the functional model as life-cycle functions (CR-4.17) | (a) Add the build blocks to the §2 diagram: one picture, but it mixes two life-cycle phases and breaks one question per diagram. (b) Leave the functions unallocated (FG-1 stays open). (c) Drop the functions from the model: loses the verification path for NFR-6, RISK-1 and PLAT-2 | DECIDED (delegated) 2026-10-28 |
| D-32 | Allocation granularity and the desktop adapter (FG-2, FG-3, FG-4) | **§2.2 is a list of normative allocation names**: the §4.1 sub-blocks as `chess-app core :: X`, the three transport blocks (`CLI front-end`, `axum HTTP + WebSocket`, `Tauri shell and IPC adapter`), the data store, one `stockfish`, and the enabling-system blocks. The Tauri shell becomes a block in §2. The three engine boxes in §2 are one block, `stockfish` | (a) Keep `chess-app core` as one block: the allocation cannot tell the services apart. (b) Allocate to crate names: couples the model to the repository layout. (c) A full container diagram per process: more pictures than a question needs | DECIDED (delegated) 2026-10-28 |
| D-33 | In-play action rules: takeback, draw offer, move now, resume list (CR-4.4, CR-4.6, CR-4.7, CR-4.8) | As written in §4.4. Takeback stops at the human's first turn and removes the taken-back moves from the record. All clocks stop in `DrawOffered`. `CancelEngine` is the one command behind `movenow`, F2 and the GUI button, valid only in `EngineThinking` of an h–c game. The resume list is built from `GAME`, not from slots | (a) Keep taken-back moves in the record (rejected by CR-4.6). (b) Let clocks run while an offer is pending (rejected by CR-4.7; it would make a slow answer lose on time). (c) Several commands for move now: more to test, no gain | DECIDED (delegated) 2026-10-28 |
| D-34 | Import and identity rules: PGN duplicates, archive contents, deterministic roster UUIDs (CR-4.1, CR-4.2, CR-4.3, CR-4.11) | PGN duplicates: Skip (default) or Keep both, found by a `content_hash` kept on every game. The archive gains `games.json`; an archive without it still imports. Unfinished games are restored as unfinished when no slot is doubled; a game live in the importing process is never replaced. Roster UUIDs are UUIDv5 of `(roster_seed, roster_index)`, and the name is the first unused candidate at that index | (a) Offer Replace for PGN duplicates (rejected by CR-4.2). (b) Recompute analysis after import and keep `games.json` out (the earlier rule; reversed by DAT-2). (c) Random roster UUIDs: DAT-2 would prompt for every computer when archives cross installs | DECIDED (delegated) 2026-10-28. Changing the UUID derivation after release breaks cross-install matching |
| D-35 | Handling damaged data and backups (CR-4.16, DAT-5, DAT-6, DAT-7) | A failed pre-upgrade backup stops the upgrade. Every store-changing command is all-or-nothing. A damaged journal resumes from the longest intact run held by the journal or the store, or the user voids it with termination "voided: unreadable". `RestoreBackup` works on a copy, replays every retained journal including `journal/done/`, re-finalises games in end-time order, then renames into place. **Architect-added: an automatic backup** at start-up when the newest is older than 24 h (TBD-33), newest 3 kept, so "journals are kept until the next backup" has a meaning (stated in DAT-5 by CR-5.1) | (a) No automatic backups: `done/` journals accumulate until a migration or import, and a restore goes back to the last such event. (b) Restore in place: a failure half-way leaves a mixed store. (c) Refuse to resume a damaged game: loses moves the store still holds | DECIDED (delegated) 2026-10-28. Cadence and restore limit accepted into DAT-5 by requirements Amendment 3 (CR-5.1) |
| D-36 | CLI grammar and server lifecycle (CR-4.12, CR-4.13, CR-4.14) | Headless is c–c and Strict only, stored and rated like any c–c game. `--data-dir` > `CHESS_DATA_DIR` > default. Exit code 4 for a signal. `chess export game <game>` and `chess export data` are distinct (UC-22, UC-24), as are `chess import pgn <file>` and `chess import data <file.zip>` (UC-23, UC-25). `save` prints the stored move count. `chess serve` fails fast on a bad config, survives a browser-open failure, and stops on SIGINT or SIGTERM with exit 0 after suspending open games | (a) Keep one `export` and one `import` and tell the kind by the argument: ambiguous for a file name that is both. (b) Scripted runs unrated (needs an ELO-2 exception; the user may still prefer it) | DECIDED (delegated) 2026-10-28. After release, command names are a scripting contract |
| D-37 | Elo chart in the CLI (H-14, UI-2) | ratatui `Chart` with a Braille marker plus a table of the latest points; an ASCII-grid renderer in the ASCII style; plain mode prints the table and a sparkline | (a) Table only: fails UI-2's chart. (b) Render an image: no terminal support to rely on | DECIDED (delegated) 2026-10-28. D-21 stays deferred for the GUI |

### 11.1 D-4: the GUI board component (AGREED 2026-10-25)

**Decision.** chessground plus our own shared keyboard and accessibility layer (Option C in the [refined trade study](trade-study-gui-board-refined.md)). The user ruled out building our own board for v1. The [first-pass study](trade-study-gui-board.md) is kept for the record. **Fallback:** react-chessboard with the same layer and a small patch removing its built-in keyboard drag (study Option D1, not decision D-1), if the spike below fails.

**Why.** C and D1 scored nearly level (88.8 vs. 87.4). C was chosen because:
- the main reason for using a library is mature pointer interaction, and chessground's is proven on Lichess;
- C's weak spot (selection reconciliation) is our own testable code, while D1's (a carried patch, a single maintainer, a pending drag-engine migration) are external;
- chessground is inert to focus and assistive technology as shipped, so it needs no modification.

**Design.**

```mermaid
flowchart TB
    props["BoardProps<br/>(fen, orientation, legalMoves, interactive,<br/>lastMove, check, arrows, theme, onMove)"]
    model["BoardModel reducer (pure TS)<br/>cursor · selected · pendingPromotion · announcement"]
    overlay["AccessibleOverlay<br/>transparent CSS grid over the board<br/>role=grid · 64 labelled gridcells · roving tabindex<br/>visible focus ring · pointer-events: none"]
    live["LiveRegion (aria-live)"]
    picker["PromotionPicker"]
    subgraph adapter["chessground adapter (D-4, chosen)"]
        map["toConfig(props, model)<br/>fen · orientation · movable.dests · lastMove · check ·<br/>selected · drawable.autoShapes · viewOnly · jsHover ·<br/>theme CSS classes"]
        cg["chessground instance<br/>(aria-hidden=true)"]
        rec["Reconciler<br/>events.select → PointerSelect<br/>events.change → read api.state.selected<br/>movable.events.after → PointerMove"]
    end
    fallback["Fallback adapter (only if the spike fails)<br/>react-chessboard + patch (study option D1)<br/>same BoardProps, same shared layer"]
    props --> model
    props --> map
    model --> map --> cg
    cg --> rec --> model
    overlay -- "keys" --> model
    model --> overlay
    model --> live
    model --> picker
    model -- "move complete" --> onMove["onMove → MakeMove{ply,…}<br/>(rejected move: core keeps fen → piece reverts)"]
    adapter -. "swappable behind the Board contract" .- fallback
```

- **`Board` contract.** Screens use the board only through `BoardProps` (refined study §5, first-pass §5.0), so the fallback replaces just the adapter.
- **One selection state.** `BoardModel` is the source of truth. Its selection is pushed into chessground with the `selected` config value. chessground's own changes come back through `events.select` (including drag start). Silent unselects, which fire no event, are caught on `events.change` by reading `api.state.selected`.
- **Accessibility overlay.** A transparent ARIA grid sized from chessground's `---cg-width`/`---cg-height` CSS variables sits exactly over the squares.
  - Keyboard focus and the VoiceOver cursor land on the real squares.
  - Mouse and touch pass through to chessground.
  - The chessground board is `aria-hidden="true"`, which is axe-clean because chessground has no focusable elements.
- **Moves.** Pointer moves are applied optimistically by chessground and confirmed by the core's next snapshot. A rejected move (e.g., `StalePosition`) is reverted by re-setting `fen`. Keyboard moves go through `onMove` and are animated when the snapshot's `fen` arrives.
- **Read-only.** `viewOnly` while rewound (GAME-3), on computer turns, and in computer–computer games.
- **Spike (2 days, first GUI task).** The exit criteria are:
  1. `BoardModel.selected` equals `api.state.selected` after every step of scripted mixed mouse/keyboard sequences: click→Enter, Space→click, drag→Esc, click elsewhere, rejected-move revert, and promotion by mouse and by keyboard.
  2. axe is clean.
  3. The overlay aligns in both orientations at three sizes.
  4. Drag works in Safari, Chrome, Firefox, Edge, and the Tauri WKWebView.
  5. The overlay stays aligned during continuous window resizing down to a narrow width (no GUI minimum size, Q55).

  If criterion 1 can't be met reliably, switch the adapter to study option D1 (about 3–4 days; the shared layer carries over).
- **Upstream.** Propose a configurable sensor list to react-chessboard, to keep the fallback patch-free.

### 11.2 Decision log

| Date | Decision | IDs |
|---|---|---|
| 2026-10-24 | Rust backend, TypeScript UI, Tauri desktop, one shared core. | inputs |
| 2026-10-24 | CLI is a full-screen ratatui TUI, keyboard only, with plain and headless modes. | D-6 |
| 2026-10-25 | Accepted recommendations: in-process core with Tauri IPC (D-1); each entry point embeds the core, coordination via SQLite (D-2); `shakmaty` + `pgn-reader` (D-3); `ts-rs` (D-7); `chess serve` started by the user (D-8); pinned, bundled Stockfish downloaded at build time (D-9); GPL-3.0-or-later (D-10); legal-move lookups via the core (D-11). | D-1–D-3, D-7–D-11 |
| 2026-10-25 | React + TypeScript + Vite for the GUI. | D-5 |
| 2026-10-25 | Offline operation for desktop, web, and CLI made an explicit, tested v1 requirement; architecture confirmed to have no run-time network dependency (§8.1). | PLAT-3 |
| 2026-10-25 | v1 will not build its own board component (user decision after the first-pass trade study). | D-4 |
| 2026-10-25 | GUI board = chessground + shared keyboard/accessibility layer, gated by a 2-day spike; fallback react-chessboard + layer (patched). | D-4 |
| 2026-10-26 | Architecture review: D-12, D-13, D-16, D-18–D-20, D-22 PROPOSED; D-21 DEFERRED; D-14, D-15, D-17 OPEN (user-visible rules, need the user's choice). No AGREED decision was changed. | D-12–D-22 |
| 2026-10-26 | User answers Q41–Q55: D-15 decided (clock keeps running), D-17 decided (merge). D-23–D-27 added from Q46–Q49, Q54, Q55. D-14 is still OPEN. | D-15, D-17, D-23–D-27 |
| 2026-10-26 | Second round of user answers: D-14 decided (suspend immediately). D-16 simplified to one bundle with mismatch refusal. D-18, D-25, D-26, D-27 revised. D-28 (two Elo tracks) and D-29 (export archive) added. Human abort removed in favour of Resign. | D-14, D-16, D-18, D-25–D-29 |
| 2026-10-26 | Final answers: every PROPOSED decision accepted and AGREED (D-12, D-13, D-16, D-18–D-20, D-22–D-24, D-26–D-29, plus the mechanisms in D-17 and D-25). D-12 and D-16 recorded as one-way for data, consequences accepted by the user. D-26 revised: old games stay in opponents' views (Q76). D-27 revised: an h–c game never uses the computer's slot (Q77). D-28: computers' Training Elo, K counted per track (Q78), label from the Strict Elo (Q79). ASM-1 CONFIRMED. D-21 stays DEFERRED. Architecture baselined as v1.0. | D-12–D-29 |
| 2026-10-27 | v1.0 amendment 1 (user decisions). Reset with unfinished games allowed: they are voided and count against neither player (D-30 added, D-26 revised, OQ-8 and G-20 closed, `UnfinishedGameExists` kept for setup only). The voided game's visibility is an architect interpretation (Q-A18). Stockfish 19 (`sf_19`, 2026-09-05) pinned as the latest release (D-9 revised, TBD-26 closed); build plan prefers the official universal macOS asset, checked with `lipo -info`, with the separate-builds plan as fallback; ASM-3 narrowed. No ID renumbered. | D-9, D-25, D-26, D-30 |
| 2026-10-28 | v1.0 amendment 2 (requirements Amendment 2 / CR-4; decided by the architect under the user's delegation, the user may correct). Enabling-system view and allocation blocks (D-31, D-32). In-play rules (D-33). Import, identity and archive rules (D-34). Damaged-data handling and automatic backups (D-35). CLI grammar and server lifecycle (D-36). CLI Elo chart (D-37). D-17, D-18, D-19, D-21, D-24, D-29 amended. Name rules corrected in §5.5. No ID renumbered. | D-17, D-18, D-19, D-21, D-24, D-29, D-31–D-37 |
| 2026-10-28 | v1.0 amendment 3 (requirements Amendment 3 / CR-5; consistency pass by the systems engineer). No decision added or changed. Automatic backup and restore limit accepted into DAT-5 (D-35, R-19, OQ-12 closed); TBD-21 closed with exit code 4; TBD-33 added to the requirements register; CLI additions and `--config` accepted as design or test-only; R-20 accepted; §4.1 and §10.4 aligned with the functional decomposition (F-2.5.7 on `chess-app core`, F-6.15 automatic backup); §10.2 traces `engine_version` and `content_hash`. Text-only: no diagram changed. | D-35, D-36, R-19, R-20, OQ-12 |
| 2026-10-28 | v1.0 amendment 4 (requirements Amendment 4 / CR-6; the user's answers to the readiness check). No decision added or changed. Criticality and coding standard recorded (§8, §10.2). Q-A18 answered. Design approvers recorded. No Intel test hardware: PLAT-2, D-25 and R-13 unchanged. | Q-A18, §8 |
| 2026-10-30 | v1.0 amendment 5 (requirements Amendment 5 / CR-7; the user's answers to the open readiness findings). No decision added. Integer ratings (§7.2, ELO-1); standard-conforming PGN export (§5.7); baseline versions (§3.4); NFR-8 budget a Should (§4.8); CERT-informed coding standard, local-only security and CVE scanning (§8); best-effort platform matrix (§9). D-25 and R-13 unchanged. | ELO-1, IF-1, NFR-7, PLAT-2 |
| 2026-10-30 | v1.0 amendment 6 (requirements CR-8; FIND-008). G-15 resolved; D-20 amended: source, snapshot, code and name, position matching. | DAT-3, NFR-6 |

### 11.3 Door type, consequences, and revisit triggers

One-way door = costly to reverse: weeks of work or data migration. Rationale for D-1–D-11 is in the table above and in §11.1. D-4 is gated by its spike.

`GATE-005 | Subject: amendment 2, D-31 to D-37 | Scores: blast 1, reversibility 2, novelty 1, regulatory 0, cost 1, data 1, coupling 1 = 7 | Tier: R1 | Overrides: none.` R1 asks for the architect's checklist, two options noted per decision, and an ADR for any one-way door. The options are in the table above, and the one one-way door (UUID derivation after a release, D-34) has its escape route in the table below. The checklist was run by the architect against the guardrails and the definition of done; no separate Review Board pass was held at R1. **Human sign-off is not claimed.** The systems engineer and the user may correct any entry marked DECIDED (delegated).

| ID | Door | Main consequences | Revisit trigger | Escape route |
|---|---|---|---|---|
| D-1 | Two-way | No TCP port for desktop. The desktop E2E can't drive the app exactly like the web (R-1) | Tauri gains macOS WebDriver, or desktop/web divergence bugs appear | Switch the desktop to the local server transport; `CoreClient` already abstracts it |
| D-2 | One-way (moderate) | Engine pool per process. No live cross-process updates. Leases needed (R-3, R-9) | A need for live cross-process sync, or contention bugs | Extract a daemon that hosts `chess-app`. The front-ends already speak the protocol |
| D-3 | One-way (moderate) | GPL dependency (fits D-10). Rules quality inherited, and perft-verified | A blocking bug or license change upstream | Own implementation behind the `chess-rules` API |
| D-4 | Two-way (behind the `Board` contract) | Our Reconciler carries selection-sync risk (R-8) | Spike fails, or a chessground upgrade breaks the Reconciler | Study option D1 adapter (3–4 days) |
| D-5 | One-way (UI rewrite) | Ecosystem for testing and accessibility | — (low likelihood) | — |
| D-6 | Two-way | Plain mode must stay first-class (R-6) | Screen-reader users report TUI problems | Plain mode already exists |
| D-7 | Two-way | CI staleness check needed | Tauri-specific typing needs | `specta` |
| D-8 | Two-way | The user must start `chess serve` | The user wants the URL to "just work" | launchd agent |
| D-9 | Two-way | Builds need the Internet once (§8.1). Stockfish 19 is a major release: **any later Stockfish version needs its own decision-log entry and a full calibration re-run** (§4.6), because strength, the `UCI_Elo` range, and the network change between releases. Engine parameters of existing computer Players stay fixed (PLY-4), so a version change can shift their real strength | Fully offline builds wanted; a Stockfish release fixes a bug that affects us; the official macOS asset fails the `lipo -info` check (use the fallback build) | Vendored archive; fallback build from source (§8.2) |
| D-10 | One-way | All code GPL-3.0-or-later | A non-GPL dependency is needed | — (relicensing requires all authors) |
| D-11 | Two-way | Round trip per what-if move | Measured latency is noticeable | WASM `chess-rules` |
| D-12 | One-way (data). **Accepted by the user, 2026-10-26** | SQL stats views, WAL multi-process. Every entry point and the journal design depend on it, and the user's data lives in its format | Corruption or contention under real use | Full-data export and import (DAT-2) into a new store |
| D-13 | Two-way | One store thread per process: writes are serialised per process | Store-thread queueing visible in latency benchmarks | Connection pool or `sqlx` behind the `Store` trait |
| D-14 | Two-way | Closing a tab pauses that game's clock, the same as quitting the app | User feedback | Add a grace period |
| D-15 | Two-way (code) | A computer can lose on time while its engine recovers | User feedback | Freeze clocks (option A) |
| D-16 | One-way (data). **Accepted by the user, 2026-10-26** | One bundle only; partial installs unsupported. A store migrated by a newer build can never be opened by an older one (`SchemaTooNew`), so a downgrade means restoring the pre-migration backup or exporting and re-importing | A need to ship the CLI separately, or to downgrade | Restore `chess.db.bak-v<N>` (journals replay on top), or export and import; add compatibility rules for a separate CLI |
| D-17 | One-way for user data (a merged store can't be un-merged, except from the pre-import backup) | Records need stable `uuid`s; Replace/Keep-both UI | Merge errors reported | Restore the automatic backup |
| D-18 | Two-way | Install script maintained | Distribution beyond one Mac | Homebrew formula |
| D-19 | Two-way | Ordered rule must be documented in help | Confusion in scripts | Explicit `--headless` only |
| D-20 | Two-way | Bundled data file and its license (CC0-1.0, chosen). Where each name in the dataset originally came from was not checked | Data quality issues, or the upstream license changes (a snapshot already taken is unaffected) | Swap the table |
| D-21 | Two-way | — | — | — |
| D-22 | Two-way | Any local process can drive the core | ASM-1 changes (shared Mac accounts, or the app is distributed) | Add a token |
| D-23 | Two-way (behind `GameSession`) | One extra fsync per move. Recovery code paths need failure-injection tests (R-15) | Journal and DB bugs, or measured latency over the NFR-8 budget | SQLite FULL only |
| D-24 | Two-way | One more file to document (NFR-5) | User prefers XDG | Read both locations |
| D-25 | One-way-ish (support commitment) | Re-testing on every new macOS release. Intel test hardware while Intel is supported. Two Stockfish slices | A macOS release drops Intel (expected after Tahoe, R-13) | Freeze Intel at its last macOS; arm64 continues |
| D-26 | Two-way (no data destroyed) | Queries for the reset Player filter by `reset_at`; opponents' queries don't. Reset points in `RATING_HISTORY`. Unfinished games are voided, not refused (D-30) | User wants a physical wipe | Add deletion |
| D-27 | Two-way | `StartGame` needs the DB. Computers are never blocked by h–c games, so one computer can be in many unfinished h–c games at once (one per human per mode) | Slot rules change | Change which participants insert slots |
| D-28 | One-way (data model; splitting history later is hard) | Two rating series and two K counters per Player in stats and charts | — | Merge tracks in views |
| D-29 | Two-way (versioned) | Exporter and importer per `format_version` | — | New format version |
| D-30 | Two-way as a rule (code in `PlayerService` and queries). Each void is permanent for that game: it can't be resumed or given a result afterwards, except by restoring the pre-reset backup | `PlayerService` becomes a second writer of `GAME.status` and `UNFINISHED_SLOT`. Cross-process sessions learn of the void within one lease heartbeat (2 s, `ESTIMATE`). New state, event, journal record, status value, and PGN tag | The user corrects the interpretation (Q-A18), or users report voids they didn't expect | Change visibility in queries only (no data change). Stronger warning in the confirmation |
| D-31 | Two-way | One more view to keep current | The release procedure changes shape (hosting, notarization) | Merge the blocks into §2 |
| D-32 | Two-way | Names must match functional_decomp and requirements | A rename of a service | Update §2.2 and the allocation cells |
| D-33 | Two-way (code and rules) | A draw offer stops the clocks, so an unanswered offer in hot-seat stops both (R-20) | User feedback on the rules | Change the Policy rows and the journal handling |
| D-34 | Two-way, except UUID derivation: **one-way once a release exists** (stored identities) | `content_hash` on every game; deterministic roster UUIDs | A release ships; cross-install matching needs a change | Migration that rewrites computer Player UUIDs |
| D-35 | Two-way (code) | Restore on a copy; automatic backups use disk (a few MB, `ESTIMATE`); a design limit for Players created after the backup (R-19) | Backup size or restore time complains | Change the cadence or retention |
| D-36 | Two-way before release, a scripting contract after | Exit code 4 and command names are public | Scripts in use | Keep old names as aliases |
| D-37 | Two-way | Two renderers (Braille chart, ASCII grid) to snapshot-test | Terminal rendering problems | Table and sparkline only |

## 12. Risks

| ID | Risk | Mitigation |
|---|---|---|
| R-1 | **`tauri-driver` (WebDriver) does not support macOS**, so "same E2E suite against the desktop shell" (GUI-1) can't use the standard Tauri E2E path on our only platform. | Run the full GUI suite in Playwright **WebKit** (same engine family as WKWebView) using the Tauri transport against a test harness, plus a scripted smoke test of the real `.app`. (D-1's rejected alternative — loading the UI from the local server — would have let the desktop app be driven exactly like the web app.) Revisit if Tauri adds macOS WebDriver support. |
| R-2 | Beginner/Novice strength (400/600) is below Stockfish's limiter; calibration may take several iterations. | Personality parameters live in data, not code, and `chess calibrate` runs nightly on the M4 baseline. Generator bounds are tuned **before v1**. After release, engine parameters never change (PLY-4). Ratings are dynamic (Q41), so a Player whose parameters play weaker or stronger than its category simply drifts to its true rating. Calibration error therefore shows up as rating drift, not as a defect. |
| R-3 | Concurrent writers across three processes. | Short transactions, WAL, `busy_timeout`, leases; concurrency integration test in CI. |
| R-4 | GPL obligations if the app is ever distributed. | Choose GPL-3.0-or-later now (D-10); license audit in CI. |
| R-5 | Localhost web server reachable by other local apps or malicious web pages. | Loopback bind, strict Host/Origin checks, no CORS. Optional per-launch token in the URL if we want more. |
| R-6 | Full-screen TUIs read poorly with screen readers, and terminal state can be left broken on crashes. | Plain mode is a first-class, tested mode (automatic off-TTY); restore guard + panic hook + signal handling, each covered by a PTY test. |
| R-7 | Unicode chess glyph widths vary across terminal fonts, which can misalign the board. | Fixed cell width per square with padding; plain ASCII style always available; snapshot tests per style; manual check in Terminal.app and iTerm2. |
| R-9 | Builds of different versions share one DB (an upgraded CLI next to an old `Chess.app`), so an old build could misread a migrated schema. | D-16: `SchemaTooNew` guard, migrate only when alone, one build version for all artifacts; fixture test. |
| R-10 | A closed tab or a dropped WebSocket now suspends the game at once (D-14), so a brief network-free reload or a crash of the browser pauses a timed game. A player could also close the tab to stop their own clock, exactly as quitting the app allows (RULE-5). | Accepted, because it is the user's rule. Suspension and resume are journalled (§7.6). Reconnect resumes automatically, so a reload costs only the reload time. |
| R-11 | Synchronous `rusqlite` calls on async workers stall the actors and the TUI loop (clock display, NFR-8 responsiveness). | D-13 store thread. A lint or review rule against `Store` use outside the store thread, and a latency test with a slow fake store. |
| R-12 | Several processes running engines at once (e.g., a c–c game in the CLI and a game in `Chess.app`) compete for CPU. Computer strength and move times drift from calibration (ENG-2, ENG-3). | Fixed `Threads`/`Hash` per instance; background analysis at low priority; time limits by `movetime`, not depth, for clocked games; a soak test with two processes. |
| R-8 | chessground's internal selection drifts from `BoardModel` in an unanticipated input sequence (it fires no event on unselect). | Single Reconciler module; reconciliation on `events.select`/`events.change`/`after`; scripted mixed-input Playwright suite; spike exit criterion; fallback to react-chessboard (study option D1) behind the same `Board` contract. |
| R-13 | **Intel support depends on Apple.** The app must support Tahoe and every later macOS (user, 2026-10-26), but later releases may drop Intel Macs. Apple said at WWDC 2025 that Tahoe is the last macOS release for Intel Macs (from memory, `Unverified` online; re-check at each app release under the PLAT-2 release checklist). Mirrors requirements RISK-1. | Keep the universal build (D-25). When a macOS release drops Intel, Intel support stays frozen at the last macOS that runs on it, and arm64 continues. Re-test on each new release (§8.2). Keep one Intel Mac for the platform matrix while Intel is supported. **No Intel Mac is available, so Intel is built but its testing is deferred and best effort (CR-7.6).** |
| R-14 | Intel Macs and M1–M3 Macs search fewer nodes in the same `movetime`, so computer Players play weaker there than on the M4 baseline, and timeouts tuned on M4 may be tight. | Timeouts have generous grace values (§4.8). Dynamic ratings (Q41) absorb the strength difference over time. Engine integration tests run on the Intel machine too. |
| R-17 | Computer ratings now change (Q41) on two tracks, including through c–c games. Ratings can drift away from real strength, and soak or calibration runs could pollute the user's ratings (mirrors RISK-2). | `chess calibrate`, soak, and every test run against an isolated store (`CHESS_DATA_DIR` set to a temp dir; calibrate refuses to run on the default data dir). Engine parameters stay fixed, so drift reflects real results only. Monitor class-band spread in the nightly report. |
| R-15 | Journal-to-DB recovery (replay, conflict replacement, torn tails, takeover) is new code on the most critical path (no completed move lost). | Failure-injection tests: kill at every write point, corrupt the journal tail, restore an older backup, and contend for the DB from another process. A property test checks that replay is idempotent. |
| R-16 | The TUI's resize request (`CSI 8 t`) can be disabled by the user's terminal settings (e.g., iTerm2) or ignored inside tmux/screen. | Fall back to the "please enlarge" message (§6.1). PTY tests cover both paths. |
| R-18 | A reset voids games of other people too (D-30): in a hot-seat h–h game, one human's reset voids the game in progress for the other human, who may have been winning. | Accepted, because it is the user's rule (2026-10-27). The confirmation names each game, its opponent, and the moves played, and says the game will count for neither player. The opponent's history keeps the game, marked "voided by reset", and it can still be analysed. |
| R-19 | A restore cannot rebuild everything a journal does not hold. A journal records the moves of a game, not the registration, rename or reset of a Player. A game that finished after the backup and involves a Player created after it cannot be finalised in the restored copy (D-35, OQ-12). | The game's journal is kept in `journal/unrestored/` and named in the restore message, so nothing is deleted. The automatic backup cadence (24 h, TBD-33) bounds the window. No automatic recovery in v1. Accepted and stated in DAT-5 (CR-5.1). |
| R-20 | In a hot-seat h–h game an unanswered draw offer stops both clocks with no time-out (CR-4.7), so a player can pause a timed game by offering and not answering. | Accepted, because it is the user's rule and the same two people share one screen (nobody is cheated). An offer to a computer is bounded by the ENG-3 maximum think time. Revisit if a networked seat is ever added (§8.4). |

## 13. Next steps

1. Workspace skeleton, then `chess-rules` + `chess-domain` with their unit tests first (the test-first core).
2. `chess-store`, `chess-engine`, `chess-app`, then the CLI (TUI, plain, headless).
3. GUI: start with the 2-day board spike (§11.1) against the `Board` contract, then screens, transports, and the Tauri shell.
4. Close the remaining open items in §14 at the step named there: ASM-2, ASM-3 (closed by the first run of the Stockfish 19 build script), and ASM-4 in step 2; the D-4 spike (OQ-9), the browserslist query (OQ-10), the Tauri sidecar naming (OQ-11), and the TBD-18 wording in step 3. TBD-26 was closed on 2026-10-27 (Stockfish 19).
6. Amendment 2 (2026-10-28): the systems engineer reviewed §10.4 and the items marked **SE note** and **proposed** (amendment 3, CR-5, all accepted): the automatic backup cadence (D-35, TBD-33), the DAT-5 limit for Players created after a backup (R-19, OQ-12), exit code 4, and the UI-12 list (the void-unreadable dialog). ASM-5 (TBC-10) is closed by the M walkthrough at step 3. The build-and-release blocks (§2.1) are created at step 3 with the packaging work.
5. ~~The systems engineer confirms OQ-8 (reset with an unfinished game).~~ Done 2026-10-27: the user decided (D-30). The systems engineer adds the void rule to PLY-7 (and its verification), updates TBD-24 (`EndStatus`) and TBD-26 (Stockfish 19) in requirements.md, and fixes any requirements.md inconsistencies that remain. The user may correct the D-30 visibility interpretation (Q-A18).

## 14. Assumptions and open items

| ID | Type | Statement | Validation / owner | Blocks |
|---|---|---|---|---|
| ASM-1 | Assumption, **CONFIRMED** (user, 2026-10-26) | One person uses the Mac account. Other local users and processes are not adversaries, so the loopback server needs no access token (D-22) | Confirmed. Revisit if the app is distributed or the Mac account is shared | — |
| ASM-2 | Assumption | The timeout and period values in §4.8 are `ESTIMATE`s for the M4, 16 GB, Tahoe baseline, and the journal flush costs a few ms per move | **Open.** Owner: core developer. Closing step: measure in the engine and core integration tests and the NFR-8 latency test (step 2); change values only through the decision log | — |
| ASM-3 | Assumption (narrowed 2026-10-27) | About the pinned Stockfish 19 (`sf_19`), two facts not verified because the binary could not be downloaded and run here: (1) the official `stockfish-macos-universal.tar.gz` asset contains both arm64 and x86_64 slices; (2) its `UCI_Elo` floor and range. These are requirements.md TBC-7 and TBC-8. The exact GPL version (TBC-9) is read from the shipped `COPYING` in the same step. *Verified 2026-10-27 from the GitHub release API and notes:* version, tag, date, not a pre-release, the asset names, universal binaries with run-time CPU-feature detection, the SFNNv16 network, and the GPL license statement. The embedded network is proven by the offline test (§8.1). Tauri sidecar naming moved to OQ-11 | **Open.** Owner: architect. Closing step: implementation step 2 (engine build script). **The first build-script run closes it:** `lipo -info` answers (1) and selects the preferred or fallback path (§8.2); the `uci` handshake answers (2), and the value is recorded for §4.6 and calibration | Calibration bounds (not the design: §4.6 reads the range at run time), the §8.2 path choice |
| ASM-4 | Assumption | Terminal.app and iTerm2 honour `CSI 8 ; rows ; cols t` in their default settings | **Open.** Owner: CLI developer. Closing step: PTY and manual check (R-16), step 2. The fallback message already covers failure | — |
| ASM-5 | Assumption (added 2026-10-28; requirements TBC-10) | macOS facts behind `chess install-cli` and signing: (a) arm64 executables need at least an ad-hoc signature; (b) a locally built app never downloaded carries no quarantine mark; (c) the steps to open a blocked app on macOS 26; (d) `/usr/local/bin` and `~/.local/bin` behaviour. From general knowledge, **not verified** on a macOS 26 machine | **Open.** Owner: architect. Closing step: the M walkthrough of PLAT-6 on a clean macOS 26 Mac (step 3). The design tolerates a wrong answer: the fallback folder and the failure message cover (d), and the documented steps cover (c) | PLAT-6 wording only |
| OQ-1 | Resolved | D-14: suspend immediately when no front-end is attached (user, 2026-10-26) | — | — |
| OQ-2 | Resolved | D-15: clock keeps running (Q53) | — | — |
| OQ-3 | Resolved | D-17: merge (Q51) | — | — |
| OQ-4 | Resolved | Requirement gaps G-1–G-20 are resolved, (G-15 was resolved 2026-10-30). G-20 was resolved 2026-10-27 (D-30) | — | — |
| OQ-5 | Resolved | All PROPOSED requirements were accepted (Q75), so the §10.1 allocation is final | — | — |
| OQ-6 | Resolved | Q41, Q42, Q54, and Ctrl-C (Q52) are answered and applied (§4.4, §4.6, §6.1, §7.1, §7.6) | — | — |
| OQ-7 | Resolved | GUI light/dark theme: defined by Q52 (§6.2) and backed by UI-10 | — | — |
| OQ-8 | Resolved 2026-10-27 | Reset with an unfinished game. *Baseline rule (superseded): the reset was refused.* User decision: "allow it; the unfinished game does not count against either player." Each unfinished game is voided in the reset transaction (D-30, §7.2). Systems engineer to add the rule to PLY-7 (G-20) | — | — |
| OQ-9 | Measurement | The D-4 board spike (§11.1) | **Open.** Owner: GUI developer. Closing step: run the 2-day spike as the first GUI task (step 3). On failure, switch to the study option D1 adapter | D-4 adapter |
| OQ-10 | Measurement | The browserslist query for "the last 3 years" of the four browsers (§8.2) | **Open.** Owner: GUI developer. Closing step: check with `npx browserslist` when the build is set up (step 3) | — |
| OQ-11 | Check | Tauri's sidecar naming rule for a universal target (split out of ASM-3 on 2026-10-27, §8.2) | **Open.** Owner: architect. Closing step: check against the pinned Tauri version when packaging is set up (step 3) | — |
| OQ-12 | Resolved 2026-10-28 | What a restore cannot recover: a game that finished after the backup and involves a Player created, renamed or reset after it. Resolved by requirements Amendment 3 (CR-5.1): DAT-5 states the limit and the automatic backup. Registration is not journalled | — | — |

**Follow-up questions for the user (Q-A).** IDs are never reused. Answered questions stay listed with their answer.

| ID | Question | Status / working default | Where |
|---|---|---|---|
| Q-A1 | Tab closed during a timed game? | **Answered:** suspend immediately (D-14) | §4.3 |
| Q-A2 | One unfinished game per machine or per Player? | **Answered:** per Player; c–c counts for each computer (D-27) | §7.2 |
| Q-A3 | Case-variant names duplicates? | **Answered:** no. Names are unique case-sensitively, and identity is the UUID | §7.2 |
| Q-A4 | What does a reset clear? | **Answered:** everything except Elo, humans only (D-26). See Q-A15 | §7.2 |
| Q-A5 | Do c–c games change computer Elo? | **Answered:** yes, both computers (Strict track) | §4.4 |
| Q-A6 | Hot-seat abort / c–c abort? | **Answered:** humans only Resign (h–h asks which side). The watcher's c–c abort has no effect | §4.4 |
| Q-A7 | Computer category label? | **Answered:** follows its current Strict rating (Q79). Engine settings fixed | §4.6 |
| Q-A8 | Port scope? | **Answered:** `chess serve` and the browser URL only | §8.2 |
| Q-A9 | Computer K-factor? | **Answered:** same as humans | §4.6 |
| Q-A10 | Aborted Strict game in W/L/D? | **Superseded:** human games have no abort; a resignation counts in Elo and W/L/D | §7.2 |
| Q-A11 | Terminal bell for sound? | **Superseded:** no sound in v1 | §6.2 |
| Q-A12 | Do you accept the PROPOSED decisions D-12, D-13, D-16, D-18–D-20, D-22–D-24, the mechanisms in D-26–D-29, and ASM-1? | **Answered:** yes, all AGREED 2026-10-26. D-12 and D-16 are one-way for data, consequences accepted. ASM-1 CONFIRMED | §11 |
| Q-A13 | Whose max-think time in c–c and h–h? | **Answered:** the active profile's. In h–h it applies to engine analysis only | §4.6 |
| Q-A14 | macOS releases after Tahoe? | **Answered:** Tahoe and every later release must be supported. Intel only as long as Apple supports it (R-13) | §8.2 |
| Q-A15 | After a reset, should the Player's old games be hidden from their own views, or deleted outright, which also removes them from opponents' history? | **Answered:** hidden. They stay in opponents' history and stats and are hidden only from the reset Player's views (D-26, Q76) | §7.2, D-26 |
| Q-A16 | In a Strict h–c game, does the computer also use up its one unfinished Strict slot? | **Answered:** no. Human–computer games never use a computer's slot, so any Player can always play any computer. Computers hold slots only in c–c (D-27, Q77) | §7.2, D-27 |
| Q-A17 | Do computers also get a Training Elo that changes in Training games against humans? | **Answered:** yes. The K = 40 count is per track (Q78), and the label follows the Strict Elo (Q79) | §4.6, D-28 |
| Q-A18 | A voided game (D-30): is the architect's reading right? It stays in the opponent's history marked "voided by reset", with no result, and can be analysed; it is hidden from the reset Player's own views; it is not a resignation, so no loss is recorded | **Answered 2026-10-28 (user, readiness FIND-003):** yes, the reading stands. A voided game stays in the opponent's history marked "voided by reset", has no result, can be analysed, is hidden from the reset Player's own views, and records no loss | §7.2, D-30 |
