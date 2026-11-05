# READY-chess-rules: detailed-design readiness check for the `chess-rules` crate

| Field | Value |
|---|---|
| Document | docs/engineering/readiness-chess-rules.md |
| Task | chess-a0r.1.7 (baseline B1) |
| Date | 2026-11-04 |
| Stance | Reviewer (readiness assessor). Findings and questions only. No requirement, architecture or SDD file was edited. Nothing here is an approval |
| Target | Crate `chess-rules` (architecture step 1, section 13), detailed design and implementation |
| Gate view | In-crate implementation is expected to be **G1** (SWE-001 section 3; system-architect desk review), PENDING. See section 4 |
| Verdict | **READY-WITH-CONDITIONS** (no blocker; 4 Major, 8 Minor) |

Evidence tags: `[OBS file:line]` read this session, `[RAN]` executed, `[INF]` inferred, `[UNV]` not verified.

## 1 Upstream artifacts and revisions read

| Artifact | Revision as read | Used for |
|---|---|---|
| docs/design/SDD-chess-rules.md | 0.3.1 DRAFT, 2026-11-01 (git `7b55e21`) | Target of the check |
| docs/requirements.md | v1.0 Amendment 7 (CR-9 incl. CR-9.14); RULE-1, RULE-6/7/8, CLI-3, IF-1, IF-2, IF-3, DAT-3 pointers, TBD/TBC register | R1 to R12 |
| docs/architecture.md | v1.0 amendment 7, addenda A, B, C: s3.1, s3.2, s3.3, s3.4, s5.7, s5.5, s9, s13, s14, decisions D-3, D-11, D-20, D-38, D-42, D-45, D-47, D-51, D-52, D-59, D-60 | A1 to A12 |
| docs/test-strategy.md | 1.0 Amendment 6: test levels, Reference data, Quality gates | A11, R12 |
| docs/functional_decomp.md | F-2.1.3, F-2.2.1, F-2.2.2 (primary), F-2.2.4, F-4.6, F-6.10 to F-6.12 (supporting) | Allocation |
| docs/use_cases/ | Greps only (UC-5, UC-15, UC-22, UC-23 mention FEN or PGN) | Allocation |
| docs/engineering/SWE-001-engagement-profile.md | 1.1 | Profile, gates, standard |
| docs/design/SDD-chess-domain.md, SDD-v0.3-joint-review.md, SDD-v0.4-consistency-pass.md, SDD-consistency-review.md, SDD-architect-triage.md | grep and targeted reads only | Seam with `chess-domain`; rulings already made |

Not depended on, as instructed: `chess-store`, the Review Board pass (OQ-16).

## 2 Checklist results

### 2.1 Requirement readiness (owner: Systems Engineer)

