# Decision package: chess-store part D5, two Major findings and ten minor ones

Architect stance, 2026-11-13, baseline B6, bead chess-a0r.3.41 follow-up. For **Brian Prascher** where a gate class below says G2 or G3. **Nothing here is decided, reviewed or approved.** Recommendations only. Sources: [step2-store-D5-evidence.md](step2-store-D5-evidence.md) section 6, [GATE-110](gates/GATE-110-chess-store-D5.md), `crates/chess-store/src/{api_analysis.rs,store/d5.rs,repo/analysis.rs}` and `crates/chess-domain/src/stats.rs` (read, not re-run), SDD-chess-store 3.5.2 (line 103), 3.7.2, 3.9.1 and the R-ST-12 row, SDD-chess-domain 3.11 (StatsRow, line 810), SDD-chess-app (DD-APP-12, DD-APP-14, UF-APP-06), architecture D-38, D-40, D-44, ASM-6, `store-completion-allocation.md` F-ALLOC-3. Tags: `[OBS]` read, `[INF]` inferred, `[UNV]` unverified.

**Door type, once.** Nothing is released and `schema_version` is 1 with one migration (`migrations/0001_init.sql`) `[OBS]`. FIND-D5-01 touches no stored format and no wire type, so it is **two-way**. FIND-D5-02 is two-way if it stays in the API and **one-way at the first release** if it adds a column (a `uuid` in `variation` reaches exports and the protocol; D-40 and ASM-6 say the same about every stored-format choice).

## Summary

| Item | Recommendation | Gate class | Confidence |
|---|---|---|---|
| FIND-D5-01 `StatsRow` colour type | **Option A: accept the store-owned `StatsGameRow` and `StatsSide`; `chess-app` maps to `StatsRow`.** | **G1** (text already directs it) | High |
| FIND-D5-02 identity for `delete_variation` | **Option B: natural key `(game, from_ply, pgn)` with `save_variation` idempotent on that key; drop `VariationRef`.** No schema change. | **G2** (interface two crates share; the SDD names none). Option C would be **G3** plus G2 | Medium |
| FIND-D5-03 to 12 | See the table at the end: 5 accept, 2 accept with a condition, 3 fix (one text-only, two small code or test) | G0 or G1 | Medium to High |

## FIND-D5-01: the statistics row type

**Problem.** `chess_domain::StatsRow.color` is a `chess_rules::Side` `[OBS stats.rs:36]`. `chess-store` has an edge to `chess-domain` only (`ci/check-crate-edges.mjs`: "8 crates, 11 workspace edges, all allowed" `[RAN]` today). So the store cannot build a `StatsRow` without a new edge or a changed domain type.

**What the baselined text already says.** SDD-chess-store (crate-edges paragraph, line 210): "Because `Side` is a `chess-rules` type, the store keeps its own stored-side enum `StoredSide` in its journal **and row DTOs** (D-38) ... `chess-app` converts at the boundary." R-ST-12 rejects "re-export `Side` through `chess-domain`" with the cost "one small conversion in `chess-app`". D-38: `chess-domain` and `chess-store` re-export nothing from `chess-rules`. D-44 and SDD-chess-domain line 810 say the store returns "rows" and name no Rust type. So the engineer's choice is the reading the SDD already points to `[OBS]`, not a new design.

| Option | What it is | Trade-offs | Door | Gate |
|---|---|---|---|---|
| **A (as built, recommended)** | Store returns `StatsGameRow` (the 11 `StatsRow` fields, `color: StatsSide`). `chess-app` maps each row to `StatsRow`. | Matches R-ST-12, D-38 and the existing `StoredSide` precedent. No edge change, no baselined text changed. Cost: a second struct with the same fields and one conversion per row. Drift risk: a field added to `StatsRow` but not to `StatsGameRow`. | Two-way: code in two unreleased crates, nothing stored or sent | G1 (inside one crate plus one conversion in `chess-app`; conforms to SDD line 210) |
| B | Add edge `chess-store -> chess-rules`, return `StatsRow`. | Removes the copy. Reverses D-38/R-ST-12 (ED-8, rejected before), edits `ci/check-crate-edges.mjs` and the `deny.toml` wrappers list, and lets the store name rules types in its API. The "pure store" independence of stored format from rules enums is lost. | Two-way, but edges are expensive to remove once code uses them | G2 |
| C | Give `StatsRow` a domain-owned colour enum. | One conversion moves to `chess-app` anyway (it holds the edge to rules); the store would map to the domain enum. Changes `chess-domain` public API and SDD (baselined v1.0, code exists, tests `UT-DOM-*` and `aggregate` use `Side`). More churn than A for the same result. | Two-way | G2 |

