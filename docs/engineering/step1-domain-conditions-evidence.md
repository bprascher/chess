# Step 1 chess-domain: conditions C-D2, C-D3, C-D4 (evidence)

| Field | Value |
|---|---|
| Tasks | chess-gx6 (C-D2), chess-oa9 (C-D3), chess-1vm (C-D4); baseline B4 |
| Date | 2026-11-09 |
| Stance | Engineer (software-engineer). Not a Reviewer pass, not an approval |
| Gate | GATE-103 (G1, one crate). Decisions applied: B5 (Brian Prascher, G-3 passed with conditions, 2026-11-09); desk review rulings D1(b), D2, D3, D7, D10 (approved by Brian the same day). Approval of this work: **PENDING**, nobody |
| Scope kept | `crates/chess-domain` src and tests only. `tests/drift_guard.rs`, `tests/data/fen_drift_guard.txt` and `crates/chess-rules` not touched by me. No SDD edited. `Cargo.toml`, `Cargo.lock` unchanged. No commit, no push, no beads change |
| Stopped for G2/G3 | Nothing. No journal contract, stored format or architecture 5.7 change was needed |

## 1 Baseline [RAN]

`cargo test -p chess-domain --offline` before any change: all suites ok (no failure). `cargo fmt`, `clippy` and `ci/check.sh` were green at the desk review; I re-ran them at the end (section 7).

## 2 C-D2 (B5, DF-01): finished-game clocks are not journaled

Changes (`src/game.rs`):
- `Game::journal_eq` doc comment now lists what is compared and what is **not**: the remaining times of a finished game, `EngineThinking`, `EngineError`, a step in progress (`stepping`), `Finalized`, the running interval of a clock.
- The skip is narrower than before. Before: a finished game skipped the whole clock comparison. Now: a finished game still compares the clock **spec** (timed or not, base, increment); only the remaining times are skipped. [OBS game.rs `journal_eq`]
- No `Clock` fact before `Ended`. No journal contract change.

Tests (all `[RAN]` green):

| Test | File | Shows |
|---|---|---|
| `ut_dom_042_b5_flag_final_clock_live_is_zero_replay_is_the_full_base_and_journal_eq_is_true` | tests/replay.rs | DF-01 reproduction 1: 10 s base, Flag at 11 s: live White 0 ms, replay 10 000 ms, `journal_eq` true |
| `ut_dom_042_b5_resign_final_clock_live_100000_replay_300000_and_journal_eq_is_true` | tests/replay.rs | reproduction 2: 5+0, White moves at 1 s, Black resigns at 201 s: live 100 000, replay 300 000, `journal_eq` true |
| `ut_dom_042_b5_no_clock_fact_is_written_before_ended` | tests/replay.rs | the ending step carries only `GameEnd` |
| `ut_dom_042_b5_journal_eq_still_compares_every_other_field_of_a_finished_game` | tests/replay.rs | negative: a changed `think_ms`, a changed outcome, a missing ply each make a finished game unequal |
| `ut_dom_042_b5_journal_eq_still_compares_the_clock_while_the_game_is_running` | tests/replay.rs | negative: a changed journaled clock fails while live |
| `ut_dom_042_b5_finished_games_still_compare_the_clock_spec` | src/game.rs (unit) | the narrowing: spec difference on a finished game is unequal |
| existing `ut_dom_042_rule_5_replay_equals_the_live_game_and_is_idempotent` (proptest, 5 setups) | tests/replay.rs | unchanged; still asserts `journal_eq` for every random live game. Every field except the documented ones is checked [INF from `journal_eq` body] |

Red then green: the spec test failed against the old behavior (`[RAN]` with the new comparison replaced by `true`: 1 failed; restored: 1 passed). The reproduction tests pin existing behavior and passed on first run, as expected (they assert the documented difference).

## 3 C-D3 (chess-oa9)

(a) **DF-05 / D2.** A `Clock` fact after a finished state (`Completed`, `Finalized`, `Aborted`, `Voided`) is now a no-op in `apply_fact` (was `WrongState`). An `Ended` outcome is stored as recorded (comment added; behavior unchanged). Tests: `ut_dom_044_d2_a_clock_fact_after_a_finished_state_is_ignored_not_refused` (four finished states, through `evolve` and `replay`, nothing changes, clock not restored), `..._in_a_live_state_still_restores_the_checkpoint`, `ut_dom_044_d2_an_ended_outcome_is_trusted_as_recorded` (checkmate outcome on the start position replays as recorded). Red: the first failed before the change with `refused with WrongState { state: Completed }` `[RAN]`.

