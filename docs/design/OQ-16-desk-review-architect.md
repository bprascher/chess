# OQ-16 desk review, system-architect part (chess-a0r.16.1, baseline B2)

| Field | Value |
|---|---|
| Document | `docs/design/OQ-16-desk-review-architect.md` |
| Version | 1.0, 2026-11-04 |
| Task | chess-a0r.16.1, part 1 of 2 (system-architect desk review). Part 2 (systems engineer) is not covered here |
| Stance | Review Board (reviewer). Findings only. I did not write Amendment 8, Addenda A to D or the SDD alignment. I edited nothing but this file |
| Gate | G0 (a review document). This review approves nothing. **Brian Prascher decides G-2** and whether OQ-16 closes |
| Evidence tags | `[OBS file:line or section]` read this session; `[RAN]` executed; `[INF]` inferred; `[UNV]` not verified. Nothing is built, so every runtime claim is `[INF]` or `[UNV]` |

## 1. Verdict

**PASS WITH CONDITIONS.**

The batch is carried into the three SDDs with high fidelity. I found no dropped SF item and no BP disposition that was ignored. The journal format is a sound design for FF-3 at the `Fact` level. But the review found **three Major defects** in the journal write path and recovery path that the Board pass, the amendment and the SDD alignment all missed. Two of them can lose or reorder an acknowledged move or produce a false "damaged journal" (NFR-2), and one makes an approved format rule (SF-16) impossible to implement as written. All three are fixable by text before any code and none needs Brian's gate beyond G1 (the first and third) and G0/G1 (the second, see tier column). Under Brian's own closing condition, OQ-16 **must not close** until the conditions in section 2 hold.

Entry condition for `chess-store` code (OQ-16, D-77 (2)): **not met.** Condition (1) (batch applied) is met except for the Major findings below. Condition (2) needs both reviewers, and this is one of two.

## 2. Conditions that must hold before OQ-16 may close

