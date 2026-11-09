# Step 1, chess-domain rework evidence (chess-a0r.2.12)

Date 2026-11-06. Stance: Engineer. Baseline B4. Input: `docs/engineering/review-chess-domain-code.md` (PASS WITH CONDITIONS), `docs/design/SDD-chess-domain.md`. Gate: GATE-103 rework note (G0 class). Nothing here is approved or committed.

## Scope

| Finding | Action | Result |
|---|---|---|
| DF-02 | Fixed | `Tally::games()` saturates |
| DF-03 | Cause found; documented; behavior pinned by test; distinct error NOT added (needs a decision) | Partly done, see below |
| DF-06 | Tests added; no defect needing a signature change; two lenient behaviors recorded | Done |
| DF-07 | SDD silent; behavior unchanged; finding recorded | Resolved as finding |
| DF-01, DF-05, DF-08, DF-09, DF-10 | Not touched | Await owner decisions |

## DF-02

- Test first: `tests/stats.rs::df_02_tally_games_saturates_instead_of_overflowing` (wins at `u32::MAX`, all three at `u32::MAX`, a small exact sum, and `PlayerStats::games`).
- Red `[RAN cargo test -p chess-domain --test stats]`: `panicked at crates/chess-domain/src/stats.rs:54:9: attempt to add with overflow`.
- Fix: `src/stats.rs` `Tally::games` uses `saturating_add` twice; doc comment added.
- Green `[RAN]`: `test result: ok. 11 passed; 0 failed` for the stats test file.

## DF-03

- Cause `[OBS crates/chess-rules/src/pgn/mod.rs:19,400-404]`: the `chess-rules` writer refuses variations nested deeper than `DEFAULT_MAX_VARIATION_DEPTH` (64) with `PgnWriteError::VariationTooDeep`; `format_line` is where `to_pgn_text` gets it, and `src/variation.rs` maps every writer error to `TreeError::BadText`. The reader has the same limit `[OBS crates/chess-rules/src/pgn/reader.rs:456]` (`parse_line` uses `PgnLimits::default()`). The review's "about 100" is this 64.
- The review's repro chain was not the nesting chain: nesting needs the continuing move to be a non-first child. My test builds the chain that way, through `play` only.
- Test `tests/variation.rs::df_03_deep_tree_text_is_limited_by_the_rules_writer_depth` `[RAN]`: depth 64 serialises and round-trips through `from_pgn_text` with the same node count; depth 65 gives `Err(TreeError::BadText)` with the tree well under `MAX_NODES`; a 256-level tree builds and its main line is one node. Passes. It pins the observed boundary at the rules constant. It was green on first run (the code was already doing this), so there is no red step. The defect was the false doc claim.
- Fix: doc comments corrected on `VariationTree::to_pgn_text` and `from_pgn_text` and on `TreeError::BadText` (no signature or message change).
- NOT done: serialising deep trees (needs `chess-rules` edit, forbidden) or a distinct error such as `TreeError::TooDeep` or a depth cap in `play` (adds an enum variant, a public API change). Needs an owner decision. Until then a legal tree deeper than 64 variation levels cannot be exported as a whole and the error says "bad text".
- Author evidence F-B9 (`step1-domain-B-evidence.md:86`, "not reachable") is now contradicted; I did not edit that file.

## DF-06

