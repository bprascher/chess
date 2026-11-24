# Stockfish 19 fetch results (chess-a0r.3.3)

Status: **FILLED from the live run of 2026-11-09. Pins APPLIED to `fetch.sh` on 2026-11-09 (section 8): asset SHA-256 and tag commit both confirmed by Brian Prascher (relayed by coordinator, bead chess-a0r.3.34).** Evidence tags: `[RAN]` executed by `fetch.sh` or a command named here, `[OBS]` read from a file, `[INF]` inferred, `[UNV]` unverified. Values are copied from `third_party/stockfish/build/results.txt` (key in brackets) and the raw files in `third_party/stockfish/build/out/`.

Gate: G0 (script and note only; no crate, interface or edge touched). Closes the inputs of ASM-3, TBC-7, TBC-8, TBC-9, OQ-EN-001. Recording results in the requirements register (chess-a0r.3.5) and setting `PINNED_UCI_ELO_RANGE` (chess-a0r.3.4) are separate tasks and amendments.

## 1. Run record

| Field | Value |
|---|---|
| Date and time (UTC) | 2026-11-09T20:19:53Z `[RAN results.txt:header]` |
| Run by / authorisation | PM ran the script; Brian authorised the one-time live run on 2026-11-09 (per the task order; reference not recorded in the files read) |
| Command line | `third_party/stockfish/fetch.sh --trust-on-first-use` (per task order; not stored in results.txt) |
| Mode | `release-asset` `[RAN mode]` |
| Host | arm64 macOS 26.7.1 `[RAN host]` |
| Script exit status | 0 `[RAN exit_status]` |
| Script git revision | Repo HEAD is `07a42b57bfdacf9246c129ac716442889edfa58b` `[RAN git rev-parse HEAD]`, but `third_party/stockfish/` is **untracked** (`git status`: `?? third_party/stockfish/`), so `fetch.sh` as run is not in any commit. See FIND-A1 |

## 2. Source and checksums

| Field | Value |
|---|---|
| Repo and tag | official-stockfish/Stockfish, `sf_19`; release published 2026-09-05T08:33:17Z `[OBS release-sf_19.json:published_at]`; `target_commitish` is `master` (a branch name, not a commit) `[OBS]` |
| Archive URL fetched | https://github.com/official-stockfish/Stockfish/releases/download/sf_19/stockfish-macos-universal.tar.gz `[RAN archive_url]` |
| Asset names (`release_asset_names`) | stockfish-android-arm64-universal.tar.gz, stockfish-android-armv7-neon.tar.gz, stockfish-linux-arm64-universal.tar.gz, stockfish-linux-riscv64-universal.tar.gz, stockfish-linux-x86-64-universal.tar.gz, stockfish-macos-universal.tar.gz, stockfish-windows-arm64-universal.zip, stockfish-windows-x86-64-universal.zip `[RAN]` |
| Tag commit | `edb0d9db6731067ec50ce619ff372b463bc4dd5d  refs/tags/sf_19` `[RAN git ls-remote https://github.com/official-stockfish/Stockfish.git refs/tags/sf_19 'refs/tags/sf_19^{}', 2026-11-09]`. No `^{}` line was returned, so the tag is lightweight and this is the commit itself `[INF]` |
| SHA-256 of the archive | `a1f0e3bcc5a6927a11fe6fc8e54a779754645f3c2bae2cf13420fd1957adaa77` `[RAN archive_sha256]`; re-hashed from `build/download/stockfish-macos-universal.tar.gz` with `shasum -a 256`: same value `[RAN]` |
| SHA-256 reported by release API | `a1f0e3bc...adaa77`, identical `[RAN release_api_digest]` (also `sha256:a1f0e3bc...` in release-sf_19.json for the macOS asset `[OBS]`) |
| Checksum decision | TOFU accepted; equals the API digest. Two TLS answers from the same origin, no signature, so this is not independent `[RAN checksum]` |
| Human comparison against the release page | Done: confirmed by Brian Prascher, 2026-11-09 (relayed by coordinator) `[INF from the relay; not independently observed]` |
| SHA-256 of the extracted binary | `8eed61129d1493c5d1f2fd9323f0c54c47ac49319911fbde18c6b9c87e8b13c5` `[RAN binary_sha256]`; file size 105458632 bytes `[OBS ls -la]` |
| Pin diff reviewed into `fetch.sh` | Applied 2026-11-09 (section 8). Only the asset SHA-256 is enforced by the script; `PIN_COMMIT` is documentation |

## 3. TBC-7: architectures

