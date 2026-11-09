# REV-chess-domain-code: adversarial code review of `crates/chess-domain` (parts A and B)

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-domain-code.md |
| Task | chess-a0r.2.8 (baseline B4) |
| Date | 2026-11-06 |
| Stance | **Reviewer** (software-engineer agent, fresh invocation, not the author of parts A or B). Independence: independent of the author invocation; same agent type. Desk review only. It is **not** the System Architect desk review and **not** an approval of GATE-103 |
| Scope | `crates/chess-domain/{Cargo.toml,src,tests,tests/data}`, working tree (uncommitted) |
| Intent sources | docs/design/SDD-chess-domain.md 1.0 (baselined), readiness-step2-3.md, SWE-001, test-strategy.md, requirements.md (ELO-1), architecture 3.2 / 5.7 / 7.6, ruling-test-helper-placement.md |
| Standard | SEI CERT applied **by intent only** (panics, overflow, unchecked conversions, untrusted input). **No conformance claim** |
| Method | Code and tests read first; the author evidence files read afterwards; claims re-run. Throwaway tests lived only in a scratch copy of the workspace (outside the repo). The repo working tree is unchanged by this review except this file |
| Verdict | **PASS WITH CONDITIONS** (section 6). No Blocker. One Major (DF-01). Nine Minor, five Info |

## 1. Baseline and tool runs

| Check | Command | Result |
|---|---|---|
| Quality gates | `CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh` | `[RAN]` exit 0, "All gates passed" (fmt, build, clippy -D warnings, test, crate-edge, ECO checksums, cargo-deny offline). npm audit skipped by design |
| chess-domain tests | inside `ci/check.sh` | `[RAN]` 8 unit + 15+14+2+4+8+16+41+16+23+20+5+10+8+8 integration tests, all ok |
| Coverage | `cargo llvm-cov -p chess-domain --offline --summary-only` (scratch copy, author tests only) | `[RAN]` regions 97.60 % (97 missed), lines 98.09 % (48 missed of 2507), functions 96.69 %. Matches the author's 98.09 % line figure. Branches: 0 reported by this build, MC/DC not measured and not claimed. Lines include `cfg(test)` modules inside `src/` (author says so) |
| Mutation re-run (sample) | `cargo mutants -p chess-domain --file src/{stats,variation,eco,board_grid,totals}.rs --no-shuffle -j 4` | see section 5 |
| Randomized state sweep (mine) | scratch `tests/adv.rs`: 8 setups x 400 seeds x 300 random commands, fake time with 0, 61 s, 100 s jumps and backwards readings; after every step `check_invariants`, `replay(all facts)` against live with `journal_eq`, and "`decide` never returns `InvalidTransition`" | `[RAN]` 0 problems. About 45 000 commands applied, all 19 command kinds exercised, including Takeback (1 751), Step (928), EngineFailed (999), Suspend (850), Void (792) |
| Pure-crate rule | `grep -rn "tokio\|rusqlite\|std::fs\|std::time\|SystemTime\|Instant\|std::env\|rand::" src`; `cargo tree -p chess-domain -e normal,build` | `[RAN]` no hits; normal tree = chess-rules (shakmaty, thiserror), serde, thiserror, uuid. No tokio, rusqlite, rand, getrandom (uuid `v5` only, default features off) |
| `forbid(unsafe_code)` | `src/lib.rs:10` | `[OBS]` |
| Suppressions | `grep allow(` in `src` | `[OBS]` three, each with a written reason: `elo.rs:139` (`as i32` after range check), `game.rs:309` (`new` returns `Step`, SDD name), `stats.rs:152` (u128 to f64 for a mean). All acceptable |

## 2. Gate check

Reviewed as the author's GATE-103 tier (G2, two crates' edges: chess-domain to chess-rules and the `uuid`/`serde` dependencies, per SDD). The review found no new dependency edge, no new component and no interface outside the SDD 5.7 curated list. Public API names `chess_rules::{Side, Move, Position, PositionKey, Fen}` but no `shakmaty` type `[OBS grep, lib.rs]`. `Orientation`, `BoardGrid` and `EcoMatch` are domain types. Approval status of GATE-103 is not examined here and nothing in this review approves it.

