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

tmp_output="$(mktemp)"
cleanup() {
  rm -f "$tmp_output"
}
trap cleanup EXIT

base_cmd="cargo run -q -p laminar-cli -- validate test-vectors/valid-simple.csv --network mainnet"

if command -v script >/dev/null 2>&1; then
  if ! script -qec "$base_cmd" /dev/null >"$tmp_output" 2>&1; then
    cargo run -q -p laminar-cli -- --interactive validate test-vectors/valid-simple.csv --network mainnet >"$tmp_output"
  fi
else
  cargo run -q -p laminar-cli -- --interactive validate test-vectors/valid-simple.csv --network mainnet >"$tmp_output"
fi

if grep -q '"mode"[[:space:]]*:[[:space:]]*"agent"' "$tmp_output"; then
  echo "expected operator mode, but agent JSON was emitted"
  exit 1
fi

if ! grep -qi "validation completed" "$tmp_output"; then
  echo "operator output did not contain completion text"
  exit 1
fi

echo "tty-detection.sh: operator mode detected"