| # | Condition | Owner | Tier |
|---|---|---|---|
| C-1 | F-01 resolved in `SDD-chess-store.md` (and one sentence in architecture 7.6 Write path): the rule for what the writer does after a failed or ambiguous append | Software engineer drafts; architect desk review | G1 (inside `chess-store`; no format change) |
| C-2 | F-02 resolved: one stamp for `GAME.started_at` and the `Start` record `wall_ms`, stated in `start_game`/`NewGame`, the start order, and the "row without journal" resume path | Software engineer drafts; architect confirms | G1 if text only; G2 if the `NewGame` DTO crosses the `chess-store`/`chess-app` contract (it does, `[INF]`, so treat as G2) |
| C-3 | F-03 resolved: the rebase is either defined (file swap order, what is preserved, seq numbering, crash windows) or removed in favour of the architecture's "keep the damaged file aside" | Software engineer drafts; architect | G1 |
| C-4 | The systems engineer part of this desk review passes, **including the DAT-3 reading that D-78 depends on**. If SE reads DAT-3 as exact, D-78 is withdrawn and goes to Brian as G3 (architecture 11.4.12 says so) | Systems engineer | G2 |
| C-5 | Brian explicitly accepts or reverses D-78, D-79 and D-80. They are the architect's rulings under delegation, not his approval of content. This review checked them (section 5) but cannot approve them | Brian Prascher | G2 / G1 |
| C-6 | The open items XC-APP-A8-2 and XC-APP-A8-4 are ruled (D-80's route to resign depends on XC-APP-A8-2) | Architect | G1 |
| C-7 | The front-end SDDs that contradict D-80 and D-71 are listed in beads before their code starts (F-05). Not required before store code, required before OQ-16 is recorded as "carried" for rules/engine/server/cli/ui | Program manager / software engineer | G0 |

Minor and Info findings do not block closure; they are dispositioned by the owners named.

## 3. Findings

Severity: Major = a defect that can lose, reorder or misreport an acknowledged move, or makes an approved frozen rule unimplementable; Minor = a defect with a bounded effect or a missing statement; Info = a note.

### Major

**F-01. Major. A failed or ambiguous journal append leaves the file in an indeterminate state, and no document says what the writer does next.**
- *Evidence.* The writer API says append is "Cancel-safe: a dropped future either wrote the whole record durably or nothing the reader accepts" `[OBS SDD-chess-store.md:3.7.3, around 515]`. The only truncation rule is for a **new owner** resuming after a crash ("`ftruncate` to `pos`, then fsync" `[OBS store 3.11.2]`). ALG-APP-01 step 4 says that on an append error the session drops its clone, answers `JournalWriteFailed`, and "the clocks of the old state keep running"; step 5 increments `seq` only after success `[OBS app ALG-APP-01 steps 4 and 5]`. `append` runs in `spawn_blocking` `[OBS store 3.7.3]`, which a dropped future cannot cancel `[INF]`. A grep for `truncate` and `poison` over the store and app SDDs finds only the resume-time rule `[RAN grep]`.
- *Failure scenarios.* (a) `write` fails part-way (ENOSPC, the case D-65 and D-80 are built around): garbage bytes remain at the tail. The session continues (step 4), and the 1 Hz `Clock` tick or the next move appends **after** the garbage. The next scan sees a bad record followed by intact records: "damaged middle", not a torn tail. The intact run ends before the garbage, so acknowledged records after it are outside the run that recovery trusts. If the store has not applied them (backlog), they are lost from recovery, an NFR-2 violation, and the user gets a false `JournalUnreadable`. (b) `write` succeeds and `F_FULLFSYNC` fails: the record may reach disk, the session reverts and tells the user the move failed, then the user plays a different move that is appended with the **same `seq`**. After a crash, the scan reads the rejected move, expects `seq+1`, finds a duplicate `seq`, and classifies the acknowledged move as damage or a torn tail. The rejected move is replayed and the acknowledged one is dropped. (c) D-65 part 2: "applies exactly the facts whose records were synced" and discards the rest, but a discarded record can still be on disk and wins on resume (journal wins).
- *Why the amendment missed it.* BP-003 was framed as session-state divergence; the file-state side was not examined. D-65 part 2 and D-80 inherit the gap: the terminal re-check "run at once with the same write path" appends after a possibly partial record.
- *Required.* State one rule in the store SDD and architecture 7.6: on any append error the writer either truncates the file back to the last acknowledged offset and syncs (and if that fails, poisons the writer so every later append returns `WriteFailed` and the only exit is Detach and resume), or it never appends again in that session. Say that `seq` is not reused after an ambiguous fsync failure unless the truncate succeeded. Add failpoints `journal.partial_write` and `journal.fsync_error` and a test: partial record then successful append, then scan must classify no damage.
- *Owner and tier.* Store SDD author, G1. `[INF]` throughout; nothing is built.

**F-02. Major. SF-16 cannot be implemented as written: `GAME.started_at` and the `Start` record `wall_ms` come from two different moments, and no document unifies them.**
- *Evidence.* SF-16 (approved) makes `GAME.started_at` the `Start` record's `wall_ms` "one truth" `[OBS architecture 11.4.2 SF-16; store 3.11.1]`. The start order is: (1) `start_game` inserts the `GAME` row, (2) then `MoveJournal::create` writes the `Start` record, (3) acknowledge `[OBS store 3.9.2, around 672; app SEQ-APP-04 lines 715-730]`. `NewGame` and `start_game` name no `started_at` and no stamp `[OBS store 3.7.2:476; app ALG-APP-07 step 3, around 1044]`. The domain keeps `Fact::Start{setup, started_at}` `[OBS domain 3.11.1:619]`.
- *Failure scenarios.* (a) The live row's `started_at` is earlier than the record's `wall_ms` by the gap between (1) and (2). A restore from the journal writes the later value. FF-3's added clause, "finalize a game live and from its journal alone and compare all `GAME` columns" `[OBS architecture 11.4.6; UT-APP-26]`, fails on `started_at`. (b) SF-04 defines the hash `Date` as the UTC date of `started_at`. A game started in the milliseconds before UTC midnight has one `Date` live and another after a restore, so the stored hash and a re-imported export of the restored row disagree (FF-4). (c) The crash gap between (1) and (2): the row exists with no `Start` record, resume creates a journal for a zero-move game `[OBS store 3.9.2]`, and the new `Start` carries a `wall_ms` later than the row, which makes the row and the journal disagree from the first record. The same gap means `Start` has no stamp from the original start.
- *Required.* Decide one stamp: the `Stamp` taken once by the session at start is passed in `NewGame` as `started_at` and is the `wall_ms` of the `Start` record; for the "row without journal" case the new `Start` takes `wall_ms = GAME.started_at`. State it in `NewGame`, the start order, the resume path and the FF-3 property precondition (see F-10). Add the table test "a restore of a journal with no later edit gives the same `started_at`".
- *Owner and tier.* Store and app SDD authors. G2 if `NewGame` changes (a DTO across the §5.7 contract) `[INF]`.

**F-03. Major. The journal "rebase" after damage is undefined, is not in the architecture, and contradicts D-78 and the seq rules.**
- *Evidence.* Architecture 7.6 says the damaged file is "kept aside as `journal/damaged/`" and the game resumes from the longer run `[OBS architecture 7.6 recovery bullet]`. The store SDD instead says resume "continues on a **rebased journal**: a new file starts with the header, a `Start`, the recovered `Move` records and one `Clock`; `GAME.journal_seq` is set to the new last sequence number in the same transaction; the old file moves to `journal/damaged/`" `[OBS store 3.11.3, end; STATE-ST-01 diagram]`. No document states the file-swap order, the new records' `seq` numbering, their `wall_ms`, or the crash windows.
- *Failure scenarios.* (a) Renumbering: the new file restarts `seq` at 1. If the process dies after the file swap and before the DB transaction, `GAME.journal_seq` is the old, higher value and the new journal's last `seq` is lower, so D-66 (`db_seq > last_intact_seq`) reports damage again and rebases again, creating a second `damaged/` file. The "idempotent" claim of reconcile `[OBS store 3.11.3]` covers the plan, not this file swap. (b) The rebased `Start` and `Move` records need `wall_ms` values; unstated. If the `Start` gets a new time, F-02's problem recurs and `started_at` changes mid-game. (c) The rebased journal holds only `Move` records for plies in force. `Takeback`, `Pause`, `DrawOffer` and the `Move` records of taken-back plies are gone. D-78 (2) says the seed sums `think_ms` over **every** `Move` fact "including plies that a later `Takeback` removed"; after a rebase that is false, and the time of every taken-back ply is silently refunded in a Training game. (d) The `Move` records are rebuilt from `MOVE` rows; the architecture does not say the rows carry `think_ms` and `san`, which the `MoveCheck` of D-79 needs `[UNV]`.
- *Required.* Either define the rebase (write the new file under a temporary name, sync file and directory, rename the old file to `damaged/`, rename the new file into place, then the DB transaction; preserve `wall_ms` of every copied record; keep `seq` continuous by continuing the old numbering or define why not; say that D-78's seed after a rebase omits taken-back plies) or drop the rebase and append to the intact run after truncating, as the architecture implies. Whichever is chosen, update architecture 7.6 so the two documents agree.
- *Owner and tier.* Store SDD author with the architect, G1; G2 if the architecture text changes and Brian wants to see it (a recovery behaviour change).

### Minor

**F-04. Minor. Tier labels of two items look low against Brian's own precedent.** SF-06 is classed G1 `[OBS 11.4.2]`, but it changes what `apply_records` does for `Voided` and requires the app to pass the appended record across the `chess-app`/`chess-store` contract; D-67 was classed G2 for exactly that reason. D-66 is G1, but the store's `RecoveryReport.damaged = Some(DamagedInfo{kind: StoreAhead})` is "a design proposal for the field" `[OBS store 3.11.2]` that the app consumes (CT-APP-30). Brian approved both inside the G-2 batch, so no unapproved change exists; the label should say G2 so that a later reader does not reuse G1 as precedent. Owner: architect (text). Tier: G0.

**F-05. Minor. The front-end SDDs contradict or omit what the batch decided.** `SDD-ui.md:591` says on `JournalWriteFailed`: "move not made; the board reverts; clock keeps running" `[OBS]`. D-65 part 2 and D-80 say the session adopts the durable prefix, never reverts, and runs no clock in the degraded state. `SDD-chess-cli.md:396` and `:616` have the same wording and its error list has no `DurabilityUnsupported` or exit-class mapping for it `[OBS; RAN grep: no match in cli, server, ui or desktop SDDs]`. D-71's front-end behaviour (show the path, the reason and the way out; desktop dialog with Quit) is stated only in architecture 5.5 and 7.1. None of the cli, ui, server or desktop SDDs was edited `[OBS git status]`. Owner: software engineer; tier G1 per SDD (the exit-code class of a new code is G3 under SWE-001 if it adds a public exit code `[UNV: not checked against the exit-code table]`). Add to the plan as C-7.

**F-06. Minor. D-78 states the unfinished turn is not carried by any record, which is overstated, and its tension with Brian's delegation guidance is not shown.** Brian's rule was "add a journal field only if no existing record can carry the information" `[OBS 11.4.12 provenance]`. For a clean stop, the `wall_ms` of the `Suspend` record minus the `wall_ms` of the preceding `Move`, less `Pause` and `DrawOffer` intervals (all records with `wall_ms`), gives the unfinished turn to wall-clock accuracy `[INF]`. D-78 offers narrowing only through `Clock` records and calls untimed games underivable `[OBS D-78 (4)]`. It never discusses the wall-time option, and it closes the cheap pre-release moment for a field (ASM-6) with a DAT-3 reading not yet confirmed. The ruling is defensible (the loss is bounded, `[OBS]` D-78 (3)), but the record should say the option was considered, and Brian should see this when he accepts or reverses it (C-5). Owner: architect. Tier: G2.

**F-07. Minor. D-79 moves the dependency-bump risk of BP-013 from SAN to FEN text.** Any `fen_after` difference stops replay (`FenMismatch`) `[OBS 11.4.12 D-79 (3); domain 3.11.1]`. The tolerance for a text-only difference (en passant square written or not) depends on the `chess-rules` FEN type and is `[UNV]` (XC-DOM-A8-5, still open). If the normalisation is wrong, an old journal can become `JournalUnreadable` after a `shakmaty` bump with every `uci` intact, which is the failure BP-013 set out to prevent. D-79 also reads D-72's wording more strictly than Brian approved (flagged by the architect, finding 4) `[OBS]`. Owner: software engineer at step 1; add a fixture with both en passant spellings before the domain code is accepted. Tier: G1.

**F-08. Minor. `ended_at` of a game ended by the terminal re-check is unspecified.** `decide(ResolveTerminal, stamp)` takes the resume time `[OBS domain 3.11.1, IFP-DOM-2]`. `GameEnd.ended_at` then differs from the time of the mating `Move`, and restore and startup finalize order games by `ended_at` (D-72, BP-012), which orders Elo changes against a computer Player. Whether the game "ended" at the move or at the recovery is a rule that must be stated once. Owner: domain SDD author. Tier: G1. `[UNV]` the SDDs do not say.

**F-09. Minor. D-80 stands up, but the player-visible consequence is not designed.** D-80 does not leave the player unable to leave: `Detach` and `Shutdown` always succeed because they append nothing, and the route to resign is Detach, resume prompt, `open_held` `[OBS D-80; app 4.11 state table; domain T16]`. What remains: (a) if the journal volume stays unwritable, resign is impossible, the game stays `in_progress`, and the unfinished slot (D-27) blocks that Player from starting another game in that mode; the only exit is reset (D-30), which voids the game and discards the Player's history. The front-end must say this and name the volume problem; no SDD does (F-05). (b) For a c–c game, `Abort` is the only exit and has the same route. (c) A degraded `Detach` of a timed game journals no `Suspend`/`Clock`, so the resume restores the last `Clock` record, at most a tick plus the unwritten time behind it; accepted by RULE-5's 1 s bound only for the normal tick `[INF]`. Owner: front-end SDD authors. Tier: G1.

**F-10. Minor. FF-3 as worded does not state its precondition.** The property `record_to_fact(fact_to_record(f, ctx)) == f` `[OBS architecture 7.6 Conformance]` holds for `Start` and `Move` only when `ctx.wall_ms` equals `f.started_at` or `f.played_at`, since the record carries no copy (SF-16) `[OBS]`. The generator must derive `ctx` from the fact. Say so in UT-APP-16/UT-ST-070. Owner: software engineer. Tier: G0.

**F-11. Minor. FF-4 will fail for an imported game whose `FEN` tag equals the standard start.** Such a game has `custom_start = 0` and `start_fen = NULL` (SF-02), so the export omits `SetUp` and `FEN`. The canonical form includes `SetUp` and `FEN` "when present" `[OBS app ALG-APP-09]`. The hash computed at import included the tags; the hash of the re-imported export does not. Decide whether the canonical form normalises a standard `FEN` tag away (a `canonical_form` rule frozen by SF-03 `v1`). Owner: architect and software engineer. Tier: G3 if the canonical form changes before release, because `v1` is the frozen version.

**F-12. Minor. Open rulings that D-80 and the state machine depend on are still open.** XC-APP-A8-2 (held session journals no `SuspendResume`, stays `Suspended`), XC-APP-A8-4 (`Ended` then `Voided` pairs), XC-DOM-A8-3 (the `Completed -> Voided` transition is in the domain table T18 but not in architecture 4.3), XC-DOM-A8-4 and XC-DOM-A8-5 `[OBS app 1509-1511; domain 1233-1235; 11.4.12 findings 1 to 3]`. Owner: architect. Tier: G1. This is condition C-6.

**F-13. Minor. The crash gap in the start order leaves an unacknowledged game that holds a slot.** A crash between `start_game` and `MoveJournal::create` leaves an `in_progress` row and a held slot for a start that was never acknowledged `[OBS store 3.9.2]`. The SDD resumes it as a zero-move game. That is safe for NFR-2 (no move exists) but the Player sees a game they never started, in Strict mode may be unable to start another, and must resume and resign. Say whether startup should void such a row (zero moves, no journal, never acknowledged). Owner: store and app SDD authors. Tier: G1.

### Info

- **I-1.** A final record that fails its CRC with nothing after it is treated as a torn tail and dropped silently, unless the store already holds it (D-66) `[OBS store 3.11.2]`. At-rest damage to the last acknowledged record, with the store backlogged, is lost silently. Bit rot is outside the torn-write model and the design cannot distinguish it from a torn write; noted so the limit is known.
- **I-2.** The CRC check value was re-run: `zlib.crc32(b"123456789") = 0xcbf43926` `[RAN python3]`, matching SF-08.
- **I-3.** ASM-10 (do untimed games measure `think_ms`) gates both SF-01 and D-78 and is still open `[OBS architecture 14]`.
- **I-4.** `SDD-v0.5-consistency-pass.md` is an untracked, partial draft by another invocation (header only when I read it). I did not rely on it.
- **I-5.** The statements in the architecture that "no Review Board pass, desk review or ATAM of this amendment has been held" and GATE-008/009/010 become stale once this review is recorded. Update them when Brian records the outcome; this file is that evidence.

## 4. Adversarial questions, answered

**Does the journal format round-trip (FF-3)?** At the `Fact` level, yes, by inspection `[INF]`: every `GameSetup` field (`game_id`, `matchup`, `mode`, two `Seat`s, `start`, `clock`) has a `Start` payload field `[OBS domain class diagram 213-222; store 3.11.1 Start row]`, and `StartPosition` is defined once with a normalising constructor. `Ended`'s nine reasons map 1:1 to codes `[OBS domain 3.5]`. The `Fact` is unchanged by D-63, D-78 and D-79. Three conditions apply: the FF-3 precondition on `wall_ms` (F-10); `Start.started_at` has two sources (F-02); and the rebased journal does not preserve all facts (F-03). At the **GAME-row** level, D-63 and the `GameEnd` totals make the finalize reproducible from the record (UT-APP-26), subject to F-02.

**Can a completed move be lost (NFR-2)?** Not by the designed happy path: journal first, `F_FULLFSYNC`, then acknowledge. Two designed-for gaps are closed correctly: D-65 part 1 (terminal re-check) and D-66 (store ahead of journal). Three paths remain: F-01 (after a failed or ambiguous append, later acknowledged records can be dropped or the wrong record replayed), F-03 (the rebase swap and renumbering), and I-1 (at-rest damage). F-01 is the one I would stop on.

**One-way-door stored formats tiered correctly?** SF-01 to SF-05, SF-07 to SF-21 are G3: correct. SF-06 and SF-22 are labelled G1 inside a G3 batch: SF-22 is correct; SF-06 should read G2 (F-04). D-58's reclassification to "one-way after release" with an honest score of 4 that says two-way is a judgement Brian approved; I find the reasoning (replay interpretation of already-written journals) sound `[INF]`. D-79 and D-80 introduce no stored format `[OBS]`. D-78 closes the pre-release window for a field (F-06). D-71's new code is additive (G2) but its CLI exit class is not checked (F-05).

**Does D-78 stand up?** Conditionally. Alternatives A to F are tabulated with consequences and rollback (B, C, D are G3, reversible before release by an edit, after release by `journal_version` 2). The ruling is the least irreversible and the loss is bounded and stated. It fails the stop rule only if DAT-3 is exact (C-4), it is undermined after a rebase (F-03), and it omits the wall-time option (F-06).

**Does D-79 stand up?** Yes with F-07. Options B to F are compared; the chosen interface keeps `Fact`, the record format and `fact_to_record` unchanged; the single reading closes the hedge. Rollback before code is mechanical.

**Does D-80 stand up, and can the player leave a game?** Yes, with F-09. The conflict between "appends nothing" and "resign or abort" is stated honestly, and the resolution reuses the D-58 path with no new mechanism. The player can always leave (Detach appends nothing). The cost is that resign needs a writable journal at the resume, and while the volume is unwritable the Player's slot is blocked. The dependency on XC-APP-A8-2 is real and open (F-12). The option C rollback (allow `Resign` with a guard) is stated.

**Any crash, torn-write or recovery path the SDDs leave inconsistent with the architecture?** Yes: the rebase (F-03), the append-failure state (F-01), the start-order stamp (F-02), the "row without journal" resume (F-02c, F-13).

## 5. Coverage

Verdicts: **OK** applied and consistent as read; **OK+** applied with a finding noted; **GAP** applied text conflicts or is incomplete. "Applied where" lists the documents I read; line numbers drift.

### 5.1 SF-01 to SF-22

| SF | Applied where | Verdict |
|---|---|---|
| SF-01 totals in `GameEnd` | arch 7.6 table; store 3.11.1, 3.11.4 (finalize from request); domain 3.11.13; app finalise bullet, UT-APP-26 | OK+ (F-02, F-03 affect D-78 seed) |
| SF-02 custom start | arch 7.2/7.6; store DDL (column and two CHECKs); domain 3.11.6 `StartPosition`; app UT-APP-29 | OK+ (F-11 for imports) |
| SF-03 `sha256:v1:` | store DDL comment; app ALG-APP-09, ASM-APP-06, CT-APP-18 | OK |
| SF-04 hash scope, `Date` | app ALG-APP-09 | OK+ (F-02b, F-11) |
| SF-05 `Voided.ended_at` | store table and 3.11.4; domain; app assembler | OK |
| SF-06 `journal_seq` of voided | store 3.11.4 "Voided applied from a record" | OK+ (F-04 tier) |
| SF-07 kind codes | store 3.11.1; scan rule rejects invalid | OK |
| SF-08 CRC-32 | store 3.11.1, UT-ST-081; check value re-run | OK |
| SF-09 64 KiB | store 3.11.1, 3.11.2 | OK |
| SF-10 `VoidReason` | store, domain 3.5 (UT-DOM-121) | OK |
| SF-11 `ClockSpec` text | store, domain UT-DOM-121 | OK |
| SF-12 UUID form | store DDL CHECK, payload rule | OK |
| SF-13 `start_fen` null | store, domain, app | OK |
| SF-14 half-null clocks | store 3.11.1 decode rule; DDL SF-18 | OK |
| SF-15 reason codes | domain 3.5 table and `journal_code`, store, app codec | OK |
| SF-16 no `Start.started_at` | store, domain, app text | **GAP** (F-02) |
| SF-17 hash CHECK | store DDL; app import rule UT-APP-33 | OK (ASM-9 text closed by app import bullet) |
| SF-18 clock-pair CHECKs | store DDL (`MOVE` and `GAME`), UT-ST-084 | OK |
| SF-19 profile CHECKs | store DDL | OK |
| SF-20 player ids | store DDL | OK |
| SF-21 `Suspend.cause` | store, domain T14, app UT-APP-31 | OK |
| SF-22 `DrawDropped.by` | store table, domain `ReplayWarning::DrawDroppedBy` | OK |

### 5.2 D-63 to D-81

| D | Where carried | Verdict |
|---|---|---|
| D-63 | store, domain, app | OK |
| D-64 | store DDL, domain, app | OK |
| D-65 (1) terminal re-check | domain `pending_termination`, `ResolveTerminal`; app resume and SEQ-APP-09; UT-DOM-117, CT-APP-29 | OK+ (F-08) |
| D-65 (2) durable prefix | app ALG-APP-01 step 5a, UT-APP-28; store only 2 mentions | **GAP** (F-01: file state not covered) |
| D-66 | store 3.11.2, 3.11.3, UT-ST-083; app CT-APP-30 | OK+ (F-04) |
| D-67 | store 3.11.4; app CT-APP-32; domain T18 | OK+ (F-12) |
| D-68 | app ALG-APP-09; store | OK+ (F-11) |
| D-69 | store, domain, app | OK |
| D-70 | store (batch), domain | OK |
| D-71 | store 3.11.5, ASM-ST-4, IFP-ST-03; app CT-APP-33; arch 5.5, 7.1 | OK for store/app; **GAP** for cli, ui, server, desktop (F-05) |
| D-72 | arch 7.6; store 3.11.4; domain; app | OK+ (F-12) |
| D-73 | app IFP-APP-04 | OK |
| D-74 | app 0.4.1 (many); confirmed by Brian | OK (outside the store core; not examined in depth) |
| D-75 | rules SDD (11 mentions), domain (2) | OK (not examined in depth) |
| D-76 | rules, engine, server, store, domain, app mention it | OK (not examined in depth) |
| D-77 | store, app | OK (policy; my verdict does not change it) |
| D-78 | domain 3.11.13, app resume rows, store minors | OK+ (F-03, F-06; C-4) |
| D-79 | domain IFP-DOM-2 and 3.11.1; app codec `record_to_replay_item` | OK+ (F-07) |
| D-80 | app step 5b, state table, UT-APP-28; store 1 mention | OK+ (F-09, F-12) |
| D-81 | no SDD names nextest `[RAN grep, per architecture]` | OK (G0 text) |

### 5.3 BP-001 to BP-017 and Brian's dispositions (architecture 11.4.3)

| BP | Disposition | Applied where (as I read it) | Verdict |
|---|---|---|---|
| BP-001 | D-63, SF-01 | arch, store, domain, app | OK |
| BP-002 | D-65 (1) | domain, app | OK+ (F-08) |
| BP-003 | D-65 (2) | app step 5a | **GAP** (F-01: the file-state half) |
| BP-004 | D-66 | store 3.11.2, 3.11.3, tests | OK+ (F-03 renumbering, I-1) |
| BP-005 | D-64, SF-02, SF-13 | all three | OK |
| BP-006 | D-68, SF-04 | app | OK+ (F-02b, F-11) |
| BP-007 | D-68, SF-03 | app, store | OK |
| BP-008 | D-67 | app CT-APP-32; store 3.11.4 (`void_unreadable_game` with hash, zero moves) | OK |
| BP-009 | D-69, SF-05, SF-06 | store | OK+ (F-04) |
| BP-010 (1) to (10) | SF-07 to SF-16 | store 3.11.1; domain | OK except SF-16 (F-02) |
| BP-011 | D-72, D-58 text and door class | arch 7.6, 11.3; app. I did not read the store startup scan wording in full | OK in arch and app; store scan `[UNV]` |
| BP-012 | D-72 | store 3.11.4 | OK |
| BP-013 | D-72, D-79 | domain, app | OK+ (F-07) |
| BP-014 | D-71 | store 3.11.5 probe, ASM-ST-4; app CT-APP-33 | OK (front-ends F-05) |
| BP-015 | D-73, D-77 | app IFP-APP-04 | OK |
| BP-016 (1) to (4) | SF-17 to SF-20 | store DDL | OK |
| BP-017 | SF-21, SF-22 | store, domain | OK |

## 6. What I did not read or verify

- **Not read:** `SDD-chess-rules.md`, `SDD-chess-engine.md`, `SDD-chess-server.md`, `SDD-chess-cli.md` and `SDD-ui.md` beyond targeted greps; `SDD-desktop-tauri.md` except one grep; `requirements.md`, `test-strategy.md` and `RELEASE_NOTES.md` diffs; `SDD-architect-triage.md`; the full text of `Board-pass-store-journal.md` (I read its summary tables and the first findings; the rest of BP-002 to BP-017 I took from the architecture's mapping); `SDD-v0.5-consistency-pass.md` beyond its header.
- **Read only in part:** store SDD 3.9.1, 3.11.5 to 3.11.10, 3.12, section 6; app SDD outside ALG-APP-01, ALG-APP-09, SEQ-APP-04, SEQ-APP-09, the session table, resume and `open_held` rows, the codec, and section 11 XC rows; domain SDD outside 3.5, 3.10 T14 to T18, 3.11.1, 3.11.6, 3.11.13 and section 10; architecture 5.7, 7.2, 7.4, 9 and 11.3 were not re-read in full.
- **Not verified:** that the `MOVE` table holds `think_ms`, `san` and `fen_after` (F-03d); the exit-code table for `DurabilityUnsupported` (F-05); store startup-scan wording for BP-011; D-74 to D-77 beyond coverage greps; D-81's grep result (taken from the architecture's `[RAN]`); the Mermaid diagrams (I did not run the validator, I edited none).
- **Not possible:** nothing is built, so every runtime claim is `[INF]` or `[UNV]`. I wrote no code and ran nothing except `grep`, `git status` and the one `python3 zlib.crc32` check.
- **Out of scope:** the systems-engineer half of the desk review (requirements effects, DAT-3 reading, NFR-2/TBD-23 classification of D-71).

## 7. Sign-off statement

No human approval is claimed. This review recommends PASS WITH CONDITIONS. OQ-16 closes only when the conditions of section 2 hold and Brian Prascher records it.
