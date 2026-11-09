# Step 1 evidence: chess-domain part A (chess-a0r.2.6)

Stance: Engineer. Nothing here is a review or an approval. Gate record: [gates/GATE-103-chess-domain.md](gates/GATE-103-chess-domain.md) (G1, OPEN, not approved). Working tree only; nothing committed; beads not touched. Tags: `[RAN]` executed, `[OBS]` read, `[INF]` inferred, `[UNV]` not verified. Earlier evidence in the same style: [step1-rules-A-evidence.md](step1-rules-A-evidence.md), [step1-rules-B-evidence.md](step1-rules-B-evidence.md).

## 1. Scope and files

Implemented per SDD-chess-domain 1.0 BASELINED (the design; not edited): the identity newtypes (UUID `GameId`, `PlayerId`), the stable text types, the `Fact` vocabulary of 3.11.1, the `Game` state machine (`decide`, `evolve`, `replay`, terminal re-check, SAN policy, `DrawDropped` warning), `Policy` (the decision table, `allowed_actions`, `check_setup`, `slot_holders`, `check_player_action`, `assign_colors`, `StartPosition`, `MoveDelay`), `Clock` and `ClockSpec`, `TimeTotals` and `time_accrual` (3.11.13, D-78 seed), Elo and rating eligibility, category bands, the `TimeSource` and `Rng` traits, and the `observe`/`classify` half of `EcoTable` (needed by `Game`).

Not done, because they are part B (chess-a0r.2.7) or outside part A: `SplitMix64`, `error_pick_seed` and the shared vector file (DD-DOM-002 remainder; UT-DOM-002 golden outputs, 101, 102), `RosterGen`, name generator, `Personality` (DD-DOM-013, 014, 020; UT-DOM-060 to 067, 109, 111), `normalize_eco_tag`, `classify_import`, `resolve` (UT-DOM-071, 112, 113), `VariationTree` (073 to 076, 107), `BoardGrid` (080 to 084, 104, 110), stats (090 to 096), the `ts-rs` feature `ts` and CT-DOM-001's JSON snapshot, `Preferences` (OQ-DOM-004, not mine). The part B work is not blocked: nothing in part A names those types, and `Cargo.toml` already declares `uuid` feature `v5` for the roster.

Files (absolute, under `/Users/brianprascher/Projects/chess`):
- Edited: `crates/chess-domain/Cargo.toml` (dependencies, feature `test-support`), `crates/chess-domain/src/lib.rs`, `Cargo.lock` (two new entries, `uuid` 1.29.0 and `sha1_smol` 1.0.1; workspace-level, unavoidable, reported).
- New source: `crates/chess-domain/src/{types,error,edge,clock,elo,policy,eco,game,totals}.rs`.
- New tests: `crates/chess-domain/tests/{types,elo,clock,policy,game,replay,eco}.rs`, `crates/chess-domain/tests/common/mod.rs`; in-crate unit tests in `src/edge.rs`, `src/clock.rs`, `src/game.rs`.
- Docs: `docs/engineering/gates/GATE-103-chess-domain.md`, this file.
- Not touched: `crates/chess-rules` (another engineer works there), `ci/`, `deny.toml`, root `Cargo.toml`, any SDD or other document. `cargo fmt` was run as `cargo fmt -p chess-domain` only.
- No `CHESS_DATA_DIR` helper is included: no domain test touches a data directory (ruling-test-helper-placement.md; the crate is pure). The ruling's `dead_code` allow note does not apply.

Sizes `[RAN wc -l]`: `src` 3 323 lines in 10 files, `tests` 4 258 lines in 7 files plus `tests/common/mod.rs` (counts include blank lines and the in-crate test modules).

## 2. Tests first

Order of work: the test files for each tier were written before the code of that tier.

