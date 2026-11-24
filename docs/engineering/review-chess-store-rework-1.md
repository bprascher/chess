# Review: chess-store rework of 2026-11-22, part 1 (journal format and open path)

| Field | Value |
|---|---|
| Document | docs/engineering/review-chess-store-rework-1.md |
| Stance | Reviewer (adversarial, fresh invocation; did not write the code or the earlier reviews; author's evidence not relied on) |
| Baseline | B6. Rulings: decision bead chess-a0r.3.50. Spec: docs/engineering/store-rework-specs.md sections 1, 2, 3 |
| Snapshot | 2026-11-22T22:41:27Z, copied to the session scratchpad (`.../scratchpad/snap_clean`); tree hash of `crates/chess-store` (every file except `target/`, sorted `shasum` of the `shasum` lines): **b787080c50d6fbf7**, recomputed after the runs and found identical to the live crate at that time (`diff -rq` empty). Wider copy (crates, docs, Cargo.*, tool configs): 1a44bbdfb9ca3ebc |
| Verdict | **FAIL for part 1.** Two Major findings (FIND-RW1-01, FIND-RW1-02) in the NF-4 open path. The journal items (CH-4, CH-5 B, CH-11 B, FIND-B-11 B) are sound with Minor findings; FIND-B-06 narrowed holds. Nothing here is an approval; gate approvals GATE-108 to GATE-110 remain Brian Prascher's |
| Evidence tags | `[RAN]` executed here, `[OBS]` read here, `[INF]` inferred, `[UNV]` not verified |

## 1. How the review was done

- Copied `crates/chess-store`, `chess-domain`, `chess-rules`, the workspace manifests, tool configs, `third_party`, the architecture, requirements, SDD and spec into the scratchpad before anything else. All runs below used that copy and my own `CARGO_TARGET_DIR`s. The live crate was not edited; nothing was committed or pushed; no bead was closed.
- Reviewer-owned tests were added only to the scratchpad copy: `tests/rw1_format.rs` (17 tests) and `tests/rw1_open.rs` (17 tests), plus one unit test and two diagnostic `eprintln!` lines in the copy's `src/open.rs` (not part of any official run; the official runs used a clean copy without them). They are not delivered.
- Official reruns (fmt, clippy, tests, coverage, mutation) used a clean copy without my additions. An earlier run of the five default test passes was discarded because my own test files were in the tree (321 became 336 tests).

## 2. Commands and results [RAN]

| Check | Command (clean copy) | Result |
|---|---|---|
| rustfmt | `cargo fmt -p chess-store -- --check` | exit 0 |
| clippy, default | `cargo clippy -p chess-store --all-targets -- -D warnings` | exit 0 |
| clippy, test-support | same with `--features test-support` | exit 0 |
| tests, default, x5 | `cargo test -p chess-store` | run 1 **FAILED** (exit 101): `first_launch::ct_st_004_concurrent_first_launches_never_delete_the_winners_database_200_rounds`, panic `round 190: Missing`. Cargo stopped after that binary, so run 1 is incomplete. Runs 2 to 5: 321 passed, 0 failed each |
| tests, test-support, x5 | `cargo test -p chess-store --features test-support` | runs 1 to 5: 378 passed, 0 failed each |
| coverage | `cargo llvm-cov -p chess-store --features test-support --summary-only` | exit 0, all tests pass. Lines: total 96.91 %; journal/format.rs 98.34 %, reader.rs 100 %, reconcile.rs 100 %, rebase.rs 91.47 %, open.rs 93.76 %, datadir.rs 94.46 %. Branch coverage is not reported by this llvm-cov build (columns show "-") |

Run 1 coincided with a heavy test of mine (6 threads x 200 rounds of hot-journal recovery) running on the same disk. That is context, not an excuse: the failing test is the author's FIND-001 acceptance test (see FIND-RW1-02).

## 3. Disposition per item

| Item | Disposition | Evidence |
|---|---|---|
| (1) CH-4, refuse values above i64::MAX at encode and decode | **Accepted.** No missed field, no panic, writer never writes what the reader refuses. Two Minor notes (FIND-RW1-05, FIND-RW1-10) | The u64 fields are exactly `think_ms`, `white_ms`, `black_ms` (Move), `white_ms`, `black_ms` (Clock), the three GameEnd totals, and `seq` [OBS format.rs:411 MAX_COUNT, :444 validate, :618 encode_record, :702 decode_frame]. Start, Takeback, draw, Suspend, Abort, Voided have none; `ended_at` and `wall_ms` are i64; `ply`/`to_ply` u32; the header has a u16 version. Store-side u64 values go through `to_i64` (finalize.rs:176-224, game.rs:254). `rw1_ch4_encode_ok_iff_decode_ok_250k` [RAN]: 250 000 random payloads over all 14 kinds, edge values around i64::MAX and u64::MAX: 167 878 encoded and each decoded to an equal record and scanned clean as record 1; 82 122 refused, each for a u64 or pair reason; 0 mismatches. Boundary literals and the 65 536 body edge [RAN] pass. Mutants M01 to M16 all killed |
| (2) CH-5 B, `UndecodableTail` | **Accepted; the new behaviour is right.** One Minor label issue (FIND-RW1-09); consumers do not exist yet (FIND-RW1-11) | Adversarial cases [RAN]: refused frame followed by 1 000 random garbage tails (always `UndecodableTail{at}`); duplicate seq at tail; 1,2,2,3 (Middle); 1,2,5,4 (Middle); 1,2,3,1 and 1,2,3,4,4 (UndecodableTail); truncation at every byte of a 5-record file (never damage; `torn_tail_dropped` exactly when the cut is inside a record); truncation at every byte inside a refused tail frame (torn until the frame is whole, then `UndecodableTail`); length-prefix lies of -10000 to +70000 at the tail (torn, silent) and in the middle (Middle); 200 000 random 1 to 3 bit flips and truncations (records always a prefix 1..n, `intact_end` exact, no panic). Reconcile matrix [RAN]: refused tail gives `plan.damaged = true` for db_seq 2, 3, 4, 5; a CRC-bad torn tail gives false for 2 and 3, true for 4 and 5 (D-66 unchanged). False damage from a power cut: see section 3.1 |
| (3) CH-11 B, no closing Clock after a terminal record; FIND-ST-RW-04 | **CH-11 B accepted. FIND-ST-RW-04 confirmed as a spec deviation** (FIND-RW1-03, Minor) | R ending GameEnd, Abort, Voided: rebased file equals R byte for byte, record count stays 3 on three successive rebases [RAN]. R ending Move, Pause or Clock: count grows 4, 5, 6 on three rebases; spec 3.3 test (e) says the second rebase equals the first, byte for byte. Mutants M29 to M36: M29 to M35 killed (M33 unviable), M36 survived |
| (4) FIND-B-11 B, tolerant decode | **Accepted with a Minor finding** (FIND-RW1-04: positional arrays accepted as structs) | Unknown field in every kind: decodes equal to the plain payload (up to 2 800 payloads) [RAN]. Unknown field survives rebase: the new file's record region starts with R's bytes, and rescans as 3 records [RAN]. 300 000 mutated inputs, each decoded as its own kind and as a random kind (600 000 decodes): 19 103 accepted, 580 897 refused, 0 panics, 250 ms release, every accepted payload re-encodes and decodes equal. Depth: 65 000 nested arrays in an unknown field accepted in 163 microseconds (serde_json skips iteratively); 65 000 unterminated refused. Duplicate known key refused; duplicate unknown key accepted; BOM, trailing text, `1.0`, `-0`, `1e0`, `null` for a required field, missing required field all refused. **Accepted although structurally wrong:** `[]` as Pause, `[1]` as Takeback, `["white"]` as DrawOffer, `[1,2,null]` as Clock |
| (5) FIND-B-06 narrowed, hot-journal exception on a private copy | **Accepted.** Minor findings FIND-RW1-06, 07, 08 | See 3.2 |
| (6) NF-4 | **Not accepted.** FIND-RW1-01 and FIND-RW1-02 (Major). The refusal itself, nothing-written, and the -wal rule are right | See 3.3 |
| Mutation, at least 40 new mutants | 62 hand mutants over format.rs, reader.rs, rebase.rs, reconcile.rs, open.rs, datadir.rs: 56 viable, 49 killed, 7 survived, 6 unviable | Section 6 |

### 3.1 Is the new CH-5 behaviour a false-damage risk for a legitimate crash pattern?

The old test (a CRC-bad record 3, then a CRC-valid repeat of seq 2, was a silent torn tail) now reports `UndecodableTail`. I tried to build a legitimate way to get a CRC-valid frame in the dropped region and found none that this writer produces:

- The writer writes at `acked_len` after an explicit seek (`write_and_sync`) and, on any error, `set_len(acked_len)`; if that cleanup fails it poisons itself and stops writing [OBS writer.rs `append_blocking`, `write_and_sync`, `cleanup`]. So no later frame is laid over an earlier unacknowledged one.
- A rebase never overwrites: it writes `<uuid>.jnl.rebase`, moves the old file aside and hard-links the new one in (rebase.rs swap_in_rebased). No stale generation shares a file with the new one.
- A torn frame cannot become CRC-valid by chance: the CRC is over length and body, a lie in the length prefix gives a CRC mismatch (tested: -10000 to +70000), and a whole frame cannot be embedded in a JSON payload, because a frame holds `0x00` bytes in its length field and JSON strings cannot hold raw control bytes [INF]. The chance of a random CRC match is 2^-32 per candidate [INF].
- What remains is a filesystem that persists a rewrite beyond an `ftruncate` that it did not persist (or an operator copying parts of two files). There a CRC-valid frame whose `seq` is not the expected one is evidence of something wrong, and reporting it is the conservative reading of the ruling. I judge the new behaviour right. `[UNV]` I did not test a real power cut; this is reasoning from the writer code.
- Pre-existing and unchanged: after a failed append whose cleanup also failed, a whole frame sits beyond the acknowledged length and the next scan accepts it as record N although the caller was told the append failed. That is the D-82 design and was not asked about here.

### 3.2 FIND-B-06 narrowed (item 5)

[RAN] with my own fixtures (journal built from an in-flight rollback-mode transaction, copied with the data file):

| Case | Result |
|---|---|
| Real v1 store (Ann and others, made by `Store::open`), hot journal, no marker files (my fixture is an in-flight UPDATE of a player name; the author's NF2-2 uses an in-flight table creation) | `Err(Unavailable{ReadOnly})`; every file under the directory identical by length and hash (ignoring only a zero-length -wal and any -shm that the read-only probe creates, as the spec allows); no scratch left. The clean runs of `nf2_narrowed` also pass |
| In-flight DELETE of every player and computer (raw pages show no players, the rolled-back state has them) | Refused, nothing changed. **The count is decided on the rolled-back copy, not on the raw file**, which is the spec's intent (1.2 step 4) |
| In-flight INSERT of the first player into an empty v1 schema (rolled-back state has none) | Exception applies; open succeeds with `first_launch = false` and zero players: an uncommitted insert is rolled back, as any SQLite writer would |
| `TMPDIR` a symlink to a directory | Works; the symlink's target is empty afterwards |
| `TMPDIR` missing; `TMPDIR` 0500 | Original `ReadOnly` error; real files unchanged |
| 64 pre-planted symlinks `chess-nf2-<pid>-<n>` in `TMPDIR` pointing at a victim directory | The symlinks were not followed (victim empty). Open ended with the original error [INF: after 16 name tries] |
| Kill (SIGKILL) at the `open.nf2_copy` failpoint | One `chess-nf2-<pid>-0` directory left, mode **0700**, empty at that point. It stays after a later successful open: nothing ever cleans it (FIND-RW1-08) |
| Six threads, then four processes, open one recoverable hot-journal directory at the same time | All end healthy, no scratch left. Over 200 rounds x 6 threads: 1 194 `ok`, 3 `Busy`, 3 `Unavailable{ReadOnly}` (FIND-RW1-07) |
| Failpoint `open.nf2_copy` | Reached in the copy path (author's nf2_7 and nf2_2b pass); scratch removed on the error path (Drop) |
| Copy cap 4 MiB | Database above the cap is refused without a copy (author's NF2-6). **The journal's size and the exact boundary are not tested** (M40, M41 survived) |

Not tested: a store whose header says WAL, with a hot rollback journal beside it and no -wal (the fixture converts the store to rollback mode first). `[INF]` condition F (no non-empty -wal) covers the case in which the WAL holds committed data.

### 3.3 NF-4 (item 6)

[RAN] and held: zero-length `chess.db` beside a non-empty `-wal` (with and without `store.id`, with and without `-shm`): `Missing` in 30 to 40 ms, called twice, every file identical by length and hash after both calls. SQLite itself deletes a `-wal` beside a zero-length database even on a read-only open (a read-only connection removed the file in my probe), so the pre-probe guard at open.rs:362 is what protects it: **FIND-ST-RW-01 verified, and no trace is destroyed on the refusal path.** A schemaless non-zero database beside a garbage non-empty `-wal`: `Missing` after 1.04 s (the `WAL_SETTLE` wait, FIND-ST-RW-02: costs one second per refusal, as designed), nothing changed. Half-created first launch (empty `chess.db` and `store.id`, with or without zero-length `-wal`/`-shm`, or a schemaless non-zero database): completes. 300 rounds x 4 threads: every opener succeeded, one first launch each round, no write lost (106 s, then 34 s on a quiet machine). 100 rounds x 3 processes with one opener failing at `open.after_db_created`, `open.after_store_id` or `sync.store_id_dir` (errno 5) in three of four rounds: no `Missing`, no lost write, at most one first launch (tally: 147 `ok` not first, 100 `ok` first, 53 injected `Unavailable`; 51 s). Under six CPU hogs, three more 300-round runs passed; under six `F_FULLFSYNC` loops, one 300-round run passed (460 s); a second and third were stopped by me. **But the races are not clean, see FIND-RW1-01 and FIND-RW1-02.**

## 4. Findings

| ID | Severity | Item | Summary |
|---|---|---|---|
| FIND-RW1-01 | **MAJOR** | NF-4 | A first launch that is killed leaves a state that now answers `Missing` |
| FIND-RW1-02 | **MAJOR** | NF-4 | A live concurrent creator can be refused with `Missing`; the author's own FIND-001 test failed once in ten suite runs |
| FIND-RW1-03 | Minor | CH-11 | A rebase of a rebased file appends another closing Clock; spec 3.3 (e) is not met (FIND-ST-RW-04 confirmed) |
| FIND-RW1-04 | Minor | FIND-B-11 | Tolerant decode accepts JSON arrays as payload structs |
| FIND-RW1-05 | Minor | CH-4 | `ClockText` accepts clock specs that `ClockSpec` refuses |
| FIND-RW1-06 | Minor | B-06 | Cap and scratch mode have test gaps (M40, M41, M44); copy size is not bounded after the check |
| FIND-RW1-07 | Minor | B-06 | Concurrent openers of a recoverable hot-journal directory can get a spurious `ReadOnly` or `Busy` |
| FIND-RW1-08 | Minor | B-06 | A killed opener leaves a scratch directory that holds a copy of the user's database; nothing cleans it |
| FIND-RW1-09 | Minor | CH-5 | A later refused frame counts as a resume point; look-ahead cost is O(markers x 64 KiB) |
| FIND-RW1-10 | Minor | CH-11 | `build_rebased` does not check that `intact_bytes` and `intact` agree |
| FIND-RW1-11 | Info | CH-5 | No code consumes the scan result yet; spec 3.2 (e) cannot be tested |
| FIND-RW1-12 | Minor | NF-4 | Test gaps found by mutants M51, M60, M36 |

### FIND-RW1-01 (MAJOR): a killed first launch is now refused as `Missing`

- **Where:** open.rs:487-522 (`nf4_verdict`: `traces.other || traces.hot_journal` refuses at once; a lone non-empty `-wal` refuses after `WAL_SETTLE`), datadir.rs:278-300 (`data_traces`: any non-empty `-wal` or `-journal` is a trace), open.rs:362 (zero-length database with a non-empty `-wal`). The spec text puts these in rows 7 and 9 of its decision table, so the code follows the spec.
- **Evidence [RAN]:** `rw1_kill_first_launch_at_random_instant_then_reopen`: a child runs `Store::open` on a fresh directory and is SIGKILLed after 2 to 122 ms; the directory is then opened again. Run 1: 120 kills, 12 `Missing`. Run 2: 120 kills, 9 `Missing`. The refused states: (a) `chess.db` 4 096 bytes with no tables and `user_version` 0, a non-empty `chess.db-wal` (32, 4 152 or 24 752 bytes; the 32-byte one is a bare WAL header), `-shm`, `store.id`; (b) a zero-length `chess.db` and a non-empty `chess.db-journal` (7 of 120 in run 1). Replayed from saved copies: SQLite sees 0 tables, `user_version` 0; `Store::open` answers `Missing` after 1.03 to 1.08 s (a). Hand-built (b) (zero-length database, 512-byte journal): `Missing`, journal left in place.
- **Impact:** these directories never held data. Before the rework they completed as a first launch (FIND-002, the interrupted-creation promise of SDD 3.9.1 note 1 and spec NF2-1). Now the user gets `StoreMissing` and must choose `create_fresh`. No data is lost, nothing is written, and the way out exists, but the first-launch crash safety that FIND-002 fixed is gone for roughly 8 to 10 % of kills during the first launch in my runs (the kill instants are my choice, so the rate is not a field figure).
- **Why this is a design conflict and not only a code slip:** spec 2.2 and 2.3 make any non-empty `-wal` or `-journal` a data trace, and row 5 says an interrupted creation completes. They cannot both hold for a creator killed during the switch to WAL or the first schema transaction.
- **Close by:** a ruling (Brian or the architect) on which of these may count as a trace when the database has no schema and the layout holds no data, then a test that kills a first launch at random instants (my test is a starting point) and shows 0 refusals. I do not redesign it here.

### FIND-RW1-02 (MAJOR): a live concurrent creator can be refused

- **Where:** open.rs:497 and 504-505. `traces` is read once. A `chess.db-journal` with content (the creator's rollback journal while it switches to WAL) refuses immediately; only a lone `-wal` gets the settle wait. The probe connection has a 5 s busy timeout, so `schema_appeared()` can block while the creator finishes; the verdict then uses the stale `traces` [INF].
- **Evidence [RAN]:** (1) the author's `ct_st_004_concurrent_first_launches_never_delete_the_winners_database_200_rounds` failed in my default run 1 with `round 190: Missing` (one failure in 10 suite runs; the other 9 and the coverage run passed). (2) Deterministic emulation in the copy's unit tests using the existing `BEFORE_TRACE_SCAN` seam: a thread holds `BEGIN IMMEDIATE; CREATE TABLE t; DROP TABLE t` in rollback mode (journal 512 bytes, lock RESERVED) while the opener scans: the opener answers `Missing` at once (diagnostic line: `hot_journal: true`, no `-wal`), and the same directory opens as a first launch after the creator commits. The emulation shows the logic, not how often production hits it; 3 x 300 rounds under CPU load and 1 x 300 under fsync load did not reproduce the failure.
- **Impact:** FIND-001 ("a loser of the first-launch race is never turned away") is a baselined acceptance property and the author's own test enforces it; it can fail.
- **Close by:** a first-launch race test that holds the creator inside the WAL switch and the first schema commit (the emulation above), passing; and the 200-round test passing in 10 consecutive runs under concurrent disk load.

### FIND-RW1-03 (Minor): rebase of a rebased file is not idempotent

`build_rebased` appends a Clock whenever the last record is not terminal (rebase.rs:127), including when it already is a Clock. Three successive rebases of a file ending Move, Pause or Clock give 4, 5, 6 records; spec 3.3 (e) requires the second rebase to equal the first. Each damage recovery grows the file by one record (about 60 bytes); replay accepts repeated Clocks, so there is no wrong state. Close by: skip the Clock when the last record is a Clock with the same values, and add the spec's test (e) for a non-terminal R. The author already noted this as FIND-ST-RW-04.

### FIND-RW1-04 (Minor): arrays accepted as payloads

`serde_json::from_slice` into a derived struct accepts a JSON array as the positional form (format.rs:491). [RAN]: `[]` is a Pause, `[1]` a Takeback, `["white"]` a DrawOffer, `[1,2,null]` a Clock. Spec 3.4 says wrong types are refused; the rebase then keeps such a frame verbatim. No safety effect, but "structurally wrong record accepted" was the question. Close by: refuse a payload whose first non-space byte is not `{`, with a test per kind.

### FIND-RW1-05 (Minor): `clock_spec` is a u64 field in disguise

`ClockText::parse` (format.rs:175) accepts `0+0`, `4294967296+0`, `18446744073709551615+18446744073709551615` and `86401+0` [RAN], while `chess-domain` `ClockSpec::from_spec_text` refuses everything outside base 1..86 400 s and increment 0..3 600 s [OBS clock.rs:27-33, 66-78]. A Start record the reader accepts may be one the consumer cannot apply: the CH-4 class of defect. `[UNV]` `apply_records` is not built, so I could not show the failure end to end. Close by: parse through `ClockSpec` in `ClockText`, or state in the spec that the apply step must treat it as damage.

### FIND-RW1-06 (Minor): cap and mode gaps

M40 (cap boundary `>=`), M41 (journal size not checked) and M44 (scratch mode 0755) survived the existing tests. The check reads the size with `metadata` and then `fs::copy` (open.rs:222-232) copies whatever the file is by then, so the bound is not enforced during the copy. Close by: tests for a journal over the cap, a database exactly at the cap, and the mode; optionally copy with a length limit.

### FIND-RW1-07 (Minor): spurious errors for concurrent recoverers

Six threads on a recoverable hot-journal directory, 200 rounds: 3 of 1 200 opens returned `Unavailable{ReadOnly}` and 3 `Busy`. The `ReadOnly` ones are openers whose probe saw the hot journal after which another opener recovered it, so `is_interrupted_first_creation` found no journal (open.rs:277 `open_probe` [OBS]). A retry succeeds. The window is wider than before because the copy test takes time.

### FIND-RW1-08 (Minor): stale scratch

A killed opener leaves `chess-nf2-<pid>-<n>` (mode 0700) in the temp directory; a kill during the copy leaves up to 4 MiB of the user's database plus its journal there. The spec accepts that the OS cleans it; nothing in the crate does. Close by: the architect states the exposure is accepted, or the next open removes scratch directories whose pid is dead (the `remove_stale_scratch` pattern already exists for the data directory).

### FIND-RW1-09 (Minor): look-ahead label and cost

`classify_tail` takes any later CRC-valid frame with `seq > last_good` as a resume point without decoding it (reader.rs:153-179 [OBS]). Two refused frames in a row give `Middle{resumes_at}` pointing at an undecodable frame [RAN]; both are damage, so only the label is off. Each sync marker with a plausible length costs a CRC over up to 64 KiB: a 10 MB file of such markers took 2.1 s (release) and 69 s (debug); 30 MB took 6.3 s (release) [RAN]. This needs a damaged or hostile file; the code comment says "bounded by the file size", which is true but with a 64 KiB factor.

### FIND-RW1-10 (Minor): inconsistent rebase inputs not detected

`RebaseInput` takes the intact records twice, as bytes and decoded, and trusts that they agree [OBS rebase.rs:87-90]. With empty `intact_bytes` and one decoded record, [RAN] `build_rebased` returned `last_seq = 2` and the file rescanned as 0 records. The caller (D3/D4) does not exist yet. Close by: a check that the decoded count and last `seq` match a scan of the bytes, or derive one from the other.

### FIND-RW1-11 (Info): no consumer yet

`scan_bytes`, `reconcile` and `build_rebased` have no caller outside tests, so spec 3.2 test (e) (a recovery that moves the journal to `journal/damaged/`) and the `StoreAhead` precedence rule cannot be checked. My checks of "through reconcile" are on the pure functions only. Carry to D3/D4.

### FIND-RW1-12 (Minor): surviving mutants of other kinds

M51 (`schema_appeared`: `||` to `&&`), M60 (metadata error treated as "no content") and M36 (`source != Journal`, which adds `Both`) survived: no test has the version without tables, or tables without a version, in the re-read; none makes `metadata` fail; none passes `RunSource::Both` with stored rows beyond the journal.

## 5. Other observations (not findings)

- The probe may leave a zero-length `-wal` and `-shm` in the data directory; the spec accepts it and my snapshots ignore exactly those.
- `rw1_nf4_*` timing: a refusal that waits `WAL_SETTLE` costs 1.04 s every call.
- Spec section 1.3 says a mutation check is part of the author's evidence; I did not read the author's evidence on purpose.

## 6. Mutation table (hand mutants, scratchpad copy, one mutant at a time)

Tests run per mutant: for journal files `--lib --test journal --test journal_crash --test journal_faults` with `--features test-support`; for open and datadir `--lib --test nf2_narrowed --test nf4_refusal --test hardening --test open --test crash --test first_launch` with the two long race tests skipped. 62 attempted; unviable (did not compile under `-D warnings`): M28, M29, M33, M43, M49, M53 (6). Viable 56: **killed 49, survived 7 (M23, M36, M40, M41, M44, M51, M60)**; score 49/56 = 87.5 %. M23 is equivalent: a frame needs at least 10 bytes, so the last two bytes cannot start one.

| ID | File | Mutation | Result | Killed by |
|---|---|---|---|---|
| M01 | journal/format.rs | MAX_COUNT >> 1 -> >> 2 | KILLED | ch4_seq_above_i64_max_is_refused_at_encode_and_at_decode, ch4_the_boundary_is_the_literal_i64_max |
| M02 | journal/format.rs | in_range <= -> < | KILLED | ch4_the_boundary_is_the_literal_i64_max, ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M03 | journal/format.rs | pair_ok == -> || | KILLED | a_body_above_65536_bytes_is_refused_by_the_writer_and_the_limit_is_exact, ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M04 | journal/format.rs | GameEnd range check removed | KILLED | ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M05 | journal/format.rs | Move pair&&range -> || | KILLED | ch4_the_boundary_is_the_literal_i64_max, ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M06 | journal/format.rs | think_ms dropped from range | KILLED | ch4_the_boundary_is_the_literal_i64_max, ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M07 | journal/format.rs | Clock pair check dropped | KILLED | text_forms_of_sf10_to_sf15_are_enforced |
| M08 | journal/format.rs | total_active_ms dropped from range | KILLED | ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M09 | journal/format.rs | encode seq > -> >= | KILLED | ch4_seq_above_i64_max_is_refused_at_encode_and_at_decode |
| M10 | journal/format.rs | decode seq > -> >= | KILLED | ch4_seq_above_i64_max_is_refused_at_encode_and_at_decode |
| M11 | journal/format.rs | decode seq check removed | KILLED | ch4_seq_above_i64_max_is_refused_at_encode_and_at_decode |
| M12 | journal/format.rs | from_json validate removed | KILLED | ch4_the_boundary_is_the_literal_i64_max, ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M13 | journal/format.rs | to_json validate removed | KILLED | ch4_the_boundary_is_the_literal_i64_max, ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode |
| M14 | journal/format.rs | len range ..= -> .. | KILLED | a_body_above_65536_bytes_is_refused_by_the_writer_and_the_limit_is_exact |
| M15 | journal/format.rs | body_len > -> >= | KILLED | a_body_above_65536_bytes_is_refused_by_the_writer_and_the_limit_is_exact |
| M16 | journal/format.rs | ClockText leading-zero rule removed | KILLED | text_forms_of_sf10_to_sf15_are_enforced |
| M17 | journal/reader.rs | scan seq == -> >= | KILLED | a_wrong_sequence_number_is_bad_and_a_repeated_seq_is_not_resumable, ch5_a_crc_valid_frame_the_decoder_refuses_at_the_tail_is_damage_not_a_torn_tail |
| M18 | journal/reader.rs | UndecodableTail arm -> silent torn drop | KILLED | a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point, a_wrong_sequence_number_is_bad_and_a_repeated_seq_is_not_resumable |
| M19 | journal/reader.rs | resume seq > -> >= | KILLED | a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point |
| M20 | journal/reader.rs | resume `at > pos` removed | KILLED | a_refused_frame_followed_by_an_intact_one_is_a_damaged_middle, ch5_a_crc_valid_frame_the_decoder_refuses_at_the_tail_is_damage_not_a_torn_tail |
| M21 | journal/reader.rs | Kind-error arm no longer marks crc_valid | KILLED | ch5_a_crc_valid_frame_the_decoder_refuses_at_the_tail_is_damage_not_a_torn_tail |
| M22 | journal/reader.rs | Ok-frame arm no longer marks crc_valid | KILLED | a_wrong_sequence_number_is_bad_and_a_repeated_seq_is_not_resumable, a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point |
| M23 | journal/reader.rs | look-ahead bound at+1 -> at+2 | SURVIVED | - |
| M24 | journal/reader.rs | look-ahead step 1 -> 2 | KILLED | a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point, a_wrong_sequence_number_is_bad_and_a_repeated_seq_is_not_resumable |
| M25 | journal/reader.rs | intact_end = pos -> len | KILLED | a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point, ch5_a_crc_valid_frame_the_decoder_refuses_at_the_tail_is_damage_not_a_torn_tail |
| M26 | journal/reader.rs | header version 0 check removed | KILLED | ut_st_078_header_faults_map_to_their_errors |
| M27 | journal/reader.rs | version > -> >= | KILLED | a_clean_file_and_a_header_only_file, a_flipped_byte_never_loses_earlier_records |
| M28 | journal/reader.rs | foreign-store check removed | UNVIABLE | - |
| M29 | journal/rebase.rs | CH-11: ends_terminal from last record -> false | UNVIABLE | - |
| M30 | journal/rebase.rs | CH-11: synthesized move no longer clears ends_terminal | KILLED | ch11_the_closing_clock_stays_when_the_last_record_is_not_terminal |
| M31 | journal/rebase.rs | is_terminal without Voided | KILLED | ch11_no_closing_clock_after_a_terminal_last_record |
| M32 | journal/rebase.rs | is_terminal without Abort | KILLED | ch11_no_closing_clock_after_a_terminal_last_record |
| M33 | journal/rebase.rs | closing Clock always | UNVIABLE | - |
| M34 | journal/rebase.rs | stored rows ply > -> >= | KILLED | ch11_the_closing_clock_stays_when_the_last_record_is_not_terminal, ut_st_088_the_rebase_builder_copies_synthesizes_and_is_deterministic |
| M35 | journal/rebase.rs | synth ply saturating_sub(1) removed | KILLED | ut_st_088_the_rebase_builder_copies_synthesizes_and_is_deterministic |
| M36 | journal/rebase.rs | synthesize when source != Journal (adds Both) | SURVIVED | - |
| M37 | journal/reconcile.rs | damaged = scan_damaged && db_seq>last | KILLED | ch5_the_tail_damage_reaches_reconcile_for_db_seq_3_and_4, ut_st_011_damaged_agreeing_and_store_longer_keeps_the_store |
| M38 | journal/reconcile.rs | damaged db_seq > -> >= | KILLED | ch5_the_tail_damage_reaches_reconcile_for_db_seq_3_and_4, ut_st_010_journal_wins_and_extra_rows_go |
| M39 | journal/reconcile.rs | clocks db_seq <= -> < | KILLED | ut_st_013_clocks_come_from_the_journal_unless_the_store_is_ahead |
| M40 | open.rs | copy cap > -> >= | SURVIVED | - |
| M41 | open.rs | cap checked on db only (journal size unchecked) | SURVIVED | - |
| M42 | open.rs | empty &= -> = (last table wins) | KILLED | nf2_2b_a_store_with_data_reaches_the_copy_and_is_refused, nf2_8_a_copy_that_cannot_be_examined_keeps_the_error |
| M43 | open.rs | table-present test -> true | UNVIABLE | - |
| M44 | open.rs | scratch dir mode 0700 -> 0755 | SURVIVED | - |
| M45 | open.rs | scratch dir never removed | KILLED | nf2_3_a_v1_store_with_a_game_keeps_its_error, nf2_8_a_copy_that_cannot_be_examined_keeps_the_error |
| M46 | open.rs | wal-beside-zero-length-db guard == 0 -> > 0 | KILLED | a_wal_with_content_beside_an_empty_looking_database_waits_for_a_schema |
| M47 | open.rs | WAL_SETTLE 1000 ms -> 0 | KILLED | a_wal_with_content_beside_an_empty_looking_database_waits_for_a_schema |
| M48 | open.rs | hot journal no longer a trace in verdict | KILLED | nf4_4_to_8_empty_database_beside_each_data_trace_is_refused_and_nothing_is_written |
| M49 | open.rs | no-trace shortcut removed | UNVIABLE | - |
| M50 | open.rs | first schema re-read removed | KILLED | nf4_18_a_schema_committed_before_the_trace_scan_is_not_a_refusal |
| M51 | open.rs | schema_appeared || -> && | SURVIVED | - |
| M52 | open.rs | settle-loop schema re-read removed | KILLED | a_wal_with_content_beside_an_empty_looking_database_waits_for_a_schema |
| M53 | open.rs | copy test (G/H/I) bypassed in open_probe | UNVIABLE | - |
| M54 | datadir.rs | is_interrupted: wal not excluded | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| M55 | datadir.rs | chess.db.restoring not a trace | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| M56 | datadir.rs | pre-import- name broken | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| M57 | datadir.rs | tmp-bak-v name broken | KILLED | nf2_5_each_data_marker_alone_keeps_the_error |
| M58 | datadir.rs | wal always has content | KILLED | nf4_12_and_13_benign_names_beside_an_empty_database_are_a_first_launch |
| M59 | datadir.rs | journal always has content | KILLED | nf4_12_and_13_benign_names_beside_an_empty_database_are_a_first_launch |
| M60 | datadir.rs | metadata error -> no content (was: content) | SURVIVED | - |
| M61 | datadir.rs | backups/ no longer a trace | KILLED | nf_2_the_exception_does_not_apply_to_a_directory_with_a_store_in_it |
| M62 | datadir.rs | .DS_Store no longer benign | KILLED | ds_store_files_are_not_traces_of_a_lost_store |

## 7. Conformance statement

- **Tool-checked:** rustfmt (--check), clippy with `-D warnings` (workspace lints) for both feature settings, the compiler's warnings as errors. All clean.
- **Manual:** read format.rs, reader.rs, rebase.rs, reconcile.rs, writer.rs (append and cleanup paths), open.rs (probe, NF-2 copy, NF-4 verdict), datadir.rs (traces); adversarial tests of my own as listed.
- **Standard:** SEI CERT is applied by intent only (SWE-001 section 6). **No SEI CERT, MISRA or AUTOSAR conformance is claimed.** Intent areas looked at: INT/NUM (u64 to i64 narrowing: sound in the journal; `as` casts absent in the changed code), ERR (errors on the copy path map to the original error; one hole, FIND-RW1-07), FIO (scratch mode 0700 checked; symlink planting checked), CON (races: FIND-RW1-02).
- **Deviations / suppressions:** none seen in the changed code; I did not audit every `allow` in the crate.

## 8. Not checked, not run, assumed

- Not run: Intel, a real power cut or storage fault, a store in WAL mode with a hot journal and no -wal, `fs::copy` on a volume other than the temp volume, `cargo deny` and `cargo audit`, Doc-tests beyond what `cargo test` runs, the live crate (the copy was identical at 22:41Z and when rechecked).
- Not analysed: the apply path (`apply_records`) and recovery orchestration (D3, D4), because they are not written; `repo/*` beyond the `to_i64` call sites; part 2 of this review (D5, D2, others).
- Not done: no bead comment was written. No bead was closed or changed. No commit.
- Unverified: the stale-`traces` mechanism in FIND-RW1-02 is inferred; the emulation proves the immediate refusal for a live journal, not the production frequency. The kill-rate figures depend on my kill timing and on this machine.
- Assumed: the reviewer-owned tests in the scratchpad copy are not part of the deliverable.

## 9. Needs a human

- Brian Prascher (or the architect for the spec): a ruling for FIND-RW1-01 (what may count as a trace beside a schemaless database) and acceptance or rework for FIND-RW1-02; risk acceptance or fixes for FIND-RW1-03 to 10; GATE-108 to GATE-110 are not touched by this review.
- The software engineer: fixes and tests as listed under each finding; a second review of the changed code after them.