## 3. Findings

Confidence: CONFIRMED (reproduced), LIKELY (strong reading), SUSPECTED. Severity: Blocker, Major, Minor, Note (Info).

### DF-01 [Major][CONFIRMED] The final clock of a finished game is not journaled, and the replay property test is blind to it (author F-D8, attacked, not accepted)
- Location: `src/game.rs:680-687` (`Fact::Ended` arm never touches the clock), `src/game.rs:465-467` (`journal_eq` skips the clock comparison when the persistent state is finished).
- Reproduction `[RAN]` (scratch `tests/adv4.rs`): strict h-h game, 10 s base. `decide(Flag)` at t=11 s gives live White remaining `0`. `replay(Start, Ended)` gives White remaining `10 000 ms`. Second case, 5+0 game, White moves at 1 s, Black resigns at 201 s: live Black remaining `100 000`, replayed Black remaining `300 000`. In both cases `replayed.journal_eq(&live)` is **true**.
- Impact: after a crash between `Ended` and the finalize transaction (or on any restore), a game that ended `TimeForfeit` shows the loser with its full time. `GAME.white_clock_ms` and per-side time derived from the clock are wrong for flag, resign, draw-agreed and ResolveTerminal endings. The author's "equality of live and replay" property (SDD 3.11.1 "field by field") is satisfied only because the comparison is switched off exactly where the fields differ. The author disclosed the gap (F-D8) but not that it makes the property vacuous for finished games.
- Standards: CERT intent: do not let a check pass by excluding the failing case.
- Recommended action: SE/SA decision on a `Clock` fact before `Ended` (journal contract, G2/G3), or an explicit statement in the SDD that final clocks come from `view()` at end time and are never recomputed from the journal; add a test that pins the chosen behavior for flag and resign.
- Verify fix by: the two reproductions above asserting the chosen result; `journal_eq` compares the clock for finished games or the SDD records why not.
- Status: OPEN

### DF-02 [Minor][CONFIRMED] `Tally::games()` adds `u32` without saturation
- Location: `src/stats.rs:54` (`self.wins + self.losses + self.draws`), while `Tally::add` (line 63) saturates.
- Reproduction `[RAN]`: `Tally{wins: u32::MAX, losses: 1, draws: 0}.games()` panics "attempt to add with overflow" in debug; wraps in release (silent wrong count, and `PlayerStats::games` and the empty-state rule use it).
- Impact: unreachable with real data (4 billion games) but the crate states "all arithmetic saturates" elsewhere and the fold itself saturates, so the two disagree. CERT INT30/INT32 intent.
- Action: `saturating_add`; one test at `u32::MAX`.
- Status: OPEN

### DF-03 [Minor][CONFIRMED] `VariationTree::to_pgn_text` fails for a legal tree nested about 100 variations deep; the documented "not reachable" claim is false (author F-B9)
- Location: `src/variation.rs:224-227` (maps any `format_line` error to `TreeError::BadText`); doc comment at line 222 says the error arm "is not expected to be reached". `MAX_NODES` is 4096 (line 11).
- Reproduction `[RAN]` (scratch `tests/adv3.rs`): a chain where each node has an alternative leaf plus the continuing move. N=50 nesting levels: text 772 bytes, round-trips. N=100, 200, 500, 1000: `to_pgn_text()` returns `Err(BadText)`. The tree was built only through the public `play` API.
- Impact: a what-if tree within the crate's own size limit cannot be exported as a whole; the failure is mislabeled as "bad text". `saved_line` (linear) is unaffected. Also `line_nodes`/`branch_nodes`/`add_line` recurse to a depth bounded only by `MAX_NODES` (comment at line 181); at 2 MiB thread stack the 2046-level tree did not overflow `[RAN]`, so no stack-overflow claim is made.
- Action: either limit nesting depth in `play` (a new `TreeError`) or return a distinct error; correct the comment and F-B9; add a test at depth 100.
- Status: OPEN

