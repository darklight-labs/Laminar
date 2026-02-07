# Invariants (Tracer Bullet)

These are the non-negotiable rules for the tracer-bullet implementation. The broader Laminar program may add more invariants in later phases, but the rules below are enforced in this repo.

## INV-01 Zatoshi Standard
- All monetary values are u64 zatoshis.
- No floating point math is permitted for amounts.

## INV-02 Fail-Fast Validation
- Any invalid row rejects the entire batch.
- All errors are collected and reported together.

## INV-03 No Panics in Happy Path
- Core and CLI use Result-based error handling.
- No unwrap/expect/panic in non-test code.

## INV-04 Deterministic Output
- Agent JSON output is byte-identical for the same input.
- No timestamps, UUIDs, or randomized fields.

## INV-05 Dual-Mode Output
- Human mode when stdout is a TTY, unless `--output json` is used.
- Agent mode when stdout is piped or `--output json` is set.
- Agent mode is non-interactive and emits JSON only.

## FR-701 Human Readable Output
- UTF-8 tables with aligned columns
- Colors for status
- Amounts formatted as ZEC

## FR-702 Confirmation
- Human mode prompts before intent construction unless `--force` is set.
- Agent mode requires `--force` or exits with code 2 and JSON error.

## INV-07 Memo Byte Limit
- Memo fields must be UTF-8 and <= 512 bytes.
- Over-limit memos fail validation with error `E1004 MEMO_TOO_LONG`.
