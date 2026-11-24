# GATE-106: chess-engine (implementation)

| Field | Value |
|---|---|
| Gate | **GATE-106** (numbering from GATE-101 per SWE-001 ASM-S5; satisfies chess-ckq C-7, readiness-step2-3.md C-7, for this block) |
| Tier | **G1** (one crate; architecture 3.2 edges unchanged; 5.7 contracts unchanged; see the Q-answers and FIND-E4) |
| Approver | System Architect, **desk review**, to be requested after the Reviewer pass |
| Status | **OPEN. NOT APPROVED.** No approval has been received from anyone. The author is the Engineer stance and does not approve its own gate |
| Date opened | 2026-11-09 |
| Opened by | Software engineer (task chess-a0r.3.10, Engineer stance, baseline B5, autonomy run-to-gate, CR chess-a0r.19) |
| Evidence | [step2-engine-evidence.md](../step2-engine-evidence.md) |

## CHG-106 Implement chess-engine per SDD-chess-engine 1.0 BASELINED (with the 2026-11-09 G0 notes)

- **Goal and driver:** ENG-1 to ENG-5, IF-4, NFR-1, NFR-3, NFR-8, PLAT-3, NFR-4, PLY-4, UI-1; D-9, D-13, D-15, D-18, D-33, D-43, D-46, D-47, FF-7, FF-8; functions F-3.1, F-3.4 (and the client side of F-3.2). Design entities DD-EN-001 to DD-EN-015 (all fifteen; see the map in the evidence file).
- **Components touched:** `crates/chess-engine` only (`Cargo.toml`, `src/`, `tests/`), plus `Cargo.lock`, which records the resolution of the admitted workspace dependencies this crate opts in to (`tokio`, `thiserror`, `tracing`, dev: `proptest`, `tempfile`). `Cargo.lock` is shared with the concurrent `chess-store` work, so its diff contains that crate's entries too. No other crate, no root `Cargo.toml`, `deny.toml`, `ci/` or other document was edited. The SDD "Code:" fields (SDD section 4) were **not** updated (the work order forbids editing other documents); the map is in the evidence file for the SDD owner.
- **Interfaces touched:** adds the provided interface IFP-EN-01 to IFP-EN-05. Nothing consumes it yet, so no existing contract changes. The crate manifest has no workspace dependency but `chess-rules` (no `engine -> domain`, no `engine -> app` edge).
- **Dependencies added:** none beyond the admitted workspace entries. Features: `tokio` `process, io-util, sync, time, rt, macros` (DL2-5, SDD 3.4). `tokio` `test-util` is **not** admitted and was not used (FIND-E2). No `libc`, `rustix`, `rand`, `tokio-util` or `unsafe` (`#![forbid(unsafe_code)]`).
- **Gate answers:**
  - Q1 no (one crate).
  - Q2 no (first implementation of the SDD cards; no existing interface changes). One interface-shape question is raised for the app owner: FIND-E4 (the `Engine` trait is not dyn-compatible).
  - Q3 no (the crate exists in 3.1; the member list is unchanged).
  - Q4 no (`node ci/check-crate-edges.mjs` -> `crate-edge check OK: 8 crates, 11 workspace edges, all allowed`).
  - Q5 no (the crate stores nothing).
  - Q6 no (actor per child process, command channel, `oneshot` replies, as SDD 3.5 and 3.10.2).
  - Q7 no (no ADR, QAS or architecture statement is contradicted). Two places where the code differs from SDD text are choices recorded as FIND-E3 (no `required-features` on `fake_uci`; no `stockfish-tests` feature) and FIND-E5 (the SDD hard-deadline rule is implemented as written and has a gap); both are for the SA desk review.
  - Q8 no (ordinary criticality, SWE-001). No stored format, public command or exit code is touched.
  - Q9 yes (new observable behaviour of one existing component, architecture intact).
  - Q10 no.
  - **Result G1.**
