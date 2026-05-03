# Transparent Browser Abstractions - Design Document

## Vision: Write Once, Run Anywhere (For Real)

**Goal**: Users write pure Windjammer code using standard APIs. The compiler automatically generates platform-appropriate implementations, including Web Workers, fetch() calls, and backend proxies where needed.

---

## Philosophy

### Current State (Good)
```windjammer
use std::process::*

// User writes this
let result = process::execute("ls", vec!["-la"])

// WASM: Returns error message
// Native: Actually executes
```

**Problem**: User gets an error and has to handle it manually.

### Desired State (Better)
```windjammer
use std::process::*

// User writes THE SAME CODE
let result = process::execute("ls", vec!["-la"])

// WASM: Automatically proxies to backend API
// Native: Executes directly
// User doesn't need to know!
```

---

## Design: Three-Tier Abstraction

### Tier 1: Pure Computation (No Backend Needed)
**Use Case**: CPU-intensive tasks that don't need OS access

```windjammer
use std::compute::*

// User writes
let result = compute::parallel(data, |item| {
    // Heavy computation
    item * item
})

// Native: Uses rayon for parallel processing
// WASM: Automatically spawns Web Workers
// User doesn't care!
```

**Compiler generates**:
```rust
// Native
use rayon::prelude::*;
data.par_iter().map(|item| item * item).collect()

// WASM
use web_sys::Worker;
// Spawn workers, distribute work, collect results
```

### Tier 2: Network Operations (Transparent)
**Use Case**: HTTP requests, API calls

```windjammer
use std::http::*

// User writes
let data = http::get("https://api.github.com/repos/rust-lang/rust")

// Native: Uses reqwest
// WASM: Uses fetch() API
// Same code, different implementation!
```

**Already works!** Just needs better error handling.

### Tier 3: OS Operations (Backend Proxy)
**Use Case**: File system, process execution

```windjammer
use std::process::*

// User writes
let output = process::execute("rustc", vec!["main.rs"])

// Native: Executes directly
// WASM: Proxies to backend API automatically
```

**Compiler generates**:
```rust
// Native
std::process::Command::new("rustc").args(&["main.rs"]).output()

// WASM
http::post("/api/process/execute", json!({
    "command": "rustc",
    "args": ["main.rs"]
}))
```

---

## Implementation Plan

### Phase 1: Compute Abstraction (Web Workers)

#### 1.1 New Standard Library Module: `std::compute`

```windjammer
// std/compute/mod.wj

/// Run a computation in parallel
pub fn parallel<T, R>(items: Vec<T>, f: fn(T) -> R) -> Vec<R> {
    // Compiler generates platform-specific code
}

/// Run a computation in the background
pub fn background<F: FnOnce() -> T>(f: F) -> Future<T> {
    // Compiler generates platform-specific code
}

/// Get number of available cores/workers
pub fn num_workers() -> int {
    // Compiler generates platform-specific code
}
```

#### 1.2 Compiler Detection

```rust
// In codegen/rust/generator.rs

fn detect_compute_usage(&self, program: &Program) -> bool {
    // Check for use std::compute::*
}

fn generate_compute_impl(&self) -> String {
    match self.target {
        CompilationTarget::Wasm => {
            // Generate Web Worker pool
            r#"
            use web_sys::Worker;
            
            struct WorkerPool {
                workers: Vec<Worker>,
            }
            
            impl WorkerPool {
                fn new(size: usize) -> Self {
                    let workers = (0..size)
                        .map(|_| Worker::new("worker.js").unwrap())
                        .collect();
                    Self { workers }
                }
                
                fn execute<T>(&self, task: T) -> Future<Result> {
                    // Distribute work across workers
                }
            }
            "#
        }
        CompilationTarget::Rust => {
            // Generate rayon parallel code
            r#"
            use rayon::prelude::*;
            
            fn parallel<T, R>(items: Vec<T>, f: impl Fn(T) -> R) -> Vec<R> {
                items.par_iter().map(f).collect()
            }
            "#
        }
    }
}
```

#### 1.3 Runtime Implementation

```rust
// crates/windjammer-runtime/src/platform/wasm/compute.rs

use web_sys::{Worker, MessageEvent};
use wasm_bindgen::prelude::*;

pub struct WorkerPool {
    workers: Vec<Worker>,
    available: Vec<usize>,
}

impl WorkerPool {
    pub fn new(size: usize) -> Self {
        let workers = (0..size)
            .map(|_| Worker::new("worker.js").unwrap())
            .collect();
        
        let available = (0..size).collect();
        
        Self { workers, available }
    }
    
    pub async fn execute<T, R>(&mut self, data: Vec<T>, f: &str) -> Vec<R> {
        // Serialize function and data
        // Send to workers
        // Collect results
        // Return
    }
}
```

