# Step-2 preparation: evidence note (P3-PREP)

Software engineer, Engineer stance, 2026-11-09, baseline B4. Beads: chess-a0r.3.27, chess-85m, chess-a0r.3.3, first dependency entries (F5 A). Gate record: gates/GATE-104-step2-prep.md. Tags: `[OBS]` read, `[RAN]` executed, `[INF]` inferred, `[UNV]` unverified. Nothing was committed or closed. Rulings are as relayed by the coordinator (no verbatim quotation).

## 1. ci/check.sh (chess-a0r.3.27), G0

- Reproduced: `/bin/bash -c 'set -u; a=(); echo "${a[@]}"'` prints `a[@]: unbound variable` on bash 3.2.57 `[RAN]`.
- Fix: `cargo deny ${deny_args[@]+"${deny_args[@]}"} check`. Idiom tested in isolation, empty and non-empty array, `set -euo pipefail`, bash 3.2 `[RAN]`. The full script run uses `CHESS_CI_OFFLINE=1`, so the array was non-empty there; the empty case rests on the isolated test.
- npm audit step: `npm audit --omit=dev --audit-level=low`, run only if `ui/package-lock.json` exists (it does `[OBS]`); otherwise a skip message. `bash -n` OK `[RAN]`. The online npm branch was not run (no network run authorised) `[UNV]`.

## 2. deny.toml (F3 B, F4 A), G2

Changes `[OBS deny.toml]`: `[graph] targets` = aarch64-apple-darwin and x86_64-apple-darwin; one `ignore` for RUSTSEC-2026-0441 with the reason as comments (VULN-004 revisit triggers written in the file); `Zlib` added; five `[[licenses.exceptions]]`-style entries `{ allow = ["MPL-2.0"], crate = ... }` for cssparser, cssparser-macros, dtoa-short, selectors, option-ext; `unused-allowed-license = "allow"` in `[licenses]`.

- Before: `advisories ok, bans ok, licenses ok, sources ok` with one warning (`Unlicense` not encountered) `[RAN]`.
- After (`cargo deny --offline check`): `advisories ok, bans ok, licenses ok, sources ok` `[RAN]`. Licence-allowance warnings are gone. Six warnings remain by design: five `license-exception-not-encountered` and one `advisory-not-detected` (RUSTSEC-2026-0441). They clear when `desktop/src-tauri` and the CLI dev-dependency `expectrl` enter the lock `[INF]`. The unused-licence lint was silenced because Unlicense, Zlib, BSD-3-Clause and Unicode-3.0 are expected to be encountered once crates arrive; the lookup doc named BSD-3-Clause and Unicode-3.0 but the warning actually printed was for Unlicense `[RAN]`.
- Against the probe lock with the new `deny.toml` (licenses, advisories, sources): `advisories ok, licenses ok, sources ok`, so the ignore works with `expectrl` present and `Zlib` is accepted for `zip` `[RAN]`. The MPL exceptions were not exercised: the probe has no Tauri `[RAN]`.
- `node ci/check-crate-edges.mjs`: `crate-edge check OK: 8 crates, 11 workspace edges, all allowed` `[RAN]`.
- The key name `unused-allowed-license` was accepted by cargo-deny 0.20.2 (no parse error) `[RAN]`. Whether `license-exception-not-encountered` has a lint switch was not checked `[UNV]`.

## 3. Stockfish pin (chess-a0r.3.3), G0

