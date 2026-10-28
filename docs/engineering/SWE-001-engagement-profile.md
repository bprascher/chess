# SWE-001 Engagement Profile

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/engineering/SWE-001-engagement-profile.md | 1.0 | 2026-10-28 | **Active.** Created from the user's answers to the readiness check (FIND-001, FIND-002, FIND-003, FIND-009) | Software engineer role (Claude), for Brian Prascher | Brian Prascher (user) gave the answers recorded below. The profile itself is not a design approval |

This is the first file of the software engineering record. It follows the `swe-governance` profile template. Entries marked `ASM-S` are assumptions of this profile, with a validation plan. They are not decisions.

## 1. Profile

| Field | Value | Source / ASM |
|---|---|---|
| System / repo | Chess: an offline, single-machine chess application for macOS. Rust core, terminal UI, local web UI and Tauri desktop app. Repository `/Users/brianprascher/Projects/chess`, branch `main`. | [OBS README.md] |
| Task type | Design. Detailed design (IEEE 1016 SDDs) and implementation, in the order of architecture §13: step 1 `chess-rules` and `chess-domain`, step 2 store, engine, core, CLI, server, step 3 GUI, desktop, packaging | [OBS architecture.md §13] |
| Languages and versions | **Rust** for the core, CLI, server and Tauri shell. **TypeScript** for the GUI. The Rust edition, the minimum Rust version, and the Node and TypeScript versions are **not set** in the architecture (readiness FIND-006). Stockfish 19 is a pinned third-party C++ binary, not project code. The language of the build and packaging scripts (`third_party/stockfish/`, `ci/`, `packaging/`) is not decided | [OBS architecture.md §3, D-5, D-9], ASM-S4 |
| Toolchain installed on the development Mac | `cargo 1.89.0`, `rustc 1.89.0`, `rustfmt 1.8.0-stable`, `clippy 0.1.89` (toolchain `stable-aarch64-apple-darwin`); `node v26.12.1`, `npm 11.20.0`. No `tsc` on the path (it would come with the project's own dependencies). These are what is installed, not what the project pins | [RAN `cargo --version`, `rustc --version`, `rustfmt --version`, `cargo clippy --version`, `node --version`, `npm --version` -> versions above, 2026-10-28] |
| Build and test commands | **None.** No code exists. The repository holds documents only | [RAN `git ls-files` -> 40 tracked files, all documents; no `Cargo.toml`, no `package.json`, 2026-10-28] |
| Criticality | **Ordinary** (business). Not safety-related. Not regulated. The user's word "ordinary" is read as the kit's "business" class | User, 2026-10-28 (FIND-001); ASM-S3. Whether the loopback web server (NFR-7) counts as security-critical for gate Q8 was **not answered**; treated as the higher tier until it is (FIND-016) |
| Coding standard in force | **SEI CERT**, together with the **standard static checks of Rust and TypeScript, with warnings treated as errors.** The kit's CERT coverage lists C, C++ and Java. It lists no Rust or TypeScript rule set, so how CERT applies to this code is open (FIND-014) | User, 2026-10-28 (FIND-001); ASM-S1, ASM-S2 |
| Process authority | **G0:** no approver. **G1** (one existing component, architecture intact): the **system-architect agent**, as a desk review, recorded as a desk review. **G2 and G3** (anything wider): **Brian Prascher**. A G3 change also needs an independent reviewer; **none is named yet** (FIND-016) | User, 2026-10-28 (FIND-002); `swe-governance` §4 rule 4 |
| Upstream artifacts | requirements.md v1.0 Amendment 4 (CR-1 to CR-6); architecture.md v1.0 amendment 4 (D-1 to D-37); functional_decomp.md v1.0 (baselined 2026-10-28); docs/use_cases UC-1 to UC-29 v1.0 (baselined 2026-10-28); test-strategy.md 1.0 Amendment 4; SRR-SFR-readiness.md. No separate SRS, ADR files or QAS artefacts: decisions are in architecture §11, and the measurable drivers are in the NFRs and architecture §4.8 | [OBS docs/] |
| Environment limits | Development host: macOS 26.7.1 on arm64. **No Intel Mac is available and none is expected soon** (user: "we won't be able to test that requirement anytime soon"); Intel-only verification can't be run (FIND-015). The Stockfish build needs the Internet once (architecture §8.1); the shipped app does not. `cargo-deny`, `cargo-fuzz`, `cargo-mutants`, `proptest`, Playwright and similar tools named in architecture §9 were not checked for presence | User, 2026-10-28 (FIND-009); [RAN `sw_vers`, `uname -m`] |
| Existing conventions | No code. Commit messages in the history are short imperative sentences ("Add use cases and functional decomposition", "Amend requirements and architecture"). Document style follows the requirements and architecture files (IDs `PREFIX-n`, tables, Mermaid) | [RAN `git log --format=%s`] |
| Definition of done | The quality gates of test-strategy.md: all unit and integration tests green, line coverage of at least 90 % for the rules, Elo, clock, policy and parser modules, mutation testing on Elo and mode policy, licence audit and lint in CI, both architectures building and passing. Plus the `swe-governance` §11 checklist | [OBS test-strategy.md "Quality gates"] |

## 2. Decisions recorded from the user (2026-10-28)

The user answered the readiness questions in the session on 2026-10-28. The answers are copied as given, with what each one settles.

| Finding | Question | User's answer | Recorded as |
|---|---|---|---|
| FIND-001 | Criticality class and coding standard | "ordinary, not safety or regulated; use CERT as the coding standard and use standard Rust and TS checks with warnings treated as errors" | Criticality and coding-standard rows above; requirements Decision Log CR-6.1; architecture §8 |
| FIND-002 | Who approves design changes | "yes, SA agent approves changes inside a component, I approve anything else" | Process authority row above; requirements Decision Log CR-6.2 |
| FIND-003 | Does approval of the use cases confirm the systems engineer's earlier interpretations | "yes" | Requirements Decision Log CR-6.3; architecture Q-A18 answered |
| FIND-009 | Is an Intel Mac available for the platform matrix | "no; we won't be able to test that requirement anytime soon" | Environment limits row above; requirements Decision Log CR-6.4; consequence raised as FIND-015 |

## 3. Gate view for design work

Classification follows `swe-governance` §4: take the highest tier any question gives.

| Work | Tier | Approver | Note |
|---|---|---|---|
| Tests only, documents only, formatting | G0 | none | Self-check against the guardrails and run the tests |
| SDD and implementation inside one crate with the §3.2 dependency edges and §5.7 contracts unchanged | G1 | System-architect agent (desk review) | The review is recorded as a desk review. It never satisfies G2 or G3 |
| A change that touches two crates, an interface, a dependency edge, data ownership, a protocol or an ADR | G2 | Brian Prascher | Needs at least two options, consumer impact, and a rollback plan |
| Irreversible work: data migration, a stored-format change (journal, archive `format_version`, schema), a public command or exit-code change after release | G3 | Brian Prascher, with a named independent reviewer | Q8 of the gate applies whatever the criticality class. No independent reviewer is named yet (FIND-016) |
| Host/Origin checks (NFR-7), file modes, anything else that could be called security-critical | **G3 until the user says otherwise** | Brian Prascher | Rule: when unsure between two tiers, take the higher (`swe-governance` §4) |

## 4. Assumptions of this profile

| ID | Assumption | Validation / owner | Impact if wrong |
|---|---|---|---|
| ASM-S1 | "The standard Rust and TS checks" means: for Rust the compiler warnings, `clippy` and `rustfmt --check`; for TypeScript the compiler in strict mode, ESLint with the TypeScript plugin, and a formatter. These are the checks the kit's language table lists | User confirms when CI is set up (step 1). Owner: software engineer | Wrong tool set; the CI job is changed, not the design |
| ASM-S2 | "Warnings treated as errors" applies to the compiler, to the linters and to the CI build, and a suppression needs a written reason in the code | User confirms at the first CI run | The build would be stricter or looser than intended |
| ASM-S3 | "Ordinary" means the kit's "business" criticality | User may correct | None for design; it would change review depth only |
| ASM-S4 | The coding standard applies to code written for this project, not to third-party code (Stockfish, crates, npm packages). Build and packaging scripts follow it for whichever language they are written in | User may correct | Scope of reviews |
| ASM-S5 | The architecture already uses the ID `GATE-005` for its own review tier (§11.3, tier R1). Software design gates (G0 to G3) are recorded in `docs/engineering/gates/` and numbered from `GATE-101` so no ID is reused | Software engineer, at the first gate | A renumbering of gate files |

## 5. Not done

No gate has been recorded yet, because no design work has started. No build or test baseline exists, because there is no code. The engagement profile does not approve any design.
