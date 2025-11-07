# Security Policy

## Supported Versions

Currently supported versions for security updates:

| Version | Supported          |
| ------- | ------------------ |
| 2.1.x   | :white_check_mark: |
| 2.0.x   | :white_check_mark: |
| < 2.0   | :x:                |

## Reporting a Vulnerability

### Where to Report

**DO NOT** create public GitHub issues for security vulnerabilities.

Please report security vulnerabilities privately through one of these channels:

1. **GitHub Security Advisories** (preferred):
   - Go to the repository's Security tab
   - Click "Report a vulnerability"
   - Fill out the form with details

2. **Email**: Send details to the maintainers (see README for contacts)

### What to Include

Please include as much of the following information as possible:

- **Vulnerability Type**: (e.g., memory leak, code injection, authentication bypass)
- **Affected Component**: (e.g., storage layer, API server, orchestration)
- **Impact Assessment**: Stability, Safety, or Security concern
- **Steps to Reproduce**: Detailed reproduction steps
- **Proof of Concept**: Code, commands, or screenshots
- **Suggested Fix**: If you have a patch or mitigation strategy
- **Disclosure Timeline**: Your preferred disclosure date

### Response Timeline

- **Initial Response**: Within 48 hours of report
- **Triage**: Within 7 days (severity assessment, scope)
- **Fix Development**: 30-90 days depending on severity
- **Public Disclosure**: Coordinated with reporter after fix

### Severity Levels

| Level | Criteria | Response Time |
|-------|----------|---------------|
| **Critical** | Remote code execution, data loss, complete system compromise | 7 days |
| **High** | Authentication bypass, privilege escalation, memory corruption | 14 days |
| **Medium** | Information disclosure, DoS, memory leaks | 30 days |
| **Low** | Minor information leaks, edge case crashes | 90 days |

## Security Best Practices

### For Users

1. **Keep mnemosyne updated**: Always use the latest stable version
2. **Protect secrets**: Never commit `.age` files or API keys to version control
3. **Use namespaces**: Isolate project memories to prevent cross-contamination
4. **Monitor resources**: Set up alerts for memory/CPU usage
5. **Review logs**: Check for suspicious activity or errors

### For Contributors

1. **Never hardcode secrets**: Use environment variables or encrypted storage
2. **Validate all inputs**: Sanitize file paths, command arguments, user input
3. **Use parameterized queries**: Never concatenate SQL strings
4. **Avoid unsafe code**: Document and justify any `unsafe` blocks
5. **Handle errors properly**: No `.unwrap()` in production code paths
6. **Test security**: Add tests for authentication, authorization, input validation

## Known Vulnerabilities

### Disclosed Vulnerabilities

None publicly disclosed at this time.

### Current Audit Status

**Active Audit (2025-11-07)**: Comprehensive stability, safety, and security audit in progress.

- **Phase 1 (In Progress)**: Stability - Memory leaks, resource exhaustion (Exit code 143)
- **Phase 2 (Planned)**: Safety - Unsafe code, panic points, concurrency
- **Phase 3 (Planned)**: Security - API hardening, input validation, secrets management
- **Phase 4 (Planned)**: Observability - Monitoring, testing, documentation

See `docs/security/AUDIT_2025_PHASE1.md` for current findings.

## Acknowledgments

We appreciate responsible disclosure and will acknowledge security researchers in:

- Release notes for the fixed version
- Security advisories (with permission)
- This document's Hall of Fame (with permission)

### Hall of Fame

_No public disclosures yet._

## Security Features

### Current Protections

✅ **Secrets Management**:
- Age encryption (x25519) for API keys at rest
- File permissions: 0600 (owner-only)
- Environment variable priority (prevents file leaks)
- No hardcoded credentials

✅ **SQL Injection Protection**:
- Parameterized queries throughout
- No string concatenation in SQL construction

✅ **Authentication**:
- HMAC-SHA256 for cross-process authentication
- Cryptographic signatures for multi-instance coordination

✅ **Network Security**:
- TLS for all remote connections (rustls-tls)
- Localhost-only API server binding (127.0.0.1:3000)

### Known Gaps (Being Addressed)

⚠️ **API Security**:
- Permissive CORS (`CorsLayer::permissive()`)
- No authentication on state-changing endpoints
- No rate limiting

⚠️ **Input Validation**:
- Path traversal risks in file operations
- Command injection risks in git operations
- Namespace validation gaps

⚠️ **Memory Safety**:
- Unbounded event broadcaster
- No database connection pooling
- Spawned task leaks

See active audit documentation for remediation timeline.

## Third-Party Dependencies

We rely on well-maintained crates and monitor for security advisories:

- **Critical Dependencies**: libsql, tokio, axum, PyO3
- **Security Tools**: age, hmac, sha2
- **Audit Process**: `cargo audit` in CI/CD (planned)

## Contact

For security concerns, please contact the maintainers (see README for contact information).

For general questions, use GitHub Discussions or Issues.

---
**Last Updated**: 2025-11-07
**Next Review**: After Phase 3 completion
