# G1 desk review: chess-store part D3 (GATE-111), baseline B6

| Field | Value |
|---|---|
| Document | docs/engineering/desk-review-G1-step2-D3.md |
| Reviewer | System architect agent, desk review (conformance to the SDD, the architecture, crate edges and the section 5.7 contracts) |
| Date | 2026-11-23 |
| What this is | **Input for Brian Prascher. It is not an approval.** GATE-111 is G3 (stored values) and G2 (new public items). Only Brian approves it. A desk review satisfies G1 only. Brian declined delegation |
| Stance | Review Board: findings only, no redesign. Recommendations are in [decision-package-store-D3.md](decision-package-store-D3.md) (Architect stance, separate document) |
| Evidence tags | `[RAN]` executed by me, `[OBS]` read by me, `[INF]` inferred, `[UNV]` not verified. "Engineer" means the evidence file [step2-store-D3-evidence.md](step2-store-D3-evidence.md); I re-ran only the crate-edge check |
| Independence | No independent Reviewer pass was done (waived for v1, FIND-016). Every test count below is the Engineer's own `[UNV by me]` |

## 1. Verdict

**PASS WITH CONDITIONS (G1).** The code conforms to the SDD, the crate graph and the cited decisions on every point I could read. I found no Critical finding and no defect that corrupts a stored value on the happy path. Three conditions are Major and need Brian before GATE-111 is approved: the end-time rule of the journal format (DR-01), the gap after a first-attempt permanent error (DR-02), and the approval order with GATE-109 (DR-03). The rest are Minor text or record keeping.

## 2. What I checked and found conforming

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Crate edges (architecture 3.2, D-38) | **Conforms.** `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` | `[RAN]` `node ci/check-crate-edges.mjs` |
| 2 | Public surface | Exactly the items GATE-111 lists: `Store::{apply_records, sync_backlog, backlog_status}` (`store/d3.rs`), `ApplyOutcome` and `BacklogStatus` (`api.rs`, exported in `lib.rs`); `MonotonicClock` and `with_monotonic_clock` only under `feature = "test-support"` | `[OBS]` lib.rs 44-64, store.rs 62-70, d3.rs |
| 3 | D-40 (UUID outside the store) | Conforms: `apply_records` takes a `GameId`; no integer key crosses the API, `BacklogStatus` and `ApplyOutcome` carry none | `[OBS]` |
| 4 | D-39 (concrete `Store`) | Conforms: no trait, no `dyn Store` | `[OBS]` |
| 5 | D-92 ply base | `MOVE.ply = journal ply + 1` with `checked_add` in one place (`write_move`); the plan applier takes already one-based plies | `[OBS]` apply.rs 270-280, 429-436 |
| 6 | D-87 pair | A `GameEnd` with a `Voided` after it in the same batch is skipped (`superseded`); a `Voided` on a settled completed row is refused by `void_game`'s precondition; `Abort` then `Voided` is refused (`apply_voided` returns `InvariantViolated` for other statuses), which is RR-A-2 | `[OBS]` apply.rs 98-112, 402-427; tests named in the evidence `[UNV by me]` |
| 7 | Atomicity | One `BEGIN IMMEDIATE` per batch; `apply.before_commit` and `apply.after_commit` failpoints; 100 ms lock wait with restore of the 5 s wait (SDD 3.12 "Apply attempts") | `[OBS]` apply.rs 88-123 |
| 8 | Journal first (SEQ-ST-02, SDD 3.12) | Conforms: the database follows the journal, a busy database queues, play does not wait | `[OBS]` thread.rs 326-348 |
| 9 | Single writer (D-13, DD-ST-002) | The backlog and its timer live on the store thread; no second execution context | `[OBS]` thread.rs |
| 10 | Layering | `tests/boundary.rs` (no SQL words in `thread.rs`, `store.rs`) | Engineer `[UNV by me]` |
| 11 | D-66 | `journal_seq` is never lowered by a batch, so `db_seq > last_intact_seq` keeps its meaning; only the plan applier sets it exactly | `[OBS]` apply.rs 114-121, 464-470 |
| 12 | D2 code edit | `finalize.rs` change is a move of two bodies into `finalize_in_tx` and `abort_in_tx`; wrappers keep transaction, failpoint and commit | `[OBS]` finalize.rs (not diffed line by line by me) |
| 13 | Local gates | Engineer: fmt, clippy `-D warnings`, 5 runs of 413 and 485 tests green, `ci/check.sh` offline exit 0 | `[UNV by me]` |

## 3. Findings

