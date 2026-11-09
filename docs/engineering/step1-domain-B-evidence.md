# Step 1, chess-domain part B: evidence (task chess-a0r.2.7)

Stance: Engineer. Nothing here is reviewed or approved. Gate: GATE-103, section CHG-103-B (G1, desk review pending). Date: 2026-11-06. Tags: `[RAN]` executed, `[OBS]` read, `[INF]` inferred, `[UNV]` not verified.

## 1. Scope delivered

| SDD item | Where | Tests |
|---|---|---|
| `SplitMix64`, `error_pick_seed` (DD-DOM-002 rest) | `src/edge.rs` | `tests/splitmix.rs`, `tests/data/splitmix64_vectors.txt` (UT-DOM-002, 101, 102 partly) |
| `RosterGen`, `first_k_after`, `Personality`, names, `roster_uuid` (DD-DOM-013, 014, 020) | `src/roster.rs` | `tests/roster.rs`, `tests/data/roster_seed1_golden.txt` (UT-DOM-060 to 067, 109, 111) |
| `normalize_eco_tag`, `EcoCode`, `classify_import`, `resolve`, `EcoChoice`, `EcoImportOutcome`, `EcoFields`, `EcoUnanswered` (DD-DOM-015) | `src/eco.rs` | `tests/eco_import.rs` (UT-DOM-071, 112, 113, 114 additions, 070 with F-D9) |
| `VariationTree`, `NodeId`, `SavedLine`, `MAX_NODES` (DD-DOM-016) | `src/variation.rs` | `tests/variation.rs` (UT-DOM-073 to 076, 107) |
| `BoardGrid`, `BoardView`, `BoardStyle`, `Orientation`, `Cell`, `Shade`, `Highlight`, `render_text` (DD-DOM-017) | `src/board_grid.rs` | `tests/board.rs` (UT-DOM-080 to 084, 104, 110) |
| `StatsRow`, `PlayerStats`, `aggregate`, `counts_in_aggregates`, `RatingPoint`, `rating_series`, `check_history` (DD-DOM-018) | `src/stats.rs` | `tests/stats.rs` (UT-DOM-090 to 096) |
| `RosterError`, `PersonalityError`, `TreeError`, `HistoryError`, `BoardError`, `EcoUnanswered` (DD-DOM-019) | `src/error.rs` | UT-DOM-100 additions in `roster.rs`, `variation.rs`, `stats.rs`, `board.rs` |
| UT-DOM-115 drift-guard table | `tests/data/fen_drift_guard.txt` | `tests/drift_guard.rs` |
| Journal vocabulary assigned to this crate | nothing left for part B `[OBS]`: `Fact`, the 1:1 table, `EndReason::journal_code`, `VoidReason`, `ClockSpec` text were part A (UT-DOM-105, 108, 121). Part B adds only the vector file for `SplitMix64` (FF-8) | |

Manifest: `Cargo.toml` and `Cargo.lock` unchanged by part B `[OBS git status]`. No `ts-rs`: no part B item needs the `ts` feature (it derives six types owned by part A and the app SDD), so it is left for the step that generates the TypeScript types. No `insta` (finding F-B1). `crates/chess-rules` not edited by this task (the modified rules files in `git status` predate it).

## 2. Baseline and red phase

`[RAN]` `cargo test -p chess-domain --offline` before any part B code: all part A targets green (8 unit, clock 14, eco 4, elo 16, game 41, policy 16, replay 23, types 8).

Red: the six new test files were written first. `[RAN]` `cargo test -p chess-domain --offline --no-run` failed with E0432 "unresolved imports" in `board`, `eco_import`, `roster`, `splitmix`, `stats`, `variation` (for example `chess_domain::BoardGrid`, `RosterGen`, `SplitMix64`, `VariationTree`, `aggregate`, `normalize_eco_tag`). `drift_guard` exercises part A behaviour and was green from the start.

Green: the implementation was then written; first full run had 1 failure in the golden roster test (the golden file was empty; generated from the implementation and then pinned), 1 in `ut_dom_071_classify_import_over_a_real_game` (my fixture `1.a3` is named A00, a wrong test premise, fixed) and 1 in `ut_dom_076` (my fixture played an illegal move for the side to move). No test was weakened; each was a fixture error.