- `serde_json` is not admitted (architecture §3.4 lists it `[UNV]`, OQ-17 part 2 open), so adding it as a dev-dependency would be a dependency decision; not done. `tests/serde_shapes.rs` carries a small test-only value type (`Val`) with its own `Serializer` and `Deserializer` covering strings, integers, null and objects. It is a stand-in for the JSON data model, not the JSON codec. Escape and number-syntax behavior of real JSON is not tested `[UNV]`.
- 9 tests, all `[RAN]` passing: Seat round trip and shape; Seat unknown, missing, integer id, empty id, bad kind rejected; GameSetup round trip (untimed, timed, h-c), custom start FEN round trip and bad FEN rejected; GameSetup unknown top-level and nested fields rejected; missing `game_id`, `matchup`, `mode`, `white`, `black` rejected; integer ids, empty id, bad mode and matchup, bad clock text and integer clock rejected; Personality round trip with and without `uci_elo`, unknown field, each missing required key rejected, `uci_elo` optional, `try_from` validation (permille 1001, multipv 1), wrong types and out-of-range integers rejected.
- Behavior as the SDD says: unknown fields rejected (GameSetup doc comment and `deny_unknown_fields`); integers are not ids (D-40).
- Observed lenient behaviors, left unchanged (a change would alter the received-command contract):
  - FIND-DOM-R01: a missing `clock` key deserialises to `None` (untimed). The SDD does not say it must be present.
  - FIND-DOM-R02: a missing `start` key deserialises to `StartPosition::Standard` (the custom `Deserialize` reads `Option<String>` and serde passes "none" for an absent field). Same status.
  - Impact: a command that drops either key silently becomes an untimed standard-start game. The app layer may want an explicit presence check. Owner decision.
- Non-canonical UUID spellings (uppercase, no hyphens, braced, urn) are still accepted as DF-06 says: the "canonical input rule" decision is the owner's; no test pins it either way.

## DF-07

- SDD text `[OBS docs/design/SDD-chess-domain.md:730-734]`: `check_setup` rejects Training not h-c, a clock with Training, seat kinds that do not match the matchup, invalid custom clock, terminal start FEN. "Self-play is derived, not chosen." `slot_holders` for h-c: "the human only". Nothing says the same `PlayerId` on both seats with different kinds is rejected or is self-play.
- Decision: SDD silent, so behavior unchanged and no test pins it. Observed `[OBS src/policy.rs:95,393-421]`: such a setup passes `check_setup`, `is_self_play()` is true, `slot_holders` returns the human only.
- FIND-DOM-R03 (Minor): recommend the SE add a rule (reject, `SetupField::Players`, `PlayersWrongKind`) or state it is intended. If added, the fix is small and needs no signature change.

## Gates run

- `cargo fmt -p chess-domain` `[RAN]`: applied, exit 0.
- `cargo clippy -p chess-domain --all-targets -- -D warnings` `[RAN]`: `Finished dev profile`, no warnings.
- `cargo test -p chess-domain` `[RAN]`: 209 passed, 0 failed (summed over all test binaries; 4 test files unchanged).
- `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` `[RAN]`: ends `All gates passed.` Includes crate-edge check OK, ECO snapshot OK, `cargo deny` (advisories, bans, licenses, sources ok; one unmatched-license warning for "Unlicense", pre-existing), `npm audit` skipped by `CHESS_CI_OFFLINE=1`. The shell exit code was not captured by my pipeline; the pass is from the script's last line.
- `cargo llvm-cov -p chess-domain --summary-only` `[RAN]`: TOTAL regions 98.29%, functions 98.34%, lines 98.76% (2509 lines, 31 missed). Meets the 90% line target. Figure includes in-`src` test code.

## Files changed

- `crates/chess-domain/src/stats.rs` (DF-02 fix)
- `crates/chess-domain/src/variation.rs`, `crates/chess-domain/src/error.rs` (doc comments only)
- `crates/chess-domain/tests/stats.rs`, `tests/variation.rs` (appended tests), `tests/serde_shapes.rs` (new)
- `docs/engineering/gates/GATE-103-chess-domain.md` (rework note), this file.
- `Cargo.toml`, `Cargo.lock` and `chess-rules`: untouched by me (Cargo.lock and others were already modified before this task).

## Not run, not verified

- Mutation testing of the changed code was not run. DF-02 is a one-line change covered by the test at `u32::MAX`; `[INF]` that it is killed.
- Release-mode behavior of the DF-02 test (wrap) not run; debug panic was the red evidence.
- Real JSON behavior not tested (see DF-06). Intel and non-arm64 not run.
- Whether the SE intends DF-07 rejection, and whether DF-03 deserves a new error, are open decisions.
