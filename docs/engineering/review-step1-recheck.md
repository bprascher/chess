# REV-step1-recheck: Reviewer re-check of the rework and conditions changes in chess-rules and chess-domain

| Field | Value |
|---|---|
| Date | 2026-11-09 |
| Stance | Reviewer (software-engineer agent, adversarial). **Independent of the authoring invocations**: I did not write or edit any of this code. Same agent type, fresh invocation; I re-ran everything instead of reading the authors' transcripts |
| Scope | Working tree on branch `feat/step1-workspace-scaffolding`, base commit `51da006` plus uncommitted changes (`git status`: 23 modified/renamed files, 13 untracked new files in `crates/` and `docs/engineering/`). Baseline B4 |
| Intent sources | `review-chess-rules-code.md` (RF-01..RF-15), `review-chess-domain-code.md` (DF-01..DF-15), `desk-review-G1-step1.md` rulings, `step1-{rules,domain}-{rework,conditions}-evidence.md`, GATE-102 and GATE-103 notes, SDD-chess-rules 3.11.10 and SDD-chess-domain 3.11.1, 3.11.5 |
| Gate | This review approves nothing. GATE-102 and GATE-103 status is not changed by it. Per CR-15 this review is a precondition for baselining the rules SDD, not the baseline itself |
| Tags | `[OBS]` read in code, `[RAN]` executed here, `[INF]` inferred, `[UNV]` not verified |
| Throwaway code | All differential, fuzz and reproduction harnesses live in the session scratchpad (`.../scratchpad/h`, `.../scratchpad/d`), not in the repo. `git status` after the work shows no file created by me in the working tree other than this record (section 8) |

## 1 Verdict

**PASS WITH CONDITIONS.**

No Blocker and no Major. The two highest-risk claims held up under attack:

1. **The `canonical_form` marker changes no byte of any legal game.** Differential run of the committed pre-change code (`git archive HEAD`) against the working tree over 320 000 random legal games gave zero byte differences, and the author's corpus digest `0xb679052c13383ab7` is reproduced by the HEAD code (section 3.1).
2. **The reader and the tree limits do not panic, hang or overflow** under 400 000 fuzz inputs with overflow checks on, a 40 MB nesting bomb and the 256/64 boundaries (sections 3.3, 3.4).

Conditions (details in section 2 and section 6):

