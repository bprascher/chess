# Step-2 dependency findings: decision packages (F3, F4, F5, F7) and recorded decisions (F1, F2)

| Field | Value |
|---|---|
| Document | docs/engineering/step2-deps-findings-packages.md, v0.1 DRAFT, 2026-11-09 |
| Task | chess-a0r.3.2 (baseline B4). **Stays open until Brian Prascher rules on F3, F4, F5, F7.** |
| Author | System-architect agent, Architect stance. It proposes; it approves nothing. No Review Board pass |
| Input | `dependency-lookup-step2.md` v0.2 (evidence tags `[OBS] [RAN] [INF] [UNV]` as there; S2-E-numbers refer to its section 1) |
| Gate | Every item below touches a dependency edge, the dependency policy or a CI gate: **G2, Brian Prascher** (SWE-001 section 3). Tier R1: all choices are two-way doors before code exists |
| Decider | Brian Prascher. Nothing here is accepted until he says so |

Read order: section 1 (what is already decided), sections 2 to 5 (four packages), section 6 (a one-page summary), section 7 (records to enter).

## 1. Recorded decisions: F1 and F2 (2026-11-09, G2, Brian Prascher, relayed by the coordinator)

| Finding | Decision | What follows | Residual (not a new decision) |
|---|---|---|---|
| **F1** TypeScript 7.0.2 and `typescript-eslint` 8.72.1 cannot run together | **Accepted: TypeScript 7.0.2 through the vendor side-by-side route** (DL2-8 A): `typescript` = `npm:@typescript/typescript6@6.0.2` for `typescript-eslint`, `@typescript/native` = `npm:typescript@7.0.2` for `tsc` | Amends the §3.4 TypeScript, ESLint and `typescript-eslint` rows and adds `@typescript/typescript6` (P4, P2) | (1) Exact pins for both aliases in `ui/package.json`. (2) A CI line that prints both compiler versions. (3) Revisit when `typescript-eslint` supports TypeScript 7.1 or later. (4) `tsc6` printed 6.0.3 in S2-E15 while the package is 6.0.2; the reason was not checked `[UNV]`. (5) Lint sees the TS 6 API, so a TS 7-only error is caught by `tsc`, not by lint. Software engineer implements under chess-dfi |
| **F2** `npm audit` fails on the baseline ESLint (`minimatch`, dev-only) | **Accepted: gate on `npm audit --omit=dev --audit-level=low`; the full-tree audit is a non-gating report** (DL2-9 A). This decides bead **chess-dfi** | P8: `ci/check.sh` and `ui/package.json` change; the software engineer edits them | (1) `VULN-001` (`minimatch` via ESLint, proposed `not_affected`, `code_not_reachable`) still needs a VEX entry; whether his F2 decision also accepts that triage was not stated, see finding S-3. (2) Re-check trigger: ESLint depends on `minimatch >= 11.0.1`. (3) Where the full-tree report lands (a CI artifact). (4) ESLint 10.14.0 not audited `[UNV]`. (5) test-strategy line 88 could say "production tree" |

## 2. F3: `deny.toml` fails on the first `expectrl` (`unmaintained`)

**Ask.** Choose how the repository `deny.toml` treats the unmaintained-crate advisory RUSTSEC-2026-0441 (`pin-utils`), so that `cargo deny check` can be green once `expectrl` is a dev-dependency. This is the open half of DL2-10 ("`unmaintained`: (b) or (c)") and the consequence of DL2-11 A (keep `expectrl`), both of which Brian accepted.

**Context.** Dev-only crate, no vulnerability, upstream archived 2026-10-01, "no safe upgrade is available". `cargo audit` shows it as a warning; `cargo deny` 0.20.2 fails on it by default `[RAN S2-E6, S2-E7]`.

**Evidence.** Chain `expectrl` 0.9.0 → `nix` 0.26.4 → `pin-utils` 0.1.1 `[RAN S2-E9]`. Scope setting tested three ways: `unmaintained = "workspace"` and `"none"` give `advisories ok`; `"transitive"` and the default fail `[RAN S2-E18]`. The alternative PTY tool `portable-pty` 0.9.0 has no advisory at the DB date (26-crate tree, last release 2025-02-11, upstream activity `[UNV]`) `[RAN S2-E11]`. That an `ignore` entry can carry an expiry date inside `deny.toml` itself is `[UNV]`; I believe `ignore` takes an id and a reason only.

