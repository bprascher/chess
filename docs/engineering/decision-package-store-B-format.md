# Decision package: chess-store part B, journal format and API choices

Architect stance, 2026-11-11, baseline B6, beads chess-a0r.3.47 and chess-a0r.3.48. For **Brian Prascher** (G3 or G2 as marked). **Nothing here is decided, reviewed or approved.** Sources: [step2-store-B-evidence.md](step2-store-B-evidence.md) sections 3, 8, 9; `crates/chess-store/src/journal/{format,reader,rebase,writer}.rs`, `src/proc.rs` (read, not re-run); SDD-chess-store 3.6, 3.9.1, 3.11.1 to 3.11.6; architecture 7.6. Tags: `[OBS]` read, `[INF]` inferred, `[UNV]` unverified.

**Door type, once, for the whole package.** Nothing is released and `journal_version` stays 1, so every stored-format choice below is **two-way today** (code, tests and SDD text change; no file in the field). It becomes **one-way at the first release** (D-41, D-34): a v1 journal that a restore replays must stay readable. Journal files are short-lived (a `done/` journal is deleted after a later backup holds the game, architecture 7.6), which softens the one-way door but does not remove it. Decide before the first release; after it, each of these needs `journal_version` 2.

## Summary

| Item | Recommendation | Gate | Confidence |
|---|---|---|---|
| CH-1 missing nullable key is a decode error | Accept as built | G3 | High |
| CH-2 `custom_start` not checked at decode | Accept as built | G3 | High |
| CH-3 text fields not pattern-checked | Accept as built; the app codec and the DDL CHECKs are the validators | G3 | High |
| CH-4 integer widths | Accept, **plus** refuse values above `i64::MAX` at decode | G3 | Medium |
| CH-5 CRC-valid but undecodable record is a bad record | Accept the "never applied" rule; **change**: when it is the last record, report damage instead of dropping it silently | G3 | Medium-High |
| CH-6 header edge cases | Accept as built | G3 | High |
| CH-7 compact JSON, table order | Accept as built | G3 | High |
| CH-8 `damaged/<uuid>.<k>.jnl`, `k` from 1 | Accept as built | G3 (name) | High |
| CH-9 pid check | See FIND-B-01: keep `/bin/kill -0` for v1 | G2 | Medium |
| CH-10 NULL wildcard in `matches` | Accept as built | G1 | Medium |
| CH-11 closing `Clock` always appended by the rebase | Accept, **except** when the last record is terminal | G3 | Medium |
| CH-12 backup temp name `chess.db.tmp-bak-v<N>-<pid>` | Approve the name | G2 (D-83) | High |
| CH-13 API shapes (`append(wall_ms, payload)`, `lease_delete(game, owner)`, others) | Approve; one G1 note in the app SDD | G2 | High |
| CH-14 registration order (before upgrade; after schema on a first launch; guard twice) | Accept as built | G1 | High |
| FIND-B-01 pid liveness | `/bin/kill -0` for v1, with a revisit trigger | G2 | Medium |
| FIND-B-06 hot-journal rollback exception | Approve the narrow exception | G2 | Medium-High |
| FIND-B-11 additive fields | **Tolerant decode** (ignore unknown fields), version bump for any change of meaning | G3 | Medium |
| FIND-B-02, FIND-B-03 SDD text | G0 notes; **entered** in the store SDD section 4 (FIND-B-02 text is my wording, flagged) | G0 | High |

Items where I recommend a change to the as-built code: CH-4, CH-5, CH-11, FIND-B-11. Each is small (decoder or reader branch plus a test). They go to the software engineer only after you decide.

## CH-1 to CH-3, CH-6 to CH-8: accept as built

Each is the strict reading of a silent SDD point, and the strict reading is the safe side before release (a decoder can be loosened later by a newer build; it cannot be tightened).

