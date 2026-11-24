# Dependency lookup, build steps 2 and 3 (OQ-17, part 2)

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/engineering/dependency-lookup-step2.md | 0.2 DRAFT | 2026-11-09 | **Draft. Not accepted as a document.** v0.2 records Brian Prascher's decisions of 2026-11-09 on DL2-1 to DL2-15, F1 and F2 (section 5B). Task chess-a0r.3.2 stays open until he rules on F3, F4, F5 and F7 (`step2-deps-findings-packages.md`). Written by the system-architect agent (Architect stance). No Review Board pass has been held on it | System-architect agent, for Brian Prascher | Brian Prascher accepted the recommendations of DL2-1 to DL2-15 (G2) on 2026-11-09 (relayed by the coordinator). That is not an acceptance of this document as a whole. v0.1 header, for the record: "0.1 DRAFT, Draft proposal. Not accepted" |

**Change log.**

| Version | Date | Change | Acceptance |
|---|---|---|---|
| 0.1 DRAFT | 2026-11-09 | First issue | None |
| 0.2 DRAFT | 2026-11-09 | Adds the header update, a status line under each DL2 heading (ACCEPTED, with the accepted option), section 5B (decision record for DL2-1 to DL2-15, F1 and F2, and the residual items), and the v0.2 notes N1 to N6 inside it. Nothing in sections 1 to 9 of v0.1 is deleted or reworded except where a v0.2 line is marked | DL2-1 to DL2-15, F1, F2 accepted by Brian Prascher, G2, 2026-11-09 (relayed by the coordinator; no verbatim quotation was supplied). DL2-14 is recorded as a recommendation still to be confirmed. The document as a whole is not accepted |

**Gate record (proposal).** Tier **R1**: every choice below is a two-way door (a crate swap before code exists), the document is evidence plus options, and the first-`Cargo.toml` entries are blocked until Brian accepts the G2 items of section 5A and section 6. This is not a claim of any review.

Scope: every crate and npm package named in the step-2 and step-3 SDD dependency lists (`chess-store`, `chess-engine`, `chess-app`, `chess-cli`, `chess-server`, `ui`, `desktop-tauri`), the crates and packages the SDDs need but do not name, and the dev and test tools of those blocks. Not covered: see section 7 (items not looked up) and F20.

Evidence tags (SWE-001): `[RAN]` a command was executed, `[OBS]` read from a source (registry metadata, source file, advisory text), `[INF]` inferred, `[UNV]` not verified. All dates are 2026-11-09 (UTC) unless stated. Method, columns and sections follow `dependency-lookup-step1.md` v0.2 (accepted 2026-11-06).

## 1. Method and environment

**Rule applied.** Same as step 1 (architecture §3.4, SWE-001 FIND-006): the pin is the highest stable, non-yanked release **published on or before 2026-10-28**. Where a newer release exists today it is shown, but it is not the pin. Crates that carry a `+build` suffix (`toml` 1.1.8+spec-1.1.0) are ordered by their numeric version.

| # | Check | Command (exact, shortened where noted) | Result | Tag |
|---|---|---|---|---|
| S2-E1 | Toolchain and tools | `rustc --version; node --version; npm --version; cargo deny --version; cargo audit --version` | rustc 1.99.1 (58cf8f9c3 2026-10-13); node v26.13.0; npm 11.20.0; cargo-deny 0.20.2; cargo-audit 0.22.2. Same as step 1 | `[RAN]` |
| S2-E2 | crates.io metadata | `curl -s -A "chess-oq17 (chess-dev)" https://crates.io/api/v1/crates/<crate>` for 47 crates (the 31 of section 2, the candidates `libc`, `rustix`, `portable-pty`, `rexpect`, `pty-process`, `async_zip`, `rc-zip-sync`, `flate2`, `miniz_oxide`, `zip-extract`, `digest`, `nix`, `ptyprocess`, `ratatui-crossterm`, and the step-1 crates re-read for `uuid`, `ts-rs`, `serde`, `thiserror`). A script (session scratchpad) takes per crate the highest non-yanked stable version with `created_at` on or before 2026-10-28T23:59:59 | Sections 2 and 5A. Files kept only in the session scratchpad | `[RAN]` |
| S2-E3 | npm registry metadata | `npm view <pkg> time --json`, then `npm view <pkg>@<pin> license engines peerDependencies dependencies deprecated --json`, for 32 packages. Same pin rule | Section 2.5 | `[RAN]` |
| S2-E4 | Resolve the Rust set | A scratch workspace with six probe crates (store, engine, app, cli, server, desktop), edition 2024, `rust-version = "1.99"`, `license = "GPL-3.0-or-later"`, exact `=` pins of section 2 for the direct dependencies and the feature sets of section 5A. `cargo generate-lockfile` | 590 lock entries (the six probes are among them). Transitive crates resolved to **today's** versions, not to the 2026-10-28 rule (ASM-DL2-2) | `[RAN]` |
| S2-E5 | Build check | `cargo check --all-targets --workspace` in the scratch workspace on rustc 1.99.1, macOS aarch64 host | `Finished dev profile`, exit 0. This compiled `libsqlite3-sys` 0.39.1 (bundled C build), `tauri` 2.12.5 and all six probes. No project code was compiled (the probes are empty) | `[RAN]` |
| S2-E6 | Advisories (tree), RustSec | `cargo audit --db <scratch>/advisory-db --file Cargo.lock` and `cargo audit --no-fetch` | RustSec DB at commit `d9cb059f3217db87aeccd190b81eaffe8cc29cb9` (2026-11-09T17:13:08+01:00), **1430 advisories**. 590 crate entries scanned. **0 vulnerabilities, 3 warnings**: `pin-utils` 0.1.1 unmaintained (RUSTSEC-2026-0441), `proc-macro-error` 1.0.4 unmaintained (RUSTSEC-2024-0370), `glib` 0.18.5 unsound (RUSTSEC-2024-0429). Where each comes from: S2-E9 | `[RAN]` |
| S2-E7 | Licences, bans, sources, advisories (second tool) | `cargo deny --config <copy of repository deny.toml with the [bans] deny list emptied, because the probe crates are not the workspace crates> check`, run twice: (a) `[graph] targets = [aarch64-apple-darwin, x86_64-apple-darwin]`, (b) no targets (all platforms) | (a) `advisories FAILED, bans ok, licenses FAILED, sources ok`. Errors: `pin-utils` unmaintained (1), licences rejected for 7 crates (section 4). (b) the same plus `proc-macro-error` unmaintained, and an eighth licence rejection, `target-lexicon` 0.12.16 (a platform-specific crate; not in the macOS tree, licence not read `[UNV]`). Yanked: no error (the repository policy is `yanked = "deny"`). Many `multiple-versions` warnings (policy is `warn`) | `[RAN]` |
| S2-E8 | Per-crate advisory history | `ls <advisory-db>/crates/<crate>`, then the advisory text, for every crate in section 2 | Section 2 "Advisories" column. Advisory files in 2026 for `tokio` (2), `libsqlite3-sys` (1), `zip` (4); none for the other new crates | `[RAN]` |
| S2-E9 | Dependency-tree assertions | `cargo tree -i <crate> -e normal,build --target <triple>` for `pin-utils`, `proc-macro-error`, `glib`, `rand`, `cssparser`, `selectors`, `dtoa-short`, `option-ext`, `foldhash`, `zlib-rs`, `mime_guess`, `crossterm`, `getrandom`; `cargo tree -p probe-<c> -e normal,build --target aarch64-apple-darwin --prefix none` for each probe | `pin-utils` only through `expectrl` (dev) via `nix` 0.26.4. `proc-macro-error` and `glib` only through Tauri's Linux (GTK) stack and absent from the macOS target. No `reqwest`, `ureq`, `hyper-tls`, `rustls`, `native-tls`, `openssl`, `h2`, `curl` in any of the six probes (macOS, normal and build edges). Details in the section 2 notes | `[RAN]` |
| S2-E10 | ZIP feature variants | A second scratch workspace: `zip` 9.1.3 with `deflate`, with `deflate-flate2`, with `deflate-flate2-zlib-rs`; `cargo tree`, `cargo check` | `deflate`: 21 crates. `deflate-flate2`: **does not compile** (`flate2` has no backend selected, 3 errors). `deflate-flate2-zlib-rs`: compiles, 15 crates (no `zopfli`). Decision DL2-1 | `[RAN]` |
| S2-E11 | PTY test tool candidates | A third scratch workspace: `expectrl` 0.9.0 + `vt100` 0.18.1, and `portable-pty` 0.9.0 + `vt100`; `cargo audit --no-fetch`, `cargo tree` | The `expectrl` tree (21 crates) holds `pin-utils` (unmaintained). The `portable-pty` tree (26 crates) holds no advisory at the DB date. Decision DL2-11 | `[RAN]` |
| S2-E12 | Bundled SQLite | Read from the cached source of `libsqlite3-sys` 0.39.1 in `~/.cargo/registry/src`: `grep "define SQLITE_VERSION" sqlite3/sqlite3.c`, `build.rs` compile flags, `ls sqlite3`. NVD REST queries (`services.nvd.nist.gov/rest/json/cves/2.0`) for `cpe:...sqlite:3.54.1` and the two CVE ids below | Bundled SQLite **3.54.1**. `build.rs` defines `SQLITE_ENABLE_JSON1`, `SQLITE_ENABLE_LOAD_EXTENSION=1`, `SQLITE_DEFAULT_FOREIGN_KEYS=1`, `SQLITE_THREADSAFE=1`, `SQLITE_ENABLE_FTS5`, `SQLITE_ENABLE_RTREE`, and no `SQLITE_OMIT_JSON`. `sqlite3.c` contains `json_valid`. No `shell.c` in the crate. NVD: CVE-2026-119375 (fixed in 3.54.0), CVE-2026-119586 (CLI shell only). Section 5 F10 | `[OBS]` for the source, `[RAN]` for the NVD queries |
| S2-E13 | Standard library facts | Read from the `rust-src` component of 1.99.1: `library/std/src/fs.rs`, `library/std/src/sys/fs/unix.rs`, `library/std/src/os/unix/process.rs` | `File::lock`, `lock_shared`, `try_lock`, `unlock` are `#[stable(since = "1.89.0")]` and call `flock`. On Apple targets `File::sync_all` and `sync_data` call `fcntl(fd, F_FULLFSYNC)`. `CommandExt::process_group` is `#[stable(since = "1.64.0")]`. Section 5 F15 | `[OBS]` |
| S2-E14 | npm resolution and audit | In a scratch directory outside the repository: `npm install --package-lock-only --ignore-scripts --no-audit --no-fund`, then `npm audit --json` and `npm audit --omit=dev --json`, on the baseline set (variant v0 without `typescript-eslint`, base with it, v3 final) | Base set: **ERESOLVE** (F1). v3 (recommended set, 253 packages in the lock): `npm audit` 10 moderate, **0 with `--omit=dev`**. Registry contacted for the audit | `[RAN]` |
| S2-E15 | typescript-eslint against TypeScript 7 | In a scratch directory: `npm install --legacy-peer-deps` of `typescript` 7.0.2, `eslint` 10.13.0, `typescript-eslint` 8.72.1, then `eslint src`; then the side-by-side install of section 5A DL2-8 | Plain: ESLint aborts with "typescript-eslint does not support TS 7.0". Side-by-side: `tsc` 7.0.2 passes, `tsc6` reports 6.0.3, type-aware `eslint` runs and reports 5 problems on a deliberately bad file | `[RAN]` |
| S2-E16 | Library source reads | `tracing-appender` 0.2.5 `src/rolling.rs`; `uuid` 1.29.0 `src/builder.rs`; `tokio-util` 0.7.20 `src/sync/mod.rs`; `libsqlite3-sys` `Cargo.toml` features | `tracing-appender` rotation is `MINUTELY`, `HOURLY`, `DAILY`, `WEEKLY` or `NEVER`, with `max_files` pruning; no size-based rotation. `uuid::Builder::from_random_bytes` exists. `CancellationToken` is not feature-gated. | `[OBS]` |
| S2-E17 | A run of two small programs | In a scratch crate: `crc32fast` 1.5.3 over `b"123456789"`; `uuid` 1.29.0 (features `v5` only) `Builder::from_random_bytes([0xff; 16])` | `0xCBF43926` (the CRC-32/ISO-HDLC check value); the `uuid` call compiles and yields version `Random` without the `v4` feature | `[RAN]` |
| S2-E18 | `cargo-deny` `unmaintained` scope | `cargo deny check advisories` with `unmaintained = "workspace"`, `"transitive"`, `"none"` added to `[advisories]` | `workspace` and `none`: `advisories ok`. `transitive` and the default: fails on `pin-utils`. Decision DL2-10 | `[RAN]` |

