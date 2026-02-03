#!/usr/bin/env bash
# Demo script for running human and agent modes end-to-end.
set -euo pipefail

echo "== Build (release) =="
cargo build --release

echo
echo "== Human Mode (TTY) =="
echo "Running: cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force"
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force

echo
echo "== Agent Mode (forced json) =="
echo "Running: cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force"
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force
echo

echo "== Agent Mode (auto via pipe) =="
if command -v jq >/dev/null 2>&1; then
  echo "Running: ... | jq '.recipient_count'"
  cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force 2>/dev/null | jq '.recipient_count'
else
  echo "jq not found; printing raw JSON:"
  cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force 2>/dev/null
fi

echo
echo "== Invalid Batch (should fail-fast) =="
echo "Running: cargo run --release -p laminar-cli -- --input ./demo/invalid.csv --output json --force 2>&1"
set +e
cargo run --release -p laminar-cli -- --input ./demo/invalid.csv --output json --force 2>&1
status=$?
set -e
echo "Exit code: $status"
echo
