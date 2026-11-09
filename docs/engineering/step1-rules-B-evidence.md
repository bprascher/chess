# Step 1 evidence: chess-rules part B (chess-a0r.2.4)

Stance: Engineer. Nothing here is a review or an approval. Gate record: [gates/GATE-102-chess-rules.md](gates/GATE-102-chess-rules.md) (G1, OPEN, not approved; part B added as "CHG-102 part B"). Working tree only; nothing committed; beads not touched. Tags: `[RAN]` executed, `[OBS]` read, `[INF]` inferred, `[UNV]` not verified, `[DOC]` fetched documentation. Part A evidence: [step1-rules-A-evidence.md](step1-rules-A-evidence.md).

## 1. Scope and files

Implemented per SDD-chess-rules 0.3.5 DRAFT: DD-RUL-005 (SAN formatter), 006 (move-text parser, `ParseMode::{Typed, Pgn}`), 009 (PGN model), 010 (writer), 011 (reader), 012 (line text), 016 (`canonical_form`), the raw `ECO` tag (`PgnTags::eco_tag`, DD-RUL-009 / UT-RUL-029), and the part B variants of DD-RUL-014 (`MoveParseError` taxonomy, `PgnParseError`, `PgnWriteError`).

**ECO table (added in the second pass, after the PM ruling that DD-RUL-013 / IFP-RUL-6 is in scope):** DD-RUL-013 (`build.rs`, shared key module `src/key_derive.rs`, generator `build/eco_gen.rs`, static table, `eco::lookup`, `EcoEntry`, `TABLE_LEN`) with UT-RUL-023, UT-RUL-030, CT-RUL-003 and CI-RUL-001. Section 11 records the data fetch and the results. **Still not done:** UT-RUL-031 row 5 (the drift-guard table shared with `chess-domain`), left for chess-a0r.2.7 as the PM allowed.

Files (absolute, under `/Users/brianprascher/Projects/chess`):
- New: `crates/chess-rules/src/notation.rs`, `crates/chess-rules/src/pgn/mod.rs`, `crates/chess-rules/src/pgn/reader.rs`, `crates/chess-rules/tests/notation.rs`, `crates/chess-rules/tests/pgn.rs`, this file.
- Edited: `crates/chess-rules/src/{lib,error,position}.rs` (exports, error variants, `Position::san`), `crates/chess-rules/tests/common/mod.rs` (two helpers `nodes`, `game`), `docs/engineering/gates/GATE-102-chess-rules.md`.
- Second pass (ECO): new `crates/chess-rules/build.rs`, `build/eco_gen.rs`, `src/key_derive.rs`, `src/eco.rs`, `tests/eco.rs`, `tests/eco_build.rs`; new `third_party/eco/{a,b,c,d,e}.tsv, COPYING.txt, SHA256SUMS, SOURCE.md`; new `ci/check-eco.mjs`; edited `ci/check.sh` (one step), `crates/chess-rules/Cargo.toml` (`[build-dependencies] shakmaty = "=0.30.2"`), `src/lib.rs`, `src/position.rs` (`PositionKey::to_epd` now calls `key_derive::epd`).
- `Cargo.lock` unchanged (the build-dependency is the same crate and version already locked). `pgn-reader` is **not** a dependency (decision below).
- Not mine: the working tree also shows `docs/engineering/dependency-lookup-step1.md` (modified, amendment 0.3 itemising `shakmaty` transitives), `docs/engineering/ruling-test-helper-placement.md` (new) and `.beads/issues.jsonl` (modified at session start). They appeared from other work during this session. I did not edit them.

## 2. Upstream check (OQ-RUL-001, remainder)

