# G1 desk review: chess-store part B (GATE-108), D2 (GATE-109), D5 (GATE-110), baseline B6

| Field | Value |
|---|---|
| Document | docs/engineering/desk-review-G1-step2-B.md |
| Reviewer | System architect agent, desk review (conformance to the SDD, the architecture, crate edges and the §5.7 contracts) |
| Date | 2026-11-22 |
| Decision bead | chess-a0r.3.50 |
| What this is | **Input for Brian Prascher. It is not an approval.** GATE-108 and GATE-109 are G3 (stored format) and GATE-110 is G2 as reworked (public interface). Only Brian approves them. A desk review satisfies G1 only |
| Stance | Review Board: findings only, no redesign. Recommendations on the open rulings are in [decision-package-store-rework-findings.md](decision-package-store-rework-findings.md) (Architect stance, separate document) |
| Evidence tags | `[RAN]` executed by me, `[OBS]` read by me, `[INF]` inferred, `[UNV]` not verified. "Reviewer" means the independent Reviewer reports, not me |
| Snapshot | The live working tree on 2026-11-22. The two independent reviews hashed `crates/chess-store` at 22:41Z and 22:43Z. I did not recompute their hashes, so the crate may have changed since (an engineer is editing it now) |

## 1. Verdicts

| Gate | Tier (my reading) | Verdict | In one line |
|---|---|---|---|
| **GATE-108** part B | G3 (journal format, stored states) and G2 (open path, public items) | **FAIL at this snapshot.** Becomes PASS WITH CONDITIONS when conditions B1 to B4 close and a fresh Reviewer re-checks the open path | Format and structure conform. Two open Majors in the NF-4 open path, one of which is a conflict inside the spec that needs a ruling, not only a fix |
| **GATE-109** D2 | G3 (stored values and shapes) and G2 (public items) | **PASS WITH CONDITIONS.** Condition D1 is a Major and blocks approval | Write path conforms to SDD 3.11.4 and the Elo is right in 3,675 independent vectors. `check_rating_invariant` reports violations that do not exist while a finalize commits |
| **GATE-110** D5 | **G2** (the variation API changed in the rework; the gate record still says G1) | **PASS WITH CONDITIONS** | No semantic defect found. Conditions are record keeping, one scanner gap and decisions that are Brian's |

Why B is FAIL and D2 is not, although both have an open Major: the B Major (FIND-RW1-01) is a contradiction between rows of the spec that the code follows faithfully, so the fix needs a ruling first; D2's Major is a local code defect with a known, small closure (one read transaction) and a ready probe. The Reviewer reached the same split (FAIL for B part 1, PASS WITH CONDITIONS for D2 with a blocking Major).

## 2. Open Majors (as of this snapshot)

| ID | Gate | Summary | State |
|---|---|---|---|
| FIND-RW1-01 | 108 | A first launch killed during creation (schemaless `chess.db` plus a non-empty `-wal`; zero-length `chess.db` plus a non-empty `-journal`) is refused as `Missing`. 9 to 12 of 120 random kills per run | Spec conflict (rows 5, 7, 9 against NF2-1 and FIND-002). Recommendation in the decision package, item (a). Needs Brian (G2). An engineer is fixing now |
| FIND-RW1-02 | 108 | A live concurrent creator can be refused; the author's FIND-001 acceptance test failed once in ten suite runs (`round 190: Missing`) | Same cause as RW1-01 for the journal case `[INF]`. An engineer is fixing now |
| FIND-D2R-01 | 109 | `check_rating_invariant` reads without a snapshot: 25 false violations in 31,023 checks while one thread finalizes | An engineer is fixing now. Closure: one read transaction and a concurrent finalize-and-check test |

No Critical finding and no open Major in D5 or D1.