- Tier 1 (`tests/types.rs`, `elo.rs`, `clock.rs`): first `cargo test -p chess-domain --offline` after writing them, with `src/lib.rs` still the empty scaffold: **compile failure, red**. `[RAN]` the first lines of the error summary (`sort | uniq -c`): `3 error[E0425]: cannot find value AwaitingMove in this scope`, `2 ... Voided`, `2 ... Suspended`, `2 ... Paused`, `2 ... Finalized`, `2 ... EngineThinking`, `2 ... EngineError`, `2 ... DrawOffered` (the rest of the output was not kept). As in rules part A and B this is a compile red, not an assertion red.
- Tier 2 and 3 (`tests/policy.rs`, `game.rs`, `replay.rs`): after writing `policy.rs`, and again after writing `game.rs` and `replay.rs`, and before `policy.rs`, `game.rs` and `eco.rs` existed in `src/`, the same command printed **5 `error` lines** `[RAN]` each time; red again by compilation.
- First green run with everything written (`cargo test -p chess-domain --offline --no-fail-fast`): **8 assertion failures** `[RAN]`, the real reds (1 in `elo`, 1 in `game`, 3 in `policy`, 3 in `replay`). Each, with the side that was fixed:
  1. `ut_dom_054_elo_1_overflow_is_an_error_not_a_wrap`: my fixture was wrong (`i32::MAX` against 0 gives delta 0, no overflow). Test fixed (equal ratings).
  2. `ut_dom_046_checkpoint_command_...`: my expectation was wrong (a checkpoint re-bases the running clock, so `journal_eq` differs; what the player sees must not). Test fixed to compare the view.
  3. `ut_dom_025_game_4_terminal_start_position_...`: my fixture FEN had the side not to move in check. Test fixed.
  4. `ut_dom_025_game_setup_is_self_play_and_serde_...`: **production defect**. `ClockSpec` and the text enums deserialised with `<&str>::deserialize`, which accepts only borrowed input, so a reader-based JSON decoder would fail. Fixed to `String` in `types.rs` and `clock.rs`.
  5. `ut_dom_020_..._oracle`: my size assertion (`n > 40 000`) was wrong; the product is 36 000 cases. Test fixed (`> 30 000`).
  6. `ut_dom_118_...san_policy`: my "other fen_after" edit changed nothing (the side to move was already `w`). Test fixed.
  7. `ut_dom_123_...`: the simulated session kept the live `Game` across a stop instead of rebuilding it by replay, which hid the "unfinished turn is lost" case the design states. Test fixed (the stop rebuilds the game by `Game::replay`). 
  8. `ut_dom_105_d_58_draw_dropped_is_emitted_only_where_the_design_says`: the pseudo-random driver never reached a dropped offer (`seen == 0`). Test fixed with three explicit scripted cases first.
- Later reds from the soak and mutation runs, each a production or test-strength finding:
  - The property test `ut_dom_042_...` (default 96 cases, then a soak at `PROPTEST_CASES=20000` that came after it) found, on a run before the soak, a **production defect**: `Pause, Step, Pause` in a c-c game made replay fail with `WrongState{Paused}`, because `Step` has no fact and the journal then shows `Pause` while the replayed game is already `Paused`. Fixed: replay accepts `Pause` in `Paused`. Pinned by `ut_dom_042_rule_5_pause_step_pause_replays_although_step_has_no_fact`.
  - cargo-mutants on `game.rs` first showed 20 missed mutants (section 7); tests were added for each.

## 3. Final gate run

`[RAN]` `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` -> exit 0, "All gates passed." Steps printed: `cargo fmt --check` (whole workspace, so it also covered `chess-rules` as it was in the tree at that moment), `cargo build --locked`, `cargo clippy -D warnings`, `cargo test`, `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`, ECO snapshot checksums, `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`), `npm audit: skipped (CHESS_CI_OFFLINE=1)`. The run was done twice (once mid-way, once at the end after the last edit); both exit 0. The concurrent `chess-rules` edits did not break the build at either moment.