---

### Phase 2: Backend Proxy (Automatic)

#### 2.1 Configuration File

```toml
# windjammer.toml (in user's project)

[backend]
# Optional: If not provided, WASM operations return errors
url = "https://myapi.com"
# or for development
url = "http://localhost:3000"

[backend.auth]
# Optional: API key for backend
api_key = "${API_KEY}"
```

#### 2.2 Compiler Behavior

```rust
// When compiling to WASM and backend is configured

fn generate_process_impl(&self, config: &BackendConfig) -> String {
    if let Some(backend_url) = &config.backend_url {
        // Generate proxy code
        format!(r#"
        async fn execute(command: String, args: Vec<String>) -> Result<String, String> {{
            let response = http::post("{}/api/process/execute", json!({{
                "command": command,
                "args": args
            }})).await?;
            
            Ok(response.body)
        }}
        "#, backend_url)
    } else {
        // Generate error code (current behavior)
        r#"
        fn execute(command: String, args: Vec<String>) -> Result<String, String> {
            Err("Process execution not available in browser. Configure a backend in windjammer.toml".to_string())
        }
        "#
    }
}
```

#### 2.3 Backend Server (Generated!)

```bash
# Compiler can generate a backend server!
wj generate-backend --output backend/

# Generates:
# backend/
#   src/
#     main.rs          # Axum server
#     process.rs       # Process execution handlers
#     fs.rs            # File system handlers
#   Cargo.toml
#   README.md
```

**Generated backend**:
```rust
// backend/src/main.rs (generated by Windjammer)

use axum::{Router, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ProcessRequest {
    command: String,
    args: Vec<String>,
}

#[derive(Serialize)]
struct ProcessResponse {
    stdout: String,
    stderr: String,
    status: i32,
}

async fn execute_process(Json(req): Json<ProcessRequest>) -> Json<ProcessResponse> {
    // Security: Whitelist allowed commands
    let allowed = ["rustc", "cargo", "ls", "cat"];
    if !allowed.contains(&req.command.as_str()) {
        return Json(ProcessResponse {
            stdout: String::new(),
            stderr: "Command not allowed".to_string(),
            status: 1,
        });
    }
    
    // Execute
    let output = std::process::Command::new(&req.command)
        .args(&req.args)
        .output()
        .unwrap();
    
    Json(ProcessResponse {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status.code().unwrap_or(1),
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/process/execute", axum::routing::post(execute_process));
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

### Phase 3: Smart Defaults

#### 3.1 Automatic Fallback Chain

```windjammer
use std::process::*

// User writes this
let result = process::execute("ls", vec!["-la"])

// Compiler generates (WASM):
async fn execute(command: String, args: Vec<String>) -> Result<String, String> {
    // Try 1: Backend proxy (if configured)
    if let Some(backend) = get_backend_config() {
        match http::post(backend + "/api/process/execute", ...).await {
            Ok(result) => return Ok(result),
            Err(_) => {} // Fall through
        }
    }
    
    // Try 2: Browser extension (if available)
    if let Some(extension) = check_browser_extension() {
        match extension.execute(command, args).await {
            Ok(result) => return Ok(result),
            Err(_) => {} // Fall through
        }
    }
    
    // Try 3: Return helpful error
    Err(format!(
        "Process execution not available in browser.\n\
         Options:\n\
         1. Configure a backend in windjammer.toml\n\
         2. Install the Windjammer browser extension\n\
         3. Use compute::background() for CPU tasks\n\
         Command: {} {:?}",
        command, args
    ))
}
```

---

## User Experience

### Example 1: Image Processing

```windjammer
use std::compute::*
use std::fs::*

fn process_images() {
    let images = fs::list_directory("./images")
    
    // Automatically uses Web Workers in browser, rayon on native
    let processed = compute::parallel(images, |img| {
        // Heavy image processing
        apply_filters(img)
    })
    
    // Save results
    for (i, result) in processed.iter().enumerate() {
        fs::write_file("output_{}.png".format(i), result)
    }
}
```

**Native**: Uses all CPU cores with rayon  
**WASM (no backend)**: Uses Web Workers for processing, shows error for file operations  
**WASM (with backend)**: Uses Web Workers + backend proxy for files

### Example 2: Build System

```windjammer
use std::process::*

fn build_project() {
    // User doesn't care about platform!
    let result = process::execute("cargo", vec!["build", "--release"])
    
    match result {
        Ok(output) => println!("Build succeeded: {}", output),
        Err(e) => println!("Build failed: {}", e)
    }
}
```

**Native**: Executes directly  
**WASM (with backend)**: Proxies to backend  
**WASM (no backend)**: Clear error with suggestions

---

## Configuration Examples

### Development (Local Backend)

```toml
# windjammer.toml
[backend]
url = "http://localhost:3000"
```

### Production (Cloud Backend)

```toml
# windjammer.toml
[backend]
url = "https://api.myapp.com"