**Recommendation: A.** Confidence **High**: the SDD text already prescribes it, no option removes the conversion (C only moves it), and B reverses a decision Brian made.

**Consequences for chess-app.**
- `.3.12` (app part A): none. It does not need D5 (store-completion-allocation 5.6) unless `StatsService` stays in it.
- `.3.13` (app part B, depends on D5): `StatsService` (DD-APP-14, ALG-APP-13) maps `StatsGameRow -> StatsRow` (`StatsSide::White -> Side::White`, and so on), then calls `chess_domain::aggregate`. Write the mapper as an exhaustive struct construction and destructuring with no `..`, so a new field on either struct breaks the build (the drift guard). ALG-APP-13 says the service "adds no filter and no arithmetic"; a field-for-field conversion is neither, but the sentence should say so.
- FF-5: the store half is recorded (BM-ST-03, 10 000 rows in 10.9 to 18.6 ms release, 46.6 to 52.9 ms debug, noisy `[RAN, engineer]`). The app half (rows plus map plus `aggregate`) is **not measured** `[UNV]`; the map is one pass over 10 000 small structs and I expect it to be small next to the query, but that is an `ESTIMATE` until `.3.13` measures it.

**SDD text needed (all G0/G1 dated notes, none rewrites a baselined row).**
1. SDD-chess-store 3.7.2 (statistics rows) and R-ST-12: the queries return `StatsGameRow` with `color: StatsSide`; all other fields are those of `chess_domain::StatsRow`. Entered as dated note 2026-11-13, notes 18 to 20, in SDD-chess-store (after note 17 of 3.9.1).
2. SDD-chess-app ALG-APP-13 / DD-APP-14: the field-for-field mapping and the exhaustive-construction rule. Owner: app SDD owner, at `.3.13` start (G1 text).
3. SDD-chess-domain: no change.

## FIND-D5-02: identity of a saved variation

**Problem.** `delete_variation` must name one row. Schema 1 gives `variation` an integer key only (`variation (id INTEGER PRIMARY KEY, game_id, from_ply, pgn, created_at)` `[OBS 0001_init.sql, SDD line 103]`). D-40: integer row keys never cross the store API, the journal, the protocol or an export. The SDD names no argument for `delete_variation` (SDD line 491: `delete_variation` with no parameters). The engineer built `VariationRef(pub(crate) u64)`, with no constructor and no accessor `[OBS api_analysis.rs:148]`.

**Why the as-built handle is a weak answer `[INF]`, for the app.** `DeleteVariation` is a protocol command carried over three transports (in-process, Tauri IPC, WebSocket). A handle that cannot be read or built outside the store cannot be put in a payload, so `chess-app` would have to keep a table from some wire token to `VariationRef` for the life of the process. That is new per-process state (the app SDD rationale at line 1138 rejects server-held what-if state for exactly this reason), it dies on restart, and it is wrong across two front-ends sharing one data directory (each process has its own table). Also the wrapped value is the integer row key, and D-40's own reason for keeping integer ids internal is that "a restore can reuse integer ids": a UI that lists, then a restore runs, then the user deletes, would hit a different row. `[INF]`: I did not test this; it follows from D-40's stated rationale and from the type having no encoding.

