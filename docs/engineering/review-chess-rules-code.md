# REV-chess-rules-code: adversarial code review of `crates/chess-rules` (parts A and B)

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-rules-code.md |
| Task | chess-a0r.2.5 (baseline B4) |
| Date | 2026-11-06 |
| Stance | **Reviewer** (software-engineer agent, fresh invocation, not the author of parts A or B). Independence: independent of the author invocation; same agent type. This is a desk review. It is **not** the System Architect desk review and **not** an approval of GATE-102 |
| Scope | `crates/chess-rules/{src,build.rs,build,tests}`, `third_party/eco/`, `ci/check-eco.mjs`, the `ci/check.sh` hunk. Working tree: committed part A (0087962) plus uncommitted part B and the ECO pass |
| Intent sources | docs/design/SDD-chess-rules.md 0.3.5 DRAFT (not baselined), docs/engineering/readiness-chess-rules.md, SWE-001, test-strategy.md, requirements IF-2 / RULE-1 / RULE-7 / NFR-3, architecture 3.2 and 5.7 |
| Standard | SEI CERT applied **by intent only**. This review makes **no conformance claim** (SWE-001; test-strategy "Quality gates") |
| Verdict | **PASS WITH CONDITIONS** (section 6). No Blocker, no Major. Twelve Minor (RF-01 to RF-12), three Info (RF-13 to RF-15) |

Evidence tags: `[OBS file:line]` read, `[RAN]` executed here, `[INF]` inferred, `[UNV]` not verified. Finding confidence: CONFIRMED (reproduced), LIKELY, SUSPECTED. Finding prefix `RF-` (review finding) to avoid clashing with `FIND-RUL-nnn`. Severity scale: Blocker, Major, Minor, Info.

The author's evidence (step1-rules-A-evidence.md, step1-rules-B-evidence.md, GATE-102) was read only after I had formed a view from code, tests and my own runs.

## 1 Baseline and tool runs [RAN]

| Run | Result |
|---|---|
| `CHESS_CI_OFFLINE=1 ci/check.sh` | exit 0, "All gates passed". fmt, build, clippy `-D warnings`, tests, crate-edge check (8 crates, 11 edges), ECO checksums (6 files), `cargo deny` (advisories, bans, licenses, sources ok; two `license-not-encountered` warnings for BSD-3-Clause and Unlicense in `deny.toml`) |
| `cargo test` for chess-rules | unit 8 passed / 4 ignored; eco 6; eco_build 4; fen 15; isolated_data_dir 4; notation 29; pgn 63; properties 3; rules 37; types 5. Matches the author's counts |
| Ignored deep perft, `cargo test --release -p chess-rules --lib -- --ignored` | 4 passed in 3.39 s (start depth 5 and 6, Kiwipete depth 5, positions 3/4/5/6) |
| Perft node counts | The counts in `src/testing.rs` are "recalled, not fetched" `[UNV]` by the author. I compared the depth 1 to 5 values for start, Kiwipete, positions 3, 4 and 5 with the published Chess Programming Wiki figures from memory; they agree `[INF]`, not fetched |
| `cargo llvm-cov -p chess-rules --summary-only` (separate target dir) | lines: eco 100, error 100, fen 92.45, key_derive 100, notation 97.48, pgn/mod 99.14, pgn/reader 96.92, position 99.39, testing 77.46, types 100; **TOTAL 97.04 %**. Branch column 0 (not reported by this toolchain). Matches the author's per-file figures; the author's total (96.99) differs by 0.05 pt because of `testing.rs` |
| `cargo mutants` on `fen.rs`, `key_derive.rs`, `eco.rs`, `types.rs` (modules the author did **not** mutate in part B) | 129 mutants: 77 caught, 46 unviable, 6 missed, 0 timeouts. The 6 missed are the same ones the author dispositioned in part A: five deletions of the unreachable `ParseFenError` mapping arms (fen.rs:83-87) and `|`->`^` on disjoint bit flags (fen.rs:204). See RF-07 on the dead arms |
| `cargo mutants` sample on `notation.rs` and `reader.rs` | see section 5.2 |
| ECO snapshot vs upstream | `curl` of the five TSV files at commit `3ba6b20d31b2e6125cb0e6a2e792f400717d9d39`: SHA-256 equal to `third_party/eco/SHA256SUMS` for all five (a to e) `[RAN]`. The commit's existence and CC0 text were not re-verified beyond that |
| Working tree after review | `git status --short` unchanged (23 entries before and after); all adversarial code lives in the scratchpad |

## 2 Gate check

GATE-102 is G1, OPEN, not approved, and says so. I agree no Q-table question forces G2 for the Rust code: one crate, no new workspace edge (`ci/check-crate-edges.mjs`: 11 edges), no §5.7 contract change. Two process points are in RF-12.