- **CH-1** (missing key is an error). Options: error (as built); treat a missing nullable key as `null`. Error is safer because a missing key can be a truncated or a different writer's record. Consequence: a future nullable field needs `serde(default)` in the newer build so it can still read v1 records. D3/D4/C/app: none.
- **CH-2** (`custom_start` not compared with `start_fen` at decode). The DDL CHECK (SF-02) catches it when D3 applies the `Start`. D3: a CHECK failure on apply must map to a typed damage or `Corrupt`, not `Internal` (see CH-5). App: the codec builds both from one source, so it cannot disagree `[INF]`.
- **CH-3** (no pattern check on `content_hash`, `result`, `matchup`, `mode`, `eco`, `opening_name`, `san`, `fen_after`, `uci`). Options: validate in the store (needs rules vocabulary, which the store must not have, D-42, D-68); do not validate (as built). App: `JournalCodec` and `record_to_fact` are the validators; a bad `san` with a legal `uci` is already handled (D-72).
- **CH-6** (header: version 0 is `Unreadable`; wrong `game_uuid` is `Unreadable`; `VersionMismatch` is tested before `ForeignStore`). The SDD names three outcomes, not the edges. Reversible before release. D4: all three lead to the same user path (rebase from the store or `VoidUnreadableGame`), except that a `VersionMismatch` journal must **not** be rebased away from a newer build's file: D4 must keep it aside and refuse, not overwrite `[INF]`; I list this as a D4 requirement.
- **CH-7** (compact JSON in table order; decoder accepts any order). Cosmetic for the format; a hex dump matches the SDD table. None.
- **CH-8** (`damaged/<uuid>.<k>.jnl`, `k` counts from 1). Name only; "first unused integer" allows 0 or 1. C: backup and `done/` collection never delete `damaged/` (unchanged).

## CH-4: integer widths (accept, plus a bound)

As built: `ply` u32; `think_ms`, `white_ms`, `black_ms`, time totals u64; `ended_at`, `wall_ms` i64. SQLite `INTEGER` is a signed 64-bit value. I found no `i64::try_from` or bound in `format.rs` `[OBS grep]`, so a record that holds a u64 above `i64::MAX` would decode and then fail at the D3 apply (a CHECK or a conversion error), after an acknowledged write.
- **Options:** (A) accept as built and let D3 map the failure; (B) refuse values above `i64::MAX` in `decode` (a decode error, so the record is a bad record, CH-5); (C) change the field types to i64 with a `>= 0` check.
- **Recommendation: B.** The writer never produces such a value (milliseconds); B moves the failure to the one place that already has a typed "bad record" path. Trade-off: one more decode rule. Two-way before release.
- **D3:** `apply_records` can convert with `i64::try_from(..)` and treat failure as `Internal` (cannot happen). **D4, C, app:** none.
- Confidence Medium: I did not check what the D3 CHECKs would do with such a value (D3 is not built `[UNV]`).

## CH-5: a CRC-valid record that the decoder refuses (accept the rule, change the tail case)

As built: such a record is never returned or applied; it is handled like any bad record (`reader.rs`): with intact records after it it is damage, **as the last record it is a torn tail, dropped silently** (unless `db_seq` exceeds it, D-66).
- **Problem.** A torn write fails the CRC with overwhelming probability. A record that passes the CRC and fails the decode is **not torn**: it is a writer bug, a newer build's record (FIND-B-11) or an incompatible one. The SDD's reason for dropping a torn tail silently ("it was never acknowledged") does not apply, because this record **was** acknowledged (synced). Dropping it silently can lose a completed move, which the architecture forbids (NFR-2, DAT-4).
- **Options:** (A) as built; (B) keep "never applied", but report a CRC-valid, undecodable last record as **damage** (`JournalUnreadable{game_id, recovered_moves}` through the existing D-66 damage path, no look-ahead needed); (C) refuse to open the journal at all (too strict: one bad record blocks a game).
- **Recommendation: B.** Cost: a reader flag (a new `DamageKind` tail variant) and one test; the file format is unchanged. With FIND-B-11 tolerant (below) the main cause (an additive field) disappears, which leaves B as the net for real bugs.
- **D4:** resume and startup recovery must treat the new tail-damage flag like a middle damage with no resume point: rebase from the intact run, journal kept in `damaged/`. **D3:** the reconcile input already has a damage flag (UT-ST-012), so no new function. **App:** the existing `JournalUnreadable` message. **C:** none.
- Reversibility: reader behaviour, two-way even after release (no byte of a file changes). Confidence Medium-High.

