# System Architecture (Tracer Bullet)

NOTE: This document focuses on what is implemented in this repository. Any future features (wallet integration, QR/UR output, ZIP-321 URIs) are intentionally out of scope here.

## Components

### laminar-core (Rust crate)
Stateless core logic that implements:
- CSV amount parsing into zatoshis (u64 only)
- Address validation (prefix-only in this tracer bullet)
- Shared data types for intent output

### laminar-cli (Rust binary)
CLI wrapper that provides:
- TTY detection for human vs agent output
- Fail-fast batch validation
- JSON intent emission in agent mode
- Human-friendly tables and confirmation prompt in operator mode

### demo/
Sample CSV files and scripts that exercise the core flow.

## Data Flow
1. Read CSV
2. Validate each row (address + amount)
3. Accumulate totals in zatoshis
4. On any error: reject entire batch
5. On success: emit intent JSON

## Output Modes
- Human mode: TTY detected, spinner, tables, confirmation prompt
- Agent mode: stdout piped or `--output json`, JSON only, no prompts

## Determinism
Agent output is byte-identical for the same input. No timestamps, random IDs, or map iteration order are used.

## File Map
- Core parsing: `laminar-core/src/parser.rs`
- Address validation: `laminar-core/src/validation.rs`
- Output helpers: `laminar-core/src/output.rs`
- CLI logic: `laminar-cli/src/main.rs`