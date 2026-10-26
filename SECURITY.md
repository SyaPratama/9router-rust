# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x (Rust) | ✅ |
| < 1.0 (Node.js legacy) | ❌ |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Please report security issues privately via:

- **Email**: [security@9router.com](mailto:security@9router.com)
- **GitHub Security Advisories**: [Report a vulnerability](https://github.com/SyaPratama/9router-rust/security/advisories/new)

### What to include

- Description of the vulnerability
- Steps to reproduce
- Affected version(s)
- Impact assessment (if known)

### Response timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 5 business days
- **Fix or mitigation**: Targeting 30 days for critical, 90 days for non-critical

### Scope

The following areas are in scope for security reports:

- Authentication bypass (Bearer / `x-api-key` validation)
- Client IP spoofing (TCP socket derivation, `X-Forwarded-For` handling)
- Credential leakage (API keys, provider tokens in logs or responses)
- SQLite injection via API inputs
- SSRF via upstream provider URL handling
- Memory safety issues in Rust code (unsafe blocks, FFI)
- Docker container escape or privilege escalation

### Out of scope

- Denial of service via high request volume (rate limiting is not yet implemented)
- Issues in upstream AI provider APIs
- Social engineering
