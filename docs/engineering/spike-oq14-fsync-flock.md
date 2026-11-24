# Spike OQ-14: file locking, full sync and related primitives on macOS

| Field | Value |
|---|---|
| Document | docs/engineering/spike-oq14-fsync-flock.md, version 0.1 (draft, not reviewed) |
| Date | 2026-11-09 |
| Task | beads chess-a0r.3.1 (baseline B4), widened by FIND-R23-004; also covers spike item S1 of dependency-lookup-step1.md section 6A |
| Stance | Engineer only. No Reviewer pass has been made. Nothing here is approved |
| Gate | GATE-S01: **G0** (throwaway code outside `crates/`, no design effect). The recommendations in section 5 would be G1/G2 changes to architecture 3.4 and the store SDD; they are findings, not edits. Approver for those: system-architect (G1) / Brian Prascher (G2). Status PENDING |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1, aarch64-apple-darwin `[RAN rustc -V; cargo -V]` |
| Volume | `/System/Volumes/Data` is APFS `[RAN df, mount]`. The spike work dir `spikes/oq14/oq14-work` is on it |
| Network | **None used.** `crc32fast 1.5.4` and `rustix 1.1.6` came from the local cargo cache (`cargo build --offline`) |
| Code | `/Users/brianprascher/Projects/chess/spikes/oq14/` (own `[workspace]` root, so no `exclude` was added to the repository `Cargo.toml`) |

## 1. Answer

**The Rust standard library is enough for every file primitive the store needs.** `flock`, a full drive-cache sync, `ftruncate`, directory sync and a no-replace rename (by `hard_link` then `remove_file`) are all in safe `std` on the 1.99.1 toolchain. Neither `rustix` nor `libc` is required, and `chess-store` does not need `unsafe`. The only new crate is `crc32fast` (already in architecture 3.4 as D-6). The one thing `std` cannot do is a *single atomic* no-replace rename (`renamex_np` / `RENAME_EXCL`); if the architect wants that, it costs the `rustix` crate (safe API, no `unsafe` in our code). The design does not need it (item 4).

## 2. Verdict per item

| # | Item | Verdict | Unsafe in our code |
|---|---|---|---|
| 1 | `flock` locking, contention, release on death | **std suffices** (`File::lock`, `try_lock`, `lock_shared`, `try_lock_shared`, `unlock`) | none |
| 2 | `File::sync_all` is `F_FULLFSYNC` | **std suffices.** Yes on Apple targets; `sync_data` is also `F_FULLFSYNC` | none |
| 3 | `ftruncate` then full sync | **std suffices** (`File::set_len` then `sync_all`) | none |
| 4 | No-replace rename (D-83) | **std suffices** with `hard_link` then `remove_file`. Atomic `RENAME_EXCL` **needs `rustix`** (safe) or `libc` (unsafe) | none (std route or rustix) |
| 5 | Directory fsync | **std suffices** (`File::open(dir)?.sync_all()`) | none |
| 6 | CRC-32 ISO-HDLC | **needs a crate** (`crc32fast` 1.5.4 passes the check value); `std` has none | none |
| 7 | SQLite `json_valid` CHECK in the *bundled* SQLite | **[UNV]** (no offline binding). Proxy with the system `sqlite3` passes | n/a |

## 3. Evidence

### 3.1 Item 1: locking

- `[OBS library/std/src/sys/fs/unix.rs:1321-1338, 1369-1392, 1404-1425]` (toolchain source under `$(rustc --print sysroot)/lib/rustlib/src/rust/library/std/src`): `lock`, `try_lock`, `try_lock_shared` call `libc::flock` with `LOCK_EX`, `LOCK_EX|LOCK_NB`, `LOCK_SH|LOCK_NB`; the cfg list includes `target_vendor = "apple"`. `WouldBlock` maps to `TryLockError::WouldBlock`; other errors to `TryLockError::Error`. `unlock` uses `LOCK_UN` (line 1454).
- No feature gate is needed: the spike builds on stable 1.99.1 `[RAN cargo build --offline -> Finished]`.
- `[RAN ./target/debug/oq14-spike oq14-work]`:
  - L1 a second `open` of the same path in the same process gets `WouldBlock` (true). `flock` belongs to the open file description, so one process cannot take the same journal twice.
  - L2 another process while held: `WOULDBLOCK`. L3 after `unlock`: `ACQUIRED`.
  - L4 two shared locks coexist.
  - L5 a child holds the lock and prints `LOCKED`; another process gets `WOULDBLOCK`; after `SIGKILL` of the holder the next process gets `ACQUIRED`. **The lock is released on process death.**
  - L6 the lock survives a `rename` of the locked file (the other process, opening the new path, gets `WOULDBLOCK`).
