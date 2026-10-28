# UC-27: Run Scripted Headless Game

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-27_run_scripted_headless_game.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants a script to run a complete game from command-line flags with no prompts, and to get the finished game as PGN and a usable exit status.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a script author or operator of the CLI

**Supporting Actors:** Stockfish engine (block `stockfish`, bundled and pinned; driven over UCI)

**Stakeholders and Interests:**
- Script author: stable output and exit codes (CLI-5).
- Project owner: the same scripted game gives the same final position and PGN in all three entry points (PLAT-1).
- User: scripted runs must not change the ratings of their real Players unintentionally: `--data-dir` selects an isolated store (RISK-2, CLI-4).

**Preconditions:** Chess is installed. Setup flags describe a valid game, for example `chess --matchup cvc --mode strict --white <computer> --black <computer> --clock 5+0 --no-color` (CLI-4).

**Minimal Guarantees:** On invalid setup nothing is written to standard output, a message goes to standard error, and the exit status is non-zero (CLI-5).

**Success Guarantees:** The finished game is on standard output as PGN (IF-1) and the process exits with status 0.

**Trigger:** The User runs `chess` with scripting flags.

**Main Success Scenario:**

1. User runs `chess` with setup flags.
2. System checks the setup, without any prompt and without profile selection, against the same rules as UC-5.
3. System plays the game without starting the TUI, asking Stockfish for each computer move.
4. System detects the end of the game and records it.
5. System writes the finished game as PGN to standard output.
6. System exits with status 0.

**Extensions:**

- **2a.** The setup is invalid:
  - 2a1. System writes a message to standard error, nothing to standard output, and exits non-zero (CLI-5). Use case fails.
- **2b.** A chosen Player has an unfinished game that blocks the start:
  - 2b1. System fails as 2a, with exit status 1 and no prompt (CLI-4, CLI-5). Headless mode never offers resume.
- **2c.** The setup names a human Player:
  - 2c1. System fails as 2a (CLI-4): a human side needs input that a script cannot give. Use case fails.
- **3a.** The engine fails or the game cannot be completed:
  - 3a1. System exits non-zero (CLI-5, ENG-4): code 2 when the engine is unavailable, 3 on a store error. The game stays resumable if moves were stored. Use case fails.
- **3b.** The run receives SIGINT or SIGTERM:
  - 3b1. System leaves an unfinished game that can be resumed as a watched game (RULE-5, UC-9), prints no PGN, and exits with code 4 (CLI-4, CLI-5). Use case fails.

**Technology and Data Variations:**
- Exit codes (TBD-21, closed 2026-10-28, CR-5): 0 result, 1 usage or setup error, 2 engine unavailable, 3 store error, 4 stopped by a signal (architecture §6.1, D-36).
- A scripted run is a normal computer–computer game: it is stored, takes the computers' c–c slots, is rated by ELO-2 (both Strict ratings change unless one computer plays itself) and counts in W/L/D (CLI-4). `--data-dir <path>` (or `CHESS_DATA_DIR`) selects another data directory, created with the roster if absent; tests, calibration and soak runs use it (RISK-2). A run stopped by a signal leaves an unfinished game and a non-zero status (RULE-5).
- Human-involved games cannot be headless, because there is no input channel (CLI-4). A script that must drive a human side (for example the SAN list of PLAT-1) runs the plain mode with the moves piped to standard input (CLI-1, CLI-4), which is not this use case.

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- PLAT-3: works offline. NFR-3: soak runs of computer–computer games are headless.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | NFR-1: CLI startup 1 s. |
| Channels to actors | Command line, standard output and standard error. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | None |
| Open issues | None. UG-13 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | CLI-4, CLI-5, PLAT-1, PLY-2, GAME-1, ENG-4, RISK-2, NFR-1 |
| Functions (functional_decomp v0.3) | F-7.1.4, F-2.1.1, F-3.7 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: scripted runs are computer-only, stored and rated, with `--data-dir` for isolation (CLI-4, CLI-5; UG-13, A-5 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. Exit codes closed (TBD-21), signal extension 3b, scripted human play via plain mode. |