Numbering is mine (DR-nn). Engineer findings keep their FIND-D3 IDs. Severity: Major = resolve or accept by Brian before approval; Minor = resolve or accept with a note; Info = noted.

| ID | Severity | Lens | Finding |
|---|---|---|---|
| DR-01 | **Major** (needs Brian, G3) | Data, Evolution | **The journal format and the store disagree on a valid end time (FIND-D3-01 = F2-04).** `GameEnd`, `Abort` and `Voided` `ended_at` are plain `i64` `[OBS format.rs 340-364]`; `finalize_in_tx` and `abort_in_tx` refuse `<= 0` `[OBS finalize.rs 144, 245]`; `void_game` and `void_unreadable_game` accept any value. A journal that holds such a record decodes and then can never be applied, restored or finalized by the scan, and the refusal repeats at every start. This is the CH-4 defect class and the range rule is one-way at release (Addendum M (3)). The built refusal (`InvariantViolated`, whole batch rolled back, no panic) is correct as a second line; the open question is the first line. Required action: Brian rules the format rule before GATE-111 is approved. Interplay: with a decode-only refusal a CRC-valid refused tail becomes `UndecodableTail` damage (D-129) for a record this build itself wrote; the writer side must refuse too |
| DR-02 | **Major** (code and text, G1; observable part G2) | Quality attributes (NFR-2, DAT-4), Data | **A first-attempt permanent error leaves a gap; a retry parks.** On the first attempt `apply()` returns the error and queues nothing `[OBS thread.rs 339-347]`; the next batch of that game applies on top. On a retry the game parks `[OBS backlog.rs 17, 45, 144]`. After the first path the journal holds a record that the database lacks, and a later `GameEnd` finalizes a row with a missing move or clock, silently `[INF]`. Evidence needed: a test that applies a refused batch, then a valid batch and a `GameEnd`, and compares the rows with the journal fold. Required action: one rule for both paths (decision package item (b)); the SDD 3.12 row for an uncurable batch (FIND-D3-02) |
| DR-03 | **Major** (approval order, Brian) | Cost and organization | **GATE-111 edits D2 code that GATE-109 has not approved** (FIND-D3-15, `finalize.rs`), and GATE-108 and GATE-110 are also not approved. Approving D3 first would approve a change to unapproved code, and a ruling that changes D2 (decision package (e), delete `void_game_repair_pair`) then lands on two gates. Required action: Brian orders the approvals (109 before 111, or one combined step) |
| DR-04 | Minor | Evolution | **`BacklogStatus` has five public fields and no `#[non_exhaustive]`** (`ApplyOutcome` has it) `[OBS api.rs 328-354]`. A new field breaks a consumer that builds or destructures it. Required action: add the attribute before `chess-app` uses it (package (d)) |
| DR-05 | Minor | Data | **FIND-D3-07 SDD conflict.** SDD 3.7.2 and 3.11.4 say "set `journal_seq` to the record's `seq`"; the code uses the larger of the stored and the last `seq`. The code is right under D-66 `[INF]`; the text is stale. Required action: dated note (package (c)) |
| DR-06 | Minor (G3 format gap) | Interface | **`Start.matchup` has no text form** (FIND-D3-06). The SDD lists the field without its text, the store ignores it and derives the column `[OBS]`, and the test builder writes `human-human` while the table CHECK says `hh`, `hc`, `cc`. A value that nobody reads is a value nobody notices when wrong, and a journal field text is one-way at release. Required action: Brian rules the text and whether it is checked at decode (package (f)) |
| DR-07 | Minor | Interface | **`backlog_status`, `ApplyOutcome` and `BacklogStatus` are not in SDD 3.7.2** (FIND-D3-11; `apply_records` and `sync_backlog` are, lines 483 and 497). The `apply_records` row does not say what `Applied`, `Queued` or an `Err` mean. Required action: SDD rows (package (d)); the app SDD names how it polls `store_backlog` |
| DR-08 | Minor | Data | **Takeback deletes `move_eval` rows** (FIND-D3-04) and the SDD does not say so `[OBS apply.rs 329-335]`. The app analysis service must expect recomputation. Required action: sentence in the `Takeback` row of 3.11.1 |
| DR-09 | Minor | Data, Operability | **A live finalize, abort, void or reset does not flush the game's apply queue** (FIND-D3-09). A queued `Voided` behind a live-finalized game is refused and parks the game (DR-02). The app SDD already calls `sync_backlog` before `prepare_void` (CT-APP-32 (a)), which covers one path only. Required action: the store flushes the one game, or the app SDD states the rule for all four commands |
| DR-10 | Minor | Evolution | **`apply_records` does not check the game's status for `Move`, `Takeback` and `Clock`** `[OBS apply.rs 100-111]`. A stale or replayed batch applied after a finalize can rewrite moves and clocks of a completed or voided row (FIND-D3-08 describes the clock part; "the journal wins" is the intent). Not a defect while only the journal feeds it; D4 and resume must not pass a journal that is older than the row. Required action: state it in SDD 3.11.1 and as a D4 acceptance line |
| DR-11 | Minor | Operability | **The backlog and the park are in memory** (FIND-D3-13). A restart clears both, the scan repairs from the journal, and a game with a permanent error meets it again. The user-visible handling of `parked_games > 0` is not designed (only `StoreBacklog`, an age). Required action: the app SDD maps `parked_games` to a log or a condition (G2 for the app, not decided here) |
| DR-12 | Minor | Evolution | **FIND-D3-10: the D-87 pair is detected inside one call only.** A `GameEnd` in one batch and its `Voided` in a later batch would settle a rating and then refuse the void. Required action: D4 (.3.40) and app resume pass the whole journal, or the `GameEnd` with its `Voided`, in one batch; add it to their acceptance lines |
| DR-13 | Minor | Quality attributes | **FIND-D3-14: the plan applier uses the 5 s lock wait**, not 100 ms. D4 decides whether an open or resume path may wait that long (NFR-1, CLI at most 1 s start-up). Required action: a stated decision in D4 |
| DR-14 | Info | | Independent Reviewer pass not done (waived for v1 by Brian, FIND-016); mutation figures are hand mutants (61, 60 killed, 1 equivalent), `cargo-mutants` not installed `[UNV]`. One load-related baseline failure in the mutation copy was not identified (FIND-D3-16) |
| DR-15 | Info | | FIND-D3-05 (no slot rows until the last restore pass), FIND-D3-12 (poll, not push) and FIND-D3-13 need no ruling |

