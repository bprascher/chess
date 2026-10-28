# Chess Application — SRR/SFR Detailed-Design Readiness

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/SRR-SFR-readiness.md | **1.1 (run 2)** | 2026-10-28 | **Run 2 complete. No blocker. 4 findings closed, 3 opened, 12 open.** | Software engineer role (Claude), for Brian Prascher | Not an approval document. The owners named in §5 answer the open findings |

**Verdict in one line:** **READY-WITH-CONDITIONS.** Run 1 found one blocker (no criticality class or coding standard). The user answered it, and the answer is now on the page in the requirements Decision Log, the architecture, the test strategy and a new engagement profile. Detailed design of step 1 (`chess-rules`, `chess-domain`) can start, with the provisional parts listed in §6. Steps 2 and 3 carry further conditions that are not yet due. Two new findings need the user: how SEI CERT applies to Rust and TypeScript (FIND-014), and what happens to PLAT-2 now that Intel can't be tested (FIND-015).

**Reading guide.** §0 compares run 1 and run 2. §1 gives the target and inputs. §2 maps SRR and SFR exit criteria. §3 gives the checklist results. §4 covers the allocation and the design gate. §5 lists the findings by owner. §6 gives the verdict per step. §7 is the hand-back log. §8 lists what was not analysed.

---

## 0. Run history

| Run | Date | Trigger | Verdict | Blockers | Findings |
|---|---|---|---|---|---|
| 1 | 2026-10-28 | First check of the baselined documents | **NOT-READY** | 1 (FIND-001) | 13 raised: 1 Blocker, 8 Major, 4 Minor |
| 2 | 2026-10-28 | The user's answers to FIND-001, FIND-002, FIND-003 and FIND-009, recorded in the documents | **READY-WITH-CONDITIONS** | 0 | 4 closed. 3 new (FIND-014 to FIND-016). **12 open: 7 Major, 5 Minor** |

**What changed between the runs.** The answers are recorded in requirements Amendment 4 (CR-6, Decision Log rows CR-6.1 to CR-6.4), architecture amendment 4 (§8 and §10.2 rows, §11.2 row, Q-A18 answered), test-strategy Amendment 4 (quality gate), and the new [engineering/SWE-001-engagement-profile.md](engineering/SWE-001-engagement-profile.md). No requirement text, ID or verification changed, so the use cases and the functional decomposition (both v1.0) are unaffected. No code exists, so nothing was built or run.

**What the rerun did.** Every check of §3 was evaluated again against the changed documents. The coverage counts of §4.1 were re-derived by script instead of quoted (run 1 quoted the 104-of-104 leaf count from the functional decomposition). Link integrity of the changed documents was checked.

---

## 1. Target, inputs and stance

