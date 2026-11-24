# Step 2 CI evidence (EF-03 change, 2026-11-11, baseline B6)

Evidence tags: [RAN] executed locally on macOS (Darwin 25.6.0); [UNV] not run.

| Case | Command | Result |
|---|---|---|
| Normal, offline | `CHESS_CI_OFFLINE=1 ci/check.sh` | [RAN] exit 0. New stage "cargo test chess-store crash/fault suite (--features test-support)" ran; 269 tests passed (sum of `test result` lines), 0 failed. Ends `All gates passed.` |
| Binary missing | `CHESS_STOCKFISH=/nonexistent/sf ci/check.sh` | [RAN] exit 1 at the first step with the fetch.sh / opt-out message. |
| Opt-out | `CHESS_ALLOW_STOCKFISH_SKIP=1 CHESS_STOCKFISH=/nonexistent/sf CHESS_CI_OFFLINE=1 ci/check.sh` | [RAN] exit 0, 0 FAILED; warning at the start and again at the end; last line `All gates passed (Stockfish integration SKIPPED by opt-out).` |
| Syntax | `bash -n ci/check.sh` | [RAN] ok |

Not run: hosted workflow; bash 3.2 itself (reviewed by inspection only: no `pgrep`, empty-array expansion guarded); npm audit (offline mode skips it).