| Field | Value |
|---|---|
| `lipo -info` | `Architectures in the fat file: .../stockfish-macos-universal are: x86_64 arm64` `[RAN lipo_info]` |
| `lipo -archs` | `x86_64 arm64` `[RAN lipo_archs]` |
| Both present? | Yes `[RAN TBC-7]` |
| Decision | Architecture 8.2 **step 1 applies: the release asset is accepted**; the source-build fallback (step 2) is not needed. Acceptance is the engineer's proposal; Brian/SA record it in chess-a0r.3.5 |
| x86_64 slice under Rosetta | Ran: `id name Stockfish 19` and `option name UCI_Elo type spin default 1320 min 1320 max 3190` `[RAN uci_x86_64_rosetta_*]`; output file `uci-x86_64.txt` has the same size as the native one (1173 bytes) `[OBS]` |
| Intel hardware run | Not run (no Intel Mac, SWE-001) |

## 4. TBC-8: UCI_Elo

| Field | Value |
|---|---|
| `id name` | `id name Stockfish 19` `[RAN]` |
| `UCI_Elo` option | `option name UCI_Elo type spin default 1320 min 1320 max 3190` `[RAN]` |
| `UCI_LimitStrength` option | `option name UCI_LimitStrength type check default false` `[RAN]` |
| min / max | 1320 / 3190 `[RAN TBC-8]` |
| setoption probe | `readyok` count 4 (expected 4); `bestmove` lines 1; raw file `third_party/stockfish/build/out/setoption-native.txt` `[RAN]` |
| Behaviour at min-1 (1319) and max+1 (3191) | The probe sent `UCI_LimitStrength true`, then `UCI_Elo` 1320, 3190, 1319, 3191, each followed by `isready`. stdout and stderr were captured together (`>out 2>&1`) `[OBS fetch.sh probe()]`. The raw output shows **no error line, no warning, no `info string` about the value and no connection drop** after any setoption; each `isready` got `readyok` and the later `go depth 1` produced `info depth 1 ... pv d2d4` and `bestmove e2e4` `[OBS]`. The output does **not** show whether 1319 or 3191 was clamped, ignored or stored. Conclusion: out-of-range values are **silently accepted or ignored; the effect is not observable from this probe** `[INF]`. A client must therefore range-check before sending. Establishing clamp vs ignore would need the source (`src/engine.cpp`/`ucioption.cpp` at the tag) or a probe that reads the value back; not done `[UNV]` |
| Proposed `PINNED_UCI_ELO_RANGE` (proposal; chess-a0r.3.4 sets it) | `1320..=3190` inclusive, pinned to Stockfish `sf_19`; recheck on any engine pin change. Note the minimum equals the default, so with `UCI_LimitStrength=false` the engine plays at full strength regardless of `UCI_Elo` `[OBS option lines]` |

## 5. TBC-9: licence

| Field | Value |
|---|---|
| COPYING | Found in the archive as `Copying.txt` (`copying_source: archive`) `[RAN]` |
| SHA-256 of the file | `3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986` `[RAN copying_sha256]` |
| Header and version line | `GNU GENERAL PUBLIC LICENSE` / `Version 3, 29 June 2007` `[RAN]` |
| GPL version | 3 (no "or later" statement was read from this file; the README of the tag says "GNU General Public License version 3 (GPL v3)" `[OBS README.md]`; per-file source headers not checked `[UNV]`) |
| Compatible with project GPL-3.0-or-later (D-10)? | Engineer view `[INF]`: a GPL-3.0 engine bundled and run as a separate process is compatible in direction with a GPL-3.0-or-later project; the combined work is distributed under GPLv3 terms. This is not a legal opinion. **Decision owner: Brian** |

## 6. NNUE networks

| Field | Value |
|---|---|
| `.nnue` files in archive | None; the archive listing has 111 members and `ls` of the extract has no `.nnue` `[RAN nnue_files_in_archive: none; OBS]` |
| Embedded in binary | Yes `[INF]`: the binary is 105458632 bytes, `strings` finds `nn-1a298aa575a0.nnue`, and the probe printed `NNUE evaluation using nn-1a298aa575a0.nnue (109MiB, (86896, 1024, 32, 32, 1))` with `Network replica 1: Shared memory.` `[OBS setoption-native.txt]`. That run was not network-isolated, so this is not the architecture 8.1 offline test |
| Net file names (source at tag) | One net: `#define EvalFileDefaultName "nn-1a298aa575a0.nnue"` in `src/evaluate.h` of the extracted source tree `[OBS]`. The extracted source is the release archive's copy, not a fresh checkout of the tag `[INF]`; the UCI option `EvalFile` defaults to the same name `[OBS]`. Only a single default net was found; a second (small) net was not found by this method `[UNV]` |
| Net licence and source | **No licence for the net file itself was found** in the archive. `README.md` (Acknowledgements) states the nets are trained on Leela Chess Zero data "made available under the Open Database License (ODbL)" `[OBS]`. Whether the ODbL imposes terms on the net, and the net's own licence, are `[UNV]`. Needs Brian/legal read; candidate source is the net's download page on the Stockfish testing site, not fetched |

