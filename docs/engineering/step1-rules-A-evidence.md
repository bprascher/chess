# Step 1 evidence: chess-rules part A (chess-a0r.2.3)

Stance: Engineer. Nothing here is a review or an approval. Gate record: [gates/GATE-102-chess-rules.md](gates/GATE-102-chess-rules.md) (G1, OPEN, not approved). Working tree only; nothing committed. Tags: `[RAN]` executed, `[OBS]` read, `[INF]` inferred, `[UNV]` not verified, `[DOC]` fetched documentation.

## 1. Scope and files

Part A of `chess-rules` per SDD-chess-rules 0.3.5 DRAFT: DD-RUL-001 (types), 002 (Position), 003 (PositionKey), 004 (predicates), 007 (illegal-move explainer), 008 (FEN), 014 (errors, part A variants), 015 (perft). Not done (part B, chess-a0r.2.4): `san`, `parse_move`, SAN/PGN/ECO, `canonical_form`, `build.rs`, remaining `MoveParseError` variants.

Created/changed (all under `/Users/brianprascher/Projects/chess`): `crates/chess-rules/Cargo.toml` (deps), `Cargo.lock` (resolution only), `crates/chess-rules/src/{lib,types,error,position,fen,testing}.rs`, `crates/chess-rules/tests/{common/mod,types,fen,rules,properties}.rs`, `docs/engineering/gates/GATE-101-scaffolding.md`, `GATE-102-chess-rules.md`, this file. `.beads/issues.jsonl` was already modified before this task and was not touched by me `[OBS git status at start]`.

Dependencies (all vetted in dependency-lookup-step1.md): `shakmaty =0.30.2`, `thiserror =2.0.21`, dev `proptest =1.12.0`. The crates were resolved from the local cargo cache (`CARGO_NET_OFFLINE=true`); no download was needed `[RAN]`.

## 2. Upstream API check (OQ-RUL-001, part)

The `shakmaty` 0.30.2 API used was read from the unpacked crate source, not recalled `[OBS]`: `Chess`, `Position` trait (`legal_moves`, `play_unchecked`, `has_insufficient_material`, `ep_square(EnPassantMode)`, `maybe_ep_square`, `castles`), `Move::{Normal,EnPassant,Castle,Put}`, `fen::Fen::{from_ascii,from_position,into_position}`, `PositionErrorKinds`, `Castles::rook`, `Board::attacks_to`, `perft`. `Square::new` panics out of range, so it is called only with the checked invariant `< 64`. The upstream FEN parser accepts fewer than six fields, `_` separators, pockets and check counters `[OBS fen.rs]`; the crate therefore validates syntax itself before calling it.

## 3. Tests first: red, then green

Red `[RAN]`: after writing `tests/{types,fen,rules,properties}.rs` and the dependency lines, and before any implementation:

```
CARGO_NET_OFFLINE=true cargo test -p chess-rules --no-fail-fast
error: could not compile `chess-rules` (test "fen") due to 2 previous errors
error: could not compile `chess-rules` (test "properties") due to 6 previous errors
error: could not compile `chess-rules` (test "rules") due to 17 previous errors
error: could not compile `chess-rules` (test "types") due to 1 previous error
error[E0432]: unresolved imports `chess_rules::Fen`, `chess_rules::Move`, `chess_rules::Position`, ...
```

This red is a compile failure (the API did not exist), not an assertion failure. Several assertions were red for real later: the first green run had 1 failing fixture in `fen` and 5 in `rules`; all were errors in my fixtures (castling tests put the black king on h8 in check from the white rook on h1; a placement variant with castling rights the pieces did not support; a queen on the h8 diagonal). I corrected the fixtures, not the production code or the expectations of the rules `[RAN]`. Perft counts needed no change.

## 4. Final gate run

`[RAN] CHESS_CI_OFFLINE=1 ci/check.sh` -> exit 0, "All gates passed."

