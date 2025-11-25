# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.36.x  | :white_check_mark: |
| 0.35.x  | :white_check_mark: |
| < 0.35  | :x:                |

## Reporting a Vulnerability

We take the security of Windjammer seriously. If you believe you have found a security vulnerability, please report it to us responsibly.

### Please Do Not

- Open a public GitHub issue for security vulnerabilities
- Disclose the vulnerability publicly before it has been addressed

### Please Do

1. **Email us directly** at [INSERT SECURITY EMAIL]
2. **Provide details** including:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)
3. **Allow time** for us to address the issue before public disclosure

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Assessment**: We will assess the vulnerability and determine its severity
- **Fix**: We will work on a fix and coordinate a release timeline with you
- **Credit**: We will credit you in the security advisory (unless you prefer to remain anonymous)
- **Disclosure**: We will publicly disclose the vulnerability after a fix is released

## Security Best Practices

When using Windjammer:

1. **Keep updated**: Always use the latest stable version
2. **Review dependencies**: Regularly audit your project's dependencies
3. **Validate input**: Sanitize user input in your Windjammer applications
4. **Follow guidelines**: Adhere to secure coding practices
5. **Report issues**: Help us identify and fix security issues

## Security Features

Windjammer includes several security features:

- **Memory safety**: Leverages Rust's memory safety guarantees
- **Type safety**: Strong static typing prevents many common vulnerabilities
- **Dependency scanning**: Automated dependency vulnerability checks via Dependabot
- **CI/CD security**: Automated security checks in our build pipeline

## Vulnerability Disclosure Timeline

Our typical vulnerability disclosure timeline:

1. **Day 0**: Vulnerability reported
2. **Day 1-2**: Acknowledgment sent
3. **Day 3-7**: Assessment and severity determination
4. **Day 8-30**: Fix development and testing
5. **Day 31**: Coordinated disclosure and patch release

## Security Updates

Security updates are released as:

- **Patch releases** (0.36.x) for minor vulnerabilities
- **Minor releases** (0.x.0) for moderate vulnerabilities
- **Immediate hotfixes** for critical vulnerabilities

Subscribe to our [GitHub releases](https://github.com/jeffreyfriedman/windjammer/releases) to stay informed about security updates.

## Bug Bounty Program

We currently do not have a formal bug bounty program, but we deeply appreciate security researchers who responsibly disclose vulnerabilities. We will publicly acknowledge your contribution in our security advisories and release notes.

## Questions?

If you have questions about our security policy, please open a [GitHub Discussion](https://github.com/jeffreyfriedman/windjammer/discussions).

---

**Last Updated**: November 25, 2024