**Limits of S2-E6 and S2-E7.** They read the advisory database on 2026-11-09, twelve days after the baseline date. A result is a point in time. CI and each release re-run them (SWE-001 §5). The scratch lock resolved transitive crates to today's versions, so it is a stand-in for the future `Cargo.lock`, not a record of it (ASM-DL2-2). `cargo audit` printed no yanked-check problem in this run; the `cargo audit` tool does not use the project's `yanked = "deny"` policy, so the yanked result rests on `cargo deny` (no error) `[RAN]`.

## 2. Lookup tables

"Admitted" means architecture §3.4 as amended (AMD-8, D-47, D-76, D-81, D-93). "In principle" means admitted with no version (D-47 table). "No" means not named in §3.4. MSRV is the registry `rust_version`; the workspace is `rust-version = "1.99"` and the local rustc is 1.99.1, so every MSRV at or below 1.99 builds, and S2-E5 compiled clean. Every pin below exists, is not yanked and was published on or before 2026-10-28 `[RAN S2-E2]` unless the row says otherwise. The "cargo-deny" column is the result of S2-E7 (a) with the allow list of `deny.toml` as it is today.

### 2.1 Rust runtime dependencies: named in an SDD

| Crate (features proposed, section 5A) | Used by (source: SDD, decision) | Pin | Published | Licence (registry SPDX) | cargo-deny | Advisories (S2-E8) | MSRV | Admitted in §3.4 | Tag |
|---|---|---|---|---|---|---|---|---|---|
| `tokio` (per crate, DL2-5) | store `sync`, `rt`, `time`; engine `process`, `io-util`, `sync`, `time`, `rt`, `macros`; app, cli, server, desktop (SDD-store §3.4, SDD-engine §3.4, SDD-server §3.4, D-8, D-13) | **1.54.1** (newest; 1.54.0 is yanked) | 2026-10-21 | MIT | ok | 7 files. **RUSTSEC-2026-0384** (`Interval` race, affects 1.54.0 only, patched `>= 1.54.1`, so the pin is patched). **RUSTSEC-2026-0386** (`AsyncFd::new` I/O-safety, informational "unsound", patched `>= 1.53.3`, applies to four constructors; the pin carries the fix by deprecation). Five older files, all patched far below | 1.85 | Yes, table row | `[RAN]` |
| `rusqlite`, `default-features = false`, `bundled` (DL2-5 note) | store (SDD-store §3.4, D-12) | **0.41.1** (newest) | 2026-10-28 | MIT | ok | 2 files (2020, 2021), patched `>= 0.26.2` | **1.89.0** | Yes, table row (`[UNV]` for the SQLite release, now read: 3.54.1) | `[RAN]` |
| `libsqlite3-sys` (through `rusqlite`) | store (architecture §3.4) | **0.39.1** (newest) | 2026-10-25 | MIT (the bundled SQLite source is public domain `[INF]`; its licence text was not read `[UNV]`) | ok | **RUSTSEC-2026-0459** (CVE-2026-119375: SQLite before 3.54.0 can return uninitialised page-cache memory for a database whose `-wal` has a different page size; patched `>= 0.39.0`, so the pin is patched; bundled SQLite is 3.54.1). RUSTSEC-2022-0090 (patched `>= 0.25.1`) | 1.88.0 | Yes (as a parenthesis) | `[RAN]`, `[OBS]` for 3.54.1 |
| `serde` (`derive`), `serde_json` | store (journal payloads, row DTOs), app (protocol, archive) (SDD-store §3.4, SDD-app §4.4, D-4) | `serde` **1.0.229** (step 1), `serde_json` **1.0.154** | 2026-07-18, 2026-10-11 | MIT OR Apache-2.0 | ok | none for `serde_json`; step 1 covers `serde` | 1.71 | `serde` Yes; `serde_json` in principle | `[RAN]` |
| `uuid`, `v5` + `serde` (store); `v5` + `serde` and see DL2-4 for `v4` (app) | store, app (SDD-store §3.4, D-5, D-46) | **1.29.0** (step 1; 1.29.2 is dated 2026-11-09) | 2026-10-23 | Apache-2.0 OR MIT | ok | none | 1.89.0 | In principle; step 1 supplied the version | `[RAN]` |
| `crc32fast` | store, journal record checksum (SDD-store §3.4, D-6, SF-08) | **1.5.3** (newest is 1.5.4 dated **2026-11-01**, after the baseline) | 2026-10-20 | MIT OR Apache-2.0 | ok | none | 1.63 | In principle | `[RAN]`. The check value over `"123456789"` is `0xCBF43926` `[RAN S2-E17]` |
| `tracing` | store, engine, app, cli, server, desktop (all SDD §3.4) | **0.1.44** | 2025-12-18 | MIT | ok | RUSTSEC-2023-0078, patched `>= 0.1.40` | 1.65.0 | Yes, table row | `[RAN]` |
| `tracing-subscriber` (DL2-6) | app (SDD-app §4.4, D-11) | **0.3.23** | 2026-03-13 | MIT | ok | RUSTSEC-2025-0055 (ANSI escape in logs), patched `>= 0.3.20` | 1.65.0 | In principle | `[RAN]` |
| `tracing-appender` (DL2-6, F6) | app (SDD-app §4.4 "named in §3.4 and used"; architecture §3.4) | **0.2.5** | 2026-04-17 | MIT | ok | none | 1.63.0 | Yes, table row | `[RAN]` |
| `tokio-util` (no features) | app, `CancellationToken` (SDD-app §4.4, D-18) | **0.7.20** | 2026-10-09 | MIT | ok | none | 1.85 | In principle | `[RAN]`. `CancellationToken` needs no feature `[OBS S2-E16]` |
| `futures-util` (`default-features = false`, `std`; `sink` for the server) | app, cli, server (D-17) | **0.3.34** | 2026-08-11 | MIT OR Apache-2.0 | ok | 2 files (2020), patched `>= 0.3.7` | 1.71 | In principle | `[RAN]` |
| `tokio-stream` | app, "only if needed" (D-17) | 0.1.19 (2026-07-22), MIT, MSRV 1.71, no advisory. **Not proposed**: no document names a consumer, and crossterm's `EventStream` implements `futures-core::Stream`, which `futures-util` extends `[INF]` | | | | | | In principle | `[RAN]` for the facts, `[INF]` for the verdict |
| ZIP library (DL2-1): `zip` | app, archive export and import (SDD-app ALG-APP-10, -11, D-15) | **9.1.3** (9.5.0 is dated 2026-11-09), `default-features = false`, feature `deflate-flate2-zlib-rs` | 2026-10-27 | MIT | ok (transitive `zlib-rs`, `foldhash`: Zlib, see section 4) | **RUSTSEC-2025-0168, -2026-0367, -0368, -0369, -0429**, all patched `>= 9.0.0`; all concern `ZipArchive::extract` or `ZipStreamReader::extract` (SUID bits, symlink targets, a large symlink size aborting the process). The pin is patched | 1.88 | In principle ("chosen after lookup") | `[RAN]` |
| SHA-256 (DL2-2): `sha2` | app, `content_hash` (SDD-app ALG-APP-09, D-16, D-42, D-68) | **0.11.1**, `default-features = false` | 2026-10-12 | MIT OR Apache-2.0 | ok | RUSTSEC-2021-0100 (patched `>= 0.9.8`) | 1.88 | In principle | `[RAN]` |
| `unicode-segmentation` | app, grapheme count (SDD-app §3.2, PLY-6) | **1.13.3** | 2026-06-01 | MIT OR Apache-2.0 | ok | none | 1.85.0 | Yes, table row | `[RAN]` |
| `unicode-normalization` | app, NFC (SDD-app, PLY-6) | **0.1.26** | 2026-10-16 | (MIT OR Apache-2.0) AND Unicode-3.0 | ok (`Unicode-3.0` is in the allow list) | none | 1.36 | Yes, table row | `[RAN]` |
| `ts-rs` (feature `ts`) | app enables it; step 1 covers it | **12.0.1** (step 1) | 2026-01-31 | MIT | ok | none | 1.78.0 | Yes | `[RAN]`; the app probe resolves its tree (`ts-rs-macros` with `syn` 2 and `termcolor`) |
| `thiserror` | store, engine, app, cli, server, desktop | **2.0.21** (step 1) | 2026-09-23 | MIT OR Apache-2.0 | ok | none | 1.77 | Yes | `[RAN]` |
| `toml` | cli, `config.toml` and line numbers (SDD-cli §3.4, architecture §3.2, D-24) | **1.1.8+spec-1.1.0** (newest 1.2.1, 2026-11-06). An exact `=1.1.8` requirement resolves to this | 2026-10-09 | MIT OR Apache-2.0 | ok | none | 1.85 | Yes, table row | `[RAN]`. That `toml` yields a line and column for a parse error is `[UNV]` (SDD API names are from memory, ASM-CLI-1) |
| `clap` (`derive`) | cli (SDD-cli §3.4, D-21) | **4.6.7** | 2026-09-14 | MIT OR Apache-2.0 | ok | none | 1.85 | `clap` named in §3.3 and the D-47 table, no version | `[RAN]` |
| `ratatui` | cli (D-6) | **0.30.2** | 2026-06-19 | MIT | ok | none | 1.88.0 | Yes, table row | `[RAN]` |
| `crossterm` (`event-stream`) | cli (D-6, SDD-cli §3.4) | **0.29.0** (newest) | 2025-04-05 | MIT | ok | none | 1.63.0 | Yes, table row | `[RAN]`. One `crossterm` 0.29.0 in the lock, shared with `ratatui-crossterm` 0.1.2 |
| `axum` (`default-features = false`, `http1`, `tokio`, `ws`) | server (SDD-server §3.4) | **0.8.9** | 2026-04-14 | MIT | ok | none | 1.80 | Yes, table row | `[RAN]`. That the SDD code needs none of the other default features (`json`, `query`, `form`, `matched-path`, `original-uri`, `tower-log`, `tracing`) is `[UNV]` |
| `rust-embed` | server, embeds `ui/dist` (SDD-server §3.4) | **8.13.0** | 2026-10-07 | MIT | ok | RUSTSEC-2021-0126 (patched `>= 6.3.0`) | 1.80.0 | Yes, table row | `[RAN]`. No feature is on by default; `mime_guess` 2.0.5 arrives through `rust-embed-impl` as SDD-server UF-SRV-9 says `[RAN S2-E9]`. In a debug build the crate reads files from disk unless the `debug-embed` feature is on `[UNV]` (not read) |
| `tauri` | desktop (SDD-desktop-tauri §3.4) | **2.12.5** | 2026-10-21 | Apache-2.0 OR MIT | ok for the macOS targets, with `MPL-2.0` entries (section 4) | RUSTSEC-2022-0088, -0091, patched `>= 1.0.6` and `>= 1.1.2` | **1.90** | Yes, table row | `[RAN]` |
| `tauri-build` | desktop `build.rs` (implied by Tauri 2 project layout; not named in SDD-desktop-tauri §3.4) | **2.7.4** | 2026-10-21 | Apache-2.0 OR MIT | ok with the same `MPL-2.0` entries | none | 1.90 | **No** (F5) | `[RAN]` |
| `serde` direct dependency of `chess-cli` and `desktop/src-tauri` | cli config structs; desktop envelope (SDD-desktop-tauri §3.4 lists `serde`; SDD-cli §3.4 does not) | 1.0.229 (step 1) | | | | | | §3.4 row names `chess-domain`, `chess-store`, `chess-app` only | `[INF]` |