Also `[RAN]`: `cargo clippy -p chess-domain --all-targets --offline -- -D warnings` clean; `cargo fmt -p chess-domain` clean; `cargo tree -e normal,build -p chess-domain --offline` lists `chess-rules` (with `shakmaty`, `thiserror`), `serde`, `thiserror`, `uuid` (with `serde_core`, `sha1_smol`) and the proc-macro crates, and `grep -E "tokio|rusqlite|rand|time|chrono"` on it finds nothing (CT-DOM-003, done by hand, not a CI step: SDD 3.4 says the CI assertion is `[UNV: not written]`).

Suppressions in `src/`: two, both with written reasons. `clippy::new_ret_no_self` on `Game::new` (SDD IFP-DOM-2 names the constructor `Game::new` and has it return the `Step` that carries the `Start` fact), and `clippy::cast_possible_truncation` on one `f64 as i32` in `elo.rs` (SDD 3.11.7: range checked on the line above). The integration-test files carry the same crate-level `#![allow(unwrap_used, expect_used, panic, indexing_slicing, reason = ...)]` header that the `chess-rules` test files use. `#![forbid(unsafe_code)]` stays.

## 4. Acceptance criteria

| Criterion (verbatim from the work order) | Status | Evidence |
|---|---|---|
| UUID `PlayerId`/`GameId` | **Met** | `types.rs` `uuid_id!`; `uuid` 1.29.0, `default-features = false`, features `v5` and `serde`; text round trip, hyphenated Display, `FromStr` rejects non-UUID text, serde as a string and an integer is rejected: `ut_dom_121_td_3_ids_are_uuid_newtypes_with_text_round_trip`, `ut_dom_121_ids_deserialise_from_uuid_text_and_reject_other_shapes` `[RAN]`. No random source (`v4` not enabled) |
| Exact-integer Elo known-value tests | **Met** | `ut_dom_051_*` (4 tests): 1200 v 1200 K 32 +16/-16, 0/0, -16/+16; K 40 v 32 +20/-16; 1400 v 1200 K 32 +8/-8, -8/+8, -24/+24; a settlement with `before`, `after`, `delta`, `k`, `counter_after`. `ut_dom_050` hand values of the expected score, `ut_dom_052` half-away rounding, `ut_dom_053` K switch at exactly 30 games and per-side counters, `ut_dom_056` no clamp, `ut_dom_054` overflow and a property for bounds, sign, symmetry, `ut_dom_059` band edges and baselines `[RAN]` |
| Mode-policy table test for every action x mode | **Met** | `ut_dom_020_ply_5_full_product_equals_the_table_oracle`: 36 000 cases (2 modes x 4 seat layouts x 10 states x 2 turns x 3 offer states x 3 block states x 25 action instances) against an independent oracle written from SDD 3.11.4/3.11.5, 4 392 of them allowed `[RAN]`. `ut_dom_021` `allowed_actions` agrees with `check` for every context and has the fixed order. Named scenarios `ut_dom_022`, `023`, `024` (x2), `025`, `026`, `106` |
| Clock logic with fake time | **Met** | `tests/clock.rs`, `tests/game.rs` with scripted `Stamp`s from a fake time source (`common::FakeTime` in the integration tests, `FakeTimeSource` in the crate): start, stop, switch, Fischer increment after the move, pause and resume, flag deadline and re-arm, floor at zero, backwards reading, untimed think time, restore from a checkpoint, flag on a move after expiry, black flag, bare-king draw `[RAN]` |
| Property tests for state invariants | **Met** | `ut_dom_042` (replay equals live and is idempotent, random command scripts over five setups), `ut_dom_035` (random legal playouts: invariants hold, at most one `Ended`), `ut_dom_012` (clock invariant), `ut_dom_054`, `ut_dom_122` (total >= sides), `ut_dom_123` (totals bounds), `fz_dom_001` (arbitrary and mutated fact sequences, 2 tests). Default 96 to 200 cases; soak `[RAN]` `PROPTEST_CASES=20000 cargo test -p chess-domain --release --test replay` -> 21 passed (the two scenario tests were added after this soak), no failure. `Game::check_invariants` is the state-invariant oracle |
| >= 90 % coverage | **Met** | 97.82 % lines (section 6) `[RAN]` |
| Mutation testing on Elo and policy | **Met** | `elo.rs` + `policy.rs`: 147 mutants, **108 caught, 39 unviable, 0 missed** (section 7) `[RAN]`. Also run: `clock.rs` + `totals.rs` 72 mutants, 32 caught, 0 missed; `game.rs` 174 mutants, 128 caught, 0 missed |
| Tests first, red then green, shown | Met with the compile-red caveat | section 2 |
| Requirement or test IDs in test names | **Met** | every test is `ut_dom_nnn_<req>_...`, `fz_dom_001_...`; the in-crate tests of `check_invariants` and `journal_eq` carry `ut_dom_035` and `ut_dom_042` |
| Gate record GATE-103, tier, Q-answers, STOP on G2 | **Met** | gate record; G1; no G2 trigger found |
| chess-2bb recorded as far as part A touches it | **Met** | section 8 |
| clippy `-D warnings`, fmt, `ci/check.sh` pass | **Met** | section 3 |

