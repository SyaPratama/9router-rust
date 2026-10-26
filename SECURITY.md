# Security Policy

> **9Router-Rust takes security as its highest priority.**
> Every PR, commit, and code change MUST pass security review before merge. This policy is mandatory and non-negotiable.

---

## Supported Versions

| Version | Status | Security Updates |
|---------|--------|-----------------|
| 1.0.x (Native Rust) | ✅ Active | Full support — patches within SLA |
| < 1.0 (Node.js legacy) | ❌ End of Life | No patches. Migrate immediately. |

Only the latest release on the `1.0.x` branch receives security patches. Users on older builds MUST upgrade.

---

## Reporting a Vulnerability

### ⚠️ NEVER open a public GitHub issue for security vulnerabilities.

Public disclosure before a fix is available puts all users at risk. Violations of responsible disclosure may result in the reporter being banned from the project.

### How to Report

| Channel | Link |
|---------|------|
| **GitHub Security Advisory** (preferred) | [Create private advisory](https://github.com/SyaPratama/9router-rust/security/advisories/new) |
| **Email** | [security@9router.com](mailto:security@9router.com) |

### Required Information

Your report MUST include:

1. **Vulnerability type** — e.g., authentication bypass, injection, credential leak, SSRF
2. **Affected component** — exact file path and function (e.g., `src/api/middleware.rs::security_middleware`)
3. **Reproduction steps** — minimal, deterministic steps to trigger the vulnerability
4. **Proof of concept** — working exploit code, curl commands, or test case
5. **Impact assessment** — what an attacker gains (data access, code execution, privilege escalation)
6. **Affected version(s)** — commit hash or release tag
7. **Environment** — OS, Rust version, Docker version if applicable

Incomplete reports without reproduction steps will be deprioritized.

### Severity Classification

We use [CVSS v3.1](https://www.first.org/cvss/calculator/3.1) for severity scoring:

| Severity | CVSS Score | Response SLA | Fix SLA |
|----------|-----------|-------------|---------|
| 🔴 **Critical** (9.0–10.0) | RCE, auth bypass, credential exfiltration | Acknowledge within **4 hours** | Patch within **72 hours** |
| 🟠 **High** (7.0–8.9) | Privilege escalation, SSRF with internal access, SQL injection | Acknowledge within **24 hours** | Patch within **7 days** |
| 🟡 **Medium** (4.0–6.9) | Information disclosure, IP spoofing, timing attacks | Acknowledge within **48 hours** | Patch within **30 days** |
| 🟢 **Low** (0.1–3.9) | Non-exploitable edge cases, hardening recommendations | Acknowledge within **5 days** | Patch within **90 days** |

### Disclosure Timeline

1. **Day 0**: Report received → acknowledgment sent within SLA above
2. **Day 1–3**: Triage and reproduce → severity assignment and CVE reservation (if applicable)
3. **Day 3–N**: Develop and test fix in private branch
4. **Fix day**: Security release published → advisory disclosed → credit to reporter
5. **Fix +30 days**: Full technical details published (coordinated disclosure)

We follow a strict **maximum 90-day disclosure window**. If we fail to patch within 90 days, the reporter may publish at their discretion.

---

## Security Architecture

### Authentication & Authorization

| Control | Implementation | File |
|---------|---------------|------|
| API key validation | Bearer token and `x-api-key` header checked against `apiKeys` SQLite table | `src/api/middleware.rs` |
| Key deactivation | `isActive` flag enforced at middleware layer — deactivated keys are rejected immediately | `src/api/middleware.rs:58-60` |
| Mandatory key enforcement | `settings.require_api_key` toggle — when enabled, all `/v1/*` endpoints require valid key | `src/api/middleware.rs:64-70` |
| Access restrictions | Per-key `accessRestricted` + `accessAllow` whitelist for model/endpoint scoping | `src/db/schema.rs:62-63` |

### Network Security

| Control | Implementation | File |
|---------|---------------|------|
| Client IP extraction | Derived from raw TCP socket (`ConnectInfo<SocketAddr>`), NOT from HTTP headers | `src/api/middleware.rs:22-26` |
| Anti-IP-spoofing | `X-Forwarded-For` trusted ONLY from loopback connections (`127.0.0.1`, `::1`) | `src/api/middleware.rs:31-41` |
| Peer token fingerprint | SHA-256 hash of client IP + salt for session correlation | `src/auth/mod.rs:4-9` |
| TLS upstream | All upstream provider calls use `rustls-tls` (no OpenSSL, no system cert store) | `Cargo.toml:22` |

### Data Protection

| Control | Implementation | File |
|---------|---------------|------|
| SQLite WAL mode | Write-ahead logging prevents corruption on crashes | `src/db/pool.rs` |
| Busy timeout | Connection pool with retry to prevent lock contention DoS | `src/db/pool.rs` |
| Parameterized queries | ALL SQL uses rusqlite parameter binding (`?1`, `?2`) — zero string concatenation | `src/db/repos/*` |
| Credential storage | Provider API keys stored in `providerConnections.data` JSON field — encrypted at rest is user's responsibility via filesystem encryption | `src/db/schema.rs:18-29` |
| No credential logging | API keys, provider tokens, and auth headers are NEVER written to stdout, stderr, or trace logs | `src/api/middleware.rs`, `src/routing/engine.rs` |

### Headroom Compression Security

| Control | Implementation | File |
|---------|---------------|------|
| Fail-open guarantee | Compression errors return original content unmodified — never corrupt or lose data | `src/headroom/pipeline.rs:92-105` |
| Minimum payload threshold | Payloads under 500 bytes bypass compression entirely (prevents micro-payload oracle attacks) | `src/headroom/pipeline.rs:49-51` |
| Minimum savings threshold | Compressions saving < 5% are discarded (prevents information leakage via compression ratio) | `src/headroom/pipeline.rs:96` |
| CCR store isolation | In-memory LRU keyed by SHA-256 — no cross-request data leakage, cleared on restart | `src/headroom/ccr.rs` |
| No `unsafe` blocks | The entire Headroom engine is 100% safe Rust — zero `unsafe`, zero FFI | `src/headroom/*.rs` |

### Build & Supply Chain

| Control | Implementation |
|---------|---------------|
| Rust memory safety | No manual memory management — ownership, borrowing, and lifetimes enforced at compile time |
| Zero `unsafe` | The codebase contains zero `unsafe` blocks |
| Dependency audit | `cargo audit` MUST pass with zero known vulnerabilities before any release |
| Minimal dependencies | 20 direct dependencies — each vetted and pinned in `Cargo.lock` |
| Reproducible builds | `Cargo.lock` committed — identical binary output from identical source |
| Static linking | Release binary is fully static (musl target in Docker) — no shared library injection |
| No JavaScript runtime | Zero Node.js, Deno, or Bun dependency — eliminates entire class of prototype pollution, eval injection, and supply chain attacks |

---

## Security Scope

### In Scope ✅

All of the following are valid targets for security reports:

- **Authentication bypass** — accessing `/v1/*` endpoints without valid API key when `require_api_key` is enabled
- **Authorization escape** — using a restricted API key to access models/endpoints outside its `accessAllow` list
- **Client IP spoofing** — bypassing the TCP socket IP extraction to inject a forged client IP from a non-loopback address
- **Credential leakage** — provider API keys, OAuth tokens, or user API keys appearing in logs, error responses, SSE streams, or `/api/health` output
- **SQL injection** — any path where user-controlled input reaches SQLite without parameterized binding
- **SSRF** — manipulating provider base URLs or model names to cause the gateway to make requests to internal/unintended hosts
- **Path traversal** — accessing files outside the embedded dashboard assets via crafted `/dashboard/` paths
- **Denial of service** — crashes, panics, OOM, or resource exhaustion triggered by a single malformed request (not volume-based)
- **Deserialization attacks** — crafted JSON payloads causing panics or memory exhaustion in serde_json parsing
- **Header injection** — injecting malicious headers into upstream provider requests via user-controlled fields
- **Response splitting** — injecting headers or content into SSE streams via crafted model responses
- **Information disclosure** — leaking internal state, file paths, stack traces, or database schema via error responses
- **Docker escape** — container breakout or privilege escalation in the published Docker image
- **Dependency vulnerabilities** — known CVEs in any crate listed in `Cargo.lock`

### Out of Scope ❌

- Volumetric DDoS / brute-force attacks (rate limiting is a planned feature, not a vulnerability)
- Vulnerabilities in upstream AI provider APIs (OpenAI, Anthropic, Google, etc.)
- Social engineering attacks against maintainers or users
- Physical access attacks
- Issues requiring a pre-compromised server or root access
- Self-hosted misconfiguration (binding to `0.0.0.0` on a public network without firewall is user responsibility)
- Feature requests disguised as security reports

---

## Security Requirements for Contributors

### Mandatory for ALL Pull Requests

Every PR touching the following areas MUST include a security assessment in the PR description:

1. **`src/api/`** — Authentication, authorization, header handling, input parsing
2. **`src/auth/`** — Credential validation, IP extraction, peer tokens
3. **`src/db/`** — SQL queries, schema changes, data access patterns
4. **`src/routing/`** — Upstream URL construction, request forwarding, credential injection
5. **`src/config/`** — Environment variable handling, file path resolution
6. **`Dockerfile`** — Base image, user privileges, exposed ports, volume mounts

### Prohibited Patterns

The following patterns are **FORBIDDEN** in this codebase and will result in immediate PR rejection:

```
❌ unsafe { }                          // No unsafe Rust
❌ format!("SELECT ... {}", user_input) // No SQL string concatenation
❌ println!("{}", api_key)              // No credential logging
❌ .unwrap() on user input              // No panics on untrusted input
❌ std::process::Command::new(user_val) // No command injection
❌ reqwest::get(user_provided_url)      // No unvalidated SSRF
❌ include_str! on user paths           // No path traversal
❌ eval / shell execution               // No code injection
```

### Required Practices

```
✅ rusqlite parameter binding (?1, ?2)  // ALL SQL queries
✅ .map_err() with sanitized messages   // Error responses hide internals
✅ ConnectInfo<SocketAddr> for IP       // Never trust HTTP headers for IP
✅ is_loopback() before X-Forwarded-For // Anti-spoofing gate
✅ cargo audit                          // Zero known CVEs before merge
✅ cargo clippy -- -D warnings          // Zero warnings policy
✅ #[cfg(test)] for security test cases // Every security fix needs a regression test
```

---

## Hardening Recommendations for Operators

### Production Deployment Checklist

```bash
# 1. Enable mandatory API key authentication
# Set via /api/settings or direct SQLite:
#   require_api_key = true

# 2. Bind to localhost if behind reverse proxy
HOST=127.0.0.1 ./9router start

# 3. Use a reverse proxy (nginx/caddy) for TLS termination
# 9Router does NOT terminate TLS — HTTPS must be handled by your proxy

# 4. Restrict filesystem permissions on data directory
chmod 700 ~/.9router
chmod 600 ~/.9router/db.sqlite

# 5. Run as non-root user (Docker image runs as 'node' user by default)
# Never run the binary as root

# 6. Enable filesystem encryption for credential-at-rest protection
# Provider API keys in SQLite are NOT encrypted by the application

# 7. Monitor logs for auth failures
# Failed auth attempts return HTTP 401 — monitor for brute-force patterns

# 8. Keep updated
cargo install --locked --path .
# Or pull latest Docker image
```

### Environment Variables — Security Impact

| Variable | Default | Security Note |
|----------|---------|---------------|
| `PORT` | `20128` | Non-privileged port. Do NOT use port 80/443 without a reverse proxy. |
| `HOST` | `0.0.0.0` | **⚠️ Binds to all interfaces.** Set to `127.0.0.1` if not behind a firewall. |
| `DATA_DIR` | `~/.9router` | Contains SQLite with provider credentials. Restrict permissions to `700`. |

---

## Security Audit History

| Date | Scope | Auditor | Findings |
|------|-------|---------|----------|
| 2026-10-26 | Initial release review | Internal | Architecture reviewed; zero `unsafe`, parameterized SQL, loopback-only header trust |

---

## Acknowledgments

We gratefully credit security researchers who responsibly disclose vulnerabilities. Credits are published in security advisories and listed here after disclosure:

| Reporter | Vulnerability | Severity | Date |
|----------|--------------|----------|------|
| — | No vulnerabilities reported yet | — | — |

---

## Contact

- **Security reports**: [GitHub Security Advisory](https://github.com/SyaPratama/9router-rust/security/advisories/new) or [security@9router.com](mailto:security@9router.com)
- **General questions**: [GitHub Issues](https://github.com/SyaPratama/9router-rust/issues)
- **PGP key**: Available on request for encrypted communication

---

<sub>This security policy is reviewed and updated with every major release. Last updated: 2026-10-26.</sub>