`[OBS]` read from the unpacked crates in `~/.cargo/registry/src`:
- `pgn-reader` 0.29.1: no line or column anywhere in `src/` (grep for `line|column|offset` finds only buffer sizes); SAN is parsed to the upstream `SanPlus` inside the reader. It therefore cannot give `PgnParseError { line, col }`, the lenient forms, or `IllegalReason` text. Per DD-RUL-011 and D-3's escape route, the reader is an **in-crate tokenizer** (`src/pgn/reader.rs`, about the size of the automaton in SDD 3.10). `pgn-reader` is left out of `Cargo.toml`.
- `shakmaty` 0.30.2: `san::SanPlus::from_move(pos, m)` and its `Display` exist (`src/san.rs:607`); `Position::san` uses it after the same lookup as `play` (DD-RUL-002 processing). The lenient parsing is ours.

## 3. Tests first

Red `[RAN]` (first run of `cargo test -p chess-rules --test notation` after writing `tests/notation.rs` and `tests/pgn.rs`, before any implementation): compile failure `error[E0432]: unresolved import crate::notation` (the types did not exist). As in part A, this is a compile red, not an assertion red. After the implementation the first run was green (24 + 43 tests). Real assertion reds seen later, each fixed on the side shown:
1. Two fixtures of mine were illegal FENs (black king on the h-file against the white rook): fixture corrected, not production code.
2. The property test `ut_rul_009_*` (run in the mutation baseline) found a **production defect**: typed `B2d5` read as the coordinate move b2d5 (pawn) when it meant a bishop on rank 2 going to d5. Fixed in `resolve` (upper-case `B` reads as a bishop first when a bishop move matches, else the coordinate), regression test `ut_rul_012_cli_3_b2d5_is_a_bishop_move_...` added.
3. Some of my new tests had wrong expectations (a date with month 13 keeps its year; a column reported after the name instead of at its start; decoy cases in the skipping test). The column report was a small production change (error position now at the start of the name); the others were test fixes.
4. Mutation testing (section 6) drove about 20 additional tests.

## 4. Final gate run

`[RAN] CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` (run before the last test additions) -> exit 0, "All gates passed." Re-run at the end: see section 4a.

4a. Final `[RAN] CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` -> exit 0, "All gates passed." (after the last test additions). `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` and the same with `-p chess-rules --features test-support` clean; `RUSTDOCFLAGS="-D warnings" cargo doc -p chess-rules --no-deps --features test-support` clean; `node ci/check-crate-edges.mjs`: `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`; `cargo deny`: `advisories ok, bans ok, licenses ok, sources ok`. chess-rules targets: unit 8 passed (4 ignored perft, as in part A); `fen` 15; `isolated_data_dir` 4; `notation` 29; `pgn` 63; `properties` 3; `rules` 37 (types 5 per part A). Suppressions: none added in `src/`; the two test files carry the same `#![allow(... reason = ...)]` header as the part A test files.

## 5. Acceptance criteria and SDD mapping

