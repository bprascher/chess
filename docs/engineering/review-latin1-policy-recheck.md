# Review: Latin-1 per-payload (chess-rules) and SelectClock / WhatIf policy (chess-domain), re-check

Task chess-0gb, baseline B4. Stance: Reviewer (adversarial, findings only, no fixes). Reviewer is a fresh invocation and did not author the code. Date 2026-11-09. Base: HEAD a166990 plus the uncommitted working tree. CERT applied by intent only; no standards conformance is claimed. This review approves nothing; the gates for both changes (G1 desk review, SDD amendments) remain with their owners.

## Verdict: PASS WITH CONDITIONS

No critical or major finding. Both changes behave as the decisions say, and nothing changed outside the allowed classes except one error-value difference (FIND-001). The conditions are dispositions only: FIND-001 (accept or document), FIND-002 and FIND-003 (stale text). None blocks the G1 desk review.

## Findings

| ID | Sev | Where | Finding |
|---|---|---|---|
| FIND-001 | minor | `crates/chess-rules/src/pgn/reader.rs:209-217` (`Decoder::decode`), `:778` (tag value), FEN parse at `:817-823` | An all-Latin-1 game whose `FEN` tag is longer than 128 characters now reports a different `BadFen(TooLong { len })` than HEAD. Old reader parsed once in UTF-8 mode with lossy decoding (each bad byte became U+FFFD, 3 bytes) and returned that Err without a Latin-1 re-pass; new reader decodes the payload as Latin-1 (2 bytes per high byte). This is an input class (all-Latin-1) that the decision said must be unchanged. Impact: error value only; no stored output, no `canonical_form`. |
| FIND-002 | minor | `crates/chess-domain/tests/data/policy_independent_table.txt:53` | Stale comment: "SelectClock in S" is listed among cases "already decided by an earlier FD6 or global row". FD6d-1 was removed; the case is now decided by A-SC1. The header (lines 1-8) also still says the table was not derived from the code but does not record that rows FD6d-1, A-WI3, A-SC2 were rewritten on 2026-11-09 after the code existed. |
| FIND-003 | minor | `crates/chess-rules/tests/pgn.rs` test name `ut_rul_020_if_2_reader_decodes_utf_8_and_latin_1_per_game_and_drops_a_bom` | The name still says "per_game"; the behaviour it asserts is now per payload. Renaming would change a test ID referenced by the SDD, so it is a wording follow-up for the owner, not a defect. |
| FIND-004 | info | `crates/chess-domain/src/policy.rs:116,138,160,330`; `error.rs:40,58,76` | `Action::SelectClock`, `ActionKind::SelectClock` and `DeniedReason::TrainingUntimed` are now permanently unreachable as an allowed or produced value from `Policy::check` (sweep: SelectClock ok in 0 of 108 000 rows; `TrainingUntimed` produced 0 times, was 1 296). The author left the variants in place on purpose (protocol enum). Retiring them is a G2/G3 decision for Brian; record it so it is not forgotten. |
| FIND-005 | info | `crates/chess-domain/tests/policy_independent.rs` `df_08...` | The `training_untimed` exemption in df_08 is justified (the reason is no longer produced; `check_setup` owns it via `SetupReason::TrainingUntimed`, unchanged). It hides nothing today: A-SC1 pins SelectClock, and any other action wrongly returning `training_untimed` would still fail the table compare unless that cell is `deny` or `?` (A-MM7 is `deny`). Residual: a future `training_untimed` produced only in an `A-MM7`-class cell would pass. Low. |
| FIND-006 | info | `crates/chess-domain/tests/policy.rs` (`oracle`) | The in-file oracle is a restatement of `Policy::check` by the same author, so it is not an independent check. The independent evidence is the table plus the sweep in this review. Whether the table rows were written before the code (author says tests first) is `[UNV]` for me; the row contents match the two decisions and my separately written spec (0 mismatches). |
| FIND-007 | info | `crates/chess-rules/src/pgn/reader.rs:322`, `:658` | Mutation run (below) left 2 survivors, both `delete field fatal from struct Fail` on the UnterminatedComment path. Not in the diff; pre-existing weakness: nothing observes `fatal` for an unterminated comment at EOF. |

No finding against the new behaviour of change A for mixed input, or against change B beyond the above.

## Change A: per-payload Latin-1 (evidence)