### Open findings carried from the reviews and relevant to D3

| ID | Status in D3 | Note |
|---|---|---|
| F2-04 | Same as DR-01 | |
| F2-09 | Answered by D3 as built | A `Voided` on a settled completed row is refused. Needs the text of the answer in SDD 3.11.4 |
| F2-01, F2-02, F2-06 | Not D3 | Recommendations in the decision package (g); D5 and part B |

## 4. Conditions

| # | Condition | Owner |
|---|---|---|
| C1 | Brian rules DR-01 (end-time rule) | Brian, G3 |
| C2 | One rule for permanent errors on both paths, with a test (DR-02), and the SDD 3.12 row | Engineer after a ruling; text by the system architect |
| C3 | Brian orders GATE-109 and GATE-111 (DR-03) | Brian |
| C4 | Text updates: DR-05, DR-07, DR-08, DR-10, DR-11 (G0 dated notes; the SDD note and the architecture note added by this review say only that these items are open) | System architect |
| C5 | `#[non_exhaustive]` on `BacklogStatus` (DR-04) | Engineer |
| C6 | Brian rules the `Start.matchup` text (DR-06) | Brian, G3 |
| C7 | Acceptance lines for D4 (.3.40) and the app (.3.12): DR-09, DR-10, DR-12, DR-13 | Program manager and the owners |

Conditions C1, C3 and C6 are Brian's. C2 and C5 are code and need his ruling on the rule first (C2). C4 is document work that does not need approval beyond G0.

## 5. For Brian

1. **G3:** rule the end-time rule of the journal format (DR-01), the `Start.matchup` text form (DR-06), and the `journal_seq` value rule (DR-05) as part of GATE-111.
2. **G2:** the new public surface (DR-07, DR-04) and the residual `$TMPDIR` risk of F2-06 (not a D3 item, listed in the decision package).
3. **Order:** whether GATE-109 is approved before GATE-111 (DR-03).
4. The independent Reviewer pass is waived; the evidence is the Engineer's. Whether that remains acceptable for a G3 gate with a stored-value rule is yours.

## 6. What I did not analyze

Power-loss behavior and Intel Macs. The test suites (not re-run; only `check-crate-edges.mjs`). The SQL of `finalize_in_tx` and `abort_in_tx` line by line. Rating arithmetic. The plan applier beyond its documented contract (D4 is its first caller). `void_unreadable_game` and `reset_player` with a non-empty backlog, beyond DR-09. `cargo deny` and `npm audit`. I did not check that the two dated notes below match every other document that names the same items.

## 7. Notes added by this review

Dated notes only, no row rewritten: docs/design/SDD-chess-store.md (before 3.13) and docs/architecture.md (after Addendum M in the header list). Neither claims an approval, and neither records a decision; they record that rulings are open.
