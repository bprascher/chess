# Dependency lookup, build step 1 (OQ-17, part 1)

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/engineering/dependency-lookup-step1.md | 0.3 | 2026-11-06 (0.2: 2026-11-05; 0.1: 2026-11-04) | **Accepted by Brian Prascher (G2), 2026-11-06 (CR-15; tracker chess-out); no content change; version stays 0.2.** *Status before 2026-11-06: Draft proposal, reissued (0.2) for Brian Prascher's G2 acceptance. Not accepted.* Task chess-a0r.1.6, baseline B1; reissued under CR-4 task T-A (chess-a0r.1.54.1) after Brian's ruling of 2026-11-05 (chess-a0r.1.49, chess-a0r.1.53) that 0.1 was not accepted as a whole. Written by the system-architect agent (Architect stance). It proposes; it approves nothing | System-architect agent, for Brian Prascher | None claimed. Any crate not already admitted in architecture §3.4 is a **G2** proposal and needs **Brian Prascher** |

**Change log.**

| Version | Date | Change | Acceptance |
|---|---|---|---|
| 0.3 | 2026-11-06 | Itemisation only: section 9 lists the transitive crates of `shakmaty` 0.30.2 that sections 2 and 4 did not name. No vetting conclusion changed. No other section edited, apart from this change log and the header version and date | Brian Prascher accepted the itemisation on 2026-11-06 (task chess-pe1, F-A3). He has not accepted the 0.3 text beyond that |
| 0.2 | 2026-11-05 | See the change note below | Accepted by Brian Prascher (G2), 2026-11-06 (CR-15; chess-out), no content change |
| 0.1 | 2026-11-04 | First issue | Not accepted as a whole (ruling of 2026-11-05) |

**Change note, 0.2 (2026-11-05).** Brian Prascher ruled on 2026-11-05 (OQ-17 part 1, chess-a0r.1.49) that 0.1 is not accepted as a whole, and asked for two changes. Everything else in part 1 is unchanged from 0.1 (same runs, same results, same pins).
1. **Spike item added, not a lookup result.** Whether the bundled SQLite build supports `json_valid` (the store DDL uses it in a `CHECK`) is added as spike item S1 in section 6A and is folded into the OQ-14 spike scope (tracker chess-a0r.3.1). No lookup in this document tested it; it is `[UNV]`.
2. **Fuzzing proposals dropped (D-76: no nightly toolchain ever; `cargo-fuzz`, `libfuzzer-sys` and `arbitrary` not admitted).** Removed: the `cargo-fuzz`, `libfuzzer-sys` and `arbitrary` rows (section 2.3) and the `cargo-fuzz` scan row (section 3); the fuzzing parts of finding F1 (item (c)), E3 and E9; findings F4 and F12 (IDs kept, marked withdrawn); proposals P5 and P7 (IDs kept, marked withdrawn); the matching `[UNV]` bullets. Counts that came from a run that included those crates are labelled where they remain (E4).
3. **Status notes, no content change:** P4 (`cargo-llvm-cov`) and P6 (`cargo-nextest`) were decided after 0.1 by D-76 (2) and D-81. They stay in the document as lookup evidence and carry a status note in section 6.

**Acceptance.** This reissue asks for Brian's G2 acceptance of part 1 (the proposals still marked G2 in section 6). Acceptance is his and is not recorded here.

Scope: the crates used by `chess-rules`, `chess-domain` and the Rust workspace tooling of build step 1 (architecture §13, item 1). Not covered: crates of steps 2 and 3 (`tokio`, `rusqlite`, `axum`, `ratatui`, Tauri, `clap`, ZIP, SHA-256 and the rest of the §3.4 admission list), and all npm packages. Those stay `[UNV]` in §3.4 until their own lookup.

Evidence tags (SWE-001): `[RAN]` a command was executed, `[OBS]` read from a source, `[INF]` inferred, `[UNV]` not looked up. All dates are 2026-11-04 (UTC) unless stated.

## 1. Method and environment

**Rule applied.** Architecture §3.4 and SWE-001 FIND-006: the pin is the highest stable, non-yanked release published on or before the baseline date **2026-10-28**. Where a newer release exists today it is shown, but it is not the pin.