## CH-9 and FIND-B-01: pid liveness without `libc`

SDD 3.11.6 says a row is live when its pid exists (`kill(pid, 0)`) and its heartbeat is at most 30 s old. D-107 admits no `libc`, `rustix` or `unsafe`; D-111 forbids `unsafe` in the workspace. `std` cannot signal a pid. As built, `proc.rs` runs `/bin/kill -0 <pid>`: exit 0 means exists; "No such process" means gone; **any other outcome counts as exists** `[OBS]`. On this Mac `kill -0 999999` gave "No such process" and `kill -0 1` gave "Operation not permitted" `[RAN by the engineer]`.

| Option | Correctness | Security | Cost | Reversibility |
|---|---|---|---|---|
| A `/bin/kill -0` (as built) | exact for a missing pid; a tool or message failure falls to "exists", so it errs toward refusing, never toward deleting a live row | absolute path, fixed arguments, a `u32` formatted by us (no shell, no injection); depends on `/bin/kill` being the system binary (SIP-protected on macOS `[INF]`); App Sandbox, if ever enabled, may deny the spawn `[UNV]` | one process spawn per foreign row per check (not timed `[UNV]`); at most a few rows | two-way: one function |
| B admit `rustix` | exact, in-process, no message parsing | safe API, but 4 more crates and a reversal of D-107 (a G2 amendment) | build and audit cost, `cargo-deny` surface | two-way |
| C admit `libc` | exact | one `unsafe` block, reverses D-107 and D-111 | high for one call | two-way, but weakens the workspace lint |
| D heartbeat age only (30 s) | a killed process blocks a different-version or upgrade start for up to 30 s (it refuses with `OtherVersionRunning` or `Exclusive`, self-heals, no data at risk); CT-ST-014 and CT-ST-015 "refusal ends at once after `kill -9`" become "within 30 s" | none | none | two-way, but changes SDD 3.11.6 and two tests |

- **Recommendation: A for v1.** It honours your D-107 as written (no new edge), it fails safe, and it can be replaced behind one function. Trade-off: shelling out is unusual and its cost is untimed. **Conditions:** (1) the engineer times the check once (target `ESTIMATE` under 20 ms; if it is above 100 ms, move to B); (2) revisit trigger: the desktop app enables App Sandbox, or `/bin/kill` is missing on a supported macOS release, or the cost shows in the open path; (3) the SDD text for 3.11.6 gets a dated note that the check is by `/bin/kill -0` with the fail-safe rule. If you prefer no shelling out at all, B is my second choice and D my third (D is the only one with no cost but it weakens two verified behaviours).
- **D3, D4, C:** none. **App:** none. Confidence Medium: I could not test sandboxed or localized behaviour `[UNV]`.

## CH-10: NULL wildcard in `matches` (accept)

As built: a NULL stored `think_ms`, clock pair or `played_at` equals any journal value in the **reconcile plan**; the plan is "delete rows with `ply >=` min(`replace_from`, `rows_to_delete_from`), then upsert `rows_to_write`". Reason: a rebase synthesizes `Move` records from rows with NULL columns (`think_ms.unwrap_or(0)`), and without the wildcard the plan after a rebase is never empty (UT-ST-088). Cost: a stored NULL stays NULL when the journal holds a value, if the ply is otherwise equal. This affects `think_ms`, clocks and `played_at` only, not the move. D3: the ordinary apply path (`seq > journal_seq`) upserts full values, so only the reconcile plan skips; D3 needs one test for that. Recommendation: accept; confidence Medium (this is a judgement on a rare case). Gate: G1 inside the crate.

## CH-11: the rebase always appends the closing `Clock` (accept, with one exception)

