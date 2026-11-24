# Decision package: TBD-36, public CLI surface (exit status and setup flags)

| Field | Value |
|---|---|
| Document | docs/engineering/decision-package-tbd36-cli-surface.md, version 0.2 (**DECIDED 2026-11-09, A1, B1, C1 plus `--version`**; section 12 added; sections 1 to 11 unchanged from 0.1) |
| Date | 2026-11-09 |
| Task | beads chess-a0r.3.16 (baseline B4); blocks chess-a0r.3.17 (CLI code) |
| Author / stance | Systems engineer, **Engineer stance**. No Reviewer pass made. Nothing here is decided or approved |
| Decider | Brian Prascher (G3: public commands and exit codes). The author does not decide. **Ruled 2026-11-09 (relayed by the coordinator, no verbatim quote): A1, B1 with the six sub-rules, C1 plus `--version`; recorded as requirements CR-19; SDD-chess-cli 1.0.1** |
| Gate | Ruling = **G3**. This file is G0 text only; no baselined document was edited |
| Evidence tags | `[OBS]` read in a file, `[INF]` inferred, `[UNV]` not verified. No code was run; the CLI crate is an empty scaffold |

## 1. The ask

Decide three things, in order of importance. Each has a recommendation in section 6; a "yes to all" is possible.

1. **(a)** What exit status the TUI, plain mode and the subcommands return when a signal stops them (SIGINT, SIGTERM, SIGHUP).
2. **(b)** What exit status the non-headless subcommands return (`play`, `install-cli`, `games`, `export`, `import`, `stats`, `players`, `calibrate`; `serve` is already fixed).
3. **(c)** Whether any setup flag is added beyond those already named.

Why now: once released, exit numbers and flag names are a scripting contract (D-36 reversibility row, architecture:2256 `[OBS]`). chess-a0r.3.17 must code the `exit_code` mapping and the tests, and today the SDD tells it to treat everything outside headless as a placeholder (SDD-chess-cli A3, UT-CLI-08, E2E-CLI-03..07) `[OBS]`.

## 2. What is already decided (not reopened)