| # | Check | Command (exact) | Result | Tag |
|---|---|---|---|---|
| E1 | Toolchain | `cargo --version; rustc --version; node --version; npm --version; cargo clippy --version; rustfmt --version` | cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13); node v26.13.0; npm 11.20.0; clippy 0.1.99; rustfmt 1.10.0-stable | `[RAN]` |
| E2 | Toolchains and components | `rustup toolchain list; rustup component list --installed` | `stable-aarch64-apple-darwin` (active) and `1.89.0-aarch64-apple-darwin`. **No nightly.** Components: cargo, clippy, llvm-tools, rust-docs, rust-src, rust-std, rustc, rustfmt | `[RAN]` |
| E3 | Cargo tools present | `cargo-deny --version; cargo-audit --version; cargo-nextest --version; cargo llvm-cov --version; cargo-mutants --version; which cargo-insta` | cargo-deny 0.20.2, cargo-audit 0.22.2, cargo-nextest 0.9.128, cargo-llvm-cov 0.9.1 present. **cargo-mutants, cargo-insta not installed** (0.1 also ran `cargo-fuzz --version`: not installed; dropped under D-76) | `[RAN]` |
| E4 | Registry metadata | `curl -s -A "chess-oq17 (<user email>)" https://crates.io/api/v1/crates/<crate>` for 17 crates in the 0.1 run (including `cargo-fuzz`, `libfuzzer-sys` and `arbitrary`, whose rows are removed in 0.2); fields `max_stable_version`, and per version `license`, `rust_version`, `yanked`, `created_at`, `features` | See section 2. Files kept only in the session scratchpad | `[RAN]` |
| E5 | Resolve the step-1 set | A scratch crate (not in the repository) with edition 2024, `rust-version = "1.99"`, exact `=` pins of the section 2 versions, `uuid` features `v5` only, `ts-rs` optional. `cargo generate-lockfile` then `cargo metadata --all-features` | 68 packages besides the probe locked with `--all-features` (69 entries in the lock); about 30 unique packages on the normal and build edges | `[RAN]` |
| E6 | Build check | `cargo check --all-features --all-targets` in the scratch crate on rustc 1.99.1 | `Finished dev profile` with no error | `[RAN]` |
| E7 | Advisories (tree) | `cargo audit --db <scratch>/advisory-db --file Cargo.lock` | RustSec DB at commit `965bf9ad083b58e6340d65829d85217b18be9290` (2026-11-03T21:34:28+01:00), 1403 advisories. 69 crate entries scanned. **No vulnerability, no warning** | `[RAN]` |
| E8 | Advisories and licences (second tool) | `cargo deny --all-features check` with a scratch `deny.toml` (db-path in scratchpad; allow list in section 4) | `advisories ok, bans ok, licenses ok, sources ok`. Warnings: `syn` 2 and 3 both present (via `ts-rs-macros`); one unused allow entry | `[RAN]` |
| E9 | Per-crate advisory history | `ls <advisory-db>/crates/<crate>` for each crate in section 2 | **No advisory file ever** for shakmaty, pgn-reader, serde, thiserror, uuid, ts-rs, proptest, insta, sha1_smol, cargo-deny, cargo-audit, cargo-mutants, cargo-llvm-cov. Files exist for `cargo-nextest` (RUSTSEC-2026-0412) and `rand` (RUSTSEC-2026-0097) | `[RAN]` |
| E10 | Dev-tool dependency scan | For each tool: download `https://crates.io/api/v1/crates/<c>/<v>/download`, then `cargo audit --no-fetch --json` on (a) the published `Cargo.lock` and (b) a fresh `cargo generate-lockfile` with the lock removed | Section 3 | `[RAN]` |
| E11 | Dependency-tree assertions | `cargo tree -i rand` and `cargo tree -e normal,build -i rand` in the scratch crate | `rand 0.10.3` appears through `proptest` under `[dev-dependencies]`; with `-e normal,build` it does not appear (finding F6) | `[RAN]` |

**Limit of E7 and E8.** They read the advisory database as of 2026-11-03/04, which is six days after the baseline date. A result is a point in time. CI and each release re-run them (SWE-001 §5).

## 2. Lookup table: crates in the step-1 build

"Admitted" means architecture §3.4 as amended (AMD-8, D-47). "Pin" is the version to put in the workspace skeleton. MSRV is the registry `rust_version` field; the workspace is `rust-version = "1.99"` and the local rustc is 1.99.1, so every MSRV at or below 1.99 builds. Every row below: pin exists, not yanked, and published on or before 2026-10-28 `[RAN E4]` unless the row says otherwise.

### 2.1 Runtime dependencies

