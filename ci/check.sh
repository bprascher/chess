#!/usr/bin/env bash
# Local and CI quality gates for the Rust workspace (test-strategy "Quality gates").
# Every warning is an error. Run from anywhere; it cds to the repository root.
#
#   ci/check.sh            full run (cargo-deny fetches the advisory DB, npm audit uses the registry)
#   CHESS_CI_OFFLINE=1 ci/check.sh
#                          no network: cargo-deny uses its cached advisory DB, npm audit is skipped
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTFLAGS="-D warnings"
export RUSTDOCFLAGS="-D warnings"

deny_args=()
if [[ "${CHESS_CI_OFFLINE:-0}" == "1" ]]; then deny_args+=(--offline); fi

step() { printf '\n== %s\n' "$*"; }

step "cargo fmt --check"
cargo fmt --all -- --check
step "cargo build --locked"
cargo build --workspace --all-targets --locked
step "cargo clippy -D warnings"
cargo clippy --workspace --all-targets --locked -- -D warnings
step "cargo test"
cargo test --workspace --locked
step "crate-edge check (architecture 3.2)"
node ci/check-crate-edges.mjs
step "cargo deny check"
cargo deny "${deny_args[@]}" check
if [[ "${CHESS_CI_OFFLINE:-0}" == "1" ]]; then
  step "npm audit: skipped (CHESS_CI_OFFLINE=1)"
else
  step "npm audit (ui/)"
  (cd ui && npm audit --audit-level=low)
fi
printf '\nAll gates passed.\n'