- Not tested: lock on a network volume, lock inherited through `fork` without close-on-exec (Rust opens files with `O_CLOEXEC` `[INF]`, not verified here), `SIGSTOP`/`SIGTSTP` behaviour of a lock holder (architecture row at line 1180 mentions "raising SIGTSTP: file-lock spike"). A stopped holder keeps the lock `[INF]`, `[UNV]`.

### 3.2 Item 2: `sync_all` is `F_FULLFSYNC`

- `[OBS library/std/src/sys/fs/unix.rs:1264-1276]` `fsync`: on `target_vendor = "apple"` it calls `libc::fcntl(fd, libc::F_FULLFSYNC)`; elsewhere `libc::fsync`. It runs under `cvt_r` (retries on `EINTR`). **There is no fallback to `fsync`: an `F_FULLFSYNC` error is returned to the caller.**
- `[OBS unix.rs:1278-1285]` `datasync` is **also** `F_FULLFSYNC` on Apple. `sync_data` is not a cheaper option.
- `[RAN]` timing, 50 iterations of `write 64 bytes` plus sync, same APFS file: Rust `sync_all` 4.08 ms mean; Rust `sync_data` 4.00 ms; C `fsync` 0.029 ms; C `fcntl(F_FULLFSYNC)` 4.10 ms (`c/cmp.c`). `sync_all` matches `F_FULLFSYNC`, not `fsync`, by two orders of magnitude. This supports the source reading; it is one machine and one drive `[RAN]`.
- Error propagation `[RAN syncpath]`: `sync_all` on `/dev/null` returns `raw_os_error = Some(19)` (`ENODEV`, "Operation not supported by device"), kind `Uncategorized`. So an unsupported target yields an `io::Error` with the errno intact (`e.raw_os_error()`), which is what D-71's `DurabilityUnsupported{path, errno}` needs.
- Gap: I could not produce `ENOTSUP` or `ENOTTY` (the two errnos D-71 names). A FAT32 disk image (`hdiutil`, mounted, then detached and deleted) accepted `F_FULLFSYNC` (`PROBE ok`). No network or exotic volume was available. `ASM-11` stays open for a real unsupported volume `[UNV]`.

### 3.3 Item 3: `ftruncate` then full sync (D-82)

- `[OBS unix.rs:1466]` `File::set_len` is `ftruncate64` under `cvt_r`.
- `[RAN]` T1: after an "ambiguous" 100-byte append, `set_len(acked_len)` then `sync_all`; reopen: length 10496 equals `acked_len`. T2: seek to `acked_len`, write, `sync_all`: length 10500. The cursor is not moved by `set_len`; the writer must seek or use positioned writes (`FileExt::write_at`, std, `std::os::unix::fs`; not exercised here `[UNV]`). D-82 says "write the record at `acked_len`", which fits.
- Not provable here: that APFS makes the truncation durable after power loss. A `[RAN]` test cannot show that; `F_FULLFSYNC` on the same fd after `ftruncate` is the documented-by-Apple approach `[UNV]` (Apple documentation not fetched; no network).

### 3.4 Item 4: no-replace rename (D-83)

