# Chess Application — SRR/SFR Detailed-Design Readiness

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/SRR-SFR-readiness.md | **1.2 (run 3, answers applied)** | 2026-10-30 | **Run 3: the user's answers to 8 findings are applied. No blocker. 8 closed this run, 4 open (all Minor).** | Software engineer role (Claude), for Brian Prascher | Not an approval document. The owners named in §5 answer the open findings |

**Run 3 (2026-10-30).** The user answered FIND-008 (the ECO source, afterwards, with CR-8), FIND-004, FIND-005, FIND-006, FIND-007, FIND-014, FIND-015 and FIND-016. Each answer is now on the page (requirements Amendment 5 / CR-7, architecture amendment 5, test-strategy Amendment 5, SWE-001 v1.1), and the seven are closed in §5.0. Run 3 is an update, not a full re-run: only the checks these seven touch were re-evaluated (R3, R4, R11, R12, A2, A7, A10), and the verdict per step in §6.2 was recomputed. Sections 1, 2 and 4 describe run 2 and are kept as the record.

**Verdict in one line (run 2, updated by run 3):** **READY-WITH-CONDITIONS.** Run 1 found one blocker (no criticality class or coding standard). The user answered it, and the answer is now on the page in the requirements Decision Log, the architecture, the test strategy and a new engagement profile. Detailed design of step 1 (`chess-rules`, `chess-domain`) can start, with the provisional parts listed in §6. Steps 2 and 3 carry further conditions that are not yet due. Two new findings need the user: how SEI CERT applies to Rust and TypeScript (FIND-014), and what happens to PLAT-2 now that Intel can't be tested (FIND-015).

**Reading guide.** §0 compares run 1 and run 2. §1 gives the target and inputs. §2 maps SRR and SFR exit criteria. §3 gives the checklist results. §4 covers the allocation and the design gate. §5 lists the findings by owner. §6 gives the verdict per step. §7 is the hand-back log. §8 lists what was not analysed.

---

## 0. Run history

| Run | Date | Trigger | Verdict | Blockers | Findings |
|---|---|---|---|---|---|
| 1 | 2026-10-28 | First check of the baselined documents | **NOT-READY** | 1 (FIND-001) | 13 raised: 1 Blocker, 8 Major, 4 Minor |
| 2 | 2026-10-28 | The user's answers to FIND-001, FIND-002, FIND-003 and FIND-009, recorded in the documents | **READY-WITH-CONDITIONS** | 0 | 4 closed. 3 new (FIND-014 to FIND-016). **12 open: 7 Major, 5 Minor** |
| 3 | 2026-10-30 | The user's answers to FIND-008 (ECO source, CR-8), FIND-004, FIND-005, FIND-006, FIND-007, FIND-014, FIND-015, FIND-016, recorded in the documents (update, not a full re-run) | **READY-WITH-CONDITIONS** | 0 | 8 closed. **4 open, all Minor (FIND-010 to FIND-013)** |

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
| SFR | Open decisions and risks have owners and closing points | A8, R8 | Met, with 3 findings | **Met**, with Minor findings only |
| Both | Authority for approving design changes is defined | swe-governance gate | **Not met** as a document | **Met**: SWE-001 names the approvers (FIND-002 closed) |

---

## 3. Checklist results

Run 1 results are shown where run 2 differs.

### 3.1 Requirement readiness (owner: Systems Engineer)