| Item | Ruling | Source |
|---|---|---|
| Headless exit codes | 0 result, 1 usage or setup, 2 engine unavailable, 3 store error, 4 stopped by SIGINT/SIGTERM | CLI-5, TBD-21 closed (CR-5), D-36 `[OBS]` |
| `chess serve` | exit 0 on SIGINT or SIGTERM; 1 on bind or config error; a browser failure does not change the status | PLAT-4, SDD DD-CLI-9 `[OBS]` |
| `DurabilityUnsupported` | exit class 3 in every run form | CR-12 item 7, SDD A3 `[OBS]` |
| clap usage errors | map to exit 1 (clap's default 2 would collide with "engine unavailable") | SDD DD-CLI-1 `[OBS]` |
| Plain mode EOF with an unfinished game | treated as `quit`, exit 0 (ASM-CLI-3). The SDD also says "the exit status is TBD-36", so this package asks Brian to confirm it | SDD DD-CLI-7, ASM-CLI-3 `[OBS]` |
| TUI confirmed Ctrl-C and `quit` | suspend the game, exit 0 (not a signal stop: SIGINT is a key in raw mode) | SDD DD-CLI-6 `[OBS]` |
| Existing flags | `--data-dir`, `--plain`, `--headless`, `--no-color`, `--matchup --mode --white --black --clock`, `play --player`, `serve --port --no-open --config`, `install-cli --remove`, `export ... --output`, `import ... --on-duplicate --eco --yes` | SDD DD-CLI-1, CLI-4, D-36 `[OBS]` |

## 3. Sub-question (a): exit status on signals in the TUI, plain mode and subcommands

**Current text.** requirements.md CLI-5, "Noted 2026-10-31 (CR-9.6)" (line 988): not decided; no stable status outside headless. SDD DD-CLI-6 (line 477): SIGTERM and SIGHUP restore the terminal and suspend the game; "the placeholder is 4, `ESTIMATE`, not a documented status". A3 table row "signal ... 4 (fixed for headless; placeholder for TUI and plain mode)" `[OBS]`. Desk answer Q-SE-17 proposed 4 `[OBS]`.

**Fact that shapes the options.** In every option the process first restores the terminal and suspends the game through `Core::shutdown`; only the number differs. In a degraded session (D-80) nothing is appended and the game stays "interrupted"; the number is the same `[OBS DD-CLI-6]`.

| Option | Rule | For | Against |
|---|---|---|---|
| **A1** (recommended) | One rule for every form except `serve`: stopped by a signal before finishing means **exit 4** (SIGINT in plain mode, SIGTERM, SIGHUP; also during `import`, `export`, `stats` and the other subcommands) | One number to document and test, already public once headless ships; no new number; matches the SE proposal and the SDD placeholder, so no design rework | Not the shell convention (128+n); a wrapper that expects 143 for SIGTERM sees 4 |
| **A2** | Interactive forms and subcommands use the convention: SIGINT 130, SIGHUP 129, SIGTERM 143. Headless stays 4 (fixed by CR-5) | Familiar to shell users, `timeout`, supervisors | Two schemes in one binary: SIGTERM is 4 headless but 143 in plain mode. Adds three public numbers. Whether to `exit(128+n)` or re-raise the default signal is a further choice (re-raise gives a "killed by signal" wait status; exit(n) does not) `[UNV: macOS behaviour not tested]` |
| **A3** | SIGTERM and SIGHUP in the TUI and plain mode exit 0, like `serve` | The game was suspended cleanly and is resumable; same as `quit` | A script cannot tell "game over" from "killed"; contradicts the headless rule that a signal exits non-zero (CLI-4 (f), CLI-5) |

**Related gaps this ruling should also close (see findings F-2, F-3).**
- Plain mode on SIGINT is not specified (the TUI treats it as a key; plain mode has no raw mode). Proposed: same path as SIGTERM, no confirmation, because stdin may be a pipe.
- SIGHUP in headless and in `serve` is not worded (the A3 table lists it as 4 for headless; DD-CLI-8 and DD-CLI-9 list only SIGINT and SIGTERM). Proposed: headless SIGHUP = 4; `serve` SIGHUP = 0 (a stop request, as PLAT-4 treats SIGINT and SIGTERM).
- Rationale for `serve` staying 0 while the TUI is 4: `serve`'s documented normal stop is a signal; every other form has its own normal stop (`quit`, result, EOF).

**Consequences for tests (verification cases).**
- A1: E2E-CLI-03..07 (PTY, SIGTERM and SIGHUP) add an assertion that the child's exit status is 4; add a plain-mode SIGINT case (PTY and pipe) and a subcommand-in-flight case (SIGTERM during `import pgn` of a large file: exit 4, no partial import, DAT-2 all-or-nothing). UT-CLI-08 row "signal" changes from placeholder to fixed. Remove "exit number not asserted" wording.
- A2: same cases, with a rule on how status is observed (exit code or signal); the harness (`expectrl`) must distinguish the two `[UNV]`. Six assertions instead of one.
- A3: the existing "exit non-zero" wording of CLI-4 (f) needs a carve-out.

**Stored and public contract (G3).** No stored format changes in any option (the journal and `Suspend` record are unaffected). Public: the number(s) in `--help` and README.

**Recommendation: A1. Confidence: medium.** The choice between a single project rule (A1) and shell convention (A2) is a matter of taste for the scripts Brian expects; nothing in the documents shows a consumer that needs 128+n `[INF]`. If Brian expects supervisors such as `launchd` or `timeout` wrapping `chess`, A2 is the better fit.

## 4. Sub-question (b): exit status of the non-headless subcommands

**Current text.** CLI-5 note (line 988): "no other command is given a stable exit status". SDD A3: placeholder map by catalogue code, 0/1/2/3, "not written into help text or README, no test asserts it" (line 386); DD-CLI-10: "Exit codes per A3 (placeholder)"; UF-CLI-6 `[OBS]`. Subcommands in the grammar: `play`, `serve`, `install-cli [--remove]`, `stats [player]`, `players`, `games`, `export game|data`, `import pgn|data`, `calibrate` (hidden), bare `export` / `import` print the four forms and exit 1, `--help` (DD-CLI-1) `[OBS]`.

| Option | Rule | For | Against |
|---|---|---|---|
| **B1** (recommended) | Promote the A3 catalogue map to a **fixed contract** for all run forms: 0 success, 1 usage, setup or refusal, 2 engine unavailable, 3 store error; plus the three sub-rules below. No new number | The map already exists for every catalogue code and UT-CLI-08 already checks coverage; reuses the numbers Brian accepted for `DurabilityUnsupported` | Class 1 is broad (usage, refusal, not found, conflicts); scripts must read stderr to tell them apart |
| **B2** | B1 plus new classes, for example 5 = declined or cancelled by the user, 6 = partial success (some games failed to import) | Scripts can tell "user said no" and "partly done" from usage errors | Adds public numbers and a decision per code; scope creep for a single-user offline tool; the numbers 5 and up are new public surface |
| **B3** | Subcommands use only 0 and 1 | Simplest | Conflicts with the 2026-11-04 ruling that `DurabilityUnsupported` is 3 in every run form; loses store-error versus usage-error |
| **B4** | Keep the placeholder (defer again) | No decision | Blocks chess-a0r.3.17 acceptance ("exit codes") and leaves tests unable to assert numbers; TBD-36 close-by is "before CLI code" |

**Sub-rules proposed with B1 (new text; the documents are silent on them, see F-4).**
1. **User declines a confirmation or cancels** (UI-12 "no", a declined `import data`, Ctrl-C at a prompt in a subcommand that is a key not a signal): nothing changes, exit **1**, stderr "cancelled". Rationale: the requested action was not performed.
2. **PGN import with per-game failures**: games imported before the failure are kept and reported (DD-CLI-10). Exit **0** if every game was imported or skipped as a duplicate; exit **1** if any game failed to parse or import. Skipped duplicates are not failures.
3. **`install-cli`**: every failure (cannot create the link, refuses to replace another file) is exit **1** with the reason and nothing changed (PLAT-6 (b)); it never opens the store, so 2 and 3 cannot occur.
4. **`calibrate`** is hidden from `--help` (RAT-CLI-6); it uses the same map but is **outside the stable contract** and not documented.
5. **TUI and plain mode "exit" after a fatal start-up or engine failure** (ENG-4 retry-or-exit; store refusal) use the class of the catalogue code (2 or 3). A normal `quit` is 0.
6. `--help` and a successful `games`, `players`, `stats` (including an empty list) are 0; `stats <unknown name>` is 1 (already in DD-CLI-10).

**Consequences for tests.**
- UT-CLI-08 changes from "other paths marked placeholder" to "every catalogue code has a fixed row in every run form".
- New or extended cases: CT-CLI-04..12 each assert the number for their error path (they were told to assert the visible effect only); E2E: decline-the-confirmation exit 1, PGN import with one bad game exit 1 with the others stored, `install-cli` failures exit 1 and change nothing (PLAT-6 manual cases may gain an automated twin).
- E2E-CLI-22 already asserts 3 for `DurabilityUnsupported` in every form; unchanged. The wording clash noted in RR-S-4 (SDD line 388 versus test) disappears.
- chess-a0r.3.17 acceptance ("exit codes 0-4") becomes testable for all forms.

**Stored and public contract (G3).** No stored format. Public: the numbers and the sub-rules above, to be printed in help (UI-9) and README. Declaring `calibrate` unstable keeps one hidden command out of the contract.

**Recommendation: B1 with sub-rules 1 to 6. Confidence: medium-high** for the map (already built and partly ruled), **medium** for sub-rules 1 and 2, which are new policy and are the ones to scrutinise. Choose B2 only if Brian wants scripts to branch on "declined" or "partial".

## 5. Sub-question (c): extra setup flags

**Current text.** SDD DD-CLI-1 (line 427): setup flags are `--matchup --mode --white --black --clock` and `play --player`; "Other setup fields of GAME-1 (colour, start position) would need flags that the architecture does not name; none is added (TBD-36, UF-CLI-5)". A1 says the same (line 365). No test-only variable for `install-cli` is added (line 542) `[OBS]`. GAME-1 collects matchup, mode, players, colour (white, black or random), clock (preset, custom, untimed) and an optional start position (RULE-8, unrated) `[OBS]`.

What the current flags cannot do from a script: start from a FEN, pick a random colour, choose "untimed" explicitly (whether `--clock` absent means untimed is not stated `[UNV]`), and print the version. The interactive screens (TUI and plain mode) cover all of them. PLAT-1's scripted scenario is a list of SAN moves only (requirements line 139) and needs no FEN `[OBS]`.

| Option | Flags | For | Against |
|---|---|---|---|
| **C1** (recommended) | None added | No new G3 surface; every scenario in PLAT-1 and the c-c scripted run (UC-27) is already covered; interactive forms keep the full set | A script cannot start a custom-FEN game or a random-colour game; must drive plain mode by piped answers |
| **C2** | Add on `play` only: `--fen <FEN>` (RULE-8, unrated) and `--color white\|black\|random` | Scripted tests of RULE-8 and GAME-1 colour without driving prompts | Two new public names; `--color` overlaps `--white/--black` (see F-5); FEN in headless would break "stored and rated" (CLI-4 (c)) so it must be refused there |
| **C3** | C2 plus `--version`, `--move-delay`, `--style` | Convenience | More surface, no requirement asks for it |

**Small separate item (c2).** `--version` is not in any document `[OBS grep]`. Printing the program version and exiting 0 is the usual convention and useful for bug reports and the PLAT-5 version-mismatch message; clap provides it at no design cost. It is still public surface. Proposed: accept it as an addition (no requirement, listed as a design choice like the data commands in CLI-4 CR-5). If Brian prefers none, say so.

**Consequences for tests.** C1: none (UT-CLI-01 grammar table unchanged; add one row for `--version` if c2 accepted). C2: grammar rows, a refusal row for `--fen` in headless (exit 1), E2E from a custom FEN to a result with no rating change (RULE-8 verification today has no CLI-scripted case).

**Stored and public contract (G3).** No stored format in any option. Public: flag spelling (`--fen`, `--color`, hyphenation as `--no-open`, lowercase values as `--eco keep-pgn`), defaults (absent `--fen` = standard start; absent `--color` = from the names given, or the interactive default), validation errors exit 1.

**Recommendation: C1, plus `--version` (c2). Confidence: medium-high.** Revisit trigger: a verification case that needs a scripted FEN or random-colour start.

## 6. Recommended ruling in one block

| Sub-question | Ruling proposed | Confidence |
|---|---|---|
| (a) | A1: exit 4 for any signal stop outside `serve` (SIGINT in plain mode, SIGTERM, SIGHUP); `serve` SIGHUP = 0; headless SIGHUP = 4; plain mode SIGINT handled like SIGTERM; plain EOF with unfinished game = 0 confirmed | Medium |
| (b) | B1: catalogue map fixed for all forms (0, 1, 2, 3, 4 as above), sub-rules 1 to 6, `calibrate` outside the contract | Medium-high (map), medium (sub-rules 1, 2) |
| (c) | C1: no new setup flags; add `--version` (exit 0) | Medium-high |

## 7. Edits a ruling would trigger (to be entered after Brian decides; nothing done now)

| Document | Edit | Class |
|---|---|---|
| requirements.md Decision Log | New entry (next free CR number, CR-18 at the time of writing, to be re-checked) recording the three rulings, dated, G3, Brian Prascher | G0 record |
| requirements.md CLI-5 | Replace the "Noted 2026-10-31 (CR-9.6)" paragraph by the ruled rule; extend the **Verification** line (E: each run form gives its code; signal stop exits per ruling; decline exits 1 and so on); amend the "Amended 2026-10-28 (CR-5)" sentence only if scope text needs "outside `serve`" | G3, amendment (no in-place edit of baselined text without CR) |
| requirements.md TBD-36 and TBD-21 | TBD-36: Closed, with the CR reference; TBD-21 note: scope extended to all run forms (still closed) | Register |
| requirements.md CLI-4 | Add `--version` (and any C2/C3 flag) to the design-choice sentence; leave the CLI-4 (f) wording unless A3 is chosen | G3 |
| requirements.md change-history table, RELEASE_NOTES.md | Amendment row and release-notes entry | G0 |
| architecture.md | §6.1 exit-code paragraph and D-36 row (exit code rule extends to all forms); a new D- number (next free after D-100, to be checked) if Brian wants the signal rule recorded as a decision; D-36 reversibility row stays | G3 via architect |
| SDD-chess-cli.md | A3 (remove the "Open" block and "placeholder" wording), DD-CLI-6 (exit status of the signal path), DD-CLI-7 (EOF exit), DD-CLI-8/9 (SIGHUP), DD-CLI-10 (exit rules, decline, partial import), DD-CLI-1 (`--version` or flags), ASM-CLI-3, UF-CLI-2, UF-CLI-5, UF-CLI-6, UF-CLI-10, OQ-CLI-002 (close), RAT-CLI-11 (supersede), UT-CLI-08, E2E-CLI-03..07 and CT-CLI-04..12 test rows, trace table row for CLI-5; change-log row | G3 (SDD is baselined v1.0; amendment with Decision Log and RELEASE_NOTES entries) |
| SDD-chess-server.md | Only if it states signal exit codes for `serve`: add SIGHUP | G1/G0 `[UNV: not read]` |
| Beads | Record the ruling on chess-a0r.3.16 (closed by the human or PM, not by this author); chess-a0r.3.17 acceptance unchanged | Tracker |

## 8. Findings (Engineer stance; this is self-review, not a Reviewer pass)

| ID | Severity | Finding | Evidence | Action |
|---|---|---|---|---|
| F-1 | Major | Exit status of the TUI, plain mode and all subcommands is undefined, and the CLI code cannot assert exit numbers outside headless | CLI-5 note line 988; SDD A3 line 386 | Decide (a) and (b) |
| F-2 | Major | Plain mode on SIGINT is not specified anywhere. The TUI handles SIGINT as a key; headless as a signal; plain mode has neither path, so default process termination could skip the game suspend | SDD DD-CLI-6 line 477 (TUI only), DD-CLI-7 (no signal text) | Specify with (a): handle through `tokio::signal`, same path as SIGTERM |
| F-3 | Minor | SIGHUP wording differs: A3 table lists SIGHUP for headless; DD-CLI-8 and CLI-5 amendment list only SIGINT and SIGTERM; `serve` (PLAT-4, DD-CLI-9) lists only SIGINT and SIGTERM | SDD lines 401, 496, 504 | Align in the amendment |
| F-4 | Major | No rule for a declined confirmation, a cancelled prompt in a subcommand, or a PGN import where some games fail. A script cannot know whether it succeeded | DD-CLI-10 (counts printed, no status) | Decide sub-rules 1 and 2 |
| F-5 | Minor | The relation of `play --player <name>` to `--white`/`--black` for a scripted human game is not stated, so colour and opponent selection in PLAT-1's plain-mode scripted game is ambiguous | DD-CLI-7 line 488, DD-CLI-1 line 427 | Clarify in the SDD (G1); needed for chess-a0r.3.17 |
| F-6 | Minor | `--version` is absent from every document | grep over requirements, architecture, SDD | Decide (c2) |
| F-7 | Minor | Writing the PGN to stdout can fail (closed pipe) with no exit class: `ExportWriteFailed` covers file writes only | DD-CLI-10, A3 table | Proposed: exit 1, message on stderr; add to the SDD with the ruling |
| F-8 | Editorial | SDD line 388 and E2E-CLI-22 disagreed about the TUI exit 3 being a placeholder (RR-S-4) | OQ-16-limited-rereview-systems-engineer.md line 40 | Disappears once (b) is ruled |

Disposition of this package: ready for decision; open points are F-2 to F-7, which the ruling should absorb.

## 9. Assumptions

- A-01: The signal-stop rule applies to a stop that arrives before the requested action completes; it is not applied to `serve`, whose normal stop is a signal (PLAT-4).
- A-02: `Core::shutdown` suspends the game with the same effect in all forms (DD-CLI-6, D-14, D-80), so only the number differs between options.
- A-03: Nobody currently depends on the placeholder numbers; no code or released help text exists (SDD A3 states the placeholders are unpublished).
- A-04: Sub-rules 1 to 6 in section 4 are new policy proposed by the author; they are not in any document.

## 10. Not verified

- No code, build or tool was run; the CLI crate is an empty scaffold `[OBS CLAUDE.md]`.
- Whether `expectrl` can distinguish exit(n) from death by signal (matters only for A2) `[UNV]`.
- Behaviour of macOS terminals on SIGHUP when the window closes `[UNV]`; E2E-CLI-03..07 exercise it by injection.
- SDD-chess-server.md and use-case channel lines beyond UC-27 and UC-4 were not re-read for this package.
- The next free CR and D- numbers were taken from a grep of the current files and must be re-checked when entered.

## 11. Questions for Brian

1. (a) A1 (single exit 4), A2 (128+n) or A3 (0)? Do you expect `launchd`, `timeout` or similar wrappers around `chess`?
2. (b) B1 as written, with sub-rules 1 and 2 accepted, changed or dropped? Do you want B2 (extra numbers for declined or partial)?
3. (c) C1 plus `--version`, or C2 (`--fen`, `--color`)?

**Next step:** Brian rules in a comment on chess-a0r.3.16; the systems engineer then enters the section 7 edits (G3) and the PM closes the issue, which unblocks chess-a0r.3.17.

## 12. Ruling and verification additions (added 2026-11-09, version 0.2; Engineer stance)

**Ruling (Brian Prascher, G3, 2026-11-09, as relayed):** (a) A1, (b) B1 with sub-rules 1 to 6, (c) C1 plus `--version`. Recorded in `docs/requirements.md` CR-19, CLI-4 and CLI-5 amendment bullets, TBD-36 closed; design text in `docs/design/SDD-chess-cli.md` 1.0.1 (A3 rules ER-1 to ER-8). Findings of section 8 absorbed: F-1, F-2 (DD-CLI-7), F-3 (SIGHUP in DD-CLI-8 and DD-CLI-9), F-4 (ER-3, ER-4), F-6 (`--version`), F-8. **Still open:** F-5 (relation of `play --player` to `--white` and `--black`; G1 wording, not ruled), F-7 (stdout write failure, proposed exit 1: OQ-CLI-009); new OQ-CLI-008 (cancel at an ECO prompt and end of input read as exit 1), OQ-CLI-010 (`-V`), OQ-CLI-011 (signal mid-import).

### 12.1 Test rows for the software engineer (task chess-a0r.3.17)

All are planned, none has been run. The IDs are in the SDD section 9. Observation rule for every signal case: assert the child **exited with status n**, not that it was killed by a signal (the handler must call `exit(4)`, not re-raise); whether `expectrl` can tell the two apart is `[UNV]`, and a plain `std::process` spawn with `ExitStatus::code()` returning `Some(4)` (not `None`) is the fallback.

| ID | Level | Case | Expected | Status |
|---|---|---|---|---|
| UT-CLI-08 (amended) | Unit | `exit_code` for every catalogue code and every run form | Same class in every form (table of A3); `DurabilityUnsupported` 3; signal 4 except `serve` 0 | Amended |
| UT-CLI-019 | Unit | `--version` in the grammar; help text content | Accepted with and without a subcommand; version printed; exit 0; no runtime, core or store built; help lists the five classes and the signal rule; `calibrate` not in help | New |
| UT-CLI-020 | Unit | Non-error exit rules ER-1 to ER-8 as a pure table | See the SDD row | New |
| CT-CLI-04..12 (amended) | Component | Each error path of the data commands | The exit status per A3 is asserted in addition to the visible effect | Amended |
| CT-CLI-022 | Component | Decline and cancel (ER-3): declined `import data` confirmation; Esc or Ctrl-C as keys at a prompt; `import pgn` ECO prompt cancelled; end of input at that prompt | Exit 1, `cancelled` on stderr, store unchanged | New (the last two are OQ-CLI-008) |
| CT-CLI-023 | Component | PGN import outcomes (ER-4): one bad game among valid ones; all new; all duplicates skipped | 1 with others stored and error with index, line, column; 0; 0 | New |
| CT-CLI-024 | Component | Listings and informational commands (ER-8): empty `games`, `players`, `stats`; `stats <unknown>`; bare `export`, bare `import` | 0, 0, 0; 1; 1, 1 | New |
| CT-CLI-13 (amended) | Component | `install-cli` failure cases (ER-5) | Each exits 1, changes nothing, creates no data directory | Amended |
| E2E-CLI-03..07 (amended) | PTY | TUI: SIGTERM, SIGHUP; quit; confirmed Ctrl-C | 4, 4, 0, 0; terminal restored; game suspended | Amended |
| E2E-CLI-10..14 (amended) | E2E | Headless run signalled with SIGINT, SIGTERM, SIGHUP | 4 each; nothing on stdout; unfinished game stored | Amended |
| E2E-CLI-15..20 (amended) | E2E | `serve` stopped with SIGINT, SIGTERM, SIGHUP | 0 each; game resumable | Amended |
| E2E-CLI-24 | PTY and pipe | Plain mode, game in progress, SIGINT, SIGTERM, SIGHUP | 4 each; game unfinished and resumable | New |
| E2E-CLI-25 | Pipe | Plain scripted game, moves end early; and a game that finishes | 0 with the game suspended; 0 with result line and PGN | New |
| E2E-CLI-26 | E2E | SIGTERM during `import pgn` (large file), `import data`, `export data --output` | 4; no file at the final name; `import data` store unchanged; mid-PGN content per the core (OQ-CLI-011) | New |
| E2E-CLI-27 | E2E | `chess --version`; TUI and plain mode with the engine missing and Leave | version on stdout, 0, no store created; 2 | New |

**Inspection (not a test file):** help text (UI-9) and README list the five exit classes and the signal rule; `calibrate` is absent from `--help`. README is not edited by the systems engineer; PM to assign.

**Not run, not verified:** none of the rows exists as code. `expectrl` behaviour on exit status is `[UNV]`. SIGHUP behaviour of real macOS terminals on window close is `[UNV]` (injection only).