| # | Result | Evidence |
|---|---|---|
| R1 Identified and allocated | **Pass** | RULE-1 `[OBS requirements.md:412]`; RULE-6/7/8 `[OBS requirements.md:488-510]`; CLI-3 `[OBS :934]`; IF-1/2/3 `[OBS :1051,1074,1089]`. Allocation: architecture s10.1 rows `[OBS architecture.md:1776-1783,1803,1842]`; functions F-2.1.3, F-2.2.1, F-2.2.2 allocated by name to `chess-rules` `[OBS functional_decomp.md:84,87,88]`. The F-2.2.4 vs s3.3 split is noted in FIND-009 |
| R2 Verifiable | **Pass** | Each requirement has a Verification block (perft to depth 4, FEN tables, golden PGN, fuzz); SDD s7 maps 30 UT, 3 FZ, 4 CT, 1 CI test to them `[OBS SDD-chess-rules.md:789-828]` |
| R3 Quantified | **Pass with note** | Quantities exist where they matter: perft depth 4, 90 % line coverage `[OBS test-strategy.md:85]`, lines under 80 characters, 10 MB PGN limit (EST, TBD-25). No numeric time budget for rules operations; only NFR-8 (100 ms, EST) for the typed-move round trip, measured by FF-6 `[OBS architecture.md:1748]`. Acceptable for ordinary criticality. Raised as FIND-012 (Minor) |
| R4 Singular and unambiguous | **Pass with note** | RULE-1 (c) defines "dead position by material" as "neither side can checkmate by any legal sequence" `[OBS requirements.md:420]`: unambiguous. IF-2 says duplicates have "identical tags and moves" `[OBS :1078]`, while D-42 compares a fixed tag subset. FIND-007 |
| R5 Complete for behavior | **Pass** | Normal, invalid and boundary input covered: illegal move "rejected with a reason" (RULE-1), malformed or illegal FEN (IF-3), per-game PGN failure with index and line (IF-2), Latin-1 or UTF-8 (IF-2). Startup and shutdown: n.a. (pure library) |
| R6 Interfaces specified | **Pass** | IF-1 gives tag set, order, escaping, spacing, line length, result token `[OBS requirements.md:1056-1064]`; IF-2 and IF-3 give accepted content and error content. The typed-move error shape is fixed by D-51/D-52 `[OBS architecture.md:795-806]` |
| R7 Consistent | **Pass with notes** | No conflict found among RULE-1, IF-1 to IF-3, CLI-3. Two seams to confirm: IF-2 literal text vs D-42 (FIND-007); SE review status of the RULE-1 reading (FIND-008) |
| R8 TBD/TBR controlled | **Pass** | Register entries touching rules: see section 3. None unresolved blocks design; TBD-25 is the only open one and its deferral is acceptable |
| R9 Non-functional present | **Pass** | NFR-3 (no illegal state, fuzz), NFR-6 (licence of `shakmaty`, `pgn-reader`, ECO table), NFR-9 (messages), PLAT-1 (deterministic bytes), PLAT-3 (no network) `[OBS requirements.md:1027-1046]`. Security is best practice plus CVE avoidance only (SWE-001 row Criticality) |
| R10 Traceable | **Pass** | Requirements carry Rationale/Source and Decision Log entries (CR-n.m). SDD traces `RULE/IF/CLI` to `DD-RUL` directly (no SWR layer, by SWE-001 "Upstream artifacts") `[OBS SDD-chess-rules.md:16,834-855]` |
| R11 Criticality assigned | **Pass** | Ordinary, not safety, not regulated, not security-critical; CERT-informed practice, no conformance claim `[OBS SWE-001:18,19]` |
| R12 Verification environment | **Pass with note** | Tools named and versioned in architecture s3.4 (`proptest`, `insta`, `cargo-fuzz`, `cargo-mutants`, `cargo-deny`) `[OBS architecture.md:446]`. The perft reference counts are named by source (Chess Programming Wiki) but not stored in the repo, and the independent PGN reader for the IF-1 inspection is "chosen by the test owner". FIND-011 |

### 2.2 Architecture readiness (owner: System Architect)