**Target.** The whole software in [architecture.md §3](architecture.md#3-code-organisation), assessed for readiness for detailed design (IEEE 1016 SDDs) in the build order of [architecture.md §13](architecture.md#13-next-steps):
- Step 1: `chess-rules`, `chess-domain` (pure crates).
- Step 2: `chess-store`, `chess-engine`, `chess-app`, `chess-cli`, `chess-server`.
- Step 3: `ui/`, `desktop/src-tauri`, `packaging/` and the enabling-system blocks of §2.1.

**Upstream artifacts**

| Artifact | Revision | Role in this check |
|---|---|---|
| [requirements.md](requirements.md) | v1.0, **Amendment 4** (CR-1 to CR-6), 2026-10-28; 82 requirement and risk IDs | SRR input (R-checks) |
| [architecture.md](architecture.md) | v1.0, **amendment 4**; decisions D-1 to D-37; 2026-10-28 | SFR and architecture input (A-checks) |
| [functional_decomp.md](functional_decomp.md) | v1.0, baselined 2026-10-28 (approved by the user); 124 functions, 104 leaves | SFR input (functional baseline) |
| [use_cases/README.md](use_cases/README.md), UC-1 to UC-29 | v1.0, baselined 2026-10-28 (approved by the user) | SFR input (behaviour threads) |
| [test-strategy.md](test-strategy.md) | 1.0, **Amendment 4** | Verification environment (R12, A11) |
| [engineering/SWE-001-engagement-profile.md](engineering/SWE-001-engagement-profile.md) | **1.0, new in run 2** | Criticality, coding standard, approvers, toolchain (R11, gate) |
| [handoff_se_to_architect.md](handoff_se_to_architect.md) | 0.2, closed | Background only |
| GUI board trade studies | as committed | Background for D-4 only |
| Quality attribute scenarios (QAS) | none as a separate artifact | See A5: the measurable NFRs and the §4.8 table stand in for them |
| Separate ADR files | none; decisions live in architecture §11 (table, §11.2 log, §11.3 door types) | Accepted as the decision record (A8) |

**Stance.** Reviewer only. This report lists findings and asks questions. It does not redesign anything. In run 2 the answers were written into the owners' documents at the user's request ("Update the docs with these answers"); no finding was marked closed on the strength of a promise, only on text now on the page.

**Baseline build.** No code exists, so there is no build or test baseline to run (`[RAN]` `git ls-files`: 40 tracked files, all documents; no `Cargo.toml`, no `package.json`). The development Mac has Rust 1.89.0 (`cargo`, `rustc`, `rustfmt`, `clippy`) and Node v26.12.1 installed (`[RAN]`, recorded in SWE-001). Nothing in this report claims that code compiles or passes.

---

## 2. SRR and SFR exit criteria mapped to this check

| Review | Exit criterion (what must hold before detailed design) | Checks used | Run 1 | Run 2 |
|---|---|---|---|---|
| SRR | Requirements are complete, verifiable, consistent and baselined, with TBDs controlled | R1–R10, R12 | Met, with 4 findings | **Met**, with findings FIND-004, FIND-010, FIND-011 and the Intel gap in R12 (FIND-015) |
| SRR | Criticality and the standards it implies are known | R11 | **Not met** | **Met**, with two residual questions (FIND-014, FIND-016) |
| SRR | Every requirement traces to a source, and every requirement is allocated | R10, seam | Met | **Met**: 82 of 82 allocated (re-derived, §4.1) |
| SFR | A functional baseline exists: function hierarchy, allocation to blocks, behaviour threads | Seam, A1, A6, A12 | Met | **Met**: 104 of 104 leaf functions allocated and cited by a use case (re-derived) |
| SFR | Interfaces between blocks are identified and defined at contract level | A2, R6 | Met for step 1 | **Met for step 1**, with conditions for later steps (FIND-005, FIND-006) |
| SFR | Open decisions and risks have owners and closing points | A8, R8 | Met, with 3 findings | **Met**, with findings FIND-005, FIND-007, FIND-008 |
| Both | Authority for approving design changes is defined | swe-governance gate | **Not met** as a document | **Met**: SWE-001 names the approvers (FIND-002 closed) |

---

## 3. Checklist results

Run 1 results are shown where run 2 differs.

### 3.1 Requirement readiness (owner: Systems Engineer)

| # | Check | Run 1 | Run 2 | Evidence (run 2) |
|---|---|---|---|---|
| R1 | Identified and allocated | Pass | **Pass** | 82 IDs. Every one is allocated in architecture §10.1: 80 have their own row and IF-2 and IF-3 share the IF-1/IF-2/IF-3 row (`[RAN]` script, 0 unallocated) |
| R2 | Verifiable | Pass | **Pass** | Each requirement has a Verification entry with a level (U, I, E, M) and a method in 29148 terms (requirements preamble). PLAT-2's Intel verification is conceivable but can't be performed (FIND-015, under R12) |
| R3 | Quantified | Pass, with ESTs | **Pass, with ESTs** | NFR-1 "≤ 3 s (CLI ≤ 1 s) … median of 10 runs"; NFR-8 "within 100 ms (EST)"; ENG-5 200 ms (TBD-12); ENG-4 timeouts (TBD-13). The EST values are accepted starting values (Q53), confirmed by measurement at step 2 |
| R4 | Singular and unambiguous | Pass, minor | **Pass, minor** | Compound requirements were split at v1.0. Leftover: PLY-4 "statistics (games, W/L/D, **etc.**)" (FIND-011) |
| R5 | Complete for behaviour | Pass | **Pass** | RULE-1 (b), (c), IF-3 illegal-FEN cases, DAT-7, ENG-4, PLAT-4 start-up errors and shutdown |
| R6 | Interfaces specified | Pass | **Pass** | IF-1 to IF-5. The internal protocol is specified in architecture §5 by design |
| R7 | Consistent | Pass, with a stability caveat | **Pass** | No conflicts found. The caveat of run 1 (interpretations open to the user's correction) is closed: requirements Decision Log CR-6.3 confirms them, and architecture Q-A18 is answered |
| R8 | TBD/TBR controlled | Pass, minor | **Pass, minor** | The register gives each open item an owner and a closing step. Steps stand in for dates because the project has no schedule. Status drift between documents: FIND-010 |
| R9 | Non-functional present | Pass | **Pass** | NFR-1 to NFR-10, UI-4, UI-6 to UI-8. Safety and regulation are now stated as not applicable (R11) |
| R10 | Traceable | Pass | **Pass** | Each requirement names its source (user's 15 top-level requirements, Q1 to Q79, CR-1 to CR-6, Decision Log). The new criticality and coding-standard decisions trace to CR-6.1 |
| R11 | Criticality assigned | **Fail** | **Pass, with residual questions** | Requirements Decision Log CR-6.1: ordinary criticality, not safety-related, not regulated; SEI CERT with the standard Rust and TypeScript checks and warnings as errors. Also in SWE-001 §1, architecture §8 and test-strategy "Quality gates". `[RAN]` search counts for "criticality": requirements 5, architecture 3, test-strategy 2, SWE-001 5 (run 1: 0 in requirements and architecture). Residual: how CERT applies to Rust and TypeScript (FIND-014); whether the web server's security counts as security-critical (FIND-016) |
| R12 | Verification environment | Pass, with a hardware question | **Partial** | test-strategy.md and architecture §9 specify levels, data, determinism and tools. The user's answer: no Intel Mac and none expected soon. PLAT-2's verification also names "an Intel Mac, an M1 Mac, and an M4 Mac"; the availability of an M1 and an M4 is not recorded. Steps 1 and 2 are unaffected. FIND-015 |

### 3.2 Architecture readiness (owner: System Architect)

| # | Check | Run 1 | Run 2 | Evidence (run 2) |
|---|---|---|---|---|
| A1 | Component boundary | Pass, minor | **Pass, minor** | §3.3, §4.8 "Must not" column, §2.2 allocation names. §3.1 says `chess-store` holds "export/import" while §3.3 says "formats are owned by `ImportExportService`" (FIND-012) |
| A2 | Interfaces | Pass for step 1 | **Pass for step 1, conditional later** | §5.1 to §5.7 protocol, error catalogue, idempotency, internal contracts; `Policy::check` in §4.4. The PGN export tag set depends on TBD-24 and TBC-6 (FIND-005) |
| A3 | Dependencies | Pass | **Pass** | §3.2 dependency graph, acyclic |
| A4 | Data ownership | Pass | **Pass** | §7.4 one writer per entity, transaction boundaries; §7.6 journal wins |
| A5 | Quality drivers | Pass (equivalent form) | **Pass (equivalent form)** | No QAS artefacts, but measurable drivers are mapped: NFR-1, NFR-2, NFR-8, ENG-3 to ENG-5, §4.8 table (ESTIMATEs until measured, ASM-2) |
| A6 | Runtime behaviour | Pass | **Pass** | §4.3 state machine with failure states, §5.3, §5.4, §4.8 concurrency, §7.3, §7.6 |
| A7 | Technology constraints | Pass, major gap on versions | **Pass, major gap on versions** | Languages and frameworks decided; no version pinned (FIND-006). New: the coding standard is recorded as a constraint with its authority, the user (architecture §8, CR-6.1) |
| A8 | Decisions recorded | Pass | **Pass** | §11 D-1 to D-37 with door types and revisit triggers. Q-A18 is now answered. Open: D-21 (DEFERRED, two-way), D-4 gated by its spike (OQ-9) |
| A9 | Cross-cutting | Pass | **Pass** | §8 now also has a "Coding standard and static checks" row, with a matching §10.2 entry for the missing requirement driver |
| A10 | Resource budgets | Partial | **Partial** | The NFR-8 100 ms budget is not split between flush, core and render; flush cost unmeasured (ASM-2); no memory ceiling for the engine pool (FIND-007) |
| A11 | Testability | Pass | **Pass** | Traits at the edges, `FakeEngine`, injected `TimeSource` and `Rng`, the gateway audit log as the Strict-mode spy (§4.5), the policy table as test input (§4.4), §9 |
| A12 | Consistency with requirements | Pass | **Pass** | §10.1 allocates all 82 IDs. §10.2 lists every element without a requirement driver, now including the coding standard (driver: the user's decision CR-6.1) |

---

## 4. Allocation and gate view

### 4.1 Allocation chain (re-derived by script in run 2)

| Link | Coverage | Evidence |
|---|---|---|
| Requirement → component (architecture §10.1) | 82 of 82 | `[RAN]` ID diff between requirements.md headings and NFR/RISK rows, and §10.1 rows: 0 unallocated |
| Requirement → function (functional_decomp) | 82 of 82 appear. 6 have no function of their own by design (UI-11, NFR-1, NFR-8, PLAT-2, RISK-1, RISK-2: attributes, limits or risks) | `[RAN]` 82 of 82 IDs found in functional_decomp.md; functional_decomp §7.1, FG-6 |
| Function → block (functional_decomp §3) | 104 of 104 leaves allocated. 7 leaves name more than one front-end block (FG-4, accepted) | functional_decomp §7.1 |
| Function → use case | 104 of 104 leaf rows cited by at least one use case. Every cited function ID exists; F-1.8 and F-2.1 are parent rows (bold in the table) | `[RAN]` script over functional_decomp §3 and UC-1 to UC-29 |
| Use case → requirement | 82 of 82 cited, none cited that doesn't exist | `[RAN]` script over UC-1 to UC-29 |
| Architecture element without a requirement | None unexplained. The new coding-standard row is listed in §10.2 with the user's decision as its driver | architecture §10.2 |

### 4.2 Step-1 allocation (the components to be designed first)

| Component | Allocated requirements (architecture §3.3, §10.1) | Interfaces it provides | Readiness |
|---|---|---|---|
| `chess-rules` | RULE-1, CLI-3, RULE-6, RULE-7, RULE-8, IF-1, IF-2, IF-3 | `Position`, `Move`, `San`, `Fen`, `PgnGame`; PGN/FEN I/O contract (§5.7) | Ready, except the PGN export tag set (FIND-005) |
| `chess-domain` | PLY-1, PLY-4, PLY-5, GAME-1, GAME-3, RULE-2, RULE-3, RULE-4, ELO-1, ELO-2, ENG-2, ANA-2, DAT-3 (ECO), UI-2, UI-3, UI-5, CLI-1, CLI-2 | `Game`, `GameSetup`, `Policy`, `Action`, `Clock`, `EloCalc`, `RosterGen`, `VariationTree`, `EcoTable`, `BoardGrid`, `BoardStyle` | Ready, except `EloCalc` (FIND-004) and `EcoTable` data (FIND-008) |

### 4.3 Gate classification of the planned work

The approvers are now recorded in SWE-001 §1 and §3 (user, 2026-10-28): the system-architect agent approves G1 as a desk review, and the user approves G2 and G3.

| Work | Gate answers (swe-governance §4) | Expected tier | Approver |
|---|---|---|---|
| SDDs for `chess-rules` and `chess-domain` as drawn in §3 | No new component, interface or dependency edge beyond §3.2 and §5.7. The crates are new code but are already drawn in the architecture | **G1 per component** | System-architect agent, desk review, recorded as such |
| Fixing the public API of `chess-rules` (types and signatures consumed by `chess-domain`, `chess-engine`, `ImportExportService`) | Defines an interface that other components use (Q2). It realises the §5.7 contract rather than changing it | G1, rising to **G2** if a signature departs from §5.7 | System-architect agent for G1; the user for G2 |
| Stored formats and migrations (journal, archive `format_version`, schema), a public command or exit-code change after release | Irreversible (Q8), whatever the criticality class | **G3** | The user, with a named independent reviewer (none named: FIND-016) |
| Host/Origin checks (NFR-7), file modes | Q8 depends on whether these are security-critical, which the user hasn't said | **G3 until answered** (the higher tier when unsure) | The user (FIND-016) |

### 4.4 Assumptions the engineer would otherwise have to make

| ID | Assumption | Run 2 status |
|---|---|---|
| ASM-R1 | The system is business criticality, not safety, not regulated | **Resolved**: the user decided it (CR-6.1). No longer an assumption |
| ASM-R2 | The CR-4 and CR-5 interpretations stand as written | **Resolved**: confirmed by the user (CR-6.3) |
| ASM-R3 | The Elo update is the standard `R' = R + K(S − E)` with `S` = 1, ½, 0 | Open. Design proceeds provisionally; FIND-004 |
| ASM-R4 | Crate versions are chosen at implementation and fixed by `Cargo.lock`; `shakmaty` and `pgn-reader` are taken at their current major versions | Open. D-3 is a one-way door, and the wrapped API depends on the major version; FIND-006. Step 1 must record the version it designs against |

The profile's own assumptions (the tool set, the meaning of "warnings as errors", the scope of the standard, "ordinary" read as "business", the gate ID range) are ASM-S1 to ASM-S5 in SWE-001 §4. They are low-impact and are not findings.

---

## 5. Findings

Severity: **Blocker** means design cannot proceed safely. **Major** means proceed only under a recorded assumption, with an owner and a closing point. **Minor** means it can be resolved during design.

### 5.0 Closed in run 2

| FIND | Owner | Answer on the page | Closed because |
|---|---|---|---|
| FIND-001 (was Blocker) | Human | Requirements Decision Log CR-6.1; SWE-001 §1; architecture §8; test-strategy "Quality gates": ordinary criticality, not safety-related, not regulated; SEI CERT with standard Rust and TypeScript checks and warnings as errors | R11 now has its evidence. **Two residual questions the answer did not cover** are raised as FIND-014 and FIND-016 |
| FIND-002 | Human | SWE-001 §1 "Process authority", §3; requirements CR-6.2: the system-architect agent approves one-component changes (desk review), the user approves anything wider | The approvers are recorded. The G3 independent reviewer is still unnamed (FIND-016) |
| FIND-003 | Human | Requirements CR-6.3; the "Open Questions, Amendment 2" paragraph now says confirmed; architecture §14 Q-A18 answered | The interpretations are confirmed. Design no longer treats them as provisional |
| FIND-009 | Human | SWE-001 §1 "Environment limits"; requirements CR-6.4: no Intel Mac, none expected soon | The question (is hardware available) is answered. **Its consequence is a new finding, FIND-015** |

### 5.1 Open: Human approver (Brian Prascher)

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Severity |
|---|---|---|---|---|---|---|
| FIND-014 | R11, A7 | SWE-001 §1 "Coding standard"; requirements Decision Log CR-6.1 | The standard is "SEI CERT, with the standard Rust and TypeScript checks and warnings as errors". The kit's CERT scope lists C, C++ and Java; it lists no Rust or TypeScript rule set (`[UNV]`: CERT's own publications were not looked up). The project's own code is Rust and TypeScript (architecture D-5, §3) | A conformance statement ("conforms to CERT") needs rules to check against. Without them, a review can apply CERT's principles by analogy but can't claim conformance. SDD authors and reviewers need to know which of the two is meant | What should "CERT" mean for Rust and TypeScript code: which rules or practices are checked, and what may a review claim (a conformance statement, or findings only)? Does the tool set of SWE-001 ASM-S1 count as the checkable part? | **Major** (step-1 condition) |
| FIND-015 | R12, R2 | requirements PLAT-2 "Verification" and IF-4; test-strategy "Quality gates" ("Intel (x86_64) and Apple silicon (arm64) builds both pass the suite"); architecture §9 "Platform matrix", D-25, R-13; requirements CR-6.4 | PLAT-2 is Must and requires the app on "every Mac that the macOS release supports, Intel or Apple silicon". Its verification includes a smoke test "on an Intel Mac … and on Tahoe on an Intel Mac, an M1 Mac, and an M4 Mac". The user: no Intel Mac, "we won't be able to test that requirement anytime soon". An M1 and an M4 Mac are not recorded as available either | A baselined Must requirement can't be verified in part. D-25 commits to a universal build with an x86_64 Stockfish slice that can't be exercised. Nothing records an accepted risk, and only a named human can accept one | Does PLAT-2 stand as written with its Intel verification not performed, recorded by you as an accepted risk? Or is something to change (PLAT-2, IF-4 and D-25's universal build, the test-strategy Intel gate)? Are an M1 and an M4 Mac available for the matrix? | **Major** (decision due before step-3 packaging and verification planning; steps 1 and 2 unaffected) |
| FIND-016 | R11, gate | SWE-001 §1 "Criticality" and "Process authority", §3; `swe-governance` §4 Q8 | The question in run 1 also asked whether the loopback web server (NFR-7) is security-critical for gate purposes. The answer ("ordinary, not safety or regulated") is silent on security. A G3 change needs a named independent reviewer; none is named | `chess-server` and data-directory work at step 2 can't be assigned G2 or G3. SWE-001 takes the higher tier (G3) meanwhile | Are the Host/Origin checks (NFR-7) and the data directory's file modes security-critical for gate purposes? Who is the independent reviewer for a G3 change? | Minor (due step 2) |

### 5.2 Open: Systems Engineer

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Severity |
|---|---|---|---|---|---|---|
| FIND-004 | R3, R4 | requirements.md ELO-1 | "Standard Elo expected score `E = 1/(1+10^((Rb−Ra)/400))` (**assumed**; standard formula)". The update rule (new rating from K, the score and E) is not stated. The verification says "matches hand-computed expected results **to rounding rule**", but gives no comparison precision; ratings are "stored as real numbers and displayed rounded to the nearest integer" | `EloCalc` is a step-1 unit with mutation testing. A formula marked "assumed" in a baselined requirement, and no test tolerance, leave its oracle open | Is the expected-score formula confirmed, and is the update `R' = R + K(S − E)` with `S` = 1 for a win, ½ for a draw and 0 for a loss? What precision do the known-value tests compare to (for example, 1e-9 on stored values, and the integer as displayed)? Should the "assumed" label move to the TBR register or be removed? | Major |
| FIND-010 | R8 | test-strategy.md "Open verification items", item "Design values that tests depend on"; requirements register TBD-24, TBD-28 | test-strategy lists TBD-28 (stale-lease takeover time) as an open design value, but the register shows it **Closed** at 0 s. TBD-24 is "Open (design)" in the register, while architecture §5.7 already gives the tag set (`Mode`, `Rated`, `WhiteClass`, `BlackClass`, `EndStatus`) | Tests written against the register could use the wrong status, and the PGN exporter needs to know whether the tag set is final | Can TBD-28 be removed from the test-strategy list? Is TBD-24 closed by architecture §5.7, or does it stay open until TBC-6 is done? | Minor |
| FIND-011 | R4 | requirements.md PLY-4 | "The Game tracks computer Players' statistics (games, W/L/D, **etc.**)" | Open-ended wording. UI-2 and DAT-3 already state the statistics, so the design risk is low | Can "etc." be replaced by a reference to UI-2 and DAT-3? | Minor |

### 5.3 Open: System Architect

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Severity |
|---|---|---|---|---|---|---|
| FIND-005 | A2, R8 | architecture §5.7 "PGN/FEN I/O"; requirements TBD-24, TBC-6 | The PGN export tag set and formatting depend on TBD-24 (custom tag names, "Open (design)") and TBC-6 (Seven Tag Roster text, export-format tag spacing, the `*` token and `Termination "abandoned"`, not yet checked against the PGN Standard). Both are due "Step 1 (before the PGN exporter)" | The `chess-rules` PGN exporter is step-1 work, and IF-1 requires byte-identical output from every front-end, so the format must be fixed before the exporter's design is final | Will TBC-6 be checked and TBD-24 closed before the `chess-rules` SDD is approved, or should the exporter section of the SDD be marked provisional? | Major (step-1 condition) |
| FIND-006 | A7 | architecture §3.2, §11 (D-3, D-5 to D-7, D-12, D-13), §6.3 | No version is stated for Rust (edition, minimum version), `shakmaty`, `pgn-reader`, `tokio`, `ratatui`, `rusqlite`, Tauri, React, Vite, chessground, Node or TypeScript. The development Mac has Rust 1.89.0 and Node v26.12.1, which is a fact about the host, not a pin | Step 1 wraps `shakmaty` and `pgn-reader` (D-3, one-way moderate), whose APIs change between major versions. Tauri's major version changes the sidecar, IPC and CSP mechanics (§6.3, OQ-11) | Which Rust edition and minimum Rust version apply? Should the architecture pin the major versions of the step-1 dependencies now, and Tauri's before step 3, or leave all versions to the implementer with `Cargo.lock` as the record? | Major |
| FIND-007 | A10 | architecture §4.8 "Journal flush"; NFR-8; ASM-2 | NFR-8 allows 100 ms from input to visible feedback. The journal flush (`F_FULLFSYNC`) is on the move path and costs "a few ms per move", unmeasured. The budget is not split between flush, core and render. No memory or disk budget is set | Affects `chess-store` and `GameSession` design at step 2, not step 1 | Will the architect allocate the NFR-8 budget (flush, core, transport, render) once ASM-2 is measured, and is a memory ceiling wanted for the engine pool (one Stockfish per computer side plus the analysis instance, §4.7)? | Major (step-2 condition) |
| FIND-008 | A8 | architecture §10.3 G-15; D-20 | The ECO opening table's data source and license are open ("owner: architect, step 1"). `EcoTable` is a `chess-domain` type | The classifier's interface can be designed now, but its data format and the bundled file can't be fixed until the source is chosen (NFR-6) | Will G-15 be closed before the `chess-domain` SDD is approved? If not, may the SDD mark `EcoTable` data loading as provisional behind a fixed interface? | Major (step-1 condition) |
| FIND-012 | A1, A12 | architecture §3.1 vs §3.3; §10.2 vs §10.3 G-9; §13 | (a) §3.1 lists "export/import" under `chess-store`, while §3.3 says "formats are owned by `ImportExportService`". (b) §10.2 says "`PROCESS_HEARTBEAT`, `SchemaTooNew` … Requirements gap G-9 asks for an explicit requirement", but §10.3 shows G-9 **Resolved** (D-16). (c) §13 steps are numbered 1, 2, 3, 4, 6, 5 | Text consistency only; (a) could mislead the `chess-store` SDD at step 2 | Can §3.1, §10.2 and §13 be corrected at the next amendment? | Minor |
| FIND-013 | A8 | architecture §14 ASM-3, ASM-4, ASM-5, OQ-9, OQ-10, OQ-11; requirements TBC-7 to TBC-10 | Each has an owner and a closing step (2 or 3) | Not needed for step 1. They gate `chess-engine` calibration bounds (TBC-8), the build path (TBC-7), the GUI board (OQ-9) and packaging (ASM-5, OQ-11) | None now. Confirm each at the step named | Minor (tracked) |

---

## 6. Verdict

### 6.1 Overall

**READY-WITH-CONDITIONS.** The rule is met: there is no blocker, and every Major finding has an owner, a recorded provisional treatment and a closing point. The closing points are implementation steps, not calendar dates, because the project has no schedule and the requirements register uses the same convention; the user can add dates if wanted.

### 6.2 Per implementation step

| Step | Components | Verdict | Conditions (the provisional parts of the design) |
|---|---|---|---|
| 1 | `chess-rules`, `chess-domain` | **READY-WITH-CONDITIONS** | (1) The PGN exporter section of the `chess-rules` SDD stays provisional until TBC-6 is checked and TBD-24 closed (FIND-005). (2) `EloCalc` stays provisional until ELO-1's formula and test precision are confirmed (FIND-004). (3) `EcoTable` data loading stays provisional until G-15 is closed (FIND-008). (4) The Rust edition, minimum Rust version and the `shakmaty` and `pgn-reader` versions designed against are recorded in the SDD (FIND-006). (5) Each SDD states the coding-standard basis it was written to and makes **no CERT conformance claim** until FIND-014 is answered. (6) Approvers: satisfied by SWE-001 (G1 by the system-architect agent as a desk review, G2 and G3 by the user) |
| 2 | `chess-store`, `chess-engine`, `chess-app`, `chess-cli`, `chess-server` | READY-WITH-CONDITIONS (not yet due) | FIND-007 (NFR-8 budget after the ASM-2 measurement); FIND-016 (security-critical designation and G3 reviewer before `chess-server` and data-directory work; G3 meanwhile); TBC-7 to TBC-9 and ASM-3 (engine facts); ASM-4 (terminal resize); FIND-012 (a) before the `chess-store` SDD |
| 3 | `ui/`, `desktop/src-tauri`, `packaging/`, enabling-system blocks | READY-WITH-CONDITIONS (not yet due) | **FIND-015 decided before the packaging and verification plan** (PLAT-2 Intel); OQ-9 (D-4 board spike) before the `Board` SDD; OQ-10, OQ-11, ASM-5; Tauri version (FIND-006); TBD-18 wording |

**What is strong.** The baselines are complete and the allocation is closed in both directions, re-derived this run: 82 requirements and 104 leaf functions, all allocated and traced. Requirements are verifiable, with test levels. The architecture defines contracts, data ownership, the state machine, the concurrency model, an error catalogue and testability seams. The approvers, criticality and coding standard are now recorded. None of the findings asks for a change to the architecture's structure.

**What to watch.** Two of the three new findings (FIND-014, FIND-015) come from the user's answers themselves. Neither blocks step 1, but FIND-014 shapes every review from the first SDD, and FIND-015 leaves a Must requirement partly unverifiable.

---

## 7. Hand-back log

The agent marks a finding resolved only when the owner's answer or revised artifact is on the page.

| FIND | Owner | Sent | Answer | Status |
|---|---|---|---|---|
| FIND-001 | Human | Run 1, 2026-10-28 | 2026-10-28 (user, in session); recorded in CR-6.1, SWE-001, architecture §8, test-strategy | **Closed** (run 2). Residuals reopened as FIND-014 and FIND-016 |
| FIND-002 | Human | Run 1 | 2026-10-28; CR-6.2, SWE-001 | **Closed** (run 2) |
| FIND-003 | Human | Run 1 | 2026-10-28 ("yes"); CR-6.3, architecture Q-A18 | **Closed** (run 2) |
| FIND-004 | Systems Engineer | Run 1 | — | Open |
| FIND-005 | System Architect | Run 1 | — | Open |
| FIND-006 | System Architect | Run 1 | — | Open |
| FIND-007 | System Architect | Run 1 | — | Open (due step 2) |
| FIND-008 | System Architect | Run 1 | — | Open |
| FIND-009 | Human | Run 1 | 2026-10-28 (no Intel Mac, none expected); CR-6.4, SWE-001 | **Closed** (run 2). Consequence raised as FIND-015 |
| FIND-010 | Systems Engineer | Run 1 | — | Open |
| FIND-011 | Systems Engineer | Run 1 | — | Open |
| FIND-012 | System Architect | Run 1 | — | Open |
| FIND-013 | System Architect | Run 1 | — | Tracked (steps 2 and 3) |
| FIND-014 | Human | Run 2, 2026-10-28 | — | Open |
| FIND-015 | Human (decision); then Systems Engineer and System Architect for any change | Run 2 | — | Open (due before step 3 planning) |
| FIND-016 | Human | Run 2 | — | Open (due step 2) |

---

## 8. Not analysed, not run, assumed

- **Not run:** no build or test, because no code exists. No Mermaid in this report, so none was validated here. The upstream documents report their own validation (architecture: 14 of 14; functional_decomp: 9 of 9; use cases: 6 of 6). The documents edited for Amendment 4 contain no new diagrams.
- **Re-run in run 2 (`[RAN]`, script over the documents):** requirement-to-architecture allocation (82 of 82); requirement IDs found in the functional decomposition (82 of 82) and in the use cases (82 of 82, none unknown); leaf functions cited by a use case (104 of 104, every cited ID exists); each Amendment 4 edit present exactly once; relative `.md` links in the changed documents (0 broken); stale "draft" or "not given" markers in the README, functional decomposition and use-case README (0).
- **Read in full (runs 1 and 2):** requirements.md preamble, §11 (NFR), §12 (IF), §13 (risks), the TBD/TBR/TBC register, RULE-1, ELO-1, ELO-2, PLAT-2, the Decision Log tail; architecture §1, §2.2, §3, §4, §5.5 to §5.7, §7.4, §8, §9, §10.2, §10.3, §11, §11.3, §12, §13, §14; functional_decomp §7 to §9; use_cases README headers, §9 and §10; test-strategy.md.
- **Checked by script, not read line by line:** the other requirement statements (presence, allocation and IDs), architecture §10.1 row coverage, §6 front-ends and §7.2 data model.
- **Not analysed:** the front-end screen designs (§6.1, §6.2), the GUI board trade studies, and each use case's flow against the state machine. The functional decomposition reports a thread walk (FG-12); it was not repeated.
- **Unverified (`[UNV]`):** that SEI CERT publishes no rule set for Rust or TypeScript. This comes from the kit's own scope list, not from CERT's publications (FIND-014). The availability of the `cargo-deny`, `cargo-fuzz`, `cargo-mutants`, `proptest` and Playwright tools named in architecture §9 was not checked.
- **Assumed:** none for the verdict. The assumptions of the engagement profile are ASM-S1 to ASM-S5 in SWE-001 §4.
- **Independence:** the same model wrote the upstream documents in other roles and wrote this review, so it is not independent in the organisational sense. The user approved the use cases and the functional decomposition on 2026-10-28 without an independent review.