(b) **DF-03 / D7.** `TreeError::TooDeep` added. `VariationTree::play` refuses a move that would nest variations deeper than `chess_rules::DEFAULT_MAX_VARIATION_DEPTH` (64): depth = number of non-first-child edges on the path from the root (new private `variation_depth`); a move that becomes a non-first child counts one more. Same move twice still returns the existing child before the check; a refusal changes nothing. Deleting a first child promotes the next sibling and only lowers depth, so refusing in `play` is enough [INF, tested]. Doc comments of `play`, `to_pgn_nodes`, `to_pgn_text` and `TreeError::BadText` corrected (the `BadText` arm of `to_pgn_text` is now a guard no built tree reaches). Tests (`tests/variation.rs`): depth 64 accepted and 65 refused with `TooDeep`; a tree built at the maximum (plus a main continuation below the deepest node) exports and round-trips through `from_pgn_text` to equal text; repeated attempts past the limit keep the tree exportable; delete-first-child makes room again; reading text nested 65 deep is still `BadText`; `TooDeep` renders a plain message. The old test `df_03_deep_tree_text_is_limited_by_the_rules_writer_depth` pinned the superseded behavior (export fails at 65) and was replaced by these. Red: the tests did not compile before `TooDeep` existed `[RAN]`.

(c) **DF-07 / D10.** `Policy::check_setup` refuses the same `PlayerId` on both seats when the kinds differ, with the existing `SetupField::Players` / `SetupReason::PlayersWrongKind` (no new reason). Test `ut_dom_025_d10_one_player_on_both_seats_with_different_kinds_is_refused` (h-c strict, training, both colour orders; self-play with one kind and a normal h-c game stay allowed). Red: with the new condition weakened the test failed `[RAN]`.

## 4 C-D4 (chess-1vm): independent policy table (DF-08)

Files: `crates/chess-domain/tests/data/policy_independent_table.txt` (the table: 122 lines including comments, rule rows with IDs, first match wins), `crates/chess-domain/tests/policy_independent.rs` (generic interpreter, no policy knowledge).

How it was made: from the text of SDD 3.11.5, 3.11.4 and architecture 4.4/4.5 only. **Disclosed limits [INF]:** I had read the F-D6 list and the first lines of the old oracle (Takeback, LiveAnalysis, WhatIf) before writing, so the table is independent of the implementation code but not blind to F-D6. I did not read `Policy::check`'s decision body before writing the table (I saw its first lines and the type definitions). I did not read the old oracle's remaining rows.

Method: every action (both sides, both actors) x mode x seating (HH, HC human White, HC computer White, CC) x 10 states x turn x pending offer x blocked, minus unreachable contexts (Training outside h-c; DrawOffered without an offer, an offer outside DrawOffered/Suspended, an offer by a computer, DrawOffered in c-c). 9 900 cases; 9 648 compared, 252 "unspecified by the text".

Result `[RAN]` (`cargo test -p chess-domain --offline --test policy_independent -- --nocapture`): **5 passed**. `Policy::check` agrees with the table in all 9 648 compared cases (verdict and reason). `Policy::allowed_actions` agrees for every action kind in every context (over 1 000 checked kind/context pairs, excluding those with an unspecified cell). So **no implementation difference was found against my reading**. Sensitivity check `[RAN]`: changing three table rows (not_human_turn, FD6b-3, draw_not_own_turn) made the comparison fail with 36, 48 and 18 differences, so the comparison is live. The table has no dead rows (a test fails on a row that decides no case).

The 252 unspecified cases: all are `MakeMove{actor: Engine}` with `by` not the side to move; the text names no `by` rule for the Engine actor. Implementation: `state_not_allowed` in all 252 [OBS test output].

### Readings and differences for the Systems Engineer

