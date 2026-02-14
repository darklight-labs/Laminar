# Laminar

Laminar is a dual-mode Zcash batch-construction toolkit for operators and automation agents.

## Project Overview
Laminar provides a deterministic pipeline:

1. Parse CSV/JSON recipient batches
2. Validate addresses, amounts, memos, and network alignment
3. Construct ZIP-321 transaction intents
4. Generate static QR or animated UR frames
5. Produce JSON receipts for auditability

It does not sign or broadcast transactions; wallet software handles that step.

## Repository Layout
- `crates/laminar-core`: core parsing, validation, ZIP-321, QR/UR, receipts
- `crates/laminar-cli`: dual-mode CLI (`operator` + `agent`)
- `desktop`: Tauri v2 desktop app shell
- `docs`: operational docs and integration guides
- `test-vectors`: fixture corpus for validation/integration testing

## Quick Start
## CLI
```bash
cargo build --release -p laminar-cli
./target/release/laminar-cli validate test-vectors/valid-simple.csv --network mainnet
```

Generate complete artifacts:
```bash
./target/release/laminar-cli generate test-vectors/valid-simple.csv --network mainnet --output-dir ./out
```

## Desktop
```bash
cd desktop
npm install
npx tauri dev
```

## CLI Usage
## Validate
```bash
laminar-cli validate <file> --network <mainnet|testnet>
```

## Construct
```bash
laminar-cli construct <file> --network <mainnet|testnet> [--output-dir <dir>]
```

## Generate
```bash
laminar-cli generate <file> --network <mainnet|testnet> [--output-dir <dir>]
```

## Agent mode
```bash
laminar-cli --output json generate <file> --network mainnet --output-dir ./out
```

## Desktop Screenshots
Placeholders:

- `docs/screenshots/import.png`
- `docs/screenshots/review.png`
- `docs/screenshots/qr.png`
- `docs/screenshots/receipt.png`

Example markdown:
```md
![Import Screen](docs/screenshots/import.png)
![Review Screen](docs/screenshots/review.png)
![QR Screen](docs/screenshots/qr.png)
![Receipt Screen](docs/screenshots/receipt.png)
```

## Build Instructions
## Workspace checks
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

## Desktop packaging
```bash
cd desktop
npx tauri build
```

Bundle targets are configured in `desktop/src-tauri/tauri.conf.json`:

- macOS: DMG (minimum 10.15)
- Windows: NSIS
- Linux: AppImage, `.deb`, `.rpm`

## Release tooling
- release gates: `scripts/release-check.sh`
- versioning + artifacts + checksums + signed tag: `scripts/release.sh <version>`
- Homebrew formula template: `docs/homebrew/laminar.rb`

## Architecture Overview
## Core types
- `ValidatedBatch`: canonical validated recipient set
- `TransactionIntent`: ZIP-321 payload + metadata
- `QrOutput`: static or animated frames + payload metadata
- `Receipt`: immutable batch execution artifact

## Mode model
- `operator`: TTY UI, progress indicators, confirmations
- `agent`: deterministic JSON output, non-blocking automation path

## Documentation Index
- Field manual: `docs/FIELD_MANUAL.md`
- Agent integration guide: `docs/AGENT_INTEGRATION.md`

## License
Licensed under either:

- Apache-2.0 (`LICENSE-APACHE`)
- MIT (`LICENSE-MIT`)

at your option.