1. Differential (own harness, not the author's; scratchpad `diff/`): old = `git archive HEAD crates/chess-rules` renamed `chess-rules-old`; new = working tree; both built offline with the same `Cargo.lock`. Own xorshift generator with: BOM, `\n`/`\r\n`/lone `\r`, 1-3 games per file, 1-5 tag values from 11 tag names (incl. `Date`, `Result`, `TimeControl`, `Termination`), `{}` and `;` comments (also inside a variation), `$1` NAG, a move token with an invalid byte (`e\xe94`), between-game comments, a `SetUp`+`FEN` tag with an invalid byte, payload fragments of ASCII, valid UTF-8 (2, 3, 4 byte, U+0085, U+00A0, NUL, TAB) and invalid sequences (`\xe9 \xff \x80 \xc3`, truncated 3 and 4 byte, `\xc0\x80` overlong, `\xe0\x80\x80` overlong, `\xed\xa0\x80` surrogate, `\xf4\x90\x80\x80`, `\xc1\xbf`, `\xc3\x28`) so truncation lands at payload boundaries. Compared per game: Debug of game (tags, start, moves, comments, variations, result, warnings), `canonical_form` bytes, `write_pgn` result, Err values.
   - A (all-UTF-8 files): old == new including all warnings and positions. 
   - E (all-Latin-1 files, every non-ASCII payload invalid UTF-8): old == new exactly, ignoring only `Latin1Fallback` entries.
   - B (mixed files): new(raw) == old(file with every invalid payload transcoded to UTF-8), i.e. per-payload decoding; positions ignored because transcoding changes byte columns.
   - C/D (structured and byte-mutated fuzz): the set of games with at least one fallback is identical in old and new; every game without a fallback is identical; Err results identical.
   - `[RAN]` seeds 11-14, 100 000 iterations each (4 x 100 000 x 5 file classes): `fails=0` on all four runs; about 46 % of mixed files carried at least one fallback. Sensitivity check `[RAN]`: 5 215 of 20 000 mixed files differ between old and new on the raw bytes (the harness detects the change).
   - Only difference found anywhere: FIND-001 (targeted run over every position and insertion of `\xe9` in a FEN plus two over-long FENs: `diffs=2`, both `TooLong { len }`, e.g. 656 vs 456 and 203 vs 202).
2. Corpus and goldens `[RAN]`: `c_r3_legal_game_corpus_bytes_are_those_of_v1` passes (digest `0xb679052c13383ab7` pinned in test, unchanged in the diff); `cargo test --workspace --locked --offline`: no failure in any suite.
3. Robustness `[RAN]` (new crate, release): 3 x 2 MB random bytes; 1M `{\xe9}` comments (3 MB) gives 257 warnings = 256 + `WarningsTruncated` (cap honoured, 55 ms); 200 000 bad games (7.6 MB, 0.7 s); 10 MB tag value, 10 MB `;` comment, 5 MB unterminated tag and comment: no panic, no hang, all under 1 s. Existing `invalid_byte_sequences_everywhere...` and `every_single_byte_at_every_position...` tests pass.
4. Tests weakened? `[OBS git diff]`: `warnings.rs` (whole-game test now expects 2 fallback entries for a tag and a comment; the "valid UTF-8 is not a fallback" assertions kept; the SUSPECT characterization replaced by the per-payload test with stronger asserts), `pgn.rs` (one expectation changed from mojibake `\u{c3}\u{a9}` to `\u{e9}`, one assert added). Every edit is what the new rule requires. No test removed, skipped or loosened. The robustness tests in `latin1_per_payload.rs` assert only "no panic, terminates" (adequate for that purpose).
5. Input classes changed beyond the decision: only mixed-encoding games (allowed) and FIND-001 (error value). Tag names, move text, structure, BOM, line ends, between-game comment handling (`note_comment` still lossy, unchanged) and `parse_line` are unchanged. The number of `Latin1Fallback` entries per game changes from at most 1 to one per payload (documented, a decision consequence; no consumer in the workspace: `grep` finds only chess-rules).
6. Mutation `[RAN]` (cargo-mutants, `Decoder::decode|parse_game|WarnLog|next|note_comment` in reader.rs): 34 mutants, 22 caught, 10 unviable, 2 missed (FIND-007).
7. Coverage `[RAN]` `cargo llvm-cov -p chess-rules --summary-only`: lines 97.68 % (2115, 49 missed), regions 96.76 %, functions 96.38 %; reader.rs lines 96.86 % (region figure as printed), mod.rs 98.15 %. Matches the author's figures.

## Change B: SelectClock removed, Training WhatIf narrowed (evidence)

1. Brute-force sweep `[RAN]` (own program, scratchpad `pol/`, old `chess-domain` from `git archive HEAD` vs working tree, shared `chess-rules`): all 10 states x 2 modes x 3 matchups x 2 turns x 4 seat-kind pairs x 3 pending offers x 3 blocked values (including incoherent contexts) x all action instances = 108 000 (context, action) rows.
   - Internal consistency: `allowed_actions` equals the set of kinds with at least one ok instance, in declaration order with no duplicates, for every context, old and new: no inconsistency printed.
   - Old-vs-new differences are only `SelectClock` (1 296 `TrainingUntimed` -> `StateNotAllowed`, 1 296 ok -> `StateNotAllowed`) and `WhatIf` (432 ok -> `StateNotAllowed`). No other action changed in any context.
   - Spec written independently of `policy.rs` (SelectClock: `voided` / `game_over` on finished games, else `state_not_allowed`; WhatIf: finished ok, Strict `strict_mode`, Training only AwaitingMove with human to move, else `state_not_allowed`): 0 mismatches, including `Suspended`, `Paused`, `EngineError`, `DrawOffered`, `EngineThinking`.
   - `allowed_actions` changes in 1 728 contexts; no kind appears that was not allowed before.
   - `RespondDraw` unchanged (`state_not_allowed` for wrong side or actor), no new reason.
2. Game paths `[OBS grep]`: `game.rs` never uses `SelectClock` or `WhatIf`; it calls `Policy::check` for MakeMove, Takeback, OfferDraw, RespondDraw, RetryEngine, Pause, Step, PlayResume, Resign, Abort only (lines 879-1063) and `allowed_actions` for the view (line 437); `check_setup` at 314 and 760. `TrainingUntimed` outside `policy.rs`/`error.rs`: none in crates, ui, desktop. No code path relies on SelectClock or on `DeniedReason::TrainingUntimed`.
3. Independent table: the changed rows (FD6d-1 removed, A-WI3 `mode=T state=A tomove=H => ok`, A-SC2 -> `A-SC1 | SelectClock => state_not_allowed`) match the stated decisions and my separate spec. Row count 61 non-comment (HEAD 62) `[RAN grep]`. Whether the author wrote them before the code is `[UNV]` (FIND-006). df_08 exemption: see FIND-005.
4. Mutation `[RAN]` (cargo-mutants on `Policy::check|allowed_actions|offer_draw` in policy.rs): 86 mutants, 79 caught, 7 unviable, 0 missed, 0 timeout.
5. Coverage `[RAN]`: `cargo llvm-cov -p chess-domain --summary-only` lines 98.90 % (2543, 28 missed); policy.rs 273 lines, 0 missed.

## Gates re-run (C)

| Command | Result |
|---|---|
| `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` | exit 0, `All gates passed.` (npm audit skipped offline, as the script says) |
| `cargo test --workspace --locked --offline` | every `test result` line `ok`, no `FAILED` |
| `cargo test -p chess-rules --release --locked --offline -- --ignored` | 4 passed (perft start d5, d6 start and position 3, kiwipete d5, positions 4-6 d5); 0 failed |
| `cargo llvm-cov` for chess-rules and chess-domain | as above; both installed |
| `git status` after the review | tracked changes and untracked files are the same set as before, plus this record; no throwaway test in the tree. Another agent's edits under `docs/` and `.beads/` were present and not touched. |

## Not verified

- Whether the table and oracle rows were written before `policy.rs` was changed (author's claim, `[UNV]`).
- SDD and architecture text (edited concurrently by another agent): not read for correctness against the code; `docs/design/*.md` and `docs/architecture.md` are out of scope.
- Behaviour on non-macOS targets and Intel hardware; Windows/Linux line endings beyond the three separators tested.
- Whole-crate mutation testing (only the named functions were mutated); no fuzzing beyond the generators described, no `cargo fuzz`.
- Consumers outside the two crates (chess-app, server, CLI, UI do not exist yet): impact of `Latin1Fallback` count semantics and of the unreachable `SelectClock` / `TrainingUntimed` is unassessed.
- Standards: no MISRA/CERT conformance claim is made or checked.
- Scratch harnesses live under `/private/tmp/claude-501/-Users-brianprascher-Projects-chess/093edff2-fc6d-44b4-88b5-ba2bb1914e40/scratchpad/` (`diff/`, `pol/`, `mut1/`, `mut2/`); not part of the repo.

## Dispositions needed (owner, not this reviewer)

- FIND-001: ACCEPT (error value, documented) or FIX; owner of chess-rules / desk review.
- FIND-002, FIND-003: wording follow-ups, owner of the tests.
- FIND-004: decision on retiring the dead variants, Brian Prascher with the System Architect.
