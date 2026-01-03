# Security Policy

## Supported Versions

We release security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.39.x  | :white_check_mark: |
| 0.38.x  | :white_check_mark: |
| < 0.38  | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

### 1. GitHub Security Advisories (Preferred)

Report vulnerabilities through [GitHub Security Advisories](https://github.com/jeffreyfriedman/windjammer/security/advisories/new).

### 2. Email

Send an email to: **security@windjammer.dev** (if available)

Or contact the maintainer directly: **jeffrey@example.com**

### What to Include

Please include the following information:

- Type of vulnerability (e.g., buffer overflow, SQL injection, cross-site scripting, etc.)
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity
  - **Critical**: Within 7 days
  - **High**: Within 14 days
  - **Medium**: Within 30 days
  - **Low**: Next regular release

## Security Measures

### Automated Scanning

- **CodeQL**: Automated security scanning on every push and pull request
- **Dependabot**: Daily checks for vulnerable dependencies
- **cargo-audit**: Security audit in pre-commit hook and CI

### Dependency Management

- All dependencies are tracked in `Cargo.lock`
- Dependabot automatically creates PRs for security updates
- Security advisories are reviewed weekly
- Unmaintained dependencies are monitored and replaced when alternatives exist

### Code Review

- All code changes require review before merging
- Security-sensitive changes require additional review
- Pre-commit hooks run security checks locally

### Release Process

- Security patches are released as patch versions (0.x.Y)
- Security advisories are published after fixes are released
- Users are notified through GitHub releases and CHANGELOG

## Known Security Considerations

### 1. Subprocess Execution

Windjammer spawns subprocesses for compilation. When using Windjammer:

- **Do NOT compile untrusted code** without sandboxing
- Consider running in a container or VM for untrusted sources
- File system access is unrestricted during compilation

### 2. Generated Code

Windjammer generates Rust code that is then compiled by `rustc`:

- Generated code may contain bugs (compiler is in active development)
- Always review generated code for production use
- Use `--no-cargo` flag to inspect generated code before running

### 3. LSP Server

The Language Server Protocol (LSP) server:

- Runs with user privileges
- Has file system access
- Should only be used with trusted projects

### 4. Dependency Supply Chain

Windjammer depends on ~400+ crates:

- All dependencies are audited with `cargo-audit`
- Unmaintained dependencies are monitored
- Consider vendoring dependencies for production use

## Security Best Practices for Users

### For Development

1. **Keep Windjammer Updated**: Always use the latest version
2. **Run `cargo audit`**: Check your project dependencies
3. **Review Generated Code**: Use `--no-cargo` flag to inspect output
4. **Use Pre-commit Hooks**: Enable security checks before commits

### For Production

1. **Vendor Dependencies**: Use `cargo vendor` for reproducible builds
2. **Pin Versions**: Lock dependency versions in `Cargo.lock`
3. **Scan Container Images**: If using Docker, scan with tools like Trivy
4. **Sandbox Compilation**: Run Windjammer in isolated environments
5. **Review Generated Code**: Never blindly trust compiler output

### For CI/CD

1. **Use Official Images**: Pull from verified GitHub Container Registry
2. **Scan Dependencies**: Run `cargo audit` in CI pipeline
3. **SBOM Generation**: Generate Software Bill of Materials
4. **Sign Artifacts**: Use checksums and signatures for releases

## Dependency Audit Results

Last audit: 2026-01-01

### Current Warnings (Non-Critical)

1. **paste** (v1.0.15) - Unmaintained
   - Used by: `ratatui` (transitive dependency)
   - Severity: Low
   - Status: Monitoring, no replacement needed yet
   - Impact: Procedural macro, no runtime security impact

2. **yaml-rust** (v0.4.5) - Unmaintained
   - Used by: `syntect` (transitive dependency)
   - Severity: Low
   - Status: Monitoring, `syntect` team aware
   - Impact: Syntax highlighting only, no user input parsing

**No critical vulnerabilities found.**

## Security Tooling

### Tools We Use

- **cargo-audit**: Dependency vulnerability scanner
- **CodeQL**: Static analysis security testing (SAST)
- **Dependabot**: Automated dependency updates
- **Clippy**: Rust linter (catches common bugs)
- **cargo-deny**: License and security policy enforcement (future)

### Recommended for Users

- **cargo-audit**: Scan your projects
- **cargo-outdated**: Check for outdated dependencies
- **cargo-tree**: Inspect dependency tree
- **cargo-geiger**: Detect unsafe code usage

## Security Disclosure Policy

### Coordinated Disclosure

We follow **coordinated disclosure**:

1. Reporter notifies maintainers privately
2. Maintainers confirm and develop fix
3. Fix is released
4. Security advisory is published
5. CVE is assigned (if applicable)

### Public Disclosure

Security advisories will be published:

- In GitHub Security Advisories
- In CHANGELOG.md (after fix is released)
- In release notes
- On security mailing lists (if established)

## Security Hall of Fame

We recognize security researchers who responsibly disclose vulnerabilities:

- *No vulnerabilities reported yet*

## Questions?

For security-related questions that are not vulnerabilities:

- Open a [GitHub Discussion](https://github.com/jeffreyfriedman/windjammer/discussions)
- Email the maintainer
- Join our community chat (if available)

---

**Last Updated**: 2026-01-01  
**Version**: 0.39.1  
**Next Review**: 2026-02-01
