# Contributing to Laminar

Thank you for your interest in contributing to Laminar.

## Before You Start

1. **Read the Invariants:** Every contribution must comply with [INVARIANTS.md](INVARIANTS.md). These are non-negotiable.

2. **Understand the Architecture:** Review [ARCHITECTURE.md](ARCHITECTURE.md) to understand system design.

3. **Check the Scope:** Review [RFC-001.md](RFC-001.md) to understand what's in scope for Phase 1.

## Development Setup

### Prerequisites

- Rust (stable, latest)
- Cargo

### Building

```bash
# Clone the repository
git clone https://github.com/darklight-labs/Laminar.git
cd Laminar

# Build the workspace
cargo build

# Run tests
cargo test

# Run the CLI
cargo run -p laminar-cli -- info
```

### Code Style

- Follow `rustfmt` defaults: `cargo fmt`
- Pass `clippy` with no warnings: `cargo clippy -- -D warnings`
- No floating-point arithmetic on monetary values (INV-04)
- All errors via `Result<T, LaminarError>`—no panics in library code

## Pull Request Process

1. **Fork and Branch:** Create a feature branch from `main`

2. **Invariant Checklist:** Include this in your PR description:
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

3. **Tests:** Add tests for new functionality. Maintain >80% coverage for `laminar-core`.

4. **Documentation:** Update relevant docs if behavior changes.

5. **CI Green:** All checks must pass before merge.

## Commit Messages

Use conventional commits:

```
feat(core): add ZIP-321 construction logic
fix(cli): handle UTF-8 BOM in CSV files
docs: update ARCHITECTURE.md with new module
test: add property tests for zatoshi parsing
```

## Issue Reporting

- **Bugs:** Include reproduction steps, expected vs actual behavior
- **Features:** Reference the roadmap (RFC-001) and explain alignment
- **Security:** See [SECURITY.md](SECURITY.md)—do not create public issues

## Code Review

All PRs require review. Reviewers will check:

1. Invariant compliance
2. Test coverage
3. Documentation updates
4. Code style
5. Security implications

## License

By contributing, you agree that your contributions will be licensed under the MIT and Apache 2.0 licenses.
