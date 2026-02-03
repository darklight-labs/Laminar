# Roadmap (Future Scope)

This document captures the broader Laminar vision beyond the tracer-bullet repository.
Items below are not implemented in this repo unless explicitly noted.

## Phase 1: Tracer Bullet (Current Repo)
- CSV -> parse -> validate -> intent -> output
- Zatoshi-only arithmetic (u64)
- Dual-mode CLI (human vs agent)
- Deterministic agent JSON output

## Phase 2: Operational Core
- ZIP-321 payment request construction
- Stronger address validation (full Zcash formats)
- Memo validation and UTF-8 bounds checks
- Batch sizing and payload segmentation

## Phase 3: Operator Interface
- Desktop UI (Tauri shell)
- Batch review and approval UX
- QR / UR encoding for wallet scanning
- Local drafts and address book

## Phase 4: Ecosystem Integration
- Agent integration guides
- CI test vectors and compatibility suites
- Formal security review and audit readiness

## Guiding Principles
- Never handle keys or sign transactions
- Deterministic, auditable output
- Non-interactive automation compatibility
