# Evidence: chess-rules rework of the adversarial review (task chess-a0r.2.11, baseline B4)

| Field | Value |
|---|---|
| Date | 2026-11-06 |
| Stance | Engineer (software-engineer agent). No approval is claimed or recorded here |
| Input | `docs/engineering/review-chess-rules-code.md` (PASS WITH CONDITIONS) |
| Gate | GATE-102, G1, OPEN, not approved (rework note appended there) |
| Scope | `crates/chess-rules/{src,tests}`, `docs/engineering/gates/GATE-102-chess-rules.md`, this file. `crates/chess-domain`, `ci/`, workspace files, `Cargo.lock`, beads: untouched by me |
| In scope | RF-01, RF-02, RF-03, RF-06, RF-05 (version constant only) |
| Out of scope | RF-04, 07, 08, 09 (decisions), RF-10..15 (RF-12 only as the GATE note) |

Tags: `[RAN]` executed, `[OBS]` read, `[INF]` inferred, `[UNV]` not verified.

## 1 Method

Tests first in new `tests/rework.rs` (14 tests, later 15). Run before the code change: the failing set is in 2.1. Then the fix, then the full suite. All commands from `/Users/brianprascher/Projects/chess`.

## 2 Results

### 2.1 Before the fix [RAN]
`cargo test -p chess-rules --test rework` (after only `CANONICAL_FORM_VERSION` was added so the file compiles):
- Passing before: `rf_05_canonical_form_version_constant_is_1`, `rf_02_a_tagged_game_with_only_a_terminator_is_still_a_game`, `rf_03_tags_without_a_blank_line_between_them_stay_one_section`, `rf_02_ctrl_z_inside_the_text_is_still_an_error` (guards that must keep passing).
- FAILED before: `rf_06_a_promotion_letter_on_a_non_promoting_move_is_not_other`, `rf_06_promotion_to_king_or_pawn_is_not_other`, `rf_06_no_ordinary_malformed_move_text_reports_other`, `rf_02_a_lone_terminator_yields_no_game`, `rf_02_a_stray_terminator_after_a_game_is_not_a_game`, `rf_02_trailing_ctrl_z_is_end_of_input`, `rf_03_consecutive_tag_sections_do_not_merge`, `rf_03_three_tag_only_sections_give_two_errors_and_the_game`.
- RF-01: `thread has overflowed its stack`, `fatal runtime error: stack overflow, aborting`, process SIGABRT (`signal: 6`), on the 256 KiB test thread with 200 000 nested variations and a raised limit. The abort ends the test binary, so `rf_01_*` have no individual FAILED line.

### 2.2 After the fix [RAN]
- `cargo test -p chess-rules`: every suite ok. Counts: unit 8 passed / 4 ignored; eco 6; eco_build 4; fen 15; isolated_data_dir 4; notation 29; pgn 63; properties 3; **rework 15**; rules 37; types 5; doc 0. No failures.
- `cargo clippy -p chess-rules --all-targets -- -D warnings`: finished, no warnings. `cargo fmt -p chess-rules` applied before the runs.
- `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh`: ends `All gates passed.` (fmt, build, clippy, tests, crate-edge check "8 crates, 11 workspace edges", ECO checksums "6 files match", `cargo deny` "advisories ok, bans ok, licenses ok, sources ok" with the same two `license-not-encountered` warnings as in the review; `npm audit` skipped offline). I piped the output through `tail`, so the shell exit status printed (`EXIT=0`) is `tail`'s; the script's own verdict is the final line.
- `cargo test -p chess-rules --release -- --ignored`: unit suite "4 passed" (deep perft: start depth 5 and 6, Kiwipete, positions 3 to 6) in 3.17 s.
- Line coverage, `cargo llvm-cov -p chess-rules --summary-only` (separate target dir): eco 100, error 100, fen 91.82, key_derive 100, notation 97.48, pgn/mod 99.14, pgn/reader 97.35, position 99.10, testing 77.46, types 100; **TOTAL lines 97.08 %** (functions 96.24 %, regions 96.12 %); the branch columns are 0 (not reported by this toolchain). Requirement of at least 90 % met. `fen.rs` shows 91.82 % lines in the "Lines" group (the 94.12 % shown first is regions).

## 3 Per-item record

