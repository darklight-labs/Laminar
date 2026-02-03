# LAMINAR

**Operational Command Console for the Zcash Shielded Economy**

> Status: `PHASE 1 — TACTICAL SPIKE`  
> Version: `2.0 (Dual-Mode)`  
> Forge: Darklight Labs

---

Laminar is a professional-grade transaction construction engine for Zcash treasury operations. It bridges the "Spreadsheet Gap"—the tooling void between CSV payroll data and shielded blockchain execution.

**Laminar is not a wallet.** It constructs Payment Intents. Your mobile wallet signs and broadcasts.

## The Problem

Zcash organizations manage disbursements with spreadsheets and manual copy-paste into mobile wallets. A 50-person payroll takes hours. Keys touch hot machines. No audit trail exists. Existing tools cannot be driven by AI agents.

## The Solution

Laminar separates **Authority** (keys) from **Intent** (construction):

1. Import CSV → Validate → Construct ZIP-321 payment request
2. Display as QR code (static or animated UR)
3. Scan with mobile wallet (Zashi/YWallet)
4. Wallet signs and broadcasts—keys never leave the device

**Result:** 50-recipient batch in under 5 minutes. Air-gapped. Auditable. **Agent-compatible.**

## Dual-Mode CLI

The `laminar-cli` automatically adapts its interface based on execution context:

| Mode | Trigger | Behavior |
|------|---------|----------|
| **Operator** | Terminal (TTY) | Spinners, tables, colors, confirmations |
| **Agent** | Piped or `--output json` | Silent, strict JSON, non-interactive |

```bash
# Human operator (interactive)
laminar construct --input payroll.csv

# Software agent (JSON output)
laminar construct --input payroll.csv --output json | jq '.result.zip321_uri'
```

## Architecture

This repository is a Rust workspace:

| Crate | Purpose |
|-------|---------|
| `laminar-core` | Stateless library—validation, construction, encoding |
| `laminar-cli` | Dual-mode CLI for batch processing |

The desktop application (Tauri shell) wraps `laminar-core` and ships separately during Phase 1.

## Documentation

| Document | Description |
|----------|-------------|
| [INVARIANTS.md](INVARIANTS.md) | The Laws of Physics—non-negotiable constraints |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design and data flow |
| [THREAT_MODEL.md](THREAT_MODEL.md) | Attackers, trust boundaries, mitigations |
| [RFC-001.md](RFC-001.md) | Tactical Spike scope and acceptance criteria |
| [CONSTANTS.md](CONSTANTS.md) | Reference values for implementation |

## Quick Start

```bash
# Build the workspace
cargo build --release

# Run CLI in Operator mode (human)
cargo run -p laminar-cli -- construct --input payroll.csv

# Run CLI in Agent mode (machine)
cargo run -p laminar-cli -- construct --input payroll.csv --output json
```

## CLI Commands

```bash
laminar construct --input <file>    # Build payment request from CSV
laminar validate --input <file>     # Validate CSV without constructing
laminar info                        # Display version and constants
```

### Flags

| Flag | Description |
|------|-------------|
| `--output json` | Force Agent mode (JSON output) |
| `--interactive` | Force Operator mode (for testing) |
| `--force` | Bypass confirmation prompts |
| `--network <net>` | Target network (mainnet/testnet) |

## Contributing

Before writing code, internalize the [Invariants](INVARIANTS.md). Every PR is reviewed against them.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow.

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE).

---

**Darklight Labs** · Applied Privacy Infrastructure