## 7. Not run / not analyzed / follow-ups

| Item | Status |
|---|---|
| Offline start with networking disabled (architecture 8.1) | Not run. Embedding is indicated (section 6) but no network-isolated start was done |
| Source build fallback (`--source`) | Untested code path; not needed if the asset is accepted |
| Intel hardware | Not run |
| Clamp/ignore semantics of out-of-range `UCI_Elo` | Not determined (section 4) |
| Per-file GPL headers, net licence, ODbL implications | Not analyzed |
| `fetch.sh` itself under version control | It is untracked |
| Requirements register entries (TBC-7/8/9), architecture ASM-3, `PINNED_UCI_ELO_RANGE` | Owners: chess-a0r.3.4, chess-a0r.3.5 |
| Risk chess-a0r.11 status update | Not updated by the engineer; PM/Brian. Inputs now exist: universal asset confirmed, range 1320..3190, GPL 3 |

## 8. Pin section (APPLIED 2026-11-09)

Brian Prascher confirmed on 2026-11-09 (relayed by coordinator, bead chess-a0r.3.34) both the asset SHA-256 and the `sf_19` tag commit. Both are now in `third_party/stockfish/fetch.sh`:

| Pin | Value | Status | Enforced by the script? |
|---|---|---|---|
| `PIN_ASSET_SHA256` | `a1f0e3bcc5a6927a11fe6fc8e54a779754645f3c2bae2cf13420fd1957adaa77` | Confirmed by Brian Prascher against the release page, 2026-11-09 (relayed by coordinator) | **Yes.** A mismatch exits 2 and nothing is extracted |
| `PIN_COMMIT` | `edb0d9db6731067ec50ce619ff372b463bc4dd5d` (lightweight tag `sf_19`) | Confirmed by Brian Prascher, 2026-11-09 (relayed by coordinator) | **No.** Documentation of the tag commit only. The script makes no network check of it and compares nothing against it; the only uses are the `--dry-run` print and the header comment `[OBS grep PIN_COMMIT fetch.sh]` |
| `PIN_SOURCE_SHA256` | empty `[UNV]` | Not set: the `--source` fallback is unused and untested | Yes for `--source` only: while empty, `--source` refuses (exit 4) |

**Only the asset SHA-256 is enforced.** No network check of `PIN_COMMIT` was added (out of scope for this G0 change). If enforcement is wanted later, it needs a separate decision: e.g. `git ls-remote` against the tag at run time, which would add a network dependency to the fetch.

Script comment changes made with the pin: the stale header text ("pins are EMPTY in this commit") was rewritten to state which pins are set and which are enforced; `PIN_SOURCE_SHA256` is described as the pin for the tag source archive used only by `--source`; `--dry-run` and the `tag_commit_cmd` result line now say `PIN_COMMIT` is documentation and not enforced. This closes FIND-003 of the prep report.

Offline verification of the edited script, 2026-11-09 `[RAN]`: `bash -n fetch.sh` passed; `fetch.sh --dry-run` printed the pins and exited 0; `fetch.sh` (no flags) used the cached archive with no download, printed `checksum: matches pin`, and exited 0 with the same binary SHA-256 (`8eed6112...13c5`), lipo, UCI_Elo and GPL results as above. That run rewrote `third_party/stockfish/build/results.txt` (untracked); its values are unchanged except the timestamp. A mismatch path (exit 2) was not re-tested.

## 9. Housekeeping checks

- `third_party/stockfish/.gitignore` contains `/build/` `[OBS]`. `git check-ignore -v` matches `third_party/stockfish/build`, `build/results.txt` and `build/extract/stockfish/stockfish-macos-universal` `[RAN]`. The binary is therefore ignored and not tracked; `git ls-files third_party` lists only `third_party/eco/*` `[RAN]`.
- The downloaded archive and the 105 MB binary live only under `build/`. The archive/binary are not in the repo history.