| Option | What it does | For | Against |
|---|---|---|---|
| A. `[advisories] unmaintained = "workspace"` | Only unmaintained crates the workspace depends on directly fail the check | One line; no per-advisory upkeep | Silences every future transitive unmaintained crate in all trees, **including production ones** (`cargo audit` would still warn but does not gate) |
| B. `ignore` entry for `RUSTSEC-2026-0441` with a written reason, plus a `VULN-004` record with a revisit date | Exactly one advisory is waived; any other unmaintained crate still fails | Narrow; visible; matches test-strategy ("Decision Log records why") | One entry to maintain. It waives the advisory for every crate, not only dev ones (`cargo-deny` has no dev-only scope `[INF]`). Expiry is a convention in the record, not enforced `[UNV]` |
| C. Replace `expectrl` by `portable-pty` + `vt100` plus an own wait-for-text helper (DL2-11 B) | No unmaintained crate in the tree | No waiver | Reverses an accepted decision; the project writes and tests a helper; the replacement's maintenance is `[UNV]` |
| D. Do the CLI end-to-end tests without a PTY crate (spawn the binary, pipes only) | Spawns the binary with pipes | No tool | Cannot test raw mode, the alternate screen or key handling, which E2E-CLI-01 and -02 need `[INF]` |

**Recommendation: B.** Confidence **Medium-High**. This differs from the lead choice in v0.1 DL2-10 (which put A first): A weakens the supply-chain gate for the shipped product to excuse one dev tool, and B costs one reviewed line. If Brian prefers zero deny.toml exceptions, C is the next choice, at the price of reversing DL2-11.

**Consequences.** `deny.toml` gains one `ignore` entry with a reason (software engineer, at the first dev-dependency entry of `chess-cli`); `VULN-004` is entered as "accepted informational warning" with a re-check at the CLI end-to-end task and again at release; the revisit trigger is `expectrl` or `ptyprocess` leaving `nix` 0.26 or the advisory getting a fix. If A is chosen instead, write the reason into the Decision Log, because the setting hides the same class of finding in production crates.

**Who and what gate.** Brian Prascher, G2 (licence and advisory policy). Software engineer edits the file after the ruling.

## 3. F4: `deny.toml` changes before desktop or ZIP code

**Ask.** DL2-10 settled the content (add `Zlib`; five `MPL-2.0` exceptions; two Apple targets). Rule on the **form and timing** of landing it, and on the two items DL2-10 left out (the unused-allow tidy, the licence notice).

**Context.** With the current allow list `cargo deny check licenses` rejects seven crates: `Zlib` (`foldhash`, `zlib-rs`) and `MPL-2.0` (`cssparser`, `cssparser-macros`, `dtoa-short`, `selectors`, `option-ext`) `[RAN S2-E7 (a)]`. All seven are in the macOS tree. Without `[graph] targets` the Linux GTK stack adds `proc-macro-error` and `glib` findings and an eighth licence rejection, `target-lexicon` (licence not read) `[RAN S2-E7 (b)]`. Whether the MPL crates are linked into the binary or only run at build time is `[UNV]`; `option-ext` is build-time only by the tree `[INF]`.

**Evidence.** Section 4.1 of the lookup. The MPL-2.0 text names GPL as a "Secondary License", so the combination with a GPL-3.0-or-later project is a reading of names, not legal advice `[INF]`. Where the notices go (NFR-6, §8.2) is not designed in any document `[UNV]`.

| Option | What it does | For | Against |
|---|---|---|---|
| A. One G2 change, all of DL2-10 at once, at the first dependency entry (`chess-store`) | `Zlib` allowed; five `MPL-2.0` exceptions; `targets` set; unused `BSD-3-Clause`/`Unicode-3.0` warning cleared | One review, one Decision Log entry; `deny.toml` is correct before any crate arrives | Exceptions name crates that no probe in the real workspace has yet (they are only in the scratch lock) |
| B. Stage it: each licence or exception lands with the crate that needs it (`Zlib` with `zip`, `MPL-2.0` with `tauri`) | Same end state | Every exception is next to its reason | Several small G2 changes; targets and `unmaintained` still need a first home |
| C. Allow `MPL-2.0` globally instead of five exceptions | Shorter | No upkeep | A new MPL crate arrives unseen; the project's notice duty (NFR-6) then has no trigger |

