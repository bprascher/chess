# Desk review (G1): chess-rules and chess-domain, step 1 code

| Field | Value |
|---|---|
| Document | docs/engineering/desk-review-G1-step1.md |
| Task | chess-a0r.2.9 (baseline B4) |
| Date | 2026-11-09 (checks run 2026-11-06 to 2026-11-09 on the final working tree; all other Phase 2 tasks closed) |
| Stance | **System Architect, Architect/Reviewer**, fresh invocation. I did not author the SDDs' code. Recorded as a **desk review (G1)** per SWE-001 section 3. It is **not** an approval by Brian Prascher |
| Subject | Working tree, uncommitted: `crates/chess-rules`, `crates/chess-domain`, `Cargo.toml`, `Cargo.lock`, `ci/check-crate-edges.mjs`, `deny.toml`, `ci/check.sh`, `ci/check-eco.mjs`, `third_party/eco` |
| Records read | GATE-102, GATE-103, step1-rules-{A,B,rework}-evidence, step1-domain-{A,B,rework}-evidence, review-chess-rules-code (PASS WITH CONDITIONS), review-chess-domain-code (PASS WITH CONDITIONS), ruling-test-helper-placement, dependency-lookup-step1 v0.3 |
| Design read | SDD-chess-rules 0.3.5 DRAFT (3.11.8, 3.11.9, 3.11.10, IFP-RUL-1..6), SDD-chess-domain 1.0 (3.11.5, 3.11.6, IFP, XC-DOM-A8-5), architecture 3.2, 3.3, 5.7, 7.6, 4.4, D-3, D-20, D-34, D-68, D-75, SWE-001 |
| Evidence tags | `[RAN]` executed by me, `[OBS]` read by me, `[INF]` inferred, `[UNV]` not verified |
| Gate record | GATE-104 is not needed; this review is recorded against GATE-102 and GATE-103 (status lines appended) |

## 1 Verdicts

| Subject | Verdict | Meaning |
|---|---|---|
| `chess-rules` (GATE-102, parts A and B, ECO pass, rework) | **PASS WITH CONDITIONS** | G1 confirmed as to design conformance (section 5). Conditions C-R1 to C-R6 in section 6 |
| `chess-domain` (GATE-103, parts A and B, rework) | **PASS WITH CONDITIONS** | G1 confirmed as to design conformance. Conditions C-D1 to C-D6 in section 6 |
| Commit readiness of the working tree | **NOT READY** until C-R1 (a git-ignored source file) is fixed | See N-01 |

No G2 deviation was found in the code. Four items need Brian Prascher's decision or awareness (section 7). I approve none of them. Nothing here baselines `SDD-chess-rules`.

## 2 Tool output (edges and gates) [RAN]

All run from the repository root, offline, on the working tree.

**Edges, per crate** (`cargo tree -p <crate> -e normal,build --offline`):

```
chess-rules v0.0.0
├── shakmaty v0.30.2
│   ├── arrayvec v0.7.8
│   ├── bitflags v2.13.2
│   └── btoi v0.5.0 -> num-traits v0.2.19 [build: autocfg v1.5.1]
└── thiserror v2.0.21 (-> thiserror-impl, proc-macro2, quote, syn v3.0.9, unicode-ident)
[build-dependencies]
└── shakmaty v0.30.2 (*)

chess-domain v0.0.0
├── chess-rules v0.0.0 (as above)
├── serde v1.0.229 (serde_core, serde_derive)
├── thiserror v2.0.21
└── uuid v1.29.0 (serde_core, sha1_smol v1.0.1)
```

- `chess-rules` has **no workspace edge** (its `[build-dependencies] shakmaty` is the same crate and version as the runtime dependency, D-75). No `serde`, no `pgn-reader`, no I/O crate.
- `chess-domain` has exactly one workspace edge, `-> chess-rules`, which is in architecture 3.2. No `tokio`, `rusqlite`, `rand`, `getrandom`; `uuid` has default features off and only `v5` and `serde` on. `serde`, `uuid` match the third-party boxes of 3.2.

```
$ node ci/check-crate-edges.mjs
crate-edge check OK: 8 crates, 11 workspace edges, all allowed

$ cargo deny --offline check
advisories ok, bans ok, licenses ok, sources ok
(one warning: license-not-encountered for "Unlicense", deny.toml:23; also BSD-3-Clause per the rules review; pre-existing, harmless)

$ CHESS_CI_OFFLINE=1 CARGO_NET_OFFLINE=true ci/check.sh
== crate-edge check (architecture 3.2)  -> OK: 8 crates, 11 workspace edges, all allowed
== ECO snapshot checksums (CI-RUL-001)   -> eco snapshot check OK: 6 files match SHA256SUMS
== cargo deny check                      -> advisories ok, bans ok, licenses ok, sources ok
== npm audit: skipped (CHESS_CI_OFFLINE=1)
All gates passed.        (exit 0; fmt, build, clippy -D warnings, tests all green earlier in the same run)

$ cargo clippy -p chess-rules -p chess-domain --all-features --all-targets --offline -- -D warnings
Finished (no warnings). The test-support feature surface is also clean.

$ (cd third_party/eco && shasum -a 256 -c SHA256SUMS)
a.tsv b.tsv c.tsv d.tsv e.tsv COPYING.txt: OK
```

`Cargo.toml`, `deny.toml` and `ci/check-crate-edges.mjs` are **unchanged** against HEAD (`git diff --stat` empty) [RAN]. `Cargo.lock` adds only `sha1_smol 1.0.1` and `uuid 1.29.0` against HEAD [RAN]; both are named in dependency-lookup-step1 (F1, F8). `ci/check.sh` gains one step (`node ci/check-eco.mjs`).