### 2.2 Rust dependencies the SDDs need but do not name

| Crate | Need (source) | Pin | Published | Licence | cargo-deny | Advisories | MSRV | Admitted | Tag |
|---|---|---|---|---|---|---|---|---|---|
| `base64` (DL2-3) | The protocol carries file contents inside frames "as base64 bytes" (SDD-app IFP-APP-03, ALG-APP-10, -11, D-49, architecture §5.7) | **0.23.1** | 2026-08-04 | MIT OR Apache-2.0 | ok | RUSTSEC-2017-0004 (patched `>= 0.5.2`) | 1.71.0 | **No, named in no document** (F5) | `[RAN]` |
| `getrandom` (DL2-4) | "OS random source" for `Core::open_default` and `new_game_uuid()` (SDD-app IFR-APP-07, §4.3; architecture §3.2 item 4; D-31 rejects `rand`) | **0.4.3** | 2026-06-17 | MIT OR Apache-2.0 | ok | none | 1.85 | **No** (F5) | `[RAN]` |
| `tempfile` (dev only) | "temp-directory store" in app, cli, desktop and server tests (architecture §9, D-39) | **3.27.0** | 2026-03-11 | MIT OR Apache-2.0 | ok | none | 1.63 | **No** | `[RAN]`. Already in the step-1 lock through `proptest`, so no new package, only a direct edge |
| `libc` (candidate, not proposed, DL2-14) | Possible `unsafe` module of `chess-store` and the CLI SIGTSTP raise, if the OQ-14 spike finds `std` not enough | 0.2.191 | 2026-10-16 | MIT OR Apache-2.0 | ok | none | 1.65 | No ("last", OQ-14) | `[RAN]`. Already in every tree through `tokio` |
| `rustix` (candidate, not proposed, DL2-14) | Same, safe wrappers | 1.1.6 | 2026-10-27 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | ok | none | 1.65 | No (OQ-14) | `[RAN]`. Already in the cli tree through `crossterm` |

### 2.3 Rust dev and test dependencies

| Crate | Used by (source) | Pin | Published | Licence | cargo-deny | Advisories | MSRV | Admitted | Tag |
|---|---|---|---|---|---|---|---|---|---|
| `proptest`, `insta` | engine, store, app tests (SDD-engine §3.4) | 1.12.0, 1.49.0 (step 1) | | | | | | Yes | `[RAN]` in step 1 |
| `expectrl` | CLI end-to-end through a PTY (SDD-cli E2E-CLI-01, -02; architecture §3.4 row "Test and lint tools") | **0.9.0** (newest) | 2026-05-11 | MIT | **fails the advisory check** with the repository `deny.toml`: the tree holds `nix` 0.26.4, which holds `pin-utils` 0.1.1 (RUSTSEC-2026-0441, unmaintained, "no safe upgrade") | none for `expectrl`; `pin-utils` per S2-E6 | none in the registry | Yes, version only; **never looked up** | `[RAN]` (F3, DL2-11) |
| `vt100` | CLI end-to-end screen parsing (same) | **0.18.1** (newest) | 2026-10-19 | MIT | ok | none | 1.85 | Yes, version only | `[RAN]` |
| `portable-pty` (alternative to `expectrl`, DL2-11) | | 0.9.0 | 2025-02-11 | MIT | ok | none at the DB date | none in the registry | No | `[RAN]` |

### 2.4 Rust workspace tooling

Step 1 section 2.3 covers `cargo-deny`, `cargo-audit`, `cargo-mutants`, `cargo-llvm-cov`, `rustfmt` and `clippy`. They are unchanged. `cargo-geiger` (unsafe census) is not installed and was not looked up `[UNV]` (F15).

### 2.5 npm packages (`ui/`, `desktop/`)

`ui/package.json` is a scaffold with no dependency (`[OBS]`, scaffold text and `package-lock.json` with the root only). Pins follow the same rule, from `npm view <pkg> time`. "Used by" names the SDD. All entries are MIT unless stated. `npm view` showed no `deprecated` field for any package `[RAN S2-E3]`.

| Package | Kind | Used by | Pin | Published | Licence | Newer than the pin | Engines and peers (registry) | Admitted in §3.4 | Tag |
|---|---|---|---|---|---|---|---|---|---|
| `react`, `react-dom` | runtime | SDD-ui §3.4, D-5 | **19.3.1** | 2026-10-14 | MIT | none | `react-dom` peer `react ^19.3.1` | Yes | `[RAN]` |
| `@lichess-org/chessground` | runtime | SDD-ui, `board/cg` only, D-4 (subject to OQ-9) | **10.5.0** | 2026-10-20 | **GPL-3.0-or-later** | none | engines `node >=24`, `pnpm >=12` (npm checks `node` only `[INF]`) | Yes | `[RAN]` |
| `@tauri-apps/api` | runtime | SDD-ui `transport/tauri.ts` | **2.12.4** | 2026-10-14 | Apache-2.0 OR MIT | none | none | Yes | `[RAN]` |
| `@tauri-apps/cli` | dev | SDD-desktop-tauri §3.4 | **2.12.4** | 2026-10-21 | Apache-2.0 OR MIT | none | engines `node >= 10` | Yes | `[RAN]` |
| `vite` | dev | SDD-ui | **8.3.5** | 2026-10-21 | MIT | 8.3.7 (2026-11-09) | engines `node ^20.19.0 \|\| >=22.12.0` | Yes | `[RAN]` |
| `@vitejs/plugin-react` | dev | SDD-ui | **6.1.2** | 2026-10-05 | MIT | 6.3.0 (2026-11-09) | peer `vite ^8.0.0` | Yes | `[RAN]` |
| `typescript` | dev | SDD-ui, SWE-001 ASM-S1 | **7.0.2** | 2026-07-08 | Apache-2.0 | none | **no programmatic API in 7.0** (S2-E15, vendor blog `[OBS]`) | Yes | `[RAN]`. **F1** |
| `@typescript/typescript6` | dev | **No SDD.** The vendor's side-by-side package for tools that need the TypeScript 6 API (DL2-8) | **6.0.2** | 2026-07-06 | Apache-2.0 | none | exposes `tsc6` | **No** | `[RAN]` |
| `eslint` | dev | SDD-ui, `no-restricted-imports` per folder | **10.13.0** | 2026-10-16 | MIT | 10.14.0 (2026-10-30) | engines `node ^20.19.0 \|\| ^22.13.0 \|\| >=24` | Yes | `[RAN]`. **F2** |
| `typescript-eslint` | dev | SDD-ui, architecture §3.4 | **8.72.1** | 2026-10-26 | MIT | 8.72.2 (2026-11-09) | **peer `typescript >=4.8.4 <6.1.0`**, runtime refuses TS 7 (F1) | Yes | `[RAN]` |
| `@eslint/js` | dev | **No SDD.** Provides `js.configs.recommended` for the flat config | **10.0.1** | 2026-02-06 | MIT | none | peer `eslint ^10.0.0` | **No** | `[RAN]` |
| `prettier` | dev | architecture §3.4 | **3.9.10** | 2026-10-10 | MIT | none | | Yes | `[RAN]` |
| `vitest` | dev | SDD-ui unit tests (UT-UI-01..04) | **5.0.5** | 2026-10-15 | MIT | 5.1.0 (2026-11-02) | engines `node ^22.12.0 \|\| ^24.0.0 \|\| >=26.0.0`; peers vite 6 to 8, optional `jsdom` and `happy-dom` | Yes | `[RAN]` |
| `@playwright/test` (the package that holds the runner) | dev | SDD-ui E2E and component tests; architecture §3.4 says "Playwright 1.64.1" without a package name | **1.64.1** (the `playwright` package has the same version) | 2026-10-14 | Apache-2.0 | 1.65.0 (2026-11-05) | engines `node >=20` | Yes (name `[INF]`) | `[RAN]`. F7, F20 |
| `@axe-core/playwright` and `axe-core` | dev | SDD-ui accessibility checks | **4.14.0** both | 2026-10-15, 2026-10-05 | **MPL-2.0** | none | engines `node >=22.12.0`; peer `playwright-core >= 1.0.0` | Yes (`@axe-core/playwright`) | `[RAN]` |
| `@testing-library/react` | dev | SDD-ui UF-UI-5 (D-27, "a component-test library") | **16.3.3** | 2026-08-27 | MIT | none | peers `react`, `react-dom`, `@types/react`, `@types/react-dom`, **`@testing-library/dom ^10`** | In principle | `[RAN]` |
| `@testing-library/dom` | dev | Peer of the line above | **10.4.2** | 2026-09-13 | MIT | none | | **No** (peer) | `[RAN]` |
| `jsdom` (DL2-7, the DOM environment) | dev | SDD-ui UF-UI-5 ("a DOM environment") | **30.1.3** | 2026-10-15 | MIT | 31.0.0 (2026-11-05) | engines `node ^22.22.2 \|\| ^24.15.0 \|\| >=26.0.0` | In principle | `[RAN]` |
| `happy-dom` (alternative) | dev | | 21.0.4 | 2026-10-20 | MIT | none | engines `node >=20` | No | `[RAN]` |
| `@testing-library/user-event`, `@testing-library/jest-dom` | dev | **Not proposed**: no SDD test needs them (keyboard scenarios are in Playwright) `[INF]` | 14.6.8; 7.0.1 | 2026-10-13; 2026-08-09 | MIT | | | No | `[RAN]` for the facts |
| `@types/react`, `@types/react-dom` | dev | Strict `tsc` over TSX (`ui/tsconfig.json` has `strict`) | **19.3.0** both | 2026-09-09 | MIT | none | `@types/react-dom` peer `@types/react ^19.3.0` | **No** | `[RAN]` |
| `@types/node` | dev | Node types for Vite and Vitest configs under strict `tsc` | **26.6.6** | 2026-10-28 | MIT | none | | **No** | `[RAN]` |
| `@vitest/browser-playwright`, `@vitest/browser` (DL2-12 alternative) | dev | Vitest browser mode in place of Playwright component mode | 5.0.5 | 2026-10-15 | MIT | 5.1.0 | peer `vitest 5.0.5`, `playwright *` | No | `[RAN]` |
| `@playwright/experimental-ct-react` (DL2-12 alternative) | dev | SDD-ui CT-UI-12 "Playwright component mode" | newest is **1.62.1** (2026-07-30); **no 1.64.1 exists** | | Apache-2.0 | | | No | `[RAN]` (F7) |
| `eslint-plugin-react-hooks`, `@vitest/coverage-v8` | dev | **Not proposed**: no SDD names a hooks lint rule or a UI coverage gate | 7.1.1; 5.0.5 | 2026-04-17; 2026-10-15 | MIT | | | No | `[RAN]` for the facts |