### DF-04 [Minor][LIKELY] Arena grows without bound under play/delete cycles
- Location: `src/variation.rs:132-157, 160-178`. `MAX_NODES` limits live nodes (`self.live`), but `nodes: Vec<Node>` only grows (tombstones are never reused, by design, line 3). Each `Node` holds a `Position`.
- Impact `[INF]`: a long what-if session of play, delete, play, delete grows memory until `u32` ids run out (`Full`) or memory is exhausted. Not run to exhaustion.
- Action: cap total arena length, not only live count, or compact on delete when no id is held (SDD decision needed since ids must stay valid).
- Status: OPEN

### DF-05 [Minor][CONFIRMED] Replay accepts facts that contradict the position, and rejects a late `Clock` fact
- Location: `src/game.rs:680-687` (`Ended` outcome is stored unchecked), `src/game.rs:619-622` (`Clock` after a finished state is `WrongState`).
- Reproduction `[RAN]`: `Start` then `Ended{WhiteWins, Checkmate}` on the starting position replays with no error and state `Completed`. `Start`, `Ended`, `Clock` stops with `WrongState{Completed}`.
- Impact: (a) a corrupt or hand-edited journal produces a "checkmate" with no mate; the domain trusts the record (acceptable if the journal hash covers it, but the SDD says replay has "one interpretation"). (b) If the app's periodic 1 s checkpoint is appended after `GameEnd` (a race the app owns), the game becomes `JournalUnreadable` although nothing is wrong. SDD 3.11.1 says `Clock` "never changes the state" and does not say it is refused after the end.
- Action: SE/SA ruling: either accept and ignore `Clock` after a finished state, or state in the SDD that the app must never write it. For (a) record the trust decision.
- Status: OPEN

### DF-06 [Minor][CONFIRMED] Identity types accept non-canonical UUID text; no serde-format test exists for `GameSetup`
- Location: `src/types.rs:14-45` (`Uuid::try_parse`, `#[serde(transparent)]`).
- Reproduction `[RAN]` (via `FromStr`): uppercase hyphenated, 32 hex without hyphens, `{...}` braced and `urn:uuid:...` all parse and re-display as the canonical lowercase hyphenated form. Integers and the empty string are rejected, as the SDD intends (D-40). The serde path uses the same `uuid` visitor `[INF]`.
- Impact: stored/wire form is canonical on output but not on input; two spellings of one id can reach a layer that compares text. Separately, `tests/` has no JSON-level test of `GameSetup` (unknown field rejected, missing `clock` key, integer id rejected): `serde_json` is not a dev-dependency, only `StrDeserializer` for `ClockSpec` (`tests/policy.rs:718-724`). By serde semantics a missing `clock` key deserializes to `None` (untimed) `[INF, not run]`, which is lenient for a received command.
- Action: decide the canonical-input rule (hyphenated lowercase only, or document leniency); add serde round-trip and reject tests for `GameSetup`, `Seat`, `Personality`.
- Status: OPEN

### DF-07 [Minor][LIKELY] `check_setup` accepts one Player on both seats with different kinds
- Location: `src/policy.rs:393-421`, `is_self_play` at line 95.
- Observation `[OBS]`: kinds are checked per matchup, but a human-vs-computer setup with the same `PlayerId` on both seats passes; `is_self_play` then returns true, the game is unrated and `slot_holders` returns only the human. A Player is either human or computer, so this is an inconsistent setup the domain should refuse.
- Not run. Action: reject `white.player == black.player` unless both kinds equal; test.
- Status: OPEN

### DF-08 [Minor][CONFIRMED] The policy test oracle is a transcription of the implementation
- Location: `tests/policy.rs:90-200` against `src/policy.rs:277-376`.
- Observation `[OBS]`: the comment says the oracle is "deliberately not shared". It is a second copy of the same decision list: same order of checks, same reason precedence, same sub-conditions, line for line (for example Takeback, OfferDraw, RespondDraw, Pause, Resign). An exhaustive sweep of 2 modes x 4 seat sets x 10 states x 2 x 3 x 3 contexts proves the two copies agree; it cannot catch a shared misreading of SDD 3.11.4/3.11.5. The author's own F-D6 lists six places where the SDD was open and the oracle pins the author's choice, so "mutation 0 missed" on `policy.rs` shows the tests pin the code, not that the code is right.
- Impact: the policy table has had no independent check. SE confirmation of F-D6 (a) to (e) is still outstanding (author says so).
- Action: derive a small table from the SDD text by someone else (or from the SDD matrix itself) and compare; record SE confirmation of F-D6.
- Status: OPEN