| # | Check | Run 1 | Run 2 | Evidence (run 2) |
|---|---|---|---|---|
| R1 | Identified and allocated | Pass | **Pass** | 82 IDs. Every one is allocated in architecture §10.1: 80 have their own row and IF-2 and IF-3 share the IF-1/IF-2/IF-3 row (`[RAN]` script, 0 unallocated) |
| R2 | Verifiable | Pass | **Pass** | Each requirement has a Verification entry with a level (U, I, E, M) and a method in 29148 terms (requirements preamble). PLAT-2's Intel verification is conceivable but can't be performed (FIND-015, under R12) |
| R3 | Quantified | Pass, with ESTs | **Pass, with ESTs** | NFR-1 "≤ 3 s (CLI ≤ 1 s) … median of 10 runs"; NFR-8 "within 100 ms (EST)"; ENG-5 200 ms (TBD-12); ENG-4 timeouts (TBD-13). The EST values are accepted starting values (Q53), confirmed by measurement at step 2. Run 3: ELO-1 now states its formula and exact-integer tests (FIND-004 closed) |
| R4 | Singular and unambiguous | Pass, minor | **Pass, minor** | Compound requirements were split at v1.0. Leftover: PLY-4 "statistics (games, W/L/D, **etc.**)" (FIND-011) |
| R5 | Complete for behaviour | Pass | **Pass** | RULE-1 (b), (c), IF-3 illegal-FEN cases, DAT-7, ENG-4, PLAT-4 start-up errors and shutdown |
| R6 | Interfaces specified | Pass | **Pass** | IF-1 to IF-5. The internal protocol is specified in architecture §5 by design |
| R7 | Consistent | Pass, with a stability caveat | **Pass** | No conflicts found. The caveat of run 1 (interpretations open to the user's correction) is closed: requirements Decision Log CR-6.3 confirms them, and architecture Q-A18 is answered |
| R8 | TBD/TBR controlled | Pass, minor | **Pass, minor** | The register gives each open item an owner and a closing step. Steps stand in for dates because the project has no schedule. Status drift between documents: FIND-010 |
| R9 | Non-functional present | Pass | **Pass** | NFR-1 to NFR-10, UI-4, UI-6 to UI-8. Safety and regulation are now stated as not applicable (R11) |
| R10 | Traceable | Pass | **Pass** | Each requirement names its source (user's 15 top-level requirements, Q1 to Q79, CR-1 to CR-6, Decision Log). The new criticality and coding-standard decisions trace to CR-6.1 |
| R11 | Criticality assigned | **Fail** | **Pass (run 3: residuals closed)** | Requirements Decision Log CR-6.1: ordinary criticality, not safety-related, not regulated; SEI CERT with the standard Rust and TypeScript checks and warnings as errors. Also in SWE-001 §1, architecture §8 and test-strategy "Quality gates". `[RAN]` search counts for "criticality": requirements 5, architecture 3, test-strategy 2, SWE-001 5 (run 1: 0 in requirements and architecture). Run 3: both residuals are answered. CERT informs best practice for Rust and TypeScript, with no conformance claim (FIND-014, SWE-001 §6). The application is local-only and not security-critical, with best practice and CVE scanning as the security bar (FIND-016). R11 is a plain **Pass** |
| R12 | Verification environment | Pass, with a hardware question | **Partial (run 3: Pass, with Intel deferred)** | test-strategy.md and architecture §9 specify levels, data, determinism and tools. The user's answer: no Intel Mac and none expected soon. PLAT-2's verification also names "an Intel Mac, an M1 Mac, and an M4 Mac"; the availability of an M1 and an M4 is not recorded. Steps 1 and 2 are unaffected. FIND-015. Run 3: the user deferred verification on unavailable platforms as best effort (test-strategy "Platform availability", CR-7.6), so R12 is **Pass** for the platforms the project has |

### 3.2 Architecture readiness (owner: System Architect)

| # | Check | Run 1 | Run 2 | Evidence (run 2) |
|---|---|---|---|---|
| A1 | Component boundary | Pass, minor | **Pass, minor** | §3.3, §4.8 "Must not" column, §2.2 allocation names. §3.1 says `chess-store` holds "export/import" while §3.3 says "formats are owned by `ImportExportService`" (FIND-012) |
| A2 | Interfaces | Pass for step 1 | **Pass for step 1, conditional later** | §5.1 to §5.7 protocol, error catalogue, idempotency, internal contracts; `Policy::check` in §4.4. The PGN export tag set depends on TBD-24 and TBC-6 (FIND-005) |
| A3 | Dependencies | Pass | **Pass** | §3.2 dependency graph, acyclic |
| A4 | Data ownership | Pass | **Pass** | §7.4 one writer per entity, transaction boundaries; §7.6 journal wins |
| A5 | Quality drivers | Pass (equivalent form) | **Pass (equivalent form)** | No QAS artefacts, but measurable drivers are mapped: NFR-1, NFR-2, NFR-8, ENG-3 to ENG-5, §4.8 table (ESTIMATEs until measured, ASM-2) |
| A6 | Runtime behaviour | Pass | **Pass** | §4.3 state machine with failure states, §5.3, §5.4, §4.8 concurrency, §7.3, §7.6 |
| A7 | Technology constraints | Pass, major gap on versions | **Pass, major gap on versions** | Languages and frameworks decided; no version pinned (FIND-006). Run 3: baseline versions are recorded in architecture §3.4, so A7 is **Pass**; the development Mac's Rust 1.89.0 and Node 26.12.1 must be updated before step 1. New: the coding standard is recorded as a constraint with its authority, the user (architecture §8, CR-6.1) |
| A8 | Decisions recorded | Pass | **Pass** | §11 D-1 to D-37 with door types and revisit triggers. Q-A18 is now answered. Open: D-21 (DEFERRED, two-way), D-4 gated by its spike (OQ-9) |
| A9 | Cross-cutting | Pass | **Pass** | §8 now also has a "Coding standard and static checks" row, with a matching §10.2 entry for the missing requirement driver |
| A10 | Resource budgets | Partial | **Partial** | The NFR-8 100 ms budget is not split between flush, core and render; flush cost unmeasured (ASM-2); no memory ceiling for the engine pool (FIND-007). Run 3: the user made the allocation a **Should** (CR-7.4), so A10 is **Pass, with a Should** |
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
| `chess-domain` | PLY-1, PLY-4, PLY-5, GAME-1, GAME-3, RULE-2, RULE-3, RULE-4, ELO-1, ELO-2, ENG-2, ANA-2, DAT-3 (ECO), UI-2, UI-3, UI-5, CLI-1, CLI-2 | `Game`, `GameSetup`, `Policy`, `Action`, `Clock`, `EloCalc`, `RosterGen`, `VariationTree`, `EcoTable`, `BoardGrid`, `BoardStyle` | **Ready** (run 3: FIND-004 and FIND-008 closed) |

### 4.3 Gate classification of the planned work

The approvers are now recorded in SWE-001 §1 and §3 (user, 2026-10-28): the system-architect agent approves G1 as a desk review, and the user approves G2 and G3.

| Work | Gate answers (swe-governance §4) | Expected tier | Approver |
|---|---|---|---|
| SDDs for `chess-rules` and `chess-domain` as drawn in §3 | No new component, interface or dependency edge beyond §3.2 and §5.7. The crates are new code but are already drawn in the architecture | **G1 per component** | System-architect agent, desk review, recorded as such |
| Fixing the public API of `chess-rules` (types and signatures consumed by `chess-domain`, `chess-engine`, `ImportExportService`) | Defines an interface that other components use (Q2). It realises the §5.7 contract rather than changing it | G1, rising to **G2** if a signature departs from §5.7 | System-architect agent for G1; the user for G2 |
| Stored formats and migrations (journal, archive `format_version`, schema), a public command or exit-code change after release | Irreversible (Q8), whatever the criticality class | **G3** | The user, with a named independent reviewer (none named: FIND-016) |
| Host/Origin checks (NFR-7), file modes | Q8 depends on whether these are security-critical, which the user hasn't said | ~~G3 until answered~~ **Ordinary** (G1 inside one crate, G2 across crates) since 2026-10-30 | The user answered (FIND-016) |

### 4.4 Assumptions the engineer would otherwise have to make

| ID | Assumption | Run 2 status |
|---|---|---|
| ASM-R1 | The system is business criticality, not safety, not regulated | **Resolved**: the user decided it (CR-6.1). No longer an assumption |
| ASM-R2 | The CR-4 and CR-5 interpretations stand as written | **Resolved**: confirmed by the user (CR-6.3) |
| ASM-R3 | The Elo update is the standard `R' = R + K(S − E)` with `S` = 1, ½, 0 | **Closed 2026-10-30.** Confirmed and completed: integer ratings, `R' = R + round_half_away(K(S − E))` (ELO-1, CR-7.1) |
| ASM-R4 | Crate versions are chosen at implementation and fixed by `Cargo.lock`; `shakmaty` and `pgn-reader` are taken at their current major versions | **Closed 2026-10-30.** Versions are the latest stable as of 2026-10-28 (architecture §3.4, CR-7.3) |

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

### 5.0a Closed in run 3 (2026-10-30)

| FIND | Owner | User's answer | Answer on the page | Closed because |
|---|---|---|---|---|
| FIND-004 (Major) | Systems Engineer | "Do your research and settle on a common Elo expression, the update formula, the values for S. The precision stored and shown will always be whole integer values; mark resolved" | Requirements ELO-1 and CR-7.1; architecture §7.2 | `E = 1/(1+10^((R_B−R_A)/400))`; S = 1, ½, 0; `R' = R + round(K(S−E))`, rounded half away from zero; integer ratings; exact-integer known-value tests. The "assumed" label is gone |
| FIND-005 (Major) | System Architect | "Check the PGN standard and update the approach to match the standard PGN approach; mark resolved after research and update" | Requirements IF-1, TBC-6 and TBD-24 closed, CR-7.2; architecture §5.7 | The standard's text was read. The export was aligned (tag order, escaping, `Round`, `Termination` mapping, movetext rules). The `chess-rules` SDD no longer needs a provisional exporter section |
| FIND-006 (Major) | System Architect | "Use the latest version of all open source components and languages as of the baselined requirements; mark resolved after noting baseline versions" | Architecture §3.4; SWE-001 §1; CR-7.3 | Versions are noted. **Action for the user:** the development Mac's Rust (1.89.0) and Node (26.12.1) must be updated before step 1 |
| FIND-007 (Major) | System Architect | "Mark as a should, not a must; mark resolved" | Architecture §4.8; CR-7.4 | The NFR-8 budget allocation and the engine-pool memory ceiling are Should items. NFR-8 stays Must. *Interpretation:* "it" is the allocation, not NFR-8 itself |
| FIND-008 (Major) | System Architect | "Option A is approved; proceed as you've suggested" (2026-10-30, after the options and four decisions were put to the user) | Requirements DAT-3(a), NFR-6, CR-8; architecture G-15, D-20, §8.1, §7.2 | The Lichess `chess-openings` dataset (CC0-1.0) is the source; a pinned snapshot goes in `third_party/eco/` when step 1 starts; code and name are stored; matching is by position, deepest named position. `EcoTable` data loading is no longer provisional |
| FIND-014 (Major) | Human | "Use CERT to inform best practice for languages not explicitly covered by CERT; mark resolved" | SWE-001 §1 and §6; architecture §8; test-strategy; CR-7.5 | A mapping from CERT areas to Rust and TypeScript practice is written. Reviews report findings and make no conformance claim |
| FIND-015 (Major) | Human | "Defer test for Intel Mac and other unavailable platforms; update the test strategy to be best effort for platforms that aren't immediately available; mark resolved" | Test-strategy "Platform availability" and quality gates; architecture §9, R-13; requirements PLAT-2 note, CR-7.6 | Verification on unavailable platforms is deferred and best effort, as an accepted risk. PLAT-2's text is unchanged |
| FIND-016 (Minor) | Human | "The application will only run locally on my machine for v1; there is no criticality or security requirements to enforce for v1 other than security best practice and avoiding known CVEs; mark resolved" | SWE-001 §1, §3, §5; architecture §8; test-strategy; CR-7.7 | Not security-critical. G3 needs no independent reviewer. CVE scanning is a CI and release gate |

### 5.1 Open: Human approver (Brian Prascher)

None. FIND-014, FIND-015 and FIND-016 are closed (§5.0a).

### 5.2 Open: Systems Engineer

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Severity |
|---|---|---|---|---|---|---|
| FIND-010 | R8 | test-strategy.md "Open verification items", item "Design values that tests depend on"; requirements register TBD-24, TBD-28 | test-strategy lists TBD-28 (stale-lease takeover time) as an open design value, but the register shows it **Closed** at 0 s. TBD-24 is "Open (design)" in the register, while architecture §5.7 already gives the tag set (`Mode`, `Rated`, `WhiteClass`, `BlackClass`, `EndStatus`) | Tests written against the register could use the wrong status, and the PGN exporter needs to know whether the tag set is final | Can TBD-28 be removed from the test-strategy list? Is TBD-24 closed by architecture §5.7, or does it stay open until TBC-6 is done? | Minor |
| FIND-011 | R4 | requirements.md PLY-4 | "The Game tracks computer Players' statistics (games, W/L/D, **etc.**)" | Open-ended wording. UI-2 and DAT-3 already state the statistics, so the design risk is low | Can "etc." be replaced by a reference to UI-2 and DAT-3? | Minor |

### 5.3 Open: System Architect

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Severity |
|---|---|---|---|---|---|---|
| FIND-012 | A1, A12 | architecture §3.1 vs §3.3; §10.2 vs §10.3 G-9; §13 | (a) §3.1 lists "export/import" under `chess-store`, while §3.3 says "formats are owned by `ImportExportService`". (b) §10.2 says "`PROCESS_HEARTBEAT`, `SchemaTooNew` … Requirements gap G-9 asks for an explicit requirement", but §10.3 shows G-9 **Resolved** (D-16). (c) §13 steps are numbered 1, 2, 3, 4, 6, 5 | Text consistency only; (a) could mislead the `chess-store` SDD at step 2 | Can §3.1, §10.2 and §13 be corrected at the next amendment? | Minor |
| FIND-013 | A8 | architecture §14 ASM-3, ASM-4, ASM-5, OQ-9, OQ-10, OQ-11; requirements TBC-7 to TBC-10 | Each has an owner and a closing step (2 or 3) | Not needed for step 1. They gate `chess-engine` calibration bounds (TBC-8), the build path (TBC-7), the GUI board (OQ-9) and packaging (ASM-5, OQ-11) | None now. Confirm each at the step named | Minor (tracked) |

---

## 6. Verdict

### 6.1 Overall

**READY-WITH-CONDITIONS.** The rule is met: there is no blocker, and every Major finding has an owner, a recorded provisional treatment and a closing point. The closing points are implementation steps, not calendar dates, because the project has no schedule and the requirements register uses the same convention; the user can add dates if wanted.

### 6.2 Per implementation step

| Step | Components | Verdict | Conditions (the provisional parts of the design) |
|---|---|---|---|
| 1 | `chess-rules`, `chess-domain` | **READY-WITH-CONDITIONS** | (1) `EcoTable`'s snapshot of the Lichess data (with its commit and SHA-256) is copied into `third_party/eco/` at the start of step 1 (FIND-008 closed). (2) The Rust and Node toolchains on the development Mac are updated to the baseline versions of architecture §3.4 (FIND-006 closed; **user action**). (3) Each SDD states the coding-standard basis (SWE-001 §6) and makes no CERT conformance claim. (4) Approvers: G1 by the system-architect agent as a desk review, G2 and G3 by the user (SWE-001). *Cleared in run 3:* the PGN exporter (FIND-005), `EloCalc` (FIND-004) and `EcoTable` data (FIND-008) are no longer provisional |
| 2 | `chess-store`, `chess-engine`, `chess-app`, `chess-cli`, `chess-server` | READY-WITH-CONDITIONS (not yet due) | TBC-7 to TBC-9 and ASM-3 (engine facts); ASM-4 (terminal resize); FIND-012 (a) before the `chess-store` SDD. *Cleared or reduced in run 3:* FIND-016 (no security-critical designation, no G3 reviewer); FIND-007 is a Should (measure ASM-2, then allocate the NFR-8 budget if the flush is a large share) |
| 3 | `ui/`, `desktop/src-tauri`, `packaging/`, enabling-system blocks | READY-WITH-CONDITIONS (not yet due) | OQ-9 (D-4 board spike) before the `Board` SDD; OQ-10, OQ-11, ASM-5; TBD-18 wording. *Cleared in run 3:* FIND-015 (Intel is best effort and recorded as an accepted risk); the Tauri version is fixed at 2.12 (FIND-006) |

**What is strong.** The baselines are complete and the allocation is closed in both directions, re-derived this run: 82 requirements and 104 leaf functions, all allocated and traced. Requirements are verifiable, with test levels. The architecture defines contracts, data ownership, the state machine, the concurrency model, an error catalogue and testability seams. The approvers, criticality and coding standard are now recorded. None of the findings asks for a change to the architecture's structure.

**What to watch (run 3).** Four Minor findings remain (FIND-010 to FIND-013); no Major finding is open. **FIND-010 is in practice done:** run 3 closed TBD-24 and TBC-6 and removed TBD-28 from the test-strategy list, which were its two points; the owner has not confirmed. The one action that blocks the first build is the toolchain update (Rust 1.89.0 cannot build the baseline `shakmaty`). *Run 2 text:* Two of the three new findings (FIND-014, FIND-015) come from the user's answers themselves. Neither blocks step 1, but FIND-014 shapes every review from the first SDD, and FIND-015 leaves a Must requirement partly unverifiable.

---

## 7. Hand-back log

The agent marks a finding resolved only when the owner's answer or revised artifact is on the page.

| FIND | Owner | Sent | Answer | Status |
|---|---|---|---|---|
| FIND-001 | Human | Run 1, 2026-10-28 | 2026-10-28 (user, in session); recorded in CR-6.1, SWE-001, architecture §8, test-strategy | **Closed** (run 2). Residuals reopened as FIND-014 and FIND-016 |
| FIND-002 | Human | Run 1 | 2026-10-28; CR-6.2, SWE-001 | **Closed** (run 2) |
| FIND-003 | Human | Run 1 | 2026-10-28 ("yes"); CR-6.3, architecture Q-A18 | **Closed** (run 2) |
| FIND-004 | Systems Engineer | Run 1 | 2026-10-30 (user); ELO-1, CR-7.1 | **Closed** (run 3) |
| FIND-005 | System Architect | Run 1 | 2026-10-30 (user); IF-1, architecture §5.7, CR-7.2 | **Closed** (run 3) |
| FIND-006 | System Architect | Run 1 | 2026-10-30 (user); architecture §3.4, CR-7.3 | **Closed** (run 3) |
| FIND-007 | System Architect | Run 1 | 2026-10-30 (user: a Should); architecture §4.8, CR-7.4 | **Closed** (run 3) |
| FIND-008 | System Architect | Run 1 | 2026-10-30 (user: Option A); DAT-3, D-20, CR-8 | **Closed** (run 3) |
| FIND-009 | Human | Run 1 | 2026-10-28 (no Intel Mac, none expected); CR-6.4, SWE-001 | **Closed** (run 2). Consequence raised as FIND-015 |
| FIND-010 | Systems Engineer | Run 1 | — | Open. Both points fixed as a side effect in run 3 (TBD-24 closed, TBD-28 off the list); awaiting the owner's confirmation |
| FIND-011 | Systems Engineer | Run 1 | — | Open |
| FIND-012 | System Architect | Run 1 | — | Open |
| FIND-013 | System Architect | Run 1 | — | Tracked (steps 2 and 3) |
| FIND-014 | Human | Run 2 | 2026-10-30 (user); SWE-001 §6, CR-7.5 | **Closed** (run 3) |
| FIND-015 | Human | Run 2 | 2026-10-30 (user: deferred, best effort); test-strategy, CR-7.6 | **Closed** (run 3) |
| FIND-016 | Human | Run 2 | 2026-10-30 (user); SWE-001 §1, §3, §5, CR-7.7 | **Closed** (run 3) |

---

## 8. Not analysed, not run, assumed

- **Not run:** no build or test, because no code exists. No Mermaid in this report, so none was validated here. The upstream documents report their own validation (architecture: 14 of 14; functional_decomp: 9 of 9; use cases: 6 of 6). The documents edited for Amendment 4 contain no new diagrams.
- **Re-run in run 2 (`[RAN]`, script over the documents):** requirement-to-architecture allocation (82 of 82); requirement IDs found in the functional decomposition (82 of 82) and in the use cases (82 of 82, none unknown); leaf functions cited by a use case (104 of 104, every cited ID exists); each Amendment 4 edit present exactly once; relative `.md` links in the changed documents (0 broken); stale "draft" or "not given" markers in the README, functional decomposition and use-case README (0).
- **Read in full (runs 1 and 2):** requirements.md preamble, §11 (NFR), §12 (IF), §13 (risks), the TBD/TBR/TBC register, RULE-1, ELO-1, ELO-2, PLAT-2, the Decision Log tail; architecture §1, §2.2, §3, §4, §5.5 to §5.7, §7.4, §8, §9, §10.2, §10.3, §11, §11.3, §12, §13, §14; functional_decomp §7 to §9; use_cases README headers, §9 and §10; test-strategy.md.
- **Checked by script, not read line by line:** the other requirement statements (presence, allocation and IDs), architecture §10.1 row coverage, §6 front-ends and §7.2 data model.
- **Not analysed:** the front-end screen designs (§6.1, §6.2), the GUI board trade studies, and each use case's flow against the state machine. The functional decomposition reports a thread walk (FG-12); it was not repeated.
- **Unverified (`[UNV]`):** that SEI CERT publishes no rule set for Rust or TypeScript. The kit's scope list and a web search on 2026-10-30 (which found none) say so; the CERT site itself was not read (FIND-014, closed on the user's decision to use CERT as an informant only).
- **Run 3 evidence (`[RAN]`, 2026-10-30):** package versions from the crates.io API, the npm registry, `channel-rust-stable.toml`, nodejs.org and the Stockfish GitHub release; the PGN Standard text at the TBC-3 source (§7, §8.1, §8.1.1, §8.2, §9.6.1, §9.7, §9.8.1; the last 27 000 characters were not read); the Elo known values computed with Python. The Elo formula was **not** checked against FIDE or USCF pages in this session; it is the standard logistic model, and ELO-1 records it as such. No build or test was run, because no code exists. The availability of the `cargo-deny`, `cargo-fuzz`, `cargo-mutants`, `proptest` and Playwright tools named in architecture §9 was not checked.
- **Assumed:** none for the verdict. The assumptions of the engagement profile are ASM-S1 to ASM-S5 in SWE-001 §4.
- **Independence:** the same model wrote the upstream documents in other roles and wrote this review, so it is not independent in the organisational sense. The user approved the use cases and the functional decomposition on 2026-10-28 without an independent review.
