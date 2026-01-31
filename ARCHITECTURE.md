# SYSTEM ARCHITECTURE

> **Stack:** Rust (Core) / Tauri (Shell) / React (UI)  
> **Pattern:** Unidirectional Data Flow

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
- No randomness—deterministic output guaranteed

### 1.2 Laminar CLI (Binary)

Thin wrapper around `laminar-core` for terminal usage. Reference implementation for testing and automation.

```
Input:  payroll.csv
Output: payment_intent.zip321 (or .ur for animated)
        receipt.json
```

### 1.3 Laminar Terminal (Desktop Application)

Cross-platform desktop application (Phase 1 deliverable). Built with Tauri.

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

## 3. The Pipeline

Laminar transforms "dirty" user data (CSV) into "clean" consensus artifacts (ZIP-321).

| Stage | Input | Output | Invariants |
|-------|-------|--------|------------|
| **Ingestion** | Raw bytes | String | — |
| **Sanitization** | String | Cleaned string | Formula injection blocked |
| **Parsing** | Cleaned string | `Vec<RawRecipient>` | Schema validated (INV-05) |
| **Validation** | `RawRecipient` | `Recipient` | Address/amount/memo checked (INV-10) |
| **Construction** | `Vec<Recipient>` | `TransactionIntent` | Zatoshi math only (INV-04) |
| **Encoding** | `TransactionIntent` | ZIP-321 URI | Deterministic (INV-04) |
| **Display** | ZIP-321 URI | QR Code | Size-appropriate mode |

---

## 4. Trust Boundaries

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

## 5. Module Hierarchy

```
laminar-core/
├── src/
│   ├── lib.rs           # Public API surface
│   ├── types.rs         # Core type definitions
│   ├── zatoshi.rs       # Monetary arithmetic (INV-04)
│   ├── address.rs       # Zcash address validation
│   ├── memo.rs          # Memo encoding (INV-07)
│   ├── csv.rs           # CSV parsing and sanitization
│   ├── validation.rs    # Batch validation (INV-10)
│   ├── zip321.rs        # Payment request construction
│   ├── ur.rs            # Uniform Resources encoding
│   ├── receipt.rs       # JSON receipt generation
│   └── error.rs         # Error taxonomy
└── Cargo.toml

laminar-cli/
├── src/
│   └── main.rs          # CLI entry point
└── Cargo.toml
```

---

## 6. Technology Decisions

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Core Logic | Rust | Memory safety, librustzcash compatibility, cross-compilation |
| Desktop Shell | Tauri | Small binary (~10MB), native performance, security sandbox |
| UI Framework | React + TypeScript | Developer availability, type safety |
| Styling | Tailwind CSS | Utility-first, consistent system, small bundle |
| Local Storage | IndexedDB + AES-GCM | Browser-native, async, encryption at rest |
| Schema Validation | Zod (TS) / serde (Rust) | Runtime type checking at boundaries |
| QR Encoding | qrcode + ur-rs | Standard libraries, UR animation support |