## 3 Findings

Sorted by severity, then by impact. No finding is a Blocker or Major; I looked specifically for one (panic, overflow, indexing, UB, false draw, illegal state, upstream-type leak) and did not find it in the paths listed in section 4.

### RF-01 [Minor][CONFIRMED] `PgnLimits::max_variation_depth` is unbounded; a large value turns hostile input into a stack-overflow abort
- Location: `src/pgn/mod.rs:22-26` (public field), `src/pgn/reader.rs:294` (only check), result type `PgnNode.variations: Vec<Vec<PgnNode>>` (`mod.rs:252-257`).
- Observation: the reader itself is iterative (explicit `frames`), but its output is a recursive tree. `Drop`, `Clone`, `Debug` and `PartialEq` of that tree recurse once per nesting level. The default (64) is safe; the SDD interface card (IFP-RUL-5) makes the limit a caller parameter with no ceiling, and F-B14 files the risk as "the caller's".
- Reproduction `[RAN]` (scratchpad `adv/tests/e.rs`, release): `1. Nf3 ` followed by N times `(Nf3 ` then N times `)`, `max_variation_depth = N + 1`. N = 5 000: parses and drops. N = 20 000 on a 2 MiB test thread, and N = 200 000: `thread has overflowed its stack`, `fatal runtime error: stack overflow, aborting`, SIGABRT. Parsing succeeded first; the abort is in `drop`.
- Impact: process abort (not a catchable panic) from a crafted PGN if any consumer lets the depth be configured above a few thousand, or passes `usize::MAX`. NFR-3 and the SDD resource row say "no unbounded recursion"; the output type breaks that promise whenever the caller raises the limit. Default configuration is not affected.
- Standards (intent): CERT MEM/ERR family intent "no unbounded recursion"; no claim.
- Recommended action: clamp the effective limit to a crate constant (for example `min(requested, 256)`), document the ceiling on `PgnLimits`, and add a test that `max_variation_depth: usize::MAX` on 100 000 nested parentheses returns `VariationTooDeep`. Alternatively give `PgnNode` an iterative `Drop`.
- Verify fix by: the test above on a 2 MiB thread, passing; same input with the limit raised does not abort.

### RF-02 [Minor][CONFIRMED] The reader emits a phantom empty game for a stray terminator
- Location: `src/pgn/reader.rs:355-363` (result word ends the game at depth 0), `reader.rs:648-684` (`parse_game` has no "game has content" check), `reader.rs:723-744` (`next`).
- Reproduction `[RAN]` (`adv/tests/i.rs`): `[Event "a"]\n\n1. e4 e5 *\n\n*\n` yields 2 items: game 1 with 2 moves and a second `Ok` game with 0 moves, tags all `?`, result Unknown. A lone `*` also yields one `Ok` empty game. A trailing DOS Ctrl-Z byte (`\x1a`) yields an extra `Err` item (game 2, "not a move").
- Impact: IF-2 asks for a report of "number imported" and failures by index. A file ending in a redundant `*` or `1-0` imports one extra empty game (or `chess-domain` must reject it, unspecified). The game index in later errors shifts.
- Recommended action: decide in the SDD whether a game with no tags and no moves is a game; if not, skip it in `next`. Add UT for the trailing terminator and for a trailing Ctrl-Z.
- Verify fix by: the two inputs above return 1 item and 1 item (or 1 Ok and 0 Err).

### RF-03 [Minor][CONFIRMED] Consecutive tag sections without movetext are merged; the first game's tags are lost without an error
- Location: `src/pgn/reader.rs:654-661` (the tag loop skips whitespace, including blank lines, and keeps reading `[` sections); `RawTags::set` (`reader.rs:699`) overwrites.
- Reproduction `[RAN]` (`adv/tests/d.rs`): `[Event "A"]\n[White "w1"]\n\n[Event "B"]\n[White "w2"]\n\n1. e4 e5 *\n` returns **1** game with `event = "B"`, `white = "w2"`. Game A vanished with no `Err`.
- Impact: silent data loss on import for a tag-only game (a game with no moves and no terminator). Low frequency, but it is the kind of case IF-2 says must be reported. The `NextGame` rule (F-B5 b) only fires after movetext has begun.
- Recommended action: a tag section after a blank line with no movetext between should end the previous game (or be an error `BadSyntax`), and a test should pin it.
- Verify fix by: the input above returns 2 items or 1 Ok and 1 Err, never 1 Ok with game B's tags only.