| Crate | Used by (source) | Pin | Published | Licence (registry SPDX) | cargo-deny result | Advisories (crate, E9) | MSRV vs 1.99.1 | Admitted in §3.4 | Tag |
|---|---|---|---|---|---|---|---|---|---|
| `shakmaty` | `chess-rules` (SDD-rules §3.4, D-3) | **0.30.2** (newest, same as §3.4) | 2026-10-07 | **GPL-3.0-or-later** | ok, when `GPL-3.0-or-later` is in the allow list | none | 1.97, ok | Yes, table row with version | `[RAN]` |
| `pgn-reader` | `chess-rules` (SDD-rules §3.4, D-3) | **0.29.1** (newest, same as §3.4) | 2026-10-07 | **GPL-3.0-or-later** | ok, same condition | none | 1.97, ok | Yes, table row with version | `[RAN]` |
| `serde` (feature `derive`) | `chess-domain` (SDD-domain §3.4, D-47 row D-3) | **1.0.229** (newest) | 2026-07-18 | MIT OR Apache-2.0 | ok | none | 1.56, ok | Yes: version in table, feature set `[UNV]` until this lookup | `[RAN]` |
| `thiserror` | `chess-rules`, `chess-domain` (SDD-rules DD error enums, SDD-domain §3.4, architecture §8) | **2.0.21** (newest) | 2026-09-23 | MIT OR Apache-2.0 | ok | none | 1.77, ok | Yes, table row with version | `[RAN]` |
| `uuid`, features `v5` and `serde` | `chess-domain` (SDD-domain §3.4, D-40, D-46, FIND-DOM-003) | **1.29.0** (newest) | 2026-10-23 | Apache-2.0 OR MIT | ok | none | **1.89.0**, ok | **In principle only** (D-47 row D-5); this lookup supplies the version | `[RAN]` |
| `ts-rs`, optional, feature `ts` of `chess-domain` | `chess-domain` (SDD-domain §3.4, D-50); enabled by `chess-app` | **12.0.1** (newest, same as §3.4) | 2026-01-31 | MIT | ok | none | 1.78.0, ok | Yes, table row with version | `[RAN]` |

Notes on the runtime rows:

- `chess-rules` needs only `shakmaty`, `pgn-reader` and `thiserror` (SDD-rules §3.4 and DR-RUL-010: no `serde`). `chess-domain` needs `serde`, `thiserror`, `uuid`, optional `ts-rs` and the workspace edge to `chess-rules` `[OBS SDDs]`.
- Features resolved in E5: `shakmaty` default (`std`, `magics`); `pgn-reader` has no features `[OBS registry]`; `uuid` with `default-features = false` and `v5` pulls only `sha1_smol` (BSD-3-Clause); `serde` `derive` pulls `serde_derive`, `syn` 3, `quote`, `proc-macro2`, `unicode-ident`.
- `uuid` feature `serde` adds `serde_core` only `[OBS registry features: "serde": ["dep:serde_core"]]`. Whether the `serde` feature is wanted is a design call for `GameId`/`PlayerId` serialisation; it was resolved here only as a candidate. See F7.
- `ts-rs` has the registry feature `uuid-impl`. Whether a `Uuid` field in a derived type needs it is `[UNV]` (not compiled against the real types).

### 2.2 Dev dependencies (tests)

| Crate | Used by (source) | Pin | Published | Licence | cargo-deny | Advisories | MSRV vs 1.99.1 | Admitted in §3.4 | Tag |
|---|---|---|---|---|---|---|---|---|---|
| `proptest` | Property tests, both crates (SDD-rules §7, SDD-domain §7, test-strategy) | **1.12.0** (newest) | 2026-10-10 | MIT OR Apache-2.0 | ok | none for `proptest`. Its tree holds `rand` 0.10.3, patched for RUSTSEC-2026-0097 (patched `>= 0.10.1`, E9) | 1.88, ok | Yes, "Test and lint tools" row | `[RAN]` |
| `insta` | Snapshot tests (SDD-rules §7, SDD-domain §7) | **1.49.0** (newest) | 2026-10-03 | Apache-2.0 | ok | none | 1.66.0, ok | Yes, same row | `[RAN]` |

`proptest` and `insta` add 30 or so dev-only packages (`rand` 0.10.3, `tempfile`, `rusty-fork`, `console`, `similar`, `once_cell` and others). Their licences are all in the allow list of section 4.

### 2.3 Workspace tooling (binaries, not linked into the product)