| Step | Result |
|---|---|
| `cargo fmt --all -- --check` | no output |
| `cargo build --workspace --all-targets --locked` | Finished |
| `cargo clippy ... -- -D warnings` (workspace lints: unwrap, expect, panic, indexing, casts denied) | Finished, no warning |
| `cargo test --workspace --locked`, chess-rules targets | unit 8 passed, 4 ignored; `fen` 15 passed; `isolated_data_dir` 4 passed; `properties` 3 passed; `rules` 37 passed; `types` 5 passed |
| `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| `cargo deny --offline check` | `advisories ok, bans ok, licenses ok, sources ok` (two `license-not-encountered` warnings, as before) |
| npm audit | skipped by `CHESS_CI_OFFLINE=1` |

Also `[RAN]` with the feature on: `cargo clippy -p chess-rules --all-targets --features test-support --locked -- -D warnings` clean; `RUSTDOCFLAGS="-D warnings" cargo doc -p chess-rules --no-deps --features test-support` clean. `cargo tree -e normal,build -p chess-rules` shows only `shakmaty` (with `arrayvec`, `bitflags`, `btoi`, `num-traits`, build-only `autocfg`) and `thiserror` (with `proc-macro2`, `quote`, `syn`, `unicode-ident`): no `rand`, no other workspace crate `[RAN]`.

Suppressions, each with a written reason beside it: `#![allow(clippy::unwrap_used, expect_used, panic, indexing_slicing, reason = ...)]` at the top of the four integration-test files and `tests/common/mod.rs` (clippy.toml allows these only inside `#[test]` functions, so shared helper functions need it); `#![allow(dead_code, reason = ...)]` in `tests/common/mod.rs`. No suppression in `src/` outside `cfg(test)` code.

## 5. Perft (UT-RUL-003)

Published counts were fetched from the Chess Programming Wiki Perft Results page on 2026-11-06 `[DOC https://www.chessprogramming.org/Perft_Results]` and compared with the counts in the tests. All counts in the tests match the page. (Before the fetch the counts were recalled from memory and agreed with both this crate and the upstream generator, `[INF]`.)

| Position (FEN in `src/testing.rs`) | Depths asserted | Counts asserted | Where run |
|---|---|---|---|
| Start | 1-4 | 20, 400, 8902, 197281 | default suite (debug) |
| Start | 5, 6 | 4865609, 119060324 | `#[ignore]`, ran |
| Kiwipete | 1-3 | 48, 2039, 97862 | default |
| Kiwipete | 4 | 4085603 | default |
| Kiwipete | 5 | 193690690 | `#[ignore]`, ran |
| Position 3 | 1-5 | 14, 191, 2812, 43238, 674624 | default |
| Position 3 | 6 | 11030083 | `#[ignore]`, ran |
| Position 4 and its colour mirror | 1-4 | 6, 264, 9467, 422333 | default |
| Position 4 | 5 | 15833292 | `#[ignore]`, ran |
| Position 5 | 1-4 | 44, 1486, 62379, 2103487 | default |
| Position 5 | 5 | 89941194 | `#[ignore]`, ran |
| Position 6 | 1-4 | 46, 2079, 89890, 3894594 | default |
| Position 6 | 5 | 164075551 | `#[ignore]`, ran |

The perft is built on this crate's `legal_moves` and `play`, so castling, en passant and promotion conversions are exercised. A further test compares it with `shakmaty::perft` at depths 1-3 for all seven positions `[RAN]`, passes.

Ignored tests: reason in the attribute (run time). Command and result `[RAN]`:
`CARGO_NET_OFFLINE=true cargo test -p chess-rules --release --lib -- --ignored` -> `4 passed; 0 failed` in 3.08 s (release). In a debug build they are impractical (not measured). The default suite's perft tests take 2.7 s in debug.

## 6. Acceptance criteria

