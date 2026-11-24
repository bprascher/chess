# G1 desk review, step 2 part A: GATE-105 (store A), GATE-106 (engine), GATE-107 (store D1)

| Field | Value |
|---|---|
| Reviewer | System architect agent, desk review (SWE-001 gate G1: conformance to the SDD, crate edges, architecture 5.7 contracts) |
| Date | 2026-11-13, baseline B6 |
| Scope | [GATE-105](gates/GATE-105-chess-store-A.md) (part A with rework 1 and 2), [GATE-106](gates/GATE-106-chess-engine.md) (engine with rework), [GATE-107](gates/GATE-107-chess-store-D1.md) (store D1). **Not in scope:** GATE-108 (store B), GATE-109 (D2), GATE-110 (D5) |
| Status of this document | **A desk review. It is not G2 or G3 approval and approves nothing that needs Brian Prascher.** It records the architect's G1 verdicts as a review outcome; the gate records stay OPEN until the PM and Brian close them |
| Independence note | I (the system-architect role) also wrote the SDD dated notes that this review checks the code against (store SDD notes 1 to 17, engine AN-EN-01 to 08). The code-to-SDD comparison for those notes is therefore **not independent**; items marked "second reader" in the conditions ask Brian or a fresh Review Board stance to read that text |
| Evidence tags | `[OBS]` read, `[RAN]` executed by me now, `[INF]` inferred, `[UNV]` not verified |

## 0. Verdicts

| Gate | Verdict | Open reviewer Majors | Open conditions |
|---|---|---|---|
| **GATE-105** store part A (rework 1 and 2) | **PASS WITH CONDITIONS** | none (FIND-001, 002, 003 FIXED per `review-chess-store-A-rework.md`) | C-105-1 to C-105-5; two items go to Brian |
| **GATE-106** engine (with rework) | **PASS WITH CONDITIONS** | none (EF-01, 02, 03 partial, 04 fixed per `review-chess-engine-rework.md`; EF-03 is a process item) | C-106-1 to C-106-5; two items go to Brian |
| **GATE-107** store D1 | **PASS WITH CONDITIONS** | none, **but no Reviewer pass exists for D1** | C-107-1 to C-107-3 |

"PASS WITH CONDITIONS" means: the code conforms to the SDD, the edges and the 5.7 contracts for what these gates cover, and I record a G1 desk-review outcome of pass once the conditions are met or consciously accepted by the named owner. It does not close any gate by itself.

## 1. Checks common to all three gates

| Check | Result |
|---|---|
| Crate edges unchanged | `node ci/check-crate-edges.mjs` `[RAN]` this session: "crate-edge check OK: 8 crates, 11 workspace edges, all allowed". Manifests `[OBS]`: `chess-store` names `chess-domain` only; `chess-engine` names `chess-rules` only; `ci/check-crate-edges.mjs` lines 13 to 15 list `chess-engine: [chess-rules]` and `chess-app` above both. `deny.toml` wrappers for `chess-engine` (app only) and `chess-rules` (domain, engine, app) agree. No `chess-store -> chess-rules` edge (D-38, R-ST-12) |
| D-38 (no re-export of rules by store or domain) | Holds for the store `[OBS]`: no `chess_rules` in the store's manifest or `lib.rs`; the store keeps `StoredSide` and, in D5 (not in scope), `StatsSide`. The engine uses `chess_rules` in `types.rs` (one reference) and is consumed by `chess-app`, which holds the edge |
| D-40 (UUID-only boundary) | Holds for the public surface I read `[OBS api.rs, api_analysis.rs, error.rs, lib.rs]`: ids are `GameId` and `PlayerId`; the integer fields are counters, times and a pid (`journal_seq`, `final_journal_seq`, `*_ms`, `heartbeat_at`, `pid`), not row keys. `VariationRef(pub(crate) u64)` in D5 wraps a row key without exposing it (see the D5 package; not in scope here) |
| Architecture 5.7 `Store` handle row | One concrete clonable `Store`, no `dyn Store`, UUID-only API, statistics rows filtered by structural keys, failures `StoreBusy`/`StoreUnavailable`/`SchemaTooNew`. Conforms for part A `[OBS]` (`Store`, `OpenedStore`, `StoreOptions`; `StoreError` has `Busy`, `Unavailable{cause}`, `SchemaTooNew`, `Corrupt`) |
| Architecture 5.7 `Engine`/`EnginePool` row | `search`, `stop`, `health`; `EnginePool::computer_side`, `analysis(priority)`; typed `EngineError`; `EnginePersonality` is the engine's type. Conforms `[OBS lib.rs exports]`. The row says nothing about dyn-compatibility; the app SDD note of 2026-11-11 (D-121, `AppEngine` enum) settles it (FIND-E4 closed, `chess-engine` unchanged) |
| Not re-run by me | `cargo test`, clippy, mutants, coverage, `ci/check.sh`: I relied on the evidence files and the two Reviewers' own runs `[UNV]` |