## 5. Test inventory and trace

`[RAN]` `cargo test -p chess-domain --offline`: **130 passed, 0 failed**: lib 8, `clock` 14, `eco` 4, `elo` 16, `game` 41, `policy` 16, `replay` 23, `types` 8, doctests 0.

Trace of the SDD section 7 table (ID: tests):
- UT-DOM-001: 3. UT-DOM-121: 5 (journal codes, `ClockSpec` text, `VoidReason`, id round trip and deserialise). UT-DOM-100: 1. UT-DOM-002: 2 (in-crate: fake time, `Rng::below` only).
- UT-DOM-010 to 014: 3, 3, 1, 3, 3. UT-DOM-103: 3.
- UT-DOM-020 to 026: 1, 1, 1, 1, 2, 5, 1. UT-DOM-106: 1. UT-DOM-115: 2 (part). `assign_colors`, `MoveDelay` and serde shapes are under UT-DOM-025 and UT-DOM-103.
- UT-DOM-030: 8 (T1, T3 to T6, every disallowed pair, T19, finished games). UT-DOM-031 to 033: 4, 1, 1. UT-DOM-034: 3. UT-DOM-035: 4. UT-DOM-036 to 041: 2, 1, 1, 3, 2, 2. UT-DOM-042: 4. UT-DOM-043: 3. UT-DOM-044: 4. UT-DOM-045: 1. UT-DOM-046: 2.
- UT-DOM-105: 3. UT-DOM-108: 1. UT-DOM-116 to 120: 1, 3, 3, 2, 1. UT-DOM-122, 123, 124: 4, 1, 1. FZ-DOM-001: 2.
- UT-DOM-050 to 057, 059: 1, 4, 1, 2, 2 (054), 2 (055), 1 (056), 1 (057), 2 (059). UT-DOM-058 is the mutation run (section 7).
- UT-DOM-070 (part A subset): 2. UT-DOM-114 (part A subset): 2.
- CT-DOM-003: by hand (section 3). CI-DOM-001: `ci/check.sh`.

Honest weak spots:
- **UT-DOM-057** ("a Training game changes only Training inputs; the Strict `RatedSide` is never read", oracle "spy type"): the test shows that `settle` is deterministic in its two `RatedSide` arguments. The function cannot see the other track by signature, so there is nothing to spy on; the caller (`chess-store`) chooses the inputs. The property is a signature property, not a tested one.
- **UT-DOM-115**: the standard FEN, the counter-spelling case and seven one-field variants are tested; the en passant row cannot be built (section 8), and the shared drift-guard table of UT-RUL-031 row (5) does not exist yet (left for chess-a0r.2.7 by the rules evidence). The "four starts of BP-005" rows that need `rated` and the statistics predicate belong to the stats part (B).
- **UT-DOM-123**: asserts the bounds with a simulated session driven by random scripts over an untimed Strict h-h game. Timed games and `Detach` versus `Shutdown` stops are not separately varied.
- **UT-DOM-043**: clocks frozen, offer survival, drop and resume target are tested; "`EngineThinking` resumes as `AwaitingMove`" is tested; the "wall_ms gap" narrowing option is not implemented (the design says it is not made now).
- **UT-DOM-024** says "`CancelEngine` only in `EngineThinking` of an h-c game, both modes": covered through the policy table and a direct test.

