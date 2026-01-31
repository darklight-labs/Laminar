# CONSTANTS REFERENCE

> **Scope:** Implementation Constants for Laminar  
> **Source:** Engineering Specification v1.0

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
pub const MAX_RECIPIENTS: usize = 500;

/// UR fragment size in bytes
pub const UR_FRAGMENT_SIZE: usize = 150;

/// UR frame rate (10 FPS = 100ms per frame)
pub const UR_FRAME_RATE_MS: u32 = 100;
```

---

## Recipient Capacity Estimates

| Mode | Typical Capacity | Notes |
|------|------------------|-------|
| Static QR | 8-12 recipients | Depends on memo length |
| Animated UR | 50+ recipients | ~15-20 frames at 100ms |
| Auto Batch Split | Unlimited | Sequential segments |

---

## Validation Regex

```rust
/// Valid zatoshi string: non-negative integer, no leading zeros (except "0")
pub const ZATOSHI_STRING_REGEX: &str = r"^(0|[1-9]\d*)$";

/// ZEC decimal format: integer or decimal with up to 8 places
pub const ZEC_DECIMAL_REGEX: &str = r"^\d+(\.\d{1,8})?$";
```

---

## Security Constants

```rust
/// PBKDF2 iteration count for key derivation
pub const PBKDF2_ITERATIONS: u32 = 100_000;

/// AES key size in bits
pub const AES_KEY_BITS: usize = 256;
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

## Schema Versions

| Schema | Current Version |
|--------|-----------------|
| TransactionIntent | `1.0` |
| HandoffResult | `1.0` |
| Receipt | `1.0` |
| CSV Input | `1.0` |