| # | Condition | Findings |
|---|---|---|
| CND-1 | RF-06 is **not closed**: a promotion letter on a legal castling move still reports `IllegalReason::Other`. Fix or get an SE/SA ruling that accepts it; add the castling case to the sweep test | RC-01 |
| CND-2 | Two wrong doc comments: `explain_illegal` (still says `CannotReach`) and `canonical_form` ("a legal game never contains `{` in its output", false for tag values) | RC-08, RC-09 |
| CND-3 | State in SDD 3.11.1 that the content of a journaled `Clock` fact is also not compared by `journal_eq` for a finished game, or narrow `journal_eq` further. The B5 ruling and the test names say more than the code does | RC-02 |
| CND-4 | The rules SDD must carry the as-built behavior before baselining: the illegal-tail marker, `NotAPromotion`, `SyntaxReason`/`TagReason`, `ImportWarning`/`PgnGame::warnings`, `e.p.`, the blank-line rule of RF-03 (authors' drafts A-R1, A-R3, A-R4, A-R5 cover most) | RC-14 |
| CND-5 | The SE confirmation of F-D6 (a) to (e) and PD-1 to PD-5 remains open; the "independent" policy table does not replace it | RC-05 |

Residual risks that are accepted by ruling or by the authors but are still true on this tree: DF-04 (arena growth), per-game Latin-1 decoding, unbounded warning list memory (RC-03).

## 2 Findings

Severity: Blocker, Major, Minor, Info. Confidence: CONFIRMED (reproduced here), LIKELY, SUSPECTED. IDs `RC-nn` are new in this record (re-check); earlier IDs are RF-/DF-.

### RC-01 [Minor][CONFIRMED] RF-06 is not closed: `Other` is still reachable from ordinary input (promotion letter on a castling move)
- Location: `crates/chess-rules/src/position.rs:354-356` (`explain_castling` returns `Some(IllegalReason::Other)` when "the castling conditions all hold"); it is called at `position.rs:~273` **before** the promotion check at `position.rs:286-293` that returns `NotAPromotion`.
- Reproduction `[RAN]` (scratch `h/src/bin/oth.rs`): position `r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1`, `parse_move("e1g1q", ParseMode::Typed)` returns `Err(NoMatch { reason: Other })`; `e1c1n` the same; `e1g1` is `Ok`. Sweep: 4 start positions x 80 random plies x 4096 square pairs x 7 promotion options (none, q, r, b, n, k, p), counting only moves for which `is_legal` is false: **9 168 104 probes, 132 `Other`, 0 `Other` for a legal move**. All 132 are a king moving two files from the e-file with a promotion letter (the sample lines all `e1c1<letter>`, and a filter for any `Other` that is not a king two-file move printed nothing).
- Impact: SDD 3.11.2 defines `Other` as "a defect indicator; logged by the caller, never shown as a rule". A client that sends UCI `e1g1q` is logged as a rules-library defect and the player sees "the move is not legal". This is the exact symptom RF-06 reported for `e2e4q`; the fix covers the pawn-move case only. The rework evidence sweep (`rf_06_*`, "64x64x3 sweep of the start position") never reaches a castling position, which is why it stayed green.
- Standards (intent): CERT ERR (a defect code must mean a defect).
- Action: in `explain_illegal`, check `promotion.is_some() && !promotes` before `explain_castling` (or make the castling arm return `NotAPromotion` for a letter), and extend the sweep to positions where castling is legal.
- Verify fix by: the two inputs above return `NotAPromotion`; the sweep above prints `Other 0`.

### RC-02 [Minor][CONFIRMED] `journal_eq` does not compare the content of a journaled `Clock` fact for a finished game; the "every other field is still compared" claim is too strong
- Location: `crates/chess-domain/src/game.rs:479-486` (`journal_eq`: a finished game compares only `clock.spec()`), `game.rs:636-646` (`Fact::Clock` restores the remaining times); doc comment `game.rs:462-477`; test `tests/replay.rs` `ut_dom_042_b5_journal_eq_still_compares_every_other_field_of_a_finished_game`.
- Observation: the B5 ruling and the doc say "No `Clock` fact precedes `Ended`", so the skipped remaining times are not journaled. But `Fact::Clock` is a journaled fact (periodic checkpoint, DF-05 names the app's 1 s checkpoint), and a checkpoint before `GameEnd` is legal. Its content is applied to the replayed clock and then ignored by `journal_eq`.
- Reproduction `[RAN]` (scratch `d/src/bin/jr.rs`): strict h-h 5+0, two moves, a `Fact::Clock{250000, 260000, running White}` inserted before the resign. Control: replay equals live, `journal_eq` true. Then (a) change the checkpoint to `{1, 2}`: replay succeeds with no error, replayed White remaining is `Some(1)`, `journal_eq(live)` **true**; (b) change `running` to Black: **true**; (c) drop the `Clock` fact: **true**. The same edits on a game that is still running are detected (the authors' test `..._while_the_game_is_running`).
- Impact: a mutated or lost journaled `Clock` fact slips through the live-versus-replay equality that SDD 3.11.1 calls "field by field". Consequence for a user is small (remaining time of an ended game; totals come from `GameEnd`), but this is the vacuous-property pattern DF-01 was filed for, now narrower and partly undisclosed.
- Action: SDD 3.11.1 wording (A-D1) must say that the remaining times of a finished game are not compared **whether or not a `Clock` checkpoint preceded the end**, and the test name and doc comment must not claim "every other field". Alternative that needs a ruling: compare the remaining times when a `Clock` fact was applied after the last `Move`.
- Verify fix by: wording corrected, or a test asserting the chosen behavior for case (a).

### RC-03 [Minor][CONFIRMED] The import warning list is unbounded: a 10 MB file costs 100 to 230 MB of warnings
- Location: `crates/chess-rules/src/pgn/reader.rs:586` (`pending: Vec<ImportWarning>`, between-game comments), `reader.rs:174-177` (`Decoder.warnings`), `reader.rs:851-870` (tag warnings hold the tag text).
- Reproduction `[RAN]` (scratch `h/src/bin/big.rs`, release, overflow checks on, `/usr/bin/time -l`):

| Input (10 MB) | Result | Max RSS |
|---|---|---|
| 3 333 333 times `{a}` before an untagged game | 1 game, 3 333 333 warnings, 104 ms | 228 MB |
| tag section, then 3 333 333 `{a}`, then a move | 1 game, 3 333 333 warnings, 103 ms | 131 MB |
| 3 333 333 `{a}` and nothing else | **0 games**, 0 warnings reported | 121 MB |

- Impact: the reader promised one game at a time (SDD 3.12, TBD-25 10 MB); RF-10 measured about 20x for a node-heavy game and this is a second, independent 10x to 20x path that needs no moves. The comment-only file shows the memory is held for the reader's lifetime and then dropped unreported. Time is linear (about 100 ms).
- Action: cap the list (for example 1 000 entries plus a counter variant `MoreWarnings { count }`) or document the factor next to RF-10 in SDD 3.12; the caller already owns the file size limit.
- Verify fix by: the first row above with a cap in place stays under about 60 MB.

### RC-04 [Minor][CONFIRMED] RF-07 is closed only for the PGN side; the "stable enum" still contains free text
- Location: `crates/chess-rules/src/error.rs:~129-136` (`FenError::BadField { detail: &'static str }`), `error.rs:~118` (`IllegalPosition::Other(String)`), `SyntaxReason::Internal`.
- Observation: `SyntaxReason`/`TagReason` are real variants and tested by variant (`tests/taxonomy.rs`, 5 tests; I read them). `BadField.detail` and `IllegalPosition::Other(String)` are still strings; a sweep of 3 000 000 structurally varied FENs `[RAN]` produced `Illegal(Other("the en passant square doe…"))` 14 924 times, so the free-text variant is reachable from ordinary input. The authors disclose `BadField.detail` (C-R5-F2) but not `Other(String)`.
- Dead-arm removal in `Fen::parse` is **safe**: the same 3 000 000 texts parsed by the HEAD code and the working tree gave **0 differences** and 0 uses of the "not accepted" fallback `[RAN]` (scratch `h/src/bin/fen.rs`). Only 5 115 of the texts were valid FENs, so this is evidence for the dead arms, not proof.
- Impact: D-51 will map errors to wire kinds; a front-end that branches on the en passant failure must compare an English string.
- Action: give the en passant illegal-position case its own `IllegalPosition` variant, or record that `Other(String)` is display-only; note both in the SDD error table.

### RC-05 [Minor][CONFIRMED] DF-08 is only partly closed: the new policy table is structurally independent but not an independent reading
- Location: `crates/chess-domain/tests/data/policy_independent_table.txt`, `tests/policy_independent.rs`.
- What I checked `[RAN]`:
  - Mutation: `cargo mutants -p chess-domain --re 'Policy::check( ->|$)|Policy::allowed_actions' -- --test policy_independent` (only the new test binary runs): **49 mutants, 46 caught, 0 missed, 3 unviable**. So the comparison is live and strong against changes of the code. This is better than the old oracle, which could only show agreement of two copies.
  - The interpreter (`policy_independent.rs:119-300`) has no policy knowledge; a row that decides no case fails a test; the `KNOWN` mechanism is unused (no `KNOWN` rows in the file `[OBS]`); the reason strings compared are the code's own `DeniedReason::as_str()` names `[OBS]`.
  - Spot-check of rows against SDD-chess-domain 3.11.5 and 3.11.4 text (Takeback, Rewind, Resign, Abort, CancelEngine, RetryEngine, the six draw-offer checks, the two global rows): consistent. The draw-offer rows restate the SDD's numbered order, so agreement there is by construction.
- Limits `[OBS]`: (1) the author is the same agent type that wrote `Policy`, and the file header discloses having read the F-D6 list and the first rows of the old oracle. The F-D6 rows (FD6a-1 to FD6e-1) **are the implementation's readings**, accepted by the desk review, not confirmed by the SE; they are the contested part and they agree with the code by design. (2) Verdict `deny` (row `A-MM7`) accepts any reason, and `?` (row `A-MM8`) excludes 252 cases (all `MakeMove{Engine}` with `by` not the side to move) from comparison. Both are disclosed, and the 252 are genuinely not decided by the SDD text. (3) The new file `docs/engineering/se-confirmation-policy-readings.md` (untracked, not mine) is the right vehicle; I did not review it.
- Action: SE confirmation of F-D6 (a) to (e) and PD-1 to PD-5; until then record DF-08 as "mechanically independent, semantically unconfirmed".

### RC-06 [Minor][CONFIRMED] DF-04 (arena growth) is still open; no risk ID found
- Location: `crates/chess-domain/src/variation.rs:49` (`nodes: Vec<Node>` never shrinks), `variation.rs:147` (`MAX_NODES` limits `live` only).
- Reproduction `[RAN]` (scratch `d/src/bin/arena.rs`, release): 2 000 000 `play`+`delete` cycles at the root: no error, live length 1, last `NodeId` 2 000 000, max RSS 355 MB.
- Status: desk review D8 accepts it for step 1 and asks for an RSK with an ID and a later G1 task; I found neither an RSK ID nor a follow-up task named in the evidence files I read `[UNV: beads not consulted by instruction]`. The rework evidence table does not mention DF-04 at all.
- Action: the PM records the RSK/task named in D8.

### RC-07 [Info][CONFIRMED] DF-06 tests use a home-made data format, not JSON
- `crates/chess-domain/tests/serde_shapes.rs` implements its own `Serializer`/`Deserializer` (`Val`), about 400 lines of test code, because `serde_json` is not admitted. It exercises serde-derive behavior (unknown field, missing key, integer id, bad enum) which is format-independent for these cases, so the value is real. Differences in numeric visitors (JSON gives `visit_u64` for positive integers, `Val` gives `visit_i64`) are not modelled. UUID leniency (uppercase, braced, urn) is accepted by ruling D9 and still not pinned by any test (rework evidence says so). The receive-side rule for missing `clock`/`start` keys is a condition on `chess-app` (D9), not on this crate.

### RC-08 [Minor][CONFIRMED] Stale doc comment on `explain_illegal`
- Location: `crates/chess-rules/src/position.rs:254-257`: "a promotion piece on a move that does not promote, or a promotion to a king or pawn, is `CannotReach` (RF-06)". The code returns `NotAPromotion` for a letter on a non-promoting move (`position.rs:286-288`); `CannotReach` only for a king/pawn chosen on a real promotion. The doc was not updated with the C-R5 change.
- Action: correct the comment (G0).

### RC-09 [Minor][CONFIRMED] `canonical_form` doc overstates the marker invariant; the test does not test what the doc says
- Location: `crates/chess-rules/src/pgn/mod.rs:590` ("A legal game never contains `{` in its output"), `crates/chess-rules/tests/canonical_v1.rs:235` (`c_r3_rf_05_the_marker_cannot_occur_in_a_legal_game`).
- Reproduction `[RAN]` (scratch `h/src/main.rs`): of 300 000 random legal games, **158 193 have a `{` somewhere in `canonical_form`** (a tag value containing `{`, which `tag_line` writes unchanged). In the **movetext line** (after the blank line) there were 0. The test uses a corpus whose tag values (`VALUES`) contain no `{`, so it cannot fail on the claim as worded.
- The actual invariant is sound and I attacked it (section 3.1): the movetext of a legal game contains only move numbers, SAN and the result, and tag values cannot contain a newline (control characters become spaces), so the marker cannot be forged from a tag. No collision found.
- Action: reword the doc to "the movetext line of a legal game never contains `{`", and add a tag value with `{illegal 1 e2e4}` to the corpus test.

### RC-10 [Info][CONFIRMED] RF-03 rule changes what the reader accepts
- `crates/chess-rules/src/pgn/reader.rs:~776-783`: a blank line between two tags of one game now fails that game with `BadSyntax{TagSectionWithoutMovetext}`. `[Event "A"]\n\n[White "x"]\n\n1. e4 *` is an error; before it was one game. Probe `[RAN]`: `[Event "A"]\n[White "w1"]\n\n[Event "B"]\n[White "w2"]\n\n1. e4 e5 *` gives `Err(g1 TagSectionWithoutMovetext)` then `Ok(g2 "B")`, so the original RF-03 silent loss is gone and the reader recovers on game 2. Whether the PGN Standard allows blank lines in the tag section is `[UNV]` (the author says so too). A tag-only game at end of input is still an `Ok` empty game (`[Event "A"]\n` gives 1 Ok), while a tag-only game followed by another section is an `Err`; this is inconsistent but disclosed (rework evidence, decision 2).

### RC-11 [Info][CONFIRMED] `e.p.` is accepted after any capture, not only after an en passant capture
- `reader.rs:396`. Probe `[RAN]`: `1. e4 d5 2. exd5 e.p. *` reads as 3 moves, no warning. This matches what the authors documented ("after a capture") and what the PGN world writes, and nothing illegal can enter (the move itself is matched against `legal_moves()`). Cases that stay errors, verified `[RAN]`: after a quiet move, twice, after `)` or a move number, glued onto a quiet move (`Nf3e.p.`).

### RC-12 [Info][CONFIRMED] A late `Clock` fact is ignored silently
- `game.rs:639-641`. Probe `[RAN]`: `Ended`, then five `Clock` facts: `error None`, `applied 10/10`, `warnings []`. Around a `Voided` fact: `error None`, state `Voided`, `warnings []`. A `Move` after a late `Clock` is still refused (`WrongState{Completed}`). The no-op cannot change any state, so it cannot hide a state divergence or make a corrupt journal accepted in a way that changes the game; it can hide a writer-order bug in the app (checkpoint timer not stopped at game end). A `ReplayOutcome.warnings` entry would make it visible.

### RC-13 [Info][CONFIRMED] A `DateIgnored` warning can describe a value that is no longer used
- Probe `[RAN]`: `[Date "2024.13.05"]` then `[Date "2024.01.05"]` gives `warnings [DateIgnored{"2024.13.05"}]` while the game's date is the valid second one (the later tag replaces the earlier, `RawTags::set`). Harmless noise; same for `TimeControl`.

### RC-14 [Minor, process][CONFIRMED] The rules SDD does not yet describe the as-built behavior
- `docs/design/SDD-chess-rules.md` 3.11.10 (read at 0.3.5 DRAFT, which another agent is editing now: `git status` shows it modified) still reads "No golden-bytes case exists in this design" and describes no marker, no `NotAPromotion`, no `SyntaxReason`/`TagReason`, no `ImportWarning`. The authors list the needed wording (conditions evidence section 8, A-R1, A-R3, A-R4, A-R5). Baselining the SDD without them would make the code and the SDD disagree on a stored-format input.

## 3 What I attacked and what held

### 3.1 The marker changes no legal-game byte (target 1) `[RAN]`
- Method: `git archive HEAD crates/chess-rules third_party` into the scratchpad, renamed `chess-rules-old`; the working tree copied as `chess-rules`; one harness crate depends on both and runs the same seeded generator (`include!` of one source file) against each. The generator makes random legal games: standard and custom starts (0 to 59 random plies deep), 0 to 129 mainline plies, NAGs, comments with `}`, tabs, newlines, NEL, `[`, quotes, backslashes, non-Latin text, nested variations (depth up to 3), all four results, random dates, `TimeControl` with and without increment, an extra tag.
- Result: **320 000 games** (20 000, then 300 000): `canonical_form` bytes identical for every game (so every SHA-256 is identical); `write_pgn` output identical; the written PGN re-parsed by the old and the new `PgnReader` and re-canonicalised gives identical bytes. `diffs 0` in both runs. I compared bytes, not hashes; equal bytes imply equal digests.
- Author corpus: I extracted the 400-game `corpus_digest` code from `tests/canonical_v1.rs` and ran it against both crates: old `0xb679052c13383ab7`, new `0xb679052c13383ab7`, 400 games each; the constant in the test is `0xb679_052c_1338_3ab7`. The digest is genuinely from the pre-change code.
- Extra starts: FENs with counters at `u32::MAX`, a 99-move clock, an en passant square, and a Chess960-style back rank with castling `-`: old and new bytes equal; no `{` in the movetext.
- Goldens: `git diff HEAD -- crates/chess-rules/tests/pgn.rs` has no hunk in `GOLDEN_V1` or the SetUp/FEN ordering test `[OBS]`. The new custom-start golden expected string was checked by hand against SDD 3.11.10 steps 1 to 5 (roster order, then `FEN`, `SetUp`, `TimeControl` in ASCII order, blank line, `7. e4 Kd7 0-1`, `Termination` excluded): correct.
- Trying to produce a marker from a legal game, or a collision: 60 000 games with random injected illegal tails (random from/to squares, promotion letters including king and pawn, tails with a legal-looking move after the illegal one) and a mix of legal games, hashed by bytes into a map keyed by (FEN, UCI list, result): **0 collisions**, the marker present in the movetext of 30 251 of them, absent from all legal ones. Reasoning `[INF]`: the legal prefix is SAN (unique per position), the tail is UCI (unique per move), the index is explicit, and tag values cannot span lines.
- Not changed, by design and pinned: control-character collapse. `char::is_control` covers C0, DEL and C1, so tab, LF, CR, BEL, NEL all become a space; the pin test passes; the code in `tag_line` is the same as the SDD 3.11.7 and ASM-RUL-004 text. Characters outside `is_control` (for example U+2028) are written raw `[OBS]`; not a collision.
- Residual: RC-09 (the doc sentence). The goldens are engineer-chosen (OQ-RUL-005 (b) says the SDD does not give them); the freeze is Brian's B1 decision as quoted in the task and the evidence, which I did not see as a signed record `[UNV]`.

### 3.2 Warnings never touch `canonical_form` or `write_pgn` (target 3) `[RAN]/[OBS]`
`grep warnings crates/chess-rules/src/pgn/mod.rs` finds only the field and its docs; neither function reads it. The 320 000-game differential includes `write_pgn` equality. Warning probes (all `[RAN]`): `2024.1.5` (unreadable width) warns, `2024.01.05`, `????.??.??`, `2024.??.??`, `-` and `?` time controls and `Termination "time forfeit"` (a written value, parsed case-insensitively) do not; a leading comment, a comment after `(`, a comment between games (attributed to the next game, known C-R5-F3), and the Latin-1 fallback (`[Latin1Fallback, CommentDropped]`, first in the list) behave as documented.

### 3.3 Reader robustness (target 3) `[RAN]`
- Fuzz: 400 000 inputs (token soup built from 50 PGN-relevant tokens including `e.p.` spellings, braces, NAGs, Latin-1 bytes; mutations of a valid two-game file; raw random bytes), each run through `PgnReader` with the default limits, then `canonical_form` and `write_pgn` on every `Ok` game, under `catch_unwind`, release build with `overflow-checks` and `debug-assertions` on: **0 panics, max 0 ms per input, 82 905 games and 1 495 warnings produced, 0.9 s total**.
- Large inputs (release): 40 MB of `(e4 ` with default limits: `VariationTooDeep` in 243 ms, 82 MB RSS. 2 000 000 `e.p.` words: error in 46 ms. 1 000 000 tag sections: 999 999 errors then one game, 171 ms, 15 MB. A 12 MB file with a non-UTF-8 byte after 3 000 000 comments (ends as one game error): 99 ms, 112 MB. The progress guard and the new `read_line` path in `skip_between_games` terminate on every input tried. Memory finding: RC-03.
- RF-01: depth 300 with `max_variation_depth` of `usize::MAX`, 300, 256 gives `VariationTooDeep{limit: 256}`; with 255 the error reports 255 and a depth-255 file parses with the limit at `usize::MAX`. The 256 KiB-thread drop test of the authors is in the passing suite.
- RF-02/RF-03 probes: see closure table.

### 3.4 Depth boundaries and the what-if tree (target 4) `[RAN]`
- `VariationTree`: a chain where every step adds a main move and then continues from an alternative, built only through `play`: variations 1 to 64 accepted, **the 65th returns `TreeError::TooDeep`**, length 130, `to_pgn_text` succeeds, `from_pgn_text(text)` rebuilds a tree whose text is identical, `write_pgn` of the nodes succeeds, the `PgnReader` reads the written game back. 300 random build/delete rounds (120 000 operations) never produced `BadText` from `to_pgn_text`, never a changed length after a refusal, and every text round-tripped (they did not reach depth 64; the chain test is the boundary evidence). `[INF]` `delete` only lowers depth (a promoted sibling becomes the main line), as the authors state.
- Writer limit (64) and reader limit (clamped 256): different on purpose; `from_pgn_text` uses the default 64.

### 3.5 `journal_eq`, late `Clock`, `Ended` (target 5)
Every `Game` field is compared except `start_position`, `positions` (derived from setup and plies) and the documented runtime state `[OBS game.rs:479-496]`. Per-fact mutation on finished games `[RAN]`: `think_ms`, outcome, ply count are detected (authors' tests, plus the mutation run below); `Move.clocks_after` and `played_at` are inside `PlyRecord` and compared. The slip is the `Clock` fact content (RC-02). The late-`Clock` no-op is unreachable for state change (RC-12). `Ended` is trusted as recorded (ruling D2(i)); `Ended` on the start position replays as `Completed` with a checkmate outcome and a non-mate position (authors' test pins it).
- C-D3 as built: `check_setup` refuses the same `PlayerId` on both seats **only when the kinds differ** (`policy.rs:413-417`); self-play with one kind stays allowed (`is_self_play`). That matches DF-07 and decision D10; a caller reading "same PlayerId both seats refused" literally would be wrong.

### 3.6 Policy table (target 6)
See RC-05. Independence of code: no shared source with `Policy` or with `tests/policy.rs`; shares the `Action`/`GameContext` types and the `DeniedReason::as_str` strings.

### 3.7 Quality gates, coverage, mutation (target 7) `[RAN]`
| Check | Command | Result |
|---|---|---|
| CI gates | `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` | exit 0, "All gates passed." fmt, build `--locked`, clippy `-D warnings`, tests, crate-edge check, ECO checksums, `cargo deny` all passed; `npm audit` skipped (offline, by design). 448 tests passed, 0 failed, 4 ignored (sum of the `test result` lines) |
| Tests, separate target dir | `cargo test --workspace --locked --offline` (`RUSTFLAGS=-D warnings`) | all `ok`, no failure |
| Deep perft | `cargo test -p chess-rules --release --offline -- --ignored` | `4 passed; 0 failed; 8 filtered out; finished in 3.34s` |
| Coverage | `cargo llvm-cov --offline -p chess-rules -p chess-domain --summary-only` | TOTAL lines 4644, missed 77, **98.34 %**; functions 97.57 %; regions 97.60 %. `pgn/reader.rs` 97.62 %, `pgn/mod.rs` 99.20 %, `position.rs` 99.10 %, `fen.rs` 98.00 %, `game.rs` 99.57 %, `variation.rs` 97.37 %, `policy.rs` 100 %. `testing.rs` 77.46 % is the test-support helper. Branch coverage is not reported by this tool (0 branches) |
| Mutation, domain | `cargo mutants -p chess-domain --re 'journal_eq\|variation_depth\|VariationTree::play\|apply_fact' -j3` | 58 mutants: **52 caught, 0 missed, 0 timeout, 6 unviable** |
| Mutation, domain, policy only against the new table | `... --re 'Policy::check( ->\|$)\|Policy::allowed_actions' -- --test policy_independent` | 49 mutants: **46 caught, 0 missed, 3 unviable** |
| Mutation, rules | `cargo mutants -p chess-rules --re 'canonical_form\|illegal_tail_marker\|strip_glued\|is_en_passant\|attach_comment\|note_comment\|explain_illegal\|at_stray_result\|at_eof_mark\|RawTags' -j3 -- --tests` | 77 mutants: 41 caught, **3 missed**, 5 timeouts, 28 unviable. Missed: `reader.rs:296` and `reader.rs:634` (delete field `fatal`), `reader.rs:463` (`>` to `>=` in `strip_glued_en_passant`). They are the same three the authors reported and their equivalence argument holds on reading `[INF]`. The 5 timeouts are all in `at_stray_result` (a mutant that loops; a hang is detection, not survival) |
| Differential | scratch harness, 320 000 games | 0 differences (3.1) |
| Fuzz | scratch harness | 0 panics in 400 000 inputs (3.3) |

Mutants were run in cargo-mutants' temporary copy with `--output` in the scratchpad; the pre-existing `mutants.out` and `mutants.out.old` in the repo root are from 2026-11-06 and untouched.

## 4 Closure table: earlier findings

"Closed" means the finding's own "verify fix by" holds on this tree, as re-run by me.

### 4.1 chess-rules (RF)

| ID | Sev | Status | Basis |
|---|---|---|---|
| RF-01 depth unbounded | Minor | **Closed** | Clamp 256 (`pgn/mod.rs`, `MAX_VARIATION_DEPTH_CEILING`); limit `usize::MAX` on 300 levels gives `VariationTooDeep{256}`; depth 255 parses; 40 MB bomb handled; the stack test is in the green suite |
| RF-02 phantom game | Minor | **Closed** | Trailing `*`, lone `*`, double terminator, trailing Ctrl-Z give 1, 0, 1, 1 items, no spurious `Err` |
| RF-03 merged tag sections | Minor | **Closed**, with the behavior change in RC-10 | First game now an `Err`, second game read and not merged |
| RF-04 silent information loss | Minor | **Closed as to signalling; per-payload Latin-1 decoding deliberately open** | `PgnGame::warnings` with five cases, probes in 3.2. Latin-1 still whole-game (`ZoÃ«` reproduced: `[Latin1Fallback, CommentDropped]`, white `"ZoÃ«"`); owner decision C-R5-F1. New cost: RC-03 |
| RF-05 canonical form | Minor | **Closed** for (a) marker, version constant and custom-start golden; (b) control collapse **accepted and pinned**; doc defect RC-09; SDD wording RC-14 | Section 3.1 |
| RF-06 `Other` reachable | Minor | **NOT closed** | RC-01 (castling + letter); stale doc RC-08 |
| RF-07 taxonomy | Minor | **Partly closed** | PGN enums closed; `BadField.detail` and `IllegalPosition::Other(String)` still free text (RC-04); dead arms safely removed (3M-text differential, 0 differences) |
| RF-08 `ParseMode::Pgn` doc | Minor | **Closed** | Doc rewritten; I checked about 30 spellings against it `[RAN]` (accepts `E4`, `e2e4`, `Ng1f3`, `Nf3+++`, `Nf3!?`, `0-0`, `a8Q`, `a8q`; rejects `g1f3`, `nf3`, `Nxf3`, `Nf3x`, `o-o`, `a8=K`). PGN Standard conformance still `[UNV]` |
| RF-09 960 FEN | Minor | **Closed as accepted** | Code unchanged; the "record the answer" part is SDD text (not checked) `[UNV]` |
| RF-10 memory 20x | Minor | **Open, accepted by desk review R9; worsened by RC-03** | SDD 3.12 / TBD-25 text not verified |
| RF-11 ECO integrity CI-only | Minor | **Closed as accepted** | CI ECO checksum step passed |
| RF-12 gate footprint | Minor, process | **Closed** | Desk review R11 ruled G1; GATE-102 note present `[OBS]` |
| RF-13 `Debug` of upstream type | Info | No action | |
| RF-14 `Bb4` fallback | Info | No action; SE confirmation pending (desk review R8) | |
| RF-15 long comment lines | Info | No action (conforms to SDD 3.11.7 step 5) | |

### 4.2 chess-domain (DF)

| ID | Sev | Status | Basis |
|---|---|---|---|
| DF-01 final clock not journaled | Major | **Closed per ruling B5, with residual RC-02** | `journal_eq` narrowed to spec compare; two reproductions pinned as documented differences; mutation 0 missed; but a journaled `Clock` fact still slips |
| DF-02 `Tally::games` overflow | Minor | **Closed** | `saturating_add` at `stats.rs:54-58` `[OBS]`; test at `u32::MAX` in the green suite |
| DF-03 deep tree text fails | Minor | **Closed** | `TooDeep` at 65, 64 accepted, export and round trip OK, doc corrected (3.4) |
| DF-04 arena growth | Minor | **Open, accepted (desk review D8)** | RC-06; no RSK ID seen |
| DF-05 replay trusts facts, rejects late `Clock` | Minor | **Closed per ruling D2** | (a) `Ended` trusted and pinned; (b) late `Clock` ignored; silent (RC-12) |
| DF-06 UUID leniency, serde tests | Minor | **Closed as accepted (D9) with RC-07** | Tests exist, in a stand-in format; leniency not pinned; `chess-app` condition carried by D9 |
| DF-07 one Player, two kinds | Minor | **Closed** | `policy.rs:413-417`; refuses only different kinds (3.5) |
| DF-08 oracle is a transcription | Minor | **Partly closed** | RC-05 |
| DF-09 ECO move-order wording | Minor | **Open (SDD wording A-D4), no code change** | Not examined here |
| DF-10 class bounds and `UCI_Elo` | Minor | **Open (D11, waits for the engine range)** | Not examined here |
| DF-11 to DF-15 | Info | No action | |

## 5 Test adequacy

- New tests are meaningful and mutation-checked: the 58-mutant domain run and the 77-mutant rules run leave only the three equivalent survivors. The marker tests pin text, not only inequality (`ends_with("1. e4 e5 {illegal 2 e1e5 a1a2} *\n")`).
- Gaps: no test sweeps `explain_illegal` on a castling position (RC-01); `c_r3_rf_05_the_marker_cannot_occur_in_a_legal_game` cannot fail on the doc claim (RC-09); `journal_eq` has no test for a mutated `Clock` fact (RC-02); no test bounds the warning list (RC-03); the tree test for `TooDeep` at the boundary is present and I confirmed it independently.
- The shared drift-guard table works as designed: both crates read one file; the rules side checks only the first two columns, and keeps/drops by comparing with `START_FEN`, so a wrong third column is caught by the domain test only (as the file header says).

## 6 Actions, in order

1. Fix RC-01 and add the castling sweep (rules, G1). Fix RC-08 and RC-09 (G0).
2. Decide RC-02: SDD wording (A-D1) or a narrower `journal_eq`; correct the test name.
3. Rules SDD: carry A-R1, A-R3, A-R4, A-R5 and the RF-03 blank-line rule before baselining (RC-14, RC-10).
4. Cap or document the warning list (RC-03); add the factor to SDD 3.12 / TBD-25 next to RF-10.
5. SE confirmation of F-D6 (a) to (e), PD-1 to PD-5 (RC-05). Record the DF-04 RSK (RC-06). Optional: en passant `IllegalPosition` variant (RC-04), replay warning for a late `Clock` (RC-12).

## 7 Not reviewed, not run, assumptions

- Not run: Intel macOS, other platforms; `npm audit` (offline); whole-crate mutation runs (scoped to changed functions only, as listed); `cargo public-api`; miri; sanitizers; a run of the repository's own proptests with a larger case count; fuzzing with a coverage-guided fuzzer (my fuzz is a random/structured generator, not libFuzzer).
- Not verified: the PGN Standard grammar for each accepted spelling and for blank lines in the tag section `[UNV]`; SDD-chess-rules text outside 3.11.10 and 3.11.7 and the amendment drafts A-R1 to A-R5; the content of `se-confirmation-policy-readings.md`; beads (not touched, by instruction); the B1/B5/B6 decisions are taken from the task text and the authors' records, not from a signed record I saw.
- Conformance: none claimed. SEI CERT is applied by intent only (project rule); no analyzer output beyond clippy `-D warnings` with the workspace lint set (`unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, casts denied) was produced.
- Independence: I am the same agent type as the authors; I did not read their transcripts, wrote my own harnesses, and re-ran their numbers. This is a desk review by an agent. It satisfies no G2 or G3 requirement and approves nothing.

## 8 Working-tree check

`git status --short` before and after my work: the same modified and untracked set, plus this file. The untracked `docs/engineering/se-confirmation-policy-readings.md` and the modified `docs/design/SDD-chess-rules.md` and `.beads/issues.jsonl` appeared from other agents. All builds, coverage and mutation outputs went to scratchpad target directories (`tgt`, `tgt2`, `tgt3`, `tgt4`) and `--output` directories. Mermaid: this document has no diagram.