| ID | Fix | Where | Tests |
|---|---|---|---|
| RF-01 | `max_variation_depth` is clamped to a crate ceiling `MAX_VARIATION_DEPTH_CEILING = 256` (`pub(crate)`, documented on `PgnLimits`). `VariationTooDeep.limit` reports the effective limit. Chosen over an iterative `Drop` because `Clone`, `Debug` and `PartialEq` also recurse and cannot be made iterative without a signature change | `src/pgn/mod.rs`, `src/pgn/reader.rs` (the `(` arm) | `rf_01_huge_depth_limit_is_clamped_...` (200 000 nested, limits 200 001 and `usize::MAX`, 256 KiB thread), `rf_01_the_ceiling_is_256_...` (256 ok, 257 error with limit 256, limit 4 honoured) |
| RF-02 | (a) A result word where a game would start, with no tag section before it, is consumed and is not a game (`at_stray_result`). (b) A Ctrl-Z byte (0x1A) followed only by whitespace and further 0x1A to the end of input is end of input, in the between-games skip and in the movetext loop (`Lexer::at_eof_mark`). A Ctrl-Z anywhere else is still the old error | `src/pgn/reader.rs` | four `rf_02_*` tests: stray `*`, `1-0`, `0-1`, `1/2-1/2` after a game; lone terminators give no item; a tagged game with only `*` is still a game; trailing Ctrl-Z variants give 1 Ok; an inner Ctrl-Z is still an Err |
| RF-03 | In the tag loop, a blank line (line number advances by 2 or more) between two tag sections, with the first section having at least one tag, fails the first game with `BadSyntax { detail: "a tag section needs movetext before the next tag section" }` positioned at the second section's `[`. The reader stays at that `[`, so the second section starts the next game. `in_movetext: true` makes the skip stop there | `src/pgn/reader.rs` (`parse_game`) | `rf_03_consecutive_tag_sections_do_not_merge` (2 items: Err game 1 at line 4, Ok game 2 with tags B/w2 and 2 moves), `rf_03_three_tag_only_sections_...`, `rf_03_tags_without_a_blank_line_...` (guard) |
| RF-06 | `explain_illegal` returns `CannotReach { piece }` (the moving piece, or the chosen promotion piece for a king or pawn promotion) instead of `Other`. No new variant, so no public enum change. `Other` remains for "the move is in fact legal" and the castling fallthrough | `src/position.rs` | `rf_06_*` (4 tests incl. a 64x64x3 sweep of the start position: no `Other` for an illegal probe); `tests/rules.rs` UT-RUL-015 assertions updated (they pinned the defect) |
| RF-05 part | `pub const CANONICAL_FORM_VERSION: u32 = 1;` in `src/pgn/mod.rs`, re-exported in `src/lib.rs`. Golden bytes untouched: `ut_rul_026_d_42_canonical_form_golden_bytes_of_v1` passes unchanged | `src/pgn/mod.rs`, `src/lib.rs` | `rf_05_canonical_form_version_constant_is_1` |

## 4 Choices to confirm (SDD owner / SA)

1. **Ceiling 256 [INF].** Chosen as a value well above any real annotated game; not derived from a requirement. The SDD says "bounded nesting" (3.12) with an ESTIMATE of 64 for the default; the ceiling is not in the SDD. The 256 KiB-stack test shows 256 is safe for Drop; it does not prove the ceiling for `Debug` or `Clone` on small stacks [UNV].
2. **RF-02 scope.** A game with no tags and no moves is not a game. A tag-only game followed by EOF with no terminator is still an Ok empty game (unchanged; the review did not list it). A tagged game with a terminator and no moves is an Ok game.
3. **RF-03 rule.** The signal is "blank line, then `[`". A hand-prepared file that puts a blank line between two tags of the same game now fails that game with `BadSyntax` at the second tag. `[UNV]` whether the PGN Standard's import format allows blank lines inside the tag section. Tags with no blank line between them never trigger it.
4. **RF-06 reason.** `CannotReach { piece: Pawn }` for `e2e4q` renders as "no pawn can reach that square", which is not an exact description (the pawn can reach e4 without promoting). A dedicated variant would read better but is a public enum change, which I did not make. The SDD owner may want one.
5. **Ctrl-Z in the middle of an error recovery** (`skip_to_terminator`) is not special-cased; a trailing mark after a bad game is skipped as an ordinary word and the next `next()` sees the end.

## 5 Findings recorded, not fixed

- **FIND-RW-001 (RF-05 (a), (b), stored-format door) [Minor, OPEN, owner Brian Prascher].** `canonical_form` is not injective for (a) an illegal mainline tail (`break` at `src/pgn/mod.rs` canonical loop) and (b) control characters in tag values (`tag_line` turns any control char into a space). Fixing either changes or restricts `v1` bytes under future stored hashes, so I changed nothing. Note: for (a) a fix that does not alter bytes of legal games (a validated constructor or a `Result`) is a public signature change, also not done. Re-confirmed [INF] from the code; not re-run, the review's reproduction is the evidence.
- **FIND-RW-002 [Info].** `ut_rul_015` and the notation test `e2e4q` assertion accepted `Other`; a test that pins a defect indicator is a weak oracle. Updated (see 3).
- **FIND-RW-003 [Info].** The SDD (3.11.2 for `Other`, 3.11.8 for the reader, 3.11.10 for the version constant, 3.12 for the depth ceiling) needs text for the four behaviours above and for OQ-RUL-005 (a) now answered in code. SDD owner's task; not edited (outside my scope).

## 6 Hard constraints check [INF from git status and diff]
- No public function or type signature changed. Additive: `CANONICAL_FORM_VERSION`. Behavioural changes listed above.
- No new dependency in chess-rules. `Cargo.lock` and `crates/chess-domain/Cargo.toml` show as modified in `git status` at the end of my run [OBS]; the `Cargo.lock` diff adds `proptest`, `serde`, `thiserror`, `uuid`, `sha1_smol` under `chess-domain` [OBS `git diff Cargo.lock`], so it is the concurrent chess-domain task's change, not mine [INF]. I did not edit either file.
- Nothing committed, pushed, or sent to beads.

## 7 Not run / not verified
- Mutation testing of the new code (`cargo mutants`): not run. The new branches are covered by the tests above, but mutation strength is unmeasured.
- Coverage-guided fuzzing: not available; the existing proptest suites (FZ-RUL-003) pass with the changes.
- The 2 M `explain_illegal` playout sweep of the review: not repeated; replaced by the start-position sweep in `rf_06_sweep_...`.
- Reviewer re-check of this rework: not done (needs an independent invocation). SA desk review and GATE-102 approval: not done.
- Intel Mac, `npm audit`: not run (offline / no hardware).