Other checks I ran:

| Check | Result |
|---|---|
| Upstream types in public API | `grep -rl shakmaty src`: `lib.rs` (doc comment only), `fen.rs`, `position.rs`, `key_derive.rs`, `testing.rs`. No `pub use shakmaty` / `pub use chess_rules` anywhere in `crates/*/src` [RAN]. All `pub` signatures use crate types [OBS lib.rs of both crates] |
| No I/O in the runtime library | grep for `std::fs/net/env/process/time`, `SystemTime`, `Instant`, `getrandom`, `tokio`, `rusqlite` in both `src` trees: no hit. Only `build.rs` and `build/eco_gen.rs` use `std::fs` and `std::env` (build time, local files, no network) [RAN] |
| `unsafe` / suppressions | `#![forbid(unsafe_code)]` in both crates [OBS]. Three reasoned `allow` in `chess-domain/src`; none in `chess-rules/src`. `expect` only in `chess-rules/src/testing.rs`, which is behind `cfg(any(test, feature = "test-support"))` [OBS] |
| Provided interfaces IFP-RUL-1..6 | Every operation named in the SDD exists with the SDD's name: `standard`, `legal_moves`, `is_legal`, `play`, `san`, `turn/piece_at/fullmove_number/halfmove_clock`, `to_uci/from_uci`, `to_fen`, `is_check/is_checkmate/is_stalemate`, `repetition_key` (`Eq + Hash + Clone`), `can_side_checkmate`, `is_dead_by_material`, `parse_move`, `explain_illegal`, `Fen::parse/as_str/Display`, `write_pgn`, `write_pgn_many`, `PgnReader::new`, `format_line`, `parse_line`, `eco_tag`, `canonical_form`, `eco::lookup`, `eco::TABLE_LEN` [OBS] |
| Curated board list (architecture 3.2 rule 3) | `chess-domain` exports `BoardGrid, BoardStyle, BoardView, Cell, Shade, Highlight, Orientation` (all seven names the list needs) [OBS lib.rs] |
| **Git hygiene** | **Finding N-01 and N-02 below** [RAN] |

## 3 Findings of this review (new)

| ID | Sev | Finding | Evidence |
|---|---|---|---|
| **N-01** | **Major for commit, none for design** | `crates/chess-rules/build/eco_gen.rs` is **ignored by `.gitignore:7` (`build/`)**. It does not appear in `git status`, `git ls-files -o --exclude-standard` lists 0 entries for it, and `build.rs` declares `mod eco_gen` from it. A `git add -A` and commit would leave it out, and a clean checkout would fail to build `chess-rules` and everything above it. Local builds and `ci/check.sh` pass only because the ignored file is on disk | `git check-ignore -v` -> `.gitignore:7:build/  crates/chess-rules/build/eco_gen.rs` [RAN] |
| N-02 | Minor | `mutants.out/` (9.3 MB) and `mutants.out.old/` (3.8 MB) are untracked and not ignored: 548 files would be added | `git ls-files -o --exclude-standard \| grep -c mutants` -> 548 [RAN] |
| N-03 | Info | Two records disagree on the gate class of GATE-103: the Reviewer wrote "reviewed as the author's GATE-103 tier (G2)" (review-chess-domain-code section 2); GATE-103 says G1. I rule **G1** (section 5). The Reviewer's own statement that no edge, component or interface outside the SDD was added supports G1 | [OBS] |
| N-04 | Minor | **The `ply` base is not pinned across crates.** `Fact::Move.ply` is the zero-based index of the move (number of plies already played; equals `expected_ply`, F-D3). `SDD-chess-store` declares `MOVE.ply INTEGER CHECK (ply >= 1)` (one-based). Architecture 7.6 and SDD-app give the journal record `Move{ply,...}` no base. The conversion must happen in `JournalCodec` or the store applier, and no document says where. Self-consistent round trips (FF-3) pass for either choice, so a test would not catch a mismatch. This is a cross-crate, stored-format (journal v1) convention: **G2/G3 for Brian**, nothing is released so the cost is text only | [OBS store SDD line 326; domain Fact; arch 7.6 table] |
| N-05 | Info | `chess-domain` exports `pub use uuid::Uuid` and public names that the SDD does not list (`MAX_ROSTER_INDEX`, `CLASS_BOUNDS`, `FAMILY_NAMES`, `GIVEN_NAMES`, `Span`, `RatingTrack`, `UnratedReason`; ASM-DB1 owns them). Additive, no upstream rules type, no edge. The domain SDD (3.8) should list them when next amended | [OBS] |
| N-06 | Info | CLAUDE.md says "None of their Code: paths is real yet". Now stale for rules and domain. Update at baselining | [OBS] |

## 4 Rulings on the open items

Gate class is mine for the item as classified: G0 no approver, G1 SA desk review (this document), G2/G3 Brian. "Fix type" says what kind of work follows. Bead IDs are those in the work order; I could not read their titles (the `.beads/issues.jsonl` export holds only chess-2bb), so I map them by content and the caller should confirm the mapping.

### 4.1 chess-rules

