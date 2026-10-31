# SWE-001 Engagement Profile

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/engineering/SWE-001-engagement-profile.md | 1.1 | 2026-10-30 | **Active.** Created from the user's answers to the readiness check (FIND-001, FIND-002, FIND-003, FIND-009); updated 2026-10-30 with the answers to FIND-006, FIND-014, FIND-015 and FIND-016 | Software engineer role (Claude), for Brian Prascher | Brian Prascher (user) gave the answers recorded below. The profile itself is not a design approval |

This is the first file of the software engineering record. It follows the `swe-governance` profile template. Entries marked `ASM-S` are assumptions of this profile, with a validation plan. They are not decisions.

## 1. Profile

| Field | Value | Source / ASM |
|---|---|---|
| System / repo | Chess: an offline, single-machine chess application for macOS. Rust core, terminal UI, local web UI and Tauri desktop app. Repository `/Users/brianprascher/Projects/chess`, branch `main`. | [OBS README.md] |
| Task type | Design. Detailed design (IEEE 1016 SDDs) and implementation, in the order of architecture §13: step 1 `chess-rules` and `chess-domain`, step 2 store, engine, core, CLI, server, step 3 GUI, desktop, packaging | [OBS architecture.md §13] |
| Languages and versions | **Rust 1.99.1, edition 2024** (workspace `rust-version = "1.99"`) for the core, CLI, server and Tauri shell. **TypeScript 7.0.2** on **Node.js 26.13.0 LTS** for the GUI. Every open-source component is at its latest stable version as of the baselined requirements (2026-10-28); the full list is [architecture §3.4](../architecture.md#34-baseline-versions-cr-73-find-006). Stockfish 19 is a pinned third-party C++ binary, not project code. The language of the build and packaging scripts (`third_party/stockfish/`, `ci/`, `packaging/`) is not decided | User, 2026-10-30 (FIND-006: "use the latest version of all open source components and languages as of the baselined requirements"); [RAN registry queries, 2026-10-30]; ASM-S4 |
| Toolchain installed on the development Mac | `cargo 1.89.0`, `rustc 1.89.0`, `rustfmt 1.8.0-stable`, `clippy 0.1.89` (toolchain `stable-aarch64-apple-darwin`); `node v26.12.1`, `npm 11.20.0`. No `tsc` on the path (it would come with the project's own dependencies). **Both the Rust and the Node toolchains are older than the baseline versions** (Rust 1.89.0 cannot build `shakmaty` 0.30.2, which needs 1.97; Node 26.12.1 is a Current release, and 26.13.0 is the LTS). **Update both before step 1** (`rustup update stable`; install Node 26.13.0) | [RAN `cargo --version`, `rustc --version`, `rustfmt --version`, `cargo clippy --version`, `node --version`, `npm --version` -> versions above, 2026-10-28; versions needed: crates.io `rust_version` fields, 2026-10-30] |
| Build and test commands | **None.** No code exists. The repository holds documents only | [RAN `git ls-files` -> 40 tracked files, all documents; no `Cargo.toml`, no `package.json`, 2026-10-28] |
| Criticality | **Ordinary** (business). Not safety-related. Not regulated. **Not security-critical:** v1 runs only locally on the user's own Mac. The only security requirements are security best practice and avoiding known CVEs. The user's word "ordinary" is read as the kit's "business" class | User, 2026-10-28 (FIND-001) and 2026-10-30 (FIND-016: "the application will only run locally on my machine for v1; there is no criticality or security requirements to enforce for v1 other than security best practice and avoiding known CVEs"); ASM-S3 |
| Coding standard in force | **Best practice informed by SEI CERT**, together with the **standard static checks of Rust and TypeScript, with warnings treated as errors.** CERT publishes rule sets for C, C++ and Java, and none for Rust or TypeScript. For those languages, reviewers apply the *intent* of the matching CERT rules (mapping in §6) and report findings. **No CERT conformance claim is made** for any SDD or review. A suppression of a check needs a written reason in the code. Known-vulnerability scanning of dependencies (`cargo-deny`/`cargo-audit`, `npm audit`) is part of the checks | User, 2026-10-28 (FIND-001) and 2026-10-30 (FIND-014: "use CERT to inform best practice for languages not explicitly covered by CERT"; FIND-016); ASM-S1, ASM-S2 |
| Process authority | **G0:** no approver. **G1** (one existing component, architecture intact): the **system-architect agent**, as a desk review, recorded as a desk review. **G2 and G3** (anything wider): **Brian Prascher**. A G3 change needs no separate independent reviewer: v1 is a local, single-user, non-critical application, so the user's approval stands (FIND-016) | User, 2026-10-28 (FIND-002) and 2026-10-30 (FIND-016); `swe-governance` §4 rule 4 |
| Upstream artifacts | requirements.md v1.0 Amendment 6 (CR-1 to CR-8); architecture.md v1.0 amendment 6 (D-1 to D-37); functional_decomp.md v1.0 (baselined 2026-10-28); docs/use_cases UC-1 to UC-29 v1.0 (baselined 2026-10-28); test-strategy.md 1.0 Amendment 6; SRR-SFR-readiness.md. No separate SRS, ADR files or QAS artefacts: decisions are in architecture §11, and the measurable drivers are in the NFRs and architecture §4.8 | [OBS docs/] |
| Environment limits | Development host: macOS 26.7.1 on arm64. **No Intel Mac is available and none is expected soon** (user: "we won't be able to test that requirement anytime soon"); Intel-only verification can't be run. **Platforms and Mac models without available hardware are tested on a best-effort basis: deferred, reported as "not run", and run if hardware appears** (user, 2026-10-30, FIND-015; test-strategy "Platform availability"). The Stockfish build needs the Internet once (architecture §8.1); the shipped app does not. `cargo-deny`, `cargo-fuzz`, `cargo-mutants`, `proptest`, Playwright and similar tools named in architecture §9 were not checked for presence | User, 2026-10-28 (FIND-009); [RAN `sw_vers`, `uname -m`] |
| Existing conventions | No code. Commit messages in the history are short imperative sentences ("Add use cases and functional decomposition", "Amend requirements and architecture"). Document style follows the requirements and architecture files (IDs `PREFIX-n`, tables, Mermaid) | [RAN `git log --format=%s`] |
| Definition of done | The quality gates of test-strategy.md: all unit and integration tests green, line coverage of at least 90 % for the rules, Elo, clock, policy and parser modules, mutation testing on Elo and mode policy, licence audit and lint in CI, both architectures building and passing. Plus the `swe-governance` §11 checklist | [OBS test-strategy.md "Quality gates"] |

## 2. Decisions recorded from the user (2026-10-28 and 2026-10-30)

The user answered the readiness questions in the session on 2026-10-28. The answers are copied as given, with what each one settles.

| Finding | Question | User's answer | Recorded as |
|---|---|---|---|
| FIND-001 | Criticality class and coding standard | "ordinary, not safety or regulated; use CERT as the coding standard and use standard Rust and TS checks with warnings treated as errors" | Criticality and coding-standard rows above; requirements Decision Log CR-6.1; architecture §8 |
| FIND-002 | Who approves design changes | "yes, SA agent approves changes inside a component, I approve anything else" | Process authority row above; requirements Decision Log CR-6.2 |
| FIND-003 | Does approval of the use cases confirm the systems engineer's earlier interpretations | "yes" | Requirements Decision Log CR-6.3; architecture Q-A18 answered |
| FIND-009 | Is an Intel Mac available for the platform matrix | "no; we won't be able to test that requirement anytime soon" | Environment limits row above; requirements Decision Log CR-6.4; consequence raised as FIND-015 |
| FIND-006 | Which versions of languages and open-source components | "Use the latest version of all open source components and languages as of the baselined requirements; mark resolved after noting baseline versions" | Languages and versions row above; architecture §3.4; requirements CR-7.3 |
| FIND-014 | How CERT applies to Rust and TypeScript | "Use CERT to inform best practice for languages not explicitly covered by CERT" | Coding-standard row above; §6; requirements CR-7.5 |
| FIND-015 | What happens to PLAT-2 now that Intel can't be tested | "Defer test for Intel Mac and other unavailable platforms; update the test strategy to be best effort for platforms that aren't immediately available" | Environment limits row above; test-strategy "Platform availability"; requirements CR-7.6 |
| FIND-016 | Is the loopback server security-critical; who is the G3 reviewer | "The application will only run locally on my machine for v1; there is no criticality or security requirements to enforce for v1 other than security best practice and avoiding known CVEs" | Criticality and process-authority rows above; §3; requirements CR-7.7 |

## 3. Gate view for design work

Classification follows `swe-governance` §4: take the highest tier any question gives.

| Work | Tier | Approver | Note |
|---|---|---|---|
| Tests only, documents only, formatting | G0 | none | Self-check against the guardrails and run the tests |
| SDD and implementation inside one crate with the §3.2 dependency edges and §5.7 contracts unchanged | G1 | System-architect agent (desk review) | The review is recorded as a desk review. It never satisfies G2 or G3 |
| A change that touches two crates, an interface, a dependency edge, data ownership, a protocol or an ADR | G2 | Brian Prascher | Needs at least two options, consumer impact, and a rollback plan |
| Irreversible work: data migration, a stored-format change (journal, archive `format_version`, schema), a public command or exit-code change after release | G3 | Brian Prascher | Q8 of the gate applies whatever the criticality class. The independent-reviewer step is waived: v1 is local and single-user (FIND-016, 2026-10-30). Nothing is released yet, so no format is frozen |
| Host/Origin checks (NFR-7), file modes | Ordinary: G1 inside one crate, G2 across crates | System-architect agent (G1), Brian Prascher (G2) | The user said on 2026-10-30 that v1 has no security-criticality requirement beyond best practice and avoiding known CVEs (FIND-016). The earlier "G3 until the user says otherwise" no longer applies |

## 4. Assumptions of this profile

| ID | Assumption | Validation / owner | Impact if wrong |
|---|---|---|---|
| ASM-S1 | "The standard Rust and TS checks" means: for Rust the compiler warnings, `clippy` and `rustfmt --check`; for TypeScript the compiler in strict mode, ESLint with the TypeScript plugin, and a formatter. These are the checks the kit's language table lists | User confirms when CI is set up (step 1). Owner: software engineer | Wrong tool set; the CI job is changed, not the design |
| ASM-S2 | "Warnings treated as errors" applies to the compiler, to the linters and to the CI build, and a suppression needs a written reason in the code | User confirms at the first CI run | The build would be stricter or looser than intended |
| ASM-S3 | "Ordinary" means the kit's "business" criticality | User may correct | None for design; it would change review depth only |
| ASM-S4 | The coding standard applies to code written for this project, not to third-party code (Stockfish, crates, npm packages). Build and packaging scripts follow it for whichever language they are written in | User may correct | Scope of reviews |
| ASM-S5 | The architecture already uses the ID `GATE-005` for its own review tier (§11.3, tier R1). Software design gates (G0 to G3) are recorded in `docs/engineering/gates/` and numbered from `GATE-101` so no ID is reused | Software engineer, at the first gate | A renumbering of gate files |

## 5. Dependencies and known vulnerabilities (FIND-016)

CI and each release run `cargo-deny` advisories (or `cargo-audit`) and `npm audit`. A known CVE in a dependency that the app uses blocks the release, unless the requirements Decision Log records why it isn't reachable. Versions follow architecture §3.4.

## 6. CERT-informed practice for Rust and TypeScript (FIND-014)

CERT's published standards cover C, C++ and Java (Perl and Android too), and none covers Rust or TypeScript `[UNV: a web search on 2026-10-30 found no SEI CERT Rust or TypeScript standard; the CERT site itself was not read]`. Reviews therefore take the *intent* of the CERT rules and apply it where the language still allows the same mistake. The table is the working mapping. It says what to look for, not that a rule is satisfied.

| CERT area (C / C++ / Java) | Intent | Rust | TypeScript |
|---|---|---|---|
| INT, NUM, FLP (integer and numeric safety) | No overflow, wrap, truncation or lossy conversion | `checked_*`/`saturating_*` arithmetic where overflow is possible; no `as` casts that narrow; `clippy::cast_*` | No `number` for money-like exact values; `Number.isSafeInteger`; no implicit coercion (`strict`, `eqeqeq`) |
| ARR, STR, MEM, EXP (bounds, strings, memory, expressions) | No out-of-bounds access, use-after-free or unchecked results | Safe Rust enforces most; every `unsafe` block needs a `// SAFETY:` reason (`clippy::undocumented_unsafe_blocks`); no `unwrap`/`expect` on external input; indexing with `get` | `noUncheckedIndexedAccess`; no `any`; no non-null `!` on external data |
| IDS (input validation, Java) and STR | Validate and sanitise all external input; no injection | Parse, don't trust: the protocol, PGN, FEN, config and import files are validated at the boundary; parameterised SQL only | Validate every message from the core; no `innerHTML` with untrusted text; the CSP |
| ERR (error handling) | Handle every error; don't leak internals | `Result` everywhere; no swallowed errors; `thiserror` mapped to the §5.5 catalogue | No empty `catch`; typed errors; `no-floating-promises` |
| CON, LCK, VNA, THI (concurrency) | No data races, deadlock or lost updates | `Send`/`Sync` checked by the compiler; no lock held across `.await`; the actor and store-thread model of §4.8 | No shared mutable state across async tasks; `await` ordering reviewed |
| FIO (files) | Safe file creation, permissions and paths | Data dir `0700`, DB `0600`; temp-file-then-rename for writes; no path built from untrusted text | Not applicable (the GUI has no file access) |
| ENV, SIG, POS (environment, signals) | Treat environment and signals as untrusted | `CHESS_DATA_DIR`/`CHESS_CONFIG` validated; signal handling per CLI-5 | Not applicable |
| MSC, DCL, PRE, MET, OBJ (misc, declarations, API design) | No dead or dangerous constructs; careful API design | `clippy::all`, `clippy::pedantic` where practical; warnings are errors | ESLint with `typescript-eslint` strict set; warnings are errors |
| SEC, SER (security, serialization) | Don't deserialise untrusted data unsafely | `serde` with `deny_unknown_fields` on external formats; size limits (TBD-25) | Parse JSON to a validated type before use |

A review states: "reviewed against CERT-informed best practice (SWE-001 §6); findings only; no conformance claim." Where a CERT rule has no Rust or TypeScript equivalent, the review says so instead of marking it satisfied.

## 7. Not done

No gate has been recorded yet, because no design work has started. No build or test baseline exists, because there is no code. The engagement profile does not approve any design.