Resolved set (S2-E14, v3 = the recommended set of DL2-7 and DL2-8): 253 packages in the lock, 6 production (`react`, `react-dom`, `scheduler`, `@lichess-org/chessground`, `@tauri-apps/api` and the root), the rest dev or optional (`[RAN]`). Licences in that lock: MIT 159, Apache-2.0 43, MPL-2.0 14, Apache-2.0 OR MIT 13, BSD-2-Clause 8, ISC 7, BSD-3-Clause 3, MIT-0 2, BlueOak-1.0.0 2, GPL-3.0-or-later 1 (`chessground`), CC0-1.0 1. The 14 MPL-2.0 entries are `axe-core`, `@axe-core/playwright` and `lightningcss` with its 11 platform builds, all dev-only; none is in the production tree `[RAN]`. One package has an install script: `fsevents` 2.3.3 (macOS file watching, a dev dependency) `[RAN]`.

### 2.6 Notes on the Rust rows

Counts are unique `name version` lines of `cargo tree -e normal,build --target aarch64-apple-darwin --prefix none` without repeated `(*)` subtrees, and they include the probe crate itself `[RAN S2-E9]`. They are approximate and move with the lock.

| Block | Direct crates in the probe | Tree size (normal and build edges) | Transitive concerns | Tag |
|---|---|---|---|---|
| `chess-store` | `rusqlite`, `tokio`, `thiserror`, `serde`, `tracing`, `serde_json`, `uuid`, `crc32fast` | 36 | `libsqlite3-sys` needs a C compiler at build time (`cc`; its default feature `min_sqlite_version_3_45_3` also pulls the build crates `pkg-config` and `vcpkg`, which the bundled build does not use `[INF]`). `rusqlite` defaults include `ffi-sqlite-wasm-rs` (a WebAssembly shim) and `cache`; the probe uses `default-features = false` (F9). Whether the store needs `cache` (prepared-statement cache) is `[UNV]` | `[RAN]` |
| `chess-engine` | `tokio`, `thiserror`, `tracing` | 20 | `tokio` `process` brings `signal-hook-registry`, `mio`, `libc`. No network crate | `[RAN]` |
| `chess-app` | the above plus `tokio-util`, `futures-util`, `zip`, `sha2`, `base64`, `getrandom` (through `uuid` `v4` in the probe), `tracing-subscriber` (with `env-filter` in the probe), `tracing-appender`, `ts-rs`, `unicode-*` | 79 | `zip` brings `flate2`, `zlib-rs`, `zopfli` (with `deflate`), `getrandom`, `nix` 0.31.3, `typed-path`, `indexmap`. `env-filter` adds `matchers`, `regex-automata`, `regex-syntax` (DL2-6). `tracing-appender` adds `time` (with `deranged`, `num-conv`, `powerfmt`), `crossbeam-channel` and `symlink` `[RAN]`. The probe used `uuid` `v4`; DL2-4 recommends against it | `[RAN]` |
| `chess-cli` | `ratatui`, `crossterm`, `clap`, `toml`, `tokio`, `futures-util`, `serde`, `thiserror`, `tracing` | 102 | `ratatui` brings `ratatui-core`, `ratatui-widgets`, `ratatui-crossterm`, `ratatui-macros`, `lru` 0.18.5, `kasuari`, `instability`, `unicode-width`, `unicode-truncate`. `crossterm` brings `mio`, **`signal-hook` 0.3.18 and `signal-hook-mio`**, `rustix` 1.1.6 (D-47 rejects `signal-hook` only as a direct dependency). `clap` brings `anstream`, `anstyle`, `strsim`, `clap_lex` | `[RAN]` |
| `chess-server` | `axum` (3 features), `rust-embed`, `tokio`, `futures-util`, `thiserror`, `tracing` | 79 | `axum` `ws` brings `tokio-tungstenite` 0.29.0, `tungstenite` 0.29.0 (with **`rand` 0.9.5**, `sha1`, `data-encoding`, `httparse`), `hyper` 1.12.0, `hyper-util`, `http`, `tower`-family, `base64` 0.22.1. `rust-embed-impl` brings `sha2` 0.11.1, `walkdir`, `mime_guess`. No TLS, no HTTP-client crate. `rand` 0.9.5 is patched for RUSTSEC-2026-0097 (`>= 0.9.3`) `[OBS S2-E8]` | `[RAN]` |
| `desktop/src-tauri` | `tauri`, `tauri-build`, `tokio`, `serde`, `thiserror`, `tracing` | 222 | `wry` 0.57.1, `tao` 0.37.2, the `objc2` family (AppKit, WebKit), `brotli`, `png`, `plist`, `swift-rs` (build), `html5ever`/`markup5ever`, `cssparser`, `selectors`, `dom_query` (the MPL-2.0 crates of section 4), `tokio` 1.54.1 (Tauri uses it; one lock entry). **No `reqwest`, `hyper`, TLS or OpenSSL crate** on macOS `[RAN]`, which supports NFR-4. The Linux GTK stack (`gtk`, `glib`, `atk`, `webkit2gtk`) appears only when all targets are resolved | `[RAN]` |

The `rand` position: D-47 rejects `rand` and `rand_chacha` for the project's own seeded generator (the project owns `SplitMix64`). `rand` still appears in the workspace lock through `tungstenite` (0.9.5, server), a Tauri dependency (0.8.8) and `proptest` (0.10.3, dev). It is not a direct dependency of any crate and the `chess-domain` and `chess-rules` assertions (step 1 F6) are per-crate `[RAN]`; see F12.

## 3. Scan of tool trees

Not repeated. Step 1 section 3 covers the Rust tools. Two new tools matter to this step and were run, not installed: `cargo audit` and `cargo deny` against the scratch lock (S2-E6, S2-E7), and `npm audit` against the scratch npm lock (S2-E14). No tool was installed on the machine. The scratch files stay in the session scratchpad.

## 4. Licence view for `deny.toml` and the npm tree

### 4.1 Rust

Licences found in the 590-entry scratch lock, from `cargo metadata` `[RAN]`: `MIT OR Apache-2.0` (258), `MIT` (163), `Apache-2.0 OR MIT` (37), `MIT/Apache-2.0` (25), `Unicode-3.0` (18), `Zlib OR Apache-2.0 OR MIT` (17), `Unlicense OR MIT` (10), `Apache-2.0` (7), **`MPL-2.0` (5)**, `Apache-2.0/MIT` (5), `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` (5), `BSD-3-Clause` (3), **`Zlib` (2)**, `ISC` (1), `0BSD OR MIT OR Apache-2.0` (1), `CC0-1.0 OR MIT-0 OR Apache-2.0` (1), `MIT OR MPL-2.0` (1), `WTFPL` (1), `Apache-2.0 OR BSL-1.0` (1), and a few `AND` or `OR` forms with `BSD-2-Clause`, `BSD-3-Clause`, `Unicode-DFS-2016`, `LGPL-2.1-or-later` (as an `OR` choice). The six probe crates have no entry until given `license =` (fixed in the run that produced the table above).

With the **current** `deny.toml` allow list (`MIT`, `Apache-2.0`, `BSD-3-Clause`, `Unicode-3.0`, `Unlicense`, `GPL-3.0-or-later`), `cargo deny check licenses` rejects exactly these seven crates (S2-E7 (a)) `[RAN]`:

| Crate | Licence | Why it is in the tree | In the macOS tree | Linked into the shipped binary |
|---|---|---|---|---|
| `cssparser` 0.37.0, `cssparser-macros` 0.7.1, `dtoa-short` 0.3.5, `selectors` 0.38.0 | MPL-2.0 | `tauri-utils` → `dom_query` (HTML and CSS handling) | Yes | `[UNV]`: they are reached through `tauri-utils`, which is both a runtime and a build dependency of `tauri`; whether the code is linked, not only run at build time, was not determined. Treat as linked for the licence notice |
| `option-ext` 0.2.0 | MPL-2.0 | `dirs-sys` → `dirs` → `tauri-build` | Yes | Build-time only by the tree (`tauri-build`) `[INF]` |
| `foldhash` 0.2.0 | Zlib | `hashbrown` 0.17 → `indexmap`, `zip`, `ratatui-core`, `lru` | Yes | Yes |
| `zlib-rs` 0.6.8 | Zlib | `flate2` (feature of `zip`, and `png` in Tauri) | Yes | Yes |

`[INF]` `MPL-2.0` (file-level copyleft; MPL-2.0 section 3.3 names the GNU GPL as a "Secondary License" that a combined work may use) and `Zlib` (permissive) are compatible with a GPL-3.0-or-later project. This is a reading of licence names, not legal advice. The MPL obligation that remains is to keep the source of the MPL files available and to include the notice (NFR-6, architecture §8.2). Where the project puts these notices is not designed in any document `[UNV]`.

Everything else in the scratch lock passes: the `OR` expressions satisfy the allow list through the `MIT` or `Apache-2.0` branch. `ISC`, `0BSD`, `CC0-1.0`, `MIT-0`, `WTFPL`, `BSL-1.0` and `BSD-2-Clause` appear only inside `OR` or `AND` expressions that cargo-deny resolved without error `[RAN]`; the `WTFPL` and `BSL-1.0` entries were not traced to a crate `[UNV]`.

### 4.2 npm

No npm licence check exists in `ci/` (grep of `ci/check.sh` shows only `npm audit`) `[RAN]`. The licence census of section 2.5 was read from the lock fields. The production tree is MIT (`react`, `react-dom`, `scheduler`), Apache-2.0 OR MIT (`@tauri-apps/api`) and GPL-3.0-or-later (`chessground`). `[INF]` all compatible with GPL-3.0-or-later. The dev-only MPL-2.0, BlueOak, CC0 and MIT-0 entries are not distributed. The licence of `chessground` also couples the UI to D-10, as step 1 F2 says for `shakmaty`.

## 5. Findings

Severity as in step 1: High blocks the next step, Medium needs a decision or a document change before the stated point, Low is a note for the skeleton, Info is recorded only.