**Recommendation: A**, with the exceptions form (DL2-10 (b)) kept as accepted. Confidence **Medium-High**. Add the notice question as its own task for packaging (chess-a0r.4.x), not as part of this change. The MPL list must be re-checked against the real `Cargo.lock` when `desktop/src-tauri` gets its first entry; the scratch lock stands in for it (ASM-DL2-2).

**Consequences.** `deny.toml` edit by the software engineer; Decision Log entry; `ci/check-crate-edges.mjs` and the `bans` list are not exercised by this lookup (F18), so run them at the same change.

**Who and what gate.** Brian Prascher, G2. Rule with F3, because both change `[advisories]` and `[licenses]` in one file.

## 4. F5: crates and packages no document names

**Ask.** DL2-3, DL2-4 and DL2-13 accepted the substance: the crates and packages exist, are needed and are proposed. Rule on **how the admission is recorded**, and on one item DL2-13 did not list: `serde` as a direct dependency of `chess-cli`.

**Context.** Needed but unnamed in §3.4: `base64` 0.23.1, `getrandom` 0.4.3, `tempfile` 3.27.0 (dev), `tauri-build` 2.7.4, `@types/react` and `@types/react-dom` 19.3.0, `@types/node` 26.6.6, `@eslint/js` 10.0.1, `@testing-library/dom` 10.4.2, `@typescript/typescript6` 6.0.2. Each is a G2 edge (SWE-001 section 3). `tempfile` is already in the step-1 lock through `proptest`; `serde`, `libc`-class crates are in every tree.

**Evidence.** All versions exist, are not yanked, were published on or before 2026-10-28 and pass `cargo deny` with the current allow list `[RAN S2-E2, S2-E7]`. `base64` has no standard-library equivalent `[INF]`. `uuid::Builder::from_random_bytes` compiles without `v4` `[RAN S2-E17]`. SDD-desktop-tauri §3.4 lists `serde`; SDD-cli §3.4 does not, though the CLI reads `config.toml` into structs through `toml` `[INF]`.

| Option | What it does | For | Against |
|---|---|---|---|
| A. One §3.4 amendment now: add every DL2-accepted row with its version, date and licence (P2, P4), and a G0 note naming SDD text to fix (SDD-cli lists `serde`; SDD-app names `getrandom` and `base64`) | The architecture table is complete before any `Cargo.toml` | One Decision Log entry; the first dependency entry has nothing to argue about; the doc stays the single source | Large table edit in a baselined document, so a RELEASE_NOTES entry |
| B. Admit each at its first `Cargo.toml` entry, with a Decision Log line per crate | No big edit | Evidence is fresh at use | 10+ small G2 round trips; the §3.4 table lags the code |
| C. A, but leave `serde` for `chess-cli` out and have the CLI parse `toml::Table` or take config types from `chess-app` | No new CLI edge | Tighter edge list | Hand-written config extraction; `serde` is already admitted and in the tree, so the edge saves nothing `[INF]` |

**Recommendation: A, and admit `serde` (derive) as a direct dependency of `chess-cli`** (Brian to confirm that last point, it was not in DL2-13). Confidence **High** for the admission in substance, **Medium** for the CLI `serde` edge. The first-entry check (S1 `json_valid`, `cargo tree -e features`) still happens at each crate's task.

**Consequences.** Architecture §3.4 amendment (systems engineer or architect text, Brian approves); SDD-cli §3.4 and SDD-app §4.4 G0 wording fixes; `ci/check-crate-edges.mjs` allow-edges updated at the first entry.

**Who and what gate.** Brian Prascher, G2 (dependency edges). The SDD wording edits are G0/G1.

## 5. F7: "Playwright component mode" has no package at the baseline

**Ask.** DL2-12 A (accepted) replaces CT-UI-12's "Playwright component mode" with Vitest browser mode. It was accepted at **Low-Medium** confidence and neither package was run. Rule on whether to **close F7 on paper or after a small spike**.

**Context.** `@playwright/experimental-ct-react` stops at 1.62.1 (2026-07-30); no 1.64.1 exists. The baseline Playwright runner is `@playwright/test` 1.64.1 (the §3.4 text says "Playwright 1.64.1" without a package name). `@axe-core/playwright` 4.14.0 needs `playwright-core`. `@vitest/browser-playwright` 5.0.5 is on the same release train as `vitest` 5.0.5 `[RAN S2-E3]`.