| Criterion | Status | Evidence |
|---|---|---|
| Tests written first | Met, with the caveat in section 3 | red compile failure recorded; later fixture corrections listed |
| Perft matches published counts (start, Kiwipete, positions 3-6) | Met | section 5; every deep run executed |
| Castling fixtures | Met | `ut_rul_004_*castling*` (both sides, blocked, through check, out of check, b1 attacked, rights lost by king/rook move/rook capture), `ut_rul_015` castling reasons |
| En passant | Met | normal capture, right expires, pinned-pawn rank case, capture of a checking pawn, property test on the written en passant square |
| Promotion | Met | each piece, capture, black side, `PromotionRequired` |
| Pins | Met | pinned bishop and rook, pinned pawn capture and en passant reasons |
| Stalemate / checkmate | Met | `ut_rul_004_*stalemate*`, `*checkmate_fools_mate`, mate on the fifty-move completing move |
| Insufficient material | Met | SDD 3.11.3 table in both colours (`ut_rul_007_*material*`); ASM-RUL-001 validated: the upstream test agrees with the table, so the in-crate classifier was not written |
| Threefold / fifty-move | Met at position level | `PositionKey` equality after a four-ply shuffle and transposition, en passant counted only if capturable, castling/side differences; `halfmove_clock` 99->100 and resets. Counting repetitions and ending the game are `chess-domain`'s (DR-RUL-004) and are not tested here |
| Blocked-pawn non-draw | Met | `ut_rul_007_rule_1_blocked_pawns_are_not_dead_by_material` |
| clippy / fmt clean | Met | section 4 |
| Requirement IDs in test names | Met | every test is named `ut_rul_nnn_<req>_...` or `fz_rul_nnn_...` |

## 7. Coverage and mutation (extra, not required by the criteria)

Coverage `[RAN] cargo llvm-cov -p chess-rules --features test-support --summary-only` (cargo-llvm-cov 0.9.1), as printed, line coverage: `fen.rs` 91.82 %, `position.rs` 99.43 %, `testing.rs` 77.46 % (the uncovered lines are the `#[ignore]`d deep tests), `types.rs` 100.00 %, TOTAL 95.62 % (31 of 707 lines missed). The test-strategy gate is "at least 90 % line for rules and parser modules"; part A modules meet it `[INF]`, the gate is formally measured when part B lands.

Mutation `[RAN] cargo mutants -p chess-rules --exclude crates/chess-rules/src/testing.rs --timeout 90` (cargo-mutants, `src/{fen,position,types,error}.rs`): run 1: 253 mutants, 67 unviable, 165 caught, 21 missed, 0 timeouts. I added tests for the real gaps (empty counter field, queen-side castling reasons and shape guards, rook diagonal, pinned pawn capture and en passant, pawn two-file jump) and re-ran the missed ones with `--iterate`: 8 killed, 13 remain missed. Disposition of the 13: 5 are the unreachable fallback arms of the upstream `ParseFenError` mapping (own syntax checks run first, so the arms cannot be reached `[INF]`); 1 `|`->`^` on disjoint bit flags (equivalent); 2 `>`->`>=` where the operands never tie (equivalent); 4 `2 * forward`/`2 * step` -> `/` where the factor is +-1 (equivalent); 1 `||`->`&&` in the off-board test of `offset`, whose callers never leave the board (unreachable defensive check). No test was weakened or suppressed. Mutation is recommended, not required, for this crate (SDD section 7).

## 8. Observation for chess-2bb (en passant FEN normalisation, relevant to SDD step 5 / `canonical_form`)

`[RAN]` test `ut_rul_027_if_3_either_en_passant_convention_is_accepted_and_kept_as_parsed` and `ut_rul_016_rule_8_standard_start_equals_only_with_all_six_fields`:

1. `Fen` equality is on the normalised six-field text, i.e. on parsed values: counters are numbers (`007 0012` -> `7 12`), castling letters are in `KQkq` order (the parser rejects other orders rather than normalising them), so none of these is a respelling case.
2. The en passant square is **kept as given**. `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1` and the same text with `-` are two different `Fen` values, although they have equal `repetition_key()`. Both are accepted (IF-3).
3. `Position::to_fen` writes the en passant square after every double push (UR-26, SDD 3.11.5); a position parsed from a FEN writes back the field it was given.
4. Consequence for step 5 (standard-start comparison in `canonical_form`): a FEN equals the standard start only if all six fields are equal, with en passant `-`. The standard start has no double push, so there is no ambiguity for it. The "pending XC-DOM-A8-5" row (en passant, reordered castling field) resolves as: the Fen type compares **parsed/normalised values**, so a reordered castling field is rejected as a syntax error (not accepted and compared), and an en passant square on an otherwise standard placement makes the Fen differ from the standard start. Whether `chess-domain`'s `StartPosition::from_fen` compares the same way is for the domain to confirm `[UNV]`. The SDD was not edited.

