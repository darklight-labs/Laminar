# ENGINEERING INVARIANTS

> **Status:** ENFORCED  
> **Scope:** All Laminar Components  
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

## INV-03: Network Isolation

**Laminar NEVER broadcasts transactions to the Zcash network.**

The final output is a ZIP-321 URI or UR-encoded QR. Broadcasting is the operator's sovereign responsibility via external tools.

**Enforcement:**
- No peer-to-peer network code
- No RPC client implementations
- Network stack limited to local IPC (Tauri bridge)

---

## INV-04: Zatoshi Standard

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

## INV-05: Schema Validation

**All cross-boundary data MUST be validated against versioned schemas.**

No raw JSON parsing. All external input passes through typed validators before processing.

**Enforcement:**
- Rust: `serde` with custom deserializers
- TypeScript: Zod schemas at every boundary
- Schema versions embedded in serialized data

---

## INV-06: Field Encryption

**Sensitive IndexedDB fields MUST be encrypted with the session key.**

Encrypted fields: `Contact.label`, `Contact.notes`, `Draft.recipients[].memo`

**Enforcement:**
- AES-256-GCM encryption for sensitive fields
- Key derived via PBKDF2 (100k iterations) from passphrase
- Encryption verified in integration tests

---

## INV-07: Memo Encoding

**Memos encoded as UTF-8 bytes → standard base64. Maximum 512 bytes.**

The 512-byte limit is a Zcash protocol constraint. Violations cause transaction rejection.

**Enforcement:**
- Byte length check before encoding
- Reject (not truncate) oversized memos
- Schema validation at ingestion

---

## INV-08: Handoff Conformance

**Handoff results MUST conform to the `HandoffResult` schema.**

All wallet interactions produce structured, versioned results for audit trail.

**Enforcement:**
- Schema validation on handoff completion
- Required fields: `schemaVersion`, `status`, `mode`, `timestamp`, `intentId`

---

## INV-09: Zero Telemetry

**No telemetry. No analytics. No external network requests.**

Laminar operates in adversarial environments. Any data exfiltration is a critical vulnerability.

**Enforcement:**
- CSP blocks external connections
- No analytics libraries in dependency tree
- Network traffic audited during security review

---

## INV-10: Fail-Fast Validation

**If ANY row fails validation, the ENTIRE batch is rejected. No partial processing.**

This prevents scenarios where treasurers unknowingly send incomplete payrolls.

**Enforcement:**
- Validation runs completely before construction begins
- Single invalid row returns error with full diagnostics
- UI cannot proceed to QR generation with validation errors

---

## Invariant Verification

During code review, verify each change against:

```
[ ] INV-01: No key material handling
[ ] INV-02: No signing operations
[ ] INV-03: No network broadcast
[ ] INV-04: Integer-only monetary math
[ ] INV-05: Schema validation at boundaries
[ ] INV-06: Sensitive fields encrypted
[ ] INV-07: Memo encoding correct
[ ] INV-08: Handoff results conform
[ ] INV-09: No telemetry/analytics
[ ] INV-10: Fail-fast on validation errors
```