## 3. What I checked and found conforming

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Crate edges (architecture §3.2, D-38) | **Conforms.** `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` | `[RAN]` `node ci/check-crate-edges.mjs`, exit 0 |
| 2 | `chess-store` depends only on `chess-domain` among workspace crates; no `chess-rules` edge; `chess-store` re-exports nothing from `chess-rules` | **Conforms** | `[OBS]` `crates/chess-store/Cargo.toml`; `lib.rs` `pub use` list |
| 3 | New dependency | One, `crc32fast`, inherited from the workspace (DL2). `#![forbid(unsafe_code)]`, no `libc`, no `rustix` (D-107) | `[OBS]` Cargo.toml and `lib.rs`; `cargo deny` not run by me `[UNV]` |
| 4 | D-40 (UUID only; integer row keys never cross the store API) | **Conforms for the store's own types.** Public ids are `GameId` and `PlayerId` (UUID newtypes from the `uuid_id!` macro). The only integers are counts, milliseconds, `journal_seq`, ply and error counters. `VariationRef` is gone (compile-fail doctest). I read `api.rs`, `api_analysis.rs`, `error.rs` and grepped `store.rs` and `store/d5.rs` for integer id fields: none. **One exception: FIND-DR-02** | `[OBS]`, grep |
| 5 | Stored format: DDL unchanged by B, D2, D5 | **Conforms.** I compared every `CREATE TABLE` body in `migrations/0001_init.sql` with the SDD 3.5.2 block after stripping comments and whitespace: 20 object names on each side, no difference in any table body | `[RAN]` python comparison (the file is untracked, so git cannot show a diff) |
| 6 | Stored format: journal against architecture 7.6 | **Conforms on the points I could read.** 14 kinds with codes 1 to 14 in the order of the SDD; payload field names for `Start`, `Move`, `Takeback`, `Clock`, `DrawOffer`, `DrawDeclined`, `DrawDropped`, `Suspend`, `GameEnd` (including `content_hash` and the three time totals), `Abort`, `Voided` (including `ended_at`) match the `Fact` table; `Pause`, `Resume`, `SuspendResume` carry `{}`. The frame, CRC-32/ISO-HDLC and 65,536-byte limit were matched byte for byte by the Reviewer (review-chess-store-B.md section 2), which I did not repeat | `[OBS]` `journal/format.rs`; the byte check is `[INF]` from the Reviewer |
| 7 | Rework rulings applied as decided | CH-4 (encode and decode), CH-5 B, CH-11 B, FIND-B-11 B, FIND-B-06 narrowed, D5-01 A, D5-02 B, `VoidSpec.game`, abort text `aborted`, rating rows by `id`: each has code and tests per the Reviewer (rework-1 section 3 and D1-D2-D5 section 3). `journal_version` stays 1. No DDL change | `[INF]` from both Reviews and the evidence |
| 8 | §5.7 contracts | `chess-store` is not in the §5.7 curated list (it is behind `chess-app`); no consumer exists yet (`chess-app` is a scaffold), so no contract is broken. New public items are additions only, plus the changed `HasUnfinishedGames{ids}` (D-117) and the variation signatures (D-134) | `[OBS]` |
| 9 | Gate rules | Q8 stored-format items were approved by Brian before the rework (relay on bead chess-a0r.3.50); no G2/G3 item has been self-approved by any stance | `[OBS]` gate records say "Approved by: nobody" |
| 10 | Test and tool gates | Both Reviewers re-ran fmt, clippy (both feature settings) and the suite five times: 321 and 378 passed. One run of the first-launch race test failed (RW1-02). I did not rerun the suite | `[INF]` from the Reviews |

## 4. Conditions

### GATE-108 (part B)

| # | Condition | Class of the closure |
|---|---|---|
| B1 | FIND-RW1-01 closed under a ruling by Brian on what counts as a data trace (decision package (a)), with a committed test that kills a first launch at random instants (at least 500) and shows 0 refusals | G2 |
| B2 | FIND-RW1-02 closed: a test that holds a creator inside the WAL switch and first schema commit and a second opener that must not be refused; the 200-round FIND-001 test passes in 10 consecutive runs under disk load | G1 |
| B3 | A fresh Reviewer re-checks the changed open path (`open.rs`, `datadir.rs`) after B1 and B2, including the real v1 store case (NF2-2) | process |
| B4 | The original Reviewer findings that have no recorded disposition get one. The evidence file and the gate record mention FIND-B-R01, R02 and R06 only through the CH and B-06 rulings. **No disposition appears for FIND-B-R03 (`close_final` leaves a journal under two names after a crash), R04 (`create` races `probe`, 70 spurious `Locked` in 3,000 rounds), R05 (an incomplete `.rebase` is installed when the journal is missing), R07 (4-process lease stalls of 5.4 s) or R08 (late `open` errors leave the registry row)**. R03 and R05 are format-crash-window items. Each is closed, deferred to D4 by name, or accepted by Brian | G3 for R03 and R05 |
| B5 | Dispositions of FIND-RW1-03 (rebase idempotence, decision package (f), G3), FIND-RW1-04 (arrays accepted as payload), FIND-RW1-05 (`ClockText` accepts what `ClockSpec` refuses; a CH-4-class gap that the apply step must treat as damage or the format must refuse), FIND-RW1-06 to 10 | G3 for 03, 04, 05, 10; G1 for 06 to 09 |
| B6 | The two gate records are corrected: GATE-108 still says "Reviewer pass NOT DONE" and "Reviewer findings: none yet", but two Reviews exist | G0 |

