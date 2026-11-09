# chess-rules conditions C-R3 to C-R6: evidence (2026-11-09)

Engineer stance, software-engineer. Gate: GATE-102 (G1, desk review PASS WITH CONDITIONS, see `desk-review-G1-step1.md`). Decisions in force: Brian Prascher, G-3 passed with conditions, 2026-11-09 (B1 freeze `canonical_form` v1 and fix the illegal tail without changing any byte of a legal game; control-character collapse accepted; B6 warnings list). **Nothing here is an approval.** No commit, no push, no beads change.

Tags: `[RAN]` executed here, `[OBS]` read in the code, `[INF]` inferred, `[UNV]` not verified.

## 1 Summary

| Item | Bead | Result |
|---|---|---|
| C-R3 custom-start golden, RF-05 (a) marker, control-character note | chess-f6t | Met |
| C-R4 single source drift-guard table | chess-maf | Met |
| C-R5 taxonomy cleanup, B6 warnings | chess-pdc | Met, except per-payload Latin-1 (deliberately not done, finding C-R5-F1) |
| C-R6 `ParseMode::Pgn` doc, `e.p.` | chess-94x | Met |
| Hard stop (any change to a legal game's `canonical_form` bytes or the v1 goldens) | | Not triggered; proof in 2.2 |

## 2 C-R3 (chess-f6t)

### 2.1 What changed
- `src/pgn/mod.rs` `canonical_form`: the two `break`s on an illegal move now push the token `{illegal <ply> <uci> <uci> ...}` first (`illegal_tail_marker`: the 0-based index of the first illegal move, then that move and every later mainline move in UCI text), then the loop ends and the result token follows as before. The `san` and `play` calls are taken together, so there is no separate branch for "san ok, play failed" [OBS].
- Doc comment of `canonical_form` now describes the marker and the accepted control-character collision (a tab and a space in a tag value give the same bytes; ASM-RUL-004; accepted by Brian Prascher 2026-11-09).
- `CANONICAL_FORM_VERSION` still `1` (`tests/rework.rs::rf_05_canonical_form_version_constant_is_1` and `tests/canonical_v1.rs` assert it).
- New `tests/canonical_v1.rs` (8 tests).

### 2.2 Proof that no legal-game byte changed
1. The marker contains `{`; no legal output can contain `{` (tag values are escaped text but live in the tag section; the movetext holds only move numbers, SAN and the result). Test `c_r3_rf_05_the_marker_cannot_occur_in_a_legal_game` checks it over the corpus [RAN]. This is an argument by construction [INF] plus the corpus test.
2. Digest pinned from the code **before** the marker existed. First run, on the old `canonical_form`: `digest is 0xb679052c13383ab7` (test `c_r3_legal_game_corpus_bytes_are_those_of_v1`, which then failed against the placeholder `0`) [RAN]. After the change the same test passes with the constant `0xb679_052c_1338_3ab7` [RAN]. Corpus: 400 games from six starts (standard, castling-rich, promotion-rich, black to move, en passant available, 99-move-clock), up to 90 random legal plies (SplitMix64, fixed seed, moves sorted by UCI), all four results, random tags (quotes, backslash, tab, non-ASCII, empty), optional date and `TimeControl`, NAGs and comments on some nodes. Hash: FNV-1a 64 over `len(8 bytes LE) || bytes` of every game.
3. The existing goldens `ut_rul_026_d_42_canonical_form_golden_bytes_of_v1` (two games) and `..._includes_set_up_fen_for_a_custom_start_in_ascii_order` pass unchanged; `git diff` of `tests/pgn.rs` has no hunk touching `GOLDEN_V1` or those tests [OBS grep `GOLDEN` in the diff: no match].
4. Every pre-existing canonical test passes: `cargo test -p chess-rules` all green (see 7.1) [RAN].
5. New custom-start golden (exact bytes) `c_r3_custom_start_golden_bytes_of_v1`: `FEN` line, then `SetUp`, then `TimeControl`, `7. e4 Kd7 0-1`. It passed on the old code before the marker existed (first run: 5 passed incl. this one; 3 marker tests failed) [RAN]. A second test pins a Black-to-move custom start (`12... Kd8 13. e4 *`).

### 2.3 Red then green
Before the change: `c_r3_rf_05_an_illegal_tail_is_marked_not_dropped`, `..._games_that_differ_only_in_an_illegal_tail_no_longer_collide`, `..._an_illegal_first_move_for_black_and_a_promotion_in_the_tail` FAILED (the output was `1. e4 e5 *`, byte-identical for different tails) [RAN]; after: 8 passed [RAN].

### 2.4 Control-character collapse
Test `c_r3_control_character_collapse_is_accepted_and_pinned`: tab, LF, CR, BEL and U+0085 in a tag value give the same bytes as a space [RAN]. Documented in the `canonical_form` doc comment. `tag_line` is unchanged.

### 2.5 Marker limits `[INF]`
Two games with the same legal prefix and illegal tails that spell the same UCI text collide only if the tails are the same text, which is the same game. A game whose tag value contains `{` cannot collide with a marker because tags are in the header. The marker is not a SAN, so it can never be read back as a move; `canonical_form` is a hash input, never parsed.

## 3 C-R4 (chess-maf)

- `git mv crates/chess-domain/tests/data/fen_drift_guard.txt crates/chess-rules/tests/data/fen_drift_guard.txt` (content unchanged; **the rename is staged in the index** by `git mv`, nothing is committed).
- `crates/chess-rules/tests/pgn.rs`: `const DRIFT_GUARD_TABLE = include_str!("data/fen_drift_guard.txt")`; two new tests `ut_rul_031_5_*`: the table has 7 rows with `standard` first; every row, read as `[SetUp "1"][FEN "..."]`, keeps `[FEN][SetUp]` in `canonical_form` unless it is the standard FEN, and the en passant row fails as `BadFen`. The three-FEN inline loop it replaces was removed from `ut_rul_031_oq_27_another_counter_or_placement_keeps_both_tags`; the table covers those rows [OBS].
- `crates/chess-domain/tests/drift_guard.rs`: `include_str!("../../chess-rules/tests/data/fen_drift_guard.txt")` (test-only, no Cargo edge); the grep of `chess-rules/tests/pgn.rs` (`RULES_PGN_TESTS`, `every_fen_the_rules_test_uses_is_a_row_of_the_table`) is removed. `check_table` returns `Err` naming the first disagreeing row; `ut_dom_115_sf_02_the_check_is_red_when_an_expectation_or_a_row_diverges` shows a flipped expectation, a two-field row and a dropped row are red [RAN: 2 passed].
- Red-if-diverge shown on the shared file itself (edit, run, restore; file restored byte-identical, `cmp` against a scratchpad copy): changing the `castling` row's expectation to `standard` made `ut_dom_115_sf_02_drift_guard_table_expected_start_kinds` FAIL with `Err("castling: classifier disagrees with standard")` [RAN]. A harmless edit (another valid single-field castling value) stays green on both sides, correctly [RAN].
- Only the domain test file was rustfmt-ed (`rustfmt --edition 2024 crates/chess-domain/tests/drift_guard.rs`).
- Note [INF]: a published `chess-domain` package would not contain `../../chess-rules/tests/...`; `publish = workspace` is checked by the workspace, not by me [UNV].

## 4 C-R5 (chess-pdc)

### 4.1 Public API changes (complete list)
| # | Change | Kind |
|---|---|---|
| 1 | `PgnErrorKind::BadSyntax { detail: &'static str }` becomes `BadSyntax { reason: SyntaxReason }` | breaking (field name and type); no consumer in the workspace [OBS grep] |
| 2 | `PgnErrorKind::BadTag { detail: &'static str }` becomes `BadTag { reason: TagReason }` | breaking, same |
| 3 | New `pub enum SyntaxReason` (Copy): `NotAMove, NotText, LowercasePieceLetter, StrayClosingBrace, VariationWithoutMove, BadNag, NagWithoutMove, TagSectionWithoutMovetext, Internal` | additive; `Display` text equals the old `detail` text (message tests unchanged and green) |
| 4 | New `pub enum TagReason` (Copy): `NameStart, ValueNotQuoted, ValueNotClosed, MissingClosingBracket` | additive |
| 5 | New `IllegalReason::NotAPromotion` | additive variant of a non-`#[non_exhaustive]` enum |
| 6 | `IllegalReason` returned by `explain_illegal` / `play` / `parse_move` for a promotion piece on a move that does not promote: `NotAPromotion` instead of `CannotReach { piece: <mover> }`; a king or pawn as the chosen piece on a non-promoting move is now also `NotAPromotion` (it was `CannotReach { piece: chosen }`); impossible geometry still reports `CannotReach` first; a king or pawn chosen on a real promotion is still `CannotReach { piece: chosen }` | behaviour |
| 7 | New `pub enum ImportWarning` (`CommentDropped { line, col }`, `DateIgnored { value }`, `TimeControlIgnored { value }`, `TerminationIgnored { value }`, `Latin1Fallback`), `Display` | additive |
| 8 | New public field `PgnGame::warnings: Vec<ImportWarning>` (empty from `PgnGame::new`) | additive for readers; breaks struct-literal construction of `PgnGame` outside the crate (none exists in the workspace [OBS grep]); `PgnGame` derives no `PartialEq`, so equality is unaffected |
| 9 | `canonical_form` output for a game with an illegal mainline move (marker) | behaviour, only for inputs a reader never produces |
| 10 | PGN reader accepts `e.p.`, `e.p` (any ASCII case) after a capture and glued `exd6e.p.`; the matcher `ParseMode::Pgn` is unchanged | behaviour (C-R6) |
| 11 | Comments with text found between games are reported as a warning on the next game | behaviour (B6) |

`FenError`, `FenField`, `IllegalPosition`, `MoveParseError`, `PgnWriteError`, `PgnParseError` shapes are unchanged. No new dependency; `Cargo.lock` unchanged (`git status --short Cargo.lock` empty) [RAN]. `FenError::BadField { detail }` keeps its free text: its details are one per check, per field, and `field` already carries the stable part; promoting them is not cheap and not needed by D-51 [INF, not done].

### 4.2 Promoted reasons
The ten `BadSyntax` reasons of RF-07 are now: `StrayClosingBrace`, `VariationWithoutMove`, `BadNag`, `NagWithoutMove`, `TagSectionWithoutMovetext`, `LowercasePieceLetter`, `NotAMove` (used for `Empty | TooLong | Syntax | BadUci` from the move matcher), `NotText`, and `Internal` (the reader's own "no open frame" bookkeeping failure, which no input reaches [INF]; three uses remain in `movetext`). Tests: `tests/taxonomy.rs` (5 tests, one row per reason, plus message wording kept) [RAN].

`kind_from` still drops the `LowercasePiece` suggestion text [OBS]; the reader error carries only the reason.

### 4.3 `NotAPromotion` (RF-06 note)
`tests/rework.rs::rf_06_a_promotion_letter_on_a_non_promoting_move_is_not_other` was changed on purpose (it pinned `CannotReach { Pawn }` for `e2e4q`); so was the same assertion in `tests/rules.rs` (`...black_castling_reasons_and_other_cases`). The tests were written first against a variant that did not exist (compile-red), then the code was added. Message: "that move does not promote a pawn, so it takes no piece letter".

### 4.4 `fen.rs` dead arms (RF-07)
The five `ParseFenError` arms (`InvalidTurn`, `InvalidCastling`, `InvalidEpSquare`, `InvalidHalfmoveClock`, `InvalidFullmoves`) and the `_ => Placement` default are replaced by one collapse `map_err(|_| bad(Placement, "not accepted"))`. Justification: the review's mutation run showed all five arms deleted with no test failing (RF-07, section 1 of the review `[RAN by the Reviewer]`); I did **not** re-run that mutation; instead I added a sweep `c_r5_rf_07_prechecked_fens_never_reach_an_upstream_parse_error` (4000 proptest cases of structurally valid FENs whose fields pass the crate's own checks) that asserts the "not accepted" fallback never appears [RAN, passed]. That is evidence, not proof, that the upstream reader cannot fail after the pre-checks [INF]. One arm remains as a defensive collapse; it is still unreachable by any known input, which is the 3 uncovered lines of `fen.rs` in section 7.2 (one of them) [INF].

### 4.5 B6 warnings
- `PgnGame.warnings` (field), reported cases (RF-04): (1) a comment with text before the first move of a sequence, at the start of the movetext or after `(`: `CommentDropped { line, col }`; (1b) a comment with text read between games (before the tags, or before the movetext of a tag-less game) is held and reported on the next game; a comment after the last game belongs to no game and is not reported [OBS test `b6_a_comment_between_two_games...`]; (2) `Date` whose text differs from the re-rendered value (unparseable, wrong width, or a part out of range): `DateIgnored`; (3) `TimeControl` that does not parse, except `-` and `?`: `TimeControlIgnored`; (4) `Termination` outside the three written values: `TerminationIgnored`; (5) whole-game Latin-1: `Latin1Fallback` (first in the list).
- Order: between-game comments, then the Latin-1 marker, then tag and comment warnings in the order met (the Latin-1 pass re-reads the game, so the warnings come from the second pass).
- Warnings do not change `canonical_form` or `write_pgn`: test `b6_warnings_never_change_canonical_form_or_the_written_pgn` [RAN]; `canonical_form` does not read the field [OBS]; and the corpus digest test passes [RAN].
- Tests: `tests/warnings.rs`, 13 tests [RAN].

### 4.6 Per-payload Latin-1: NOT done (finding C-R5-F1)
Switching to per-payload decoding is small in code (the decoder already decodes per payload), but it **changes the bytes of the game read from a mixed file**: a valid UTF-8 tag value such as `Zoë` in a game that also holds one Latin-1 byte is now read as `ZoÃ«` (whole-game rule, IF-2), and would be read as `Zoë`. That changes the `White` string, hence `canonical_form` and `content_hash` of that import. Brian's stop condition forbids a change of canonical bytes for a legal game; a game imported from such a file is a legal game. I therefore only added the warning and a characterization test that pins the current whole-game behaviour and is marked SUSPECT (`b6_characterization_latin_1_is_still_applied_to_the_whole_game_suspect`). Decision needed from Brian: switch before the first stored hash (a content change, a v1 matter) or keep and document. Cost of switching if allowed: about 10 lines in `reader.rs` (drop the two-pass `Decoder` and the `all_utf8` return) plus replacing that test [INF].

## 5 C-R6 (chess-94x)

- Doc comment of `ParseMode::Pgn` rewritten (`src/notation.rs`): states the guarantee (only moves of `legal_moves()`, `B` always a bishop) and the actual accepted and rejected spellings, each pinned by `tests/c_r6.rs::c_r6_pgn_mode_accepts_the_listed_spellings` (`e4`, `E4`, `e2e4`, `Ng1f3`, `Nf3+++`, `Nf3!?`, `0-0`, `O-O-O`, `a8=Q`, `a8Q`, `a8q`) and `..._does_not_take_coordinate_moves...` (`g1f3`, `nf3` rejected); `Nxf3` for a non-capture was already pinned in `tests/notation.rs` [OBS]. Whether the PGN Standard's import grammar allows each spelling is `[UNV]` (unchanged from RF-08).
- `e.p.` in the reader (`src/pgn/reader.rs`): a word `e.p.` or `e.p` (ASCII case-insensitive) right after a capture move (comments and NAGs in between keep the state) is skipped; `exd6e.p.` is accepted when the matched move is a capture; anywhere else (after a non-capture, before any move, twice, at the start of a variation, glued onto a non-capture) it is still `BadSyntax { NotAMove }` and the game fails. Tests `c_r6_e_p_after_a_capture_is_an_annotation` (6 spellings, Black in a variation) and `c_r6_e_p_anywhere_else_is_still_an_error` [RAN].
- The matcher (`parse_move`, both modes) does not accept `e.p.`; only the reader does.

## 6 Findings

| ID | Severity | Finding | Needs |
|---|---|---|---|
| C-R5-F1 | Minor | Per-payload Latin-1 decoding would change `canonical_form` input for mixed-encoding files; not done (4.6) | Brian: switch or keep, record in SDD |
| C-R5-F2 | Info | `FenError::BadField.detail` is still free text; `SyntaxReason::Internal` has three unreachable-by-input uses in `movetext` | none; D-51 may need `BadField` codes later |
| C-R5-F3 | Info | Comments between games are attributed to the following game, which can mislabel a trailing annotation of game N as a warning of game N+1 | accept or ask the SE |
| C-R6-F1 | Info | Mutation (section 7.3): two survivors are equivalent by reading (the fatal flag at end of input; `>` against `>=` in `strip_glued_en_passant` gives the same `NotAMove` error for the bare word) | none |

## 7 Commands and results

### 7.1 Tests
- `cargo test -p chess-rules` [RAN]: lib 8 passed 4 ignored; c_r6 4; canonical_v1 8; eco 6; eco_build 4; fen 15; isolated_data_dir 4; notation 29; pgn 65; properties 3; rework 15; rules 37; taxonomy 5; types 5; warnings 13; doctests 0. All `ok`.
- `cargo test -p chess-domain --test drift_guard` [RAN]: 2 passed.
- Deep perft: `cargo test -p chess-rules --release -- --ignored` [RAN]: `4 passed; 0 failed; 8 filtered out; finished in 5.08s` (start d5, start d6 and position 3 d6, kiwipete d5, positions 4/5/6 d5).
- `cargo fmt -p chess-rules -- --check` [RAN]: clean. `RUSTFLAGS="-D warnings" cargo clippy -p chess-rules --all-targets --locked -- -D warnings` [RAN]: finished, no warnings.
- `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` [RAN]: exit 0, last line `All gates passed.` (fmt --all, build, clippy, tests workspace, crate-edge check, ECO checksums, cargo deny; npm audit skipped by design). It ran while the other engineer's domain edits were in the tree; it passed at that moment. The `Unlicense` licence-not-encountered warning from cargo-deny is pre-existing.
- `Cargo.lock`: unchanged [RAN `git status --short Cargo.lock` empty].

### 7.2 Coverage
`cargo llvm-cov clean --workspace` then `cargo llvm-cov -p chess-rules --summary-only` [RAN]. (A first run without the clean printed `135 functions have mismatched data` and a wrong 60.41 %; the clean run is the one reported.) Lines: `TOTAL 2103 lines, 49 missed, 97.67 %`; regions 96.71 %; functions 96.31 %. Per file lines: eco 100, error 100, fen 98.00, key_derive 100, notation 97.48, pgn/mod 99.20, pgn/reader 97.62, position 99.10, testing.rs 77.46 (test-support helper, as before), types 100. Required: at least 90 % [met]. The reviewer's earlier figure was 97.08 %.

### 7.3 Mutation (scoped to the changed code)
`cargo mutants -p chess-rules -F 'illegal_tail_marker|canonical_form|note_comment|is_en_passant_note|strip_glued_en_passant|attach_comment|RawTags|ImportWarning' -j 4 -- --tests` [RAN]: `48 mutants tested in 3m: 3 missed, 20 caught, 25 unviable`. Missed: `reader.rs` delete field `fatal` in `movetext` and in `skip_between_games` (the unterminated comment ends at end of input, so the next call ends the iteration either way: equivalent [INF]); `>` to `>=` in `strip_glued_en_passant` (the bare word `e.p.` then fails as `NotAMove` through the empty-word path: same error [INF]). Not mutated: the whole crate, `taxonomy`/`NotAPromotion` code (the `position.rs` hunk is covered by exact-value assertions).

## 8 Wording the SDD needs (SDD amendment is a separate task; no SDD edited)

- **A-R1 (error shapes):** add `SyntaxReason` and `TagReason` as the stable part of `PgnErrorKind::BadSyntax` and `BadTag`; add `IllegalReason::NotAPromotion` (a promotion piece on a move that does not promote); `FenError::BadField.detail` is display only.
- **A-R3 (version rule):** "`canonical_form` v1: for a mainline move that is illegal in the replay the legal prefix is followed by `{illegal <ply> <uci> ...}` (the 0-based index of the first illegal move and that move and all later mainline moves in UCI), then the result. A legal game's bytes are unchanged. A control character in a tag value is written as a space (ASM-RUL-004); the resulting collision of a tab and a space is accepted (Brian Prascher, 2026-11-09). The golden bytes of v1 are pinned for two standard-start games and one custom-start game." Also correct UT-RUL-031 (5) to the as-built source `crates/chess-rules/tests/data/fen_drift_guard.txt`.
- **A-R5 (lossy import):** replace "says nothing about it" with the warnings list: the five `ImportWarning` cases (4.5), the `-` / `?` exception for `TimeControl`, the between-games comment rule, and that Latin-1 is still per game pending Brian's decision on C-R5-F1.
- **A-R4 / 3.11.2:** `explain_illegal` returns `NotAPromotion`, not `CannotReach`, for a promotion piece on a non-promoting move.
- **3.11.1 / 3.11.8 (R8):** the `ParseMode::Pgn` list of accepted spellings (5); `e.p.` after a capture is accepted by the reader.

## 9 Not run / not verified

- Reviewer re-check of this change and the System Architect desk review of it: not done, not requested.
- Intel macOS and other platforms: not run.
- PGN Standard import-grammar conformance of the accepted spellings: `[UNV]`.
- The review's mutation of the deleted `fen.rs` arms was not repeated; the proptest sweep is evidence only.
- Whole-crate mutation run: not done (scoped run only).
- Whether `chess-domain` can be packaged with a test that reads a sibling crate's `tests/data`: not checked.
- `npm audit`: skipped (offline).

## 10 Files

- Source: `crates/chess-rules/src/{error,fen,lib,notation,position}.rs`, `src/pgn/{mod,reader}.rs`.
- Tests: `crates/chess-rules/tests/{canonical_v1,taxonomy,warnings,c_r6}.rs` (new); `tests/{pgn,rework,rules}.rs`, `tests/common/mod.rs` (edited); `tests/data/fen_drift_guard.txt` (moved in with `git mv`, staged).
- Shared: `crates/chess-domain/tests/drift_guard.rs` (edited).
- Docs: this file; a note appended to `docs/engineering/gates/GATE-102-chess-rules.md`.

## Recheck fixes (2026-11-09, task chess-k5l, Engineer stance, gate G1 for chess-rules, PENDING desk review)

Fixes for the conditions of `review-step1-recheck.md` (PASS WITH CONDITIONS). Nothing here is approved; no commit was made.

| ID | Change | Evidence |
|---|---|---|
| RC-01 | `Position::explain_illegal`: when `explain_castling` returns `Other` (all castling conditions hold) and a promotion letter was sent, the result is `NotAPromotion`. A castling failure with a letter keeps its castling reason (`CastlingRightsLost` etc.). | `tests/recheck.rs` `rc_01_*` `[RAN]`; before the fix the first and third failed (`e1g1q` gave `Other`; 12 `Other` in the first castling position) `[RAN]`. The sweep covers 10 castling-rich positions, both colours, 7 promotion options, 4096 square pairs each. |
| RC-03 | New cap `MAX_IMPORT_WARNINGS_PER_GAME = 256` per game. Later warnings are counted; the list ends with one `ImportWarning::WarningsTruncated { dropped }` (at most 257 entries). Between-game comments (`pending`) go through the same bounded log. Typed-tag warnings are now kept as at most one per tag name. | `rc_03_*` `[RAN]`: 200 000 `{a}` before a game gives 257 entries and `dropped = 199 744`; same for a tagged game; the cap is per game; exactly 256 gives no marker; 300 000 comments with no game give zero games; 5 000 repeated bad typed tags give 3 warnings. `canonical_form` and `write_pgn` equal those of the same game without the comments. |
| RC-08 | `explain_illegal` doc comment now says `NotAPromotion` (and covers castling). | doc only |
| RC-09 | `ImportWarning` doc: a between-game comment IS reported on the next game, and dropped if no game follows. `canonical_form` doc: the claim is limited to the movetext line; tag values may contain `{`. Test renamed `c_r3_rf_05_the_marker_cannot_occur_in_the_movetext_line_of_a_legal_game` and now adds a game whose tag values contain `{illegal 1 e2e4}` and `a { b`, asserting no `{` after the blank line (one movetext line) and that the tag value is present. | `canonical_v1` 8 passed `[RAN]` |
| RC-13 | Fixed (it was trivial): `RawTags` keeps the warning of the current value of each typed tag; a later `Date`, `TimeControl` or `Termination` tag replaces the earlier warning. | `rc_13_*` `[RAN]` |

Public API changes: `ImportWarning::WarningsTruncated { dropped: usize }` (new variant of a non-`#[non_exhaustive]` public enum, so an exhaustive `match` in a consumer would stop compiling; no consumer in the workspace matches on it, grep `[RAN]`); `pub const MAX_IMPORT_WARNINGS_PER_GAME: usize = 256` (also re-exported from the crate root). No other signature changed. `IllegalReason` is unchanged; only which variant is returned for a castling move with a letter changed (`Other` to `NotAPromotion`).

Behavior note: the warning order is now between-game comments, `Latin1Fallback`, tag warnings, movetext warnings (same as before); tag warnings are appended after the tag section instead of as each tag is read (only observable by order relative to nothing else, since no other warning is produced inside the tag section). A warning for a tag replaced later no longer appears (RC-13).

Unchanged bytes: `c_r3_legal_game_corpus_bytes_are_those_of_v1` passes with digest `0xb679052c13383ab7`; the custom-start golden and all other goldens pass `[RAN]`.

Gates `[RAN]`: `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` ended `All gates passed.` (fmt, build, clippy -D warnings, all workspace tests, crate edges, ECO checksums, cargo-deny). Deep perft `cargo test -p chess-rules --release -- --ignored`: `4 passed; 0 failed`. Coverage after `cargo llvm-cov clean --workspace`, `cargo llvm-cov -p chess-rules --summary-only`: lines 97.70 % (2132 lines, 49 missed), regions 96.78 %, functions 96.38 %. `Cargo.lock` unchanged `[OBS git status]`.

Not verified: that the SDD (docs/design, not touched here) states the cap and the new variant; the architect owns that edit (RC-14 and the SDD 3.12 / TBD-25 factor named in the re-check).

## Latin-1 per-payload (2026-11-09, task chess-n3h, Engineer stance, gate G1 for chess-rules, PENDING desk review)

Implements Brian Prascher's decision typed 2026-11-09 ("option B"): the PGN Latin-1 fallback is decided per text payload, not per game. This closes finding C-R5-F1 (4.6 above, which said "NOT done"). Nothing here is approved; no commit was made. Base: HEAD a166990, `crates/chess-rules` unmodified at the start `[OBS git status]`.

### Rule as implemented
- Every text payload the reader decodes (each tag value; each `{...}` comment and `;` comment in the movetext) is decoded on its own: valid UTF-8 is kept as it is; otherwise that payload, as a whole, is read byte for byte as Latin-1. No other payload of the game is affected. `[OBS reader.rs Decoder::decode]`
- The reader makes one pass per game (the second, Latin-1 pass and the `all_utf8` return value are gone). `[OBS reader.rs parse_game, Iterator::next]`
- Unchanged: BOM drop, `\r\n` and lone `\r` as one line break, tag-name and movetext bytes (never decoded as text), between-game comments (their text is not kept; only the non-empty test, still on `from_utf8_lossy`), `parse_line` (takes `&str`), the warning cap `MAX_IMPORT_WARNINGS_PER_GAME = 256`, `CANONICAL_FORM_VERSION = 1`.
- Warning shape: `ImportWarning::Latin1Fallback` stays a unit variant. It is now **once per payload that fell back** (was once per game), pushed when the payload is decoded, so in the order met: between-game comments first, then tag-section fallbacks, then typed-tag warnings (`DateIgnored`, ...; they are appended when the tag section ends, RC-13), then movetext warnings and fallbacks in reading order. It carries no position (adding fields would break an exhaustive match). A comment that is dropped (`CommentDropped`) was still decoded and still adds a `Latin1Fallback`. Each fallback counts against the cap like any warning. Display text changed to "a text in the game is not valid UTF-8, so that text was read as Latin-1".

### Public API changes
- None in signatures, types or constants `[RAN]` (the crate and all existing tests build; test edits are listed below). Behavioral and documented changes only: (1) `ImportWarning::Latin1Fallback` doc and `Display` text (per payload); (2) a game with k fallback payloads now has k `Latin1Fallback` entries (was 1); (3) mixed-encoding games read differently (below). A consumer that counts `Latin1Fallback` as "game was Latin-1" must be told (none in the workspace: grep for `Latin1Fallback` finds only `chess-rules` `[RAN]`).

### Tests (written first; 8 of 11 failed before the change `[RAN]`)
- New `crates/chess-rules/tests/latin1_per_payload.rs`, 11 tests: the 'Zoë' (`Zo\xc3\xab`) + Latin-1 (`Caf\xe9`) example in tag values, both orders; comments one by one (including a `;` comment holding a valid sequence and a stray byte, which is one Latin-1 payload); one warning per fallback payload in order; warning order with between-game comment and a typed-tag warning; the cap holds (306 fallback comments give 256 + `WarningsTruncated { dropped: 50 }`); per-game independence with BOM and the three line separators; `canonical_form` and `write_pgn` carry the per-payload text; robustness: 9 invalid sequences (lone `\xff`, truncated 2/3/4-byte, overlong, surrogate, above U+10FFFF, bare continuation, BOM inside a payload) at 18 positions each (tag name, whole/inside/after-backslash/unterminated tag value, comments whole/inside/unterminated, `;` comment mid and at EOF, movetext word, inside a variation, NAG, between games, after the game, truncated at EOF): no panic, terminates; and every one of 6 byte values substituted or inserted at every position of a small game with a variation, comment and NAG.
- Changed existing tests (their expectation was the old whole-game rule): `warnings.rs` `b6_whole_game_latin_1_fallback_is_reported_once` is now `b6_latin_1_fallback_is_reported_once_per_fallback_payload` (2 entries); the SUSPECT characterization `b6_characterization_latin_1_is_still_applied_to_the_whole_game_suspect` is replaced by `b6_latin_1_is_decided_per_payload_so_valid_utf_8_in_the_same_game_is_kept`; `pgn.rs` `ut_rul_020_if_2_reader_decodes_utf_8_and_latin_1_per_game_and_drops_a_bom` now expects the UTF-8 comment `é` kept next to the Latin-1 tag value. No test was removed, skipped or loosened.
- Unchanged and passing: `c_r3_legal_game_corpus_bytes_are_those_of_v1` (digest `0xb679052c13383ab7`), the custom-start golden, all other goldens `[RAN]`.

### Differential proof (old = `git archive HEAD crates/chess-rules third_party/eco` copy; new = working tree)
Harness (scratchpad, not in the repo): `/private/tmp/claude-501/-Users-brianprascher-Projects-chess/093edff2-fc6d-44b4-88b5-ba2bb1914e40/scratchpad/harness/src/main.rs`, depending on the old copy (renamed `chess-rules-old`) and the new crate by path; `cargo build --release --offline`, then `./target/release/harness 300000` (fixed seed, 68 s). Generator: random legal playouts with NAGs, `{}` and `;` comments, variations (depth 2), BOM, `\n`/`\r\n`/`\r`, 1 to 3 games per file, tag values (including `\"` and `\\` escapes) and comments, between-game comments, 5 % illegal-move games; payload classes ASCII, valid UTF-8 non-ASCII (U+0080 up to U+1F600), and Latin-1 bytes constrained to be invalid UTF-8; 299 596 of the 300 000 files contain non-ASCII bytes.
`[RAN]` output:
```
files=300000 games=600436 files_with_nonascii_bytes=299596
A (pure-class files, old(orig) == new(orig) on tags/moves/result/canonical_form/write_pgn/errors/non-Latin1 warnings): checked=200227 (all-UTF-8 files=55650, files with non-UTF-8 bytes=144577) FAIL=0
   among pure files: Latin1Fallback warning-count differs old vs new in 133651 files; ...
B (all files without injected errors, new(orig) == old(transcode every payload to UTF-8)): checked=270782 FAIL=0
   B files where only the non-Latin1 warning list differs (all fields above equal): 28
mixed-generator files=99773; of these old(orig) != new(orig) in 93869
```
- A: a pure file has each game wholly UTF-8 or wholly Latin-1 (every non-ASCII payload invalid UTF-8; these are exactly the games the old reader decoded as Latin-1 as a whole). Compared per item, old vs new: ok/err, tags, start, moves with comments/NAGs/variations, result, `canonical_form` bytes, `write_pgn` result, error kind/line/col/game_index, and all warnings except `Latin1Fallback`: **0 differences in 200 227 files** (so the all-UTF-8 and all-Latin-1 outputs are identical to before).
- B (mixed encodings, the only inputs allowed to change): new on the mixed bytes equals old on the same file with every payload transcoded to UTF-8 (that is, per-payload decoding): **0 differences in 270 782 files**; the generator is sensitive: old differs from new on 93 869 of 99 773 mixed files.
- Observation `[OBS]`, not a failure: in 28 B files only the warning list differs. Cause: a between-game comment made only of a lone Latin-1 byte 0x85 or 0xA0 is "non-empty" under `from_utf8_lossy` (U+FFFD) but empty once transcoded (NEL/NBSP are whitespace), so `CommentDropped` appears. This is existing behavior (the same in old and new, check A) and touches no game content; left alone.
- Known corner `[INF]`, not covered by the generator: a game whose `FEN` tag is non-UTF-8 fails in both versions with `BadFen`; the error is built from the lossy text in the old first pass and from the Latin-1 text now, so `FenError::TooLong { len }` could report a different `len` for a >128-byte non-UTF-8 FEN. Error value only, no stored output.
- STOP condition (any all-UTF-8 or all-Latin-1 output change) did not occur.

### Gates `[RAN]`
- `cargo fmt -p chess-rules -- --check`: clean. `cargo clippy -p chess-rules --all-targets --locked --offline -- -D warnings` (with `RUSTFLAGS=-D warnings`): clean.
- `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh`: exit 0, last line `All gates passed.` (fmt, build, clippy, all workspace tests, crate edges, ECO checksums, cargo-deny; npm audit skipped offline). It ran with another engineer's uncommitted chess-domain edits in the tree.
- `cargo test -p chess-rules --release -- --ignored`: `4 passed; 0 failed`.
- `cargo llvm-cov clean --workspace`, `cargo llvm-cov -p chess-rules --summary-only`: lines 97.68 % (2115 lines, 49 missed), regions 96.76 %, functions 96.38 %.
- `Cargo.lock` unchanged, no new dependency `[OBS git status]`.

### Wording the SDD needs (SDD-chess-rules 3.11.8 and the Latin-1 notes; the architect owns the edit)
1. IF-2 / 3.11.8: "Text payloads (tag values and comments) are decoded one by one. A payload that is valid UTF-8 is read as UTF-8; a payload that is not valid UTF-8 is read as Latin-1 as a whole, and only that payload. Other payloads of the same game are not affected. Tag names, move text and structure are ASCII and are not decoded as text."
2. Replace the whole-game sentence and the "mixed file reads as mojibake" caveat; remove "pending Brian's decision on C-R5-F1" (A-R5 note in section 6 above) and mark finding C-R5-F1 closed by decision 2026-11-09.
3. Warnings: "`ImportWarning::Latin1Fallback` is recorded once for each payload that fell back (it has no position), in the order met, and counts against `MAX_IMPORT_WARNINGS_PER_GAME`." Order: between-game comments, tag-section fallbacks, typed-tag warnings, movetext warnings and fallbacks (4.5 item 5 and its order line change accordingly).
4. Canonical form: no change to `v1`. Stored bytes change only for a game read from a file that mixes UTF-8 and non-UTF-8 payloads, when that file is imported again (a decision of Brian's, recorded 2026-11-09); all-UTF-8 and all-Latin-1 files give the same bytes and hashes as before (differential above).
5. Test references: `tests/latin1_per_payload.rs`, `b6_latin_1_is_decided_per_payload_so_valid_utf_8_in_the_same_game_is_kept`.

Not verified: that the SDD or architecture state this (not touched, per instruction); behavior on non-macOS targets; any consumer outside `crates/chess-rules` (none exists yet that reads `PgnGame::warnings`, grep `[RAN]`).