- `PIN_ASSET_SHA256="a1f0e3bc...adaa77"`, comment: confirmed by Brian against the release page, 2026-11-09 (relayed).
- `PIN_COMMIT` left **empty**, with the comment `# unconfirmed by Brian; from git ls-remote 2026-11-09: edb0d9db6731067ec50ce619ff372b463bc4dd5d` above it. Reason: the script never checks it, and a filled value could read as confirmed.
- `bash -n fetch.sh` OK; `--dry-run` prints the pin and "computed SHA-256 must equal the pin, else exit 2" `[RAN]`. The dry-run line "tag commit pin" shows EMPTY, as intended.
- Offline verification: `shasum -a 256 build/download/stockfish-macos-universal.tar.gz` = pin `[RAN]`. Then `fetch.sh` (no flags) on the cached archive: "Using cached", `checksum: matches pin`, 111 members, lipo `x86_64 arm64`, TBC-8 UCI_Elo min=1320 max=3190, TBC-9 GPL line read, `exit_status: 0` `[RAN]`. No download happened (cached; COPYING was in the archive so the fallback URL was not used `[OBS results.txt copying_source: archive]`). The script ran the cached binary locally for the UCI probe, as in the earlier authorised run. `build/results.txt` was overwritten (untracked, ignored); the previous copy is in the session scratchpad.
- Header comment of `fetch.sh` still says the pins are EMPTY in this commit (stale, left alone; see FIND-003).

## 4. CI workflow (chess-85m option C), G0

- `actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0`, `actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4.4.0`. Re-verified with `git ls-remote https://github.com/actions/<name> 'refs/tags/v4*'`: both SHAs are the objects of `refs/tags/v4.4.0` (and `v4`); no peeled line shown, so lightweight tags, SHA is the commit `[RAN]`.
- The last step is `ci/check.sh` (already so), now commented as the gate of record. YAML parsed with Ruby (4 steps `[RAN]`). Not run hosted; `macos-latest` unchanged (the decision package suggests a named image; not checked `[UNV]`).

## 5. First dependency entries (F5 A), G2

- Root `Cargo.toml` `[workspace.dependencies]`: 29 third-party entries (24 runtime, 5 dev) with exact `=` pins, default-features and feature sets per lookup 5A: tokio (no features at root; per-crate sets listed in a comment, DL2-5), rusqlite (`bundled`, no defaults, DL2 F9), crc32fast, uuid (v5, serde), serde (derive), serde_json, thiserror, tracing, tracing-subscriber (fmt, std), tokio-util, futures-util (std), zip (deflate-flate2-zlib-rs), sha2, base64 (std), getrandom, unicode-segmentation, unicode-normalization, ts-rs, toml, clap (derive), ratatui, crossterm (event-stream), axum (http1, tokio, ws), rust-embed; dev: tempfile, proptest, insta, expectrl, vt100. Not added: tauri, tauri-build, libc, rustix, tokio-stream, tracing-appender, npm packages. No crate manifest edited; `crates/*/src` untouched.
- An unused workspace dependency never reaches `Cargo.lock`, so resolution had to be tested with a probe: scratch workspace in the session scratchpad (outside the repo) with five member crates inheriting the root entries with the DL2-5 per-crate tokio features. `cargo generate-lockfile` (crates.io index and downloads, for resolution only) gave 327 lock packages; `cargo check --workspace --all-targets` `Finished` OK, including bundled SQLite compile `[RAN]`. Transitive crates resolved to current versions, not the 2026-10-28 date `[INF]`.
- `cargo metadata --offline` on the real workspace OK; `node ci/check-crate-edges.mjs` OK (see section 2) `[RAN]`.
- `cargo tree -p p-store -e normal,build --prefix none --target aarch64-apple-darwin | sort -u | wc -l` = 39 lines (includes the probe) `[RAN]`; lookup said 36 for the store; the difference is not explained `[UNV]`. libsqlite3-sys 0.39.1 with feature `bundled` only through rusqlite `[RAN cargo tree -i]`.
- Gaps: the real repository `Cargo.lock` is unchanged (no crate uses the entries yet). The first crate to opt in must run `ci/check.sh` including `cargo deny`; Cargo exact-pin dedupe with the step-1 pins in domain and rules was not exercised because the probe does not contain them `[INF: same values]`.

### S1 spike (closes OQ-14 item 7), G0