As built: after copying the intact run (and any synthesized `Move` records), the rebase appends one `Clock{clocks, running_side: null}` in both the "journal wins" and "store wins" cases. SDD 3.11.3 reads "then append one `Clock`" after both cases `[OBS]`.
- **Problem.** If the intact run ends in `GameEnd`, `Abort` or `Voided` (a damaged trailer after a finished game), a `Clock` after it is a fact after a terminal state. Whether `Game::replay` accepts that is **not verified** `[UNV]`; domain T18 allows `Voided` after `Ended` only. D-94 says no `Clock` precedes a terminal record; a `Clock` after it is not mentioned.
- **Options:** (A) as built; (B) omit the closing `Clock` when the last record of the rebased file is terminal; (C) never append it when the journal wins (breaks the SDD sentence and UT-ST-090).
- **Recommendation: B.** Trade-off: one condition and one test; the SDD sentence gets "unless the last record is terminal". Record content, so G3 text. **D4:** the resume path already decides terminal re-check before `Suspend` (D-65), unchanged. **App:** none. Confidence Medium (the replay behaviour is unverified; if the domain accepts it, A is also fine).

## CH-12: new data-directory name `chess.db.tmp-bak-v<N>-<pid>` (G2, D-83)

Part A used `chess.db.tmp-bak-v<N>`; two processes upgrading at once collided on it (NF-1). A per-process name removes the collision; a sweep removes the old name and temps of dead pids and never a live pid's `[OBS evidence 13]`.
- **Options:** (A) per-process name (as built); (B) one name with a lock; (C) a temp directory (`backups/tmp/`), which is also a new name.
- **Recommendation: A.** It is transient (never read, never restored), the sweep is bounded, and the name cannot match `chess.db.bak-v*`. Trade-offs: a temp of a reused live pid lingers until that process exits (disk only); the sweep of a later release must know both names. Two-way (name only). Confidence High. Needs the SDD 3.5.1 row updated (the SDD text still shows the name without `<pid>`; **not edited here**, because it is part of this approval).
- C: the restore and `done/` collection ignore `tmp-bak-*` names (unchanged).

## CH-13: API shapes that deviate from the SDD (G2)

- **`append(wall_ms, payload) -> seq`: the writer assigns `seq`.** The writer owns `acked_len` and `next_seq` and the D-82 rule that a failed append does not consume a `seq`; a caller-assigned `seq` allows gaps and duplicates. Recommend approve. **Consequence for the app:** `JournalCodec` (DD-APP-18) produces `(wall_ms, Payload)` and receives the `seq`; if the app SDD wrote `fact_to_record` returning a record with a `seq`, it needs a G1 note (not entered by me: I did not find a statement either way `[UNV]`). D3: `apply_records` takes records that carry the returned `seq`.
- **`lease_delete(game, owner)`.** The SDD lists only the game. Without the owner, a stale holder could delete the row of the new owner after a takeover; the test shows the old owner cannot (compare-and-set). Recommend approve. App: passes its own owner.
- **`Exclusive{reason: ExclusiveReason}` (enum, `Migration`), `OtherVersionRunning{version: String, kind: String}`, `LeaseBeat::{Held, Lost{owner}, RowGone{..}}`, `StoreOptions::with_process(kind, version)`.** The SDD says the names are proposals. Approve. App: maps `OtherVersionRunning` to the existing front-end message; `with_process` is how it names itself in the registry.
- Reversibility: two-way (signatures, before `chess-app` code). Confidence High.

## CH-14: registration order (accept)

An existing store registers before the upgrade; a first launch registers after the schema exists; the "alone" guard runs before the pre-upgrade backup and again inside the exclusive transaction (FIND-010). This is SDD 3.9.1 and 3.11.5 read together. Known cost (FIND-B-07): two simultaneous starts that both need an upgrade can both be refused; the user retries; no data is at risk. Accept. None for D3, D4, C; the app maps `Exclusive` to a "retry" message.

## FIND-B-06: hot-journal rollback exception to SDD 3.9.1 note 1 (G2)

