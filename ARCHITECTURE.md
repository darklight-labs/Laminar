# SYSTEM ARCHITECTURE

> **Stack:** Rust (Core) / Tauri (Shell) / React (UI)  
> **Pattern:** Unidirectional Data Flow  
> **Version:** 2.0 (Dual-Mode)

---

## 1. System Components

### 1.1 Laminar Core (Rust Crate)

The foundational library implementing all business logic. Stateless, deterministic, panic-free.

**Responsibilities:**
- CSV/JSON parsing and normalization
- Zcash address validation (Unified, Sapling, Transparent)
- Integer arithmetic for zatoshi amounts
- ZIP-321 payment request construction
- UR (Uniform Resources) encoding for animated QR
- Receipt bundle generation

**Design Constraints:**
- All errors via `Result<T, LaminarError>`—no panics
- No I/O operations—pure functions only
- No randomness—deterministic output guaranteed (INV-04)

### 1.2 Laminar CLI (Dual-Mode Binary)

Command-line interface wrapping `laminar-core` with **automatic interface adaptation**. The CLI implements TTY detection at startup to determine output mode.

**TTY Detection Logic:**
```rust
fn detect_mode(args: &Args) -> Mode {
    if args.output_json || args.force {
        Mode::Agent
    } else if std::io::stdout().is_terminal() {
        Mode::Operator
    } else {
        Mode::Agent
    }
}
```

#### Mode A: Operator Interface (Human-Centric)

Activates when a human runs the command in a terminal.

| Feature | Description |
|---------|-------------|
| **Visual Feedback** | Spinner animations during processing |
| **Rich Formatting** | ASCII tables, color-coded status (ANSI) |
| **Safety Prompts** | Interactive confirmation before fund-affecting operations |
| **Error Messages** | Human-readable with actionable suggestions |

#### Mode B: Agent Interface (Machine-Centric)

Activates when stdout is piped or `--output json` is specified.

| Feature | Description |
|---------|-------------|
| **Silence** | No spinners, progress text, or ASCII art |
| **Strict JSON** | Only valid JSON to stdout, conforming to schema |
| **Non-Interactive** | Never blocks for input; immediate exit on missing args |
| **Deterministic** | Consistent field ordering, predictable output |

**Flag Overrides:**
- `--output json` — Forces Agent mode regardless of TTY
- `--interactive` — Forces Operator mode even when piped (testing)
- `--force` — Bypasses confirmation prompts in either mode

### 1.3 Laminar Terminal (Desktop Application)

Cross-platform desktop application wrapping Laminar Core. Built with Tauri.

**Platforms:** macOS (Intel + Apple Silicon), Windows (x64), Linux (AppImage)

**Stack:**
- Shell: Tauri (Rust + WebView)
- UI: React + TypeScript + Tailwind CSS
- Storage: Encrypted IndexedDB

---

## 2. Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         LAMINAR TERMINAL                                │
│                                                                         │
│  ┌──────────┐    ┌──────────────────────────┐    ┌─────────────────┐   │
│  │   CSV    │───▶│      LAMINAR CORE        │───▶│   QR DISPLAY    │   │
│  │  INPUT   │    │                          │    │ (Static/Animated)│   │
│  └──────────┘    │  1. Parse                │    └────────┬────────┘   │
│                  │  2. Sanitize             │             │            │
│                  │  3. Validate             │         [AIR GAP]        │
│                  │  4. Construct            │             │            │
│                  │  5. Encode               │             ▼            │
│                  └──────────────────────────┘    ┌─────────────────┐   │
│                                                  │  MOBILE WALLET  │   │
│  ┌──────────┐                                    │ (Zashi/YWallet) │   │
│  │  JSON    │◀───────────────────────────────────│                 │   │
│  │ RECEIPT  │         [After broadcast]          │  • Scan QR      │   │
│  └──────────┘                                    │  • Sign tx      │   │
│                                                  │  • Broadcast    │   │
│                                                  └─────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. CLI Dual-Mode Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          LAMINAR CLI                                    │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                      MODE DETECTION                               │  │
│  │                                                                   │  │
│  │   stdout.is_terminal()?  ──┬── YES ──▶  OPERATOR MODE (A)        │  │
│  │          │                 │                                      │  │
│  │          │                 └── NO ───▶  AGENT MODE (B)           │  │
│  │          │                                                        │  │
│  │   --output json?  ─────────────────▶  AGENT MODE (B) [override]  │  │
│  │   --interactive?  ─────────────────▶  OPERATOR MODE (A) [override]│  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  ┌─────────────────────┐          ┌─────────────────────┐              │
│  │   OPERATOR MODE     │          │    AGENT MODE       │              │
│  │                     │          │                     │              │
│  │  • Spinners         │          │  • Silent           │              │
│  │  • ASCII tables     │          │  • JSON only        │              │
│  │  • Color output     │          │  • Non-interactive  │              │
│  │  • Confirmations    │          │  • Exit codes       │              │
│  │  • Suggestions      │          │  • Schema-compliant │              │
│  └─────────────────────┘          └─────────────────────┘              │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 4. The Pipeline