### DF-09 [Minor][CONFIRMED by author's own run] ECO of a game depends on move order, contradicting the SDD wording (author F-D9)
- Location: `src/eco.rs:23-32`, SDD 3.11.9 and UT-DOM-070.
- Observation: author reports (and the test shows) 1.d4 Nf6 2.c4 e6 classifies A50 while 1.c4 e6 2.d4 Nf6 classifies A13 for the same final position, because the deepest named position is taken along the path. The code is consistent with "deepest named position along the path"; the SDD sentence "same position therefore same entry" is wrong. I did not re-run that test; I read the logic only `[OBS]`.
- Action: SDD wording fix (SE/SA); no code change. Make sure the import path (`classify_import`) and the live game are documented to use the same path rule.
- Status: OPEN (document)

### DF-10 [Minor][SUSPECTED] Class bounds can leave Beginner and Novice computers with no `UCI_Elo`
- Location: `src/roster.rs:507-508` (`uci_elo = (usable && in_range).then_some(elo)`).
- Observation `[OBS]`: if the engine's `UciEloRange` minimum is above the class baseline (400 or 600), `uci_elo` is `None` for the whole class. Whether the real bundled engine has such a floor is `[UNV]` (the crate is passed the range as a value; CLAUDE.md says no floor is assumed). If it does, the two weakest classes rely on skill level and error rate alone.
- Action: `chess-app`/calibration owner to confirm with the real range; add a golden test using that range.
- Status: OPEN

### DF-11 [Info] Roster identity and determinism checked independently
- `ROSTER_NAMESPACE` `[RAN python3 uuid.uuid5(NAMESPACE_DNS, "chess-roster.v1")]` = `aa98d4a0-fa10-585f-a6a6-268551d362c9`, equal to the constant at `src/roster.rs:211`. `roster_uuid(1, 0)` equals Python `uuid5(ns, "chess-roster/v1/1/0")` = `e150175c-fc0c-5fb5-8ee4-cab604e95634`, which is row 0 of the golden file. SplitMix64 outputs for seeds 0 and 1234567 recomputed in Python match the algorithm; seed 0 first outputs (`e220a8397b1dcdaf`, `6e789e6aa1b965f4`) match the file. So the vector file's oracle is independent of the Rust code.
- The roster golden file (`tests/data/roster_seed1_golden.txt`) is, by its own header, a characterisation of the first implementation; only the UUID column has an independent oracle. It pins determinism, not correctness. Platform independence: only `u64`/`u128` integer arithmetic, no `HashSet` iteration order in output, no floats, no `usize`-width dependence `[OBS]`; not run on another platform or endianness `[UNV]`. The namespace value is an engineer's choice (F-B2) and must be confirmed by the owner before release because it is frozen.

### DF-12 [Info] `Rng::below` comment overstates
- Location: `src/edge.rs:25`. Multiply-shift without rejection has a bias of at most n/2^64. "No modulo bias to reason about" is true in practice for n below about 2^32. No action beyond wording.

### DF-13 [Info] Unicode board glyph width is a terminal assumption
- Location: `src/board_grid.rs:215-234`. U+2654 to U+265F are East Asian Width "N" `[RAN python3 unicodedata]` (one column), but U+265F is emoji-capable and some terminals draw it two columns wide, which breaks the 3-column grid. ANSI output is correct by construction (ESC only in `Ansi`). Not rendered on any terminal `[UNV]`.