## 3. Final run

| Check | Command | Result |
|---|---|---|
| Whole gate | `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` | `[RAN]` exit 0, "All gates passed." fmt, build, clippy -D warnings, test, crate-edge (`8 crates, 11 workspace edges, all allowed`), ECO checksums, cargo deny ("advisories ok, bans ok, licenses ok, sources ok"; the pre-existing `Unlicense` allowance warning), npm audit skipped (offline) |
| Format | `cargo fmt -p chess-domain` | `[RAN]` applied; `ci/check.sh` runs `cargo fmt --all -- --check` and passed |
| Lint | `RUSTFLAGS="-D warnings" cargo clippy -p chess-domain --all-targets --offline -- -D warnings` | `[RAN]` clean |
| Tests | `cargo test -p chess-domain --offline` | `[RAN]` 198 passed, 0 failed. Part B targets: board 15, eco_import 8, roster 20, splitmix 5, stats 10, variation 8, drift_guard 2 (68 tests) |

Suppressions added by part B: test files `drift_guard.rs`, `splitmix.rs` and the new ones carry the same file-level `#![allow(clippy::unwrap_used, ...)]` with a written reason as the part A test files (helper functions outside `#[test]`). In `src/`: one `#[allow(clippy::cast_precision_loss, reason = ...)]` on `u128_to_f64` (stats means). No other.

## 4. Per-criterion table