| Option | What it is | Trade-offs | Door | Gate |
|---|---|---|---|---|
| A (as built) | Opaque `VariationRef`, valid for the returned store. | No DDL, no wire type. Cannot be sent in a protocol payload (above); forces process-local state in `chess-app`. Passes D-40 only in letter. | Two-way | G2 (an interface the SDD does not give; the app depends on it) |
| **B (recommended)** | Identity is the natural key `(game, from_ply, pgn)`. `delete_variation(game, from_ply, pgn) -> bool`. `save_variation` becomes **idempotent on that key**: if the row exists it returns it (same `created_at`) and inserts nothing, inside the one write transaction, so identical lines cannot exist. Drop `VariationRef` and the `id` field of `VariationRecord`. | No schema change, no stored-format change, no domain type. Fits the wire as is: the client already holds the line, and `chess-app` already renders `pgn` canonically from the moves it replayed (app SDD line 1133), so the same line gives the same text. Survives restart, restore and several processes. Costs: a long string as key (a line is short), the save-twice case returns one row instead of two (a small behaviour change to "one row = one saved line", which stays true), and a rename or edit feature, if one is ever wanted, needs a real id. | Two-way. A `uuid` column can be added later by migration 0002 with this API kept as an alias | G2 |
| C | Add `uuid TEXT NOT NULL UNIQUE` to `variation`; `delete_variation(VariationId)`; the id is carried in the protocol and the export. | Most uniform with `GameId` and `PlayerId` (D-40). Needs a migration (edit `0001` in place, since nothing is released and ASM-6 says it is cheap now, or add `0002` if any developer data directory exists), a `VariationRow` export field, a `VariationId` newtype (in `chess-domain` that is a second G2 on a baselined crate; or store-owned and re-exported by `chess-app`), and a protocol field. It also arrives while OQ-ST-003 (how imported nested variations are stored) is still open and may reshape this table; a column added now may be touched again. | **One-way at release** | **G3** (stored format) plus G2 |
| D | Delete by `(game, ordinal in list order)`. | Positions shift when another line is deleted or another process saves. Wrong-row deletes. | Two-way | Rejected |

**Recommendation: B.** Confidence **Medium**. What would flip it to C: Brian wants a stable id for something B cannot give (a UI that edits a saved line in place, comments or tags on a variation, or an export that must keep variation identity for a merge). If C is chosen, bundle it with the OQ-ST-003 decision so the table changes once. A is not recommended: it is the only option that adds state to `chess-app`.

