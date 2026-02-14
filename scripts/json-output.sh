#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  if [ -n "${CARGO_HOME:-}" ] && [ -d "${CARGO_HOME}/bin" ]; then
    export PATH="${CARGO_HOME}/bin:${PATH}"
  fi
  if [ -n "${HOME:-}" ] && [ -d "${HOME}/.cargo/bin" ]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
  fi
  if [ -n "${USERPROFILE:-}" ] && [ -d "${USERPROFILE}/.cargo/bin" ]; then
    export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found on PATH"
  exit 1
fi

PYTHON_BIN="python3"
if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  PYTHON_BIN="python"
fi

tmp_output="$(mktemp)"
cleanup() {
  rm -f "$tmp_output"
}
trap cleanup EXIT

cargo run -q -p laminar-cli -- --output json validate test-vectors/valid-simple.csv --network mainnet >"$tmp_output"

if command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  "${PYTHON_BIN}" - "$tmp_output" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))

if payload.get("mode") != "agent":
    raise SystemExit("expected mode=agent with --output json")
if payload.get("operation") != "validate":
    raise SystemExit("expected validate operation in --output json mode")
if payload.get("success") is not True:
    raise SystemExit("expected success=true in --output json mode")
PY
else
  if ! grep -q '"mode"[[:space:]]*:[[:space:]]*"agent"' "$tmp_output"; then
    echo "expected mode=agent with --output json"
    exit 1
  fi
  if ! grep -q '"operation"[[:space:]]*:[[:space:]]*"validate"' "$tmp_output"; then
    echo "expected operation=validate with --output json"
    exit 1
  fi
  if ! grep -q '"success"[[:space:]]*:[[:space:]]*true' "$tmp_output"; then
    echo "expected success=true with --output json"
    exit 1
  fi
fi

echo "json-output.sh: --output json forced agent mode"
