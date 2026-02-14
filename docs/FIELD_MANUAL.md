# Laminar Field Manual

## What Is Laminar
Laminar is a dual-mode Zcash batch-construction toolkit:

- `laminar-core` implements parsing, validation, ZIP-321 construction, QR/UR generation, and receipt generation.
- `laminar-cli` provides human/operator and machine/agent interfaces in one binary.
- `desktop` is a Tauri v2 application shell for operator workflows.

Laminar constructs payment requests and transfer artifacts. It does not sign or broadcast transactions.

## Installation
## CLI (from source)
```bash
cargo build --release -p laminar-cli
./target/release/laminar-cli --help
```

On Windows:
```powershell
cargo build --release -p laminar-cli
.\target\release\laminar-cli.exe --help
```

## CLI (cargo install)
```bash
cargo install --path crates/laminar-cli
laminar-cli --help
```

## CLI (Homebrew)
Template formula: `docs/homebrew/laminar.rb`

```bash
brew install --formula ./docs/homebrew/laminar.rb
laminar --help
```

## Desktop
```bash
cd desktop
npm install
npx tauri dev
```

## Quick Start
1. Prepare a batch file (`.csv` or `.json`).
2. Validate:
```bash
laminar-cli validate test-vectors/valid-simple.csv --network mainnet
```
3. Construct ZIP-321:
```bash
laminar-cli construct test-vectors/valid-simple.csv --network mainnet --output-dir ./out
```
4. Generate full artifacts (QR + receipt):
```bash
laminar-cli generate test-vectors/valid-simple.csv --network mainnet --output-dir ./out
```

## Preparing Batch Files
## CSV format
Required columns:

- address aliases: `address` or `recipient` or `to`
- amount aliases: `amount_zatoshis` or `zatoshis` or `zats` or `amount` or `value` or `zec`

Optional columns:

- memo aliases: `memo` or `message` or `note`
- label aliases: `label` or `name` or `recipient_name`

Example:
```csv
address,amount,memo,label
t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs,0.1,Invoice 42,Alice
tm9iMLAuYMzJ6jtFLcA7rzUmfreGuKvr7Ma,1000000,,Bob
```

Rules enforced by core validation:

- max file size: 10 MB
- max rows: 1000
- formula-injection protection on CSV cells
- address/network checks
- amount parsing and range checks
- memo length checks

## JSON format
Required structure:

```json
{
  "version": "1.0",
  "network": "mainnet",
  "recipients": [
    {
      "address": "t1...",
      "amount_zatoshis": 1000,
      "memo": "optional",
      "label": "optional"
    }
  ]
}
```

Notes:

- `network` must match CLI `--network`
- `amount_zatoshis` accepts non-negative integer or integer string

## Desktop Walkthrough
1. Launch app (`npx tauri dev`).
2. Import screen: drag/drop or browse CSV/JSON.
3. Review screen: inspect totals, warnings, and recipient table.
4. Proceed to construct + QR generation.
5. QR display:
- static QR for smaller payloads
- animated UR sequence for larger payloads
6. Confirm scan complete.
7. Generate and save receipt JSON.

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

Common flags:

- `--output json` forces machine/agent JSON mode
- `--interactive` forces operator mode
- `--force` bypasses confirmation prompts
- `--quiet` suppresses non-essential operator output
- `--no-color` disables color formatting in operator mode

## QR Scanning (Zashi + YWallet)
## Static QR
1. In wallet, open scan/import payment request.
2. Scan `qr-static.png`.
3. Confirm recipients and amounts in wallet before signing.

## Animated UR
1. Open wallet QR scanner.
2. Present animated sequence (`qr-ur-0001.png`, ...).
3. Keep frame loop visible until wallet reports completion.
4. Verify recipient count/amount total in wallet before signing.

If scanning fails:

- reduce glare and camera distance
- keep full quiet zone visible
- for UR, let the sequence loop at least twice

## Receipts
Generated receipt includes:

- Laminar version
- timestamp (ISO 8601)
- batch UUID
- network
- total amounts in zatoshis and ZEC string
- recipient list with memo/label
- ZIP-321 payload hash (`sha256:...`)
- segment/frame count

Default filename:

- `laminar-receipt-YYYY-MM-DD-<batchid8>.json`

## Troubleshooting
- `network mismatch`: check `--network` and input file network.
- validation code `1001`/`1005`/`1012`: inspect row-level data and schema.
- confirmation blocked in automation: use `--force` or `--output json`.
- QR decode issues: verify generated PNG integrity and scanner framing.
- Desktop build issues: run `npm install` and `cargo build` in `desktop/src-tauri`.

## Security Model
- Batch files are validated before artifact creation.
- Agent mode is non-interactive and never blocks on stdin.
- Deterministic machine JSON output supports reproducible automation.
- QR and receipt artifacts are deterministic for identical inputs.
- Desktop supports encrypted local storage for sensitive user fields.

Laminar does not custody keys and does not sign transactions.

## FAQ
## Does Laminar broadcast transactions?
No. Wallet software performs signing and broadcast.

## Can I automate Laminar in CI or bots?
Yes. Use `--output json` and parse stdout; rely on exit codes.

## What should I archive for audit?
Input batch, generated receipt JSON, and CLI/desktop version metadata.

## How do I install as a single CLI binary?
Use `cargo build --release -p laminar-cli` and distribute the produced executable.
