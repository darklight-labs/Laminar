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

echo "[1/8] rustfmt check"
cargo fmt --all --check

echo "[2/8] clippy (deny warnings)"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "[3/8] full test suite"
cargo test --all --all-targets

echo "[4/8] coverage gate (laminar-core >= 80%)"
if command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "coverage tool: cargo-tarpaulin"
  coverage_dir="target/tarpaulin-release-check"
  rm -rf target/tarpaulin "$coverage_dir"
  cargo tarpaulin --manifest-path crates/laminar-core/Cargo.toml --out Xml --output-dir "$coverage_dir" --engine llvm --timeout 300

  coverage_xml="$coverage_dir/cobertura.xml"
  if [ ! -f "$coverage_xml" ]; then
    echo "coverage report missing: $coverage_xml"
    exit 1
  fi

  coverage_summary="$(
    awk '
      /<class / {
        in_class = 0
        if (index($0, "filename=\"crates/laminar-core/") > 0) in_class = 1
        if (index($0, "filename=\"crates\\\\laminar-core\\\\") > 0) in_class = 1
      }
      in_class && /<line / {
        total += 1
        if (match($0, /hits="([0-9]+)"/, m) && (m[1] + 0) > 0) {
          covered += 1
        }
      }
      END {
        if (total == 0) {
          print "ERR no laminar-core coverage lines found in report"
          exit 2
        }
        pct = (covered / total) * 100.0
        printf "OK %.2f %d %d\n", pct, covered, total
      }
    ' "$coverage_xml"
  )"
elif command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "coverage tool: cargo-llvm-cov"
  coverage_dir="target/llvm-cov-release-check"
  coverage_lcov="$coverage_dir/lcov.info"
  rm -rf "$coverage_dir"
  mkdir -p "$coverage_dir"
  cargo llvm-cov --package laminar-core --all-features --tests --lcov --output-path "$coverage_lcov"

  if [ ! -f "$coverage_lcov" ]; then
    echo "coverage report missing: $coverage_lcov"
    exit 1
  fi

  coverage_summary="$(
    awk '
      /^SF:/ {
        in_file = 0
        file = substr($0, 4)
        if (index(file, "/crates/laminar-core/") > 0) in_file = 1
        if (index(file, "\\crates\\laminar-core\\") > 0) in_file = 1
        if (index(file, "crates/laminar-core/") == 1) in_file = 1
        if (index(file, "crates\\laminar-core\\") == 1) in_file = 1
        next
      }
      in_file && /^DA:/ {
        split(substr($0, 4), parts, ",")
        total += 1
        if ((parts[2] + 0) > 0) {
          covered += 1
        }
      }
      END {
        if (total == 0) {
          print "ERR no laminar-core coverage lines found in report"
          exit 2
        }
        pct = (covered / total) * 100.0
        printf "OK %.2f %d %d\n", pct, covered, total
      }
    ' "$coverage_lcov"
  )"
else
  echo "coverage tool not found. Install one of:"
  echo "  cargo install cargo-tarpaulin --locked"
  echo "  cargo install cargo-llvm-cov --locked"
  exit 1
fi

case "$coverage_summary" in
  "ERR "*)
    echo "${coverage_summary#ERR }"
    exit 1
    ;;
  "OK "*)
    ;;
  *)
    echo "unexpected coverage parser output: $coverage_summary"
    exit 1
    ;;
esac

read -r _ coverage_pct coverage_covered coverage_total <<<"$coverage_summary"
echo "laminar-core line coverage: ${coverage_pct}% (${coverage_covered}/${coverage_total})"
awk -v pct="$coverage_pct" 'BEGIN { exit !(pct < 80.0) }' && {
  echo "coverage gate failed: expected >= 80.0%"
  exit 1
}

echo "[5/8] audit (deny warnings)"
if ! cargo audit --version >/dev/null 2>&1; then
  echo "cargo-audit is required. Install with: cargo install cargo-audit --locked"
  exit 1
fi
cargo audit --deny warnings

echo "[6/8] npm audit (high/critical)"
if [ -f "desktop/package.json" ]; then
  if ! command -v npm >/dev/null 2>&1; then
    echo "npm not found on PATH"
    exit 1
  fi
  (
    cd desktop
    npm audit --audit-level=high
  )
else
  echo "desktop/package.json not found; skipping npm audit"
fi

echo "[7/8] dual-mode checks"
bash scripts/tty-detection.sh
bash scripts/pipe-detection.sh
bash scripts/json-output.sh

echo "[8/8] deterministic agent JSON"
tmp_one="$(mktemp)"
tmp_two="$(mktemp)"
cleanup() {
  rm -f "$tmp_one" "$tmp_two"
}
trap cleanup EXIT

cargo run -q -p laminar-cli -- --output json validate test-vectors/valid-simple.csv --network mainnet >"$tmp_one"
cargo run -q -p laminar-cli -- --output json validate test-vectors/valid-simple.csv --network mainnet >"$tmp_two"

if ! cmp -s "$tmp_one" "$tmp_two"; then
  echo "determinism check failed: repeated JSON outputs differ"
  exit 1
fi

echo "release-check.sh: all gates passed"
