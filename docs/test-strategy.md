# Test Strategy (technology-neutral until architecture is chosen)

Per-requirement test cases live under each requirement in [requirements.md](requirements.md). This file covers the overall approach. Status: updated 2026-10-24 to match the agreed requirements.

## Principles
1. **Rules logic is pure and heavily unit-tested.** Move generation, SAN parse/format, Elo, clocks, PGN/FEN, mode policy, and computer-Player generation take data in and return data out — no I/O, no UI.
2. **Engine behind an interface.** Flow/UI tests use a deterministic fake engine; the real engine (Stockfish, UCI) is used only in integration, calibration, and strength tests.
3. **One scenario, three front-ends.** Core scenarios are written once as scripted setups, move lists, and expected outcomes, then replayed through the CLI (PTY), web (browser automation), and desktop (same GUI suite against the desktop shell).
4. **Every requirement ID maps to at least one automated test or an explicit manual one.** Test names/tags carry the requirement ID so a traceability matrix can be generated.
5. **Policy is tested as a table.** Play-mode rules (Training vs. Strict) are a single policy object tested for every action in every mode and matchup, and each front-end is tested to expose exactly what the policy allows.

## Levels
| Level | Scope | Examples |
|---|---|---|
| Unit | Pure logic | perft counts, SAN parser tables, Elo calc (K=40→32 switch, no clamping, self-play/c–c/Training exclusions), draw detection, clock logic with fake time, mode-policy table, class-band mapping, roster generation determinism, variation tree, ASCII renderer snapshots |
| Property / fuzz | Invariants | random legal playouts never reach an illegal state; `parse(format(move)) == move`; arbitrary CLI input never crashes; rewind/live round-trips preserve game state |
| Integration | Component + real dependency | engine via UCI, data store with a real DB file, migrations, concurrent access from CLI + web server + desktop, engine-failure recovery |
| End-to-end | Whole app per front-end | PTY-driven CLI sessions; browser tests; desktop shell smoke; new-game setup combinations (GAME-1) |
| Visual regression | GUI themes | screenshot diffs per board/piece theme (≥10 board, ≥5 piece sets), light/dark |
| Calibration / strength (slow, nightly) | Engine classes | head-to-head matches show class ordering; measured Elo per class within tolerance of the assigned Elo, including the low-skill Beginner/Novice handicaps; 1000-game computer–computer soak |
| Manual | Judgment calls | VoiceOver, clean-Mac install, look-and-feel of themes |

## Agreed-behaviour test focus areas
These cross-cutting behaviours came out of the requirements and need explicit coverage in every front-end:

| Area | Key checks (requirement IDs) |
|---|---|
| Play modes | Training only for human–computer; Strict default; mode fixed at start and stored (PLY-5, GAME-1) |
| Strict-mode integrity | **No engine analysis during play:** no control/command exists, and a spy on engine calls proves the engine is never queried on behalf of a human in an in-progress Strict game; post-game analysis available (ANA-1, ANA-3) |
| Rewind | Read-only; moves rejected while rewound; return to live restores state; record unchanged; clock keeps running; computer move arriving while rewound leaves the view with an indicator; computers cannot rewind; hot-seat either human may rewind (GAME-3) |
| Training | Takeback restores prior state; in-game analysis and what-if never alter the game; always untimed; Elo unchanged (RULE-3, ANA-1/2, ELO-2) |
| Clocks | Strict only; presets + custom; increments; flag fall; pause on app close and resume; computer budgets from remaining time (RULE-4) |
| Elo | Start from self-assessment (400/600/1200/2200/2500); K 40→32 after 30 rated games; no cap; humans only; no change for Training, computer–computer, self-play, custom-FEN, aborted, or imported games; real-valued storage, rounded display (ELO-1/2) |
| Computer Players | 25-Player roster deterministic from seed; stable identity and parameters; unique parameter sets within class bounds; static Elo inside class band; cannot be deleted; draw/resign behaviour (PLY-4, ENG-2, RULE-2) |
| Self-play | Allowed for human–human and computer–computer; unrated; excluded from W/L/D aggregates; present in history (PLY-1, ELO-1, UI-2) |
| Stats | Any Player (human or computer) viewable; aggregates match seeded data; archived Players keep history; Computer Players screen (UI-2, UI-5) |
| Persistence | Shared store across CLI/web/desktop; crash-safe; migrations; export/import round-trip; per-move evaluations computed after the game, never during play (DAT-1/2/3) |
| Notation and input | SAN + coordinate form, case-insensitive, helpful errors (CLI-3); mouse, keyboard, typed, tap input produce identical state (GUI-2) |
| Computer–computer | Only started by an explicit human choice with two computer Player selections (may be the same); pause, step, delay, abort (PLY-1, GAME-2) |

## Reference data
- **Perft suite:** standard positions with published node counts (start, Kiwipete, positions 3–6 from the Chess Programming Wiki).
- **Rule fixtures:** FEN + expected legal moves/results for castling, en passant, promotion, pins, stalemate, insufficient material, automatic threefold and fifty-move draws.
- **Tactics fixtures:** mate-in-1/2/3 and forced wins for engine top-move assertions (Training analysis, post-game analysis).
- **Sample PGNs:** well-known games with all special moves, games with planted blunders, and malformed files.
- **Seeded data stores** at each schema version for migration and stats tests, including self-play, Training, archived-Player, and computer-vs-computer games.
- **Elo vectors:** hand-computed expected results, including a Player crossing game 30 and a Player above 2500.

## Determinism
- Inject clock and RNG; fixed engine seed, single thread, and fixed depth wherever exact moves are asserted.
- Never assert exact engine moves except in forced or tactical positions.
- Roster generation uses the fixed seed so tests and users get the same 25 default Players.

## Quality gates
- All unit and integration tests green on every change; E2E on the main branch; calibration/soak nightly.
- Coverage target for rules, Elo, clock, policy, and parser modules: ≥ 90% line coverage; mutation testing on Elo and mode policy.
- License audit (Stockfish GPLv3 compliance, dependency licenses) and lint in CI.

## Non-functional checks
Startup and engine-latency benchmarks (NFR-1, ENG-3), crash-safety fault injection on the data store (NFR-2), illegal-state fuzzing (NFR-3), network-capture test for no telemetry (NFR-4), fresh-machine install walkthrough (NFR-5), license audit (NFR-6).