**Evidence.** Registry metadata only `[RAN]`. Whether Vitest browser mode drives a React adapter test with Playwright 1.64.1, and what API it uses, is `[UNV]`. Playwright browser binaries are downloaded outside `package-lock.json`, so outside `npm audit` (F20).

| Option | What it does | For | Against |
|---|---|---|---|
| A. Accept DL2-12 A on paper; amend CT-UI-12 wording (G1 in SDD-ui) | Vitest browser mode, Playwright 1.64.1 | No delay | A wrong guess is found at the first UI task, after other UI work started |
| B. A, plus a timeboxed spike (about 60 minutes, at the first `ui/` dependency entry): one adapter component test in Vitest browser mode, with the fallback B1 or C below written down in advance | Turns `[UNV]` into `[RAN]` before CT-UI-12 is designed against it | Cheap; the door is two-way | Spike needs a network fetch of browsers; small extra task |
| B1. `@playwright/experimental-ct-react` 1.62.1 with Playwright 1.62.1 | Matches the SDD wording | Two lines behind the baseline; "experimental" | Contradicts the §3.4 baseline Playwright |
| C. Drop CT-UI-12 into an E2E test in `@playwright/test` against the built page | No extra package | Loses component isolation for the transport adapter |

**Recommendation: B.** Confidence **Medium**. A is acceptable if Brian wants no spike; the cost of being wrong is rework of one test file, not a design change. Revisit trigger: an `@playwright/experimental-ct-react` release at the baseline version.

**Consequences.** SDD-ui CT-UI-12 wording and §3.4 (G1 text edit by the SDD owner); §3.4 names `@playwright/test` 1.64.1 and `@vitest/browser-playwright` 5.0.5; a spike task or an acceptance line on the first `ui/` bead; the Playwright browser download enters the release checklist (F20).

**Who and what gate.** Brian Prascher, G2 for the package choice (already accepted as DL2-12 A, so the open decision is only the spike); G1 desk review for the SDD wording.

## 6. Summary for Brian

| Item | Ask | Recommended | Confidence |
|---|---|---|---|
| F3 | How to handle `pin-utils` unmaintained | B: one `ignore` entry with reason and revisit | Medium-High |
| F4 | Form and timing of `deny.toml` changes | A: one change at the first dependency entry | Medium-High |
| F5 | How to record unnamed admissions; `serde` for the CLI | A: one §3.4 amendment; admit the CLI `serde` edge | High / Medium |
| F7 | Spike before closing CT-UI-12 | B: 60-minute spike, fallback pre-agreed | Medium |
| DL2-14 | Confirm "no `libc`/`rustix`" (spike: `std` suffices for the store) | Confirm; leave the CLI SIGTSTP question to the CLI task | Medium-High |

F3 and F4 can be ruled together. F5 does not block the first store entry if A is chosen; F7 does not block anything before the first `ui/` entry.

## 7. Proposed records (for the systems engineer to enter; no baselined document was edited here)

Numbering assumes the highest existing IDs are D-100 and CR-17 `[OBS architecture.md, requirements.md, RELEASE_NOTES.md grep, 2026-11-09]`. The systems engineer should recheck before use. "Pending" rows are not to be entered until Brian rules.

### 7.1 Decision Log (requirements.md) and architecture §11 rows