## 6. Coverage

`[RAN]` `cargo llvm-cov -p chess-domain --offline --summary-only` (cargo-llvm-cov 0.9.1), lines:

| File | Lines | Missed | Cover |
|---|---|---|---|
| clock.rs | 189 | 7 | 96.30 % |
| eco.rs | 17 | 0 | 100.00 % |
| edge.rs | 50 | 0 | 100.00 % |
| elo.rs | 77 | 1 | 98.70 % |
| error.rs | 31 | 3 | 90.32 % |
| game.rs | 902 | 7 | 99.22 % |
| policy.rs | 269 | 11 | 95.91 % |
| totals.rs | 46 | 0 | 100.00 % |
| types.rs | 119 | 8 | 93.28 % |
| **TOTAL** | **1 700** | **37** | **97.82 %** |

Functions 92 % to 100 %; regions total 2 662 with 61 missed (97.71 %). Branch coverage is not reported by this llvm-cov build (`Branches 0`); MC/DC was not measured and none is claimed. The line counts include the `cfg(test)` modules inside `src/` (about 280 lines, all executed), so the production-only ratio is somewhat lower: with the same 37 missed lines against roughly 1 420 production lines it is about 97.4 % `ESTIMATE`. The 90 % gate for Elo, clock and policy is met by each of those files (98.70, 96.30, 95.91).

## 7. Mutation testing

`[RAN]` `cargo mutants -p chess-domain --file crates/chess-domain/src/elo.rs --file crates/chess-domain/src/policy.rs --no-shuffle -j 4 -- --offline` (cargo-mutants 27.1.0): **147 mutants tested in 3m: 108 caught, 39 unviable; 0 missed, 0 timeouts.** Unviable means the mutant did not compile; none was hidden. The first run (before the last test edits) gave the same numbers, 147 / 108 / 39.