- `[OBS]` `grep -rn "renamex_np\|RENAME_EXCL" <std src>` finds nothing. `std::fs::rename` is `libc::rename` (`unix.rs:1861`).
- `[RAN]` N1 `std::fs::rename` over an existing file replaces it silently (so plain `rename` must not be used for D-83).
- `[RAN]` N2 `std::fs::hard_link(src, dst)` onto an existing `dst` fails with `AlreadyExists`, raw errno 17, and leaves `dst` untouched. With no `dst`, link succeeds; both names exist until `remove_file(src)`; afterwards only `dst`. `[OBS unix.rs:1958]` `hard_link` uses `linkat`. Cross-directory link on APFS: `[RAN ln j/u.jnl j/damaged/u.1.jnl]` succeeded (same inode, link count 2); a second `ln` to the same target failed `File exists`. This was done with the shell `ln`, not through the Rust code, so the Rust cross-directory case is `[INF]`.
- Atomic variant: C `renamex_np(..., RENAME_EXCL)` gives errno 17 when the target exists and succeeds otherwise `[RAN c/rn.c]` (system call exists on this macOS; `std` cannot call it).
- `rustix 1.1.6` (cached, read and run): `[OBS rustix-1.1.6/src/fs/at.rs:298-302]` `renameat_with` is available on apple; `[OBS src/backend/libc/fs/types.rs:549-550]` `RenameFlags::NOREPLACE` maps to `RENAME_EXCL`; `[OBS src/backend/libc/fs/syscalls.rs:595-625]` it looks up `renameatx_np` at run time and returns `NOSYS` if the symbol is missing and flags are non-empty. `[RAN]` the spike subcommand `excl` returned `EXCL err File exists (os error 17)` for an existing target and `EXCL ok` otherwise. The call site is safe Rust (`#![forbid(unsafe_code)]` holds in the spike). Cost: `rustix` plus `bitflags`, `errno`, `libc` (`cargo tree`), all new to the workspace, and the crate itself contains `unsafe`.
- Fit with the D-83 crash windows `[INF]`: with link then unlink, a crash between the two calls leaves two names for one inode. For the second rename (`.rebase` to `<uuid>.jnl`) the existing startup rule (SDD-chess-store.md:235, "deletes it when `<game_uuid>.jnl` exists") already resolves that state with no data loss. For the first rename (damaged `.jnl` to `damaged/<uuid>.<k>.jnl`) a crash between link and unlink leaves the damaged file under both names; the rescan finds the damage again and step 3 picks the next unused `k`, so `damaged/` may end up with a byte-identical duplicate. That is harmless but the SDD crash-window list (a)-(c) at line 825 does not mention it, so CT-ST-088 does not cover it.

### 3.5 Item 5: directory sync

- `[RAN]` D1: `File::open(dir)?.sync_all()` returned `Ok(())` on an APFS directory. `[OBS]` it goes through the same `fcntl(F_FULLFSYNC)` path as files (3.2). That the directory entry survives power loss is `[UNV]` (no power-loss test).

### 3.6 Item 6: CRC-32

- `[RAN]` `crc32fast::hash(b"123456789")` = `0xCBF43926` (expected value), incremental `Hasher` over `"1234"` + `"56789"` the same, empty input `0x00000000`. This is CRC-32/ISO-HDLC, as SF-08 requires. Version tested: `crc32fast` 1.5.4, `MIT OR Apache-2.0`, `rust-version = 1.63` `[OBS ~/.cargo/registry/src/*/crc32fast-1.5.4/Cargo.toml]`, depends only on `cfg-if 1.0.5`.
- Dependency vetting (existence on crates.io, publish date against the 2026-10-28 pin rule, yank status, advisories, maintainer) was **not done**: no network. That remains OQ-17 part 2 `[UNV]`. 1.5.4 is simply the highest version in the local cache.

### 3.7 Item 7: SQLite `json_valid`

- `rusqlite` and `libsqlite3-sys` are not in the local cargo cache `[RAN ls ~/.cargo/registry/{cache,src}/*/ | grep]`, so no binding could be built offline. Fetching them (and compiling SQLite C source) was judged "not strictly needed" for the box. **The bundled SQLite release and its `json_valid` support are `[UNV]`.**
- Proxy only, and **not the bundled library** `[RAN /usr/bin/sqlite3, version 3.51.0]`: `json_valid('{"a":1}')` = 1, `json_valid('{a')` = 0, `json_valid(NULL)` = NULL; a table with `CHECK (json_valid(x))` accepts `'[1]'` and rejects `'{bad'` with `CHECK constraint failed: json_valid(x) (19)`. Note NULL passes a `CHECK` (NULL is not false), so the DDL needs `NOT NULL` or `json_valid(x) = 1` where NULL must be refused `[INF]`.
- Close it with a one-line test at the first `chess-store` `Cargo.toml` entry (`select json_valid('{}')` against the bundled build, plus `sqlite_version()`), as UT in the store, or by one network fetch in a follow-up spike.

## 4. Other observations

