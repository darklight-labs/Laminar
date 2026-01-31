# LAMINAR

**Operational Command Console for the Zcash Shielded Economy**

> Status: `PHASE 1 — TACTICAL SPIKE`  
> Forge: Darklight Labs

---

Laminar is a professional-grade transaction construction engine for Zcash treasury operations. It bridges the "Spreadsheet Gap"—the tooling void between CSV payroll data and shielded blockchain execution.

**Laminar is not a wallet.** It constructs Payment Intents. Your mobile wallet signs and broadcasts.

## The Problem

Zcash organizations manage disbursements with spreadsheets and manual copy-paste into mobile wallets. A 50-person payroll takes hours. Keys touch hot machines. No audit trail exists.

## The Solution

Laminar separates **Authority** (keys) from **Intent** (construction):

1. Import CSV → Validate → Construct ZIP-321 payment request
2. Display as QR code (static or animated UR)
3. Scan with mobile wallet (Zashi/YWallet)
4. Wallet signs and broadcasts—keys never leave the device

**Result:** 50-recipient batch in under 5 minutes. Air-gapped. Auditable.

## Architecture

This repository is a Rust workspace:

| Crate | Purpose |
|-------|---------|
| `laminar-core` | Stateless library—validation, construction, encoding |
| `laminar-cli` | Reference CLI for batch processing |

The desktop application (Tauri shell) wraps `laminar-core` and ships separately during Phase 1.

## Documentation

| Document | Description |
|----------|-------------|
| [INVARIANTS.md](./docs/INVARIANTS.md) | The Laws of Physics—non-negotiable constraints |
| [ARCHITECTURE.md](./docs/ARCHITECTURE.md) | System design and data flow |
| [THREAT_MODEL.md](./docs/THREAT_MODEL.md) | Attackers, trust boundaries, mitigations |
| [RFC-001.md](./docs/RFC-001.md) | Tactical Spike scope and acceptance criteria |
| [CONSTANTS.md](./docs/CONSTANTS.md) | Reference values for implementation |

## Quick Start

```bash
# Build the workspace
cargo build --release

# Run CLI (once implemented)
cargo run -p laminar-cli -- --input payroll.csv --output intent.txt
```

## Contributing

Before writing code, internalize the [Invariants](./docs/INVARIANTS.md). Every PR is reviewed against them.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development workflow.

## Security

See [SECURITY.md](./SECURITY.md) for vulnerability reporting.

## License

Dual-licensed under [MIT](./LICENSE-MIT) and [Apache 2.0](./LICENSE-APACHE).

---

**Darklight Labs** · Applied Privacy Infrastructure
