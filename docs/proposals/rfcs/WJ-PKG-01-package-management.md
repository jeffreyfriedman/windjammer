# WJ-PKG-01: Package Management & Registry

**Status:** Draft  
**Author:** Windjammer Core Team  
**Created:** 2026-03-21  
**Updated:** 2026-03-21  
**Depends on:** WJ-SEC-01 (capability system), WJ-PERF-01 (economics)

---

## Abstract

This RFC specifies Windjammer's package management system, including package manifest format (`wj.toml`), dependency resolution, registry protocol, publishing workflow, and integration with security/economics features. The design prioritizes **simplicity, security-by-default, and economic efficiency** while supporting the full lifecycle from development to production deployment at AI agent scale.

**Key Principle:** Package management should be invisible until you need it, and powerful when you do.

---

## Table of Contents

1. [Package Manifest (wj.toml)](#package-manifest-wjtoml)
2. [Dependency Resolution](#dependency-resolution)
3. [Lock File Format (.wj-lock)](#lock-file-format-wj-lock)
4. [Registry Protocol](#registry-protocol)
5. [Publishing Workflow](#publishing-workflow)
6. [Security Integration](#security-integration)
7. [Economic Integration](#economic-integration)
8. [CLI Commands](#cli-commands)
9. [Implementation Roadmap](#implementation-roadmap)

---

## Package Manifest (wj.toml)

### Basic Structure

```toml
[package]
name = "my-http-client"
version = "1.2.3"
authors = ["Alice <alice@example.com>"]
edition = "2026"
description = "Fast HTTP client for Windjammer"
license = "MIT"
repository = "https://github.com/alice/my-http-client"
homepage = "https://my-http-client.dev"
documentation = "https://docs.my-http-client.dev"
readme = "README.md"
keywords = ["http", "client", "async", "web"]
categories = ["network-programming", "web-programming"]

[dependencies]
json = "0.12"
regex = "1.10"

[dev-dependencies]
test-helpers = "0.3"

[build-dependencies]
codegen-tool = "2.1"

[app_capabilities]  # NEW: Security integration (WJ-SEC-01)
io = ["net_egress"]
```

### Version Specification (Semver)

**Semver format:** `MAJOR.MINOR.PATCH`

```toml
[dependencies]
# Exact version
json = "=0.12.5"

# Caret (default): Compatible minor/patch updates
json = "^0.12"  # 0.12.x (not 0.13.0)
json = "^1.0"   # 1.x.y (not 2.0.0)

# Tilde: Compatible patch updates only
json = "~0.12.5"  # 0.12.x (not 0.13.0)

# Wildcard
json = "0.*"  # Any 0.x.y

# Range
json = ">=0.12, <2.0"

# Git dependency
http-client = { git = "https://github.com/alice/http-client", branch = "main" }
http-client = { git = "https://github.com/alice/http-client", tag = "v1.2.3" }
http-client = { git = "https://github.com/alice/http-client", rev = "a3b2c1d" }

# Path dependency (local development)
my-lib = { path = "../my-lib" }

# Conditional dependencies (feature flags)
tokio = { version = "1.40", optional = true }

# Platform-specific
[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dependencies]
winapi = "0.3"
```

### Features

```toml
[features]
default = ["json", "compression"]
json = ["dep:serde_json"]
compression = ["dep:flate2"]
full = ["json", "compression", "websockets"]

[[bin]]
name = "my-cli"
path = "src/bin/cli.wj"

[lib]
name = "my_http_client"
path = "src/lib.wj"
crate-type = ["lib"]

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### Economic Configuration (WJ-PERF-01 Integration)

```toml
[economics]
# Optimization profile
profile = "balanced"  # "fast-compile" | "balanced" | "max-perf"

# Target deployment scale
scale = "medium"  # "small" | "medium" | "large" | "massive"

# Budget constraints (optional)
budget = { compilation = "10s", runtime = "p95 < 50ms", memory = "< 512MB" }

# PGO (Profile-Guided Optimization)
pgo = true
pgo_workload = "scripts/workload.wj"
```

### Security Configuration (WJ-SEC-01 Integration)

```toml
[security]
# Security profile
profile = "balanced"  # "paranoid" | "balanced" | "fast"

# Vulnerability severity threshold
vulnerability_threshold = "medium"  # "low" | "medium" | "high" | "critical"

# License policy
allowed_licenses = ["MIT", "Apache-2.0", "BSD-3-Clause"]
review_licenses = ["GPL-3.0"]
forbidden_licenses = ["AGPL-3.0"]

# Audit configuration
audit_mode = "auto"  # "auto" | "manual" | "disabled"

# EOL (End-of-Life) tracking
eol_max_age = "2 years"  # Warn if dependency unmaintained for 2+ years
```

---

## Dependency Resolution

### Resolution Algorithm (PubGrub)

**Windjammer uses the PubGrub algorithm (same as Cargo, Pub, Swift Package Manager):**

```rust
// Simplified PubGrub:
fn resolve(requirements: Vec<Requirement>) -> Result<Solution> {
    let mut solution = Solution::new();
    
    loop {
        match find_conflict(&solution, &requirements) {
            None => return Ok(solution),  // No conflicts!
            Some(conflict) => {
                let resolution = analyze_conflict(conflict);
                apply_resolution(&mut solution, resolution)?;
            }
        }
    }
}
```

**Key properties:**
- **Complete:** Always finds a solution if one exists
- **Deterministic:** Same input → same output
- **Fast:** Typical resolution < 1 second for 1000+ dependencies

### Dependency Graph Example

```
my-app 1.0.0
├─ http-client 2.3.0
│  ├─ json 0.12.5
│  └─ regex 1.10.2
├─ database 4.1.0
│  ├─ json 0.12.5 (deduplicated)
│  └─ connection-pool 1.5.0
└─ logging 0.8.3
```

**Deduplication:** If multiple dependencies require the same package, only one version is included (if compatible).

### Version Resolution Strategy

```toml
# App requires:
[dependencies]
http-client = "^2.0"  # 2.0.0 - 2.9999.9999
database = "^4.0"     # 4.0.0 - 4.9999.9999

# http-client requires:
[dependencies]
json = "^0.12"  # 0.12.0 - 0.12.9999

# database requires:
[dependencies]
json = "^0.12"  # 0.12.0 - 0.12.9999

# Resolution: json 0.12.5 (latest compatible with both)
```

**Conflict example:**

```toml
# App requires:
[dependencies]
http-client = "^2.0"
old-lib = "^1.0"

# http-client requires:
[dependencies]
json = "^0.12"  # 0.12.x

# old-lib requires:
[dependencies]
json = "^0.8"  # 0.8.x

# Resolution: ERROR - no version of json satisfies both
```

**Error message:**

```
error: failed to select a version for `json`

required by `http-client@2.3.0`:
    json = "^0.12"  (requires 0.12.x)

required by `old-lib@1.2.0`:
    json = "^0.8"   (requires 0.8.x)

help: try updating to a newer version of `old-lib` that supports `json@0.12`,
      or consider using an older version of `http-client` that works with `json@0.8`

alternatively, you can specify version overrides in wj.toml:

[patch.crates]
json = "0.12"  # Force version (may break old-lib)
```

---

## Lock File Format (.wj-lock)

### Purpose

**The lock file ensures reproducible builds:**
- Records exact versions of all dependencies (direct + transitive)
- Git-tracked (committed to version control)
- Regenerated on `wj add`, `wj update`
- Used by `wj build` for deterministic builds

### Format (TOML)

```toml
version = "1"
compiler_version = "0.50.0"

[[package]]
name = "my-app"
version = "1.0.0"
dependencies = [
    "http-client 2.3.0 (registry+https://registry.wj-lang.dev/api/v1/crates/http-client/2.3.0)",
    "database 4.1.0 (registry+https://registry.wj-lang.dev/api/v1/crates/database/4.1.0)",
]

[[package]]
name = "http-client"
version = "2.3.0"
source = "registry+https://registry.wj-lang.dev/api/v1/crates"
checksum = "a3b2c1d4e5f6..."
dependencies = [
    "json 0.12.5 (registry+https://registry.wj-lang.dev/api/v1/crates/json/0.12.5)",
    "regex 1.10.2 (registry+https://registry.wj-lang.dev/api/v1/crates/regex/1.10.2)",
]

[[package]]
name = "json"
version = "0.12.5"
source = "registry+https://registry.wj-lang.dev/api/v1/crates"
checksum = "f3a7b8c9d2e1..."

[[package]]
name = "database"
version = "4.1.0"
source = "registry+https://registry.wj-lang.dev/api/v1/crates"
checksum = "1a2b3c4d5e6f..."
dependencies = [
    "json 0.12.5 (registry+https://registry.wj-lang.dev/api/v1/crates/json/0.12.5)",
    "connection-pool 1.5.0 (registry+https://registry.wj-lang.dev/api/v1/crates/connection-pool/1.5.0)",
]

# ... more packages ...

[metadata]
# Dependency graph metadata for tools
```

### Capability Lock File (.wj-capabilities.lock)

**Security integration (WJ-SEC-01):**

```toml
version = "1"

[app]
name = "my-app"
declared_capabilities = ["fs_read", "fs_write", "net_egress"]

[[dependency]]
name = "http-client"
version = "2.3.0"
declared_capabilities = ["net_egress"]
verified_capabilities = ["net_egress"]
allowed_capabilities = ["net_egress"]
trust_score = 9.2

[[dependency]]
name = "json"
version = "0.12.5"
declared_capabilities = []
verified_capabilities = []
allowed_capabilities = []
trust_score = 9.8

# ... more dependencies ...
```

**Key fields:**
- `declared`: What the package claims to need
- `verified`: What code analysis detected
- `allowed`: What the developer approved
- `trust_score`: Community/automated trust score (0-10)

---

## Registry Protocol

### Registry API

**Windjammer registry follows Cargo's HTTP-based protocol:**

```
Registry URL: https://registry.wj-lang.dev

Endpoints:
  GET  /api/v1/crates                      List all packages
  GET  /api/v1/crates/:name                Get package metadata
  GET  /api/v1/crates/:name/:version       Get specific version
  GET  /api/v1/crates/:name/:version/download   Download .wj-crate file
  POST /api/v1/crates/new                  Publish new version (auth required)
  GET  /api/v1/crates/:name/owners         List package owners
  PUT  /api/v1/crates/:name/owners         Add owner (auth required)
  
  GET  /api/v1/security/:name/:version     Security analysis report
  GET  /api/v1/economics/:name/:version    Economic profile
```

### Package Metadata

```json
{
  "name": "http-client",
  "version": "2.3.0",
  "authors": ["Alice <alice@example.com>"],
  "description": "Fast HTTP client for Windjammer",
  "license": "MIT",
  "repository": "https://github.com/alice/http-client",
  "homepage": "https://http-client.dev",
  "documentation": "https://docs.http-client.dev",
  "keywords": ["http", "client", "async"],
  "categories": ["network-programming"],
  "dependencies": [
    {"name": "json", "version": "^0.12"},
    {"name": "regex", "version": "^1.10"}
  ],
  "capabilities": {
    "declared": ["net_egress"],
    "verified": ["net_egress"],
    "trust_score": 9.2
  },
  "economics": {
    "compile_time_avg": "3.2s",
    "binary_size_avg": "1.2 MB",
    "runtime_perf": "p95: 45ms"
  },
  "downloads": 1234567,
  "created_at": "2025-03-15T12:00:00Z",
  "updated_at": "2026-01-10T08:30:00Z",
  "checksum": "sha256:a3b2c1d4e5f6...",
  "yanked": false
}
```

### Package File Format (.wj-crate)

**A `.wj-crate` file is a gzipped tarball:**

```
http-client-2.3.0.wj-crate
├─ wj.toml              (manifest)
├─ .wj-capabilities     (capability manifest)
├─ README.md
├─ LICENSE
├─ CHANGELOG.md
├─ src/
│  ├─ lib.wj
│  ├─ client.wj
│  └─ request.wj
├─ tests/
│  └─ integration_test.wj
└─ .wj-crate-metadata   (registry metadata)
```

**Build process:**

```bash
wj package

Creating http-client-2.3.0.wj-crate...
  ├─ Analyzing capabilities...
  ├─ Running security checks...
  ├─ Signing with Sigstore...
  ├─ Generating SBOM...
  └─ Compressing...

✅ Package created: http-client-2.3.0.wj-crate (1.2 MB)

Verify: wj package verify http-client-2.3.0.wj-crate
```

### Private Registries

```toml
# .wj/config.toml (global config)
[registries]
default = "https://registry.wj-lang.dev"

[registries.company]
url = "https://registry.company.internal"
token_env = "COMPANY_REGISTRY_TOKEN"

# In project wj.toml:
[dependencies]
public-lib = "1.0"
company-lib = { version = "2.3", registry = "company" }
```

### Registry Authentication

```bash
# Login to registry
wj registry login

Registry URL [https://registry.wj-lang.dev]: https://registry.company.internal
Username: alice
Password: ********

✅ Logged in to https://registry.company.internal
Token saved to ~/.wj/credentials.toml

# Or use token directly
export WJ_REGISTRY_TOKEN="abc123..."
wj publish
```

---

## Publishing Workflow

### Step-by-Step

**1. Prepare package:**

```bash
cd my-http-client/

# Ensure tests pass
wj test

# Security audit
wj security audit

# Economic analysis
wj economics report

# Lint
wj lint
```

**2. Package:**

```bash
wj package

Creating http-client-2.3.0.wj-crate...

Running pre-publish checks:
  ✅ wj.toml valid
  ✅ All tests passing
  ✅ Security audit clean
  ✅ No uncommitted changes
  ✅ Version 2.3.0 not yet published
  ✅ License file present (MIT)
  ✅ README.md present

Analyzing capabilities...
  Declared: net_egress
  Verified: net_egress
  ✅ Match!

Signing package (Sigstore)...
  ✅ Signed with keyless signature
  ✅ Certificate: https://rekor.sigstore.dev/api/v1/log/entries/abc123

Generating SBOM...
  ✅ CycloneDX SBOM: sbom.json
  ✅ SPDX SBOM: sbom.spdx.json

✅ Package ready: http-client-2.3.0.wj-crate (1.2 MB)
```

**3. Dry-run publish:**

```bash
wj publish --dry-run

Publishing http-client 2.3.0 to https://registry.wj-lang.dev

Pre-publish validation:
  ✅ Authentication valid
  ✅ Package name available
  ✅ Version 2.3.0 not yet published
  ✅ All dependencies exist in registry
  ✅ Capability manifest valid
  ✅ Signature valid
  ✅ SBOM present

Registry checks:
  ✅ No supply chain attacks detected
  ✅ Trust score: 9.2/10
  ✅ License compatible: MIT
  ✅ No known vulnerabilities

Economic profile:
  Compile time: 3.2s (avg)
  Binary size: 1.2 MB (avg)
  Runtime: p95 < 50ms

--dry-run complete. Run without --dry-run to publish.
```

**4. Publish:**

```bash
wj publish

Publishing http-client 2.3.0 to https://registry.wj-lang.dev

Uploading package (1.2 MB)...
████████████████████████████████████████ 100%

Registry processing...
  ├─ Extracting package...
  ├─ Running security analysis...
  ├─ Verifying signature...
  ├─> Generating documentation...
  └─ Indexing...

✅ Published http-client 2.3.0

View at: https://registry.wj-lang.dev/crates/http-client/2.3.0
Docs at: https://docs.wj-lang.dev/http-client/2.3.0

Installed in ~30 seconds (CDN propagation).

Economics:
  - 0 existing users (new package)
  - Estimated reach: 10K downloads/month
  - Community impact: $15K/month saved (if widely adopted)
```

### Versioning Best Practices

**Semantic versioning rules:**

```
MAJOR.MINOR.PATCH

MAJOR: Breaking changes (incompatible API)
MINOR: New features (backward compatible)
PATCH: Bug fixes (backward compatible)
```

**Examples:**

```
1.2.3 → 1.2.4  (bug fix)
1.2.3 → 1.3.0  (new feature)
1.2.3 → 2.0.0  (breaking change)
```

**Pre-releases:**

```
1.0.0-alpha.1
1.0.0-beta.2
1.0.0-rc.1
1.0.0  (stable release)
```

**Windjammer convention:**

```toml
[dependencies]
# Stable packages (1.0+): Use caret
json = "^1.2"

# Pre-1.0 packages: Pin exact version
experimental = "=0.3.2"
```

---

## Security Integration (WJ-SEC-01)

### Capability Declaration

**Every package declares its capabilities in `wj.toml`:**

```toml
[package]
name = "http-client"
version = "2.3.0"

[app_capabilities]
io = ["net_egress"]  # This package needs network access
```

**On `wj add http-client`:**

```bash
wj add http-client

Resolving dependencies...
  ✅ http-client 2.3.0

Security analysis:
  Package: http-client 2.3.0
  Declared capabilities: net_egress
  Verified capabilities: net_egress ✅
  Trust score: 9.2/10 ✅
  
  Code analysis:
    - Public API: 23 functions (all network-related) ✅
    - Data flow: No suspicious file access ✅
    - Anomaly score: 0.1 (low) ✅
  
  Community signals:
    - Downloads: 1.2M
    - Age: 2 years
    - Maintainers: 3 active
    - Security audits: 2 (passed)
  
Allow net_egress for http-client? [Y/n] Y

✅ Added http-client 2.3.0
✅ Updated .wj-capabilities.lock
```

### Vulnerability Scanning

**On every `wj build`:**

```bash
wj build --release

Checking dependencies for vulnerabilities...

Scanning 47 dependencies...
  ✅ 45 packages clean
  ⚠️  2 packages have advisories

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚠️  SECURITY ADVISORY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Package: regex 1.9.0
Severity: MEDIUM
Advisory: RUSTSEC-2024-0001
Description: ReDoS vulnerability in complex patterns

Affected: regex 1.9.0 - 1.10.1
Fixed: regex 1.10.2

Fix: wj update regex

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Continue build? [y/N] n

Build cancelled. Fix vulnerabilities first:
  wj update regex
```

### Supply Chain Attack Detection

**Automated checks on `wj add`:**

```bash
wj add suspicious-lib

Resolving dependencies...
  ✅ suspicious-lib 0.1.0

Security analysis:
  Package: suspicious-lib 0.1.0
  Trust score: 3.2/10 ⚠️  LOW
  
  ⚠️  WARNING: Supply chain risks detected
  
  Issues:
    1. Typosquatting risk (85% similar to 'popular-lib')
    2. New package (<30 days old)
    3. No repository URL
    4. Excessive capabilities (fs_read, net_egress, process_spawn)
    5. Obfuscated code detected (base64 strings, eval-like patterns)
  
  Recommendation: DO NOT USE
  
  Did you mean 'popular-lib' instead? [Y/n] Y
  
✅ Adding popular-lib 2.3.0 instead
```

---

## Economic Integration (WJ-PERF-01)

### Dependency Cost Analysis

```bash
wj economics deps

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💰 Dependency Economics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total dependencies: 47

Cost breakdown:

Compilation time:
  Total: 12.3s
  Top 5 slowest:
    1. tokio 1.40.0        (2.1s, 17%)
    2. regex 1.10.2        (1.3s, 11%)
    3. serde 1.0.200       (0.9s, 7%)
    4. hyper 1.4.0         (0.8s, 7%)
    5. serde_json 1.0.120  (0.7s, 6%)

Binary size:
  Total: 8.4 MB
  Top 5 largest:
    1. tokio 1.40.0        (2.1 MB, 25%)
    2. hyper 1.4.0         (1.2 MB, 14%)
    3. regex 1.10.2        (0.9 MB, 11%)
    4. rustls 0.23.0       (0.7 MB, 8%)
    5. serde_json 1.0.120  (0.5 MB, 6%)

Runtime cost (estimated):
  At 1M req/day:
    CPU: $23.40/month
    Memory: $12.80/month
    Total: $36.20/month

Optimization opportunities:
  1. Replace tokio with lightweight async runtime (-1.2 MB, -$5/month)
  2. Use binary JSON format instead of serde_json (-0.3 MB, -$2/month)
  
  Potential savings: $7/month = $84/year per instance

Apply: wj optimize --deps
```

### Dependency Optimization

```bash
wj optimize --deps

Analyzing dependency usage...

[1/3] tokio 1.40.0
      Usage: 23 functions, 12 macros
      Features used: runtime-multi-thread, io-util, net
      Features unused: process, signal, sync (12 more)
      
      Optimization: Enable only used features
      
      Before:
        [dependencies]
        tokio = { version = "1.40", features = ["full"] }
      
      After:
        [dependencies]
        tokio = { version = "1.40", features = ["rt-multi-thread", "io-util", "net"] }
      
      Savings: -1.2 MB binary, -0.8s compile time
      
      Apply? [Y/n] Y
      ✅ Applied

[2/3] regex 1.10.2
      Usage: 3 patterns (all simple)
      
      Optimization: Consider string methods instead
      
      Code:
        let re = Regex::new(r"^\d{3}-\d{3}-\d{4}$").unwrap();
        re.is_match(phone)
      
      Alternative:
        phone.len() == 12 && phone.chars().nth(3) == Some('-') && ...
      
      Savings: -0.9 MB binary (remove regex dependency)
      Impact: Manual pattern validation (less maintainable)
      
      Recommendation: Keep regex (small complexity benefit)
      
      Apply? [y/N] N
      ⏭️  Skipped

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Optimization complete!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total savings:
  Binary size: -1.2 MB (-14%)
  Compile time: -0.8s (-7%)
  Runtime cost: -$5/month (-14%)

Updated: wj.toml
Rebuild: wj build --release
```

---

## CLI Commands

### Core Commands

```bash
# Initialize new package
wj init my-package
wj init --lib my-library
wj init --bin my-cli-tool

# Add dependency
wj add json
wj add "json@^0.12"
wj add json --features full
wj add json --dev         # Dev dependency
wj add json --build       # Build dependency

# Remove dependency
wj remove json

# Update dependencies
wj update                 # Update all (respect semver)
wj update json            # Update specific package
wj update --breaking      # Allow major version updates

# List dependencies
wj list
wj list --tree            # Show dependency tree
wj list --outdated        # Show packages with updates

# Search registry
wj search http
wj search --category network-programming

# Show package info
wj info json
wj info json@0.12.5

# Build
wj build
wj build --release

# Test
wj test
wj test --package json

# Package management
wj package                # Create .wj-crate
wj publish                # Publish to registry
wj publish --dry-run      # Test publish without uploading
wj yank 1.2.3             # Remove version from registry (doesn't delete)
wj unyank 1.2.3           # Restore yanked version

# Registry management
wj registry login
wj registry logout
wj registry list          # List configured registries

# Security
wj security audit         # Check vulnerabilities
wj security scan          # Analyze capabilities

# Economics
wj economics deps         # Analyze dependency costs
wj optimize --deps        # Optimize dependencies
```

### Advanced Commands

```bash
# Dependency graph visualization
wj deps graph
wj deps graph --format=svg > graph.svg
wj deps why json          # Why is json included?

# Lock file management
wj lock update            # Regenerate lock file
wj lock check             # Verify lock file integrity

# Workspace commands (monorepo)
wj workspace list
wj workspace run test     # Run command in all packages

# Cache management
wj cache clean            # Clear download cache
wj cache size             # Show cache size
wj cache list             # List cached packages

# Vendor dependencies (offline builds)
wj vendor                 # Download all deps to vendor/
wj build --offline        # Use vendored dependencies
```

---

## Monorepo Support (Workspaces)

### Workspace Structure

```
my-monorepo/
├─ wj.toml              (root manifest)
├─ packages/
│  ├─ core/
│  │  └─ wj.toml
│  ├─ cli/
│  │  └─ wj.toml
│  └─ web/
│     └─ wj.toml
└─ shared/
   └─ utils/
      └─ wj.toml
```

### Root Manifest

```toml
[workspace]
members = [
    "packages/core",
    "packages/cli",
    "packages/web",
    "shared/utils",
]

# Shared dependencies (all workspace members use same versions)
[workspace.dependencies]
json = "0.12"
regex = "1.10"

[workspace.package]
authors = ["Alice <alice@example.com>"]
edition = "2026"
license = "MIT"
repository = "https://github.com/alice/my-monorepo"
```

### Package Manifest (Workspace Member)

```toml
[package]
name = "my-core"
version = "1.0.0"
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
# Use workspace-defined version
json.workspace = true

# Local workspace dependency
my-utils = { path = "../../shared/utils" }

# Package-specific dependency
specific-lib = "2.3"
```

### Workspace Commands

```bash
# Build all packages
wj build --workspace

# Test all packages
wj test --workspace

# Run command in specific package
wj run --package my-cli

# Publish all packages (in dependency order)
wj publish --workspace

# Update dependencies across workspace
wj update --workspace
```

---

## Implementation Roadmap

### Phase 1: Core Package Management (v0.50)

**Week 1-2: Manifest Parsing**
- Implement `wj.toml` parser
- Support basic dependencies, features, profiles
- Validate semver versions

**Week 3-4: Dependency Resolution**
- Implement PubGrub algorithm
- Generate `.wj-lock` file
- Handle version conflicts

**Week 5-6: Registry Client**
- HTTP client for registry API
- Download and verify packages
- Authenticate with registry

**Estimated effort:** 6 weeks

### Phase 2: Publishing (v0.51)

**Week 7-8: Package Creation**
- `wj package` command
- Create `.wj-crate` tarball
- Capability analysis integration

**Week 9-10: Publishing**
- `wj publish` command
- Upload to registry
- Sigstore signing integration

**Estimated effort:** 4 weeks

### Phase 3: Advanced Features (v0.52)

**Week 11-12: Workspaces**
- Multi-package projects
- Shared dependencies
- Cross-package building

**Week 13-14: Private Registries**
- Custom registry support
- Authentication
- Mirror/proxy registries

**Estimated effort:** 4 weeks

### Phase 4: Integration (v0.53)

**Week 15-16: Security Integration (WJ-SEC-01)**
- Capability lock file
- Vulnerability scanning
- Supply chain detection

**Week 17-18: Economic Integration (WJ-PERF-01)**
- Dependency cost analysis
- Optimization suggestions
- Economic reporting

**Estimated effort:** 4 weeks

**Total estimate:** 18 weeks (~4.5 months)

---

## Comparison with Other Package Managers

### vs. Cargo (Rust)

| Feature | Cargo | Windjammer | Winner |
|---------|-------|------------|--------|
| **Manifest format** | TOML | TOML | TIE |
| **Resolution algorithm** | PubGrub | PubGrub | TIE |
| **Lock file** | Cargo.lock | .wj-lock | TIE |
| **Security scanning** | cargo-audit (external) | Built-in | Windjammer ✅ |
| **Capability system** | None | Native (WJ-SEC-01) | Windjammer ✅ |
| **Economic analysis** | None | Native (WJ-PERF-01) | Windjammer ✅ |
| **Supply chain detection** | None | Native | Windjammer ✅ |

### vs. npm (Node.js)

| Feature | npm | Windjammer | Winner |
|---------|-----|------------|--------|
| **Manifest format** | JSON | TOML | Windjammer (more human-friendly) |
| **Resolution algorithm** | Tree-based | PubGrub | Windjammer (deterministic) |
| **Lock file** | package-lock.json | .wj-lock | TIE |
| **Security** | npm audit | Built-in + capability system | Windjammer ✅ |
| **Workspace support** | YES | YES | TIE |
| **Disk usage** | High (node_modules hell) | Low (global cache) | Windjammer ✅ |

### vs. Go Modules

| Feature | Go Modules | Windjammer | Winner |
|---------|-----------|------------|--------|
| **Manifest format** | go.mod | wj.toml | TIE |
| **Resolution** | Minimal version selection | PubGrub | Depends (both have merits) |
| **Lock file** | go.sum (checksums) | .wj-lock (full graph) | Windjammer (more info) |
| **Security** | None built-in | Native | Windjammer ✅ |
| **Versioning** | Git tags | Semver | TIE |

---

## Success Criteria

**Package management is successful if:**

1. ✅ `wj add <package>` completes in < 5 seconds (avg)
2. ✅ Resolution deterministic (same input → same output)
3. ✅ 99% of published packages have valid capability manifests
4. ✅ Security scanning detects 95%+ of known vulnerabilities
5. ✅ Economic analysis accurate within ±10%
6. ✅ Developer satisfaction: "Easier than Cargo, safer than npm"

---

## References

- **PubGrub algorithm:** https://github.com/dart-lang/pub/blob/master/doc/solver.md
- **Cargo manifest:** https://doc.rust-lang.org/cargo/reference/manifest.html
- **Semantic versioning:** https://semver.org
- **WJ-SEC-01:** Inferred Effect Capabilities
- **WJ-PERF-01:** Economic Efficiency Framework

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-03-21 | Use PubGrub for resolution | Proven, deterministic, complete |
| 2026-03-21 | TOML for manifest | Human-friendly, widely adopted |
| 2026-03-21 | Native security integration | Critical for AI agent scale |
| 2026-03-21 | Native economic analysis | Differentiation, cost-conscious scale |

---

**Status:** Ready for Implementation  
**Target Version:** v0.50  
**Depends on:** WJ-SEC-01 (security), WJ-PERF-01 (economics)  
**Implementation Estimate:** 18 weeks (4.5 months)