| # | Result | Evidence |
|---|---|---|
| A1 Component boundary | **Pass with notes** | Responsibilities `[OBS architecture.md:412]`; what it must not do: pure, no `serde`, no other workspace crate `[OBS architecture.md:404,261]`, SDD s1.2 out-of-scope list. Notes: ECO ownership (FIND-001) and the termination wording (FIND-009) |
| A2 Interfaces | **Pass with note** | Provided: s5.7 "PGN/FEN I/O" row `[OBS architecture.md:993]` (export format, import rules, `Fen::as_str`, `parse_move`, `MatchedMove`, `Move::from_uci`, `canonical_form`); s5.5 error mapping and D-51 `[OBS :795-806]`. Required: `shakmaty`, `pgn-reader` (D-3). Note: `eco::lookup` (IFP-RUL-6) is not named in s5.7 or the s3.3 key types (FIND-001) |
| A3 Dependencies | **Pass** | s3.2 edges `domain -> rules`, `engine -> rules`, `app -> rules` (D-38, approved 2026-10-31); front-ends have no edge; `chess-rules` has no workspace dependency `[OBS architecture.md:398-406]`. No cycle. SDD s3.4 agrees |
| A4 Data ownership | **Pass (n.a. for storage)** | The crate persists nothing. Text forms are defined here, stored elsewhere; `content_hash` is computed in `chess-app`, input from `canonical_form` (D-42) `[OBS SDD-chess-rules.md:271-282]` |
| A5 Quality drivers | **Pass with note** | NFR-3, NFR-8/FF-6, PLAT-1, NFR-6 map to the crate. No rules-specific measurable scenario (FIND-012) |
| A6 Runtime behavior | **Pass** | Pure functions; outside the s4.8 contexts; caller guidance for blocking PGN work `[OBS SDD-chess-rules.md:768-781]`. Typed-move sequence in s5.1/D-45. Failure paths in SDD s3.9 |
| A7 Technology constraints | **Pass** | Rust 1.99.1, edition 2024, `shakmaty` 0.30.2, `pgn-reader` 0.29.1, `thiserror` 2.0.21, test tools: architecture s3.4 `[OBS architecture.md:426-446]`, authority: user rule FIND-006. Rust toolchain on the dev Mac is now 1.99.1 `[RAN cargo --version -> cargo 1.99.1; rustc 1.99.1]` |
| A8 Decisions recorded | **Pass with note** | D-3, D-11, D-38, D-42, D-45, D-51, D-52, D-59, D-60 are AGREED. The one-way door `canonical_form`/`content_hash` is D-42 (door "one-way at release", no release yet, ASM-6). Open: D-20 placement of the ECO table (FIND-001); OQ-17 dependency admission (FIND-002) |
| A9 Cross-cutting | **Pass** | Error handling: typed `thiserror` enums mapped in `chess-app` (s8, s5.5). Logging, config, authn: n.a. for a pure crate. Input bounds per SWE-001 s6 |
| A10 Resource budgets | **Pass with note (n.a.)** | None allocated to this crate; SDD values are `ESTIMATE`/`[INF]` (s3.12). Acceptable; measure with the perft benchmark |
| A11 Testability | **Pass** | Test-first core is step 1 of s13 `[OBS architecture.md:2198]`; s9 names unit, property, fuzz levels; `test-support` feature and perft through our API (DD-RUL-015) |
| A12 Consistency with requirements | **Pass** | Every allocated requirement in the s10.1 rows has a design entity (SDD s8 "Reverse check"); no orphan found. Seam with `chess-domain` agrees: it consumes `PositionKey`, `eco::lookup`, `PgnNode`, `Termination` `[OBS SDD-chess-domain.md:264,745]` |

## 3 Open TBD, TBC, OQ and ASM items that touch `chess-rules`

| Item | State `[OBS]` | Touches | Deferral acceptable? |
|---|---|---|---|
| TBD-24 custom PGN tags | CLOSED 2026-10-30 `[OBS requirements.md:1229]` | IF-1 writer, `canonical_form` | Closed |
| TBC-2, TBC-3, TBC-6 (FIDE Laws, PGN standard, PGN export rules) | CLOSED | RULE-1, IF-1, IF-2 | Closed. Re-check at release (checklist) |
| TBD-25 PGN import size | Open (design), 10 MB EST, PGN files only `[OBS requirements.md:1230]`; closes at step 2 (importer) | `PgnReader` takes the limit from the caller | **Yes.** The limit is a parameter; SDD FIND-RUL-008 notes it |
| TBD-34 custom clock limits | EST, whole seconds | `TimeControl` representation (`seconds[+increment]`) | Yes. Whole seconds is consistent with the writer |
| OQ-17 dependency lookups (version, licence, CVE) | Open, "closing step: before each crate is first used" `[OBS architecture.md s14 table, row OQ-17]` | `shakmaty`, `pgn-reader`, `thiserror`, test tools | **Not for the first compile.** FIND-002 |
| OQ-RUL-001 upstream API shape | Open; spike is "step 1, first task" `[OBS SDD-chess-rules.md:921]` | DD-RUL-002, 008, 009, 011 | Yes, as the first task, with the in-crate fallback of D-3. FIND-003 |
| OQ-RUL-002 ECO table size | Open, step 1 | DD-RUL-013 | Yes |
| ASM-RUL-001 `shakmaty` insufficient-material test equals s3.11.3 | Open | `can_side_checkmate` | Yes, validated by the fixture table; FIND-004 |
| ASM-RUL-005 TSV columns | Open, needs the snapshot | ECO generator | Yes until the snapshot is taken; FIND-010 |
| FIND-RUL-005 residual (`[UNV]` FEN en-passant convention vs standard text) | Check "when this crate is written" `[OBS SDD-chess-rules.md:549]` | `to_fen` | Yes; a one-line check at implementation |
| OQ-15 nested variation storage (deferred) | Deferred by the user | Store shape, not the reader/writer; the model keeps nested variations | Yes |
| OQ-16 (Review Board on D-41 to D-43), OQ-18, OQ-22 | Open | `chess-store` only | Not applicable, per instructions |