| ID | Point | Table reading (from text) | Implementation | Status |
|---|---|---|---|---|
| PD-1 | `MakeMove{Engine}` with `by` not the side to move | text silent; row `A-MM8` is `?` | `state_not_allowed` | **Spec gap**, not a conflict. SE/SA to say whether `by` must be the side to move and which reason |
| PD-2 | `RespondDraw` by the wrong side, or a human actor for a computer offeree (and the reverse) | the row lists only `state_not_allowed` as its reason, so that reason | `state_not_allowed` | Agrees; wording is thin: SDD should say the reason for a wrong answerer |
| PD-3 | Strict + `Suspended` for Takeback, LiveAnalysis, WhatIf | `strict_mode` (arch 4.4: Strict is never allowed "until the game ends") | `strict_mode` | Agrees, but the SDD cells say "strict_mode (S, in progress)" for LiveAnalysis and WhatIf and `Suspended` is not InProgress (F-D6 b), which could be read as `state_not_allowed`. Wording ambiguity |
| PD-4 | Order of global rows versus per-action reasons (Strict Takeback on a Completed game) | `game_over` first ("Any action on a ... game") | `game_over` first | Agrees; SDD does not state the precedence in one sentence |
| PD-5 | `SetMoveDelay` in a Suspended or Strict c-c state; `SelectClock` in any non-finished Strict state | `ok` (the text puts no state condition on them) | `ok` | Agrees (this is F-D6 d and e) |

No row made the implementation wrong against the text as I read it. Nothing in the implementation was changed. The F-D6 readings are explicit rows `FD6a-1` to `FD6e-1` in the table; (f) is a scenario test `fd6_f_an_engine_failure_drops_an_offer_to_a_computer_and_not_an_offer_to_a_human` (an h-c offer is dropped with a `DrawDropped` fact on `EngineFailed`; in h-h `EngineFailed` is refused and the offer stays) `[RAN]` passed.

### What the Systems Engineer must still confirm: F-D6 (a) to (e)

| Reading | Intent question for the SE | Alternative if the SE disagrees |
|---|---|---|
| (a) Analysis exemption from `game_over`/`voided` is exactly `PostGameAnalysis` and `WhatIf`; `LiveAnalysis` on a finished game is `game_over` | Is live analysis of a finished game meant to be refused (Training too)? | Allow `LiveAnalysis` after the end, or treat it as `PostGameAnalysis` |
| (b) `Suspended` is not InProgress: Rewind, LiveAnalysis and a Training WhatIf are `state_not_allowed` there; Strict reports `strict_mode` | May the player rewind or what-if a suspended (not attached) game? Which reason does Strict show (PD-3)? | Allow Rewind/WhatIf in `Suspended`; or Strict reports `state_not_allowed` |
| (c) A human `MakeMove` reports `not_human_turn` before `state_not_allowed` | Is the reason order intended (a click on the wrong side during a pause)? | Swap the order |
| (d) `SelectClock` is allowed in every non-finished Strict game ("S only") | Is it meant only before the first ply (desk review note: UI must not draw it in progress)? | Restrict to games with no ply |
| (e) `SetMoveDelay` is allowed in any non-finished c-c state | Is the delay adjustable while Suspended or EngineError? | Restrict to Pause/Playing states |

(f) is required by architecture 4.4 ("an engine failure drops the offer (ENG-4)") and needs no SE decision; only the SDD T4 text.

## 5 Public API changes (complete list)

1. **Additive:** `chess_domain::TreeError::TooDeep` (new enum variant; `TreeError` is not `#[non_exhaustive]`, so a downstream exhaustive `match` would need an arm. No other crate in the workspace references `TreeError` at present [RAN: grep, no hits outside chess-domain]).
2. No other signature changed. Behavior changes inside existing signatures (all rulings):
   - `Game::evolve` / `Game::replay`: a `Clock` fact in a finished state returns `Ok` (was `Err(ReplayError::WrongState)`).
   - `VariationTree::play`: can return `Err(TreeError::TooDeep)`.
   - `VariationTree::to_pgn_text`: can no longer fail for a tree built through the API (was `BadText` past depth 64).
   - `Policy::check_setup` (hence `Game::new`): refuses the same Player on both seats with different kinds (`players` / `players_wrong_kind`).
   - `Game::journal_eq`: for finished games it now also compares the clock spec (before: nothing).

## 6 SDD wording needed (SDD not edited; drafts A-D1 to A-D6 of the desk review cover most)

