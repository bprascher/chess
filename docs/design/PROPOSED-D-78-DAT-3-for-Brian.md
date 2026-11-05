# PROPOSED, FOR BRIAN: D-78 / DAT-3, the time of an unfinished turn (not applied)

| Field | Value |
|---|---|
| Document | `docs/design/PROPOSED-D-78-DAT-3-for-Brian.md` |
| Version | 0.1 PROPOSED, 2026-11-04 |
| Task | chess-a0r.1.31 (findings chess-a0r.1.24, chess-a0r.1.25; M-1, M-2, F-06 of the OQ-16 desk reviews; Brian's ruling 1 of 2026-11-04, chess-a0r.1.30) |
| Stance | Architect (author). This is a proposal. **Nothing in it is applied.** No D-78 text, DAT-3 text, journal field, verification case or `SF-` item was changed because of it |
| Decides | **Brian Prascher.** G3 if option B or C (a journal field), G2 if option A (a reading of DAT-3 written into the requirement) |
| Evidence tags | `[OBS file §]` read; `[RAN]` executed; `[INF]` inferred; `[UNV]` not verified. Nothing is built, so every runtime claim is `[INF]` or `[UNV]` |

## 1. The one choice Brian must make

**Does the app accept that a game that is stopped and resumed under-counts the unfinished turn of the side to move at each stop (option A), or must the journal carry the totals at a stop (option B), or in every `Clock` record (option C)?**

**Recommendation: A.** Confidence: **Medium**. The reasons are in section 4. If Brian wants the totals exact for a clean stop, the second choice is B, not C.

## 2. ASM-10 is closed (evidence)

ASM-10 asked: does an untimed game measure `think_ms` (so the three totals are never null and D-78's seed is not all zero)? **Yes, by design `[OBS]`:**

- `Clock::after_move` sets `think_ms = turn_elapsed`; "Untimed games (`spec = None`) keep `turn_elapsed` only, so `think_ms` exists for DAT-3 without any limit" `[OBS SDD-chess-domain.md 3.11.12]`; test UT-DOM-014 "Untimed game still reports `think_ms`"; decision DR-DOM-004 "one `Clock` type for timed and untimed games, every move gets `think_ms`" `[OBS]`.
- `Move{ply, mv, think_ms, ...}` is a `Fact` field and a journal payload field for every game `[OBS architecture §7.6 table]`. The `MOVE.think_ms` column is nullable in the DDL (for imported rows and stepped moves), and "a `Move` without `think_ms` adds 0" `[OBS domain 3.11.13]`.

So the premise of Brian's ruling 1 ("if untimed games record no `think_ms`, the answer is B or C") **does not hold**. The loss in a Training game is not "all the time before the stop"; it is the unfinished turn only, as D-78 says. M-2's worry is answered. Evidence level: design text read (`[OBS]`); the runtime fact is `[UNV]` until UT-DOM-014 runs. ASM-10 is marked closed in architecture section 14 on this evidence; that edit changes neither D-78 nor SF-01.

## 3. Test: do differences in `wall_ms` between journal records already carry the time of the unfinished turn?

**Method.** No code exists, so the test is a model of the design's own record stream, not a test of the product. `[RAN]` a Python model (`scratchpad/wall_model.py`, not committed, 70 lines) generated 2000 random untimed games per case, with the records that architecture 7.6, ALG-APP-01/02, D-58 and D-80 say are journaled (`Move`, `Pause`, `Resume`, `DrawOffer`, `DrawDeclined`, `Suspend`, `SuspendResume`, no `Clock` ticks because an untimed game arms no timer), each with a `wall_ms`. Ground truth is the monotonic think time of the unfinished turn. The derivation is: sum the `wall_ms` gaps between records in which the game is awaiting a move, since the last `Move`, excluding `Pause`, `DrawOffer` and `Suspend` intervals.

| Case (2000 games each) | Games where the turn was running at the stop | Derived equals truth (within 1 ms) |
|---|---|---|
| Clean stop (`Suspend{detach or shutdown}` written), no system sleep while the app was open | 1359 | **1359 of 1359** |
| Clean stop, with the Mac asleep while the app was open (30 % of turns) | 1353 | 706 of 1353 (the wall clock counts the sleep, the monotonic think time does not `[UNV]`: Rust's `Instant` on macOS is expected to exclude sleep) |
| Crash, then recovery writes `Suspend{recovered}` | 1321 | **0 of 1321** (the recovery record's `wall_ms` is the recovery time, so the derived value includes the whole downtime) |

**Result.**

1. **For a clean stop, the `wall_ms` gap already carries the unfinished turn** to wall-clock accuracy `[RAN model; INF for the product]`, provided the system does not sleep with the app open. This confirms the desk review's F-06 observation: D-78's statement that "no record carries it" is overstated for a clean stop.
2. **For a crash it carries nothing usable in an untimed game.** No `Clock` tick exists to bound the time of the stop, and the recovery `Suspend` is written at resume time. A timed game is different: its 1 s `Clock` ticks bound the crash loss to 1 s, and the fall of the running clock gives the unfinished turn `[INF]` (D-78 (4), already available without a format change).
3. A derivation from `wall_ms` mixes wall time with the monotonic time that `think_ms` uses, so it is wrong after a system sleep with the app open and after a wall-clock step `[UNV]`.

So the answer to "do differences in `wall_ms` already carry the clean-stop time?" is **yes for a clean stop, no for a crash of an untimed game**, and the wall-clock derivation is not exact.

## 4. Options, with the costs Brian asked for

Costs are `[INF]` from the design text; nothing is built.

| | A. Accept the loss; amend DAT-3; add a DAT-3 verification case | B. Totals in the `Suspend` payload | C. Totals in every `Clock` record |
|---|---|---|---|
| Journal change | **None** | G3: three fields in `Suspend` (SF-21 payload grows) | G3: three fields in `Clock` |
| Exact when | a game is never stopped | a clean stop | a stop, to 1 s, **timed games only** (an untimed game writes no ticks) |
| After a crash | unfinished turn lost | unfinished turn lost: the `Suspend{recovered}` cannot know the totals, so the D-78 seed is still needed. Two mechanisms | timed: lost at most 1 s. **Untimed: unfinished turn lost**, same as A |
| `Fact` | unchanged | unchanged if the codec adds the totals from the assembler, as it does for `GameEnd` (SF-01); `Fact::Suspend{cause}` keeps its shape | unchanged on the same condition |
| Codec | unchanged | `Suspend` assembler gains `FinalizeInputs`-like totals; a record-side field the decoder must accept and `record_to_fact` ignore | same for `Clock`, on a record written about once a second |
| FF-3 | unchanged | second property gains `Suspend` | second property gains `Clock`; the largest record-rate change |
| SF-01 | unchanged | unchanged (totals still always present in `GameEnd`) | unchanged |
| SF-21 / SF-11-style frozen items | none | `Suspend` payload extended, frozen at release (D-41) | `Clock` payload extended, frozen at release |
| Cost | a Training or Strict game stopped mid-turn shows lower per-Player time and `total_active_ms` than a never-stopped game would | one more payload shape to test and freeze; crash path unchanged | larger record every second for every timed game; no gain for untimed games |
| Reversible | yes, by editing `seed_from_facts` and, after release, `journal_version` 2 | before release by edit; after release `journal_version` 2 with a v1 reader | same as B |

**Why A.**

- The loss is a **statistic** (per-Player time, `total_active_ms`, and so UI-2 and UI-5 "average game length in time"). It is not a move, a clock value or a result, so NFR-2 is untouched `[OBS DAT-3, NFR-2]`.
- The loss is the **unfinished turn of the side to move at each stop**. It is **not bounded in time**: in an untimed game that turn can be hours. This must be said to Brian plainly (M-1). But a long unfinished turn at a stop is often time the player was away with the tab open, and a live game counts the same away time before a move, so the two behaviours differ only in whether the game was stopped during the turn.
- B repairs only the clean stop, and the model shows the clean stop is also the case that a later change to `seed_from_facts` could narrow without a format change (section 3, result 1), at the price of the sleep inaccuracy. C buys nothing for untimed games, which are the only games without a clock to read.
- A is the only option that keeps the journal format unchanged. ASM-6 ("nothing is released, so format changes are cheap now") makes B and C possible, but each is G3 and permanent at release.

**If Brian chooses B instead:** edit architecture 7.6 table (`Suspend` payload), SF-21, store 3.11.1, domain `Suspend` assembler, app DD-APP-18, FF-3, add tests; D-78 is rewritten to "seed from the last `Suspend` totals plus later `think_ms`". Brian's G3 approval is needed. **If C:** the same for `Clock`; not recommended.

## 5. If Brian chooses A: text proposed for the Decision Log and DAT-3 (PROPOSED, not applied)

PROPOSED requirements Decision Log entry (for the systems engineer to enter only after Brian's choice): "2026-11-04. Brian Prascher decided that a game that is stopped (detach, shutdown or crash) and resumed stores per-Player time and total active time that omit the unfinished turn of the side to move at each stop and any draw-offer interval open at the stop (D-78, option A). The loss is not bounded in time. The elapsed start and end times are not affected. The three totals of a game that is never stopped are exact. The omission is written into DAT-3 as an amendment and a DAT-3 verification case is added."

PROPOSED DAT-3 verification case (pass criterion): "A game suspended and resumed N times, with a fake `TimeSource`, stores per-Player time equal to the sum of the time spent in completed turns plus the live time of the final segment; the time of each unfinished turn at a stop is not counted; a game never stopped stores exact totals (UT-DOM-122, UT-DOM-123, UT-APP-32 are the design-level cases)."

PROPOSED DAT-3 amendment wording (shall-text change, so a CR): "... time spent by each Player, excluding the time the app was closed or paused, and excluding the unfinished turn of the side to move at each stop (detach, shutdown or crash) of a saved-and-resumed game."

PROPOSED D-63 and SF-01 reconciliation (M-1 condition C-1): replace "nothing recomputes them from `Move.think_ms`" by "a live finalize and a finalize from the `GameEnd` record never recompute the totals; the seed of a resumed game is derived from `Move.think_ms` (D-78)". This is a text change to approved D-63 and SF-01 wording and is part of Brian's decision.

## 6. What this proposal did not do

- It did not change D-78's text beyond a note that D-78 awaits Brian's choice; it did not change D-63, SF-01, DAT-3, any journal field, any verification case or any SDD seed rule.
- It did not run the product (none exists). The model in section 3 shows the logic of the record stream only; the system-sleep behaviour of `Instant` on macOS is `[UNV]`.
- It did not examine UI-2 and UI-5 beyond the desk review's statement that they inherit the loss.
