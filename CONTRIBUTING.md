# Contributing to Laminar

Thank you for your interest in contributing to Laminar.

## Before You Start
1. Read the invariants in `INVARIANTS.md`.
2. Review the current architecture in `ARCHITECTURE.md`.
3. Check the tracer-bullet scope in `RFC-001.md`.

## Development Setup
### Prerequisites
- Rust (stable)
- Cargo

### Building
```bash
cargo build
```

### Testing
```bash
cargo test
```

### Linting
```bash
cargo clippy -- -D warnings
```

## CLI Usage (Current)
```bash
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force
```

## Pull Request Process
1. Create a feature branch from `main`.
2. Ensure invariants are preserved.
3. Add or update tests when behavior changes.
4. Update docs if user-facing behavior changes.

## Commit Messages
Use conventional commits:
```
feat(core): add parser edge case
fix(cli): correct agent error output
docs: update architecture notes
```

## Code Review
Reviewers will check:
- Invariant compliance
- Test coverage
- Documentation updates
- Security implications