### GATE-109 (D2)

| # | Condition | Class |
|---|---|---|
| D1 | **Major.** FIND-D2R-01 closed: `check_rating_invariant` runs in one read transaction, with a concurrent finalize-and-check test showing zero violations over several thousand checks. Blocks approval | G1 |
| D2 | FIND-D2R-02 (`analysis_used` overwrite) ruled by Brian; recommendation is OR (decision package (b)) | G3 |
| D3 | FIND-D2R-03 (terminal validation) placed; recommendation in the package (c) | G1 plus a note |
| D4 | Tests for the two gaps the mutants found: `abort_game` of a voided game answers `AlreadyFinal`; `void_unreadable_game` of a completed, aborted or voided game answers `AlreadyFinal` (M14, M25); a test for `INSERT OR IGNORE` in `release_and_queue` (N01) goes to D3 acceptance because only D3 can reach it | G1 |
| D5 | D3 acceptance must include FIND-D2R-05 (`void_game` has no precondition: a `Voided` record on a completed row must be refused or defined) | for D3 |
| D6 | One kill test per D2 `before_commit` failpoint committed (FIND-D1R-05); the Reviewer's probe is ready to lift | G1 |

### GATE-110 (D5)

| # | Condition | Class |
|---|---|---|
| E1 | **The gate record is wrong on the tier.** Its header says G1 with the desk review as approver. The rework changed the public variation API, which the record's own rework section calls G2. Brian is the approver. Update the header | G0 |
| E2 | Add `api_analysis.rs` and `store/d5.rs` to the UT-ST-038 integer-identity scan (FIND-D5R-04). My manual grep found nothing today; the scanner would not notice a future leak | G1 |
| E3 | Ruled by Brian or the architect: ordering key (decision package (d1)), the variation index (d2), a `pgn` size cap or the app owning it (FIND-D5R-01) | G1 / G3 |
| E4 | App SDD must say the app produces **one canonical PGN text per line** (the store compares bytes) and that a worker treats `NotOwner` after its own success as success (FIND-D5R-05) | G1 text |

### Crate-wide (all three)

| # | Condition |
|---|---|
| X1 | Failpoint owners (FIND-D1R-01, decision package (e)) and the registry test |
| X2 | `lib.rs` module doc (lines 4 to 9) still says the journal, finalize, abort, void, reset and `FaultPlan` are "not here yet" (Reviewer I-5). Fix the text |
| X3 | Default `cargo test` runs no failure-injection test (FIND-D1R-04); say so in `lib.rs` or the README. `ci/check.sh` runs both, so the gate is real |

## 5. Items that go to Brian (G2 and G3)

| # | Item | Gate | Where |
|---|---|---|---|
| 1 | Approve or reject GATE-108 (G3 and G2) | G3 | after B1 to B5 |
| 2 | Approve or reject GATE-109 (G3 and G2) | G3 | after D1 to D4 |
| 3 | Approve or reject GATE-110 (**G2**, not G1) | G2 | after E1 to E4 |
| 4 | The trace classification that closes FIND-RW1-01 (narrows D-126) | G2 | package (a) |
| 5 | `analysis_used` as a latch | G3 | package (b) |
| 6 | Rebase idempotence (omit an identical closing `Clock`) | G3 | package (f) |
| 7 | CH-5 B read over the whole dropped region (FIND-ST-RW-05), including the duplicate-`seq` false positive | G3 detail | package (g) |
| 8 | FIND-RW1-04 (arrays accepted as payload) and FIND-RW1-05 (clock text range): refuse in the format or leave to apply | G3 | B5 |
| 9 | FIND-B-R03 and R05 dispositions | G3 | B4 |
| 10 | The three architect additions to D-125 (condition F, `NF2_COPY_CAP`, private directory in `temp_dir()`) and the D-127 reading, already listed in Addendum M as needing him | G2 | architecture Addendum M |
| 11 | FIND-RW1-08: a killed opener leaves a 0700 scratch directory with up to 4 MiB of the user's database in `temp_dir()`. Accept the exposure or add cleanup of directories whose pid is dead | G2 | rework-1 |
| 12 | FIND-DR-02 below (a `u64` row id in `RatingPoint`) | G2 | this review |
| 13 | The variation index and any UNIQUE constraint on `(game, from_ply, pgn)` | G3 | package (d2) |