| Criterion | Status | Evidence |
|---|---|---|
| Tests first | Met with the part A caveat (compile red) | section 3 |
| SAN format: pieces, captures, castling (`O-O`, `O-O-O`), promotion, disambiguation (file, rank, both), `+`/`#` | Met | UT-RUL-008: `ut_rul_008_cli_3_san_*` (8 tests) |
| SAN parse: valid forms, either-case files, `0-0`, optional `x`, coordinate, promotion spellings | Met | UT-RUL-010, 014: `ut_rul_010_*`, `ut_rul_014_*` |
| Uppercase-B rule (bishop `B` vs b-file pawn), Typed vs Pgn | Met | UT-RUL-012: `ut_rul_012_*` (4 tests, positions with both captures, pawn only, bishop only; Pgn mode `Bxc3` is an error) |
| `ParseMove` errors per SDD taxonomy (`Empty`, `TooLong`, `Syntax`, `LowercasePiece{suggestion}`, `PromotionRequired{choices}`, `Ambiguous{candidates}`, `NoMatch{reason}`) | Met | UT-RUL-011, 013, 014; error text UT-RUL-024 (`ut_rul_024_nfr_9_part_b_errors_*`) |
| `MatchedMove` computed from the move, not the input | Met | UT-RUL-028 |
| PGN write: Seven Tag Roster order, extras in ASCII order, `SetUp`/`FEN`, `TimeControl`, `Termination`, escapes, `N.`/`N...` rules, lines under 80, result, final newline, deterministic, `write_pgn_many`, never partial | Met | UT-RUL-017 (golden strings), UT-RUL-019 |
| PGN read: multi-game, variations, brace and semicolon comments, NAGs and suffix marks, `0-0`, UTF-8 and Latin-1 per game, BOM, CRLF and lone CR, `SetUp`/`FEN`, missing tags, missing terminator, malformed input with game index, line, column | Met | UT-RUL-020, 021 |
| Per-game recovery; unterminated comment ends the input once | Met | UT-RUL-021 |
| ECO tag handling: raw, unchanged, `None` when absent; ignored by `canonical_form`; the writer never emits `ECO` or `Opening` | Met | UT-RUL-029, UT-RUL-017, UT-RUL-026 |
| Results: movetext terminator authoritative, `Result` tag fallback, else `*` | Met | `ut_rul_020_if_2_result_token_is_authoritative_*` |
| Comments, variations, NAGs as the SDD specifies | Met, with the limits in F-B7 (a comment before the first move has no node) | UT-RUL-020, 022 |
| Line text `format_line` / `parse_line` | Met | UT-RUL-022 |
| `canonical_form` v1 incl. OQ-27 rule | Met | UT-RUL-026, 031 (1) to (4), CT-RUL-004. UT-RUL-031 (5), the shared drift-guard table, is not built (the shared data file is a cross-crate proposal, `[INF]` "step 1 to confirm"); single-field variants for counter, placement, side and castling are in `ut_rul_031_oq_27_another_counter_or_placement_keeps_both_tags`, the en passant variant is not (a FEN with an en passant square after a start position is a different position, F-B8) |
| Golden bytes of `canonical_form` v1 (OQ-RUL-005 (b)) | Done as a proposal | `ut_rul_026_d_42_canonical_form_golden_bytes_of_v1`: two fixed games chosen by me; the SDD did not give any. A change needs the `v2` bump of ALG-APP-09. The owner should confirm or replace the fixtures. OQ-RUL-005 (a), a version constant, not added (F-B9) |
| Round trips | Met | SAN <-> position: UT-RUL-009 (150 random playouts, every legal move, Pgn and Typed and UCI forms). PGN write -> read: UT-RUL-018 (fixtures and 100 random playouts with comments and NAGs), CT-RUL-004 |
| Random-input tests (NFR-3) | Met at the `proptest` level (D-76) | FZ-RUL-001 (3 string strategies x 2 modes, plus hostile fixed inputs), FZ-RUL-003 (arbitrary bytes, PGN-shaped text, mutations of a valid PGN, 50 000-deep nesting, 100 000 closers, 2 MB comment, 2 MB tag value, binary data, a 1 MB token). All assert termination (items at most bytes + 1) |
| Requirement IDs in test names | Met | every test is `ut_rul_nnn_<req>_...`, `fz_rul_nnn_...` or `ct_rul_nnn_...` |
| `Send + Sync` of new public types | Met | `ut_rul_025_architecture_4_8_part_b_types_are_send_and_sync` |
| No `shakmaty` / `pgn-reader` type in the public API (CT-RUL-001) | Met by rustdoc check | section 7 |
| clippy `-D warnings`, `rustfmt --check`, `ci/check.sh` | Met | section 4 |
| ECO table and `eco::lookup` (DD-RUL-013, IFP-RUL-6): snapshot in `third_party/eco/` with source commit, retrieval date, licence and SHA-256; `build.rs` replay with `shakmaty` (build-dependency), fails with file and line, UR-27 tie rule, prints row and duplicate counts, no network; public `eco::lookup`, `PositionKey`, `EcoEntry`, `TABLE_LEN` | Met | section 11; UT-RUL-023 (`tests/eco.rs`, 5 tests), UT-RUL-030, CT-RUL-003 and the duplicate rule (`tests/eco_build.rs`, 4 tests), CI-RUL-001 (`ci/check-eco.mjs`, in `ci/check.sh`) |
| Not met / not in this change | UT-RUL-031 (5) shared drift-guard table with `chess-domain` (left for chess-a0r.2.7, PM-approved) | F-B8 |

