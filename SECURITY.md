# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Laminar, please report it responsibly.

**DO NOT** create a public GitHub issue for security vulnerabilities.

### Contact

Email: security@darklightlabs.net

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment:** Within 48 hours
- **Initial Assessment:** Within 7 days
- **Resolution Target:** Within 30 days for critical issues

## Scope

The following are in scope for security reports:

| Component | In Scope |
|-----------|----------|
| `laminar-core` | Yes |
| `laminar-cli` | Yes |
| Laminar Terminal (when released) | Yes |
| Documentation | No (unless it causes security issues) |

## Invariant Violations

Violations of the [Engineering Invariants](INVARIANTS.md) are considered security vulnerabilities:

- **INV-01**: Any code path that could access spending keys
- **INV-02**: Any code path that could sign transactions
- **INV-03**: Any code path that could broadcast to the network
- **INV-06**: Sensitive data stored unencrypted
- **INV-09**: Any telemetry or external network requests

## Bug Bounty

A community "Capture the Flag" bounty will be announced for the Alpha release. Details will be posted in the Zcash forums.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x (when released) | ✓ |
| < 1.0 (pre-release) | Best effort |

## Security Advisories

Security advisories will be published via:
- GitHub Security Advisories
- Zcash Community Forums
- Direct notification to known users