## 4 Allocation and gate view

**Allocation (SYS to component to interface).** RULE-1 -> `chess-rules` -> IFP-RUL-1/2; CLI-3, GUI-2 (c) -> IFP-RUL-3 (through `Query::ParseMove`); RULE-8/IF-3 -> IFP-RUL-4; RULE-6/7, IF-1/2, ANA-2 -> IFP-RUL-5; DAT-3 -> IFP-RUL-6 (see FIND-001). Functions: F-2.1.3, F-2.2.1, F-2.2.2 primary; the termination predicates support F-2.2.4 (allocated to the domain). No requirement without a design entity and no design entity without a driver `[OBS SDD-chess-rules.md:855]`.

**Expected gate.**

| GATE (to be numbered from GATE-101 by the implementer, ASM-S5) | Tier | Approver | Status |
|---|---|---|---|
| Implement the SDD inside `chess-rules`, with s3.2 edges and s5.7 contracts unchanged (Q9) | G1 | System-architect agent, desk review (SWE-001 s3) | **PENDING.** No GATE record exists yet (`docs/engineering/gates/` absent `[RAN ls docs/engineering]`) |
| If the SA rules in FIND-001 that the ECO table or lookup belongs outside `chess-rules` | G2 (changes a provided interface used by `chess-domain`, Q2) | Brian Prascher | Not triggered unless the SA so rules |
| Workspace skeleton and `third_party/eco/` snapshot | Separate tasks; classify there | | Not assessed here |

The SDD itself records the Reviewer pass as not done, the SA desk review as PENDING and the SE review of the RULE-1 reading as PENDING `[OBS SDD-chess-rules.md:947-950]`. Those are completion conditions for the implementation gate, not entry blockers: SWE-001/governance allow drafting and prototyping while a gate is PENDING.

## 5 Findings, addressed by owner