`spikes/sqlite-json` (own `[workspace]`, rusqlite =0.41.1 bundled, built with a scratchpad `CARGO_TARGET_DIR`), `cargo run` `[RAN]`:

| Observation | Result |
|---|---|
| `sqlite_version()` / version number | 3.54.1 / 3054001 (matches F10 and libsqlite3-sys 0.39.1) |
| `SELECT json_valid('{bad')` | `Some(0)` |
| `SELECT json_valid('{"a":1}')` / `json_valid('')` / `json_valid(NULL)` | `Some(1)` / `Some(0)` / `None` |
| `CHECK (json_valid(x))`, insert valid | `Ok(1)` |
| insert `{bad` | `ConstraintViolation`, extended code 275, "CHECK constraint failed: json_valid(x)" |
| insert NULL into the same column | `Ok(1)`: CHECK passes on NULL, so the column needs `NOT NULL` if NULL is not allowed |
| compile options shown (filtered for JSON, FOREIGN, LOAD_EXT) | `DEFAULT_FOREIGN_KEYS`, `ENABLE_LOAD_EXTENSION`; no JSON option listed. `[INF]` JSON is built in to SQLite 3.54 without a flag; F10's "SQLITE_ENABLE_JSON1 is set" is therefore not visible in `PRAGMA compile_options` |

Not tested: `json_valid` on BLOB input, JSON5 acceptance, file-backed database.

## 6. Final gate run

`CHESS_CI_OFFLINE=1 ci/check.sh` exit 0; stages: fmt, build --locked, clippy -D warnings, test (all `test result: ok`, no failures), crate-edge OK, ECO checksums, `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`, six expected warnings), `npm audit: skipped (CHESS_CI_OFFLINE=1)`, final line `All gates passed.` `[RAN]`. Output was saved to the session scratchpad.

## 7. Findings

| ID | Sev | Finding |
|---|---|---|
| FIND-001 | Medium | The online path of `ci/check.sh` (advisory DB fetch, `npm audit --omit=dev` against `ui/package-lock.json`) has not run since the change `[UNV]`. First hosted or networked run may surface findings (the lookup recorded 0 production findings 2026-11-09). |
| FIND-002 | Low | Six cargo-deny warnings remain until Tauri and `expectrl` are in the lock; a future reader may take them as defects. Re-check the MPL list against the real lock at the desktop entry (ASM-DL2-2). |
| FIND-003 | Low | `fetch.sh` header comment still says pins are EMPTY; `PIN_SOURCE_SHA256` still empty (source path unused). Doc fix is G0, left alone to keep the diff to the pin. |
| FIND-004 | Low | `PIN_COMMIT` unconfirmed by Brian and unchecked by the script; the asset hash is the only check. |
| FIND-005 | Low | `workspace.dependencies` entries are inert until used; a typo in a feature name would show only at the first crate that opts in (the probe covers the current names). |
| FIND-006 | Info | Lookup F10 says `SQLITE_ENABLE_JSON1` is set; `PRAGMA compile_options` does not show it, but `json_valid` works, so the claim needs rewording, not action. |
| FIND-007 | Info | The `ignore` for RUSTSEC-2026-0441 has no enforced expiry; revisit triggers are comments and VULN-004 (record to be written by whoever owns `security/triage.md`; no such file was created here). |

## 8. Not run / not analysed

Hosted CI; online `ci/check.sh`; `npm audit`; the five `MPL-2.0` exceptions against a Tauri lock; Intel anything; SBOM; a licence-text review of bundled SQLite; the `cargo tree` count difference (39 vs 36); the Decision Log and RELEASE_NOTES entries (systems engineer's); VULN-004 record file.

## 9. Assumptions

ASM-P3-1: the rulings relayed by the coordinator are accurate (no verbatim text). ASM-P3-2: `unused-allowed-license = "allow"` is the intended reading of "clear the unused-allow warning" (alternative: delete the unused entries, which would break when the crates arrive).