- 3.11.1 (A-D1, A-D2): finished-game remaining times, `EngineThinking`, `EngineError`, step in progress and `Finalized` are not journaled; `journal_eq` compares the clock **spec** of a finished game; a `Clock` fact after the end is ignored; `Ended` is trusted as recorded; give the DF-01 numbers as the example (flag at 11 s on 10 s: live 0 ms, replay 10 000 ms).
- 3.11.5 (A-D3): the notes in the desk review, plus PD-1 (Engine `MakeMove` with `by` not the mover), PD-2 (reason for a wrong answerer), PD-3 and PD-4 (Strict + Suspended reason; global rows first).
- 3.11.4 T4 list: `EngineFailed` in `DrawOffered` when the offer is to a computer.
- 3.11.6 `check_setup` (A-D6): the same Player on both seats with different kinds is `players` / `players_wrong_kind`.
- 3.11.10 / TreeError (A-D5): `TooDeep`, the depth definition (non-first-child edges) and that `play` refuses it; the IFP card for `play`.
- UT-DOM-020 text: the independent table (`tests/data/policy_independent_table.txt`) as the oracle source.

## 7 Gates `[RAN]`

| Command | Result |
|---|---|
| `cargo test -p chess-domain --offline` | 227 passed, 0 failed (sum of the suites) |
| `cargo fmt -p chess-domain` | ran, no error (it may have reformatted nothing of drift_guard.rs; I cannot tell from the shared tree) |
| `cargo clippy -p chess-domain --all-features --all-targets --offline -- -D warnings` | Finished, no warning |
| `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` | "All gates passed." (fmt --check, build --locked, clippy -D warnings, tests, crate-edge OK, ECO checksums OK, cargo deny ok; npm audit skipped offline). Run while another engineer was editing chess-rules, so it covers the tree as it stood then |
| `cargo llvm-cov -p chess-domain --offline --summary-only` | TOTAL lines 2541, missed 28, **98.90%** (cargo-llvm-cov 0.9.1). game.rs 99.57%, policy.rs 100%, variation.rs 97.37%, error.rs 90.32%. Uncovered lines in the changed code: none in `variation_depth`, `check_setup`; game.rs misses are pre-existing guards (`TooLong`, a state guard) |
| `cargo mutants -p chess-domain --re 'journal_eq\|variation_depth\|VariationTree::play\|check_setup\|apply_fact\|to_pgn_text' -j3` (cargo-mutants 27.1.0) | **79 mutants: 72 caught, 0 missed, 0 timeout, 7 unviable** (4 min). Unviable mutants are not counted as kills. This is a scoped sample of the changed functions, not the whole crate. Output kept in the session scratchpad (not in the repo) |
| `git diff Cargo.lock` | empty |

## 8 Not run / not verified

- No Reviewer pass and no SA desk review of this work. Nothing approved.
- Mutation testing covered only the functions above in game.rs, policy.rs, variation.rs; not `cargo mutants` for the whole crate.
- The table is one author's reading of the text; the SE confirmation of F-D6 (a) to (e) is open. The table is not blind to F-D6 (section 4).
- `ci/check.sh` ran against a shared working tree with concurrent chess-rules edits; a later run may differ.
- Replay/store/app consumers of the journal (`chess-store`, `chess-app`) do not exist yet; the effect of "Clock after end is ignored" on JournalCodec is [UNV].
- Intel Macs and Windows/Linux: not run.

## 9 Files

Changed: `crates/chess-domain/src/{error,game,policy,variation}.rs`, `crates/chess-domain/tests/{policy,replay,variation}.rs`. New: `crates/chess-domain/tests/policy_independent.rs`, `crates/chess-domain/tests/data/policy_independent_table.txt`, this file. Appended: `docs/engineering/gates/GATE-103-chess-domain.md`.

## Recheck fixes (2026-11-09, task chess-k5l, Engineer stance, gate G0 for chess-domain, no behavior change)

RC-02 of `review-step1-recheck.md`. `Game::journal_eq` is unchanged (ruling B5 stands: the remaining times of a finished game are not compared).