## 6. Coverage and mutation (beyond the criteria)

Line coverage `[RAN] cargo llvm-cov -p chess-rules --locked --summary-only` (target: at least 90 % for rules and parser modules, test-strategy): `notation.rs` 97.48 %, `pgn/mod.rs` 99.14 %, `pgn/reader.rs` 96.92 %, `position.rs` 99.45 %, `error.rs` 100 %; crate total 96.96 % of lines (95.99 % of regions). Branch coverage is not reported by this toolchain (the branches column is 0).

Mutation `[RAN] cargo mutants -p chess-rules --file src/notation.rs --file src/pgn/mod.rs --file src/pgn/reader.rs --timeout 40 -j 4` (cargo-mutants 27.1.0):
- Run 1 (first test set, stopped early at 256 of 401 because of 120 s timeouts): 25 missed. I added tests and simplified some code (square construction, promotion split, number parse) after reading the list.
- Run 2 (383 mutants): 209 caught, 131 unviable, 14 timeouts, 29 missed. Before it, the baseline run failed once and exposed the `B2d5` defect (section 3).
- Run 3 (after more tests; `pgn/mod.rs` and `pgn/reader.rs` only, 264 mutants): 158 caught, 85 unviable, 9 timeouts, 12 missed. `notation.rs` was last run in run 2 (1 missed there after the fixes). Timeouts are mutants that make the reader loop; the 40 s limit kills them, which counts as detected `[INF]`.
- Survivors I judge equivalent or defensive, not closed: `PgnResult::from_token` arm `"*"` (the fallback is also `Unknown`); the coordinate form's promotion arm (the SAN path reads the same text to the same move); the progress guard `+=` in `PgnReader::next` (unreachable by construction); `fatal` field deletion in the unterminated-comment errors (the skip then reaches the end of input anyway); `frames.len() - 1` depth in the unbalanced-variation failure (only changes where the skip resumes); `at_line_start` guard polarity in `skip_to_terminator`, `movetext` `[` guard and `skip_between_games` (skip heuristics whose observable effect needs inputs I did not find); `b'}'` arm in `skip_to_terminator` (the word path consumes the byte too); `== -> !=` at `skip_to_terminator` line 476 and `&& -> ||` at line 492 (same family). Not proven equivalent for each; I did not write a test for them. `[INF]`

## 7. CT-RUL-001: upstream types in the public API

`[RAN] RUSTDOCFLAGS="-D warnings" cargo doc -p chess-rules --no-deps --features test-support` -> finished, no warning. `grep -rl shakmaty target/doc/chess_rules` -> only `index.html`, whose text is the crate-level sentence "`shakmaty` types never appear in the public API"; no item page names `shakmaty` or `pgn_reader`. `pgn_reader` is not a dependency. `[OBS]` `grep -n "use shakmaty" src` is limited to `position.rs`, `fen.rs` (part A) and the test module of `testing.rs`; the new modules `notation` and `pgn` import no upstream crate. This is a rustdoc grep, not a `cargo public-api` run (tool not chosen, `[UNV]`).

## 8. New shape choices and findings (for the Reviewer, the SA desk review and owners)

F-A2 (chess-4c7) asked for part B to stay consistent with part A shapes and for new choices to be listed.