### DF-14 [Info] Elo numeric edge cases are safe
- `[RAN]`: `settle` with ratings `i32::MAX` and `i32::MIN`, `u32::MAX` rated games: no panic; a win at `i32::MAX` returns `delta 0`, a draw-edge case near `i32::MAX` returns `Err(RatingOverflow)` (checked add). Over 1 000+ rating pairs x 3 results at equal K, `white.delta + black.delta == 0` in every case (symmetry holds, rounding half away from zero). K switches at 30 per ELO-1 and the bands match ELO-1 (`src/elo.rs:70-77, 147-158`).

### DF-15 [Info] Author-raised items not otherwise findings
F-D1 to F-D14 and F-B1 to F-B14: F-D8 is DF-01, F-D6 is DF-08, F-D9 is DF-09, F-B2 is DF-11, F-B6 (`MAX_ROSTER_INDEX` 5 000, `MAX_NAME_SUFFIX` 100 000) is an invented bound but is stated `ESTIMATE`, documented, and ends in a typed error; accepted as Info. The rest are scope and wording notes I did not need to contest. `FakeTimeSource` wall is stored as `u64` and cannot express negative wall times (test helper only).

## 4. Per-criterion check

| Criterion | Result |
|---|---|
| Game state machine, every SDD 3.10 transition incl. T10, T16, T18 | Read all `decide` and `apply_fact` arms against the table `[OBS]`. Sweep shows `decide` and `evolve` never disagree (no `InvalidTransition`). T18 from `Completed`: `evolve` accepts (`game.rs:696`), `decide` refuses via `deny_finished` (`game.rs:798-804`), as the SDD says. `Voided` after `Aborted` is `WrongState` (SDD: a replay error). T10 draw agreed, T16 flag, resign, ResolveTerminal all emit `Ended` and stop the clock |
| Illegal sequences | Random sweep with illegal moves, stale plies, wrong actor and wrong state: all give typed `Err`, no panic `[RAN]` |
| Terminal re-check D-65 | `pending_termination` is `None` with no plies (start position terminal is refused at setup). Replay reports `terminal` and `interrupted` as the SDD says. See DF-05 for trusting `Ended` |
| Replay vs live | Equal under `journal_eq` over 45 000 commands. Not equal for finished-game clocks (DF-01) |
| DrawDropped, Suspend, Resume | `decide(Suspend)` emits `DrawDropped` first when the offer is to a computer; target state derived from the state found; `SuspendResume` restarts the clock only for `AwaitingMove`. Covered by the sweep |
| Voided after Ended | Replay only, as ruled (D-87). A `Clock` after `Ended` is refused (DF-05) |
| Clocks and TimeTotals | All clock arithmetic saturates or uses `saturating_sub`; a backwards stamp is zero elapsed `[OBS clock.rs:180-201]`. Move after expiry gives `Ended TimeForfeit` and no move `[RAN]`. `seed_from_facts` saturates. `restore` and `apply_move` accept any `u64` remaining without comparing with the spec (journal trust, same class as DF-05). Overflow in `TimeTotals`: none (all `saturating_add`) |
| Elo | DF-14 |
| Mode policy, oracle independence | DF-08 |
| Roster determinism, SplitMix64, namespace, golden file | DF-11 |
| `first_k_after` | `src/roster.rs:433-441`: index `5k+c`, order and duplicates irrelevant, saturating `+1`; author tests include overflow. No defect found |
| BoardGrid | Width 3 per square, labels on four sides, ESC only for `Ansi`, orientation reversal correct by reading (rank 1 at top for Black) `[OBS]`; glyph width DF-13 |
| ECO normalisation, `classify_import`, `resolve` | `^[A-E][0-9]{2}$` after ASCII trim and upper-case; fullwidth digits and trailing NUL rejected by construction. All five outcomes and the unanswered mismatch are total `[OBS]`. No defect |
| Stats counting predicate | `counts_in_aggregates` = Completed, not self-play, not custom start, has a result; the reset window drops `started_at < reset_at` and rating series keep `at >= reset_at` (consistent) `[OBS]`. DF-02 for the one overflow |
| Serde representations | DF-06. `deny_unknown_fields` present on `GameSetup`, `Seat`, `RawPersonality`; `Fact` and `Outcome` are not serde types (the SDD says they are received in protocol commands: a gap to be closed when the app wires the protocol, noted not counted) |
| Pure-crate rule | Section 1: met |
| Public API leaking rules/shakmaty types | Rules types named only for `Side`, `Move`, `Position`, `PositionKey`, `Fen`; no shakmaty |
| Suppressions | Section 1: three, all reasoned |
| Panics (unwrap/expect/indexing/arith) | Production code uses `get`, `try_from` with fallbacks and `saturating_*`; clippy `-D warnings` is green with the workspace panic lints. The one arithmetic panic found is DF-02. `unwrap_or` fallbacks (for example `pick` returns `"Player"`) hide impossible states rather than surface them; acceptable, noted |