| Tool | Role (source) | Pin | Published | Licence | Advisories on the tool itself (E9) | Tool dependency scan (section 3) | MSRV | Installed here (E3) | Admitted in §3.4 | Tag |
|---|---|---|---|---|---|---|---|---|---|---|
| `cargo-deny` | Licence and advisory gate in CI (NFR-6, test-strategy) | **0.20.2** (newest) | 2026-07-09 | MIT OR Apache-2.0 | none | published lock: 10 vulnerabilities; fresh resolve: 0 | 1.88.0, ok | 0.20.2, matches | Yes | `[RAN]` |
| `cargo-audit` | Advisory scan (SWE-001 §5) | **0.22.2** (newest) | 2026-06-05 | Apache-2.0 OR MIT | none | published lock: 13 vulnerabilities; fresh: 0 | 1.88, ok | 0.22.2, matches | Yes | `[RAN]` |
| `cargo-mutants` | Mutation testing, Elo and Policy (test-strategy) | **27.1.0** (newest) | 2026-06-02 | MIT | none | published lock: 1; fresh: 0 | 1.88, ok | **Not installed** | Yes | `[RAN]` |
| `cargo-llvm-cov` | Line coverage, the 90 % gate (test-strategy "Quality gates") | **0.9.1** (newest) | 2026-09-06 | Apache-2.0 OR MIT | none | published lock: 3; fresh: 0 | 1.87, ok | 0.9.1; `llvm-tools` component installed | **No** (F1) | `[RAN]` |
| `cargo-nextest` | Test runner (optional; not named in any document) | **0.9.150** (newest on or before 2026-10-28; 0.9.151 is dated 2026-11-03) | 2026-10-26 | Apache-2.0 OR MIT | **RUSTSEC-2026-0412**, affects 0.9.125 to 0.9.149, patched `>= 0.9.150` | published lock: 10; fresh: 2 (`quick-xml` 0.38.4, RUSTSEC-2026-0194 and -0195) | 1.91, ok | **0.9.128, in the affected range** (F3) | **No** (F1) | `[RAN]` |
| `cargo-insta` | Optional CLI to review `insta` snapshots (`cargo insta review`) | not looked up | | | | | | Not installed | **No** | `[UNV]` |
| `rustfmt`, `clippy` | Format and lint gates (SWE-001 ASM-S1) | Toolchain components of 1.99.1: rustfmt 1.10.0-stable, clippy 0.1.99 | 2026-10-13 | Part of the Rust toolchain | n/a | n/a | n/a | Installed (E1, E2) | Rust row | `[RAN]` |

## 3. Scan of the dev tools' own dependencies (E10)

The tools are installed as binaries and are not shipped, so they are outside the "dependency the app uses" release gate (SWE-001 §5). The result still matters for where the toolchain is built from. `cargo install --locked` builds with the lock the author published; plain `cargo install` resolves fresh.

| Tool and version | Published `Cargo.lock`: vulnerabilities (unsound or unmaintained warnings) | Fresh resolve, no lock: vulnerabilities (warnings) |
|---|---|---|
| cargo-deny 0.20.2 | 10 (`rustls` 0.23.40 x4, `rustls-webpki`, `webpki-roots`, `zstd`, `zstd-safe` x2, `zstd-sys`) (10 unsound, `zstd-safe`) | 0 (0) |
| cargo-audit 0.22.2 | 13 (`crossbeam-epoch`, `h2`, `quinn-proto` x5, `rustls` x4, `rustls-webpki`, `webpki-root-certs`) (4 unsound: `anyhow`, `faster-hex`, `memmap2`, `tokio`) | 0 (0) |
| cargo-mutants 27.1.0 | 1 (`crossbeam-epoch`) (`atty` and `yaml-rust` unmaintained, `anyhow` unsound) | 0 (0) |
| cargo-llvm-cov 0.9.1 | 3 (`quick-xml` x2, `ruzstd`) (0) | 0 (0) |
| cargo-nextest 0.9.150 | 10 (`h2`, `pulldown-cmark` x2, `quick-xml` x2, `webpki-root-certs`, `zstd`, `zstd-safe` x2, `zstd-sys`) (14 unsound: `anyhow`, `lru`, `memmap2`, `tokio`, `zstd-safe` x10) | 2 (`quick-xml` 0.38.4) (`lru` unsound) |

`[RAN]` for every cell. The `cargo audit` runs also printed "couldn't check if the package is yanked: No such crate in crates.io index" for several crate names (for example `assert_cmd`, `indoc`). The crates exist, so this is a lookup problem of the tool in this environment, and the yanked check is `[UNV]` for the tool trees. `[INF]` The installed binaries' own lock files are unknown: when and how they were built was not checked `[UNV]`.

## 4. Licence view for `deny.toml` (cargo-deny compatible)

Licences found in the full resolved set of 68 packages (E5), taken from `cargo metadata` `[RAN]`:

