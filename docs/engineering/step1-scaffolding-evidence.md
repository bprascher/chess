# Step 1 scaffolding evidence (chess-a0r.2.2)

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/engineering/step1-scaffolding-evidence.md | 0.1 | 2026-11-05 | **G0 evidence record, not an approval.** Written by the software-engineer role (Engineer stance) for task chess-a0r.2.2 (baseline B4). The work is in the working tree, **not committed**. No Reviewer pass has been done on it | Software engineer role (Claude) | None for G0. Acceptance of the task is the program manager's check against the acceptance criteria, and any G1/G2 question below is for the system architect or Brian Prascher |

Evidence tags: `[RAN]` executed, `[OBS]` read, `[INF]` inferred, `[UNV]` not verified.

## 1. Gate note (chess-ckq condition C-7)

- Note only, no gate file created: **GATE-101 (proposed), tier G0, approver none, status RECORDED (not an approval).** Basis: the layout follows architecture 3.1 and 3.2 with no deviation (Q1 to Q7 answered "no": the crates are the baseline layout, not a new component; the edges are the 3.2 edges; no protocol, store or ADR is touched) `[INF]`. Empty stubs, configuration and tests only.
- Judgement calls inside the layout are listed in section 8 so that the system architect can overrule any of them. If the process requires a gate file under `docs/engineering/gates/` (the directory does not exist `[OBS]`), the program manager should say so.
- The ID `GATE-101` is only proposed here; the profile (ASM-S5) says gate records start at `GATE-101`. No other new ID was created.

## 2. What was built (all paths relative to the repository root)

| Item | Path |
|---|---|
| Workspace, allowed edges, workspace lints | `Cargo.toml`, `Cargo.lock` |
| Toolchain pin (1.99.1, stable only, rustfmt, clippy, llvm-tools) | `rust-toolchain.toml` |
| rustfmt, clippy configuration | `rustfmt.toml`, `clippy.toml` |
| cargo-deny (advisories, licences, bans = allowed edges, sources) | `deny.toml` |
| Eight crate stubs (no logic) | `crates/chess-{rules,domain,engine,store,app,server,cli}/`, `desktop/src-tauri/` |
| Crate-edge check | `ci/check-crate-edges.mjs` |
| Gate runner (local and CI) | `ci/check.sh` |
| GitHub Actions workflow | `.github/workflows/ci.yml` |
| Isolated `CHESS_DATA_DIR` helper and its self-test | `ci/test-support/isolated_data_dir.rs`, `crates/chess-rules/tests/isolated_data_dir.rs` |
| UI package scaffold (no dependency), `npm audit` config, strict `tsconfig` | `ui/package.json`, `ui/.npmrc`, `ui/package-lock.json`, `ui/tsconfig.json` |
| `/target/` ignored | `.gitignore` |

