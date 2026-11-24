#!/usr/bin/env bash
# Local and CI quality gates for the Rust workspace (test-strategy "Quality gates").
# Every warning is an error. Run from anywhere; it cds to the repository root.
#
#   ci/check.sh            full run (cargo-deny fetches the advisory DB, npm audit uses the registry)
#   CHESS_CI_OFFLINE=1 ci/check.sh
#                          no network: cargo-deny uses its cached advisory DB, npm audit is skipped
#
# The real-Stockfish tests (EI-EN-001 .. 008, FF-7) must RUN, not skip. A missing binary is a gate
# failure here: the script exports CHESS_REQUIRE_STOCKFISH=1 (the tests then panic instead of
# skipping) and stops early with a clear message. Obtain the binary with
# third_party/stockfish/fetch.sh (needs the network once; the pinned SHA-256 is enforced), or
# point CHESS_STOCKFISH at an existing build. A machine that really has no Stockfish may opt out
# explicitly with CHESS_ALLOW_STOCKFISH_SKIP=1; the skip is then printed, and the gate result is
# not a full pass of FF-7.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTFLAGS="-D warnings"
export RUSTDOCFLAGS="-D warnings"

deny_args=()
if [[ "${CHESS_CI_OFFLINE:-0}" == "1" ]]; then deny_args+=(--offline); fi

step() { printf '\n== %s\n' "$*"; }

step "Stockfish binary (FF-7 / EI-EN-002)"
stockfish_bin="${CHESS_STOCKFISH:-third_party/stockfish/build/extract/stockfish/stockfish-macos-universal}"
if [[ "${CHESS_ALLOW_STOCKFISH_SKIP:-0}" == "1" ]]; then
  unset CHESS_REQUIRE_STOCKFISH
  echo "WARNING: CHESS_ALLOW_STOCKFISH_SKIP=1: the real-Stockfish tests may skip. This is NOT a full pass of FF-7."
elif [[ -f "$stockfish_bin" ]]; then
  export CHESS_REQUIRE_STOCKFISH=1
  echo "found $stockfish_bin; CHESS_REQUIRE_STOCKFISH=1"
else
  echo "ERROR: the Stockfish binary is missing: $stockfish_bin" >&2
  echo "  Run third_party/stockfish/fetch.sh (needs the network once; the pinned SHA-256 is enforced)," >&2
  echo "  or set CHESS_STOCKFISH to an existing binary," >&2
  echo "  or, on a machine without Stockfish, set CHESS_ALLOW_STOCKFISH_SKIP=1 to skip knowingly." >&2
  exit 1
fi

step "cargo fmt --check"
cargo fmt --all -- --check
step "cargo build --locked"
cargo build --workspace --all-targets --locked
step "cargo clippy -D warnings"
cargo clippy --workspace --all-targets --locked -- -D warnings
step "cargo test"
cargo test --workspace --locked
step "cargo test chess-store crash/fault suite (--features test-support)"
cargo test -p chess-store --features test-support --locked
step "crate-edge check (architecture 3.2)"
node ci/check-crate-edges.mjs
step "ECO snapshot checksums (CI-RUL-001)"
node ci/check-eco.mjs
step "cargo deny check"
cargo deny ${deny_args[@]+"${deny_args[@]}"} check
if [[ "${CHESS_CI_OFFLINE:-0}" == "1" ]]; then
  step "npm audit: skipped (CHESS_CI_OFFLINE=1)"
else
  if [[ -f ui/package-lock.json ]]; then
    step "npm audit (ui/)"
    (cd ui && npm audit --omit=dev --audit-level=low)
  else
    step "npm audit: skipped (ui/package-lock.json does not exist)"
  fi
fi
if [[ "${CHESS_ALLOW_STOCKFISH_SKIP:-0}" == "1" ]]; then
  printf '\nWARNING: CHESS_ALLOW_STOCKFISH_SKIP=1 was used: the real-Stockfish tests (FF-7, EI-EN-001..008) may have SKIPPED. This is NOT a full pass.\n'
  printf 'All gates passed (Stockfish integration SKIPPED by opt-out).\n'
else
  printf '\nAll gates passed.\n'
fi