## 9. Findings and observations (for the Reviewer and owners)

| ID | Severity | Finding |
|---|---|---|
| F-A1 | Minor | Integration tests cannot use `testing::perft` because feature `test-support` is off by default and a self dev-dependency is a `chess-rules -> chess-rules` edge that `ci/check-crate-edges.mjs` rejects. Perft tests therefore sit in `src/testing.rs` unit tests (cfg(test)). Options for the owner: accept; or run CI with `--all-features`. Not changed (CI file out of scope) |
| F-A2 | Minor | SDD signatures are sketches; part A chose: `IllegalMove` as a struct, new `SquareError`, `FenError` variant shapes, `MoveParseError::BadUci` only. These are inside the crate and not yet consumed (G1 inside the SDD's freedom) but the SA desk review should confirm them before `chess-domain`/`chess-engine` code them |
| F-A3 | Minor | `dependency-lookup-step1.md` does not itemise the transitive crates of `shakmaty` (`arrayvec`, `bitflags`, `btoi`, `num-traits`, `autocfg`) `[OBS grep]`. `cargo deny` licences and advisories pass with them |
| F-A4 | Info | SDD 3.11.3 table validated against the upstream material test (ASM-RUL-001 closed in practice); SDD text could record it. RSK-RUL-002 (brute-force mate-search property test of the material rule) was not written `[UNV]` |
| F-A5 | Info | `explain_illegal` is implemented in part A (it is needed for `play`'s error). Its `Other` result for a legal move is by SDD 3.11.2 |
| F-A6 | Minor | `legal_moves` drops a move silently if conversion fails (never happens for standard chess: only drops, which are not generated). Perft equality with the upstream count covers this `[RAN]` |
| F-A7 | Info | `PositionKey` is a 64-entry array plus fields, a few hundred bytes. The SDD said "tens of bytes" (3.11.4, estimate). Size not measured `[UNV]`; the cost is per ply stored by `Game` |

No G2 matter was found: no edge, §5.7 contract or protocol change was needed.

## 10. Assumptions

ASM-A1: the SDD's "Position::san" belongs to part B (notation). ASM-A2: `explain_illegal` belongs with `Position` (SDD 3.2 lists DD-RUL-007 under `position`). ASM-A3: halfmove clock and fullmove counters accept any `u32` (halfmove) and `1..=u32::MAX` (fullmove); the upstream increment behaviour at the limit was not examined. ASM-A4: `Fen::parse` rejects every deviation from the strict six-field form (single spaces, no pockets, no check counters, no Shredder/X-FEN castling letters).

## 11. Not run / not verified

- Intel Mac and any other platform: not run. Only macOS arm64 `[OBS rustc 1.99.1 host]`.
- `ci/check.sh` ran with `CHESS_CI_OFFLINE=1`: `npm audit` skipped; `cargo deny` used the cached advisory DB.
- CT-RUL-001 (no upstream type in the public API) has no automated check; by reading, `shakmaty` appears in `pub` signatures nowhere (`pub(crate)` only) and `grep -l shakmaty src` lists `lib.rs` (doc comment), `fen.rs`, `position.rs`, `testing.rs` (test module) `[RAN]`.
- UT-RUL-005 property tests run 64 cases per property; no longer soak.
- FZ-RUL-002 runs 256 random cases per property.
- Part B tests and entities (UT-RUL-008 to 014, 017 to 023, 026, 028 to 031, FZ-RUL-001/003, CT-RUL-003/004) are out of scope.
- Reviewer pass and SA desk review: not done.