## 2. GATE-105: chess-store part A

**What I read.** `gates/GATE-105-chess-store-A.md` (with rework records 1 and 2), `step2-store-A-evidence.md` (headings, sections 7, 8, 12, 13 by grep and partial read), `review-chess-store-A.md` (verdict, FIND-001 to 015, section 4) and `review-chess-store-A-rework.md` (verdict, dispositions, NF-1 to NF-9), SDD-chess-store 3.9.1 notes 1 to 17 and the section 4 lists, `crates/chess-store/src/{lib,api,error,store,migrate,datadir,backup}.rs` public items, `docs/engineering/step2-store-B-evidence.md` (for what changed part A files later).

### 2.1 Conformance

| Area | Finding |
|---|---|
| SDD (DDL, open order, backup) | Reviewer: DDL equals SDD 3.5.2 after comment removal `[OBS review]`. Open path, rework 1 (D-108 replace-by-rename, D-109 any `sync_all` error) and rework 2 (first-launch order database, `store.id`, schema; removes nothing) are written as dated notes 1 to 10 of 3.9.1 and **accepted as built by Brian** in note 17 (D-124) `[OBS SDD]`. The code and the notes were written by agents of my role, so the match is by the Reviewer's reading, which I did not repeat `[UNV]` |
| Brian's rulings on the carve-outs (GATE-105 "Escalation check") | F-A02 D-108 (replace), F-A03 (part B), F-A04 D-116 (`k = 0`), F-A05 D-117 (`HasUnfinishedGames{ids: Vec<GameId>}`), F-A06 D-118, F-A07 D-119 (`create_fresh`), F-A08 D-120 (`RandomState` bytes) are all decided in SDD notes 11 to 16 `[OBS]`. `StoreError::HasUnfinishedGames { ids: Vec<GameId> }` exists `[OBS error.rs:135]`. **Not built yet, by plan:** `Store::create_fresh` (D-119; not found in `src/` by grep) and the D-120 uniqueness test; both are obligations of later parts, not defects of A |
| Reviewer Majors | FIND-001 (first-launch race deletes a live database), FIND-002 (poisoned `Missing`), FIND-003 (pragmas untested): FIXED, reproduced by the second Reviewer with 900 thread rounds and 1500 process rounds `[OBS review, not re-run]` |
| Reviewer minors still open | NF-4 (empty `chess.db` hides a lost store): accepted by Brian, note 17(e). NF-5 (first launch can block 43 s: 8 attempts of up to 5 s): the count is accepted (note 17(b)); the 43 s worst case is not named in the note, so I treat the latency as **not separately ruled**. NF-6 and NF-8 info. NF-7 test gaps: closed in part B evidence rows 14 to 17 `[OBS]`, not re-reviewed |
| Part A files changed after the rework 2 review | `backup.rs` (per-process temp name `chess.db.tmp-bak-v<N>-<pid>`, NF-1), `datadir.rs` (NF-3, stale scratch removal), `open.rs` (NF-2 hot-journal recovery, guard before backup per FIND-010, registration) were changed under **GATE-108** `[OBS step2-store-B-evidence.md lines 17 to 20]`. `GATE-105` does not say so and no Reviewer has re-read those hunks (the second Reviewer's last review is of rework 2) |

### 2.2 Public surface against the SDD

`lib.rs` exports `Migrator`, `Migration`, `MIGRATIONS`, `MigrateOutcome`, `DataDir`, `EnsureReport`, `StoreId`, `DIR_MODE`, `FILE_MODE`, `StoreOptions` (public fields `migrator`, `read_connections`, `clock`, `process`), `OpenedStore`, `WallClock`, and the cause enums `BackupCause`, `UnavailableCause`, `ExclusiveReason`, `WriteCause`, `Entity` `[OBS lib.rs:41-59, migrate.rs, store.rs:36-42]`. By grep of exact names, none of `StoreId`, `EnsureReport`, `MigrateOutcome`, `UnavailableCause`, `BackupCause`, `ExclusiveReason`, `WriteCause`, `Entity`, `WallClock` appears in SDD-chess-store `[OBS grep, exact names; the SDD may describe some in prose]`. `Migrator::upgrade` takes any connection, a backup-path closure and a guard closure (Reviewer FIND-011: a closure that returns the database path makes the backup rename a copy over the live file).

| Deviation | Needs G2? |
|---|---|
| Types and constants not named in the SDD (above) | **No, if the SDD lists them.** They are additive and no consumer exists. A dated G0 note listing the as-built surface fixes it (DR-04) |
| `Migrator::upgrade`/`with`, `StoreOptions.migrator`, `read_connections` public, not gated by `test-support` (FIND-011) | **Narrowing is G1** (inside the crate, reduces the surface). Keeping them as a documented contract for `chess-app` would be G2. They are test and migration seams; nothing in the SDD or in `chess-app` needs them at release. Narrow before `chess-app` code uses the crate (C-105-3) |
| `HasUnfinishedGames` variant, `preferences_json` field, derived `self_play` and `custom_start` | Already ruled by Brian (D-117, D-118): no further gate |

### 2.3 Conditions

| ID | Condition | Owner | Gate class |
|---|---|---|---|
| **C-105-1** | The part A hunks changed under GATE-108 (`backup.rs` temp name, `datadir.rs` NF-3 and stale scratch removal, `open.rs` NF-2, guard order, registration) must be inside the Reviewer pass of GATE-108, and GATE-105 gets a one-line pointer to that effect | PM (Reviewer scope), software engineer (pointer) | G0 |
| **C-105-2** | Two behaviours that live in part A files are not decided: **FIND-B-06** (the NF-2 exception to SDD 3.9.1 note 1, "no write before the checks": a write connection recovers a hot rollback journal of a cut-off first creation) and **CH-12** (temp name with `-<pid>`, D-83 precedent). Both are in `decision-package-store-B-format.md` for Brian. Until he rules, the code carries an unapproved exception to a baselined note | Brian Prascher | **G2** both |
| **C-105-3** | Narrow or gate the extra public surface (`Migrator::upgrade`/`with`, `StoreOptions.migrator`, `read_connections`, layout paths) before `chess-app` code consumes `chess-store`, or record each as a test seam in the SDD | software engineer; SA records | G1 |
| **C-105-4** | State the retry latency (8 attempts, 5 ms times attempt, `busy_timeout` per attempt; the Reviewer measured a 43 s worst case) in SDD note 3 or confirm Brian accepted it, since `StoreBusy` is user-visible (5.7) | SA (text), Brian to confirm the 43 s | G1 text; G2 if the count or timeout changes |
| **C-105-5** | Second reader for note 17(d) and (e) (missing `store.id`, empty `chess.db` next to traces): the relay lists them as accepted but the SDD records "no separate confirmation `[UNV]`" and the PM asked for explicit confirmation | Brian | G3 class (stored state) |

## 3. GATE-106: chess-engine

**What I read.** `gates/GATE-106-chess-engine.md` (original, rework, EF-03 CI diff and the applied record), `step2-engine-evidence.md` (sections 1 to 9 by grep, findings table, section 10 headings), `review-chess-engine-code.md` (verdict, gate check, EF-01 to 16) and `review-chess-engine-rework.md` (dispositions, NF-01 to 09, verdict), SDD-chess-engine 3.10.13 AN-EN-01 to 08 (read AN-EN-08 in full), `crates/chess-engine/src/lib.rs` exports and `Cargo.toml`, `ci/check.sh`, `.github/workflows/ci.yml`.

### 3.1 Conformance

| Area | Finding |
|---|---|
| Edges and dependencies | `chess-rules` only; tokio features `process, io-util, sync, time, rt, macros`; no `rt-multi-thread`, no `test-util`, no `unsafe` `[OBS Cargo.toml, review]`. Unchanged edges `[RAN]` |
| 5.7 `Engine`/`EnginePool` | Conforms (section 1). Additions beyond the cards (error variants `Busy`, `Closed`, `InvalidConfig`; `ErrorClass`; constants `MAX_MOVETIME` (1 h, Brian 2026-11-10), `MAX_MOVES`, `MAX_DURATION`) are inside the crate and consumed by nobody yet; the app SDD already maps engine errors by class |
| Reviewer Majors | EF-01 (timers starved by a line flood), EF-02 (`Duration` overflow panic), EF-04 (arbiter lost wakeup): FIXED and reproduced by a second Reviewer (floods, 7000 races, 0 lost). EF-03 (a skipped real-Stockfish test counted as a pass): **PARTIAL**, a process item (below) |
| `PINNED_UCI_ELO_RANGE` | `1320..=3190`, final by Brian (D-112, 2026-11-09), single definition in `src/pinned.rs` `[OBS SDD note, review]` |
| FIND-E4 (`Engine` not dyn-compatible) | Closed: app SDD note 2026-11-11 (D-121, option A, `AppEngine` enum in `chess-app`); `chess-engine` unchanged, so no G2 arises |
| FIND-E5 / ENG-5 | **Open for Brian (OQ-EN-004).** A `stop`-ignoring engine is killed at movetime plus 2 s, not within 200 ms. AN-EN-08 R-3 says `stop_ack_bound` is not implemented and that ENG-5's 200 ms holds for a cooperating engine; the real Stockfish measurement is two orders of magnitude inside the limit (`[OBS evidence section 6]`, engineer, not re-run). The behaviour is bounded and safe by the Reviewer's floods |
| SDD versus code | AN-EN-01 to 04 promised mechanisms the code lacks (Reviewer NF-01). AN-EN-08 (2026-11-10) now says as built: poll order closed, hard, soft, commands, output; no `max_lines_per_poll`; no `stop_ack_bound`; cap as constant; every write bounded by `handshake_timeout` except the shutdown `quit`. I read AN-EN-08 against the Reviewer's descriptions of `driver.rs`, not against the source line by line `[UNV]`; grep confirms `max_lines_per_poll` and `stop_ack_bound` do not exist in `src/` `[RAN]`. The note says "not independently reviewed" and I wrote it (independence note above) |
| Verification IDs | `CT-EN-080` to `086` and `UT-EN-063` to `065` are mapped by name in R-6; `CT-EN-083` (engine that ignores `stop`) and `CT-EN-086` (one contract suite over fake and real) are not built. NF-07 (parallel pool shutdown with `quit`-ignoring engines: mutants N4 and N8 survive) is open `[OBS review]`; I did not find a test for it `[UNV]`, `tests/pool.rs` has an `ignore_quit` scripted case at line 99 but I did not read it |

### 3.2 Public surface against the SDD

Exports: `AnalysisPriority`, `EngineConfig`, `EngineInfo`, `HandshakeReport`, `UciEngine`, `Engine`, `EngineError`, `ErrorClass`, `FakeEngine` family (feature `test-support` or `cfg(test)`), `BinaryLocator`, `BinarySource`, `LocatedBinary`, `EloRange`, `PINNED_UCI_ELO_RANGE`, `InstanceProfile`, `OptionPlan`, `PlanReport`, `ComputerKey`, `EngineHandle`, `EnginePool`, `GameKey`, `MoveChoice`, `MoveKind`, `SplitMix64`, `clock_budget`, `select_move`, `BoxFuture`, `ExitInfo`, `ProcessControl`, `ProcessSpawner`, `SpawnedProcess`, `Spawner`, the request and result types, and the whole `pub mod uci` `[OBS lib.rs:9-41]`. By exact-name grep of SDD-chess-engine, `ProcessControl`, `SpawnedProcess`, `ExitInfo`, `LocatedBinary`, `ComputerKey`, `HandshakeReport`, `InstanceProfile`, `AnalysisPriority`, `ErrorClass`, `MoveKind`, `BinarySource` and `InvalidConfig` do not appear `[OBS grep]`; `Spawner`, `BinaryLocator`, `EnginePool`, `EngineHandle`, `SplitMix64`, `clock_budget` do. Several of these may be described in prose under other names.

| Deviation | Needs G2? |
|---|---|
| Items not named in the SDD, all inside `chess-engine` and additive | No, if the SDD lists the as-built surface (DR-04). The surface only becomes a cross-crate contract when `chess-app` calls it |
| `pub mod uci` (parser and formatter) exported whole | **Question, not a finding:** only the app-facing items of 5.7 need to be public. A narrower export is G1; leaving it is harmless but widens what `chess-app` may depend on. Recommend narrowing to what the app SDD names (OQ-APP-014 already asks which constructor names the app needs) |
| New observable behaviour for `chess-app` (depth 0 with `Cp(0)` means "no evaluation", Reviewer NF-03; `SearchResult.pv/score/depth` describe `best`, EF-08; OQ-EN-006) | Contract text, G1 now. It becomes **G2** only if the app asks the engine to change shape. The app's analysis worker must read depth 0 as no evaluation: this is not yet in SDD 3.6.1 |

### 3.3 Conditions

| ID | Condition | Owner | Gate class |
|---|---|---|---|
| **C-106-1** | **EF-03 hosted:** `.github/workflows/ci.yml` now runs `third_party/stockfish/fetch.sh` `[OBS ci.yml:30]`, but `third_party/stockfish/` is untracked `[RAN git ls-files, git status]`, and GATE-106 says "`ci.yml` unchanged" in the applied-change record. A clean checkout fails. The script and its `.gitignore` must be committed with the CI change, the record corrected, and one hosted run made. Brian approved the CI change on 2026-11-11 (relayed); the hosted run and the commit are his and the PM's | PM, Brian (CI owner) | CI decision, Brian's |
| **C-106-2** | **FIND-E5 / OQ-EN-004:** Brian accepts that ENG-5's 200 ms holds only for a cooperating engine (and drops `stop_ack_bound`), or schedules it. Implementing it would add public `EngineConfig` fields | Brian | **G2** if scheduled; accept is a requirements-level risk acceptance |
| **C-106-3** | Add the pool test for parallel shutdown with several `quit`-ignoring engines (NF-07, N4/N8) and either build `CT-EN-083` or record it as deferred in the plan | software engineer | G1 |
| **C-106-4** | State in SDD 3.6.1 that depth 0 with `Cp(0)` means "no evaluation" and decide OQ-EN-006 (document the `best`/`pv` rule) | SA (text) with the app SDD owner | G1 |
| **C-106-5** | A second reader (Brian or a fresh Board stance) reads AN-EN-08 R-1, R-3 and R-5 against `driver.rs`, because the same role wrote the note and reviewed the code against it | PM to assign | G1 |

## 4. GATE-107: chess-store D1 (FaultPlan and crash harness)

**What I read.** `gates/GATE-107-chess-store-D1.md`, `step2-store-D1-evidence.md` in full, `crates/chess-store/src/lib.rs` (the `fault` module cfg lines 26 to 39), `ci/check.sh` lines 51 to 52, SDD-chess-store section 4 list (referenced by evidence), `store-completion-allocation.md` 4.1 by reference. There is **no Reviewer report for D1** in `docs/engineering/` `[OBS ls]`.

### 4.1 Conformance

| Area | Finding |
|---|---|
| Scope against the SDD | DD-ST-024 and IFP-ST-06 ("test-only"): the `fault` module exists only with feature `test-support`; with the feature off a private `fault` module holds `check` and two sync helpers, and the surface is unchanged. Evidence: compile-fail doctest, `cargo doc` without `fault` `[OBS evidence, not re-run]`; `lib.rs` shows the two cfg arms `[OBS]` |
| Edges, dependencies | None added; no `unsafe`, no `libc` (kill is `Child::kill`). Edges `[RAN]` unchanged |
| 5.7 | No contract touched; no production behaviour change with the feature off. With the feature on, production call sites carry failpoint checks: `[INF]` they cost nothing off (`#[inline(always)]`, constant `Ok(())`); the engineer did not inspect assembly `[UNV]` |
| Acceptance (1) to (5) | MET per the evidence, with a stated reading for (2): of the 16 SDD and Addendum K names only `backup.before_rename` has a call site in part A code; the rest are registered and reachable through `trigger`, and later parts wire their own calls (FIND-D1-05). I accept this reading: it is the only one the code state allows |
| The six Reviewer survivors (M16, M17, M22, M30, M31, M35) | Killed `[OBS evidence section 5]`. The one mutant that cannot be killed here (the real `sync_all` inside the helper replaced by `Ok(())`) is stated honestly; accepted as a known limit |
| Engineer findings | FIND-D1-01 (stale scratch files): the later code adds `remove_stale_scratch` (part B evidence line 19), so the gate record is out of date. FIND-D1-02 (failpoint names missing in SDD section 4): fixed by the dated note I entered today in SDD-chess-store (G0). FIND-D1-03 (CI does not run the feature-on tests): closed, `ci/check.sh` line 51 runs `cargo test -p chess-store --features test-support --locked` `[OBS]` (Brian approved the CI change 2026-11-11). FIND-D1-04, 05, 06, 07: info |
| Cross-cutting trap found by D5 (FIND-D5-06) | A failpoint context built from `Connection::path()` did not match a plan installed for the temporary directory; `DataDir::db_path()` did. Cause `[INF]` a symlinked `/var` versus `/private/var`. A silent no-op failpoint makes a fault test pass for the wrong reason. The D1 harness has counters (`hits`, `fired`), which lets a test assert the point fired, but the failure mode is in D1's code. See DR-05 |

### 4.2 Conditions

| ID | Condition | Owner | Gate class |
|---|---|---|---|
| **C-107-1** | **Reviewer pass of D1 not done.** The gate record says the desk review follows the Reviewer pass. A fresh Reviewer reads `fault.rs`, `tests/crash.rs`, `tests/fault_injection.rs` and the call sites. I cannot waive it; the PM or Brian decides whether test-only code needs it | PM (dispatch), Brian if waived | process |
| **C-107-2** | Make `fault::install` and `fault::trigger` compare canonicalized paths (or fail loudly when a plan never matches), with a test using a symlinked directory (FIND-D5-06, in the D5 package as "fix, small") | software engineer | G1 |
| **C-107-3** | Update GATE-107 with: the SDD note of 2026-11-13 covering the 10 + 6 names (this review), the CI stage that now runs the feature-on suite, and the part B change that removes stale scratch files, so the record matches the tree | software engineer | G0 |

## 5. Findings of this desk review

Severity: Major = must be resolved before the gate closes; Minor = resolve or accept with a note; Note = observation.

| ID | Sev | Gate | Finding | Action |
|---|---|---|---|---|
| DR-01 | **Major (process)** | 105 | Code under GATE-105 changed in later work (NF-1, NF-2, NF-3, guard order, registration) under GATE-108, with no Reviewer re-read of part A and no pointer in GATE-105 | C-105-1 |
| DR-02 | **Major** | 105 | An exception to SDD 3.9.1 note 1 (FIND-B-06: write connection recovers a hot journal before the checks) and the per-pid backup temp name (CH-12) are in the tree and not yet decided; both are G2 | C-105-2, Brian |
| DR-03 | Minor | 105 | Extra public surface (`Migrator::upgrade`/`with`, `StoreOptions.migrator`, `read_connections`) is not behind `test-support` and one closure can aim the backup rename at the live file | C-105-3 |
| DR-04 | Minor | 105, 106 | Neither SDD lists the as-built public types (store: `StoreId`, `EnsureReport`, `MigrateOutcome`, cause enums, `WallClock`; engine: `ProcessControl`, `SpawnedProcess`, `ExitInfo`, `LocatedBinary`, `ComputerKey`, `HandshakeReport`, `InstanceProfile`, `AnalysisPriority`, `ErrorClass`, `MoveKind`, `BinarySource`). Exact-name grep, so prose mentions may exist | Add an "as-built public surface" dated note to each SDD (G0), owner SA, at the next SDD pass. Not written today because the engine and store SDD owners are editing them `[UNV]` |
| DR-05 | Minor | 107 | `fault` path matching can silently never fire (FIND-D5-06) | C-107-2 |
| DR-06 | Minor | 107 | No Reviewer pass | C-107-1 |
| DR-07 | Minor | 105 | Retry latency worst case 43 s on first launch is not named in the SDD note and not separately ruled | C-105-4 |
| DR-08 | Minor | 106 | `ci.yml` calls an untracked script; GATE-106 says `ci.yml` unchanged | C-106-1 |
| DR-09 | Minor | 106 | FIND-E5: ENG-5 holds only for a cooperating engine; open decision | C-106-2, Brian |
| DR-10 | Minor | 106 | NF-07 test gap and unbuilt `CT-EN-083`, `CT-EN-086` | C-106-3 (`CT-EN-086` belongs to the app plan, `.3.12`) |
| DR-11 | Minor | 106 | EF-08 and depth 0 semantics are not in SDD 3.6.1 | C-106-4 |
| DR-12 | Minor | 106 | `pub mod uci` exports the parser wholesale | Narrow, G1 (suggestion in C-106-4 owners' next pass) |
| DR-13 | Note | 105, 106 | Independence: the same role wrote the SDD notes this review checks the code against | C-106-5, C-105-5 |
| DR-14 | Note | 105 | `Store::create_fresh` (D-119) and the D-120 uniqueness test are not built; they belong to parts C and D2, and `CreateFreshStore` must wait for the exclusive lock (store-A review F-A07 note) | track in the part C and D2 plans |
| DR-15 | Note | all | I did not re-run tests, mutants, coverage or `ci/check.sh`; the evidence is the engineers' and the Reviewers' runs | none |

No Blocker. Reviewer-level Majors open: none. DR-01 and DR-02 are my own Majors and are the reason the gates stay conditional.

## 6. Items that must go to Brian Prascher

| # | Item | Class | What he decides |
|---|---|---|---|
| 1 | FIND-B-06 exception to SDD 3.9.1 note 1 (hot-journal recovery in a cut-off first creation) | G2 | Approve the narrow exception, or require the old behaviour; see `decision-package-store-B-format.md` |
| 2 | CH-12: backup temp name `chess.db.tmp-bak-v<N>-<pid>` (D-83 precedent) | G2 | Approve the name (fixes NF-1: concurrent upgraders destroyed each other's backup) |
| 3 | FIND-E5 / OQ-EN-004: ENG-5 holds only for a cooperating engine | G2 if `stop_ack_bound` is scheduled; risk acceptance otherwise | Accept and drop `stop_ack_bound`, or schedule it |
| 4 | Retry latency of a first launch (up to 43 s) and note 17(d), (e) confirmation | G2 (visible `StoreBusy` behaviour) and G3 class (stored state) | Confirm as built or change the count and timeout |
| 5 | CI: commit `third_party/stockfish/` with the `ci.yml` step and run one hosted job (EF-03); correct GATE-106's "ci.yml unchanged" | CI decision (Brian's) | Decide and commit |
| 6 | From the D5 package (not these gates): FIND-D5-02 variation identity | G2 or G3 | See `decision-package-store-D5.md` |

None of these is approved by this review.

## 7. Not reviewed, not run, assumed

- Not reviewed: GATE-108, GATE-109, GATE-110 and their evidence; part B code that changed part A files (only what its evidence says changed).
- Not run: all tests, clippy, fmt, mutants, coverage, `ci/check.sh`, the hosted workflow. The only command I ran is `node ci/check-crate-edges.mjs`.
- Not read line by line: the source of `driver.rs`, `open.rs`, `migrate.rs`, `backup.rs` (beyond the passages cited); evidence sections 3 to 6 of the store A file; the engine tests.
- Assumed: the Reviewers' reproductions are accurate; the SDD notes cited are the current text; the working tree is the state the engineers evidenced (it is not: the tree also contains parts B, D2 and D5, so counts in the evidence files may differ from a run now).
- Mermaid: none in this document.
