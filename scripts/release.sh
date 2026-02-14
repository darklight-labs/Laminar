#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/release.sh <version> [--dry-run]

Example:
  scripts/release.sh 0.2.0
  scripts/release.sh 0.2.0 --dry-run
USAGE
}

if [ "${1:-}" = "" ] || [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
  exit 1
fi

VERSION="$1"
TAG="v${VERSION}"
DRY_RUN=0
if [ "${2:-}" = "--dry-run" ]; then
  DRY_RUN=1
elif [ "${2:-}" != "" ]; then
  echo "unknown flag: ${2}"
  usage
  exit 1
fi

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "invalid version '$VERSION' (expected semver, e.g. 1.2.3)"
  exit 1
fi

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

for tool in cargo git; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool: $tool"
    exit 1
  fi
done

if [ "$DRY_RUN" -eq 0 ]; then
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "working tree must be clean before running release.sh"
    exit 1
  fi

  if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null 2>&1; then
    echo "tag ${TAG} already exists"
    exit 1
  fi
fi

update_package_version() {
  local file="$1"
  local version="$2"
  local tmp
  tmp="$(mktemp)"

  awk -v version="$version" '
    BEGIN { in_package = 0; updated = 0; has_package = 0 }
    /^\[package\]/ { in_package = 1; has_package = 1; print; next }
    /^\[/ {
      if (in_package == 1) {
        in_package = 0
      }
    }
    {
      if (in_package == 1 && updated == 0 && $0 ~ /^version[[:space:]]*=/) {
        print "version = \"" version "\""
        updated = 1
      } else {
        print
      }
    }
    END {
      if (has_package == 1 && updated == 0) {
        exit 9
      }
    }
  ' "$file" >"$tmp"

  if [ -s "$tmp" ]; then
    mv "$tmp" "$file"
  else
    rm -f "$tmp"
    echo "failed updating version in $file"
    exit 1
  fi
}

echo "[1/7] run release gates"
bash scripts/release-check.sh

echo "[2/7] update versions"
while IFS= read -r cargo_toml; do
  if grep -q "^\[package\]" "$cargo_toml"; then
    update_package_version "$cargo_toml" "$VERSION"
    echo "updated $cargo_toml"
  fi
done < <(find . -name Cargo.toml \
  -not -path "./target/*" \
  -not -path "./desktop/src-tauri/target/*" \
  -not -path "./desktop/node_modules/*" \
  | sort)

if [ -f "desktop/src-tauri/tauri.conf.json" ]; then
  awk -v version="$VERSION" '
    BEGIN { updated = 0 }
    {
      if (updated == 0 && $0 ~ /^[[:space:]]*"version"[[:space:]]*:/) {
        print "  \"version\": \"" version "\","
        updated = 1
      } else {
        print
      }
    }
    END { if (updated == 0) exit 7 }
  ' desktop/src-tauri/tauri.conf.json > desktop/src-tauri/tauri.conf.json.tmp
  mv desktop/src-tauri/tauri.conf.json.tmp desktop/src-tauri/tauri.conf.json
  echo "updated desktop/src-tauri/tauri.conf.json"
fi

if [ -f "desktop/package.json" ]; then
  awk -v version="$VERSION" '
    BEGIN { updated = 0 }
    {
      if (updated == 0 && $0 ~ /^[[:space:]]*"version"[[:space:]]*:/) {
        print "  \"version\": \"" version "\","
        updated = 1
      } else {
        print
      }
    }
    END { if (updated == 0) exit 8 }
  ' desktop/package.json > desktop/package.json.tmp
  mv desktop/package.json.tmp desktop/package.json
  echo "updated desktop/package.json"
fi

if [ -f "docs/homebrew/laminar.rb" ]; then
  awk -v version="$VERSION" '
    {
      line = $0
      gsub(/releases\/download\/v[0-9A-Za-z.\-]+/, "releases/download/v" version, line)
      if (line ~ /^[[:space:]]*version "/) {
        line = "  version \"" version "\""
      }
      print line
    }
  ' docs/homebrew/laminar.rb > docs/homebrew/laminar.rb.tmp
  mv docs/homebrew/laminar.rb.tmp docs/homebrew/laminar.rb
  echo "updated docs/homebrew/laminar.rb"
fi

echo "[3/7] build CLI release binary"
cargo build --release -p laminar-cli

CLI_BIN="target/release/laminar-cli"
if [ -f "${CLI_BIN}.exe" ]; then
  CLI_BIN="${CLI_BIN}.exe"
fi
if [ ! -f "$CLI_BIN" ]; then
  echo "CLI binary missing: $CLI_BIN"
  exit 1
fi

echo "[4/7] build desktop bundles"
(
  cd desktop
  npx tauri build
)

echo "[5/7] collect artifacts"
ARTIFACT_DIR="dist/release/${TAG}"
rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR"

cp "$CLI_BIN" "$ARTIFACT_DIR/"

if [ -d "desktop/src-tauri/target/release/bundle" ]; then
  mkdir -p "$ARTIFACT_DIR/desktop-bundles"
  cp -R desktop/src-tauri/target/release/bundle/. "$ARTIFACT_DIR/desktop-bundles/"
else
  echo "warning: desktop bundle directory not found (desktop/src-tauri/target/release/bundle)"
fi

echo "[6/7] generate checksums"
CHECKSUM_FILE="$ARTIFACT_DIR/SHA256SUMS.txt"
if command -v sha256sum >/dev/null 2>&1; then
  (
    cd "$ARTIFACT_DIR"
    find . -type f ! -name "SHA256SUMS.txt" -print0 | sort -z | xargs -0 sha256sum
  ) > "$CHECKSUM_FILE"
elif command -v shasum >/dev/null 2>&1; then
  (
    cd "$ARTIFACT_DIR"
    find . -type f ! -name "SHA256SUMS.txt" -print0 | sort -z | xargs -0 shasum -a 256
  ) > "$CHECKSUM_FILE"
else
  echo "missing checksum tool (sha256sum or shasum)"
  exit 1
fi

echo "[7/7] commit version bump and create signed tag"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "dry-run enabled: skipping git commit and signed tag creation"
else
  git add \
    crates/*/Cargo.toml \
    desktop/src-tauri/Cargo.toml \
    desktop/src-tauri/tauri.conf.json \
    desktop/package.json \
    docs/homebrew/laminar.rb
  git commit -m "release: ${TAG}"
  git tag -s "$TAG" -m "Laminar ${TAG}"
fi

echo "release complete"
echo "tag: $TAG"
echo "artifacts: $ARTIFACT_DIR"