| Proposed ID | Gate | Text (one line) | Covers |
|---|---|---|---|
| CR-18 (requirements Decision Log) | G2 | Brian Prascher, 2026-11-09 (relayed by the coordinator, no verbatim quote): accepted the recommendations of dependency lookup step 2 v0.2, DL2-1 to DL2-13 and DL2-15, F1 and F2. Lookup document not accepted as a whole; task chess-a0r.3.2 open | all below |
| D-101 | G2 | Admit as direct dependencies at the pinned versions and feature sets of lookup step 2 section 5A: `zip` 9.1.3 (`default-features = false`, `deflate-flate2-zlib-rs`), `sha2` 0.11.1, `base64` 0.23.1, `getrandom` 0.4.3 (UUIDv4 through `uuid::Builder::from_random_bytes`, `uuid` stays `v5`+`serde`), plus the DL2-13 batch (`tempfile` 3.27.0, `tauri-build` 2.7.4 and the npm packages listed there) | DL2-1, -2, -3, -4, -13, -15 |
| D-102 | G2 | `tokio` feature sets per crate as in DL2-5; `tracing-subscriber` 0.3.23 (`fmt`, `std`, no `env-filter`); `tracing-appender` dropped from the admitted list; `RotatingFile` per ALG-APP-14 (amends the §3.4 row and SDD-app §4.4) | DL2-5, DL2-6 |
| D-103 | G2 | TypeScript 7.0.2 with `typescript-eslint` through the vendor side-by-side route (`@typescript/typescript6` 6.0.2); `npm audit --omit=dev --audit-level=low` is the CI gate, the full-tree audit is a non-gating report with `VULN-` records. Decides bead chess-dfi | DL2-8, DL2-9, F1, F2 |
| D-104 | G2 | `deny.toml` policy: allow `Zlib`; `MPL-2.0` by `[[licenses.exceptions]]` for the five named crates; `[graph] targets` the two Apple targets. `unmaintained` handling is **pending** (F3) | DL2-10 |
| D-105 | G2 | Test tools: keep `expectrl` 0.9.0 + `vt100` 0.18.1 (with the F3 handling); component tests through `@vitest/browser-playwright` 5.0.5 with Playwright 1.64.1; `@testing-library/react` 16.3.3, `@testing-library/dom` 10.4.2, `jsdom` 30.1.3 | DL2-7, -11, -12 |
| Pending | G2 | DL2-14 (no `libc`/`rustix`), F3, F4, F5 (incl. CLI `serde`), F7 | `step2-deps-findings-packages.md` |
| Also (G0) | G0 | D-47 note: `rand` and `signal-hook` are rejected as direct dependencies and appear transitively (F12) | P6 |

### 7.2 RELEASE_NOTES lines

- Heading proposal: "Step-2 dependency lookup, decisions recorded (CR-18): 2026-11-09".
- "Brian Prascher (G2) accepted DL2-1 to DL2-13 and DL2-15 of `docs/engineering/dependency-lookup-step2.md` v0.2, and F1 and F2 (TypeScript 7.0.2 via the side-by-side route; npm audit gates the production tree). Relayed by the coordinator; no verbatim quote."
- "Not decided: DL2-14 (recommendation: no `libc`/`rustix`), F3, F4, F5, F7. Task chess-a0r.3.2 stays open. No `Cargo.toml`, `package.json`, `deny.toml` or `ci/` file was changed."
- "Documents: `dependency-lookup-step2.md` 0.1 to 0.2 (DRAFT, working record); `step2-deps-findings-packages.md` 0.1 (DRAFT)."
- "Follow-ups: chess-dfi implementation (software engineer); SDD-app §4.4 (`tracing-appender`); SDD-ui CT-UI-12 wording."

## 8. Findings of this task

| # | Sev | Finding | Needed from |
|---|---|---|---|
| S-1 | Medium | DL2-10's `unmaintained` row was a recommendation with two options ("(b), or (c)"); acceptance did not choose one. I carried it into F3 instead of assuming (b) | Brian (F3) |
| S-2 | Medium | DL2-14 had no recommendation in v0.1 and was accepted "as recommended". It is recorded as unconfirmed. The spike left the CLI SIGTSTP raise untested, so "no `libc`/`rustix` anywhere" is not yet supported for `chess-cli` | Brian; software engineer at the CLI task |
| S-3 | Low | Whether the F2 decision also accepts the `VULN-001` triage (`not_affected`, `code_not_reachable`) was not stated | Brian |
| S-4 | Low | DL2-12 A was accepted at Low-Medium confidence on registry metadata alone (F7) | Brian |
| S-5 | Low | `tsc6` printed 6.0.3 against package 6.0.2 in S2-E15, unexplained | Software engineer at chess-dfi |
| S-6 | Info | The spike ran `crc32fast` 1.5.4; step 2 pins 1.5.3. Both pass the check value on the evidence of the two runs; the S1 `json_valid` item is still open | Software engineer, first store entry |

## 9. Not verified, not analysed

- Decision text of 2026-11-09 comes through the coordinator; no verbatim quotation.
- No new registry, advisory or build run was made for this document; all facts are from `dependency-lookup-step2.md` v0.1 and the spike file, with their original tags. That `cargo-deny` has no per-entry expiry and no dev-only scope is from my knowledge of the tool, not run `[UNV]`.
- D-n and CR-n numbers are the next free ones as of the grep above; the systems engineer must confirm them.
- Not analysed: legal reading of MPL-2.0 or Zlib, notice placement, an Intel build, Playwright browser binaries, SBOM.