| ID | Severity | Finding |
|---|---|---|
| F-B1 | Minor (needs SA confirmation, F-A2) | `MoveParseError` (part A: `BadUci` only, `Copy`) now has `BadUci`, `Empty`, `TooLong`, `Syntax`, `LowercasePiece{suggestion: String}`, `PromotionRequired{choices: Vec<PieceKind>}`, `Ambiguous{candidates: Vec<San>}`, `NoMatch{reason: IllegalReason}`. It is no longer `Copy` (payloads own data). `TooLong` carries no length (D-51 gives `too_long` no field). `suggestion` is the whole corrected text (`nf3` -> `Nf3`); `choices` are `Queen, Rook, Bishop, Knight` in that order; `candidates` are sorted by UCI text, so the order is stable. Part A callers (`Move::from_uci` and its tests) are unchanged. No consumer exists yet |
| F-B2 | Minor | New public types and their shapes: `San(String)` (opaque, `as_str`, `Display`, not constructible outside the crate), `ParseMode`, `MatchedMove{mv, uci, san}`, `PgnGame{tags, start: Position, moves, result}`, `PgnTags{event, site, date, round, white, black: String; time_control, termination; extras: BTreeMap}`, `PgnNode{mv, nags: Vec<u8>, comment: Option<String>, variations}`, `PgnDate` (private fields, getters), `PgnResult`, `Termination`, `TimeControl{seconds, increment}`, `PgnLimits{max_variation_depth}`, `PgnErrorKind`, `PgnParseError{game_index, line, col, kind}`, `PgnWriteError`. The Seven Tag Roster values are `String` with the writer defaults as `Default` (so a write and re-read is exact); `Result` is `PgnGame::result` only. `SetUp` and `FEN` are not tags in the model: `start: Position` carries them (the writer and `canonical_form` derive them, so the "FEN that does not parse stays" case of UT-RUL-031 (4) cannot exist at the model level; the reader fails such a game with `BadFen`) |
| F-B3 | Minor | `Termination` text: the writer emits the PGN-standard lower case values `normal`, `time forfeit`, `abandoned`; the reader accepts any letter case. The SDD names the enum, not the text. The PGN standard wording is recalled, not re-read `[UNV]` |
| F-B4 | Info | In-crate tokenizer instead of `pgn-reader` (section 2). `pgn-reader` was vetted in `dependency-lookup-step1.md` but is now unused, so it is not in `Cargo.toml` or the SBOM for this crate. If the SDD's text "wraps `pgn-reader`" is to stay, the SDD needs an amendment (not edited here) |
| F-B5 | Minor (reader leniencies and one addition, all inside the SDD's silence) | (a) A `FEN` tag is honoured unless `SetUp` is `0` (SDD 3.11.8 says "if `SetUp "1"` and `FEN` are present"); I chose this because ignoring a custom FEN replays the moves from the wrong position, and UT-RUL-031 (4) expects a FEN-alone case. (b) A tag section at a line start ends a game that has no terminator, so one unterminated game does not swallow the next. (c) Error recovery refines SDD 3.10 "Skipping": quoted strings are skipped, and for a movetext error a tag section at a line start also ends the skip. (d) A repeated tag keeps the last value. (e) Tag names must start with an upper-case letter (same rule as the writer) else `BadTag` |
| F-B6 | Minor | Lossy import of typed tags: a `Date` that is not `YYYY.MM.DD` becomes unknown (a part out of range becomes unknown, the others stay); a `TimeControl` that is not `seconds[+increment]` (such as `40/7200`) and a `Termination` outside the three values (such as `Unterminated`) are dropped. They are not kept in `extras` because the writer rejects reserved names. Two imported games that differ only in such a tag have equal `canonical_form` when the dropped tag was the only difference (for `TimeControl` this changes the hash input of an imported game relative to a game that kept it). The SE/SA should decide whether the model needs raw values (G1 inside the crate, but it changes the data the app stores in `GAME_TAG`) |
| F-B7 | Minor | A comment before the first move of a sequence (before the first move of the game, or right after `(`) has no node to attach to and is dropped; several comments after one move are joined with a space; line breaks inside comments become spaces; comment text is trimmed, an empty comment is `None`. The writer replaces `}` and control characters in a comment with a space (ASM-RUL-004 style) |
| F-B8 | Info | UT-RUL-031 (5), the shared drift-guard table with `chess-domain` (UT-DOM-115), is not built: the SDD calls its source "proposed, step 1 to confirm" and it spans two crates. Not attempted; a cross-crate test data file is a decision for the owner. `canonical_form` compares `start.to_fen()` with the standard FEN on all six fields (the `Fen` type compares normalised values, part A) |
| F-B9 | Minor | OQ-RUL-005: (a) no version constant added (SDD leaves it to a G1 decision at the code step; adding one is cheap, but the app's `v1` string is its own, so I did not); (b) golden bytes added as a proposal (section 5). `canonical_form` on a game whose mainline has an illegal move stops the movetext at that move instead of returning an error (the function is total by SDD 3.11.10); a game from the reader or built from `legal_moves` never has one |
| F-B10 | Closed | Raised in the first pass (ECO table not delivered, no data in the repository). The PM ruled it in scope; delivered in the second pass, section 11 |
| F-B11 | Minor | Typed `B2d5` and similar texts are both a coordinate move and a bishop move with a rank hint; the rule I implemented (bishop first when it matches) is an extension of Q44 found by the property test. The same ambiguity exists for any uppercase-file coordinate text but only `B` collides with a piece letter. SE should confirm (CLI-3) |
| F-B12 | Minor | Over-disambiguated SAN is accepted in both modes (`Ng1f3`, `e2e4` as SAN, `Rxd1`-style with a needless file). In `Pgn` mode these are still legal moves, so no illegal SAN is accepted (FM-RUL-003), but "strict import" is strict about legality, not about canonical spelling. Lower-case `x` only; `0-0` accepted in `Pgn` mode too (needed for real files) |
| F-B13 | Info | A `Position` with `PartialEq` does not exist, so `PgnGame` has no `PartialEq`; tests compare tags, nodes, results and the FEN |
| F-B14 | Info | The writer's variation depth limit (64) is a constant (`DEFAULT_MAX_VARIATION_DEPTH`) shared with `PgnLimits::default()`; `write_pgn` takes no limit argument (IFP-RUL-5 shows none). A caller passing a huge `max_variation_depth` to the reader can get a deeper tree than the writer accepts, and recursive drop of a very deep tree is the caller's risk |
| F-B15 | Info | `write_pgn` / `canonical_form` replay each game: cost is O(moves x legal-move generation); not measured `[UNV]` |

No G2 matter was found: no dependency edge, §5.7 contract or protocol change was needed.

## 9. Assumptions

ASM-B1: the lenient/strict split of `ParseMode` is only the Q44 fallback, the coordinate form and the NAG request (SDD DD-RUL-006 rationale). ASM-B2: an upper-case file letter is accepted in SAN text in both modes (SDD 3.11.1 step 5). ASM-B3: `!`/`?` suffix marks map to NAGs 1 to 6; other mark strings (`!!!`) are ignored. ASM-B4: the PGN standard's `Termination` and `TimeControl` text forms as in F-B3 and F-B6 `[UNV]`. ASM-B5: a 1-based `game_index` counts every game start including failed ones; `parse_line` errors use `game_index` 1. ASM-B6: `line`/`col` of a `BadTag` for an unclosed value point to the opening quote; of other tag errors to the offending token; of a movetext error to the first byte of the offending token.

## 10. Not run / not verified

- Intel Mac and other platforms: not run. macOS arm64 only.
- `npm audit` skipped by `CHESS_CI_OFFLINE=1`; `cargo deny` used the cached advisory database.
- The PGN standard text and the FIDE Laws were not re-read; the writer format follows SDD 3.11.7 and the golden strings in my tests, which I wrote from the SDD. An independent PGN reader (RSK-RUL-003 mitigation, IF-1 inspection) was **not** run on the writer output. `[UNV]`
- No benchmark; 10 MB import time not measured (SDD 3.12 says so too).
- No coverage-guided fuzzer (D-76: accepted loss). Property runs are 100 to 150 cases each, plus the fixed hostile inputs.
- CT-RUL-002 (consumer view builds against the public API): no consumer crate exists yet.
- Reviewer pass and System Architect desk review of part B: not done.
- Mutation testing covered `notation.rs`, `pgn/mod.rs`, `pgn/reader.rs` (before the ECO pass); `Position::san`, `eco.rs`, `key_derive.rs` and `build/eco_gen.rs` were not mutated.
- The tie rule for duplicate positions (UR-27) never fires on the real snapshot (0 duplicates, section 11); it is tested on synthetic TSV copies only.
- The SDD's wording that `build.rs` "uses the shared key module by `include!` or `#[path]`" is realised with `#[path]`; whether the shared module compiles in both contexts without a second copy of the `shakmaty` types (the SDD's `[UNV]`) is now `[RAN]`: it does, because the module takes plain values.

## 11. ECO table (second pass)

### 11.1 Data fetch `[RAN]` (date 2026-11-06 UTC)

- `curl https://api.github.com/repos/lichess-org/chess-openings/commits/master` -> commit `3ba6b20d31b2e6125cb0e6a2e792f400717d9d39` (merge of PR #417, 2026-11-05T21:00:07Z).
- `curl -sSf https://raw.githubusercontent.com/lichess-org/chess-openings/3ba6b20d.../{a,b,c,d,e}.tsv` and `.../COPYING.txt` into `third_party/eco/`. The tree listing of that commit (API `git/trees`) shows `COPYING.txt` and no file named `LICENSE`. The upstream README (read) states the data is public domain and released under CC0.
- `shasum -a 256` (also in `third_party/eco/SHA256SUMS`): a.tsv `3282e4c9...f793`, b.tsv `2744a389...1c48`, c.tsv `447bbd4b...06ef`, d.tsv `1cf4057a...cceda`, e.tsv `48c6f664...8546`, COPYING.txt `a2010f34...0499` (full values in the file).
- Lines: 824, 782, 1251, 645, 368; 3865 data rows. The files are unmodified copies. Source, licence reference and refresh procedure: `third_party/eco/SOURCE.md`. This also closes the "CC0 claim stays `[UNV]` until the file is read" note of architecture §3.4 for this commit: the upstream README and `COPYING.txt` were read; no independent legal review was done `[UNV]`.

### 11.2 Design as built

- `src/key_derive.rs`: `epd(piece_at, white_to_move, castling, en_passant)` over plain values; used by `PositionKey::to_epd` (our types) and by the generator (`shakmaty` values). Written once.
- `build/eco_gen.rs`: `generate(dir)` reads `a.tsv`..`e.tsv`, checks the header and three columns, replays each `pgn` with `shakmaty::san::San` (skipping move-number tokens), derives the key, applies UR-27 (longer line, then smaller code, then smaller name), renders `TABLE_LEN` and a sorted `static TABLE`. Errors read `a.tsv:3: illegal SAN move: "Nh5" in "..."`. The file is included by `build.rs` and by `tests/eco_build.rs`, so the tests run the build-time code itself.
- `build.rs`: reads only `../../third_party/eco`, writes `OUT_DIR/eco_generated.rs`, `cargo:rerun-if-changed` on the data, itself and the two sources; prints `cargo:warning=ECO table: 3865 rows, 3865 positions, 0 duplicate positions` `[RAN]`; exits non-zero with the generator's message on failure. No network code.
- `src/eco.rs`: `EcoEntry{code, name}` (Copy), `TABLE_LEN`, `lookup(&PositionKey) -> Option<EcoEntry>` by binary search on the EPD text.

### 11.3 Results `[RAN]`

- Real snapshot: 3865 rows, 3865 distinct positions, 0 duplicate positions (the upstream data has one row per position). `TABLE_LEN` = 3865.
- UT-RUL-023: for every one of the 3865 rows, the position reached with `parse_line` and `Position::play` finds an entry with the expected code and name, and `TABLE_LEN` equals the number of distinct keys (this catches any disagreement between the build-time `shakmaty` replay and our parser, play and key). Fixed openings (`1. Nh3` is A00 "Amar Opening", a row named "Italian Game"), two transposition pairs, the unnamed standard start and an unnamed gambit line, counters and unusable en passant squares ignored.
- UT-RUL-030: a custom-start FEN equal to a reached named position has the same key and entry; a FEN equal to the standard start has the same key and answer as the start.
- CT-RUL-003 (generator level): an illegal SAN reports `a.tsv:4` and the token `Nh5`; a two-column row, a wrong header, an empty pgn and a missing file each fail with file (and line). **End to end** `[RAN]` (manual, restored afterwards): editing line 3 of `third_party/eco/a.tsv` to `1. Nh5 d5 ...` made `cargo build -p chess-rules --offline` fail with `error: failed to run custom build command for chess-rules` and `ECO table generation failed: a.tsv:3: illegal SAN move: "Nh5" in "1. Nh5 d5 2. g3 e5 3. f4"`; after restoring the file `node ci/check-eco.mjs` -> OK. There is no automated test that runs `build.rs` itself on a corrupted copy; the generator tests cover the same code.
- UR-27 rule: synthetic TSV copies (a 1-ply and a 5-ply line reaching one position; equal-length lines with different codes and names) keep the longer line, then the smaller code, then the smaller name.
- CI-RUL-001: `node ci/check-eco.mjs` -> `eco snapshot check OK: 6 files match SHA256SUMS`; on a copy with one appended line in `c.tsv` it exits 1 with `FAIL c.tsv: SHA-256 is b3d99f33..., recorded 447bbd4b...`. The script is step "ECO snapshot checksums (CI-RUL-001)" in `ci/check.sh`.
- Gates `[RAN]`: `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` -> exit 0, "All gates passed" (fmt, build, clippy `-D warnings` over all targets including `build.rs`, tests, edge check `OK: 8 crates, 11 workspace edges`, ECO checksum step, `cargo deny` `advisories ok, bans ok, licenses ok, sources ok`). No change to `deny.toml` or `ci/check-crate-edges.mjs` was needed: a build-dependency on `shakmaty` is not a workspace edge. `Cargo.lock` is unchanged. Tests now: `eco` 6, `eco_build` 4, `notation` 29, `pgn` 63, others as before.
- Coverage `[RAN] cargo llvm-cov -p chess-rules --locked --summary-only` (lines): `eco.rs` 100 %, `key_derive.rs` 100 %, `notation.rs` 97.48 %, `pgn/mod.rs` 99.14 %, `pgn/reader.rs` 96.92 %, `position.rs` 99.39 %; crate 96.99 %. `build.rs` and `build/eco_gen.rs` are not in the instrumented library report (build script); `eco_gen.rs` is exercised by `tests/eco_build.rs` (included there, so its lines are not attributed in this table `[UNV]` as a number).
- Public API: `RUSTDOCFLAGS="-D warnings" cargo doc` clean; `grep -rl shakmaty target/doc/chess_rules` -> only the crate-level sentence in `index.html`.

### 11.4 New findings

| ID | Severity | Finding |
|---|---|---|
| F-B16 | Minor | The upstream data has no duplicate positions (0 of 3865), so UR-27's tie rule is dead code on this snapshot. It is kept and tested synthetically because a later snapshot may differ. |
| F-B17 | Minor | `build.rs` prints its counts with `cargo:warning=`, which shows as a `warning:` line on every rebuild of the crate. It is not a rustc warning, so `-D warnings` does not trip. If the owners dislike the noise, the counts could go to a file in `OUT_DIR` instead. |
| F-B18 | Info | `third_party/eco/COPYING.txt` is the upstream filename; the SDD and architecture speak of a "notice file" and "licence file present" (CI-RUL-001). `SOURCE.md` plus `COPYING.txt` are my reading of that. `ci/check.sh` gained one step (`ci/check-eco.mjs`): the minimal CI change for CI-RUL-001; report to the owner of `ci/`. |
| F-B19 | Info | The SDD sentence "`pgn-reader` ... wraps" (SDD 1.2 / architecture §3.3) and the SDD statement that `build.rs` replays "with `parse_move`" (already corrected by D-75) are wording issues; the SDD was not edited. |
| F-B20 | Info | The ECO key is the EPD of `PositionKey` (placement, side, castling, legal en passant). Lichess's own `epd` column (dist/) uses its own castling field convention; we do not use it, so no comparison with it was made `[UNV]`. |