## 6. Desk-review findings

Severity as in the governance skill: Blocker, Major, Minor, Note.

| ID | Severity | Gate | Finding |
|---|---|---|---|
| FIND-DR-01 | Major | 108 | **The NF-4 spec contradicts itself.** Spec 2.2 items 5 and 6 define any non-empty `-wal` or `-journal` as a data trace, and spec rows 5, 7 and 9 plus NF2-1 promise that an interrupted creation completes. A creator killed during the WAL switch or the first schema commit leaves exactly those files. The Reviewer found it by test (RW1-01); a review of the spec would have found it. This is my spec's defect (Architect stance). Route: ruling in package (a) |
| FIND-DR-02 | Minor | 108/109 | **D-40 reading.** `Store::rating_history` returns `chess_domain::RatingPoint`, which carries `id: u64`, the `RATING_HISTORY` row id (`repo/rating.rs:36`). D-40 says integer row keys "never cross the store API". The domain SDD defines `RatingPoint` with `id` because the domain orders by `(at, id)` (D-135). Nobody recorded this as an exception, and the UT-ST-038 scanner cannot see it because the type is in `chess-domain`. Options: record it as an ordering token that is not an identity (G2 text), or give the point a `seq` field instead. Not a blocker |
| FIND-DR-03 | Minor | 108 | **Gate records are stale.** GATE-108, 109 and 110 say "Reviewer pass NOT DONE" while two independent Reviews exist, and GATE-110 carries the wrong tier (E1). The record is the audit trail; correct it (G0) |
| FIND-DR-04 | Minor | 108 | **Spec 3.2 says one thing and the code another**: spec reads the frame at `pos`, code reads the whole dropped region (ST-RW-05). The entry now exists in SDD 3.9.1 note 28 as a proposal; Brian decides |
| FIND-DR-05 | Minor | 108 | **`UndecodableTail` has no consumer** (RW1-11). The scan result, `StoreAhead` precedence and the end-to-end recovery test (spec 3.2 (e)) cannot be checked until D3 and D4 exist. The format claim "never dropped silently" is therefore proven only on the pure functions. D4 acceptance must carry spec 3.2 (e) |
| FIND-DR-06 | Minor | 109 | **The invariant and the store disagree on scope.** `check_rating_invariant` checks ratings only: not `*_rated_games` against game rows, not the `k` of game rows, not the `GAME.*_elo_*` columns against the rating rows (FIND-D2R-06, FIND-D2-10), and an unknown Player discards the violations already found. SDD 3.7.2 says only "current = last reset point plus later deltas", so this conforms to the text. Say in the SDD that it is the whole check, so a caller does not read "clean" as "consistent" |
| FIND-DR-07 | Note | 110 | The statistics rows are store-owned DTOs (D-133). Conforms. `chess-app` must map every `StatsGameRow` field; that test belongs to bead .3.13 and cannot run yet |
| FIND-DR-08 | Note | all | No Mermaid diagram was edited by the reworks in the files I read. Nothing to validate. Not machine-validated by me |

## 7. Not reviewed, not run, assumed

- Not run by me: fmt, clippy, the test suite, coverage, mutation. I relied on the two Reviews (suite 321 and 378 passed in five runs each; line coverage 96.91 %). I ran `node ci/check-crate-edges.mjs` and a text comparison of the DDL.
- Not analysed: the apply path, recovery and `StartupRecovery` (D3, D4: not built); restore, import, backup manager, `create_fresh` (part C: not built); power loss and APFS behaviour; Intel; `cargo deny`; branch coverage and MC/DC.
- I did not recompute the Reviewers' snapshot hashes. If the crate has changed since 22:43Z, the findings above are about that earlier state plus whatever the engineer's current fix does.
- I read `journal/format.rs` field names and kinds but not the frame code or the CRC; for those I rely on the Reviewer's byte-for-byte match.
- Assumed: the working tree, not git HEAD, is the baseline B6 (as the Reviewers assumed).
- No SEI CERT, MISRA or AUTOSAR conformance is claimed (SWE-001 section 6). This is a conformance review against the project's own documents.

## 8. Recording

This desk review is entered as the G1 input for the three gates. **No gate is approved by this document.** GATE-108 and GATE-109 (G3) and GATE-110 (G2) remain with Brian Prascher. I did not edit the gate records, any crate, `requirements.md` or `RELEASE_NOTES.md`; I closed and created no bead and made no commit.
