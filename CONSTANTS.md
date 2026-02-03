# CONSTANTS REFERENCE

> **Scope:** Implementation Constants for Laminar  
> **Version:** 2.0 (Dual-Mode)

---

## Monetary Constants

```rust
/// Conversion factor: 1 ZEC = 100,000,000 zatoshis
pub const ZATOSHI_PER_ZEC: u64 = 100_000_000;

/// Minimum valid amount (1 zatoshi)
pub const ZATOSHI_MIN: u64 = 1;

/// Maximum valid amount (21 million ZEC total supply)
pub const ZATOSHI_MAX: u64 = 2_100_000_000_000_000;

/// Dust threshold (0.0001 ZEC = 10,000 zatoshis)
pub const DUST_THRESHOLD_ZAT: u64 = 10_000;
```

---

## Payload Limits

| Mode | Raw Limit | Safety Margin | Effective Limit |
|------|-----------|---------------|-----------------|
| QR Static (ZIP-321) | 2,953 bytes | 15% | **2,510 bytes** |
| QR Animated (UR) | 32 KB | 10% | **29,000 bytes** |
| Deep Link | 8 KB | 10% | **7,200 bytes** |

```rust
/// Maximum payload for static QR code (with safety margin)
pub const PAYLOAD_LIMIT_QR_STATIC: usize = 2510;

/// Maximum payload for animated UR sequence (with safety margin)
pub const PAYLOAD_LIMIT_QR_ANIMATED: usize = 29_000;

/// Maximum payload for deep link (with safety margin)
pub const PAYLOAD_LIMIT_DEEPLINK: usize = 7200;
```

---

## Encoding Constants

```rust
/// Maximum memo size in bytes (Zcash protocol limit)
pub const MEMO_MAX_BYTES: usize = 512;

/// Maximum recipients per batch
pub const MAX_RECIPIENTS: usize = 1000;

/// UR fragment size in bytes
pub const UR_FRAGMENT_SIZE: usize = 150;

/// UR frame rate (10 FPS = 100ms per frame)
pub const UR_FRAME_RATE_MS: u32 = 100;

/// Maximum CSV file size (10MB)
pub const MAX_CSV_FILE_SIZE: usize = 10_485_760;
```

---

## Recipient Capacity Estimates

| Mode | Typical Capacity | Notes |
|------|------------------|-------|
| Static QR | 8-12 recipients | Depends on memo length |
| Animated UR | 50+ recipients | ~15-20 frames at 100ms |
| Auto Batch Split | Unlimited | Sequential segments |

---

## CLI Exit Codes

| Exit Code | Name | Meaning |
|-----------|------|---------|
| `0` | SUCCESS | Operation completed successfully |
| `1` | VALIDATION_ERROR | Input data invalid |
| `2` | CONFIG_ERROR | Invalid arguments or flags |
| `3` | IO_ERROR | File read/write failure |
| `4` | INTERNAL_ERROR | Unexpected failure |

```rust
pub enum ExitCode {
    Success = 0,
    ValidationError = 1,
    ConfigError = 2,
    IoError = 3,
    InternalError = 4,
}
```

---

## Error Codes

### Validation Errors (E001-E009)

| Code | Name | Description |
|------|------|-------------|
| E001 | INVALID_ADDRESS_FORMAT | Address does not match any valid Zcash encoding |
| E002 | NETWORK_MISMATCH | Address network does not match configured network |
| E003 | AMOUNT_OUT_OF_RANGE | Amount is zero, negative, or exceeds maximum |
| E004 | AMOUNT_PRECISION_LOSS | Amount cannot be represented as integer zatoshis |
| E005 | MEMO_TOO_LONG | Memo exceeds 512 bytes when UTF-8 encoded |
| E006 | MEMO_INVALID_UTF8 | Memo contains invalid UTF-8 sequences |
| E007 | BATCH_TOTAL_OVERFLOW | Sum of all amounts exceeds u64 maximum |
| E008 | CSV_PARSE_ERROR | CSV file is malformed or uses unsupported encoding |
| E009 | MISSING_REQUIRED_COLUMN | Required column (address or amount) not found |

### CLI Errors (E010-E011) — NEW

| Code | Name | Description |
|------|------|-------------|
| E010 | MISSING_REQUIRED_ARGUMENT | Required CLI argument not provided (Agent mode) |
| E011 | CONFIRMATION_REQUIRED | Operation requires `--force` flag in non-interactive mode |

### Warnings (W001)

| Code | Name | Description |
|------|------|-------------|
| W001 | DUPLICATE_ADDRESS | Same address appears multiple times (warning only) |

---

## Validation Regex

```rust
/// Valid zatoshi string: non-negative integer, no leading zeros (except "0")
pub const ZATOSHI_STRING_REGEX: &str = r"^(0|[1-9]\d*)$";

/// ZEC decimal format: integer or decimal with up to 8 places
pub const ZEC_DECIMAL_REGEX: &str = r"^\d+(\.\d{1,8})?$";
```

---

## CSV Formula Prefixes

Characters that indicate potential formula injection attack:

```rust
pub const FORMULA_PREFIXES: &[char] = &['=', '+', '-', '@', '\t', '\r'];
```

---

## Network Identifiers

```rust
pub enum Network {
    Mainnet,  // "mainnet"
    Testnet,  // "testnet"
}
```

---

## CLI Mode Detection

```rust
pub enum Mode {
    Operator,  // Human-centric (TTY detected)
    Agent,     // Machine-centric (piped or --output json)
}
```

---

## Schema Versions

| Schema | Current Version |
|--------|-----------------|
| TransactionIntent | `1.0` |
| HandoffResult | `1.0` |
| Receipt | `1.0` |
| CSV Input | `1.0` |
| **CLI Output (Agent Mode)** | `1.0` |