[backend.auth]
api_key = "${WINDJAMMER_API_KEY}"

[backend.security]
# Whitelist allowed commands
allowed_commands = ["rustc", "cargo", "git"]
```

### No Backend (Pure WASM)

```toml
# No backend section = errors for OS operations
# But compute::* still works with Web Workers!
```

---

## Security Considerations

### Backend Security
1. **Command Whitelist**: Only allow specific commands
2. **API Authentication**: Require API keys
3. **Rate Limiting**: Prevent abuse
4. **Sandboxing**: Run commands in containers
5. **Audit Logging**: Track all executions

### Generated Backend Includes
```rust
// Security middleware (auto-generated)

async fn validate_command(command: &str) -> Result<(), Error> {
    // Check whitelist
    let allowed = load_allowed_commands();
    if !allowed.contains(command) {
        return Err(Error::CommandNotAllowed);
    }
    
    // Check rate limit
    if rate_limiter.check_limit(&get_client_ip()).is_err() {
        return Err(Error::RateLimitExceeded);
    }
    
    // Validate API key
    if !validate_api_key(&get_auth_header()) {
        return Err(Error::Unauthorized);
    }
    
    Ok(())
}
```

---

## Compiler Flags

```bash
# Generate backend server
wj generate-backend --output backend/

# Build with backend proxy
wj build app.wj --target wasm --backend https://api.myapp.com

# Build without backend (errors for OS ops)
wj build app.wj --target wasm

# Build with Web Workers only
wj build app.wj --target wasm --features compute
```

---

## Benefits

### For Users
1. ✅ **Write once, run anywhere** (for real)
2. ✅ **No platform-specific code**
3. ✅ **Automatic optimization** (Web Workers, parallel, etc.)
4. ✅ **Clear error messages** when features unavailable
5. ✅ **Optional backend** for full functionality

### For Windjammer
1. ✅ **True platform abstraction**
2. ✅ **Competitive advantage** (no other language does this)
3. ✅ **Dogfooding** (use Windjammer to build Windjammer tools)
4. ✅ **Ecosystem growth** (backend generation, extensions)

---

## Implementation Priority

### Phase 1: Foundation (2-3 weeks)
1. ✅ Platform abstraction (DONE!)
2. ⏳ `std::compute` module
3. ⏳ Web Worker generation
4. ⏳ Rayon parallel generation

### Phase 2: Backend Proxy (2-3 weeks)
1. ⏳ `windjammer.toml` configuration
2. ⏳ Backend proxy code generation
3. ⏳ `wj generate-backend` command
4. ⏳ Security middleware

### Phase 3: Polish (1-2 weeks)
1. ⏳ Automatic fallback chain
2. ⏳ Better error messages
3. ⏳ Documentation
4. ⏳ Examples

---

## Example: Complete Workflow

### 1. User Writes Pure Windjammer
```windjammer
// app.wj
use std::compute::*
use std::process::*
use std::fs::*

fn main() {
    // Heavy computation
    let data = vec![1, 2, 3, 4, 5]
    let results = compute::parallel(data, |x| x * x)
    
    // File operations
    fs::write_file("results.txt", format!("{:?}", results))
    
    // Process execution
    let output = process::execute("ls", vec!["-la"])
    println!("{}", output)
}
```

### 2. Compile for Native
```bash
wj build app.wj --target rust
./app
# Everything works directly!
```

### 3. Compile for WASM (No Backend)
```bash
wj build app.wj --target wasm
# Warnings:
# - fs::write_file will fail (no backend)
# - process::execute will fail (no backend)
# - compute::parallel will use Web Workers ✓
```

### 4. Generate Backend
```bash
wj generate-backend --output backend/
cd backend && cargo run
# Backend running on http://localhost:3000
```

### 5. Compile for WASM (With Backend)
```bash
# windjammer.toml
[backend]
url = "http://localhost:3000"

wj build app.wj --target wasm
# Everything works! Backend proxy handles fs/process
```

---

## Conclusion

This design achieves **true platform abstraction**:

1. ✅ **Users write pure Windjammer** - no platform-specific code
2. ✅ **Compiler handles complexity** - generates Web Workers, proxies, etc.
3. ✅ **Optional backend** - full functionality when needed
4. ✅ **Graceful degradation** - clear errors when features unavailable
5. ✅ **Security built-in** - whitelists, auth, rate limiting

**This is what makes Windjammer special.** No other language offers this level of transparent platform abstraction.

**Next Steps**: Implement `std::compute` module with Web Workers support!