Note 1 says no write happens before the checks. A first creation cut off by a kill can leave a rollback `chess.db-journal` that a read-only SQLite connection cannot open (`ReadOnly`), so the directory would be wedged for ever (NF-2).
- **Options:** (A) a narrow exception: when the probe answers `ReadOnly` and the directory shows **only** a cut-off first creation (a non-empty `chess.db-journal`, no journal content, no backups, no `chess.db.bak-*`), open `chess.db` read-write once so SQLite rolls the journal back, then probe again; (B) no exception: `Corrupt`/`ReadOnly` forever, the user must delete files by hand; (C) always open read-write (drops the read-only probe, which protects a newer-schema store).
- **Recommendation: A.** The state it applies to holds no user data by its own conditions; the change to `chess.db` is the rollback SQLite itself would do on any ordinary read-write open `[INF]`; the negative test shows a directory with a store in it keeps the error and no byte changes `[OBS]`. Trade-off: one write before the checks, in one named state. Not a stored-format change. Two-way. Confidence Medium-High (the conditions are the engineer's and the reviewer's NF-2 judgement; power-loss behaviour was not run `[UNV]`).
- **SDD text:** a dated note at 3.9.1 note 1 naming the exception and its four conditions. **D4, C:** a restore into a hot-journal state is not covered (C should check `chess.db-journal` too, a note for the C task). **App:** none.

## FIND-B-11: additive fields in payloads (G3)

SDD 3.11.1 says the payload is "tolerant of additive fields" and also `deny_unknown_fields`. As built: unknown fields are refused. A newer build that adds a field without bumping `journal_version` then makes an older build treat those records as bad (CH-5), and with the as-built CH-5 a bad **last** record is dropped silently.
- **Options:** (A) strict as built, and every new field bumps `journal_version` (old build refuses the file: `VersionMismatch`); (B) **tolerant**: unknown fields are ignored on read, never written by this build, and **kept byte for byte by the rebase** (it copies record bytes); any change that an old build must not ignore (a new kind, a changed meaning, a removed field) bumps `journal_version`; (C) as built, no policy.
- **Recommendation: B.** Reason: it is the only choice that fails safe if a developer forgets the bump (an old build ignores an extra field instead of dropping an acknowledged move). Strict-now cannot be turned into tolerant later for builds already in the field; tolerant-now can be tightened for new files by a version bump. Trade-offs: a stored-field typo in a future writer is ignored by the decoder, so the writer's tests must pin the field names (UT-ST-070 does); the SDD sentence `deny_unknown_fields` and the test that says "an unknown field is a decode error" change.
- **Door:** one-way at release, as it fixes what a v1 reader does with a v1.1 file. **D3:** none. **D4:** `build_rebased` extends the file with the intact run's raw bytes (`input.intact_bytes`) `[OBS rebase.rs]`, so unknown fields survive a rebase; no test pins that yet `[UNV]`. **C:** a restore replays whatever journals exist, same rule. **App:** a new field in a payload is additive for old builds only if `record_to_fact` can do without it.
- Confidence Medium: the downgrade or cross-version path is rare (D-16 allows one version live at a time) but the cost of B is small.

## FIND-B-02 and FIND-B-03: SDD text (G0)

- **FIND-B-03.** Five failpoints are registered and used but not in the SDD section 4 list: `journal.truncate_error`, `sync.journal_create_file`, `sync.journal_create_dir`, `sync.journal_move_dir`, `sync.journal_rebase_file`. **Entered** as a dated note in the store SDD section 4 together with FIND-D1-02 (the 10 + 6 names of part D1).
- **FIND-B-02.** SDD 3.8 and the section 4 `Code` column name `sys.rs` and `journal/recover.rs`; the code has no `sys.rs` (D-107, D-111) and no `recover.rs` yet (D4); it has `journal/rebase.rs`, `journal/files.rs` and `proc.rs`. **Entered** as one sentence in the same note (wording of fact only; the `Code` cells are not rewritten).

## What this package does not cover

FIND-B-04 (timers belong to `chess-app`: I agree, it keeps `Store` free of timers, and the app SDD owner should state it), FIND-B-07 to B-10 and B-12 to B-17 (minor or info; no decision needed from you except FIND-B-12, which is D1-03, pending under chess-a0r.3.43). Not run: power loss, sandboxed spawn, non-English `kill` messages, the cost of the pid check.