`MIT`, `Apache-2.0`, `BSD-3-Clause` (`sha1_smol`), `Unlicense OR MIT` (`memchr`, `termcolor`, `winapi-util`), `(MIT OR Apache-2.0) AND Unicode-3.0` (`unicode-ident`), `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` (`linux-raw-sys`, `rustix`), `MIT OR Apache-2.0 OR LGPL-2.1-or-later` (`r-efi`), **`GPL-3.0-or-later` (`shakmaty`, `pgn-reader` only)**, and the legacy forms `MIT/Apache-2.0` and `Apache-2.0 / MIT` (`quick-error`, `rusty-fork`, `wait-timeout`, `core_detect`, `fnv`).

Allow list that passed `cargo deny --all-features check licenses` in E8: `MIT`, `Apache-2.0`, `BSD-3-Clause`, `Unicode-3.0`, `Unlicense`, `GPL-3.0-or-later` (plus `LGPL-2.1-or-later`, which no package needed, because `r-efi` offers an `OR` choice). The legacy `/` forms passed without a warning in this version `[RAN]`.

`[INF]` The two GPL crates are compatible with the project licence GPL-3.0-or-later (NFR-6, D-10 as quoted in requirements NFR-6 `[OBS]`). This is a reading of licence names, not legal advice. It is also a coupling: see F2.

## 5. Findings

Severity: High blocks the next step, Medium needs a decision or a document change before the stated point, Low is a note for the skeleton, Info is recorded only.

| # | Sev | Finding | Evidence | Needed from |
|---|---|---|---|---|
| F1 | **Medium** | **Tools and crates named or needed by step 1 that §3.4 lacks.** (a) `uuid` is admitted in principle with no version; this lookup supplies 1.29.0 and the feature set. (b) **`cargo-llvm-cov`**: test-strategy sets a 90 % line-coverage gate but no document names a coverage tool. 0.9.1 works with the installed `llvm-tools` component. (c) *Withdrawn in 0.2 (fuzzing, D-76).* (d) `cargo-nextest` (installed, not named anywhere) and `cargo-insta` (not installed, not named) are optional. (e) `sha1_smol` (BSD-3-Clause) enters through `uuid` `v5`; it is a transitive crate and needs only an allow-list entry. | Sections 2.1 to 2.3; grep of test-strategy and architecture for coverage tools found none `[RAN]` | **G2, Brian Prascher:** admit (b); decide (d). (b) and (d) were later ruled by D-76 (2) and D-81; see section 6 |
| F2 | **Medium** | **The project licence is load-bearing for D-3.** `shakmaty` and `pgn-reader` are GPL-3.0-or-later; `chess-rules`, and through the edge `chess-domain` and everything above, are combined works. This is fine while D-10 stands (GPL-3.0-or-later). If D-10 ever changes, D-3 and the whole rules crate change with it. Also, every workspace `Cargo.toml` must declare `license = "GPL-3.0-or-later"` or `cargo-deny` has nothing to check against. | E5, E8; section 4 | Note in D-3 or D-10; no approval needed to record |
| F3 | **Medium** | **The installed `cargo-nextest` 0.9.128 is in the affected range of RUSTSEC-2026-0412** (0.9.125 to 0.9.149; fixed `>= 0.9.150`; code execution when a malicious portable recording is replayed with `--rerun`, experimental recording must be enabled). It is a local developer tool and the feature is experimental, so exposure is low in practice. Do not adopt it at 0.9.128. | E3, E9 | Brian: upgrade to 0.9.150 or later, or leave it unused. No project action |
| F4 | Withdrawn | *Withdrawn in 0.2.* The 0.1 finding on a nightly compiler for `cargo-fuzz` is closed by D-76: no nightly toolchain ever, fuzzing tools not admitted | D-76 | None |
| F5 | **Medium** | **Dev-tool installs: use of `--locked` changes the vulnerability result.** Published locks of `cargo-deny` and `cargo-audit` carry 10 and 13 known vulnerabilities in transitive crates (`rustls`, `zstd`, `quinn-proto`, `h2` and others); a fresh resolve of the same crate versions is clean (cargo-nextest still has 2). So the install command is part of the supply-chain posture. `[INF]` For CI, install without `--locked` (or record each exception). The tools are not shipped, so the release gate in SWE-001 §5 does not apply to them, but the CI runner executes them. | Section 3 | Software engineer, in the CI task. No approval needed |
| F6 | **Medium** | **The "no `rand` / no time crate in the tree" checks are wrong as written.** SDD-domain CT-DOM-003 and SDD-rules §3.4 (and the SDD-domain §3.4 note) say a `cargo tree` assertion shows no `rand`. With `proptest` as a dev dependency, `cargo tree` (default edges) shows `rand 0.10.3`; with `-e normal,build` it shows nothing. The assertion must use `-e normal,build` (and say so), or it fails on the first build. | E11 | SDD owner (G0 edit later); I made no edit |
| F7 | **Medium** | **Design gap: the ECO generator.** SDD-rules §3.11.9 has `build.rs` replay each ECO line "with `parse_move`". A `build.rs` cannot call the crate it builds. The generator needs `shakmaty` as a `[build-dependencies]` entry (already admitted, but a build edge and a second parse path next to `parse_move`), or a separate generator crate or `xtask` that writes a checked-in file. The second option adds a workspace member. This is a dependency-edge question (G2 if it adds a crate). | SDD-rules §3.11.9, CT-RUL-003 `[OBS]` | Architect and SDD owner; **G2** if a crate is added |
| F8 | **Low** | **Feature sets still open.** `uuid`: `v5` is needed (D-46); `serde` is needed only if `GameId`/`PlayerId` cross a serde boundary (SDD-domain §3.4 says `GameSetup` and others are received in protocol commands, so yes in practice). `ts-rs`: the optional `uuid-impl` feature may be needed if a `Uuid` field sits in the six derived types. The SDD says `uuid` is for domain identity and the derived types hold `GameSetup` and `Preferences`. Neither point was compiled against real types. Also SDD-domain §10 line "The `uuid`, `serde` and `thiserror` crates were not looked up" is now out of date for those three. | Section 2.1 | Software engineer to settle when the types exist |
| F9 | **Low** | **Duplicate `syn` (2 and 3)**: `ts-rs-macros` 12.0.1 uses `syn` 2; `serde_derive` and `thiserror-impl` use `syn` 3. Only appears with feature `ts`. Set `multiple-versions = "warn"`, not `deny`. | E8 | Skeleton task |
| F10 | **Low** | **MSRV margin.** The highest MSRV in the resolved tree is 1.97 (`shakmaty`, `pgn-reader`), then `uuid` 1.89.0. All are at or below the workspace `rust-version = "1.99"` and the local rustc 1.99.1. Only `cargo-nextest` (1.91) and the cargo tools (1.87 to 1.88) have tool MSRVs, also fine. Compiled clean in E6. | Section 2 | None |
| F11 | **Info** | **Results are a point in time.** The advisory DB read on 2026-11-04 is dated six days after the baseline (2026-10-28). Re-run `cargo audit` and `cargo deny` in CI and at release; do not reuse this table as a release record. | E7, E8 | CI task |
| F12 | Withdrawn | *Withdrawn in 0.2.* It recorded the `cargo-fuzz` and `libfuzzer-sys` MSRV as `[UNV]`; both are not admitted (D-76) | D-76 | None |
| F13 | **Info** | **Not covered here:** OQ-17 stays open for every crate of steps 2 and 3, for all npm packages, for the ECO data file (CC0 claim, D-20), for Stockfish, and for the SQLite inside `libsqlite3-sys` (`json_valid` support is spike item S1, section 6A, not a lookup). | Scope | Later OQ-17 parts |

