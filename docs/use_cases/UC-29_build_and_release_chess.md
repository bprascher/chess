# UC-29: Build and Release Chess

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-29_build_and_release_chess.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The release engineer wants to build a version of Chess and prove, before it is used, that it meets the release-time requirements: the bundled engine is the pinned one, licences are compliant, and the app works on every macOS release that is supported at that date.

**Scope:** The **build and release enabling system** (build scripts, the continuous-integration checks, the release checklist and the install documentation), not the Chess application at run time. This is a second scope, separate from the other 28 use cases, which are black-box uses of the running application. It exists because NFR-6, RISK-1 and parts of PLAT-2 and IF-4 are verified at release, not by a User of the running system (README UG-1, UG-2; functional_decomp FG-1, FG-10). The architecture context diagram (§2) shows run-time blocks only, so this scope has no block there (see docs/handoff_se_to_architect.md).

**Level:** User goal

**Primary Actor:** Release engineer. This is the project owner in the build role, who is the same person as the User of the other use cases (ASM-1, one person per Mac account), acting in a different role.

**Supporting Actors:**
- Stockfish release source: the published Stockfish 19 release and its checksum, reached at build time only. Running the app never uses it (PLAT-3).
- Build and continuous-integration environment: runs the builds, the licence audit and the test suites.

**Stakeholders and Interests:**
- User: an app that runs offline on the Mac they have, on Intel or Apple silicon (PLAT-2, PLAT-3).
- Project owner: GPL-3.0-or-later compliance of the engine, code and bundled assets (NFR-6, ENG-1); computer Players that keep the strength they were calibrated for (IF-4, ENG-2).

**Preconditions:** The source for the version is ready. The pinned Stockfish release is named in IF-4. The release checklist exists.

**Minimal Guarantees:** A failed step stops the release and leaves the previous release unchanged. Nothing is distributed.

**Success Guarantees:** `Chess.app` contains the desktop app, the `chess` command and the pinned engine, universal for Intel and Apple silicon, ad-hoc signed. The licence audit, the platform matrix and the engine facts are recorded in the release checklist. The install documentation is current.

**Trigger:** The release engineer decides to cut a version.

**Main Success Scenario:**

1. Release engineer starts the release procedure for the version.
2. Build environment downloads the pinned Stockfish release, verifies its checksum, and checks the architectures it holds (IF-4, TBC-7). It bundles the engine.
3. Build environment builds the universal `Chess.app` with the desktop app, the `chess` command and the engine, and stamps one build version on all of them (PLAT-1, PLAT-2, PLAT-5).
4. Build environment audits the licences of the code dependencies and every bundled asset, and records the Stockfish GPL version read from its shipped `COPYING` file (NFR-6, ENG-1, TBC-9).
5. Release engineer records the macOS releases that are current on this date and which of them run on Intel (PLAT-2, RISK-1).
6. Build environment and release engineer verify the app on each recorded release, and on an Intel Mac for each release that supports Intel (PLAT-2).
7. Release engineer checks whether a newer FIDE Laws edition is in force (RULE-1 release checklist).
8. Release engineer updates the install and run documentation (NFR-5, PLAT-6).
9. Release engineer records the result in the release checklist and releases the version.

**Extensions:**

- **2a.** The checksum differs, or the archive lacks a required architecture:
  - 2a1. The build fails with a message (IF-4). If the engine does not hold both architectures, the separate builds are bundled instead (TBC-7). Use case resumes at step 2 or fails.
- **2b.** The engine release differs from the one pinned:
  - 2b1. A Decision Log entry and a new calibration run precede the pin (IF-4, ENG-2). The calibration runs on an isolated store (RISK-2, F-3.9). Use case resumes at step 2.
- **4a.** The audit finds an incompatible or unknown licence:
  - 4a1. The release stops until the dependency or asset is replaced or its licence is cleared (NFR-6). Use case resumes at step 4.
- **5a.** No supported macOS release runs on Intel any more:
  - 5a1. The Intel build is kept only for the releases that support it, or the change is raised as a change request against PLAT-2 (RISK-1). Use case resumes at step 6.
- **6a.** The app fails on a recorded release:
  - 6a1. The failure is logged and fixed, or the release is held. Use case resumes at step 3.
- **7a.** A newer FIDE Laws edition is in force:
  - 7a1. A change request is raised against RULE-1, RULE-2 and RULE-4 (requirements RULE-1 note). The release continues unless the request blocks it.

**Technology and Data Variations:**
- Building may download dependencies; running never does (PLAT-3).
- The desktop packaging is ad-hoc signed, with no Developer ID and no notarisation (PLAT-1, TBC-10).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- NFR-6: licence audit in CI, covering code dependencies and every bundled asset, and recording the Stockfish GPL version.
- RISK-1: handling is carried out at step 5.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | Once per app release (TBD: no schedule exists) |
| Performance target | None of its own. |
| Channels to actors | Shell, CI, the release checklist and the documentation. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | None. Produces the build that UC-1 installs. |
| Open issues | None specific. The blocks for this scope are in architecture §2.1 (D-31, 2026-10-28). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | NFR-6, RISK-1, PLAT-2, IF-4, ENG-1, ENG-2, NFR-5, PLAT-1, PLAT-5, PLAT-6, RISK-2, RULE-1 |
| Functions (functional_decomp v1.0) | F-8.6, F-8.7, F-8.8, F-8.9, F-8.10, F-3.9 |
| Architecture | See requirements allocation, architecture 10.1 (`packaging/`, CI, `third_party/`). |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-28 | First draft. Added to close UG-1 and UG-2 (enabling-system scope). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
