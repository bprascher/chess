# Ruling: where the isolated CHESS_DATA_DIR test helper lives

| Field | Value |
|---|---|
| Task | chess-0s0 (finding F-2, medium, from docs/engineering/step1-scaffolding-evidence.md line 108) |
| Stance | Architect (system-architect agent) |
| Date | 2026-11-06 |
| Gate class | **G1** (desk review by the system-architect agent; recorded as a desk review, not an approval by Brian). The ruling keeps the status quo and changes no dependency edge, no layout in architecture 3.1, no protocol and no ADR. The edits that apply it are G0 (test and CI support files). |
| Door type | Two-way. The helper is one std-only file; moving it later is a mechanical edit. |
| Approval | None claimed. Brian Prascher is not asked to approve anything here. See "G2 matters" below (none). |

## Question

Keep the `#[path]` include of `ci/test-support/isolated_data_dir.rs`, or use a per-crate `test-support` feature or module, or a shared crate?

## Drivers and constraints

- test-strategy.md (Determinism, line 82; item 6, line 31): every automated run uses an isolated data store (`CHESS_DATA_DIR`), never the user's. The rule is the same in every crate, so the helper's safety properties (fresh unique directory, removal guarded by canonical-temp-root and prefix, no `set_var`) should exist once. [OBS]
- Architecture 3.1 lists `ci/` as "enabling system" and has no test-support crate. Architecture 3.2 and `ci/check-crate-edges.mjs` accept only the listed crates; an unknown crate is reported as a layout change needing G2 (`check-crate-edges.mjs:39`). [OBS]
- Architecture section 4 items 3 and 4 already define a `chess-app` Cargo feature `test-support` for `Seams`, `Core::open`, the `FakeEngine` factory and "a helper that opens a Core over a temp-directory store", so that cli, server and src-tauri tests need no edge to engine or store. [OBS]
- SDD-chess-store DD-ST-024 and R-ST-2: store tests use a real `Store` over a temp directory plus `FaultPlan` behind feature `test-support`. [OBS]
- Coding standard: warnings, clippy and rustfmt are errors; suppressions need a written reason (CLAUDE.md, SWE-001). [OBS]

## Options

| # | Option | For | Against |
|---|---|---|---|
| A | Keep one file in `ci/test-support/`, included with `#[path]` | One copy of the safety logic. No Cargo edge, so nothing for the edge script or `cargo-deny` to see. No layout change. Already built and tested (4 tests). Path depth is the same (`../../../ci/...`) from `tests/` in `crates/<x>/` and `desktop/src-tauri/` | Path string repeated in each includer. Each includer compiles every item, so unused items raise `dead_code` (an error here). Not usable from a published crate (`cargo package`), but all crates are `publish = false` |
| B | Copy the helper into each crate's `test-support` feature or `tests/common` | Each crate self-contained | Five or more copies of code whose whole value is a deletion guard. A fix in one copy can miss the others (drift). A `test-support` feature compiles test code into the library's feature surface for something that is only needed by test targets |
| C | New `chess-test-support` crate | Normal Cargo reuse, no `#[path]` | New crate and edges from every crate's dev-dependencies. Deviates from architecture 3.1 and 3.2, fails the edge script and the `deny.toml` bans until both change. That is G2 for a 90-line helper |
| D | Put the helper in a crate that already exists and re-export (for example in `chess-domain`) | No new crate | Gives the pure crates a filesystem helper and makes `chess-store` and others depend on `chess-domain` for test code only. Distorts the "pure" crate and adds dev-edges the graph does not list. Rejected |

## Decision

**Option A.** Keep `ci/test-support/isolated_data_dir.rs` as the single source and include it with `#[path]` from the test targets that need an isolated data directory.

The two kinds of "test-support" are different and both stay:

1. **Generic temp-directory helper** (this file): the isolation guard. Shared by `#[path]`, std only, no Cargo edge.
2. **Domain test-support features** named in the SDDs (`FaultPlan` in `chess-store`, the `FakeEngine` factory and `Core`-over-temp-store helper in `chess-app`, perft and fixtures in `chess-rules`): code that needs crate internals and is feature-gated in that crate. These are not replaced by this ruling. Where one of them needs a temp directory (for example the `chess-app` helper that opens a `Core` over a temp-directory store), it uses the shared helper rather than a second implementation.