## 6. Proposed edits to architecture §3.4 (proposal only; `docs/architecture.md` and the SDDs were not edited)

All edits need a Decision Log entry and a RELEASE_NOTES entry as an amendment (CLAUDE.md). Items marked **G2** wait for Brian Prascher. Items marked **G0** record a lookup result for an already admitted dependency, so they are an evidence update, not a new decision. IDs are not renumbered.

| # | Edit | Type |
|---|---|---|
| P1 | In the baseline table, add a row: `uuid` **1.29.0** (2026-10-23), features `v5` (`serde` per F8), MIT OR Apache-2.0, MSRV 1.89.0, `[RAN 2026-11-04]` | G0 (lookup of an admitted dependency) |
| P2 | In the admission table, for `serde` set the feature set to `derive` (and state that `chess-rules` has none, DR-RUL-010); for `uuid` replace "`[UNV]`" with "looked up 2026-11-04, see `docs/engineering/dependency-lookup-step1.md`"; for `thiserror` add the note that both step-1 crates use it | G0 |
| P3 | Replace the sentence "Their presence on the development Mac was not checked" in the "Test and lint tools" row with the E3 result: `cargo-deny` 0.20.2 and `cargo-audit` 0.22.2 present at the baseline versions; `cargo-mutants`, `cargo-insta` absent | G0 |
| P4 | Add to the baseline table `cargo-llvm-cov` **0.9.1** (2026-09-06, Apache-2.0 OR MIT, MSRV 1.87) as the coverage tool for the test-strategy 90 % gate. *Status note, 0.2: D-76 (2) has since admitted `cargo-llvm-cov` 0.9.1; kept here as lookup evidence.* | **G2** (F1) |
| P5 | *Withdrawn in 0.2* (fuzzing; D-76: `libfuzzer-sys` and `arbitrary` not admitted) | Withdrawn |
| P6 | Decide `cargo-nextest`: if adopted, pin **0.9.150 or later** (not 0.9.128, F3); if not, say it is not used. *Status note, 0.2: D-81 records `cargo-nextest` as not adopted for now; kept here as lookup evidence.* | **G2** (F1, F3) |
| P7 | *Withdrawn in 0.2* (fuzzing; D-76: no nightly toolchain ever) | Withdrawn |
| P8 | Add a note in D-47 or §3.4: the `cargo-deny` allow list for step 1 is `MIT`, `Apache-2.0`, `BSD-3-Clause`, `Unicode-3.0`, `Unlicense`, `GPL-3.0-or-later` (section 4), `multiple-versions = "warn"` | G0 |
| P9 | Add to the "Dependency admission" paragraph: dev tools are installed without `--locked`, and `cargo audit`/`cargo deny` run in CI on the project lock file (F5, F11) | G0 |
| P10 | Edits to the SDDs (outside §3.4, for the SDD owner): CT-DOM-003 and SDD-rules §3.4 tree assertions use `cargo tree -e normal,build` (F6); SDD-rules §3.11.9 decides where the ECO generator gets its parser (F7); SDD-domain §10 note on the three lookups (F8) | G0 text edits, F7 is **G2** if a crate is added |