Severity: Blocker (design cannot proceed), Major (proceed only on a recorded assumption; owner resolves by the stated point), Minor. "Blocks code start?" means blocks starting the `chess-rules` code tasks.

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Owner | Severity | Blocks code start? |
|---|---|---|---|---|---|---|---|---|
| FIND-001 | A1, A2, A8, A12 | architecture D-20 `[OBS :1972]`, s3.1 `[OBS :289-292]`, s3.3 rows `[OBS :412-413]`, s9 data row `[OBS :1643]` vs SDD IFP-RUL-6, DD-RUL-013, SDD-chess-domain DD-DOM-015 | D-20 says "Bundled opening table in `chess-domain`" and the s3.1/s3.3/s9 text puts ECO classification and the table in `chess-domain`, but also says a `chess-rules` build step replays the lines and keys the table. The s3.3 `chess-rules` row and the s5.7 PGN/FEN row do not list an ECO lookup. The SDDs of rules and domain both place the generated table and `eco::lookup(&PositionKey)` in `chess-rules` and the "deepest named position" rule in `chess-domain` | A provided interface (`eco::lookup`, `PositionKey`) used across a crate edge is not in the architecture contract. If the SA reads D-20 differently (table lives in `chess-domain`), `build.rs`, `PositionKey::to_epd` and IFP-RUL-6 move | Does D-20 place the generated table and `lookup` in `chess-rules` as the two SDDs assume? If yes, please add it to s3.3/s5.7 so the contract matches; if no, state where it lives | System-architect | **Major** | Only the ECO module (DD-RUL-013). All other modules can start. Assumption until answered: ASM-RR-1 |
| FIND-002 | A7, A8 | architecture s3.4 dependency admission (AMD-8, D-47) and OQ-17 `[OBS :436, s14]` | Every dependency needs a registry lookup, licence check and CVE scan "before a version enters the table / is first used"; OQ-17 is Open, owner architect. For this crate: `shakmaty` 0.30.2, `pgn-reader` 0.29.1, `thiserror` 2.0.21, `proptest`, `insta`, `cargo-fuzz`. Both upstream crates are described as GPL-3.0 and compatible with GPL-3.0-or-later; this is `[UNV]` | Adding the dependencies to `Cargo.toml` without the lookup breaks SWE-001 s5 and the guardrail on dependencies | Have the lookups for the crates above been done and recorded? Who records the licence compatibility result (CI-RUL-001, NFR-6)? | System-architect (OQ-17 owner) | **Major** | Blocks adding the dependencies (first compile), not the skeleton or tests that need no crates |
| FIND-003 | A2, A11 | SDD OQ-RUL-001, s3.7 closing paragraph `[OBS :371,921]` | Behaviour of `shakmaty` 0.30.2 and `pgn-reader` 0.29.1 was never read: castling form, per-token position (line and column), insufficient-material test, FEN error set. Every upstream call name in the SDD is `[UNV]`. I did not verify them either | The SDD specifies behaviour, not calls, and has a fallback (in-crate tokenizer, own code behind the same API; D-3 escape). The risk is rework in DD-RUL-002/008/009/011, not an architecture change | Software engineer: run the OQ-RUL-001 spike first, record results, and update the SDD before coding the affected entities | Software-engineer | **Major** | No. It is the first code task; condition for DD-RUL-002/008/009/011 only |
| FIND-004 | R2, R4 | SDD s3.11.3 rules 1 to 4 vs its fixture table `[OBS :522-541]` | The reference classifier is labelled "to be validated". It does not cover every row of its own table: for example K+N v K+N (table: both can mate) is not derived by rule 2, whose second sentence only covers the knight owner's own material. Rule 2 also says a lone knight cannot mate when the opponent has "only a king and possibly queens"; I could not establish that from the text `[UNV]` and did not verify it | A wrong rule here ends games wrongly or awards a wrong flag-fall result (RULE-1 (c), RULE-4). The risk is already RSK-RUL-002 | Software engineer: complete the rule so every case of the table follows from it, and validate with the planned brute-force mate search on small material before relying on it (ASM-RUL-001) | Software-engineer | **Major** | No. Condition for DD-RUL-004 only |
| FIND-005 | R5, A2 | SDD IFP-RUL-1 and s3.11.1 step 8; architecture D-51 table `[OBS :805]` | `legal_moves` order is "unspecified", yet `Ambiguous.candidates` is passed on "in the order `chess-rules` returns them". The SDD does not say what that order is | The same text must appear from every front-end (PLAT-1, NFR-9). Identical for one build, but a test cannot assert it and an upstream bump may reorder | Define the candidate order (a stated sort), or state that it is unspecified and tests must not assert it | Software-engineer | Minor | No |
| FIND-006 | Seam / gate | SDD s12 `[OBS :946-950]` | No `GATE-101+` record yet; Reviewer pass not done; SA desk review not requested | G1 needs a recorded change proposal and the SA desk review before the work is called done | Record the GATE at the first code task, and request the desk review after the Reviewer pass | Software-engineer (record), System-architect (desk review) | Minor | No |
| FIND-007 | R4, R7 | requirements IF-2 `[OBS :1078]`, F-6.11 `[OBS functional_decomp.md:167]` vs D-42 / CR-9.2 / Q-SE-1 | IF-2 and F-6.11 define a duplicate as "identical tags and moves". The accepted reading compares only the Seven Tag Roster, `SetUp`, `FEN`, `TimeControl` and the SAN movetext. CR-9 says IF-2 is "not changed" `[OBS requirements.md Amendment 7 text, "Not changed" bullet, line 28]` | Tests UT-RUL-026 and CT-RUL-004 verify the interpretation, not the literal requirement. A later reader could take the words literally | Please make the interpretation visible in IF-2 (or confirm the CR-9.2 record is the controlling statement) | Systems-engineer | Minor | No |
| FIND-008 | R7 | SDD s12 row "Systems engineer review of RULE-1 reading ... PENDING" vs FIND-RUL-001 RESOLVED by Q-SE-4 | The SDD says the precedence was confirmed (Q-SE-4) but also lists the SE review of the RULE-1 reading (dead position by material, checkmate precedence) as PENDING | Unclear whether the dead-by-material reading in s3.11.3 (judged by material only, cooperative sequences) has SE sign-off | Is the reading in SDD s3.11.3 an accepted interpretation of RULE-1 (c)? | Systems-engineer | Minor | No |
| FIND-009 | A1, A12 | architecture s3.3 `chess-rules` row (AMD-10) `[OBS :412]` vs `chess-domain` row `[OBS :413]` and SDD DR-RUL-004 | The `chess-rules` row says termination detection "(mate, stalemate, dead position, repetition, fifty-move) is a query of this crate that the domain invokes". The SDD supplies only position-level predicates (`halfmove_clock`, `repetition_key`, `is_checkmate`, ...); history counting and precedence are in `Game` | Reading the row literally suggests a history-aware API in this crate. UR-20 already resolved the intent | Does the row mean position predicates plus domain history logic, as the SDD reads it? If so, reword the row | System-architect | Minor | No |
| FIND-010 | R12, A7 | architecture D-20, D-32; SDD s3.5, ASM-RUL-005; `third_party/` absent `[RAN ls third_party -> No such file]` | The ECO snapshot is not in the repo. Its commit and SHA-256 are to be recorded when taken "at the start of step 1". CC0 status and the TSV columns (`eco`, `name`, `pgn`) are `[UNV]` until the file is read. D-20 forbids fetching at build time, so a one-time manual fetch is needed | The ECO generator and licence record cannot be written until then | Software engineer takes and records the snapshot as a step-1 task (needs the Internet once) and checks columns and licence; SA/user confirm the licence record | Software-engineer | Minor | Only the ECO module |
| FIND-011 | R12 | test-strategy "Reference data" `[OBS :68-70]`; IF-1 verification | Perft node counts are cited from the Chess Programming Wiki but not stored; the independent PGN reader for the IF-1 inspection is unnamed | Fixture sources must be quoted when the tests are written, not recalled | Collect the counts from the cited source into a fixture file with the source and date; the test owner names the reader at test time | Software-engineer (fixtures), test owner (reader) | Minor | No |
| FIND-012 | R3, A5, A10 | NFR-8, FF-6, SDD s3.12 | No time or memory budget is allocated to rules operations; SDD figures are `[INF]` or `ESTIMATE` | Low risk for an ordinary-criticality library; the typed-move budget is covered by FF-6 | None required. If a budget is wanted, the SE/SA may set one after the first perft benchmark | Systems-engineer / System-architect (optional) | Minor | No |