- **Design summary:** the SDD design as written. One tokio task per child owns stdin and stdout. `stop` goes over an unbounded command channel and is written ahead of every other pending command. A dropped `search` future is detected through its `oneshot` receiver, `stop` is written, and the abandoned `bestmove` is drained before the next `go`. Failures in flight are reported, never retried; the next request restarts the child. A handshake timeout restarts once. Every `setoption` is range-checked against the handshake table (Stockfish accepts out-of-range values silently). `PINNED_UCI_ELO_RANGE` is declared once, in `src/pinned.rs`, with a comment that it is a proposal that nobody has approved.
- **Options considered:** not required below G2. Two alternatives were weighed: (a) a `Mutex<ChildStdin>` shared by callers (rejected by SDD 3.5, a misuse tell); (b) a `required-features` gate on `fake_uci` (rejected, FIND-E3: the integration tests would silently not run in a plain `cargo test`).
- **Choices inside the SDD's freedom** (numbered F-E1 to F-E10 in the evidence file section 8, for the Reviewer and the desk review).
- **Escalation check (G2 triggers):** none found. FIND-E4 and FIND-E10 touch interfaces consumed later by `chess-app`; they are raised, not decided here. No stored format, protocol or journal is involved.
- **Evidence:** docs/engineering/step2-engine-evidence.md.
- **Tests added:** 114 tests, IDs UT-EN-001..006, 010..019, 020..026, 030..039, 040..047, 050, 060, 061, 062, FZ-EN-001, CT-EN-001..006, 010..017, 020, 021, 030..036, 040..043, 050..055, 060, 070..078, EI-EN-001..008. See the evidence file for the trace.
- **Risk and rollback:** working tree only, nothing committed. Rollback is `git checkout crates/chess-engine/Cargo.toml crates/chess-engine/src/lib.rs`, deleting the other new files under `crates/chess-engine/`, restoring `Cargo.lock` (shared with `chess-store`: remove only the entries that this crate added, or regenerate), and deleting this record and the evidence file.
- **Reviewer findings and dispositions:** none yet. The Reviewer pass is a separate stance and has not happened. The findings FIND-E1 to FIND-E13 in the evidence file are the Engineer's own list, not a Reviewer pass.

| Item | Status |
|---|---|
| Reviewer pass | NOT DONE |
| System Architect desk review | NOT REQUESTED yet (after the Reviewer pass) |
| `PINNED_UCI_ELO_RANGE` value | PROPOSAL, awaiting Brian Prascher (chess-a0r.3.4); not approved |
| Approved by | nobody |

## Rework record (chess-a0r.3.10, dispatch 2, 2026-11-10)

Author stance: Engineer. Input: docs/engineering/review-chess-engine-code.md (Reviewer, PASS WITH CONDITIONS, EF-01..EF-16). Details, commands and results: docs/engineering/step2-engine-evidence.md section 10.

- **Gate answers unchanged:** Q1..Q8 no, Q9 yes, **G1** for `crates/chess-engine`. Re-checked after the rework: no new dependency, no new crate edge, no `unsafe`, no new tokio feature (`rt-multi-thread` and `test-util` are still not admitted; the new concurrency tests use std threads that each own a current-thread runtime).
- **Observable changes in this crate that the SA desk review should read as one list** (all inside IFP-EN-01 to 05; no consumer exists yet, so none breaks a caller):
  1. New refusals: `SearchLimits::MAX_MOVETIME` (1 h, RULED by Brian Prascher 2026-11-10, chess-a0r.3.42), `SearchRequest::MAX_MOVES` (10 000, ESTIMATE), `EngineConfig::MAX_DURATION` (1 h, ESTIMATE). They return the existing `InvalidRequest` / `InvalidConfig` variants. Three new public associated constants.
  2. `SearchResult.pv/score/depth` now describe `best` (EF-08); `Cp(0)` with depth 0 now also means "no line named the returned move".
  3. `EngineError` has a hand-written `Debug` that hides engine text (EF-14).
  4. A write to the engine that does not complete within `handshake_timeout` kills the child and returns `SearchTimeout`.
  5. `UciEngine::shutdown` also interrupts start-up, preparation, a blocked write and a drain; queued commands behind it get `Closed`. `EnginePool::shutdown` and `release_game` run the instance shutdowns in parallel.
  6. Driver `select!` order: dropped future, hard timer, soft timer, commands, output (was: dropped future, commands, output, soft, hard). **SDD 3.10.2 text conflicts with this order and with the reviewer-confirmed deviation EF-13; the architect's parallel amendment must say so.**
  7. Analysis arbiter: stop requests are sent by the search's own task behind its `Search` command (lane signals). `Arbiter::stop` and preemption no longer call `UciEngine::stop` directly.
  8. `FakeEngine`: health, stop during latency, multipv cap 8, latency capped by movetime, deterministic `elapsed` (EF-12). One existing fake test changed its latency from 60 ms to 40 ms because the request movetime is 50 ms.
  9. `fake_uci` has a new `stderr-probe` mode (test tool only).