- `flock` is advisory and per open file description: a second `open` in the same process is refused (L1). The store must keep one `File` per held journal and not reopen it for reading (a read-only `open` is fine, it does not lock).
- Only `sync_all`/`sync_data` errors are visible; there is no way to ask std whether a volume supports `F_FULLFSYNC` without doing it. D-71's probe is therefore "call `sync_all` on a file we own and inspect `raw_os_error()`", which is what the spike does.
- Everything the spike did runs with `#![forbid(unsafe_code)]`. The `unsafe` in the primitives lives inside `std`, which is not our code.

## 5. Findings and recommendations (for the architect; no baselined document was edited)

| ID | Severity | Finding | Recommendation | Owner |
|---|---|---|---|---|
| FIND-OQ14-001 | Minor | Architecture 3.4 "Deferred (spike first)" (line 498) and SDD-chess-store.md:219 leave the `sys` module as an open question. The spike answers it: std only, no `sys` module, no `libc`/`rustix`. | Replace the deferred row by: "locking: `std::fs::File::try_lock`; full sync: `File::sync_all` (`F_FULLFSYNC` on Apple, std source reading plus timing, OQ-14 spike 2026-11-09); `chess-store` needs no `unsafe`". Close OQ-14. Keep `libc`/`rustix` out of the admission list. G1 text, system-architect | SA |
| FIND-OQ14-002 | Minor (decision for chess-me6) | **The workspace `unsafe_code` question can be settled as "forbid everywhere"** for the store, given the std route. Only if the architect picks atomic `RENAME_EXCL` (FIND-OQ14-004) is `rustix` added, and even then our crate stays free of `unsafe`. `libc` is never needed. Today the workspace `Cargo.toml` has no `unsafe_code` lint; each crate has `#![forbid(unsafe_code)]` in `lib.rs` `[OBS crates/*/src/lib.rs:2,9,10]`. | Record in chess-me6: set `unsafe_code = "forbid"` in `[workspace.lints.rust]` (or keep the per-crate attribute) for `chess-store` too. Decision is the architect's / Brian's | SA, Brian |
| FIND-OQ14-003 | Minor | D-71 refuses the directory on `ENOTSUP` or `ENOTTY`. The one failure observed, `F_FULLFSYNC` on a non-regular file, was `ENODEV` (19). Real unsupported volumes may return other errnos. | Word D-71/3.11.5 as "refuse on **any** error from the probe `sync_all`, reporting `errno`", with the two named codes as examples. Add a CT with a non-supporting target (the spike's `/dev/null` case is not a directory volume) if one is available | SA |
| FIND-OQ14-004 | Minor | D-83's "chosen at implementation: `renamex_np` with `RENAME_EXCL`, or `link` then `unlink` `[UNV]`" is now decided on evidence: `hard_link` + `remove_file` works in std and the existing crash rules cover it. Atomic `RENAME_EXCL` needs `rustix` (4 new crates) for no benefit the lock holder does not already give (the `flock` holder is the only writer of these names `[INF]`). | Choose `hard_link` + `remove_file`; drop the `[UNV]`. Add the "damaged file under two names" crash state to the crash-window list at SDD line 825 and to CT-ST-088. G1 text in the store SDD, though D-83 itself is a G2 decision of Brian's, so the SA should ask whether a wording change under it needs him | SA, Brian |
| FIND-OQ14-005 | Minor | `File::set_len` does not move the cursor; D-82 cleanup then append needs an explicit seek or positioned write. | Say so in the writer design (DD-ST-016). Both are in `std` | SW at design |
| FIND-OQ14-006 | Info | `sync_data` is `F_FULLFSYNC` on Apple, so it gives no speed-up over `sync_all` (4.00 ms vs 4.08 ms on this machine). | Do not spend design effort on `sync_data` for the journal | SW |
| FIND-OQ14-007 | Minor | Item 7 (bundled SQLite `json_valid`) is not closed. | Keep spike item S1 open until the first `chess-store` `Cargo.toml` entry; run `sqlite_version()` and a `json_valid` CHECK test on the bundled build then. Note NULL passes `CHECK (json_valid(x))` | SW, SA |
| FIND-OQ14-008 | Info | `crc32fast` 1.5.4 passes the check value; vetting (publish date vs the 2026-10-28 pin rule, yank, advisories) not done. | Include `crc32fast` in OQ-17 part 2 with the network on | SA |

## 6. Recommendation for architecture 3.4 (text for the architect to apply)

> **File primitives (OQ-14 closed, spike 2026-11-09).** The standard library provides all of them on macOS: advisory locks (`File::lock`, `try_lock`, `unlock`: `flock`), full sync (`File::sync_all` and `sync_data`: both `fcntl(F_FULLFSYNC)` on Apple targets), truncation (`File::set_len`: `ftruncate`), directory sync (`File::open(dir)?.sync_all()`) and the no-replace rename (`std::fs::hard_link` then `remove_file`). `chess-store` adds no `libc` or `rustix` dependency and no `unsafe`. The only crate for this area is `crc32fast` (CRC-32/ISO-HDLC, check value 0xCBF43926 verified). The bundled SQLite `json_valid` check remains to be run when `rusqlite` is first added.

## 7. Acceptance criteria

| Criterion | Met? | Evidence |
|---|---|---|
| Timeboxed 60 min | Met (about 40 min working time; not clocked exactly) | |
| Written answer on whether std suffices | Met | section 1, 2 |
| Throwaway code outside `crates/chess-store` | Met | `spikes/oq14/`; `crates/` and workspace `Cargo.toml` untouched `[RAN git status --short]` shows only `spikes/` new |
| Result recorded for architecture 3.4 | Drafted, not applied (baselined doc; SA applies) | section 6 |
| (1) flock incl. 2-process contention, release on death | Met | 3.1 |
| (2) `sync_all` is `F_FULLFSYNC`, source and experiment | Met (source `[OBS]`, timing `[RAN]`); `ENOTSUP`/`ENOTTY` not reproduced | 3.2 |
| (3) ftruncate + full sync | Met functionally; power-loss durability `[UNV]` | 3.3 |
| (4) no-replace rename | Met | 3.4 |
| (5) directory fsync | Met functionally; durability `[UNV]` | 3.5 |
| (6) CRC-32 check value | Met | 3.6 |
| (7) SQLite `json_valid` | **Not met** (`[UNV]`, proxy only) | 3.7 |

## 8. Commands run

| Command | Result |
|---|---|
| `rustc -V; cargo -V` | rustc 1.99.1 (58cf8f9c3 2026-10-13); cargo 1.99.1 |
| `cargo build --offline` in `spikes/oq14` | Finished, 0 warnings (package lint `warnings = "deny"`, `unsafe_code = "forbid"`) |
| `./target/debug/oq14-spike oq14-work` | exit 0, output as quoted in section 3 |
| `gcc c/cmp.c`, `c/rn.c` and runs | `fsync` 0.029 ms, `F_FULLFSYNC` 4.104 ms; `renamex_np` EXCL errno 17 then ok |
| `hdiutil` FAT32 8 MB image, attach, `probe`, detach, delete | `PROBE ok`; image ejected and removed |
| `oq14-spike syncpath /dev/null` | `raw=Some(19)` |
| `sqlite3` (system 3.51.0) `json_valid` and CHECK | as section 3.7 |
| `cargo fmt --check` | exit 0 (after `cargo fmt` once; the first check failed on formatting only) |
| `cargo clippy --offline -- -D warnings` | Finished, no warnings (run before and after adding `rustix`) |

## 9. Not run, not analyzed, assumptions

- **Not run:** `cargo test` (the spike has no `#[test]`s; the `oq14-spike` binary is its own check, so this is a throwaway demonstration, not unit-test evidence); power-loss durability of truncate, rename, directory sync; an unsupported volume (`ENOTSUP`/`ENOTTY`); network volumes; Intel Macs; `fork` inheritance; `SIGSTOP` of a lock holder; the bundled SQLite; crates.io lookups for `crc32fast`/`rustix` (publish date, yank, advisories); Mermaid (none written); `cargo deny`/SBOM (nothing added to the workspace).
- **Assumptions:** ASM-S01 the machine's APFS data volume is representative of the user's target Macs; ASM-S02 cached `crc32fast` 1.5.4 and `rustix` 1.1.6 are the genuine crates (cache, not verified against crates.io); ASM-S03 timings from one run are indicative only, not benchmarks.
- **Side effects:** a temporary FAT32 image was mounted and removed (scratchpad); spike work files are under `spikes/oq14/oq14-work` and are git-ignored with `target/`. `spikes/oq14/Cargo.lock` is untracked and unignored. `spikes/` is untracked; nothing committed.
