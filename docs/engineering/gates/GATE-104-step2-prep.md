# GATE-104: Step-2 preparation (CI fixes, deny.toml policy, Stockfish pin, workflow SHA pins, first dependency entries)

Records the classification of the P3-PREP task. Evidence: docs/engineering/step2-prep-evidence.md. Precedent: GATE-101 (scaffolding, CI, deny.toml). This file adds no decision.

| Change | Tier | Approver | Status |
|---|---|---|---|
| `ci/check.sh`: empty-array fix, `npm audit --omit=dev` guard (chess-a0r.3.27) | G0 (bug fix in a script; the `--omit=dev` wording is DL2-9, already ruled) | none | RECORDED |
| `deny.toml`: Zlib, five MPL-2.0 exceptions, Apple targets, one `ignore` (RUSTSEC-2026-0441), unused-licence lint (F3 B, F4 A) | G2 (licence and advisory policy) | Brian Prascher | Ruled 2026-11-09 as relayed by the coordinator (no verbatim quotation); source dependency-lookup-step2.md section 5B |
| `third_party/stockfish/fetch.sh`: `PIN_ASSET_SHA256` (chess-a0r.3.3) | G0 (pin a value inside one script; build-time only) | none; the value was confirmed by Brian against the release page (relayed) | RECORDED |
| `.github/workflows/ci.yml`: SHA-pinned actions, `ci/check.sh` as gate of record (chess-85m option C) | G0 (no code or design effect); the CI product decision itself is Brian's, 2026-11-06 | none | RECORDED |
| Root `Cargo.toml` `[workspace.dependencies]`: 29 third-party entries (F5 A) | G2 (new dependency edges) | Brian Prascher | Admission ruled 2026-11-09 as relayed (DL2-1..15, F5 A). No crate manifest edited, so no edge is active yet |
| `spikes/sqlite-json` (S1) | G0 (throwaway, outside `crates/`) | none | RECORDED |

Not an approval. Every crate that opts in later is checked against ci/check-crate-edges.mjs at its own change.
