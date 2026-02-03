# Threat Model (Tracer Bullet)

This document covers threats relevant to the current CLI and core library only.

## Attackers and Risks
- Malicious CSV content (formula injection, invalid encoding)
- Unexpected input sizes leading to overflow
- Operator error in interactive mode

## Mitigations
- Strict parsing and validation rules
- Integer-only arithmetic for amounts
- Fail-fast rejection on any invalid row
- Non-interactive agent mode with explicit `--force` requirement

## Out of Scope
- Wallet security and signing
- Network broadcast or confirmation
- Desktop UI storage and encryption