## 6A. Spike items carried to the OQ-14 spike (not lookup results)

These are questions that only a run on the bundled build can answer. This document answers none of them. They are recorded so that they are not lost, and they are `[UNV]`.

| # | Item | Why it matters | Where it is carried | Status |
|---|---|---|---|---|
| S1 | **Does the SQLite built into the pinned `rusqlite` bundled build (through `libsqlite3-sys`, feature `bundled`) support `json_valid`?** Test: open an in-memory database, run `SELECT json_valid('{"a":1}')` and `CREATE TABLE t (j TEXT NOT NULL CHECK (json_valid(j)))`, then insert one valid and one invalid value and see the `CHECK` reject the invalid one | The store DDL uses `CHECK (json_valid(json))` `[OBS SDD-chess-store.md:287]`. If the function is missing, table creation fails or the `CHECK` does not guard, and the DDL changes after the store design is baselined (readiness FIND-R23-003) | Folded into the OQ-14 spike scope (tracker chess-a0r.3.1; the program manager recorded the scope there). Result to be recorded for architecture §3.4 | `[UNV]`. Not run. I recall that JSON functions are built in by default in recent SQLite releases, but this is memory, not a reading, and the bundled SQLite version was not looked up |

Other store primitives named in readiness FIND-R23-003 and FIND-R23-004 (`WITHOUT ROWID`, WAL, `VACUUM INTO`, `ftruncate` then `F_FULLFSYNC`, no-replace rename, the CRC check value) are outside this reissue; Brian's ruling asked only for S1. Their spike scope is set at chess-a0r.3.1.

## 7. Rows and facts left `[UNV]`

