#!/usr/bin/env bash
# Agent-mode regression checks for JSON output and determinism.
set -euo pipefail

echo "== Agent-mode confirmation test (should exit code 2) =="
set +e
out=$(cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json 2>&1)
status=$?
set -e
echo "$out"
echo "Exit code: $status"
if [ "$status" -ne 2 ]; then
  echo "FAIL: expected exit code 2"
  exit 1
fi

echo
echo "== Agent-mode strict JSON stdout test =="
if command -v jq >/dev/null 2>&1; then
  json=$(cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force 2>/dev/null)
  echo "$json" | jq -e '.schema_version and .total_zat and .recipients' >/dev/null
  echo "PASS: JSON schema fields present"
else
  echo "jq not found; skipping schema validation"
fi

echo
echo "== Agent-mode deterministic output test =="
a=$(cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force 2>/dev/null)
b=$(cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force 2>/dev/null)
if [ "$a" != "$b" ]; then
  echo "FAIL: output is not deterministic"
  exit 1
fi
echo "PASS: deterministic output"