Third-party dependencies added to any Cargo.toml: **none** `[OBS]`. The `Cargo.lock` holds only the eight workspace packages. Nothing from `dependency-lookup-step1.md` was needed yet (`shakmaty`, `pgn-reader`, `serde`, `thiserror`, `uuid`, `ts-rs`, `proptest`, `insta` arrive with their crates' code tasks).

Crate edges declared (architecture 3.2): `cli -> app, server`; `server -> app`; `desktop -> app`; `app -> engine, store, domain, rules`; `engine -> rules`; `store -> domain`; `domain -> rules`. These are 11 edges `[RAN` edge check, section 5`]`.

## 3. Environment and what was fetched

| # | Command | Result | Tag |
|---|---|---|---|
| E1 | `cargo --version; rustc --version` (inside the repo, after the pin) | `cargo 1.99.1 (5f94df478 2026-08-27)`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | `[RAN]` |
| E2 | `rustup toolchain list` before the first `cargo build` | `stable-aarch64-apple-darwin (active, default)`, `1.89.0-aarch64-apple-darwin` | `[RAN]` |
| E3 | `rustup toolchain list` after | adds `1.99.1-aarch64-apple-darwin (active)` | `[RAN]` |
| E4 | `which cargo-deny node npm` | `/Users/brianprascher/.cargo/bin/cargo-deny`, `/opt/homebrew/bin/node`, `/opt/homebrew/bin/npm`; `cargo deny --version` = `cargo-deny 0.20.2`; `node` v26.13.0 | `[RAN]` |

**Network use to report.** (1) The first `cargo build` printed `the missing active toolchain 1.99.1-aarch64-apple-darwin has been auto-installed` and `downloading 6 components` (rustup, because `rust-toolchain.toml` pins the exact version while the installed toolchain is named `stable`). This is a global install of a toolchain identical in version to the installed `stable` (1.99.1); it is a side effect of the pin. Remove it with `rustup toolchain uninstall 1.99.1-aarch64-apple-darwin` if unwanted. (2) No crate was fetched: there are no third-party crates. (3) cargo-deny ran with `--offline` against the cached advisory database (commit date 2026-11-03T21:34:28+01:00 `[RAN git log -1 in ~/.cargo/advisory-dbs/advisory-db-3157b0e258782691]`), so **no advisory-DB fetch**. (4) `npm install --package-lock-only` and `npm audit --offline` (see section 6). No other network use.

## 4. Build, format, lint, test, deny (all on the empty workspace)

Run through the gate runner with the network-free option: `CHESS_CI_OFFLINE=1 ci/check.sh` (it sets `RUSTFLAGS="-D warnings"` and `RUSTDOCFLAGS="-D warnings"`). Exit status 0. Key lines of the output `[RAN]`:

| Step | Command | Result |
|---|---|---|
| format | `cargo fmt --all -- --check` | no output, exit 0. Before `cargo fmt --all` was applied to the helper and its test, the same command failed with a diff (the gate is live) |
| build | `cargo build --workspace --all-targets --locked` | `Finished dev profile [unoptimized + debuginfo] target(s)`; first build on the pin: `Compiling chess-rules ... chess-cli`, `Finished ... in 2.12s` |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `Finished dev profile`, no warning |
| test | `cargo test --workspace --locked` | every target `test result: ok.`; the only crate with tests, `chess-rules` `isolated_data_dir`: `4 passed; 0 failed`; all other targets `0 passed` |
| edges | `node ci/check-crate-edges.mjs` | `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` |
| deny | `cargo deny --offline check` | `advisories ok, bans ok, licenses ok, sources ok`. Five `license-not-encountered` warnings (`MIT`, `Apache-2.0`, `BSD-3-Clause`, `Unicode-3.0`, `Unlicense` are on the allow list for the later crates but unused today; `GPL-3.0-or-later` matches the workspace crates) |
| npm | skipped by `CHESS_CI_OFFLINE=1` in that run; see section 6 |

**Clippy gate is live `[RAN]`.** A deliberate `pub fn bad(v: Option<u8>) -> u8 { v.unwrap() }` appended to `crates/chess-rules/src/lib.rs` gave `error: used unwrap() on an Option value` (lint `clippy::unwrap_used`) and `error: could not compile chess-rules (lib)`. The line was removed and `cargo clippy ... -D warnings` is green again `[RAN]`. The lib file is back to its two-line content `[OBS]`.

Suppressions: **none** in any source or configuration file `[OBS]`. The workspace lint table sets `warnings = "deny"` and `clippy::all = deny`, plus deny on `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation`, `cast_sign_loss` (from SDD-chess-rules and SDD-chess-domain, "denied outside tests"); `clippy.toml` allows the first four in tests. Each crate has `#![forbid(unsafe_code)]` (SDD-rules and SDD-domain state it for those two; for the others see section 8, item 6).

Coverage tooling (`cargo-llvm-cov`) and mutation tooling were **not** run: no logic exists yet. The `llvm-tools` component is in the pin so that `cargo-llvm-cov` works.

## 5. Crate-edge enforcement, with deliberate failures

Two independent mechanisms, both configured from the 3.2 edge table:

1. `ci/check-crate-edges.mjs` reads `cargo metadata --no-deps --locked` and fails on any workspace-to-workspace dependency (normal, build or dev) that is not allowed, on an unknown workspace crate and on a missing one. It also guards the leaf rule (nothing may depend on `chess-cli` or `chess-desktop`).
2. `deny.toml` `[bans] deny` with `wrappers` for each crate that has dependents (`chess-rules`, `chess-domain`, `chess-engine`, `chess-store`, `chess-app`, `chess-server`): only the listed direct dependents are allowed. cargo-deny cannot express "no dependents" (an empty `wrappers` bans the crate outright: tried, `chess-cli` and `chess-desktop` were reported as banned), so the leaves are covered by mechanism 1 only. Wrappers match direct parents only `[INF from the message "direct parent ... was not marked as a wrapper"]`.

**Deliberate bad edges `[RAN]`.** Two edges were added to Cargo.toml files: `chess-store -> chess-rules` (forbidden by D-38 item 1: the store has no edge to the rules) and `chess-cli -> chess-domain` (forbidden by D-38 item 1: front-ends depend only on `chess-app`). (A first attempt put the line under `[lints]` and failed as a manifest error, not as an edge error; it was redone correctly.)

```
$ node ci/check-crate-edges.mjs
FAIL disallowed edge: chess-store -> chess-rules (normal)
FAIL disallowed edge: chess-cli -> chess-domain (normal)
crate-edge check FAILED (2 problem(s))                       exit 1

$ cargo deny --offline check bans
warning[unmatched-wrapper]: direct parent 'chess-cli = 0.0.0' of banned crate 'chess-domain = 0.0.0' was not marked as a wrapper
error[banned]: crate 'chess-domain = 0.0.0' is explicitly banned
warning[unmatched-wrapper]: direct parent 'chess-store = 0.0.0' of banned crate 'chess-rules = 0.0.0' was not marked as a wrapper
error[banned]: crate 'chess-rules = 0.0.0' is explicitly banned
bans FAILED                                                  exit 2
```

Both manifests and `Cargo.lock` were restored from copies taken before the change. After the revert: `node ci/check-crate-edges.mjs` gives `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`, `cargo deny --offline check bans` gives `bans ok`, `cargo build --locked` builds `[RAN]`.

## 6. npm audit (ui/)

- The layout has a `ui/` package, so `ui/package.json` (private, `engines.node >= 26.13.0`, script `audit`), `ui/.npmrc` (`audit-level=low`, `engine-strict=true`) and `ui/package-lock.json` (lockfile v3, root package only) exist. `ui/tsconfig.json` sets `strict` and the extra strict flags `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` and others `[OBS]`.
- `cd ui && npm install --package-lock-only --ignore-scripts` -> `up to date, audited 1 package`, `found 0 vulnerabilities` `[RAN]`. `cd ui && npm audit --offline` -> `found 0 vulnerabilities` `[RAN]`.
- **That result is vacuous.** The package has no dependency, and `--offline` means the registry was not queried. A real `npm audit` against the registry has **not** been run here (the task limits network use). It runs in `ci/check.sh` (without `CHESS_CI_OFFLINE=1`) and in the CI workflow `[UNV]` until the first CI run.
- **Not created, on purpose:** TypeScript, ESLint, `typescript-eslint`, Prettier, Vite, React, Vitest. No npm package has been looked up (`dependency-lookup-step1.md` scope: "all npm packages" not covered; F13). The strict `tsc` and ESLint **run** steps, `eslint.config.*` and `ui/src/` do not exist yet. `tsc` is not on the path (SWE-001). `ui/src/protocol/` (generated by `ts-rs`) was not created. `tsc -p ui` with `"include": ["src"]` and no `src` would report no inputs; it is not run by any gate yet.

## 7. Isolated `CHESS_DATA_DIR` helper

`ci/test-support/isolated_data_dir.rs` (std only): `IsolatedDataDir::new(label)` creates a fresh directory in the canonical OS temp directory named `chess-test-<clean label>-<pid>-<counter>-<nanos>` with `create_dir` (fails if it exists); `path()`, `env()` (the `("CHESS_DATA_DIR", path)` pair for `Command::env`); removed on drop only if the path is directly inside the canonical temp root and carries the prefix. It never reads the real data directory and never calls `set_var` (unsafe in edition 2024, racy). A crate includes it with `#[path = "../../../ci/test-support/isolated_data_dir.rs"] mod isolated_data_dir;`. Self-test `crates/chess-rules/tests/isolated_data_dir.rs`: distinct existing directories under temp; removed on drop; a label such as `../../etc/x y` cannot escape; the env pair names the variable and the process environment is unchanged. `cargo test` -> `4 passed; 0 failed` `[RAN]`. After the run no `chess-test-*` directory was left in the temp directory (`ls <tmpdir> | grep -c chess-test-` -> `0`) `[RAN]`.

Limit: it isolates the directory only. It does not prove that a future `chess-store` resolver honours `CHESS_DATA_DIR` or that a test cannot fall back to the real directory; that is for the store code and its tests `[UNV]`.

## 8. Judgement calls, deviations and findings for the architect

| # | Sev | Item |
|---|---|---|
| 1 | Medium | **Helper location.** Architecture 3.1 has no shared test-support location, and a new `chess-test-support` crate would be a layout deviation, so the helper is a single source file in `ci/test-support/` included by `#[path]`. The SDDs name per-crate `test-support` features (store `FaultPlan`, domain fakes) that may be the better long-term home for temp stores. The architect may want to rule on this before the second crate includes it. Not a gate item unless a crate is added |
| 2 | Medium | **CI platform.** No document names the CI product. GitHub Actions is used because the remote is `github.com` `[OBS git remote -v]`; the workflow sits in `.github/workflows/`, a path outside the 3.1 tree (3.1 puts CI configuration in `ci/`, which holds the scripts). Third-party actions `actions/checkout@v4` and `actions/setup-node@v4` are referenced by tag, **not pinned by SHA and not vetted** `[UNV]`; the workflow has never run `[UNV]`. `cargo install cargo-deny --version 0.20.2` runs without `--locked` on purpose (lookup F5). `macos-latest` is an assumption `[UNV]`. Decision needed: keep GitHub Actions, and whether to pin action SHAs |
| 3 | Low | **Desktop crate name.** The package is `chess-desktop`, the SDD's own suggestion (OQ-TAU-002, decided at packaging, OQ-11). It appears in `deny.toml` and the edge script and must be renamed in both if the name changes. No Tauri dependency or `tauri.conf.json` exists (Tauri is not admitted, OQ-17 part 2) |
| 4 | Low | **Edges declared as dependencies now.** The 11 edges are real `path` dependencies although no code uses them yet, so the checks have something to check. `unused_crate_dependencies` is allow-by-default in rustc, so there is no warning `[RAN, clippy and build clean]` |
| 5 | Low | **`deny.toml` limits.** Leaves cannot be banned (section 5). `multiple-versions = "warn"` (lookup F9). The licence allow list is the lookup's section 4 list; five entries are unused today. `[licenses.private] ignore = false` so workspace crates are checked too. `advisories` has no ignore entries |
| 6 | Low | **`#![forbid(unsafe_code)]` on every crate.** Stated for `chess-rules` and `chess-domain` in their SDDs `[OBS SDD-chess-rules.md:80, SDD-chess-domain.md:77]`. For `chess-store` the architecture defers `flock`/`F_FULLFSYNC` to the OQ-14 spike (3.4: "libc last, with one documented unsafe module"). If that spike needs `unsafe`, the store crate attribute must change by a recorded decision. The other SDDs were not read for this `[UNV]` |
| 7 | Low | **Lint set applied workspace-wide.** The deny list for `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` and the two cast lints comes from the rules and domain SDDs. Whether it applies to the other crates, and the domain's "arithmetic that can overflow" (`clippy::arithmetic_side_effects`), is not decided here: that lint was left out because the rules SDD does not name it and it is noisy. Needs the architect or the owners of those SDDs |
| 8 | Info | **Rules SDD is 0.3.5 DRAFT, not baselined** (CLAUDE.md). Only crate names and edges were taken from it |
| 9 | Info | `.beads/issues.jsonl` shows as modified in `git status`; it was modified before this task started (initial git status in the session) and was not touched here |

## 9. Acceptance criteria

| Criterion | Result | Evidence |
|---|---|---|
| Workspace per architecture 3.1/3.2 with only the allowed crate edges (script or cargo-deny bans) | Met, with the judgement calls of section 8 | Section 2, 5; edge check OK (11 edges); `bans ok`; both fail on deliberate bad edges |
| rustfmt config | Met | `rustfmt.toml`; `cargo fmt --check` exit 0 |
| clippy `-D warnings` config | Met | `Cargo.toml` `[workspace.lints]`, `clippy.toml`, `RUSTFLAGS`; green; red on a deliberate `unwrap` |
| cargo-deny config | Met | `deny.toml`; `advisories ok, bans ok, licenses ok, sources ok` (offline, cached DB of 2026-11-03) |
| npm audit config | Partly met | Configured (`ui/.npmrc`, script, lockfile, CI step). Run only offline on an empty tree, so the result proves nothing about the registry `[UNV]` |
| Isolated `CHESS_DATA_DIR` test helper | Met | Section 7; 4 tests pass |
| Build passes empty on Rust 1.99.1 | Met | `rustc 1.99.1`; `cargo build` `Finished`; `cargo test` ok |
| No suppression without reason | Met | No suppression exists |
| Evidence: command output recorded | Met | Sections 3 to 7 |

## 10. Not run, not analyzed, assumed

- **Not run:** a real `npm audit` against the registry; the CI workflow (never executed on GitHub); `cargo deny check` with an advisory-DB fetch (offline, cached DB); `tsc`, ESLint (not installed, not admitted); `cargo-llvm-cov`, `cargo-mutants` (no logic); Intel (`x86_64`) build; any macOS version other than the development Mac.
- **Not analyzed:** the SDDs of store, engine, app, cli, server, ui and desktop-tauri beyond names and edges; the Tauri crate layout; the Reviewer pass (not done; this task is G0 and the Engineer does not review own work).
- **Assumed:** ASM-S1 and ASM-S2 of SWE-001 (the standard checks are rustfmt, clippy, strict tsc, ESLint; warnings are errors; the user confirms at the first CI run); GitHub Actions as the CI product; `license = "GPL-3.0-or-later"` for every workspace crate (lookup F2, D-10).