- Doc comment of `journal_eq` (`src/game.rs`) corrected: it said "No `Clock` fact precedes `Ended`". It now says the remaining times of a finished game are not compared whether or not a journaled `Clock` checkpoint preceded `Ended`; the checkpoint is applied on replay and its content (remaining times, running side) is then not compared; altering or dropping it does not change the result; the clock spec and all other listed fields are compared.
- Test renamed from `ut_dom_042_b5_journal_eq_still_compares_every_other_field_of_a_finished_game` to `ut_dom_042_b5_journal_eq_still_compares_think_time_outcome_and_plies_of_a_finished_game`; the section comment in `tests/replay.rs` no longer says "every other field".
- New test `ut_dom_042_rc_02_a_finished_game_compares_the_clock_spec_but_no_remaining_time` `[RAN]`: with a `Clock` fact inserted before `Ended`, (a) content `{1, 260000}` is applied (replayed White remaining `Some(1)`, differs from live) and `journal_eq` is true, (b) a different running side is true, (c) the dropped fact is true; a different clock spec (6+0 vs 5+0) is false.

Public API changes: none. Gates: see the rules section above (`ci/check.sh` passed for the workspace; `cargo llvm-cov -p chess-domain --summary-only` lines 98.90 %, functions 98.68 % `[RAN]`).

## R-1 / R-2 / PD-2 (2026-11-09, task chess-3ey, Engineer stance, baseline B4)

Decisions by Brian Prascher, typed 2026-11-09 (R-1, R-2, PD-2 of `se-confirmation-policy-readings.md`). Gate: G1 inside `chess-domain` (no protocol, journal, denial-reason or arch 4.4 table change was needed); the system-architect desk review and the SDD amendment are PENDING. Nothing here is approved by me.

### Change
- `Policy::check`, `Action::SelectClock` -> `deny(StateNotAllowed)` for every game state and mode that passes the global rows (previously `allow_if(strict, TrainingUntimed)`). Finished games still get `voided` / `game_over` first. `allowed_actions` therefore never lists `SelectClock` [OBS policy.rs `allowed_actions` filters on `check`].
- `Action::WhatIf`: finished unchanged (unconditional, both modes); Strict unfinished -> `strict_mode` (unchanged); Training unfinished -> allowed only in `AwaitingMove` with a human to move (was: any in-progress state with a human to move, i.e. also `DrawOffered`, `EngineThinking`, `EngineError`, `Paused`); all other unfinished states -> `state_not_allowed`. `Suspended` was already `state_not_allowed` (Training) and `strict_mode` (Strict).
- `RespondDraw` by the wrong side or actor: unchanged, `state_not_allowed`; no reason added (PD-2). Regression test added.
- `check_setup` untouched; it remains the single owner of the clock rules (`TrainingUntimed`, Strict timed/untimed). `Game::new` path verified by test.

### Public behaviour changes
1. `Policy::check(SelectClock, ctx)` in any unfinished state: `Err(state_not_allowed)` (was `Ok` in Strict, `training_untimed` in Training).
2. `Policy::allowed_actions` (hence `GameView`/snapshots built from it): `SelectClock` no longer listed in any state.
3. `Policy::check(WhatIf, ctx)`, Training, unfinished, in `DrawOffered`, `EngineThinking`, `EngineError`, `Paused`, `Suspended`, or when not the human's turn: `Err(state_not_allowed)` (was `Ok` in the first four with a human to move). `allowed_actions` follows.
4. No signature or type change. `DeniedReason::TrainingUntimed` stays in the public enum but `Policy::check` no longer produces it [OBS error.rs:40,58,76]; `SetupReason::TrainingUntimed` is unchanged. Removing the variant would change the protocol enum, so I did not (see "Items stopped / noticed").