| # | Sev | Finding | Evidence | Needed from |
|---|---|---|---|---|
| F1 | **High** | **The baseline pair TypeScript 7.0.2 and `typescript-eslint` 8.72.1 cannot be installed or run together.** `typescript-eslint` (8.72.1 and the newest, 8.72.2) declares the peer `typescript >=4.8.4 <6.1.0`. `npm install` fails with ERESOLVE. With `--legacy-peer-deps` it installs, and ESLint then aborts with "typescript-eslint does not support TS 7.0". TypeScript 7.0 ships no programmatic API. The vendor's published route is side by side: `typescript` aliased to `@typescript/typescript6` (the TS 6 API for tools) and `@typescript/native` aliased to `typescript` 7.0.2 (`tsc`). That route worked here: `tsc` 7.0.2 type-checks, type-aware `typescript-eslint` runs. This changes two §3.4 rows (TypeScript, `typescript-eslint`) and adds one package. Consequence if ignored: the strict-`tsc` and ESLint gates of SWE-001 ASM-S1 cannot be wired (chess-dfi) | S2-E14, S2-E15 `[RAN]`; the vendor announcement text `[OBS]` (read from a saved page; the blog was not otherwise verified) | **G2, Brian Prascher** (DL2-8) |
| F2 | **High** | **`npm audit --audit-level=low` will fail on the baseline ESLint.** ESLint 10.13.0 (and the 10.14.0 line, `[UNV]` for 10.14.0, not audited) depends on `minimatch` 10.2.x, which GHSA-g3hv-75wg-m249 (CWE-405, denial of service, range `>=7.0.0 <11.0.1`, moderate) covers. The fix is `minimatch` 11, which no ESLint 10 line uses. `npm audit` offers only a downgrade to ESLint 9.39.5. All 10 moderate findings are in dev-only packages; with `--omit=dev` the result is 0. Exposure: ESLint matches file globs from the project's own configuration, and no attacker-controlled glob reaches it `[INF]`. `ui/package.json` and `ci/check.sh` use `npm audit --audit-level=low` on the full tree, so the first dependency entry turns CI red. This is chess-dfi. test-strategy line 88 already says a CVE in a dependency "that the app uses" fails the gate, which reads as the production tree `[INF]` | S2-E14 `[RAN]`; advisory text `[OBS]` via `npm audit --json` (CVSS score is 0 and the vector is empty in the registry data) | **G2, Brian Prascher** (DL2-9); a VEX record `VULN-001` is proposed below |
| F3 | Medium | **The repository `deny.toml` fails on the first `expectrl`.** `cargo-deny` 0.20.2 treats an `unmaintained` advisory in any transitive crate as an error by default. `expectrl` 0.9.0 → `nix` 0.26.4 → `pin-utils` 0.1.1 (RUSTSEC-2026-0441, archived upstream on 2026-10-01, "no safe upgrade is available"). Dev-only, no vulnerability. `cargo audit` shows it as a warning. Options in DL2-10 and DL2-11 | S2-E6, S2-E7, S2-E9, S2-E18 `[RAN]` | **G2, Brian** (DL2-10, DL2-11) |
| F4 | Medium | **`deny.toml` needs three changes before desktop or ZIP code, and one tidy-up.** (a) `MPL-2.0` (five crates) and `Zlib` (two) are not in the allow list (section 4). (b) `[graph]` has no `targets`, so Linux-only crates of Tauri's GTK stack (`proc-macro-error` unmaintained, `glib` unsound) enter the check for a macOS-only product; with the two Apple targets they disappear `[RAN]`. (c) The `unmaintained` scope (F3). (d) Step 1 left an unused-allow-entry warning; with these crates `BSD-3-Clause` and `Unicode-3.0` are used | S2-E7 `[RAN]` | **G2, Brian** (DL2-10) |
| F5 | Medium | **Crates and packages the SDDs need but no document names** (each is a G2 admission): `base64` (file contents travel as base64 in frames; no standard-library encoder exists); an OS random source for `new_game_uuid()` and the production `Rng` (D-31 rejects `rand`; `uuid` `v4` or `getrandom`); `tempfile` (dev; "temp-directory store"); `tauri-build` (Tauri 2 build script); `@types/react`, `@types/react-dom`, `@types/node` and `@eslint/js` (strict `tsc` and ESLint flat config need them in practice `[INF]`); `@testing-library/dom` (peer of the React testing library); `@typescript/typescript6` (F1). Also `serde` as a direct dependency of `chess-cli` (config structs through `toml`; SDD-cli §3.4 omits it, the D-47 table lists three crates for it `[INF]`) | Sections 2.1, 2.2, 2.5 | **G2, Brian** (DL2-3, DL2-4, DL2-13) |
| F6 | Medium | **SDD-app §4.4 says `tracing-appender` is used; its own design does not need it.** `tracing-appender` 0.2.5 rotates by time (`MINUTELY`, `HOURLY`, `DAILY`, `WEEKLY`) and prunes by file count; it has no size-based rotation `[OBS S2-E16]`. ALG-APP-14 specifies an own `RotatingFile` that rotates at 10 MiB into `.log.1` and `.log.2`. The crate could still wrap the writer in its non-blocking worker; whether that is wanted is a design call. If not wanted, the crate is an admitted dependency with no consumer | S2-E16 `[OBS]`; SDD-app lines 247 and 919 `[OBS]` | SDD owner (software-engineer) and Brian if the row is dropped (DL2-6) |
| F7 | Medium | **"Playwright component mode" (SDD-ui CT-UI-12) has no matching package at the baseline.** `@playwright/experimental-ct-react` stops at 1.62.1 (2026-07-30); no 1.64.1 exists. The runner package would be 1.64.1. The alternative that matches the baseline is Vitest browser mode (`@vitest/browser-playwright` 5.0.5, same release train as `vitest` 5.0.5). Also §3.4 says "Playwright 1.64.1" without naming `playwright` or `@playwright/test`; `@axe-core/playwright` needs `playwright-core` | S2-E3 `[RAN]` | **G2, Brian** (DL2-12) |
| F8 | Medium | **ZIP: three facts the design must carry.** (a) The default feature set of `zip` 9.1.3 pulls bzip2, lzma, zstd, ppmd, AES and `time`; the SDD wants deflate only. `deflate-flate2` alone does not compile; `deflate-flate2-zlib-rs` does (S2-E10). (b) Five RustSec advisories of 2025 and 2026 (patched `>= 9.0.0`, so the pin is clear) all sit in the `extract()` family; ALG-APP-11 should read entries one by one and never call `extract`, and a size limit on declared and actual uncompressed size per entry is the application's job (the "about 24 MiB" limit is on the frame, not on the unpacked size) `[INF]`. (c) `zip` 9.5.0 appeared on 2026-11-09; the pin rule keeps 9.1.3 | S2-E8, S2-E10 `[RAN]`; advisory text `[OBS]` | SDD owner (SDD-app ALG-APP-11); DL2-1 |
| F9 | Low | **`rusqlite` features and build prerequisites.** Defaults include `ffi-sqlite-wasm-rs` and `cache`; use `default-features = false` with `bundled`. `libsqlite3-sys` compiles `sqlite3.c` with the C compiler of the machine (`cc`); the release builder and CI need Xcode command-line tools `[INF]`. `build.rs` compiles in `SQLITE_ENABLE_LOAD_EXTENSION=1` (the C API exists; `rusqlite`'s `load_extension` feature stays off, so safe code cannot call it `[INF]`). `SQLITE_DEFAULT_FOREIGN_KEYS=1` is compiled in, and the store SDD also sets `foreign_keys=ON`, so they agree | S2-E12 `[OBS]` | Software-engineer at the first store entry |
| F10 | Low (positive) | **The bundled SQLite is 3.54.1.** CVE-2026-119375 (RUSTSEC-2026-0459; uninitialised page-cache memory returned for a database with a mismatched-page-size `-wal`) is fixed in SQLite 3.54.0 and in `libsqlite3-sys` 0.39.0; the pin is not affected. NVD lists CVE-2026-119586 for 3.54.1: command injection in the **CLI shell's `.recover`** output; the crate ships `sqlite3.c` only (no `shell.c`), so it is not affected `[OBS]`. Record both as `not_affected` (VULN-002, VULN-003 below). Evidence toward spike item S1: `json_valid` is in the bundled source and `SQLITE_ENABLE_JSON1` is set; no run was made, so S1 stays `[UNV]` | S2-E12 | None (record only) |
| F11 | Low | **`tokio` 1.54.0 is the affected release of RUSTSEC-2026-0384 and is yanked.** The pin 1.54.1 is patched. Use `tokio = "=1.54.1"` (an exact pin) so a lock refresh cannot select 1.54.0. RUSTSEC-2026-0386 deprecates `AsyncFd::new`; no SDD uses `AsyncFd`, and the workspace `warnings = "deny"` would turn a use of the deprecated constructors into a build error `[INF]` | S2-E8 `[RAN]` | None |
| F12 | Low | **`rand` and `signal-hook` are in the lock although D-47 rejects them.** D-47 rejects them as dependencies the project chooses; they arrive through `tungstenite` (`rand` 0.9.5), `proptest` (`rand` 0.10.3), Tauri (`rand` 0.8.8) and `crossterm` (`signal-hook` 0.3.18, `signal-hook-mio`). A reader of D-47 or of a future "no `rand`" check could read it as a violation. The pure-crate assertions are per crate (step 1 F6). State in §3.4 that D-47 rejects direct use | S2-E9 `[RAN]` | Architect (G0 wording) |
| F13 | Low | **`crossterm` has had no release since 2025-04-05** (19 months; 70 releases in all). 0.29.0 is the newest and the one `ratatui` 0.30.2 resolves to. No advisory exists. Whether upstream is active is `[UNV]` (the crates.io record is all that was read). Fallback: `ratatui` 0.30 has other backends in its tree (`ratatui-termwiz`, `ratatui-termina`), but the CLI SDD is written against `crossterm` | S2-E2 `[RAN]` | Watch; revisit trigger in DL2-5 |
| F14 | Low | **Tauri: unstated features, large tree.** The desktop SDD names no feature set. The default set resolves 222 crates on macOS and none is a network client `[RAN]`; the default set also names Linux and Windows features that do nothing on macOS. Whether `custom-protocol` or others are needed for a release bundle is `[UNV]`. `tauri` 2.12.5 needs Rust 1.90 (workspace 1.99, fine). D-13 says one tokio runtime per process; Tauri starts its own async runtime, so whether the shell can share one is `[UNV]` and is a design question for the shell's first task, not a lookup result | S2-E9 `[RAN]` | Software-engineer at the desktop task |
| F15 | Info | **Inputs to OQ-14 and chess-me6 (unsafe scope).** The standard library of 1.99.1 already provides `File::lock` (stable 1.89), `File::sync_all` as `F_FULLFSYNC` on Apple, `File::set_len`, `std::fs::hard_link`, and `CommandExt::process_group` (stable 1.64) `[OBS S2-E13]`. So `libc` or `rustix` may not be needed in `chess-store` at all; a no-replace rename via `renamex_np` is the one primitive `std` lacks, and D-83 names `link` then `unlink` as the alternative. If a crate is needed later, `libc` 0.2.191 is already in every tree and `rustix` 1.1.6 in the CLI tree, so admission adds a direct edge, not a package. For chess-me6 the evidence supports proposing `unsafe_code = "forbid"` at the roots of rules, domain, engine, app, cli, server (engine already says so), and leaving `chess-store` as `deny` until the spike reports, with one audited module as the only exception. This is evidence, not a decision: the spike (chess-a0r.3.1) and Brian decide. A census of `unsafe` in the dependencies needs `cargo-geiger`, which is not installed `[UNV]` | S2-E13 `[OBS]` | Spike owner; Brian for chess-me6 |
| F16 | Low | **`toml` and `uuid` details.** `toml` versions carry `+spec-1.1.0`; an exact `=1.1.8` requirement resolved correctly in the probe. `uuid` 1.29.0 compiles `Builder::from_random_bytes` without the `v4` feature `[RAN S2-E17]`; that supports DL2-4. `uuid` `serde` feature stays on for the store because journal and row DTOs carry ids | S2-E17 `[RAN]` | None |
| F17 | Low | **Duplicate crates (policy is `warn`).** With the six probes: `base64` (0.21.7, 0.22.1, 0.23.1), `sha2` (0.10.9 in Tauri, 0.11.1), `getrandom` (0.3.4, 0.4.3), `syn` (2, 3), `rand_core`, `indexmap`, `hashbrown`, `miniz_oxide`, `digest`, `crypto-common`, `block-buffer`, `cpufeatures`; with all targets also `toml`, `toml_edit`, `winnow`, `windows-sys`, `thiserror`, `heck`, `bitflags`. Tauri's pins cause most of them. No `deny` is warranted | S2-E7 `[RAN]` | None |
| F18 | Low | **`cargo-deny` `bans` stays out of scope here.** The edge bans in `deny.toml` cannot see a crate that a probe replaced, so the real `chess-*` edges were not exercised by this lookup (the probes are not the workspace crates). `ci/check-crate-edges.mjs` was not run `[UNV]` | S2-E7 | Software-engineer at the first dependency entry |
| F19 | Low | **Stale statements to update when this is accepted.** Architecture §3.4 marks the D-47 rows `[UNV]`; SDD-app OQ-APP-008 asks for these lookups; SDD-store §3.4 says "`uuid` `v5` use was not covered"; SDD-ui UF-UI-5 and SDD-chess-server UF-SRV-9 wait for versions; `ASM-CLI-1`, `ASM-SRV-3`, `ASM-TAU-1`, `ASM-UI-1/2`, `ASM-APP-02` say API names are "from memory" and this document does **not** verify any API (only versions, licences, advisories and trees) | Section 6 | SDD owners (G0 edits) |
| F20 | Info | **Not covered by any lookup or scan here:** Playwright browser binaries (downloaded by `playwright install`, outside `package-lock.json`, so outside `npm audit` and an npm SBOM) `[UNV]`; Stockfish 19 and its licence; fonts and piece sets; the ECO data file; `third_party/` assets; SBOM generation (`cargo-cyclonedx` or similar was not run); `cargo-geiger`; the real `Cargo.lock` and `package-lock.json` (none exist with dependencies yet) | | Later OQ-17 parts and the release task |
| F21 | Info | **Results are a point in time.** RustSec DB commit `d9cb059f…` (2026-11-09T17:13:08+01:00); `npm audit` run 2026-11-09 ~19:32 UTC. Twelve days after the baseline date. Re-run in CI and at release | S2-E6, S2-E14 | CI task |

### 5.1 Vulnerability records (VEX-style, from the `vulnerability-management` method)

Only candidates this lookup found. Status values are CycloneDX VEX states. None is an approval.

| ID | Component | Advisory | Scope, shipped | Analysis | Detail | Tag |
|---|---|---|---|---|---|---|
| VULN-001 | `minimatch` 10.2.6 via `eslint` 10.13.0, `@eslint/config-array` (npm, dev) | GHSA-g3hv-75wg-m249, CWE-405, moderate, range `>=7.0.0 <11.0.1`, no fixed 10.x | Dev tool, not shipped | `not_affected`, justification `code_not_reachable` (proposed) | ESLint passes only the project's own glob patterns; no untrusted pattern reaches it. Needs an entry in the Decision Log (test-strategy line 88) and a re-check when ESLint depends on `minimatch >= 11.0.1`. Triage is the Architect's proposal; acceptance is Brian's | `[RAN]` (audit), `[INF]` (reachability) |
| VULN-002 | `libsqlite3-sys` 0.39.1, SQLite 3.54.1 | CVE-2026-119375 / RUSTSEC-2026-0459 | Runtime, shipped | `resolved` (fixed in 3.54.0; the bundled copy is 3.54.1) | The project still opens `-wal` files in the data directory that a local user could alter; the fix is in the pinned version | `[OBS]` |
| VULN-003 | SQLite 3.54.1 CLI shell | CVE-2026-119586 (NVD, status "Analyzed"; CVSS v4.0 base 8.5 from a secondary source) | Not shipped (no shell source in the crate) | `not_affected`, `code_not_present` | `ls libsqlite3-sys-0.39.1/sqlite3` holds no shell source | `[OBS]` |
| VULN-004 | `pin-utils` 0.1.1 via `expectrl` (Rust, dev) | RUSTSEC-2026-0441, unmaintained | Dev only, not shipped | `not_affected` is the wrong word (there is no vulnerability); record as an accepted informational warning with an expiry, if DL2-10 option (b) is chosen | | `[RAN]` |
| VULN-005 | `glib` 0.18.5, `proc-macro-error` 1.0.4 (Rust, Tauri Linux stack) | RUSTSEC-2024-0429, RUSTSEC-2024-0370 | Not built for macOS | `not_affected`, `requires_environment` | Absent from the Apple-target trees `[RAN S2-E9]` | `[RAN]` |

## 5A. Decisions for unresolved crate and package choices

Each is a two-way door before code exists (R1). "Needs Brian" means a G2 admission or change (SWE-001 §3). Confidence is mine and says how much the evidence settles it.

**DL2-1. ZIP library and features (architecture D-15, "chosen after lookup").** Needs Brian (G2).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `zip` 9.1.3, `default-features = false`, feature `deflate-flate2-zlib-rs`.

| Option | For | Against |
|---|---|---|
| A. `zip` 9.1.3, `default-features = false`, `deflate-flate2-zlib-rs` | 15 crates; deflate only; maintained (37 releases in 12 months); patched for the 2025 and 2026 advisories; compiles `[RAN]` | The crate is large; `zlib-rs` and `foldhash` bring `Zlib` licence (section 4); `extract` must not be used (F8) |
| B. `zip` 9.1.3, feature `deflate` | The crate's own recommended deflate set | Adds `zopfli` and 5 more crates for compression we do not need (21 crates) |
| C. `zip` 9.5.0 | Newest | Dated 2026-11-09, after the pin rule |
| D. `rc-zip-sync` 4.4.2 (read only) or `async_zip` 0.0.19 | Smaller trees, other designs | No writer or no sync API; one crate per job fails; `async_zip` 0.0.x; maintenance and API unverified `[UNV]` |
| E. No ZIP; own container | No dependency | A security-relevant parser of untrusted input; violates the "format is a ZIP" requirement of IF-5 `[INF]` |

**Recommendation: A.** Confidence **Medium-High**: the facts (compiles, advisories patched) are `[RAN]`; that deflate-only output is accepted by other ZIP tools is `[UNV]`. Revisit trigger: a RustSec advisory against the 9.1 line, or a 9.x release the project needs.

**DL2-2. SHA-256 implementation (D-16, D-42).** Needs Brian (G2).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `sha2` 0.11.1, `default-features = false`.

| Option | For | Against |
|---|---|---|
| A. `sha2` 0.11.1, `default-features = false` | RustCrypto, MIT OR Apache-2.0, 1.2 billion downloads; already in the tree of `rust-embed-impl`; digest 0.11 stable | Tauri's tree holds `sha2` 0.10.9, so two versions in the desktop lock |
| B. `sha2` 0.10.9 | Unifies with Tauri | Older line, against the "latest" rule (CR-7.3) |
| C. `ring` or a C library | Fast | A C or assembly build and a different licence set; no consumer needs the speed |
| D. Own implementation | None | Cryptographic code written by hand to dedupe games; rejected by D-42's reasoning |

**Recommendation: A.** Confidence **High**. Revisit trigger: none expected.

**DL2-3. Base64 for file frames (D-49).** Needs Brian (G2). Not named in any document (F5).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `base64` 0.23.1 (`std`), only in `chess-app`.

| Option | For | Against |
|---|---|---|
| A. `base64` 0.23.1 (`std`) | Strict, widely used decoder; 1.9 billion downloads; MIT OR Apache-2.0 | A third `base64` version in the lock (axum 0.22, Tauri 0.21) |
| B. Own encoder and decoder in `chess-app` | No dependency | Hand-written parser for up to about 24 MiB of untrusted input; needs its own tests and review; the project principle "prefer std" has no std base64 to prefer |
| C. `data-encoding` | Already in the server tree through `tungstenite` | Not in the CLI or desktop trees; a second API for the same job |

**Recommendation: A**, only in `chess-app`. Confidence **Medium-High**. Duplicate versions are a `warn` (F17).

**DL2-4. OS random source and UUIDv4 (SDD-app IFR-APP-07; D-31 rejects `rand`).** Needs Brian (G2).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `getrandom` 0.4.3 directly in `chess-app` and `uuid::Builder::from_random_bytes`; `uuid` features stay `v5` and `serde`.

| Option | For | Against |
|---|---|---|
| A. `getrandom` 0.4.3 directly in `chess-app`, and `uuid::Builder::from_random_bytes` for `new_game_uuid()` and any v4 need; `uuid` features stay `v5` and `serde` | One new direct crate for both the `Rng` seed and the id; the `uuid` feature set stays the same in every crate, so the domain's "no random source" rule keeps its meaning under Cargo's feature unification; `from_random_bytes` compiles without `v4` `[RAN S2-E17]` | The crate is already in the lock through `uuid`'s optional `rng` and `zip`; a hand-built version-4 UUID (the helper sets the version and variant bits `[OBS]`) |
| B. `uuid` feature `v4` in `chess-app`, and `getrandom` still needed for the seed | The idiomatic call | Feature unification turns `v4` on for `chess-domain`'s `uuid` build as well, although the domain never calls it; `cargo tree -p chess-domain` stays clean but a workspace build does not `[INF]` |
| C. `rand` | Familiar | Rejected by D-31 |

**Recommendation: A.** Confidence **Medium**: the compile is `[RAN]`, the unification effect is `[INF]`. Revisit trigger: if the domain no longer asserts "no random source" by tree.

**DL2-5. `tokio` feature sets per crate (D-8).** Needs Brian (G2, as a batch with the table below). The SDDs give store and engine; the rest are proposals. Features unify across the workspace, so the effective set is the union.

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: the per-crate feature table as proposed (store and engine from the SDDs; app, cli, server, desktop as proposed).

| Crate | Features | Source | Tag |
|---|---|---|---|
| `chess-store` | `sync`, `rt`, `time` | SDD-store §3.4 | `[OBS]` |
| `chess-engine` | `process`, `io-util`, `sync`, `time`, `rt`, `macros` | SDD-engine §3.4 | `[OBS]` |
| `chess-app` | `rt-multi-thread`, `sync`, `time`, `macros` | proposal; ZIP and file I/O run on `spawn_blocking` (SDD-app §4.8), so no `fs` | `[INF]` |
| `chess-cli` | `rt-multi-thread`, `macros`, `signal`, `sync`, `time` | proposal (SDD-cli: `tokio::signal::unix`; one runtime per process, D-13) | `[INF]` |
| `chess-server` | `rt-multi-thread`, `net`, `sync`, `time`, `macros` | proposal (SDD-consistency review row D-8: `net`, `[UNV]`) | `[INF]` |
| `desktop/src-tauri` | whatever Tauri enables, plus `sync`, `time` | proposal; F14 | `[INF]` |

Compiles with the sets above `[RAN S2-E5]`. Which crate builds the runtime (and therefore needs `rt-multi-thread`) is a design fact the SDDs state only as "one per process" `[UNV]`.

**DL2-6. Logging crates (D-11, F6).** Needs Brian (G2) if a row is dropped.

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `tracing-subscriber` 0.3.23 (`fmt`, `std`, no `env-filter`); `tracing-appender` dropped; own `RotatingFile` per ALG-APP-14.

| Option | For | Against |
|---|---|---|
| A. `tracing-subscriber` 0.3.23 with `default-features = false`, `fmt`, `std` (no `env-filter`); drop `tracing-appender` and write `RotatingFile` as ALG-APP-14 says | Smallest tree (`tracing-subscriber` `fmt` alone: 7 crates `[RAN]`); no `regex`; matches the SDD design; no log-level environment variable surface | An admitted crate row is dropped |
| B. As A, plus `tracing-appender` only for its non-blocking writer around `RotatingFile` | Log writes never block a command | One more crate and a worker thread for a feature the SDD does not require; "a log write failure never fails a command" can be met without it `[INF]` |
| C. `env-filter` on | `RUST_LOG`-style filtering | Adds `matchers`, `regex-automata`, `regex-syntax` (about 5 crates); an environment-driven log filter is a configuration surface no requirement names |

**Recommendation: A.** Confidence **Medium** (the SDD owner may want B; the facts in F6 are `[OBS]`, the preference is mine).

**DL2-7. Component-test library and DOM environment (D-27, UF-UI-5).** Needs Brian (G2).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `@testing-library/react` 16.3.3 + `@testing-library/dom` 10.4.2 + `jsdom` 30.1.3.

| Option | For | Against |
|---|---|---|
| A. `@testing-library/react` 16.3.3 + `@testing-library/dom` 10.4.2 + `jsdom` 30.1.3 | Named by D-5's rationale (Testing Library); most complete DOM and accessibility-name behaviour `[INF]`; 237-package lock without `typescript-eslint`, no findings except the ESLint ones `[RAN]` | Larger than `happy-dom`; `jsdom` 31.0.0 is dated 2026-11-05, after the pin rule |
| B. Same with `happy-dom` 21.0.4 | 210-package lock under the same conditions `[RAN]`; faster `[INF]` | Less complete accessibility tree `[INF]`; a different semantics from a real browser for `getByRole` |
| C. No DOM environment; component tests only in a real browser (Vitest browser mode, DL2-12) | One fewer package set | Slower unit loop |

**Recommendation: A.** Confidence **Medium**. `user-event` and `jest-dom` are not proposed (F5).

**DL2-8. TypeScript and ESLint together (F1).** Needs Brian (G2); amends the §3.4 TypeScript and ESLint rows.

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. side by side: `typescript` = `npm:@typescript/typescript6@6.0.2` for `typescript-eslint`, and `@typescript/native` = `npm:typescript@7.0.2` for `tsc` (this is the decision on F1).

| Option | For | Against |
|---|---|---|
| A. Side by side: `typescript` = `npm:@typescript/typescript6@6.0.2` (for `typescript-eslint`) and `@typescript/native` = `npm:typescript@7.0.2` (for `tsc`) | The vendor's documented route; works `[RAN]`; keeps the baseline TypeScript 7.0.2 for the strict type check | One extra package; the lint runs on the TS 6 API, so a TS 7 only error would not be seen by lint; revisit when `typescript-eslint` supports TS 7.1 (its tracker is named in the error text) |
| B. TypeScript 6.0.3 only | One compiler | Departs from the TypeScript 7.0.2 baseline (CR-7.3) |
| C. Drop `typescript-eslint`; core ESLint plus strict `tsc` | Fewer packages | No type-aware lint rules (floating promises, unsafe `any`) that SWE-001 maps to TypeScript guidance |
| D. `--legacy-peer-deps` with TS 7 | None | Does not run `[RAN]` |

**Recommendation: A.** Confidence **High** for "D fails", **Medium-High** for A over B. Revisit trigger: `typescript-eslint` release that supports TypeScript 7.1 or later.

**DL2-9. What `npm audit` gates (F2, chess-dfi).** Needs Brian (G2 or a process decision).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. gate `npm audit --omit=dev --audit-level=low`; the full-tree audit is a non-gating report with `VULN-` records (this is the decision on F2).

| Option | For | Against |
|---|---|---|
| A. Gate: `npm audit --omit=dev --audit-level=low` (production tree); also run the full-tree `npm audit` and keep its findings as `VULN-` records with a VEX state | Matches test-strategy ("a dependency the app uses"); today 0 findings | A dev-tool vulnerability does not fail CI (it is still listed) |
| B. Gate the full tree | Strict | Red until ESLint changes `minimatch`; forces a waiver now |
| C. Gate the full tree with an allow-list file | Visible exceptions | A second ignore mechanism next to `deny.toml` |

**Recommendation: A.** Confidence **High**.

**DL2-10. `deny.toml` changes (F3, F4).** Needs Brian (G2, licence and advisory policy).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: the recommendation column: add `Zlib`; `MPL-2.0` by `[[licenses.exceptions]]` for the five crates; `[graph] targets` the two Apple targets; `unmaintained`: see the note in section 5B (the recommendation was "(b), or (c)").

| Change | Options | Recommendation |
|---|---|---|
| `Zlib` | add to the allow list | add (permissive) |
| `MPL-2.0` | (a) add to the allow list; (b) `[[licenses.exceptions]]` for `cssparser`, `cssparser-macros`, `dtoa-short`, `selectors`, `option-ext` | (b): a new MPL crate stays visible `[INF]` |
| `[graph] targets` | none (all platforms) or `aarch64-apple-darwin` and `x86_64-apple-darwin` | the two Apple targets (the product is macOS only, PLAT-1); proc-macro-error and glib leave the check `[RAN]` |
| `unmaintained` | (a) default (error on transitive); (b) `"workspace"` (only workspace-direct crates fail; transitive ones stay visible in `cargo audit`); (c) `ignore` entry for RUSTSEC-2026-0441 with reason and expiry | (b), or (c) if Brian wants zero relaxation. Both `[RAN]` |

Confidence **Medium-High**. The four "options" are recommendations for the `deny.toml` owner; this document edits no file.

**DL2-11. PTY test tool (D-3 style row "Test and lint tools": `expectrl` 0.9.0 and `vt100` 0.18.1).** Needs Brian (G2).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. keep `expectrl` 0.9.0 + `vt100` 0.18.1 together with the DL2-10 `unmaintained` handling.

| Option | For | Against |
|---|---|---|
| A. Keep `expectrl` 0.9.0 + `vt100` 0.18.1 with DL2-10 (b) or (c) | Admitted already; 21-crate tree | The tree holds the unmaintained `pin-utils` through a pre-1.0 `nix` 0.26 (F3) |
| B. `portable-pty` 0.9.0 + `vt100` 0.18.1 plus a small own "expect" loop | No advisory at the DB date | Last release 2025-02-11; larger tree (26 crates); the project writes the wait-for-text helper; maintenance `[UNV]` |

**Recommendation: A.** Confidence **Medium**; swapping tools is cheap before the CLI E2E task. Revisit trigger: `expectrl` or `ptyprocess` moving off `nix` 0.26, or the `pin-utils` advisory getting a fix.

**DL2-12. Component test runner (F7).** Needs Brian (G2).

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: A. `@vitest/browser-playwright` 5.0.5 (+ `@vitest/browser` 5.0.5) with Playwright 1.64.1. Confidence was Low-Medium; F7 stays open (packages file).

| Option | For | Against |
|---|---|---|
| A. `@vitest/browser-playwright` 5.0.5 (+ `@vitest/browser` 5.0.5), with the Playwright 1.64.1 already needed | Same version line as `vitest` 5.0.5; no version skew | CT-UI-12 wording "Playwright component mode" changes; API unverified `[UNV]` |
| B. `@playwright/experimental-ct-react` 1.62.1 with Playwright 1.62.1 | Matches the SDD wording | Two lines behind the baseline Playwright; the package is named "experimental" |
| C. Drop CT-UI-12 into an E2E test in `@playwright/test` against the built page | No extra package | Loses the component isolation of the adapter test |

**Recommendation: A.** Confidence **Low-Medium** (neither package was run). Revisit trigger: an `@playwright/experimental-ct-react` release at the baseline version.

**DL2-13. Dev-only packages named nowhere (F5).** Needs Brian (G2, one batch). Propose: `tempfile` 3.27.0; `@types/react` 19.3.0, `@types/react-dom` 19.3.0, `@types/node` 26.6.6; `@eslint/js` 10.0.1; `@testing-library/dom` 10.4.2; `@typescript/typescript6` 6.0.2 (DL2-8); `tauri-build` 2.7.4. Not proposed: `user-event`, `jest-dom`, `eslint-plugin-react-hooks`, `@vitest/coverage-v8`, `tokio-stream`. Confidence **High** that the first group is needed in practice `[INF]`; each is a dependency edge.

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: the batch as proposed: `tempfile` 3.27.0; `@types/react` 19.3.0, `@types/react-dom` 19.3.0, `@types/node` 26.6.6; `@eslint/js` 10.0.1; `@testing-library/dom` 10.4.2; `@typescript/typescript6` 6.0.2; `tauri-build` 2.7.4. Not admitted: `user-event`, `jest-dom`, `eslint-plugin-react-hooks`, `@vitest/coverage-v8`, `tokio-stream`.

**DL2-14. `libc` and `rustix`.** Not decided here. The OQ-14 spike owns it. The lookup provides versions, licences and the fact that neither adds a new package (F15). No admission is proposed.

> **v0.2: NOT YET A DECISION.** v0.1 proposed no admission. The OQ-14 spike (`spike-oq14-fsync-flock.md`, accepted) found `std` sufficient for `flock`, `F_FULLFSYNC`, `ftruncate`, directory sync and the no-replace rename (`hard_link` then `remove_file`), so **no `libc` or `rustix` is needed in `chess-store`**. Architect recommendation, still to be confirmed as a decision by Brian (G2): admit neither crate as a direct dependency anywhere now. The spike did not test the CLI's SIGTSTP raise (the cli needs `raise(SIGTSTP)` or an equivalent and `std` has no `raise`), so a `libc`/`rustix` need for `chess-cli` stays open, `[UNV]`; `libc` 0.2.191 and `rustix` 1.1.6 are already in the cli tree through `crossterm` and `tokio`, so admission would add an edge, not a package. See section 5B, note N3.

**DL2-15. WebSocket stack.** The server uses axum's `ws` feature, which brings `tokio-tungstenite` 0.29.0 and `tungstenite` 0.29.0 (MIT, no direct dependency of the project). No separate WebSocket crate is proposed. Confidence **High** (it is what the SDD names). Needs Brian only as part of accepting section 6.

> **v0.2: ACCEPTED by Brian Prascher, 2026-11-09 (G2).** Accepted option: no separate WebSocket crate; axum `ws` only (`tokio-tungstenite` and `tungstenite` 0.29.0 transitive).

## 5B. Decision record, v0.2 (2026-11-09)

Source: Brian Prascher, typed in session on 2026-11-09 and relayed by the coordinator (no verbatim quotation was supplied). Gate G2. Recorded by the system-architect agent; the Decision Log and `RELEASE_NOTES.md` entries belong to the systems engineer (proposed rows are in `step2-deps-findings-packages.md` section 7). Nothing below edits `Cargo.toml`, `package.json` or `deny.toml`.

| Item | Decision | Accepted option | Still open |
|---|---|---|---|
| DL2-1 to DL2-13, DL2-15 | ACCEPTED, G2 | The option stated under each DL2 heading above | See N1, N2, N4 |
| DL2-14 | **Recommendation only, to be confirmed** | None admitted (`std` is enough for `chess-store`) | N3 |
| F1 | Decided through DL2-8 A | TypeScript 7.0.2 via the vendor side-by-side route | N5 (follow-up) |
| F2 | Decided through DL2-9 A | `npm audit --omit=dev --audit-level=low` gates; full tree reports only | N5 (follow-up) |
| F3, F4, F5, F7 | **Open** | Options in `step2-deps-findings-packages.md` | Brian rules; task chess-a0r.3.2 stays open |

Notes (v0.2, new text):

- **N1. DL2-10 `unmaintained` row.** The v0.1 recommendation read "(b) `unmaintained = "workspace"`, or (c) an `ignore` entry". Brian accepted the recommendation without choosing between them. This is the F3 question, so it is carried into the F3 package and is **not** treated as settled. Licence (`Zlib`, `MPL-2.0` exceptions) and `[graph] targets` are settled.
- **N2. DL2-5, DL2-6 follow-ups.** DL2-6 A drops `tracing-appender` from the admitted list, which changes the §3.4 table and SDD-app §4.4 (P2, P9 of section 6). DL2-5 features for `chess-app`, `chess-cli`, `chess-server` and `desktop/src-tauri` were proposals labelled `[INF]`; they are accepted as the starting sets and each crate's first `Cargo.toml` entry still builds with them (ASM-DL2-4).
- **N3. DL2-14.** Spike facts: `File::lock`, `try_lock`, `unlock`, `File::sync_all` (`F_FULLFSYNC` on Apple), `File::set_len`, directory sync, and `hard_link` then `remove_file` work in safe `std` on rustc 1.99.1 (spike sections 1 and 3, `[OBS]` and `[RAN]`). Gaps: the spike used `crc32fast` 1.5.4 (this document's pin is 1.5.3, whose check value was run in S2-E17), did not exercise SIGTSTP, power loss, or an unsupported volume, and did not close S1 (bundled `json_valid`). Recommendation: confirm "no `libc`, no `rustix` admitted" for `chess-store`; take the CLI raise question to the CLI task. Not yet a decision.
- **N4. Evidence gaps carried into the accepted items.** DL2-12 A rests on no run of either package (`[UNV]`). DL2-4 A rests on a compile, with the Cargo feature-unification effect `[INF]`. DL2-1 A's deflate-only interoperability is `[UNV]`.
- **N5. F1 and F2 residual.** F1: exact pins for the two TypeScript aliases in `ui/package.json`, a CI step that prints both compiler versions, and the revisit trigger of DL2-8 (a `typescript-eslint` release supporting TypeScript 7.1 or later). The `tsc6` run in S2-E15 printed 6.0.3 while the package pinned here is 6.0.2; the reason (package version against compiler version) was not checked `[UNV]`. F2: `VULN-001` needs its VEX state and a re-check trigger (ESLint depending on `minimatch >= 11.0.1`); the full-tree report needs a place to go (a CI artifact); ESLint 10.14.0 was not audited `[UNV]`; test-strategy line 88 ("a dependency that the app uses") is read as the production tree and could say so. All of this implements bead chess-dfi; the manifest and CI edits belong to the software engineer.
- **N6. Not claimed.** This section does not claim that this document, any item of section 6 (P1 to P12) or the pins are accepted as a whole. P2, P4, P5 (DL2-11, DL2-12), P7 and P8 are now authorised in substance by the DL2 decisions, but the document edits themselves are not made here.

## 6. Proposed edits (proposal only; `docs/architecture.md`, the SDDs, `deny.toml`, `ci/` and the manifests were not edited)

All edits need a Decision Log entry and a RELEASE_NOTES entry as an amendment (CLAUDE.md). **G2** items wait for Brian Prascher. **G0** items record a lookup result for an already admitted dependency. IDs are not renumbered.

| # | Edit | Type |
|---|---|---|
| P1 | Architecture §3.4 baseline table: add rows (version, date, licence, MSRV) for `serde_json` 1.0.154, `crc32fast` 1.5.3, `tracing-subscriber` 0.3.23, `tokio-util` 0.7.20, `futures-util` 0.3.34, `clap` 4.6.7, `libsqlite3-sys` 0.39.1 (already in parentheses) with the SQLite release 3.54.1, `[RAN 2026-11-09]` | G0 (lookup of in-principle crates) |
| P2 | §3.4 baseline table: add the DL2-1 to DL2-4 crates once Brian chooses (`zip` 9.1.3 with the feature set, `sha2` 0.11.1, `base64` 0.23.1, `getrandom` 0.4.3), the dev crate `tempfile` 3.27.0, `tauri-build` 2.7.4, and the npm packages of DL2-7, DL2-8, DL2-12, DL2-13 | **G2** |
| P3 | §3.4: replace the `rusqlite` note "The SQLite release inside `libsqlite3-sys` 0.39.1 was not read `[UNV]`" with "3.54.1 `[OBS 2026-11-09]`; RUSTSEC-2026-0459 patched" | G0 |
| P4 | §3.4 TypeScript, ESLint and `typescript-eslint` rows: add the side-by-side route of DL2-8 and the version of `@typescript/typescript6` | **G2** |
| P5 | §3.4 "Test and lint tools": `expectrl` 0.9.0 and `vt100` 0.18.1 are looked up (S2-E11), with F3 as a note; Playwright named as `@playwright/test` 1.64.1 | G0 for the lookup, **G2** for DL2-11 and DL2-12 |
| P6 | §3.4 "Dependency admission": state that D-47 rejects `rand`, `rand_chacha`, `signal-hook` as direct dependencies and that they may appear transitively (F12) | G0 |
| P7 | `deny.toml` per DL2-10; add this document's section 4 as the source of the allow list | **G2** |
| P8 | `ui/package.json` and `ci/check.sh`: `npm audit --omit=dev --audit-level=low` as the gate, plus a full-tree audit run that writes a report (DL2-9) | **G2** (CI gate wording); the file edit itself is for the software engineer |
| P9 | SDD-app §4.4 and ALG-APP-14: decide the `tracing-appender` question (F6); ALG-APP-11: no `extract`, per-entry size limits (F8) | SDD owner; G1 inside one crate, G2 if a crate row goes |
| P10 | SDD-ui §3.4: replace "a component-test library and a DOM environment" with the chosen packages; CT-UI-12 wording (F7) | SDD owner; G1 |
| P11 | SDD-store §3.4: the `crc32fast` check value is verified (`0xCBF43926`, S2-E17); `uuid` `v5` feature is compiled `[RAN]`; remove the "was not covered" sentences | G0 |
| P12 | SDD-chess-server NF-SRV-1 ("no HTTP client in the tree"): define the check as no `reqwest`, `ureq`, `hyper-tls`, `rustls`, `native-tls`, `openssl`, `h2` in `cargo tree -e normal,build`; the tree contains `hyper` and `tungstenite`, which have client-capable code `[INF]`. Measured result: none present in the six probes `[RAN S2-E9]` | G0 text |

## 6A. Spike items carried (not lookup results)

| # | Item | Evidence from this lookup | Status |
|---|---|---|---|
| S1 | `json_valid` and the DDL `CHECK` in the bundled SQLite (step-1 spike S1, chess-a0r.3.1) | The bundled SQLite is 3.54.1, `sqlite3.c` contains `json_valid`, `build.rs` sets `SQLITE_ENABLE_JSON1` and no `SQLITE_OMIT_JSON` `[OBS]`. No `SELECT json_valid(...)` and no `CREATE TABLE ... CHECK` was run | `[UNV]` (partly informed) |
| S2 | `crc32fast` is the ISO-HDLC variant (FIND-R23-004, SF-08, UT-ST-081) | `0xCBF43926` over `"123456789"` with `crc32fast` 1.5.3 `[RAN S2-E17]` | **Answered** for this version |
| S3 | `File::lock`, a full sync, `ftruncate`, no-replace rename, directory sync (FIND-R23-004) | `std` reads in F15 `[OBS]`: lock, `sync_all` as `F_FULLFSYNC`, `set_len` exist. Whether `ftruncate` then `F_FULLFSYNC` behaves on APFS, whether a directory sync is exposed, and the no-replace rename were not run | `[UNV]` |
| S4 | A real-use check of `toml` line and column, `crossterm` `EventStream` with signals, `axum` WebSocket limits, `ts-rs` export, `zip` deflate interoperability | Versions, licences and advisories only; no API was exercised | `[UNV]` |

## 7. Rows and facts left `[UNV]`, and what would verify each

| Item | What would verify it |
|---|---|
| Behaviour of any crate API named in an SDD (ASM-CLI-1, ASM-SRV-3, ASM-TAU-1, ASM-UI-1/2, ASM-APP-02, the `unicode-*` behaviour, `toml` line numbers, `crossterm` `event-stream` feature name `[UNV]` in SDD-cli, `axum` 0.8 `ws` limits) | Compile and run a small program per crate in a scratch crate, then record it in the owning SDD |
| Whether the SDD code needs the `axum` default features this document proposes to drop, `rusqlite` `cache`, `tauri` feature set (`custom-protocol` and others) | The first `Cargo.toml` entry plus a build; `cargo tree -e features` |
| Whether the MPL-2.0 crates of Tauri are linked into the binary (section 4) | `cargo tree -e normal` for the release profile and a symbol or `cargo-bloat`-style check, or the licence audit of the built `.app` |
| Where the project places the MPL-2.0, Zlib and SQLite notices (NFR-6, §8.2) | The packaging task (chess-a0r.4.x) |
| Whether `zip` deflate-only archives open in common tools; zip-bomb limits | A test archive and the ALG-APP-11 tests |
| `tauri` Linux and Windows features resolve on macOS, `x86_64-apple-darwin` build | Intel Macs are not testable (CLAUDE.md); the target was only resolved in `cargo deny`, not built `[UNV]` |
| `cargo audit` yanked checks and the exact transitive versions at the baseline date | Re-run at the real `Cargo.lock` per crate |
| Upstream activity of `crossterm`, `ratatui`, `expectrl`, `portable-pty` beyond the crates.io release dates | Read the upstream repositories |
| `cargo-geiger` census of `unsafe` in the dependencies (chess-me6) | Install `cargo-geiger` (needs approval) and run it on the real lock |
| `@playwright/experimental-ct-react` vs `@vitest/browser-playwright` as the runner for CT-UI-12 | A spike of the adapter test in both |
| The npm `@tauri-apps/cli` platform packages and `@typescript/typescript-darwin-*` native binaries (optional dependencies) | `npm ls` in the real lock; integrity hashes are in `package-lock.json` |
| Playwright browser binaries, Stockfish, fonts, piece sets, ECO data (F20) | Their own lookups |
| An SBOM (CycloneDX) for either ecosystem | Generate from the real locks at release |
| The 2026-11-01 `crc32fast` 1.5.4, `zip` 9.5.0, `uuid` 1.29.2, `jsdom` 31.0.0, `vite` 8.3.7 and the other post-baseline releases | Not looked up beyond the registry record; any move to them is a normal change with a scan (§3.4) |

## 8. Assumptions

- ASM-DL2-1: The pin rule is "highest stable on or before 2026-10-28" for every row, as in §3.4. Under it, every pin equals the newest release today except `crc32fast` (1.5.3 against 1.5.4), `zip` (9.1.3 against 9.5.0), `uuid` (1.29.0 against 1.29.2, from step 1), `toml` (1.1.8 against 1.2.1), `vite`, `@vitejs/plugin-react`, `vitest`, `@playwright/test`, `eslint`, `typescript-eslint` and `jsdom`. Validation: Brian or the PM confirms, as for step 1.
- ASM-DL2-2: A scratch workspace in the session scratchpad with exact `=` pins on direct dependencies stands in for the future `Cargo.lock`. Transitive versions were resolved on 2026-11-09 and can differ from a lock made on the baseline date. The real locks must be re-scanned when each crate gets its first entry. No project file was changed, no dependency was added to any project, no `cargo` or `npm` command was run inside the repository, and nothing was installed on the machine (global tools and the repository checkout are untouched). `~/.cargo` registry and advisory caches and `~/.npm` caches were written by the lookups, as in step 1.
- ASM-DL2-3: The product targets macOS only (PLAT-1), so the licence and advisory view uses the targets `aarch64-apple-darwin` and `x86_64-apple-darwin`. Intel could be resolved but not built or run.
- ASM-DL2-4: Feature sets are taken from the SDD text where it states them (tokio for store and engine, `ws` for axum, `event-stream` for crossterm, deflate only for ZIP, `v5` for `uuid`). Where it does not, the set in this document is my minimal candidate, labelled `[INF]`, and is not a decision.
- ASM-DL2-5: The npm packages marked "dev" are not shipped in the product (the bundle is built by Vite). `[INF]`. This is what makes F2 a build-tool exposure, not a product exposure.
- ASM-DL2-6: The vendor page quoted in F1 was read once from a saved copy; the statement "TypeScript 7.0 ships no API" is also confirmed by the runtime message of `typescript-eslint` `[RAN]`, which is the evidence relied on.

## 9. Not analysed

Stockfish, fonts, piece sets and the ECO data; Playwright browser binaries; SBOM generation; `cargo-geiger`; the real lock files; an Intel build; API behaviour of any crate (section 7); the design of the licence notices; any review of the SDDs beyond their dependency lists; legal advice on licence compatibility.
