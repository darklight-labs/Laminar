# ENGINEERING INVARIANTS

> **Status:** ENFORCED  
> **Scope:** All Laminar Components  
> **Version:** 2.0 (Dual-Mode)  
> **Violation:** IMMEDIATE REJECTION

These invariants are structural constraints defining Laminar's security model and legal positioning. They cannot be relaxed for "convenience" or "UX." Every pull request must be reviewed against this list.

---

## INV-01: Stateless Authority

**Laminar NEVER stores, generates, or requests spending keys or seed phrases.**

The application has no authority to sign transactions. This is the foundational security property.

**Enforcement:**
- Code review verifies no cryptographic signing operations exist
- No storage mechanisms for key material
- No network requests that could exfiltrate keys

---

## INV-02: Calculator Defense

**Laminar NEVER signs transactions. It constructs Payment Intents only.**

The mobile wallet is the sole signing authority. This ensures Laminar cannot be classified as a money transmitter or custodial service.

**Enforcement:**
- No transaction signing libraries imported
- No network broadcast code
- All outputs are data artifacts (QR codes, JSON), not blockchain operations

---

## INV-03: Zatoshi Standard

**ALL monetary arithmetic uses integer zatoshis (`u64`/`bigint`). Floating-point is PROHIBITED.**

This prevents rounding errors that could cause transaction failures or fund loss.

```
1 ZEC = 100,000,000 zatoshis
```

**Enforcement:**
- Amount fields typed as `u64` (Rust) or `bigint` (TypeScript)
- Lint rules flag floating-point operations on monetary values
- Property tests verify `parseZec` ↔ `formatZec` invertibility

---

## INV-04: Deterministic Output

**Given identical input, Laminar MUST produce byte-identical output.**

Same CSV + same configuration = same QR codes, same receipt JSON. No randomness in transaction construction. This enables verification and debugging.

**Enforcement:**
- No random number generation in core logic
- Canonical serialization (sorted keys, consistent encoding)
- Snapshot tests verify determinism

---

## INV-05: Fail-Fast Validation

**If ANY row fails validation, the ENTIRE batch is rejected. No partial processing.**

This prevents scenarios where treasurers unknowingly send incomplete payrolls.

**Enforcement:**
- Validation runs completely before construction begins
- Single invalid row returns error with full diagnostics
- UI cannot proceed to QR generation with validation errors

---

## INV-06: Modal Determinism

**The CLI MUST behave identically in Agent mode regardless of invocation method.**

Whether triggered by `--output json` flag or pipe detection, the output format and behavior MUST be identical. Mode detection MUST be deterministic and testable.

**Mode Detection Logic:**
```rust
if stdout.is_terminal() && !args.output_json {
    Mode::Operator
} else {
    Mode::Agent
}
```

**Enforcement:**
- Mode selection logic centralized in single function
- Test suite includes explicit tests for both TTY and pipe scenarios
- CI runs tests with both invocation methods

---

## INV-07: Non-Blocking Agent Mode

**In Agent mode, the CLI MUST NEVER block waiting for user input.**

Any operation requiring confirmation MUST either proceed automatically (with `--force`) or fail immediately with a specific error code. Infinite loops and stdin reads are prohibited.

**Enforcement:**
- Agent mode code paths statically analyzed to ensure no stdin reads
- Integration tests verify timeout behavior
- Missing required flags result in immediate exit with documented error codes:
  - `E010`: Missing required argument
  - `E011`: Confirmation required (use `--force`)

---

## INV-08: Schema Validation

**All cross-boundary data MUST be validated against versioned schemas.**

No raw JSON parsing. All external input passes through typed validators before processing.

**Enforcement:**
- Rust: `serde` with custom deserializers
- TypeScript: Zod schemas at every boundary
- Schema versions embedded in serialized data

---

## INV-09: Field Encryption

**Sensitive IndexedDB fields MUST be encrypted with the session key.**

Encrypted fields: `Contact.label`, `Contact.notes`, `Draft.recipients[].memo`

**Enforcement:**
- AES-256-GCM encryption for sensitive fields
- Key derived via Argon2id from passphrase
- Encryption verified in integration tests

---

## INV-10: Zero Telemetry

**No telemetry. No analytics. No external network requests.**

Laminar operates in adversarial environments. Any data exfiltration is a critical vulnerability.

**Enforcement:**
- CSP blocks external connections
- No analytics libraries in dependency tree
- Network traffic audited during security review

---

## Invariant Verification Checklist

During code review, verify each change against:

```
[ ] INV-01: No key material handling
[ ] INV-02: No signing operations
[ ] INV-03: Integer-only monetary math (Zatoshi Standard)
[ ] INV-04: Deterministic output
[ ] INV-05: Fail-fast on validation errors
[ ] INV-06: Modal determinism (CLI modes identical)
[ ] INV-07: Non-blocking agent mode
[ ] INV-08: Schema validation at boundaries
[ ] INV-09: Sensitive fields encrypted
[ ] INV-10: No telemetry/analytics
```