### Tests (tests first; the table and oracle were changed before policy.rs)
- `tests/data/policy_independent_table.txt`: FD6d-1 removed (superseded comment), A-WI3 narrowed to `mode=T state=A tomove=H => ok`, A-SC2 replaced by `A-SC1 | SelectClock => state_not_allowed`. Rows derived from the readings text plus the two decisions, not from the code [INF]. Table now has 61 non-comment rows [RAN grep -cv]. `df_08` exempts `training_untimed` from "the table expects every reason" with a comment.
- `tests/policy.rs` oracle updated (SelectClock, WhatIf). New tests: `ut_dom_027_r_1_select_clock_is_state_not_allowed_in_every_started_state_and_mode` (every unfinished context from `contexts()`, also absent from `allowed_actions`; finished -> `voided`/`game_over`), `ut_dom_027_r_1_started_games_never_list_select_clock_and_setup_clock_rules_are_unchanged` (Strict h-h, Strict h-c, Training games from `Game::new`; `check_setup` Training+clock -> `Clock/TrainingUntimed`), `ut_dom_028_r_2_...` (all six unfinished states, both sides to move, `allowed_actions` agreement, Strict `strict_mode` including `Suspended`, finished unconditional in both modes incl. Voided), `ut_dom_029_pd_2_...`.
- Reason choices (Training WhatIf, not human's-turn-in-AwaitingMove): `DrawOffered`, `EngineThinking`, `EngineError`, `Paused`, `Suspended` all `state_not_allowed`; no other reason fits. Not obvious: `DrawOffered` where the offering human is the side to move (refused, no draw-specific reason applies, since `draw_*` reasons name offers only).

### Commands [RAN 2026-11-09]
| Command | Result |
|---|---|
| `cargo test -p chess-domain --offline` | 232 passed, 0 failed (sum of suites); `policy_independent`: 5 passed |
| `cargo fmt -p chess-domain` | no error |
| `cargo clippy -p chess-domain --all-features --all-targets --offline -- -D warnings` | Finished, no warning |
| `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` | "All gates passed." (npm audit skipped offline) |
| `cargo llvm-cov -p chess-domain --offline --summary-only` | lines 2543, missed 28, **98.90%**; policy.rs lines 273, missed 0 (100%) |
| `CARGO_NET_OFFLINE=true cargo mutants -p chess-domain --re 'Policy::check\|Policy::allowed_actions' -j3` | 77 mutants: **72 caught, 0 missed, 0 timeout, 5 unviable** (3 min). Scoped sample, not the whole crate |

### SDD 3.11.5 wording needed (for the amendment; I did not edit docs/design)
1. `SelectClock`: "Not an in-game action. The clock is chosen at setup and fixed when the game starts (GAME-1, RULE-4). In every unfinished state and either mode the answer is `state_not_allowed`, and `allowed_actions` omits it. `check_setup` alone enforces the clock rules (`training_untimed`, Strict timed or untimed)." Replace the current SelectClock row and A-SC cells.
2. Training `WhatIf` on an unfinished game: "allowed only in `AwaitingMove` with a human to move; every other unfinished state, including `DrawOffered`, `EngineThinking`, `EngineError`, `Paused` and `Suspended`, gives `state_not_allowed`. Strict gives `strict_mode`. A finished game allows it unconditionally (ANA-4)."
3. PD-2: "`RespondDraw` by the offerer, or by an actor kind that does not match the answering seat (a human for a computer, an engine for a human), gives `state_not_allowed`. No response-specific reason exists."
4. SE wording items (R-3) still to apply: (a) make the last "Any action on ..." row identical to the Voided row ("except `PostGameAnalysis` and `WhatIf`"); precedence sentence (PD-4): global rows first (`voided`, `game_over`), then per-action rules in table order; (b3) cells "strict_mode (S, game not finished)"; F-3 use "unfinished" (A..SU) vs `InProgress` super-state; (c) one sentence that the more specific `not_human_turn` precedes `state_not_allowed`; PD-1 Engine `MakeMove` needs `by` = side to move, a computer, in `EngineThinking`, else `state_not_allowed`; (e) `SetMoveDelay` allowed in any non-finished c-c state including `Suspended`, the app refuses it when no session is attached; (f) add `DrawOffered --EngineFailed--> AwaitingMove` (offer to a computer) to the T4 list and diagram. Arch 4.4 clock row and 4.5 "in-progress Strict game" wording are for the architect's amendment.

### Items stopped / noticed
- Nothing stopped: no protocol, journal, new-reason or arch 4.4 table text change was required. `DeniedReason::TrainingUntimed` is now unreachable from `Policy::check`; whether to retire it is a protocol decision (G2) for Brian/SA [UNV on consumers: chess-app/server do not exist yet].
- Arch 4.4 clock row text currently says front-ends render controls from `allowed_actions`; whether its table text lists `SelectClock` as in-game is [UNV] (docs not read this task; architect amendment pending).

### Not run / not verified
No Reviewer pass, no SA desk review. Whole-crate mutation not run. `ci/check.sh` ran while chess-rules was being edited concurrently. Intel Macs not run.