Positive notes (not findings): the SWE-001 row that said the Rust toolchain must be updated before step 1 is now met: `cargo 1.99.1`, `rustc 1.99.1` `[RAN]`. The profile text is stale on that point (SW to refresh at the next profile edit). Node was not checked; the rules crate does not need it.

## 6 Assumptions needed to proceed

| ID | Assumption | Risk if wrong | Owner / when resolved |
|---|---|---|---|
| ASM-RR-1 | The ECO table and `eco::lookup` live in `chess-rules`, as both SDDs say (FIND-001) | `build.rs`, `PositionKey::to_epd`, IFP-RUL-6 move to `chess-domain`; G2 if a provided interface changes | SA answers before DD-RUL-013 starts |
| ASM-RR-2 | The three runtime crates and the test tools pass the OQ-17 lookups (FIND-002) | A crate is replaced or rejected; D-3 escape route applies to `shakmaty`/`pgn-reader` | SA, before the first `Cargo.toml` entry |
| ASM-RR-3 | The OQ-RUL-001 spike confirms what s3.11.1 to 3.11.8 assume, or the in-crate fallback is used (FIND-003) | Rework inside the crate only (G1) | SW, first code task |
| ASM-RR-4 | The s3.11.3 classifier is corrected and validated before `is_dead_by_material` is used by `Game` (FIND-004) | Wrong draw or flag-fall result | SW, with DD-RUL-004 |

None of these touches a one-way door or changes the gate tier, so none is a blocker. The one-way door in scope (`canonical_form` tag set, D-42) is decided and agreed.

## 7 Verdict

**READY-WITH-CONDITIONS.** Requirements R1 to R12 and architecture A1 to A12 pass, with notes. There is no blocker. Detailed design and implementation of `chess-rules` may start, with the conditions below; the parts they condition are provisional.