**Consequences for chess-app.**
- `.3.12`: none (analysis is `.3.13`).
- `.3.13`: `AnalysisService` (DD-APP-12) defines `DeleteVariation { game_id, from_ply, line }` (UF-APP-06 says the payload is unspecified; this fills it). The `SaveVariation` path replays the moves, renders the canonical `pgn`, calls `save_variation`, and `DeleteVariation` renders the same text from the same moves and calls `delete_variation`; unknown row gives `false`, which the app maps to `NotFound{entity: variation}` (app SDD line 1258) or to success (delete is idempotent: the engineer's test `delete_variation_removes_one_row_and_is_idempotent`). With C instead, the payload carries `variation_id: Uuid` and `list_variations` returns it.
- Rework for B (store engineer, about one task of small size `ESTIMATE`): change one query and one signature, remove `VariationRef`, add the idempotent-save branch inside `save_variation`'s transaction (and its failpoint order), update `analysis_variations.rs` and `d5_faults.rs`, re-run the mutants of `analysis.rs`.

**SDD text needed (B).**
- SDD-chess-store 3.7.2, row of `save_variation`/`delete_variation`/`list_variations`: signatures above; "a line is identified by `(game, from_ply, pgn)`; saving an existing line returns it and writes nothing; `VARIATION.id` never leaves the crate (D-40)". G1 text, but the gate for the change is G2 (interface). A dated note is **not** written for this by me because the choice is not made; it would be written after Brian decides. Under C the DDL block 3.5.2, DD-ST-011, the export row `VariationRow` and `UT-ST-037` also change (G3).
- SDD-chess-app: payload of `DeleteVariation` (UF-APP-06).

## FIND-D5-03 to FIND-D5-12 (minor and info)

Owners: SE = software engineer (store), APP = software engineer on `chess-app` `.3.13`, SA = system architect, PM = program manager. "Accept" means no code change now.

| ID | Sev | Recommendation | Owner | Reason and condition |
|---|---|---|---|---|
| FIND-D5-03 `games_for_history` returns whole `GameRecord` | Minor | **Accept** | APP to confirm at `.3.13` start | It is the same query as `list_games` with a player, window and limit; history lists include voided and unfinished games (UI-2). Opponent names need a second lookup. Revisit trigger: the history screen needs the opponent name per row and the extra call shows in a measurement; then add a joined row type (additive, G1). |
| FIND-D5-04 `limit` keeps the oldest N | Minor | **Accept with condition** | APP decides before first use; SE changes only on demand | For statistics rows `limit` is a safety cap, not a view, so order is irrelevant. For history a "recent games" list wants the newest. Same behaviour as `list_games` (the store-A review noted it). Condition: APP states which it needs when `.3.13` starts; if newest-N, SE adds an explicit order argument (G1, additive) rather than flipping the default. |
| FIND-D5-05 three failpoints not in SDD section 4 | Minor | **Fix (text)** | SA | G0 dated note in SDD-chess-store, written with this package (also covers the D1 names, FIND-D1-02). No code change. |
| FIND-D5-06 failpoint context from `Connection::path()` did not match | Minor | **Fix (small code)** | SE (D1 code, `fault.rs`), D2 engineer meanwhile | Make `install` and `trigger` compare canonicalized paths so `/var` versus `/private/var` cannot cause a silent no-op; add a test with a symlinked directory. Until then the rule is: contexts are built from `DataDir::db_path()`. A silent no-op failpoint makes a fault test pass for the wrong reason, which is the worse failure than a loud one; the cause is `[INF]`, so the fix includes a test that proves it. Rule recorded in the SDD note. |
| FIND-D5-07 `save_variation` counts stored `MOVE` rows, not the journal backlog | Minor | **Accept with condition** | SE and APP when D3 (apply) lands | The app guards first (decision 13) from the live session's ply count; the store check is the second guard. A wrong answer needs unapplied backlog on the same game. Condition: when `apply_records` and `sync_backlog` exist, either `save_variation` runs on a game whose backlog is applied, or the guard counts journal plies; add one test at that point. Revisit trigger: D3 accepted. |
| FIND-D5-08 `NotOwner` for absent, not-running or already-done job; second `complete_job` is `NotOwner` | Minor | **Accept** | SA (text), APP (worker behaviour) | SDD says only "a worker that lost ownership gets `NotOwner`". The worker must read `NotOwner` on its own `complete`/`fail` as "the job is no longer mine, stop touching it", not as an error to surface. Same family as `register_player` (store SDD note 8(b)). Text in the SDD note. |
| FIND-D5-09 "once per start" is once per `Store` open | Minor | **Accept** | SA (text) | Two processes each requeue once; `requeue_failed` moves only jobs that are `failed` at that moment, so a second requeue is harmless. A second open in one process counts as a start. Text in the SDD note. |
| FIND-D5-10 `enqueue_job` does not check the game is finished; empty PGN and empty `engine_version` accepted | Info | **Accept** | APP | The worker owns the finished-game rule (DAT-3), the sweep enforces it, and `chess-app` validates input before the store. No code change. |
| FIND-D5-11 `get_game_analysis` snapshot not tested against a concurrent writer | Info | **Fix (small test)** | SE | One test: a writer upserts evals while `get_game_analysis` runs; `evaluated` never exceeds `total` and `evals.len() == evaluated`. Cheap and it pins the one-snapshot claim `[INF]` that the API doc makes. Not a blocker for the Reviewer pass. |
| FIND-D5-12 `cargo fmt -p chess-store` may have reformatted D2's in-progress files | Info | **Accept** | PM | `cargo fmt --check` was clean afterwards and nothing was lost. Process note for PM: when two tasks edit one crate, run `rustfmt` on own files only, and run `cargo fmt --check` at the end. |

## What this package does not cover

The Reviewer pass of D5 (not done), the SQL query plans, a user with more than 10 000 games, the `chess-app` half of FF-5, Intel Macs. The desk review of GATE-110 itself is not in this package and is not done.

## Needs a human

Brian Prascher: FIND-D5-02 (G2 for B, G3 for C). FIND-D5-01 under A needs only a G1 desk review, unless Brian reads SDD-chess-domain line 810 as requiring the store to return `StatsRow` itself, in which case it is G2 (options B or C).