## Rationale

- The helper's value is that deletion is guarded in exactly one place. Option A is the only one that gives that with no new crate, no edge and no layout change.
- The edge script and `deny.toml` bans act on Cargo dependencies. A `#[path]` include creates none, so no check needs to change.
- The cost of A (a repeated path string, `dead_code`) is small and has a known fix (below). The cost of C is a G2 decision about a layout the architecture did not ask for.
- Scale: one file of about 90 lines, std only. Revisit if that stops being true.

## Consequences

- Every includer carries the line `#[path = "../../../ci/test-support/isolated_data_dir.rs"] mod isolated_data_dir;` in a top-level file of its `tests/` directory (or, for `chess-app`, inside the `#[cfg(feature = "test-support")]` module in `src/`, where the relative path differs; the software-engineer works it out and runs `cargo test`).
- The self-test stays in `crates/chess-rules/tests/isolated_data_dir.rs`. It is the only place the helper is tested. Other crates do not re-test it.
- Test targets in other crates must not use `#[path]` includes of anything else in `ci/` without a new ruling.
- A crate that does not touch the file system in its tests does not include the helper. `chess-domain` is pure (architecture 3.2, "no I/O"). Its tests probably need no data directory at all `[INF]`; the software-engineer includes the helper in chess-a0r.2.6 only if a test really needs one. The first real consumers are expected to be `chess-store` and `chess-app`.
- Front-end crates (`chess-cli`, `chess-server`, `src-tauri`) should prefer the `chess-app` `test-support` helper for in-process tests. They include the shared file only for process-level tests that must pass `CHESS_DATA_DIR` to a child process (`IsolatedDataDir::env()`).

## Required file changes

Only one, and it is small. It is applied when the second crate first includes the helper (chess-a0r.2.6 or the first later task that includes it), not now.

1. `ci/test-support/isolated_data_dir.rs`: add, after the module doc comment and before the `use` lines, an inner attribute with a reason, so an includer that uses only part of the API does not fail the warnings-as-errors build:

   ```rust
   #![allow(dead_code, reason = "shared by #[path] include; each test target uses a subset")]
   ```

   Check it against the workspace lint table: if `clippy::allow_attributes_without_reason` or a `warnings = "deny"` setting treats it differently, use the same text as an `#[allow(dead_code, reason = "...")] mod isolated_data_dir;` on the `mod` line in each includer instead. Confirm with `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --check` `[UNV until run]`.
2. Update the usage paragraph in the helper's module doc, and the "Helper location" row of docs/engineering/step1-scaffolding-evidence.md only if the software-engineer is already amending that file (that file is an evidence record; do not edit it just for this). Preferred: leave the evidence file as is and cite this ruling from the chess-a0r.2.6 evidence.
3. No change to `Cargo.toml` files, `ci/check-crate-edges.mjs`, `deny.toml`, architecture 3.1, any SDD or any baselined document. The `chess-rules` `test-support` feature (perft, DD-RUL-015) is unrelated and unchanged.

Optional, G0: the `check.sh` gate may add one line that fails if any file other than the helper defines `CHESS_DATA_DIR` removal logic. Not required.

## Revisit triggers

Reopen this ruling (new record, supersedes this one) if:

- the helper needs a third-party crate (for example `tempfile`), or grows past about 150 lines, or needs crate-specific API;
- a crate must be published, or built outside the workspace tree;
- more than about six includers exist and path drift causes a real defect.

Then the move is to a `chess-test-support` crate, which is a G2 change (architecture 3.1 layout, edge script, `deny.toml`) and goes to Brian with at least two options and a rollback plan.

## G2 matters for Brian

None. This ruling does not alter a dependency edge, the 3.1 layout, a protocol or an ADR. The only G2 path, Option C, is recorded here as a recommendation **not** to take, with the revisit triggers above. Finding F-A1 (perft tests in `src/testing.rs`) is not reopened.

## Not analyzed

- Whether `chess-app`'s `#[path]` from `src/` passes clippy and rustfmt as expected: [UNV], to be run in the task that adds it.
- Windows and Linux temp-directory behaviour: out of scope (macOS only).