### RF-04 [Minor][CONFIRMED] Silent, unsignalled information loss on import (author's F-B6 and F-B7, confirmed and widened)
- Location: `src/pgn/reader.rs:216-230` (`attach_comment` drops a comment with no node), `reader.rs:699-717` (`RawTags::set`), `src/pgn/mod.rs:97-119` (`PgnDate::parse`), `mod.rs:183-203` (`TimeControl::parse`), `mod.rs:156-163` (`Termination::parse`), `reader.rs:163-174` (Latin-1 rule).
- Reproduction `[RAN]`: (1) `{ opening remark } 1. e4 e5 *` and `1. e4 (1. d4 {why} d5) e5 *`: both comments gone (`comment0=None`). (2) `[TimeControl "40/9000:3600"]` becomes `None`; `[Date "2024.13.45"]` becomes `2024.??.??`; an unlisted `Termination` is dropped. None of these reports a warning: the iterator item type has no warning channel, so `chess-app` cannot tell the user. (3) A game with one valid UTF-8 tag value (`café`) and another value holding byte `0xE9` decodes the whole game as Latin-1: the valid value becomes `cafÃ©`. This follows IF-2 / SDD 3.11.8 literally, but it corrupts correct text in a mixed file.
- Impact: ANA-2 and IF-2 keep "comments" and "variations" for analysis. A leading comment is the usual place for a game annotator's summary. Loss is silent and cannot be seen by the player. The TimeControl loss changes `canonical_form` input relative to the same game from another source (author F-B6).
- Recommended action: SE/SA decision (not a code-only fix): either keep a game-level `preamble` comment and the raw text of unparsed typed tags, or add a `warnings` list to the reader output. Until then, record the loss in the user-facing import notes. For the Latin-1 rule, decode per payload (valid UTF-8 stays UTF-8) or document the mojibake risk in IF-2.
- Verify fix by: UT with the inputs above asserting the preserved or reported data.

### RF-05 [Minor][CONFIRMED] `canonical_form` is not injective on inputs the type system allows, and there is still no version constant
- Location: `src/pgn/mod.rs:513-543` (`canonical_form`; `break` on an illegal move at 529 and 536), `mod.rs:314-329` (`tag_line`: any control character becomes a space).
- Reproduction `[RAN]` (`adv/tests/d.rs`): (a) a hand-built `PgnGame` with `e2e4, e2e4, d7d5` and the same game without the last node give byte-identical output (`collide on illegal tail: true`); the writer rejects the same game (`IllegalMove{ply:1}`). (b) `white = "a\tb"` and `white = "a b"` give identical bytes (`collide ctrl char: true`).
- Impact: `content_hash = sha256(canonical_form)` is the duplicate key for imports (IF-2, D-42). (a) needs a game built by hand (the reader never produces it), so exposure is `chess-app`'s own construction code. (b) is reachable from import: two PGN files that differ only by a tab versus a space in a tag are one game. The hash string `v1` is stored on disk (SF-03), so any later change is a stored-format change.
- Author's F-B9 (no version constant) is correct and I agree it should be closed before the first release stores a hash: `pub const CANONICAL_FORM_VERSION: u32 = 1;` next to the function costs nothing and lets `chess-app` assert it. The golden test (`tests/pgn.rs:617-636`, two games chosen by the author, not by the SDD or SE) pins the Seven Tag Roster, escaping, `TimeControl` and the movetext joiner, but not the `FEN`/`SetUp` ordering bytes (those are `contains` checks, `tests/pgn.rs:742`).
- Recommended action: (1) owner (SE or SA) confirms or replaces the golden fixtures and adds one with a custom start; (2) add the version constant; (3) either make the illegal-move case unrepresentable (a validated `PgnGame` constructor) or return `Result`; the SDD says "total", so at minimum document that the function is injective only for legal games; (4) document (b) as intended or switch the replacement to an escape.
- Verify fix by: UT for each of (a) and (b) as the SE decides.

### RF-06 [Minor][CONFIRMED] `IllegalReason::Other` is reachable from ordinary input, contrary to SDD 3.11.2
- Location: `src/position.rs:282-285` (`promotion.is_some() && !promotes` returns `Other`), used by `Position::play`, `parse_move` Typed coordinate path (`src/notation.rs`, `resolve_coordinate`).
- Reproduction `[RAN]` (`adv/tests/c.rs`): `Position::standard().parse_move("e2e4q", Typed)` gives `NoMatch { reason: Other }` ("illegal move: the move is not legal"); `play(Move{e2e4, promotion: Some(Queen)})` gives `IllegalMove{reason: Other}`. In a 1 998 848 call sweep of `explain_illegal` over about 240 playout positions (4 seed positions, 60 random plies each; plain move and queen-promotion probes for all 4 096 square pairs), all 7 782 `Other` results came from a promotion letter on a non-promoting move; none from a plain move (`bad = 0`, `Other-for-plain-illegal = 0`), so the explainer is otherwise consistent with `is_legal` `[RAN]`.
- Impact: SDD 3.11.2 defines `Other` as "a defect indicator; logged by the caller, never shown as a rule". A client sending UCI `e2e4q` (typo, or an engine bug) will be logged as a rules-library defect, and the player sees an unhelpful message.
- Recommended action: return a specific, stable reason for "a promotion piece on a move that does not promote" (SDD owner decides: new variant, or `CannotReach`), and keep `Other` for genuine defects.
- Verify fix by: UT `e2e4q` returns the new reason; the property sweep above shows zero `Other`.