## 5. Coverage, mutation and property evidence check

| Author claim | Re-run | Result |
|---|---|---|
| Lines 97.82 % (part A) | `cargo llvm-cov` | 98.09 % for the whole crate (part B evidence figure); matches. Per file: `error.rs` 90.32 %, `types.rs` 93.28 % (function), `variation.rs` 97.16 %, `clock.rs` 96.30 %, `policy.rs` 95.91 %. Includes in-`src` test code, so production-only is lower (author estimates 97.4 %) |
| Mutation 0 missed on `elo`, `policy`, `clock`, `totals`, `game` | Not re-run in full (about 400 mutants; time). Sample re-run on `stats`, `variation`, `eco`, `board_grid`, `totals` below | see below |
| Property tests | Author's `tests/replay.rs` proptest (23 tests, 0.62 s) passes in `ci/check.sh`. My independent sweep (above) found no disagreement. The author's property tests share `journal_eq` and `check_invariants` as oracles, and `journal_eq` omits finished-game clocks (DF-01) | Coverage of live-versus-replay is real but one-sided |
| Fuzz | `cargo-fuzz` is not admitted (D-76); FZ-DOM-001 is a `proptest` random-input test by SDD. No fuzzing evidence beyond proptest and my sweep | As designed |

Mutation sample result: the sample run of 159 mutants was **stopped before completion** (slow); at stop       31 caught,        2 unviable,        0 missed,        0 timeouts of the mutants tested so far (the files run in order: stats, variation, eco, board_grid, totals, so only the first part of that order was reached). This is partial evidence only. It neither confirms nor contradicts the author's 0-missed claims for the files not reached [RAN, partial].

## 6. Verdict

**PASS WITH CONDITIONS** (desk review; not a gate approval).

Conditions before this crate is relied on by `chess-app` and `chess-store`:
1. DF-01: a recorded decision (journal contract or SDD statement) on final clocks, and `journal_eq` honest about it. Owner: Systems Engineer / System Architect, then the author.
2. DF-05: a ruling on `Clock` after `Ended` and on trusting `Ended` outcomes.
3. DF-08: SE confirmation of the F-D6 policy choices, and an independent table for the policy oracle.
4. DF-11: owner confirmation of `ROSTER_NAMESPACE` before first release (frozen value).
5. Minor DF-02, DF-03, DF-06, DF-07 fixed or deferred with a recorded id.

No finding was dispositioned by this review. Dispositions belong to the named owners.

## 7. Not reviewed, limits, assumptions

- Not run: full mutation re-run of `game.rs`, `policy.rs`, `clock.rs`, `elo.rs`, `roster.rs`, `types.rs`; Intel or any non-arm64 platform; big-endian or 32-bit; any real terminal rendering; the real engine `UCI_Elo` range; serde at JSON level (no `serde_json` in scope; two serde claims are `[INF]`).
- Not analysed: `chess-rules` behavior behind `Position::play`, `is_dead_by_material`, `can_side_checkmate`, `repetition_key`, ECO table contents (sibling review).
- Not checked: MC/DC (not measured by the author, none claimed); cargo-deny with network; `npm audit`.
- Assumptions: the author evidence files were read after forming findings; the SDD (baselined 1.0) is the intent, so "observed" behavior is not treated as "intended" where the SDD is silent.
- Mermaid: none in this document.