| # | Item (bead) | Ruling | Class | Fix type |
|---|---|---|---|---|
| R1 | F-A2, F-B1, F-B2: error and type shapes (`IllegalMove{reason}` struct, `SquareError`, `FenError`, `MoveParseError` with payloads and no `Copy`, `San`, `PgnTags`, `PgnNode`, `PgnDate`, `TimeControl`, `Termination`, `PgnLimits`, `PgnErrorKind`, `PgnParseError`, `PgnWriteError`) (chess-4c7, chess-lat) | **Confirmed.** They are inside the SDD's freedom (its signatures are sketches), nothing consumes them yet, own types only, `Send + Sync` asserted. Weakness RF-07 (free-text `detail` carries ten `BadSyntax` reasons; dead upstream-error arms in `fen.rs`) is real but not blocking: schedule it **before `chess-app` maps these to wire kinds (D-51)** | G1 | SDD text amendment (draft A-R1). Code: G1 follow-up (software-engineer): promote repeated reasons to variants, delete the dead arms |
| R2 | F-B4, F-B19: in-crate tokenizer replaces the `pgn-reader` wrapper (chess-lat) | **Accepted as G1.** It is the SDD's own named route (3.11.8 rationale, OQ-RUL-001, DR-RUL-001 "D-3's escape route"), it adds no dependency, removes a pre-1.0 GPL crate from the tree, changes no edge, no public type, no 5.7 contract (the 5.7 row says "wrapping `pgn-reader`/`shakmaty`" as description; the interface is pure functions, streaming, per-game errors, all unchanged). The cost is that the PGN grammar is now this crate's code; it is covered (63 PGN tests, 97 % lines, 300 000 mutated-input runs in the review). **Flag for Brian:** D-3 is an AGREED decision that names `pgn-reader`. I read the substitution as inside D-3's escape route, but if Brian reads D-3 as binding on the crate, this is G2. It surfaces naturally when he decides on baselining the rules SDD. Architecture text that still says `pgn-reader` (3.2 diagram box, 3.3 row, 5.7 row, 3.4 table) is stale and needs one addendum line | G1 (my ruling); Brian awareness | SDD text amendment (draft A-R2) plus an architecture addendum line (G0 text, architect, after Brian's baseline decision). No code change |
| R3 | Version constant `CANONICAL_FORM_VERSION = 1`, OQ-RUL-005 (a) (chess-pa1) | **Accepted.** Additive public item, answers the SDD question ("G1 at the code step"). It is the same `v1` that SF-03 stores | G1 | SDD text (A-R3). No code change |
| R4 | Golden bytes of `canonical_form` v1, OQ-RUL-005 (b) (chess-bvv) | I checked the expected strings of `ut_rul_026_d_42_canonical_form_golden_bytes_of_v1` against SDD 3.11.10 steps 1, 4, 5 by hand: Seven Tag Roster order, `Round "-"`, `Date "????.??.??"`, escaping of `"` and `\`, `TimeControl "300+3"` after `Result`, blank line, joiner of one space, no wrapping, last token = result, one final `\n` [OBS tests/pgn.rs:617-636]. They conform. **Gap:** no golden pins the `FEN`/`SetUp` bytes (they are `contains` checks) and `FEN` precedes `SetUp` by ASCII order (code does this [OBS]). Adding one golden with a custom start is a G0 test. **Freezing `v1` is not mine to approve:** it fixes the bytes under every future stored hash (SF-03). Brian to confirm the fixtures (decision B1). Test-only change is G0/G1; any later byte change is `v2` (G3) | G1 for the test; **G3 for the freeze** | G0 test task: add the custom-start golden. Brian: B1 |
| R5 | RF-05 (a) illegal mainline tail collides; (b) control characters in tag values become spaces (FIND-RW-001) (chess-77d) | (a) **Recommend fix**: for an illegal move, `canonical_form` appends a deterministic marker (for example the offending UCI) instead of stopping silently. This changes **no byte of any legal game**, so it is not a `v1` change for any real hash; no signature change. (b) **Recommend accept and document**: two games that differ only by a tab versus a space inside a tag value are treated as the same game. That is a benign duplicate false-positive in a local single-user app; escaping would change `v1` bytes for those inputs. Because `v1` is a stored format, both go to Brian with this recommendation (B1) rather than being decided here | G1 code for (a) if Brian agrees; (b) decision text; **G3 class because `v1` is stored** | Software-engineer G1 task for (a) after B1; SDD text for (b) |
| R6 | RF-01 depth ceiling, RF-02 phantom game, RF-03 merged tag sections, RF-06 `Other` reachable (chess-27v, chess-w9w, chess-76y, chess-ahp, by content) | **Fixed and accepted.** The rework tests (15 in `tests/rework.rs`) pass inside `ci/check.sh` [RAN]. Choices accepted as G1: ceiling 256 (`pub(crate)`, estimate; no requirement drives it, record as `ESTIMATE`); "a game with no tags and no moves is not a game"; "blank line then `[` ends the previous game with `BadSyntax`"; RF-06 maps to `CannotReach{piece}`. Two notes: (i) `CannotReach{Pawn}` for `e2e4q` reads imprecisely; a dedicated variant is better and is an additive enum change, G1, do it with R1's variant cleanup. (ii) the RF-03 rule is `[UNV]` against the PGN Standard's import grammar (blank lines inside a tag section); keep, and record in SDD | G1 | SDD text (A-R4). Optional G1 code with R1 |
| R7 | RF-04 silent information loss on import (comment before first move, unparsed `TimeControl` / `Date` / `Termination`, Latin-1 per game) (F-B6, F-B7) | **Not a design-conformance failure; it is a behaviour decision.** The SDD does not require preservation and IF-2 reports only imported/skipped/failed. Recommend: for step 1, accept and write it into SDD 3.11.8 and the import notes. Before `chess-app` builds the import path, add a **warnings list on the reader item** (additive, G1) and switch the Latin-1 rule to per-payload decoding (G1). The unparsed `TimeControl` also changes `canonical_form` input relative to a game that kept it; that is the same question as R5 (B1) | G1 (needs SE confirmation of IF-2 intent) | SDD text (A-R5); G1 code later, track as a follow-up |
| R8 | RF-08 `ParseMode::Pgn` is documented strict but lenient/strict in the wrong places; RF-09 960 FEN with `-` accepted; RF-14 `Bb4` | **Accept the behaviour, correct the text.** Legality is the safety property and holds (400 000-text differential run found no `Ok` outside `legal_moves()`). Fix the doc comment on `ParseMode::Pgn`, list the accepted spellings in the SDD, record that a non-standard back rank with castling `-` is accepted. Confirm `Bb4`/`B2d5` with the SE (CLI-3). `e.p.` after a capture failing the whole game is a real-file hazard: recommend accepting `e.p.` as an annotation (G1, small) | G1 | G0 doc comment; SDD text; optional G1 code for `e.p.` |
| R9 | RF-10 memory amplification (about 20x, 215 MB for a 10 MB game), RF-15 long comment lines | Record in SDD 3.12 and TBD-25 (`ESTIMATE` provenance: measured by the Reviewer). Consider a per-game ply cap in the app. RF-15 conforms to SDD 3.11.7 step 5 | G0 text | SDD text |
| R10 | RF-11 ECO integrity is CI-only; `cargo:warning` noise (F-B17) | **Accept** as the SDD says (3.11.9). Say in `SOURCE.md` that `SHA256SUMS` detects accident, not tampering. Noise: optional | G0 | Doc line |
| R11 | RF-12 gate record understates footprint | The rework note in GATE-102 already corrects it. **I rule the footprint G1:** (i) `ci/check.sh` +1 step and `ci/check-eco.mjs` implement CI-RUL-001, which the SDD requires; `ci/` is the enabling system (architecture 3.1), not a product crate, and no crate edge, interface or ADR changes. (ii) `third_party/eco/` is the one-time snapshot D-20 planned (CC0, commit `3ba6b20d…`, checksums, `SOURCE.md`, `COPYING.txt`; checksums re-verified here [RAN]); NFR-6 licence audit is the data licence, which `cargo deny` does not cover, so it is covered by `SOURCE.md` and the CI presence check. (iii) `[build-dependencies] shakmaty` is anticipated by D-75. (iv) the build script reads `../../third_party/eco`, outside the crate directory: fine while `publish = false` | G1 | None |
| R12 | F-A1 / F-D1 perft and fakes only reachable as `cfg(test)` (feature `test-support`, no self dev-dependency) | **Accept** (consistent with ruling-test-helper-placement). `clippy --all-features` is clean [RAN]. Consumer crates may enable the feature through dev-dependency edges that 3.2 allows | G1 | None |
| R13 | F-B8 / UT-RUL-031 (5): shared drift-guard table not built on the rules side; domain side keeps the table in `chess-domain/tests/data` and greps the rules test source with `include_str!("../../chess-rules/tests/pgn.rs")` | The SDD's proposal was one file owned by `chess-rules`. As built, a `chess-domain` test reads another crate's test source by relative path. It is test-only and creates no Cargo edge, but it is brittle (greps a line prefix). **Accept for now (G1); condition C-R4:** move the table to `chess-rules/tests/data`, have both crates read it, and drop the grep. Amend UT-RUL-031 (5) text to the as-built source | G1 | G0/G1 test task; SDD text |
| R14 | OQ-RUL-001 (upstream API check) and ASM-RUL-001 | Closed in practice by the code: `shakmaty` 0.30.2 offers what the SDD assumed; `[UNV]` for `pgn-reader` (never opened). Record | G0 | SDD text |
| R15 | F-A3 transitive crates of `shakmaty` not itemised (chess-pe1) | **Done**: dependency-lookup-step1 v0.3 itemises them; `cargo deny` passes with them [RAN]. Closed | G0 | None |
| R16 | F-A4 / RSK-RUL-002 (brute-force mate oracle for the material rule) not written | Open, accepted for step 1 (ASM-RUL-001 validated against the SDD table). Keep as an open risk with an ID | G1 | Test follow-up |

### 4.2 chess-domain

| # | Item (bead) | Ruling | Class | Fix type |
|---|---|---|---|---|
| D1 | F-D8 = DF-01 (chess-7ms = chess-kce): final clock of a finished game is not journaled; `journal_eq` skips the clock when finished, so the replay property is blind there | **Options.** (a) Emit a `Clock` fact before `Ended` in T10, T16 and `ResolveTerminal`. `Clock` is an existing record kind, so `journal_version` and the record table are not changed, but it adds a record and a sync to every ending step, changes the step-fact lists that the baselined app SDD (5a durable-prefix rule, JournalCodec FF-3) and the store apply path rely on. That touches three crates and a journal contract: **G2**. (b) Declare that a finished game's clocks are **not** journaled and no consumer may read them: `GAME.white_clock_ms`/`black_clock_ms` are the resume checkpoint (architecture 7.2 line "checkpoint for resume (RULE-5)"), the time totals come from `GameEnd` (D-63, SF-01), and no requirement reads a final remaining time. **I recommend (b)**; confidence Medium-High (I found no consumer; `[UNV]` I did not read every UI SDD for a "final clock" display). With (b): fix `journal_eq` so the omission is explicit and narrow, pin the behaviour with the reviewer's two reproductions (flag at 11 s -> live 0 vs replay 10 000 ms; resign at 201 s), and amend the SDD sentence "field by field" to "every journaled field" (author's F-D2 wording). If Brian wants final clocks durable, (a) is his G2 decision | (b): G1. (a): **G2, Brian** | (b): SDD text amendment (draft A-D1, G1 amendment of a baselined SDD with Decision Log line) plus G1 code/test (software-engineer) |
| D2 | DF-05 (chess-06b by content, per the work order's list): replay accepts `Ended` contradicting the position; refuses a late `Clock` (`WrongState`) | (i) **Trust of `Ended`: accept**; the journal has CRC and store reconciliation (7.6); the domain replays what was durably recorded. Record the trust decision in the SDD. (ii) **Late `Clock` after a finished state: ignore it (no-op), do not refuse.** Refusing turns a harmless record into `JournalUnreadable`, a worse failure than the bug it would reveal. `Clock` "never changes the state" (SDD) already; add "and is ignored after the end" | G1 | G1 code (small) + SDD text (A-D2) |
| D3 | DF-08, F-D6 (a) to (f) (chess-5vy by content): policy table choices and the oracle that transcribes the implementation | **Rulings, consistent with architecture 4.4:** (a) accepted: analysis exemption is exactly `PostGameAnalysis` and `WhatIf`; `LiveAnalysis` on a finished game is `game_over`. (b) accepted: `Suspended` is not InProgress (matches the 3.10 diagram). (c) accepted: `not_human_turn` before `state_not_allowed` (matches the order in 3.11.5). (d) accepted with a note: `SelectClock` appears in `allowed_actions` for any non-finished Strict game because the table says only "S only"; it is a setup-time action, so the UI SDD must not render a clock control from an in-progress snapshot; if SE prefers, restrict it to games with no ply. (e) accepted. (f) accepted and **required by architecture**: "an engine failure drops the offer (ENG-4)"; SDD T4 list is incomplete, fix the text. These are the architect's reading; the **Systems Engineer still owns confirmation that (a) to (e) match the requirement intent** (open). DF-08 itself (oracle mirrors code) stays open: add a small table derived from SDD 3.11.5 text by a different hand and compare | G1 | SDD text (A-D3); G0/G1 test task (independent table); SE confirmation |
| D4 | DF-09, F-D9 (chess-cdk by content): ECO of a game depends on move order | **Code is right, SDD wording is wrong.** D-20 says "the deepest named position reached gives the game's code", a path rule. Fix SDD 3.11.9 and UT-DOM-070 to require a named final position for the "same entry by transposition" claim, and say the import path (`classify_import`) uses the same path rule | G1 | SDD text (A-D4) and fixture wording; no code change |
| D5 | DF-11 `ROSTER_NAMESPACE` (chess-7n3): `aa98d4a0-fa10-585f-a6a6-268551d362c9` = `UUIDv5(DNS namespace, "chess-roster.v1")`, a one-way door (D-34, permanent after release) | I re-derived it with Python 3 `uuid.uuid5(NAMESPACE_DNS, "chess-roster.v1")` = `aa98d4a0-fa10-585f-a6a6-268551d362c9` and `roster_uuid(1, 0)` = `e150175c-fc0c-5fb5-8ee4-cab604e95634` [RAN]; both equal the constant and the golden row. The value is reproducible and carries no information. The SDD says the value "must be generated once and frozen". **Recommend accept.** The freeze after release is a decision Brian owns (B2). Pools, bounds and the default seed stay editable until release | **G3 (freeze), Brian** | Decision Log line when accepted; no code change |
| D6 | DF-02 (`Tally::games` overflow) | **Fixed**, test at `u32::MAX` passes [RAN inside ci/check]. Closed | G0 | None |
| D7 | DF-03 (a legal what-if tree nested deeper than 64 cannot be exported; error says `BadText`) | The cause is `chess_rules::DEFAULT_MAX_VARIATION_DEPTH` (64) in the writer; doc claims corrected. **Rule:** add `TreeError::TooDeep` and refuse in `play` any variation that would exceed that constant, so every tree that can exist can be exported. Additive enum, one crate, G1 | G1 | G1 code + SDD text (A-D5) |
| D8 | DF-04 arena grows under play/delete cycles | Accept as a known limit for step 1; record as an open risk (RSK) with an ID. Cap the total arena length, not only live nodes, in a later G1 task | G1 | Follow-up task |
| D9 | DF-06 non-canonical UUID input accepted; `GameSetup` serde shapes; missing `clock` / `start` keys default to untimed / Standard (FIND-DOM-R01, R02) | Accept the UUID leniency (the `uuid` crate parses, output is canonical); document it. The defaults are a receive-side hazard only for hand-written JSON; generated TypeScript types always send the keys. **Rule:** the protocol layer in `chess-app` must reject a missing `clock` or `start` key; carry this as a condition into the app code review. `Fact` and `Outcome` still have no serde derive (the SDD expects wire use); close when the app wires the protocol. `serde_json` is not admitted; the stand-in value type tests are accepted | G1 | Condition on app code; no change here |
| D10 | DF-07 same `PlayerId` on both seats with different kinds passes `check_setup` | **Reject it.** A Player is human or computer, never both. Add the check (existing `SetupField::Players` reason or one additive reason), test, and a sentence in SDD 3.11.6 | G1 | G1 code + SDD text (A-D6) |
| D11 | DF-10 class bounds can leave Beginner and Novice without `UCI_Elo` if the real engine range has a floor above 400 / 600 | Not decidable now (ASM-3, `PINNED_UCI_ELO_RANGE` unknown). Owner: calibration / `chess-app`. Add the golden once the range is known | G1 | Follow-up with the range |
| D12 | F-D1 fakes behind a feature; carries a 20-line copy in `tests/common` | Accept (same as R12) | G1 | None |
| D13 | F-D2 `journal_eq` normalises transient states; `Pause` accepted from `Paused` on replay | Accept; list both in SDD 3.11.1 (with D1) | G1 | SDD text (A-D1) |
| D14 | F-D3 `Fact::Move.ply` zero-based, `think_ms: Option<u64>` | Domain choice accepted (it equals `expected_ply`). **But see N-04:** the conversion to the one-based `MOVE.ply` and to the journal record is unspecified. That is Brian's B3 | domain: G1; cross-crate base: **G2/G3** | SDD text in domain, app and store once Brian decides |
| D15 | F-D4 `Void` carries the reset Player; `RespondDraw` has no actor | Accept. The app SDD must say the engine path and the human path are separate commands (G1 text for the app owner) | G1 | App SDD note |
| D16 | F-D5 no pending termination before the first move; F-D7 `Option` clock fields and `ClockSpec` text form; F-D11 transitions without a fact; F-D12; F-D13; F-D14 | Accept all. They follow the SDD's own 1:1 fact table (D-41); the alternative of new journal kinds is G3 and was rightly not taken | G1 | SDD text (A-D1) |
| D17 | F-D10 public API names `Side`, `Move`, `Position`, `PositionKey`, `Fen`; `pub use uuid::Uuid` | **Conforms** to 3.2 edge rule 2: `chess-domain` re-exports no `chess-rules` type [RAN grep]. The app's curated list must not add rules types without naming a consumer (FF-1). `Uuid` re-export: accept | G1 | None |
| D18 | DF-12 to DF-14 (info): `Rng::below` comment, glyph width, Elo edge cases | Accept; fix the `Rng::below` comment wording (G0) | G0 | Comment |
| D19 | `ci/check.sh` + `ci/check-eco.mjs` + `third_party/eco` (as they relate to domain: `EcoTable` reads `chess_rules::eco::lookup`) | See R11. The domain keeps no data and adds no I/O | G1 | None |
| D20 | **XC-DOM-A8-5** wording for chess-2bb | Resolved by step 1 code. Draft text in A-D7 below. Facts [OBS/RAN by the tests named]: the `Fen` type compares normalised six-field text and **keeps the en passant square as given**; `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1` and the same text with `-` are two `Fen` values with equal `repetition_key` (`ut_rul_027`); the replay `fen_after` check compares **positions** (`same_position`: repetition key plus both counters, `game.rs:299`), so a spelling difference alone is not a `FenMismatch`, while a usable en passant square versus `-` is (`ut_dom_118_..._fixture_b`); on the standard placement no en passant square parses (rejected as an illegal position), so the drift-guard en passant row has the expected result `parse_error` (neither `Standard` nor `Custom`) | G1 | SDD text (A-D7); chess-2bb condition can be marked satisfied once the text is entered |

## 5 What I confirm and what I withhold

**Confirmed (G1 desk review, design conformance only):**

1. `chess-rules` and `chess-domain` stay inside one component each, with the architecture 3.2 edges unchanged. Tool output in section 2: 11 workspace edges, all allowed; `chess-rules` has none; `chess-domain` has only `-> chess-rules`.
2. No §5.7 interface contract is changed. The provided operations of IFP-RUL-1..6 and the domain cards are present with the SDD names. Type and error shapes are choices inside the SDDs' freedom.
3. Dependency rules hold: no I/O, clock or random source in the runtime code of either crate; no `shakmaty` type reachable from a public signature; `chess-domain` re-exports no `chess-rules` type.
4. The ECO table, the checksum CI step and `third_party/eco` are the D-20 / D-75 design (G1).
5. GATE-102 and GATE-103 are correctly classified G1. This is the desk review those records said was still needed.
6. The rework (RF-01, 02, 03, 06; DF-02, 03 doc, 06 tests; DF-07 recorded) is applied and green in `ci/check.sh`.

**Withheld:**

1. No approval of any G2 or G3 item: B1 to B6 in section 7 are Brian's.
2. No confirmation that the tree can be committed: C-R1 (N-01) first.
3. No baselining of `SDD-chess-rules`. Readiness view in section 8 only.
4. No claim that the Systems Engineer has confirmed the policy interpretations F-D6 (a) to (e), or the IF-2 intent for lossy import (R7).
5. No claim of conformance to SEI CERT; applied by intent only (SWE-001).
6. This review does not replace the Reviewer's re-check of the rework (not done).

## 6 Conditions

| ID | Condition | Class | Before |
|---|---|---|---|
| C-R1 | Fix N-01: make `crates/chess-rules/build/eco_gen.rs` trackable (add a negation to `.gitignore`, or move the generator to a path that is not ignored) and confirm with a clean `git clone` or `git stash -u` style check that `cargo build` passes from tracked files only. Add `mutants.out*` to `.gitignore` or delete the directories (N-02) | G0 | the first commit |
| C-R2 | Enter the rules SDD amendment (drafts A-R1 to A-R5) and the as-built reconciliation | G1 text | Brian's baseline decision |
| C-R3 | B1 decided (canonical_form v1 freeze, RF-05 a and b); then the custom-start golden test and, if agreed, the RF-05 (a) marker | G3 decision, G0/G1 code | before the first stored `content_hash` |
| C-R4 | Single source for the drift-guard table (R13) | G0/G1 | before `chess-app` code |
| C-R5 | Variant cleanup of the error taxonomy (R1, R6 note) and the reader warnings list (R7) | G1 | before `chess-app` maps errors / builds import |
| C-R6 | Correct the `ParseMode::Pgn` doc comment (R8) | G0 | with the next rules change |
| C-D1 | Enter the domain SDD amendment (drafts A-D1 to A-D7) with a Decision Log line (D1 option b, D2, D3, D4, D7, D10, D20) | G1 text | `chess-app` code |
| C-D2 | D1 (b) code/test: `journal_eq` explicit about finished-game clocks; pin the two reproductions | G1 | with C-D1 |
| C-D3 | D2 code: ignore a late `Clock`; D7 `TooDeep`; D10 same-Player check | G1 | `chess-app` code |
| C-D4 | DF-08: independent policy table; SE confirmation of F-D6 (a) to (e) | G0/G1, SE | before the domain is relied on |
| C-D5 | B2 and B3 decided (`ROSTER_NAMESPACE`, `ply` base) | G3, G2/G3 | before the first release / before `JournalCodec` code |
| C-D6 | Carry D9 (reject missing `clock` / `start` at the protocol boundary) into the `chess-app` review | G1 | `chess-app` review |

## 7 What Brian Prascher must decide (nothing here is approved by this review)

| # | Decision | Class | Options | My recommendation (confidence) |
|---|---|---|---|---|
| B1 | **Freeze `canonical_form` v1**: accept `CANONICAL_FORM_VERSION = 1`, the two golden fixtures (plus a custom-start fixture to add), and rule on RF-05 (a) illegal-tail collision and (b) control-character collision | G3 (stored format) | (1) Accept golden as built; fix (a) with a marker; accept (b). (2) Also escape control characters (changes `v1` bytes only for such inputs, nothing released). (3) Defer the freeze past the first release (a later change is `v2`) | Option 1 (High for (a), Medium for (b)). Nothing is released, so this is the cheapest moment; decide before the first stored hash |
| B2 | **Confirm `ROSTER_NAMESPACE`** `aa98d4a0-fa10-585f-a6a6-268551d362c9` | G3 (permanent after release, D-34) | Accept as derived; or have a fresh random UUID generated once | Accept (High): reproducible, independent oracle re-run, no information in it |
| B3 | **Pin the `ply` base** across domain (`Fact::Move.ply`, zero-based), journal record `Move.ply` and `MOVE.ply` (one-based, `CHECK ply >= 1`) | G2/G3 (store, app, domain text; journal v1 field meaning) | (1) Journal stores the domain value verbatim; the store applier writes `MOVE.ply = journal ply + 1`. (2) The codec adds one when writing the journal. (3) Change the domain to one-based | Option 1 (Medium): it keeps `record_to_fact` trivial and `expected_ply` equal to the stale-guard `ply` already in the protocol (`MakeMove{ply}`, `StalePosition{live_ply}`); the +1 lives in one place, the store applier. I did not read the store applier text in full `[UNV]` |
| B4 | **Awareness / possible G2:** the in-crate PGN tokenizer replaces `pgn-reader` named by D-3. Also the decision whether to baseline `SDD-chess-rules` (CR-15 intent) | G1 by my ruling; G2 if you read D-3 as binding | Accept the substitution and record it in the baseline; or require a G2 approval record | Accept (Medium-High): it is the SDD's own escape route and removes a dependency |
| B5 | Only if you want it: **durable final clocks** (D1 option a, a `Clock` fact before `Ended`) | G2 | (a) add the fact; (b) document non-journaled final clocks | (b) (Medium-High) |
| B6 | **Import fidelity (RF-04):** what the importer must preserve or report (leading comment, unparsed typed tags, Latin-1) | G1, needs SE / your intent for IF-2 | Accept loss and record; or add a warnings list (and optionally preserve raw values) | Warnings list before the import UI is built (Medium) |

## 8 Readiness view: SDD-chess-rules 0.3.5 DRAFT (not a baseline)

My view only; baselining is Brian's decision (CR-15 intent). **READY-WITH-CONDITIONS.** The code is built, tested (rules: unit 8 + 4 ignored deep perft, eco 6, eco_build 4, fen 15, notation 29, pgn 63, rework 15, rules 37, others; line coverage 97.08 % [Reviewer, re-run by the rework author]) and conforms to the SDD's design. I would baseline it **after** a single reconciliation amendment (0.4 DRAFT, then 1.0) that carries: the as-built tokenizer (A-R2), type and error shapes (A-R1), the version constant (A-R3), reader rules and ceiling 256 (A-R4), the lossy-import statement (A-R5), the ECO build as built, the `test-support` placement, the UT-RUL-031 (5) source, `Code:` paths, and B1's answer. The G1 desk review that the SDD itself asks for (it says "not desk-reviewed") is this document for the code; the SDD text still needs its own G1 text review after the amendment.

## 9 Draft SDD amendment text (not applied; I edit no SDD)

**A-R1 (SDD-chess-rules, section 5 and IFP shapes).** "The error types are: `IllegalMove { reason: IllegalReason }`; `SquareError`; `FenError` = `TooLong | FieldCount | BadField { field, detail } | Illegal(IllegalPosition)`; `MoveParseError` = `BadUci | Empty | TooLong | Syntax | LowercasePiece { suggestion } | PromotionRequired { choices } | Ambiguous { candidates } | NoMatch { reason }` (not `Copy`); `PgnParseError { game_index, line, col, kind: PgnErrorKind }`; `PgnWriteError`. The variant names are the stable part (NFR-9); `detail` strings are for display only." (Follow with the variant cleanup of RF-07 when done.)

**A-R2 (SDD-chess-rules 1.2, 3.11.8 rationale, IFR-RUL-2, DR table, CI-RUL-001, OQ-RUL-001).** "OQ-RUL-001 is answered: `pgn-reader` 0.29.1 gives no line and column, so this crate contains its own PGN tokenizer and reader (`src/pgn/reader.rs`) behind the same interface (the D-3 escape route). `pgn-reader` is not a dependency. `shakmaty` remains the only upstream chess crate. The words 'wraps `pgn-reader`' in 1.2 and IFR-RUL-2 are replaced accordingly. The grammar and the recovery rules of 3.10 are this crate's code and test burden (UT-RUL-017 to 022, FZ-RUL-001, FZ-RUL-003)."

**A-R3 (3.11.10 Version rule, OQ-RUL-005).** "`pub const CANONICAL_FORM_VERSION: u32 = 1;` is exported next to `canonical_form`. Its value is the `v1` of the stored string `sha256:v1:<hex>`. A change to steps 1 to 5 or to the `Date` rule after release must change this constant and is a `v2` (stored-format change, G3). The golden-bytes test `ut_rul_026_d_42_canonical_form_golden_bytes_of_v1` pins the bytes of `v1` for two fixed games [and a custom-start game, once added]."

**A-R4 (3.11.8 and 3.12).** "The reader clamps `PgnLimits::max_variation_depth` to 256 (`ESTIMATE`, no requirement; it keeps `Drop`, `Clone`, `Debug` and `PartialEq` of the result tree within a small stack). A result word where a game would start and no tag section precedes it is not a game. A trailing Ctrl-Z (0x1A) followed only by whitespace is end of input. A blank line between two tag sections ends the first game with `BadSyntax`. `explain_illegal` returns `CannotReach` for a promotion piece on a move that does not promote; `Other` is reserved for 'the move is in fact legal' and defects."

**A-R5 (3.11.8).** "Import is lossy in these cases and says nothing about it: a comment before the first move of a sequence; a `Date` part out of range (becomes unknown); a `TimeControl` not of the form `seconds[+increment]`; a `Termination` outside the three written values; a game whose text contains a byte that is not UTF-8 is decoded as Latin-1 as a whole. [Replace with the warnings-list rule when B6 is decided.] A non-standard Chess960 start with castling `-` is accepted as an ordinary custom start."

**A-D1 (SDD-chess-domain 3.11.1 and Decision Log).** "The equality of a replayed `Game` with the live `Game` holds for every journaled field. The clocks of a finished game, `EngineThinking`, `EngineError`, a step in progress and `Finalized` are not journaled, so `Game::journal_eq` does not compare them. `Ended` carries no clocks; the time totals come from `GameEnd` (D-63) and no consumer reads the clock of a finished game. `Pause` is accepted on replay when the game is already `Paused` (`Step` has no fact). `Fact::Move.ply` is the number of plies already played; `think_ms` is optional; a `Move` without it adds 0 (3.11.13)."

**A-D2 (3.11.1).** "A `Clock` fact after a finished state is ignored. An `Ended` outcome is stored as recorded and is not re-derived from the position; the journal's CRC and the store reconciliation (architecture 7.6) are the integrity check."

**A-D3 (3.11.5).** Add to the table notes: "Analysis exemption from `game_over` and `voided` is `PostGameAnalysis` and `WhatIf` only. `Suspended` is not InProgress. `MakeMove` by a human reports `not_human_turn` before `state_not_allowed`. `SelectClock` is allowed in every non-finished Strict game (a setup-time control; front-ends must not draw it from an in-progress snapshot). `SetMoveDelay` is allowed in any non-finished c-c state. `EngineFailed` is accepted in `DrawOffered` when the offer is to a computer (ENG-4: an engine failure drops the offer); T4 is extended."

**A-D4 (3.11.9, UT-DOM-070).** Replace "reaches the same position and therefore the same entry" by: "A transposition reaches the same entry when the final position is itself named. The game's code is the deepest named position along the path (D-20), so two move orders to an unnamed position can give different codes. `classify_import` uses the same path rule. UT-DOM-070 uses the two Ruy Lopez move orders."

**A-D5 (TreeError, `VariationTree::play`).** "`play` returns `TreeError::TooDeep` when the new move would nest variations deeper than `chess_rules::DEFAULT_MAX_VARIATION_DEPTH` (64), so `to_pgn_text` succeeds for every tree that can be built."

**A-D6 (3.11.6 `check_setup`).** "A setup in which the same `PlayerId` occupies both seats with different kinds is refused (`SetupField::Players`)."

**A-D7 (XC-DOM-A8-5, status RESOLVED at step-1 code).** "The `chess-rules` `Fen` compares its normalised six-field text and keeps the en passant square as given, so `... e3 ...` and `... - ...` are different `Fen` values with equal `PositionKey`. `StartPosition::from_fen` compares the normalised text with the standard FEN; on the standard placement no en passant square parses, so the en passant row of the drift-guard table has the expected result `parse_error`. The replay `fen_after` check compares positions (repetition key and both counters): a spelling difference of an unusable en passant square is not a `FenMismatch`, and a usable en passant square against `-` is (UT-DOM-118 fixtures (a) and (b))."

## 10 Assumptions

| ID | Assumption | Validation |
|---|---|---|
| ASM-SA1 | Bead IDs in the work order map to findings by content as in section 4 (the tracker export lacks them) | PM confirms |
| ASM-SA2 | No requirement reads a finished game's remaining clock (D1) | SE / UI SDD owners |
| ASM-SA3 | The store applier, not the codec, is the natural place for the +1 in B3 | Store and app owners |
| ASM-SA4 | The Reviewer's tool runs (coverage, mutation sample, differential runs) are accurate as reported; I re-ran only the quality gates, edges, deny, clippy, ECO checksums, hash derivations and git hygiene checks | Reviewer re-check |

## 11 Not verified

- Intel Mac and any non-arm64 platform: not run. `npm audit`: skipped (`CHESS_CI_OFFLINE=1`).
- I did not re-run coverage, mutation testing, fuzz-like differential runs, perft depths, or the 10 MB import measurement; I rely on the Reviewers' and authors' `[RAN]` figures.
- The PGN Standard text and FIDE Laws were not re-read (RF-08, RF-15, F-B3 wording remain `[UNV]`).
- `pgn-reader` 0.29.1 was not opened (no line/column claim re-checked).
- I did not read the store applier text, the app `JournalCodec` code (does not exist yet), or every UI SDD (B3, D1 depend on them).
- Clean-checkout build was inferred from `git check-ignore` and `git ls-files -o`, not performed (it would modify the tree).
- Reviewer re-check of the rework: not done. No human approval, no SE confirmation and no Brian decision is claimed anywhere in this document.