| Criterion | Met | Evidence |
|---|---|---|
| Roster of 25 deterministic from seed | yes | `ut_dom_060_ply_4_roster_of_25_is_deterministic_from_the_seed`, golden file `ut_dom_060_golden_file_for_seed_1` (characterisation of the first implementation, not an independent oracle), `ut_dom_062` (125 sets, bounds, names unique). UUIDs checked against Python `uuid5` (`ut_dom_061`) |
| `first_k_after` | yes | `ut_dom_109_first_k_after_table` (empty, other classes, mixed, unsorted, duplicates, k = 0 stored, overflow); mutation: 0 missed |
| `BoardGrid::from_fen` | yes | `ut_dom_104_*`, `ut_dom_084_*` (both orientations), `ut_dom_080_*`, 081, 082, 083, 110; mutation: 0 missed |
| ECO normalisation (D-60) | yes | `ut_dom_071_d_60_normalisation_*` (A00..E99, F00, A0, empty, NBSP, VT, full-width, non-ASCII digit), `classify_import` five variants, `resolve` (KeepPgn gives the upper-case tag code and an empty name; KeepTable; no choice is `EcoUnanswered`; others ignore the choice), property `ut_dom_113` (2000 cases) |
| Stats aggregates against seeded data | yes | `ut_dom_090_*` against hand-computed totals (6 rows), 091 (seven non-counting row kinds), 092, 093 (window boundaries), 094, 095, 096 |
| Line coverage >= 90 % | yes | section 5 |
| Mutation testing on roster, `first_k_after`, stats, ECO normalisation | yes | section 6 |
| UT-DOM-115 drift guard | partly | domain side done; the rules side is not changed (F-B3) |
| `ts-rs` | n/a | not needed by part B; left for the type-generation step |
| UT-DOM-102 (vector file identical to the engine's) | partly | the file is here and tested; `chess-engine` has no copy yet, so the CI diff cannot be written `[UNV]` |
| UT-DOM-080 as `insta` snapshots | no, replaced | exact expected line lists in the test (F-B1) |
| CT-DOM-001 JSON shape snapshot | no | needs a JSON crate and `insta`, neither vetted for this task |

## 5. Coverage

`[RAN]` `cargo llvm-cov -p chess-domain --offline --summary-only` (cargo-llvm-cov 0.9.1): part B files by line coverage: `board_grid.rs` 147 lines 0 missed 100.00 %; `eco.rs` 100.00 %; `edge.rs` 100.00 %; `roster.rs` 280 lines 6 missed 97.86 %; `stats.rs` 113 lines 0 missed 100.00 %; `variation.rs` 176 lines 5 missed 97.16 %; `error.rs` 90.32 %. **Crate total 2 507 lines, 48 missed, 98.09 %.** Branch coverage is not reported by this build (Branches 0); MC/DC not measured. The line counts include the `cfg(test)` modules in `src/`. Missed lines in `roster.rs` and `variation.rs` are the defensive `unwrap_or` and `ok_or` arms that cannot be reached through the public API (`[INF]`).

## 6. Mutation testing

`[RAN]` `cargo mutants -p chess-domain --file src/{roster,stats,eco,edge}.rs --no-shuffle -j 4 -- --offline` (cargo-mutants 27.1.0): 184 mutants, **110 caught, 74 unviable, 0 missed, 0 timeouts** (final run, after the fixes below).
`[RAN]` same on `src/{variation,board_grid}.rs`: 81 mutants, **43 caught, 38 unviable, 0 missed**.

First runs found survivors, fixed by tests, not by hiding: `validate` (two mutants: the default table was never asserted valid; zero-error bounds with MultiPV 0), the `MAX_ROSTER_INDEX` boundary, `ansi_background` (three: no test pinned the colour per reason), the check-king search (`&&` to `||`: needed a rook on a1 before the king on e1), `NodeId::index`, `VariationTree::is_empty`. Two timeouts in `choose_name` and `build` (an endless candidate loop when a test double breaks it) led to a real change: the candidate search is bounded by `MAX_NAME_SUFFIX` (F-B6).

Not mutation tested: `types.rs`, `error.rs` (part A files), `game.rs` etc. were done in part A.

## 7. Notes on F-D9 and ECO

F-D9 (transposition holds only when the final position is named) applies to UT-DOM-070. `eco_import.rs` tests what is true: two move orders to the named Ruy Lopez position give equal matches (`ut_dom_070_f_d9_*`); the unnamed-final-position case stays pinned as observed in part A's `tests/eco.rs`. The ply-0 lookup of a custom start was already implemented in `Game::new` in part A; `ut_dom_114_cr_9_11_a_custom_start_that_transposes_*` adds a custom start that reaches the Ruy Lopez position and gets the same code and name as the standard game, at its own ply.

## 8. Findings and observations

| ID | Severity | Finding |
|---|---|---|
| F-B1 | Minor | The SDD asks for `insta` snapshots (UT-DOM-080). `insta` is in the lookup file but not in this task's vetted list, so the snapshots are exact expected line lists. If `insta` is wanted, it is a `Cargo.toml` dev-dependency edit by the owner |
| F-B2 | Medium | `ROSTER_NAMESPACE` (a frozen value after release, D-34) had to be chosen: `UUIDv5(DNS, "chess-roster.v1")` = `aa98d4a0-fa10-585f-a6a6-268551d362c9`, checked by a test and against Python. The SDD says only "generate once and freeze". The owner should confirm or replace it before the first release |
| F-B3 | Minor | The drift-guard table is in `chess-domain/tests/data/fen_drift_guard.txt`, not in `chess-rules` as UT-RUL-031 proposes (I may not edit rules). To get a red case now, `drift_guard.rs` reads the rules test source with a relative `include_str!` and fails if a FEN the rules test uses is not a row, or a row (but the en passant one) is not in the rules test. **Rules-side item for the rules owner:** make `chess-rules/tests/pgn.rs` read the shared table (and move the file to rules, as the SDD proposes) instead of repeating the FEN strings; the en passant row cannot be constructed with the rules FEN type (parse error, part A evidence section 8) so the table marks it `parse_error` and the SDD's "pending XC-DOM-A8-5" can close as "rejected before classification" |
| F-B4 | Minor | `EcoFields` uses `Some("")` for "tag kept, no name" and `None` for "no value". The SDD says "empty name" for the first and "no name" for `Neither`. The app and store must keep the two apart (NULL against empty string) |
| F-B5 | Minor | `RosterSeed` width is not in the SDD; chosen `u32` (stores in any integer column, no 2^53 issue for the TypeScript type). `RosterGen::generate` indices are `u32` as `StoreApi::roster_indexes() -> Vec<u32>` |
| F-B6 | Minor | Two bounds that the SDD does not state, both `ESTIMATE`: `MAX_ROSTER_INDEX` = 5 000 (the uniqueness walk recomputes every lower index; a corrupt stored index would otherwise run for billions of steps) and `MAX_NAME_SUFFIX` = 100 000 (the SDD says the name sequence is endless; unbounded, a `taken` that is always true never returns). Both end in `ParameterSpaceExhausted`. The defensive check "against the sets already in the store" is not implemented because the SDD's `generate` signature gives the function no stored sets |
| F-B7 | Info | `check_history` has no sum error. The ELO-1 sum identity `final = initial + sum(after - before)` follows from row-to-row continuity, so a separate branch could never fire (mutation testing would report it equivalent). The three errors are `Gap`, `FirstRowNotInitial`, `NullGameChangesRating` (a reset point must not change the rating). UT-DOM-094 "wrong sum" is tested as a gap |
| F-B8 | Info | `saved_line(node, game_plies)` is underspecified. Implemented: the saved line is the path from the tree root to `node`, anchored at the root's ply; refused with `AtFinalPosition` when the root ply is `>=` the game's ply count, `EmptyLine` for the root node. The SDD should say whether a node of a deeper branch saves the whole path (as here) or only the part after the last main-line node |
| F-B9 | Info | `to_pgn_text` returns `Result<String, TreeError>` (the SDD sketch says `String`) because `chess_rules::format_line` is fallible and the crate denies `unwrap`; the error is not reachable for a tree built through `play` |
| F-B10 | Minor | `StatsRow.result_for_viewed` is `Option` (the SDD has a plain enum): an aborted or unfinished game has no result for anyone. A completed row without a result does not count |
| F-B11 | Info | The SDD names a `Rewound` highlight but not which squares carry it. Chosen: the two last-move squares when the view is rewound; check beats rewound beats last move in the marks (`<>`, `()`, `[]`) and in the Ansi background (red, blue, yellow). Empty-square glyphs: Plain `.`; Unicode and NoColor a middle dot on dark squares, a space on light ones; Ansi a space |
| F-B12 | Info | Class bounds in `CLASS_BOUNDS` are `ESTIMATE` placeholders (SDD says so); uniqueness holds for the 125 sets tested. The golden roster file changes when the bounds are calibrated, as intended before release |
| F-B13 | Info | `RosterGen::default_roster()` takes no `taken` argument (the SDD card has none); the first roster is generated with nothing taken, and its names are unique among themselves |
| F-B14 | Info | `Personality` deserialises through a validating `TryFrom` and rejects unknown fields; `RosterSeed` serialises as a number |

## 9. Assumptions

- ASM-DB1: the SDD sketches are signatures to keep in spirit; names not in the SDD (`CLASS_BOUNDS`, `ClassBounds`, `Span`, `with_bounds`, `personality_at`, `roster_uuid`, `NodeId::from_index`, `RatingPoint` fields, `PlayerStats` field names, `OpponentRef`, `Tally`) are my choice.
- ASM-DB2: the `SplitMix64` vectors and `error_pick_seed` values come from a Python 3 implementation written for this task from the published constants; seed 0 outputs equal the widely published ones (`e220a8397b1dcdaf`, ...) `[RAN]`.
- ASM-DB3: `error_pick_seed` folds the two halves of the id and the ply through three SplitMix64 steps; the SDD gives no formula, so this is the "first implementation" it refers to ("golden values from the first implementation").

## 10. Not done / not verified

- `chess-engine` copy of the vector file and the CI diff (UT-DOM-102, FF-8): the engine crate has no copy yet.
- `ts-rs` feature and the CT-DOM-001 JSON snapshot; `Preferences` (OQ-DOM-004).
- Rules-side drift-guard edit (F-B3); not touched by instruction.
- Branch and MC/DC coverage; mutation testing of part A files other than those listed.
- Performance of `personalities_up_to` at large indices (O(n) with a hash set; bounded by 5 000; not measured).
- No Reviewer pass, no SA desk review, no SE review of the choices. Intel Mac not tested. `npm audit` skipped (offline).
- Mermaid: no diagram was written.