### RF-07 [Minor][CONFIRMED] Error taxonomy: part of the "stable enum" is free text, and some mappings are dead or lossy
- Location: `src/error.rs` (`FenError::BadField{detail: &'static str}`, `PgnErrorKind::BadSyntax{detail}`, `BadTag{detail}`, `IllegalPosition::Other(String)`), `src/fen.rs:80-90` (upstream `ParseFenError` fallback `_ => FenField::Placement`, detail "not accepted"), `src/pgn/reader.rs:201-214` (`kind_from`).
- Observation: SDD 3.11.2 / 5 say "the enum is the stable part (NFR-9)". `BadSyntax` carries ten different failures (stray `}`, variation before a move, NAG out of range, token too long, not text, empty token...) distinguished only by an English `&'static str`; a front-end that wants to localise or branch must compare strings. `kind_from` folds `Empty | TooLong | Syntax | BadUci` into "not a move" and drops the `LowercasePiece` suggestion. In `Fen::parse` the five upstream-error arms cannot be reached after the crate's own field checks (mutation: deleting each survives, section 1), and the catch-all attributes any unknown upstream error to `Placement`.
- Impact: low today (no consumer). Becomes a compatibility cost when `chess-app` D-51 maps these to wire kinds.
- Recommended action: give the repeated reasons their own variants (or numeric codes), keep `detail` for display only; delete the dead arms or make the fallback an `unreachable`-style `Illegal(Other)` with a test; keep the `LowercasePiece` suggestion through `kind_from` if the reader's error is ever shown.
- Verify fix by: UT per reason; mutation of the removed arms no longer survives.

### RF-08 [Minor][CONFIRMED] `ParseMode::Pgn` is documented as strict but is lenient in the wrong places and strict where real files are not
- Location: `src/notation.rs:34-38` (doc comment: "`Pgn` is strict (RULE-7)"), `notation.rs:166-172` (`file_of` accepts `A`-`H`), `notation.rs:293-346` (`parse_san` accepts a from-file and from-rank on a pawn move), `notation.rs:118` (`0-0`), `notation.rs:100-114` (any run of `+ # ! ?`).
- Reproduction `[RAN]` (`adv/tests/b.rs`, `i.rs`): Pgn mode accepts `E4`, `e2e4` (as SAN pawn with over-disambiguation), `Ng1f3`, `0-0`, `e4+++`, `e4!?!` (NAG dropped for the last), `a8q`, `a8Q` without `=`. It rejects, failing the **whole game**, `e.p.` after a capture (`3. exf6 e.p.`), the annotation tokens `+-` and `=`, a FEN tag with a trailing space or only four fields (`BadFen`), and castling rights with no matching rook in a `[FEN]` tag.
- Impact: author's F-B12 is right that only legal moves are accepted (no illegal SAN becomes a different legal move; FM-RUL-003 holds, and my differential run (400 000 texts, each in both modes) found no case where an `Ok` was not in `legal_moves()`). The harm is the misleading comment and the reverse: IF-2 asks for "the PGN Standard's import format (the flexible, hand-preparable form)", and several common real-world spellings kill the game. Whether the Standard's import grammar allows each of them is `[UNV]` (I did not re-read it, nor did the author).
- Recommended action: SE/SA decision on which spellings to accept; correct the doc comment either way; add UT for the decided list.

### RF-09 [Minor][CONFIRMED] `Fen::parse` and the 960 question
- Location: `src/fen.rs:60-95`.
- Observation `[RAN]` (`adv/tests/k.rs`): a Chess960 start `bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9` is rejected (castling syntax) and with `KQkq` is rejected (rights do not match); with castling `-` it is **accepted** as an ordinary game start with no castling. So "960-style FEN rejection" is only a rejection of the castling field, not of a non-standard back rank. Counter-case: `0 01` is accepted and normalised to `0 1`. Counters saturate rather than overflow: `... b - - 4294967295 4294967295` plus one move gives `... w - - 4294967295 4294967295`, no panic `[RAN]`. This closes ASM-A3 (upstream increment at the limit), but it means a game at that counter silently stops counting.
- Impact: none for correctness of rules. A user who loads a 960 position with `-` plays it under standard rules without a message.
- Recommended action: accept as designed, or have the SE state whether a non-standard back rank without castling should be refused. No code change required if accepted; record the answer.