| # | Condition | Owner | Before |
|---|---|---|---|
| C-1 | Answer FIND-001 (where the ECO table and lookup live) | System-architect | DD-RUL-013 |
| C-2 | Complete and record the OQ-17 lookups for `shakmaty`, `pgn-reader`, `thiserror`, and test tools (FIND-002) | System-architect | First `Cargo.toml` entry |
| C-3 | Run the OQ-RUL-001 spike and update the SDD (FIND-003) | Software-engineer | DD-RUL-002, 008, 009, 011 |
| C-4 | Complete and validate the dead-by-material classifier (FIND-004) | Software-engineer | DD-RUL-004 |
| C-5 | Record the G1 GATE and plan the Reviewer pass and SA desk review (FIND-006) | Software-engineer, then System-architect | First code task; completion of the work |
| C-6 | Take the ECO snapshot and record commit, SHA-256, licence (FIND-010) | Software-engineer | DD-RUL-013 |

Minor items (FIND-005, 007, 008, 009, 011, 012) can be settled during design. Nothing here routes to the conditional remediation task as NOT-READY: there are no blockers.

## 8 Hand-back log

| FIND | Sent to | Sent | Answered | Reopened |
|---|---|---|---|---|
| FIND-001, 002, 009 | System-architect | Not sent by me; the program manager routes this report | | |
| FIND-007, 008, 012 | Systems-engineer | Not sent by me | | |
| FIND-003, 004, 005, 006, 010, 011 | Software-engineer (own SDD and tasks) | In this report | | |

No finding is marked resolved. I did not edit the owners' artifacts.

## 9 Not analyzed or not run

- No code exists; no build, test, coverage, mutation, scan or benchmark was run. The only commands run were `cargo --version`, `rustc --version`, `rustup toolchain list`, `ls` and `git status`/`git log`.
- The `shakmaty` and `pgn-reader` documentation and sources were not read, and no registry query, licence or CVE lookup was made. Upstream behaviour is `[UNV]`.
- The FIDE Laws, the PGN standard and the Lichess dataset were not re-read; the checks recorded as closed (TBC-2, TBC-3, TBC-6) are relied on as stated.
- The use cases were only grepped for rules-related content, not read end to end. The sibling SDDs, the joint reviews and the consistency passes were read in the regions that touch `chess-rules` only; the v0.4 pass itself says it did not read the SDDs end to end.
- Line numbers cited from `architecture.md` and `requirements.md` were read this session from the current files; the SDD's own line citations were not re-checked.
- Mermaid: this file has no diagram; no validator was needed.

## 10 Closure note for FIND-001 and ASM-RR-1 (append-only, 2026-11-04)

Added by the software engineer for task chess-a0r.1.8 (item L4 of architecture §11.4.10). The text above is unchanged; the readiness verdict and the findings were written by the readiness assessor, and this note only records an answer that arrived afterwards. It is not a re-assessment, and it does not mark any other finding resolved.

- **FIND-001 and condition C-1 (where the ECO table and `eco::lookup` live): answered by the System Architect's decision D-75** (architecture §11, §11.4.8 Addendum A; decided under Brian Prascher's delegation on 2026-11-04, tier G2, approved by that delegation; Brian may overrule). The generated table and `eco::lookup` stay in `chess-rules`; the "deepest named position" rule and everything about a game stay in `chess-domain`; D-20, §3.1, §3.3, §5.7 and §8.1 of the architecture were corrected to match. **Status of FIND-001: answered by D-75, owner's text updated by the architect.** The G2 row in section 5 "If the SA rules ... outside `chess-rules`" is therefore not triggered.
- **ASM-RR-1 ("The ECO table and `eco::lookup` live in `chess-rules`, as both SDDs say") is confirmed by D-75.** Its "impact if wrong" row does not apply.
- **A defect found while answering** (dependency lookup F7, `[OBS dependency-lookup-step1.md]`): a `build.rs` cannot call the crate it builds, so the generator needs `shakmaty` as a build-dependency of `chess-rules`. SDD-chess-rules 0.3.2 records this (3.4, IFR-RUL-4, 3.11.9). It adds no crate to the workspace and no edge to §3.2.
- **Not changed by this note:** the verdict and conditions other than C-1; FIND-002, FIND-009 and the other findings; the readiness of `chess-domain`.
- Evidence: `[OBS]` architecture.md D-75 and §11.4.8 read on 2026-11-04. No command was run for this note.