- **Outside the crate (EF-03):** `ci/check.sh` now stops with a clear message when the Stockfish binary is missing and exports `CHESS_REQUIRE_STOCKFISH=1`; `CHESS_ALLOW_STOCKFISH_SKIP=1` is the explicit opt-out. `.github/workflows/ci.yml` runs `third_party/stockfish/fetch.sh` before `ci/check.sh`. These are process files, not architecture elements; the reviewer asked for their own gate. **Treat the CI change as requiring Brian Prascher's decision (chess-85m owner), not as covered by this G1.** The hosted workflow was not run.
- **G0 item:** the doc comment of `PINNED_UCI_ELO_RANGE` in `src/pinned.rs` now states the value is final (Brian Prascher "Ok", 2026-11-09, architecture D-112). The constant and its single definition are unchanged.

| Item | Status |
|---|---|
| Reviewer pass on the rework | NOT DONE (needs a fresh Reviewer invocation) |
| System Architect desk review | NOT REQUESTED |
| CI change (ci/check.sh, ci.yml) | PENDING, Brian Prascher |
| Approved by | nobody |

## EF-03 CI diff (PENDING Brian Prascher's approval; NOT approved)

Brian keeps the CI-gate decision. Only my hunks are shown; `ci/check.sh` and `.github/workflows/ci.yml` also carry other, earlier uncommitted edits by others (`cargo deny` argument expansion, the `npm audit` guard, action pins), which are not part of this proposal. The hosted workflow was not run.

`ci/check.sh`, header comment (added before `set -euo pipefail`):
```
+# The real-Stockfish tests (EI-EN-001 .. 008, FF-7) must RUN, not skip. A missing binary is a gate
+# failure here: the script exports CHESS_REQUIRE_STOCKFISH=1 (the tests then panic instead of
+# skipping) and stops early with a clear message. Obtain the binary with
+# third_party/stockfish/fetch.sh (needs the network once; the pinned SHA-256 is enforced), or
+# point CHESS_STOCKFISH at an existing build. A machine that really has no Stockfish may opt out
+# explicitly with CHESS_ALLOW_STOCKFISH_SKIP=1; the skip is then printed, and the gate result is
+# not a full pass of FF-7.
```
`ci/check.sh`, new first step (after the `step()` helper):
```
+step "Stockfish binary (FF-7 / EI-EN-002)"
+stockfish_bin="${CHESS_STOCKFISH:-third_party/stockfish/build/extract/stockfish/stockfish-macos-universal}"
+if [[ "${CHESS_ALLOW_STOCKFISH_SKIP:-0}" == "1" ]]; then
+  unset CHESS_REQUIRE_STOCKFISH
+  echo "WARNING: CHESS_ALLOW_STOCKFISH_SKIP=1: the real-Stockfish tests may skip. This is NOT a full pass of FF-7."
+elif [[ -f "$stockfish_bin" ]]; then
+  export CHESS_REQUIRE_STOCKFISH=1
+  echo "found $stockfish_bin; CHESS_REQUIRE_STOCKFISH=1"
+else
+  echo "ERROR: the Stockfish binary is missing: $stockfish_bin" >&2
+  echo "  Run third_party/stockfish/fetch.sh (needs the network once; the pinned SHA-256 is enforced)," >&2
+  echo "  or set CHESS_STOCKFISH to an existing binary," >&2
+  echo "  or, on a machine without Stockfish, set CHESS_ALLOW_STOCKFISH_SKIP=1 to skip knowingly." >&2
+  exit 1
+fi
```
`.github/workflows/ci.yml`, before the `ci/check.sh` step:
```
+      # The real-Stockfish tests (FF-7, EI-EN-002) must run on the clean machine, not skip: ci/check.sh
+      # fails when the binary is absent. fetch.sh downloads the pinned Stockfish 19 release (NEEDS THE
+      # NETWORK) and enforces the pinned SHA-256 of the archive (exit 2 on a mismatch). It needs curl,
+      # tar, shasum, lipo and jq on the runner [UNV: not run on a hosted runner; chess-a0r.3.10 rework].
+      - run: third_party/stockfish/fetch.sh
```
Local confirmation `[RAN]`: binary present, `CHESS_CI_OFFLINE=1 ci/check.sh` exits 0; `CHESS_STOCKFISH=/nonexistent/sf ci/check.sh` exits 1 at the first step with the message above.

## CI change applied (2026-11-11)

Brian Prascher approved the EF-03 CI change on 2026-11-11 (relayed, bead chess-a0r.3.48). Applied in `ci/check.sh` (`ci.yml` unchanged): (1) new stage `cargo test -p chess-store --features test-support --locked` after the normal `cargo test` stage; (2) with `CHESS_ALLOW_STOCKFISH_SKIP=1` the end of the run repeats the warning and prints `All gates passed (Stockfish integration SKIPPED by opt-out).`; (3) offline flag and bash 3.2 safety kept (`${deny_args[@]+...}`, no `pgrep`). Evidence: `docs/engineering/step2-ci-evidence.md`.