### RF-10 [Minor][CONFIRMED] Memory amplification of the reader is about 20x and unmeasured by the author
- Location: `src/pgn/mod.rs:252-257` (`PgnNode` is 80 bytes `[RAN] size_of`), SDD 3.12 ("The reader holds one game at a time").
- Reproduction `[RAN]` (`adv/tests/j.rs`): a single 10 000 000 byte game of knight shuffles (2.5 M plies) parses in a release build and peaks near **215 MB** resident (`/usr/bin/time -l`, includes the test harness, so an upper bound). One game of the TBD-25 estimate (10 MB) is therefore a few hundred MB on top of the file bytes.
- Impact: a desktop import of a hostile 10 MB single game is a memory spike, not a crash on a normal Mac. `chess-app` must not hold several such games. Time is fine: 10 M garbage tokens 213 ms, 500 000 bad games 275 ms, 300 000 sibling variations 136 ms, 50 MB comment 53 ms (release) `[RAN]`; no quadratic path found.
- Recommended action: record the factor in SDD 3.12 and TBD-25; consider a per-game ply limit.

### RF-11 [Minor][CONFIRMED] ECO build: integrity is checked only in CI, and the build prints a `cargo:warning` every time
- Location: `build.rs:22-28` (no hash check), `build.rs:23-28` (`cargo:warning`), `ci/check-eco.mjs`.
- Observation: SDD 3.11.9 says "CI verifies the recorded SHA-256 of each TSV; `build.rs` does not". Accepted by design. Consequence: `cargo build` or `cargo test` on a tampered or truncated TSV that still replays produces a different opening table with no failure; only `ci/check.sh` notices. `SHA256SUMS` sits in the directory it protects, so it detects accident, not an attacker who edits both. I confirmed the committed files match upstream commit `3ba6b20d…` `[RAN]`. `cargo:warning` appears on every rebuild (author's F-B17). `third_party/eco/a.tsv` has a later mtime than b to e (10:44 vs 10:41); its hash equals upstream, so no content change.
- Recommended action: accept the CI-only gate and say so in `SOURCE.md` (build-time hashing would need a hash crate, a dependency decision for Brian Prascher). Move the counts to a file in `OUT_DIR` if the noise matters.

### RF-12 [Minor, process][CONFIRMED] Gate record understates the footprint of the ECO pass
- Location: `docs/engineering/gates/GATE-102-chess-rules.md` part B "Components touched: crates/chess-rules only"; part A line "no CI file edited".
- Observation: part B edits `ci/check.sh` (shared CI, one new step), adds `ci/check-eco.mjs`, and adds `third_party/eco/` (a third-party data set with a licence obligation, NFR-6, and a one-time external fetch). The record does list these in prose and says "report to the owner of `ci/`". `build.rs` also makes the crate depend on a build-script that reads outside its directory (`../../third_party/eco`), so the crate is no longer self-contained for `cargo package` (`publish = false`, so not an issue today).
- Impact: the Q-table gives G1; I do not claim G2. But the SDD that this implements is still 0.3.5 DRAFT (not baselined), and the in-crate tokenizer replaces the SDD's named `pgn-reader` wrapper (OQ-RUL-001 escape route, F-B4/F-B19). The System Architect desk review should see the CI and data-asset footprint and the SDD amendment need explicitly.
- Recommended action: add the CI and `third_party/` rows to the "Components touched" line; the SA decides if the CI edit needs Brian Prascher (G2: "touches ... a dependency edge, data ownership"). The SDD amendment for the tokenizer is the SDD owner's.

### RF-13 [Info][CONFIRMED] `Position`'s `Debug` prints the upstream `Chess` structure
- Location: `src/position.rs:12-15` (`#[derive(Debug)]` on a struct holding `shakmaty::Chess`).
- No upstream **type** is nameable from the public API: I grepped `src/` for `shakmaty` outside `position.rs`, `fen.rs`, and the unit-test module of `testing.rs` (only comments), all `pub` signatures use crate types, and `pub(crate) fn chess()` is crate-private `[OBS]`. The `Debug` output is textual, so it is a stability concern, not a type leak. CT-RUL-001 remains a grep and a rustdoc check, not `cargo public-api` (author's `[UNV]`).

### RF-14 [Info][CONFIRMED] Typed-mode `B` fallback accepts a wrong-looking spelling
- Location: `src/notation.rs:135-150`.
- `[RAN]`: in the start position Typed `Bb4` returns `b2b4` (b-pawn push, canonical `b4`), because "B + b4" has no bishop match and the fallback reads `bb4`. `B4` is a `Syntax` error. Behaviour is inside the SDD's step 6 wording; the inconsistency between `Bb4` (accepted) and `B4` (rejected) is the user-visible tell. Author's F-B11 (`B2d5`) is consistent with this and I found no case where a legal `Ok` was wrong. Left for SE confirmation (CLI-3).

### RF-15 [Info][CONFIRMED] `wrap` can produce lines longer than 79 characters
- Location: `src/pgn/mod.rs:416-435`.
- `[RAN]`: a comment of 400 characters yields a 401-character line. SDD 3.11.7 step 5 allows it ("a token longer than the line limit stands alone"), so the code conforms. The PGN Standard's own export limit is 255 characters per line `[UNV]` (recalled). A strict external reader would still read the file; consider splitting long comments at spaces.

### Author-raised items attacked and their disposition (summary)
| Author item | My result |
|---|---|
| F-A2 / F-B1 error shapes (`MoveParseError` no longer `Copy`) | Acceptable; no consumer; see RF-07 for the free-text weakness |
| F-B2 new public types | Reasonable; `PgnTags` stores `String` for roster fields, `extras: BTreeMap` gives deterministic order `[OBS]` |
| F-B5 reader leniencies | (a) FEN honoured without `SetUp`: safe. (b) tag line ends an unterminated game: right, but incomplete (RF-03). (d) last tag wins: fine. Also `SetUp` values other than `0` all enable the FEN |
| F-B6, F-B7 lossy import | Confirmed (RF-04) |
| F-B8 UT-RUL-031 (5) not built | Agreed it is a cross-crate item; it is the guard for `chess-domain` drift and stays open |
| F-B9 / canonical golden bytes | Confirmed and widened (RF-05) |
| F-B11 `B2d5` | Confirmed behaviour, no wrong move found |
| F-B12 over-disambiguated SAN | Confirmed, harmless to legality (RF-08) |
| F-B4 / F-B19 in-crate tokenizer vs `pgn-reader` | The argument holds (no line/column in `pgn-reader` 0.29.1) `[UNV]` by me; I did not re-open the registry source. The cost is that the PGN grammar is now this crate's code and test burden, and the SDD text is stale |
| Perft cross-check uses shakmaty itself | True for the `shakmaty::perft` comparison only (`src/testing.rs:127-135`). The published node counts are the independent check and they are fixed literals; my comparison with the published values (above) agrees. The adapter is exercised by `play`/`legal_moves`, not by the upstream generator alone |
| Mutation survivors (author section 6) | I did not re-prove each as equivalent; see 5.2 for my sample |

## 4 Per-criterion check

| Criterion | Result | Evidence |
|---|---|---|
| Panic, overflow, indexing on malformed FEN | None found | 500 000 mutated FENs: 6 965 accepted, 493 035 rejected, no panic; every accepted FEN reparses to itself (`as_str` fixpoint), `to_fen` equals parse result, up to 5 legal moves played and re-serialised per case, `legal_moves().len() <= 218` `[RAN]`. Counter extremes (`4294967295`, `4294967296`) handled. `src/` has no `unwrap`, `expect`, `panic!` or indexing outside `#[cfg(test)]` `[OBS grep]`, clippy denies them |
| Malformed SAN / move text | None found | 400 000 texts (half mutated real SANs, half random from a 60-character alphabet) over 2 979 playout positions, each parsed in both modes (800 000 parses): every `Ok` is in `legal_moves()`, `uci` and `san` equal the recomputed values, no panic. Non-ASCII, NUL, empty, 17+ bytes, `e8=K`, `a8=`: typed errors |
| Malformed PGN, huge input, non-UTF-8, deep nesting | No panic, no hang; one design hazard | 300 000 mutated PGN files (≤ 20 edits of 2 seeds): `items <= bytes + 1`, every `Ok` game rewrites with `write_pgn`, rereads as exactly one game, same `canonical_form`, and the writer is idempotent (158 K games, 252 K errors) `[RAN]`. 2 M nested `(`: one `VariationTooDeep` in 58 ms. 50 MB comment, 200 K games, 10 M junk tokens, 500 K bad games, 1 M-ply game: all under 0.5 s release. Non-UTF-8: Latin-1 per game works and mojibake risk noted (RF-04). Hazard: RF-01 |
| Adversarial ECO / build paths | No defect in the generator | Table has 3 865 rows, 0 duplicates `[RAN]`; lookup agrees with transposition (Ruy Lopez via two orders C60, Queen's Gambit via two orders D06) and returns `None` after a king shuffle (castling right lost, by design of the key). Corrupt-row and missing-file behaviour is covered by `tests/eco_build.rs`. Integrity is CI-only (RF-11) |
| SAN edge cases | Correct | promotion with and without `=` and in either case, capture-promotion, missing promotion (`PromotionRequired` with four choices), disambiguation by file, rank and square (`Ra3`, `R5e3`, `R1e3`, `Nbd2`, `Nfd2`; ambiguous `Re3` lists `R1e3, R5e3`), en passant (`exd6`) and **horizontal-pin en passant** (`8/8/8/KPp4r/8/8/8/7k w - c6`: `bxc6` is `LeavesKingInCheck`, key ep `-`), castling through check, out of check and blocked, queenside with attacked b1 (legal), black castling, `Kxh1` and `e1h1` (not castling), 960 castling FEN rejection (RF-09) |
| Draw / repetition key correctness | Correct on tested cases | `PositionKey` is an exact array (no hash). En passant included only when `ep_square(Legal)`; two FENs differing only in an unusable ep square give different `Fen` but equal keys `[RAN]`; counters excluded; castling rights part of the key. Dead-position rule: shakmaty's test equals the SDD 3.11.3 reference for the cases I ran (KN v K dead; KN v KQ: only Black can mate; KN v KR, KN v KB, KN v KN: both can) `[RAN]` plus the author's fixture table; I did not run RSK-RUL-002 (an independent mate-search oracle), the author did not either (F-A4) |
| Public API leaking upstream types | None | RF-13 |
| Error taxonomy completeness | Complete for what the SDD lists; stringly in places | `IllegalReason` consistent with `is_legal` (zero `Other` for plain illegal moves in 2 M probes); RF-06, RF-07 |
| `unsafe`, suppressions | Clean | `#![forbid(unsafe_code)]` (`src/lib.rs:9`); no `allow`/`expect` attributes in `src/`, `build.rs`, `build/`; the only `allow(...)` are file-level in test crates and each carries a `reason` `[OBS grep]` |
| Determinism (PLAT-1) | Clean | No `HashMap`, `HashSet`, clock or randomness in `src/`, `build.rs`, `build/` `[OBS grep]`; extras are a `BTreeMap`; the ECO table is a sorted static; writer output is a pure function (idempotence run above) |
| Concurrency | n/a | Pure library; `Send + Sync` asserted for the public types (`tests/pgn.rs` UT-RUL-025) |

### 4.1 CERT-informed notes (no conformance claim)
Rust has no CERT standard; the checks below are by intent. Integer arithmetic: saturating or checked conversions (`File::to_char` uses `saturating_add`), counters saturate at `u32::MAX`; array access: indexing denied by clippy, `get()` used; error handling: the only ignored `Result`s are `let _ = writeln!(String)` in `build/eco_gen.rs` (infallible on `String`) and the test-support `map_or(0, …)` in `perft`; memory: `forbid(unsafe_code)`; recursion: reader iterative, writer bounded by `DEFAULT_MAX_VARIATION_DEPTH` (64), output-type recursion is RF-01; resource exhaustion: input size is the caller's (TBD-25), amplification RF-10; input validation at boundaries: FEN length, move text length, ASCII check, UTF-8 check. Tool coverage: rustc, clippy `-D warnings` with the workspace deny list, rustfmt, cargo-deny. Not checked: `cargo-geiger`, `miri` (no unsafe so low value), `cargo public-api`.

## 5 Coverage, mutation and fuzz evidence check

### 5.1 Coverage
Author's per-file numbers reproduced to two decimals (section 1). Target (test-strategy: at least 90 % line for rules and parser modules) is met in every module; the lowest non-test module is `fen.rs` at 92.45 %. Branch coverage is not available from this toolchain (column 0), so decision coverage is unmeasured. The 3.0 % of uncovered reader lines are the heuristic arms of skip recovery that the author lists as unexercised equivalents.

### 5.2 Mutation
The author's part B run covered `notation.rs`, `pgn/mod.rs`, `pgn/reader.rs` (last run 12 missed of 264 on pgn files; 1 missed on `notation.rs`). I re-ran independently on the modules the author did not mutate (section 1: 6 missed of 83 viable, all author-dispositioned) and on a shard of `notation.rs` and `reader.rs`: `notation.rs` shard 1/6 (20 mutants): 11 caught, 9 unviable, **0 missed**; `pgn/reader.rs` shard 2/8 (21 mutants): 6 caught, 15 unviable, **0 missed**, 0 timeouts. That is 41 mutants of roughly 400 in those files, a 10 % sample chosen by cargo-mutants' shard split, not by me; it does not contradict the author's table of 1 + 12 survivors, and it does not test them (the sample hit none of the survivors the author lists). I did not run the full 400. Mutation says nothing about RF-01 to RF-06: those are specification and input-space gaps that no existing test exercises, which is why they survived 160 passing tests.

### 5.3 Fuzz / random-input
The project's NFR-3 evidence is `proptest` with no coverage-guided fuzzer (D-76: accepted loss; `cargo-fuzz` is not installed here and is not required). The committed property tests run 64 cases (play/legal, FEN round trip) and 128 cases of at most 400 bytes (reader), plus fixed hostile inputs (50 000-deep nesting, 100 000 closers, 2 MB comment and tag, binary data, 1 MB token) `[OBS tests/pgn.rs:893-975]`. That is a thin random budget. I supplemented it with seeded differential runs in the scratchpad (section 4: about 1.6 M parse, FEN and reader executions plus 2 M `explain_illegal` probes, with overflow checks and debug assertions on); none found a panic, hang, `Ok` outside `legal_moves()`, or reader/writer round-trip break. This is stronger than the committed evidence but is still not a coverage-guided fuzz and I claim nothing beyond "no failure found in these runs".

## 6 Verdict

**PASS WITH CONDITIONS.**

Reasoning. I tried to produce a panic, overflow, index error, illegal state, false-draw key, illegal move accepted by the parser, upstream-type leak, non-determinism and unbounded loop. Within the tested inputs none exists: the build and gate script is green, coverage reproduces, the unmutated modules' mutation result reproduces, and independent random differential runs found no failure. The real weaknesses are at the edges the author partly knew: a configurable depth that can abort the process through the output type (RF-01), reader behaviours that silently drop or invent games (RF-02, RF-03), silent information loss with no warning channel (RF-04), a hash input that is not injective and not versioned (RF-05), a defect-indicator reason reachable by user input (RF-06), and a free-text part in a taxonomy described as stable (RF-07). None is a Blocker or Major for a local, single-user, ordinary-criticality application (SWE-001), but RF-05 involves a stored hash and is a one-way door once released.

Conditions (to be dispositioned by named owners; the reviewer does not fix):
1. RF-01: clamp or document-and-test the depth ceiling before any consumer exposes `PgnLimits`.
2. RF-05: add the version constant and have the SE confirm or replace the golden bytes **before the first stored `content_hash`**; decide (a) and (b).
3. RF-02, RF-03, RF-06: fix or record an owner decision; add the missing tests.
4. RF-04, RF-08, RF-09: SE/SA decision on what the importer must preserve, report and accept; record it in the SDD (which is still 0.3.5 DRAFT and needs the tokenizer amendment).
5. RF-12: complete the gate record's footprint line; the SA desk review is still needed, and nothing here replaces it.

Not satisfied by this review and still open: GATE-102 approval (System Architect desk review), UT-RUL-031 (5), CT-RUL-002 (no consumer yet), the independent PGN reader check on writer output (RSK-RUL-003), and the 10 MB import time measurement.

Dispositions available for each finding: FIXED (hunk), REJECTED (reason, by whom), ACCEPTED-RISK (named human), DEFERRED (RSK-/OQ- id). All are currently **OPEN**. No human approval, SE acceptance, SA desk review or deviation approval is claimed.

## 7 Not reviewed, not run, assumptions

- Not run: Intel Mac, `npm audit` (offline), `cargo public-api`, `miri`, `cargo-geiger`, a coverage-guided fuzzer, the independent PGN reader check on the writer's output, the SDD 3.11.3 brute-force mate oracle, the 10 MB import timing in a debug build, reading the PGN Standard and the FIDE Laws text (RF-08, RF-15 and the perft literals rely on memory `[UNV]`).
- Not reviewed: `tests/common/mod.rs` helper bodies beyond the sections used, `tests/isolated_data_dir.rs`, the full text of every one of the 160 tests (I sampled the golden, fuzz, reader-recovery and canonical-form tests and read all of `src/` and `build*`), documents other than those named in the scope, `.beads` state, the unrelated modified files in the tree (`docs/engineering/dependency-lookup-step1.md`, `ruling-test-helper-placement.md`).
- Assumptions: ASM-R1, the SDD 0.3.5 text is the intended behaviour even though it is not baselined; ASM-R2, the scratchpad crate `adv` (path dependency on the crate, `test-support` feature, overflow checks on) exercises the same code as the repository tests; ASM-R3, shakmaty 0.30.2 behaviour is as observed here and not re-derived from its source.
- Scratch artifacts (outside the repository): `/private/tmp/claude-501/-Users-brianprascher-Projects-chess/093edff2-fc6d-44b4-88b5-ba2bb1914e40/scratchpad/adv/tests/{a..l}.rs`, `cov.txt`, `mut/`, `mut2/`.