- `cargo-insta`: version, licence and advisories.
- Yanked status of the transitive crates in the tool trees (the tool's yanked check failed in this environment, section 3).
- Whether `uuid` `serde` and `ts-rs` `uuid-impl` are needed by the real types (F8).
- How the installed `cargo-deny`, `cargo-audit`, `cargo-llvm-cov` binaries were built (lock used, date).
- The upstream APIs of `shakmaty` 0.30.2 and `pgn-reader` 0.29.1 (SDD-rules OQ-RUL-001): not part of this lookup.
- Whether the bundled SQLite supports `json_valid` (spike item S1).
- Everything outside step 1 (F13).

## 8. Assumptions

- ASM-DL1: "Step 1 workspace tooling" means the Rust tools named in architecture §9, SDD-rules §7, SDD-domain §7 and test-strategy, plus the coverage support they imply. JavaScript tooling is step 3. Validation: Brian or the PM confirms the scope when the skeleton task starts.
- ASM-DL2: The pin rule is "highest stable on or before 2026-10-28" for all rows, as in §3.4. Under it, every runtime and dev pin equals the newest release today, except `cargo-nextest` (0.9.150 against 0.9.151).
- ASM-DL3: A scratch crate in the session scratchpad, with exact `=` pins, is an adequate stand-in for the future `Cargo.lock`. The real lock must be re-scanned when the skeleton exists. No project file was changed, no dependency was added to any project, and no software was installed.
- ASM-DL4: `cargo-deny` and `cargo-audit` read the same RustSec database, so E7 and E8 are one evidence source for advisories, not two.

## 9. Amendment 0.3 (2026-11-06): transitive crates of `shakmaty` 0.30.2

**Scope.** Itemisation only. Part A of `chess-rules` (task F-A3, chess-pe1) found that sections 2.1 and 4 name `shakmaty` and `pgn-reader` but not the crates `shakmaty` pulls in. Brian Prascher accepted on 2026-11-06 that they be listed. No pin, licence conclusion, finding or proposal above changes. Sections 1 to 8 are as in 0.2.

**Method.** Run on 2026-11-06 in the repository workspace (commit 0087962 plus the working tree), offline, against the committed `Cargo.lock`.

| # | Command | Result | Tag |
|---|---|---|---|
| E12 | `cargo tree --offline -e features -p shakmaty` and `cargo tree --offline -p shakmaty -e normal,build --prefix none` | `shakmaty` v0.30.2 with 5 crates below it: `arrayvec` 0.7.8, `bitflags` 2.13.2, `btoi` 0.5.0, `num-traits` 0.2.19, `autocfg` 1.5.1 (build dependency of `num-traits`, reached through `btoi`) | `[RAN]` |
| E13 | `cargo metadata --offline --format-version 1 --locked`, then versions, `license` and `rust_version` read from the packages under `shakmaty` in the resolve graph | Table below | `[RAN]` |
| E14 | `grep -n -A12 'name = "shakmaty"' Cargo.lock`, and `cargo tree --offline -i nohash-hasher`, `cargo tree --offline -i serde` | The lock entry for `shakmaty` 0.30.2 lists `arrayvec`, `bitflags`, `btoi`, `nohash-hasher`, `serde`. The last two are not in the build graph (both `cargo tree -i` runs print "nothing to print") | `[RAN]` |
| E15 | `cargo deny --offline check` (the repository `deny.toml`) | `advisories ok, bans ok, licenses ok, sources ok`, exit 0. Warnings: `license-not-encountered` for the `BSD-3-Clause` and `Unlicense` allow entries | `[RAN]` |

**Crates built for `shakmaty` 0.30.2 (default features `std`, `magics`; `alloc` follows from `std`).** Licence is the registry SPDX field from `cargo metadata`. MSRV is the registry `rust_version`. All are at or below the workspace 1.99.

| Crate | Version | Licence | MSRV | Role | Tag |
|---|---|---|---|---|---|
| `arrayvec` | 0.7.8 | MIT OR Apache-2.0 | 1.51 | Direct dependency of `shakmaty` | `[RAN E12, E13]` |
| `bitflags` | 2.13.2 | MIT OR Apache-2.0 | 1.56.0 | Direct dependency | `[RAN E12, E13]` |
| `btoi` | 0.5.0 | MIT OR Apache-2.0 | 1.60 | Direct dependency | `[RAN E12, E13]` |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | 1.60 | Dependency of `btoi` | `[RAN E12, E13]` |
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT | 1.0 | Build dependency of `num-traits` | `[RAN E12, E13]` |

**Locked but not built (optional dependencies of `shakmaty`, features not enabled in this workspace).** They are in `Cargo.lock` because Cargo locks optional dependencies. `nohash-hasher` 0.2.0, Apache-2.0 OR MIT, no `rust_version` in the registry data; `serde` 1.0.229, MIT OR Apache-2.0. `serde` brings `serde_core`, `serde_derive`, `syn` 3.0.9, `quote`, `proc-macro2` and `unicode-ident` into the lock, all of which section 2.1 already names for `serde`. `[RAN E13, E14]`. They enter a build only if the `shakmaty` `serde` or `nohash-hasher` feature is enabled; the 0.3 run did not look at which `shakmaty` feature names enable them `[UNV]`.

**Licence view.** All five built crates carry licences already in the allow list of section 4 (`MIT`, `Apache-2.0`, as `OR` choices), so section 4 needs no new entry. `[RAN E15]` for the check, `[INF]` for the reading of the `OR` expressions. The "about 30 unique packages" figure in E5 is not recomputed.

**Advisories.** `cargo deny --offline check` passed (E15). It reads the advisory database already on the machine; the database date and commit were not re-checked in this run and may differ from E7 `[UNV]`. Per-crate advisory file history (as in E9) was not re-run for the five crates `[UNV]`. Yanked status was not checked `[UNV]`. Each crate's publication date against the 2026-10-28 pin rule was not checked; the versions are those the committed lock holds `[UNV]`.

**Not done in this amendment.** No crate was added, removed or re-pinned. No edit to `docs/architecture.md`, the SDDs or `deny.toml`.