Also run, as the SDD recommends (7, last paragraph):
- `clock.rs` + `totals.rs`: first run 74 mutants, 31 caught, **3 missed** (an `&&` to `||` in `canonical_u32` that is equivalent because an empty string also fails the parse, which I simplified; the `Clock::spec` accessor which no test read; `journaled_eq`'s `&&`). Fixed by removing the redundant test and adding the in-crate test `ut_dom_014_journaled_eq_...`. Re-run: 72 mutants, **32 caught, 40 unviable, 0 missed**.
- `game.rs` (termination order, in-play rules, replay, `journal_eq`, `check_invariants`): first run 174 mutants, 108 caught, **20 missed**: 17 in `journal_eq` and `check_invariants` (the two oracles of the property tests, which mostly assert "true", so a permissive mutant lived), `Game::eco` and `Game::repetition_keys` (weak assertions), one in the `Aborted` arm of `apply_fact`, the `ply == self.ply()` guard of the move check, and the `EngineFailed` guard for an offer to a human. Added in-crate tests that corrupt each field and expect the specific error or the inequality, plus four scenario tests. Second run: 1 missed (`||` to `&&` in the history-length check, because another check produced the same `Err`); the test now asserts the exact error text. Final run `[RAN]`: 174 mutants, **128 caught, 46 unviable, 0 missed**.

UT-DOM-058 ("zero surviving mutants as the target; survivors are reported, not hidden") is met for `elo` and `policy`, and for `clock`, `totals` and `game`.

## 8. Observations for chess-2bb (XC-DOM-A8-5: en passant and the six-field comparison)

`[RAN]` with the working-tree `chess-rules` (`Fen::parse`), checked by `ut_dom_115_*` and `ut_dom_118_*`:

1. **`StartPosition::from_fen` compares the `Fen`'s normalised text** (`Fen::as_str()`, which is six fields, counters as numbers) with `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`. Equal gives `Standard`; anything else gives `Custom`. `... - 00 01` parses to `0 1` and is `Standard`. Each of seven one-field variants (halfmove, fullmove, castling x2, side, two placements) is `Custom` and `Custom.fen()` returns exactly the parsed text.
2. **The en passant row of the drift-guard table cannot be constructed on the standard placement.** `Fen::parse("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq e3 0 1")` returns `Err(BadField{EnPassant, "the square must be on rank 6 when White is to move and on rank 3 when Black is"})`, and `... e6 0 1` returns `Err(Illegal(Other("the en passant square does not match the pieces on the board")))`. So `StartPosition::from_fen` never receives such a value; the castling respelling is also rejected (rules evidence section 8 item 1). The "unreachable en passant square" case that XC-DOM-A8-5 worries about does not occur for the standard start with this FEN type: the answer is "rejected before classification", not Standard and not Custom. The expected result of that row in UT-DOM-115 is therefore **parse error**.
3. **Replay compares a recorded `fen_after` as a parsed position, not as text.** `Game::replay` parses the recorded string with `Fen::parse`, builds the position and compares `repetition_key()` (placement, side, castling, en passant only when a legal capture exists), the halfmove clock and the fullmove number with the computed position. Results, `ut_dom_118_xc_dom_a8_5_fixture_a_*` and `fixture_b_*` `[RAN]`:
   - (a) 1.e4 (no capture possible): the recorded `... b KQkq e3 0 1` (the spelling the rules writer emits after every double push) and the same text with `-` are **both accepted**.
   - (b) 1.a3 d5 2.a4 d4 3.e4 (a capture is possible): `e3` is accepted; `-` gives `ReplayError::FenMismatch{ply: 4}`.
   - (c) a differing castling field, halfmove clock, fullmove number or placement gives `FenMismatch`; an unparseable text, an empty text and a position without kings give `FenMismatch`.
4. This differs from `Fen` equality, which keeps the en passant square as given (rules evidence section 8 item 2), so `Fen == Fen` would reject (a) with `-`. The SDD fixture allows either outcome for (a) ("the test is accepted only when the recorded result agrees with the writer's spelling"). The writer emits `e3`, which is accepted. Whether to prefer text equality or the parsed comparison is the owner's call; the parsed comparison is the one that fits SF-02 and D-79 (3) ("a difference of text only is not an error"), and it keeps (b) failing as the SDD requires. **Proposed SDD text** for 3.11.6 and XC-DOM-A8-5, for the architect or SE to adopt or reject: "`StartPosition::from_fen` compares `Fen::as_str()`; a standard placement with an en passant square or a reordered castling field does not parse. Replay compares `fen_after` as a parsed position: `repetition_key`, halfmove clock, fullmove number."

## 9. Findings and observations (for the Reviewer and owners)

| ID | Severity | Finding |
|---|---|---|
| F-D1 | Minor | `FakeTimeSource` and `ScriptedRng` are behind feature `test-support` as the SDD says, but plain `cargo test --workspace` does not enable a feature for integration tests and a self dev-dependency is a rejected edge (the same problem as GATE-102 F-A1). They are unit-tested in the crate (`cfg(test)`); the integration tests carry a 20-line copy of each in `tests/common/mod.rs`. Options for the owner: accept, or run CI with `--all-features`. Not changed (CI file out of scope) |
| F-D2 | Minor | SDD 3.11.1 says "`replay(facts_of(script))` equals the live `Game` field by field". That cannot hold literally: `EngineThinking`, `EngineError`, the step in progress, `Finalized` and the clock of a finished game are not in the journal. Part A defines `Game::journal_eq` (transient states map to the journaled one; a finished game's clock is not compared). The SDD should say "every journaled field". Also found by the property test: `Step` has no fact, so a journal can show `Pause` while the replayed game is already `Paused`; replay accepts it. SDD 3.11.1 should list that |
| F-D3 | Minor | The SDD does not say whether `Fact::Move.ply` is zero- or one-based or whether `think_ms` is optional. Chosen: `ply` is the number of plies already played (the first move has ply 0; equals `expected_ply`, `Takeback.to_ply` is the count kept); `think_ms: Option<u64>` because 3.11.13 says "a `Move` without a `think_ms` adds 0" |
| F-D4 | Minor | `GameCommand::Void{reason}` in the SDD lacks the reset Player that `Fact::Voided` carries; part A has `Void{reason, reset_player}`. `RespondDraw{accept}` carries no actor, so `Policy` cannot refuse a human who answers a computer's decision; `Game` derives the actor from the offeree's seat kind. The app must keep the engine path separate (G1 wording for the app SDD) |
| F-D5 | Info | A game with no move played has no pending termination (`pending_termination()` is `None`), so a custom start that is already dead by material is not "terminal" at replay and `ResolveTerminal` is refused. Mate and stalemate starts are refused at setup |
| F-D6 | Minor | The policy table (3.11.5) leaves points open; part A chose and the oracle test pins them: (a) the analysis exemption from `game_over` and `voided` is exactly `PostGameAnalysis` and `WhatIf`; `LiveAnalysis` on a finished game is `game_over`. (b) `Suspended` is outside "InProgress" (as in the 3.10 diagram), so `Rewind`, `LiveAnalysis` and a Training `WhatIf` are `state_not_allowed` there; Strict still reports `strict_mode` first. (c) `MakeMove` by a human checks the turn (`not_human_turn`) before the state. (d) `SelectClock` is listed by `allowed_actions` for every non-finished Strict game, because the table says only "S only". (e) `SetMoveDelay` is allowed in any non-finished c-c state. (f) `EngineFailed` is accepted in `DrawOffered` when the offer is to a computer, as UT-DOM-105 requires; T4 lists only `EngineThinking`. The Systems Engineer should confirm (a) to (e) |
| F-D7 | Minor | `ClockSync` has `Option` remaining times (untimed games have none; the SDD sketch has plain integers); `Clock::remaining` and `checkpoint` return `Option`; `ClockSpec` serialises as its `<base>+<increment>` text (3.5), not as a struct. `check_setup`'s `clock_invalid` reason does not exist: a `ClockSpec` is validated when built, so an invalid spec cannot reach the setup |
| F-D8 | Medium | The clock of a finished game is not journaled. `Ended` carries no clocks and T10/T16 list no `Clock` fact before it, so after a restart the replayed clock is the one at the last `Move` or `Clock` fact (the resign, the flag at zero and the time since the last move are lost). The live game stops the clock at the ending instant and shows the true value. If `GAME.white_clock_ms` at finalize must survive a crash between `Ended` and the finalize transaction, a `Clock` fact before `Ended` is needed; that is a journal contract question (G2/G3) and was not done. For the app owner: take the final clocks from `view()` at the moment of `Ended` |
| F-D9 | Minor | SDD 3.11.9 and UT-DOM-070 say a transposition "reaches the same position and therefore the same entry". That holds only when the final position is itself in the table. `[RAN]` `ut_dom_070_..._transposition_...`: 1.d4 Nf6 2.c4 e6 classifies `A50 Indian Defense: Normal Variation` and 1.c4 e6 2.d4 Nf6 classifies `A13 English Opening: Agincourt Defense`, because the "deepest named position" is taken along the path and the shared final position is unnamed. The SDD wording and the UT-DOM-070 fixture should require a named final position (the Ruy Lopez orders are used in the test) |
| F-D10 | Info | The public API names `chess-rules` types (`Side`, `Move`, `Fen`, `Position` in accessors and facts). `chess-store` has no edge to `chess-rules` (deny.toml `wrappers`), and the CLI and server reach the domain through `chess-app`. The store uses `EloCalc` and the value types, none of which names a rules type. The curated re-export list of `chess-app` (architecture 5.7) is the app SDD's to settle; no edge changes here |
| F-D11 | Info | `Finalized` (T19), `EngineStarted`, `EngineFailed`, `EngineRetry` and `Step` change state in `decide` without a fact: transitions the SDD marks as no-fact (T2, T4, T5, T12) and a store event (T19, FIND-DOM-013). "`evolve` is the only mutator" holds for every journaled change. The alternative (new journal kinds) is G3 and was not taken |
| F-D12 | Info | `TimeTotals::seed_from_facts` finds the mover of a `Move` from the `ply` parity and the side to move of the `Start` fact in the slice (White if the slice has no `Start`), because `Fact::Move` has no side. Correct for taken-back plies since `ply` is the index |
| F-D13 | Info | The standard-start comparison counts a FEN with leading zeros in the counters (`00 01`) as `Standard`; the stored `start_fen` is the normalised text. |
| F-D14 | Info | `uuid` feature `v5` is declared in part A and unused until part B. `cargo-deny` licence check passes with `sha1_smol` (BSD-3-Clause is in the allow list). A cargo-deny warning `unmatched license allowance: Unlicense` was printed (pre-existing, not from this crate) |

No finding rises to a G2 matter. F-D8 is the one to read first.

## 10. Assumptions

- ASM-D1: the `Game` aggregate holds `positions`, `keys` and `plies` as vectors and clones them per command (SDD 3.12 `ASM-DOM-005` says to measure; not measured, `[UNV]`).
- ASM-D2: `Fact::DrawDeclined.by` is the side whose offer was declined, as `DrawDropped.by` and `DrawOffer.by` are the offerer (architecture 7.6 says only "`by` (side)").
- ASM-D3: a Training takeback leaves the clock untouched and restarts the new turn's think time at zero (untimed).
- ASM-D4: on `Checkpoint`, `Game` re-bases a running clock (stop and start at the same instant) so the live remaining value equals the journaled one.
- ASM-D5: `Rng::below` is a provided method using multiply-shift on 128 bits (SDD 3.11.8 says so for the stream); the part B `SplitMix64` may override it.
- ASM-D6: `Game::new` starts White's clock for an untimed game too (to measure `think_ms`, DR-DOM-004).
- ASM-D7: an `EngineFailed` while a stepped search runs leaves `stepping` set, so a retry and the move end in `Paused`.

## 11. Not run / not verified

- Intel Mac and any other platform: not run. Only macOS arm64 `[OBS rustc 1.99.1 host]`.
- `ci/check.sh` ran with `CHESS_CI_OFFLINE=1`: `npm audit` skipped; `cargo deny` used the cached advisory DB.
- No Reviewer pass; no SA desk review; no SE review of the policy table choices (F-D6). Nothing is approved.
- Performance of copy-per-command (SDD 3.12, NFR-8) not measured. The soak was 20 000 cases per property on the replay tests, not a longer run.
- `cargo-llvm-cov` reports lines, regions and functions only; branch and MC/DC coverage were not measured.
- The `ts-rs` feature and the JSON snapshot of CT-DOM-001 are not built; `serde` shapes are checked through serde's value deserialisers only (no JSON crate is a dependency).
- Mutation testing was not run on `types.rs`, `error.rs`, `edge.rs` or `eco.rs`.
- `proptest` regression files are not kept in the tree (none was left after the fixes; the one found case is pinned by a unit test).
- The SDD signatures are sketches; part A's exact names (`GameCommand`, `Step`, `ReplayItem`, `ClockSync`, `UnratedReason`, `RatingTrack`) follow the SDD where it names them and are otherwise my choice (F-D3, F-D4, F-D7).