Laminar transforms "dirty" user data (CSV) into "clean" consensus artifacts (ZIP-321).

| Stage | Input | Output | Invariants |
|-------|-------|--------|------------|
| **Ingestion** | Raw bytes | String | — |
| **Sanitization** | String | Cleaned string | Formula injection blocked |
| **Parsing** | Cleaned string | `Vec<RawRecipient>` | Schema validated (INV-08) |
| **Validation** | `RawRecipient` | `Recipient` | Address/amount/memo checked (INV-05) |
| **Construction** | `Vec<Recipient>` | `TransactionIntent` | Zatoshi math only (INV-03) |
| **Encoding** | `TransactionIntent` | ZIP-321 URI | Deterministic (INV-04) |
| **Display** | ZIP-321 URI | QR Code | Size-appropriate mode |

---

## 5. Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    UNTRUSTED ZONE                           │
│                                                             │
│   User Input (CSV)    External Wallet Response              │
│         │                      │                            │
└─────────┼──────────────────────┼────────────────────────────┘
          │                      │
          ▼                      ▼
    ┌─────────────────────────────────────────────┐
    │           BOUNDARY 1: INGESTION             │
    │     Sanitize → Parse → Validate             │
    └─────────────────────────────────────────────┘
          │
          ▼
    ┌─────────────────────────────────────────────┐
    │           TRUSTED ZONE (Core)               │
    │     Construction → Encoding                 │
    └─────────────────────────────────────────────┘
          │
          ▼
    ┌─────────────────────────────────────────────┐
    │          BOUNDARY 2: STORAGE                │
    │     Encrypt → IndexedDB                     │
    └─────────────────────────────────────────────┘
          │
          ▼
    ┌─────────────────────────────────────────────┐
    │          BOUNDARY 3: HANDOFF                │
    │     QR Display → Fire-and-Forget            │
    └─────────────────────────────────────────────┘
```

---

## 6. Module Hierarchy

```
laminar-core/
├── src/
│   ├── lib.rs           # Public API surface
│   ├── types.rs         # Core type definitions
│   ├── zatoshi.rs       # Monetary arithmetic (INV-03)
│   ├── address.rs       # Zcash address validation
│   ├── memo.rs          # Memo encoding
│   ├── csv.rs           # CSV parsing and sanitization
│   ├── validation.rs    # Batch validation (INV-05)
│   ├── zip321.rs        # Payment request construction
│   ├── ur.rs            # Uniform Resources encoding
│   ├── receipt.rs       # JSON receipt generation
│   └── error.rs         # Error taxonomy
└── Cargo.toml

laminar-cli/
├── src/
│   ├── main.rs          # Entry point, mode detection
│   ├── mode.rs          # Operator/Agent mode logic
│   ├── operator.rs      # Human-centric output (spinners, tables)
│   ├── agent.rs         # Machine-centric output (JSON schema)
│   └── output.rs        # CLI output schema types
└── Cargo.toml
```

---

## 7. Technology Stack

| Layer | Technology | Justification |
|-------|------------|---------------|
| Core Logic | Rust | Memory safety, librustzcash compatibility |
| CLI Interface | Rust + clap | Native performance, TTY detection |
| Desktop Shell | Tauri | Small binary (~10MB), native performance |
| UI Framework | React + TypeScript | Developer availability, type safety |
| Styling | Tailwind CSS | Utility-first, consistent system |
| Local Storage | IndexedDB + AES-GCM | Browser-native, encryption at rest |
| Schema Validation | Zod (TS) / serde (Rust) | Runtime type checking at boundaries |
| QR Encoding | qrcode + ur-rs | Standard libraries, UR animation support |
