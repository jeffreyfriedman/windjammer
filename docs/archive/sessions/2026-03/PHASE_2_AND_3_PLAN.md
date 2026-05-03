# Phase 2 & 3: Backend Proxy + More Abstractions

## Overview

Building on the success of `std::compute`, we're expanding transparent browser abstractions to cover:
- **Phase 2**: Backend proxy for OS operations (fs, process)
- **Phase 3**: Network, storage, and more compute features

---

## Phase 2: Backend Proxy

### Goal
Enable `std::fs` and `std::process` operations in WASM by automatically proxying to a backend server.

### User Experience

```windjammer
// windjammer.toml
[backend]
url = "http://localhost:3000"
api_key = "${WINDJAMMER_API_KEY}"

// User code - SAME AS BEFORE!
use std::process::*

let output = process::execute("rustc", vec!["main.rs"])
// Native: Executes directly
// WASM (no backend): Error with helpful message
// WASM (with backend): Proxies to backend automatically!
```

### Components

#### 1. Configuration System (`std::config`)
✅ **CREATED**: `std/config/mod.wj`

- Read `windjammer.toml` at compile time
- Inject backend URL into generated code
- Support environment variable expansion

#### 2. Backend Proxy Code Generation

Modify `src/codegen/rust/generator.rs` to generate:

```rust
// WASM with backend configured
pub fn execute(command: String, args: Vec<String>) -> Result<String, String> {
    // Try backend first
    if let Some(backend_url) = option_env!("WINDJAMMER_BACKEND_URL") {
        match proxy_to_backend(backend_url, command, args).await {
            Ok(result) => return Ok(result),
            Err(_) => {} // Fall through to error
        }
    }
    
    // No backend or backend failed
    Err("Process execution not available in browser. Configure backend in windjammer.toml")
}

async fn proxy_to_backend(url: &str, cmd: String, args: Vec<String>) -> Result<String, String> {
    let response = http::post(
        format!("{}/api/process/execute", url),
        json!({ "command": cmd, "args": args })
    ).await?;
    
    Ok(response.body)
}
```

#### 3. Backend Server Generator (`wj generate-backend`)

Generate a complete Axum backend server:

```bash
wj generate-backend --output backend/

# Generates:
backend/
  src/
    main.rs          # Axum server
    process.rs       # Process execution handlers
    fs.rs            # File system handlers
    security.rs      # Authentication & authorization
  Cargo.toml
  windjammer.toml    # Backend configuration
  README.md
```

**Generated server features**:
- Command whitelist (security)
- API key authentication
- Rate limiting
- CORS support
- Request logging
- Docker deployment ready

#### 4. Fallback Chain

```
User calls: process::execute("rustc", ["main.rs"])
                    │
                    ▼
┌───────────────────────────────────────────┐
│ 1. Try Backend Proxy (if configured)     │
│    ├─ Success? Return result             │
│    └─ Failed? Continue to step 2         │
└───────────────────────────────────────────┘
                    │
                    ▼
┌───────────────────────────────────────────┐
│ 2. Try Browser Extension (if available)  │
│    ├─ Success? Return result             │
│    └─ Failed? Continue to step 3         │
└───────────────────────────────────────────┘
                    │
                    ▼
┌───────────────────────────────────────────┐
│ 3. Return Helpful Error                  │
│    "Process execution not available.     │
│     Options:                              │
│     1. Configure backend in .toml        │
│     2. Install Windjammer extension      │
│     3. Use compute:: for CPU tasks"      │
└───────────────────────────────────────────┘
```

---

## Phase 3: More Abstractions

### 1. Network Operations (`std::net`)
✅ **CREATED**: `std/net/mod.wj`

**Features**:
- HTTP requests (GET, POST, PUT, DELETE)
- Request builder pattern
- Async and sync APIs
- File upload/download
- WebSocket support

**Implementation**:
- **Native**: `reqwest` crate
- **WASM**: `fetch()` API via `web-sys`

**Example**:
```windjammer
use std::net::*

// Simple GET
let response = get("https://api.github.com/users/octocat")
println!("{}", response.body)

// Request builder
let response = Request::get("https://api.example.com")
    .header("Authorization", "Bearer token")
    .timeout(30)
    .send()

// Async
let response = get_async("https://api.example.com").await
```

### 2. Storage (`std::storage`)
✅ **CREATED**: `std/storage/mod.wj`

**Features**:
- Key-value storage
- Multiple backends (local, session, persistent)
- JSON serialization
- Binary data support
- TTL (time-to-live) support

**Implementation**:
- **Native**: File-based or SQLite
- **WASM**: localStorage, sessionStorage, IndexedDB

**Example**:
```windjammer
use std::storage::*

// Simple key-value
storage::set("user_name", "Alice")
let name = storage::get("user_name")

// JSON storage
storage::set_json("user", user_object)
let user = storage::get_json::<User>("user")

// With expiration
storage::set_with_ttl("session", token, 3600) // 1 hour

// Different backends
storage::set_with_backend("temp", data, StorageBackend::Session)
```

### 3. Enhanced Compute Features

**New APIs**:
```windjammer
use std::compute::*

// Stream processing (lazy evaluation)
let results = stream(large_dataset)
    .map(|x| x * 2)
    .filter(|x| x > 100)
    .collect()

// Pipeline (multi-stage parallel processing)
let results = pipeline(data)
    .stage(|x| parse(x))      // Stage 1: Parse
    .stage(|x| validate(x))   // Stage 2: Validate
    .stage(|x| transform(x))  // Stage 3: Transform
    .collect()

// Parallel fold
let sum = parallel_fold(data, 0, |acc, x| acc + x)

// Chunked processing (for memory efficiency)
let results = parallel_chunks(huge_dataset, 1000, |chunk| {
    process_chunk(chunk)
})
```

### 4. Clipboard (`std::clipboard`)

**Features**:
- Read/write clipboard
- Multiple formats (text, HTML, images)
- Permission handling

**Implementation**:
- **Native**: `clipboard` crate
- **WASM**: Clipboard API

**Example**:
```windjammer
use std::clipboard::*

// Copy to clipboard
clipboard::copy("Hello, World!")

// Paste from clipboard
let text = clipboard::paste()

// Copy image
clipboard::copy_image(image_data)
```

### 5. Notifications (`std::notify`)

**Features**:
- Desktop notifications
- Toast messages
- Progress notifications

**Implementation**:
- **Native**: OS notification system
- **WASM**: Notification API

**Example**:
```windjammer
use std::notify::*

// Simple notification
notify::show("Build Complete!", "Your project built successfully")

// With icon and actions
notify::builder()
    .title("New Message")
    .body("You have 3 unread messages")
    .icon("message.png")
    .action("View", || open_messages())
    .show()
```

---

## Implementation Priority

### Week 1: Backend Proxy Foundation
1. ✅ Configuration system (`std::config`)
2. ⏳ Parse `windjammer.toml` at compile time
3. ⏳ Generate backend proxy code for WASM
4. ⏳ Implement fallback chain

### Week 2: Backend Server Generator
1. ⏳ `wj generate-backend` command
2. ⏳ Generate Axum server template
3. ⏳ Security middleware (whitelist, auth, rate limit)
4. ⏳ Docker deployment files

### Week 3: Network & Storage
1. ✅ `std::net` API design
2. ✅ `std::storage` API design
3. ⏳ Native implementations (reqwest, file-based)
4. ⏳ WASM implementations (fetch, localStorage)

### Week 4: Enhanced Compute & Polish
1. ⏳ Stream processing
2. ⏳ Pipeline API
3. ⏳ Chunked processing
4. ⏳ Full Web Worker parallelism

### Week 5: Additional Abstractions
1. ⏳ Clipboard API
2. ⏳ Notifications API
3. ⏳ Documentation
4. ⏳ Examples

---

## Testing Strategy

### Unit Tests
```windjammer
// Test backend proxy fallback
#[test]
fn test_backend_fallback() {
    // No backend configured
    let result = process::execute("ls", vec![])
    assert!(result.is_err())
    assert!(result.err().contains("Configure backend"))
}

// Test storage abstraction
#[test]
fn test_storage() {
    storage::set("key", "value")
    assert_eq!(storage::get("key"), Some("value"))
}
```

### Integration Tests
```bash
# Test backend proxy
wj build app.wj --target wasm --backend http://localhost:3000
# Start backend server
cd backend && cargo run &
# Run WASM app and verify proxy works
```

### Browser Tests
```javascript
// Test WASM with all abstractions
import init, { run_app } from './windjammer_wasm.js';

await init();
run_app(); // Uses compute, storage, net, etc.
```

---

## Documentation

### User Guide
- `docs/BACKEND_PROXY.md` - How to configure and use backend proxy
- `docs/NETWORK_OPERATIONS.md` - Network API guide
- `docs/STORAGE_API.md` - Storage API guide
- `docs/COMPUTE_ADVANCED.md` - Advanced compute features

### API Reference
- `docs/api/std_config.md`
- `docs/api/std_net.md`
- `docs/api/std_storage.md`
- `docs/api/std_compute_advanced.md`

### Examples
- `examples/backend_proxy_demo.wj` - Backend proxy usage
- `examples/network_demo.wj` - HTTP requests and WebSockets
- `examples/storage_demo.wj` - Persistent storage
- `examples/compute_advanced.wj` - Streams and pipelines

---

## Security Considerations

### Backend Proxy Security

1. **Command Whitelist**
```rust
// Only allow specific commands
const ALLOWED_COMMANDS: &[&str] = &["rustc", "cargo", "git"];

fn validate_command(cmd: &str) -> Result<(), Error> {
    if !ALLOWED_COMMANDS.contains(&cmd) {
        return Err(Error::CommandNotAllowed);
    }
    Ok(())
}
```

2. **API Authentication**
```rust
// Require API key for all requests
async fn auth_middleware(req: Request) -> Result<Request, Error> {
    let api_key = req.headers().get("X-API-Key")
        .ok_or(Error::Unauthorized)?;
    
    if !validate_api_key(api_key) {
        return Err(Error::Unauthorized);
    }
    
    Ok(req)
}
```

3. **Rate Limiting**
```rust
// Limit requests per IP
let limiter = RateLimiter::new(100, Duration::from_secs(60));

if !limiter.check(&client_ip) {
    return Err(Error::RateLimitExceeded);
}
```

4. **Sandboxing**
```rust
// Run commands in Docker container
let output = Command::new("docker")
    .args(&["run", "--rm", "windjammer-sandbox", cmd])
    .output()?;
```

### Storage Security

1. **Encryption at Rest**
```windjammer
// Encrypt sensitive data
storage::set_encrypted("password", password, encryption_key)
```

2. **Access Control**
```windjammer
// Per-user storage isolation
storage::set_user("user_123", "data", value)
```

---

## Performance Targets

### Backend Proxy
- Latency: < 100ms for simple commands
- Throughput: > 100 requests/second
- Concurrent connections: > 1000

### Network Operations
- HTTP request overhead: < 10ms
- WebSocket latency: < 5ms
- Download speed: Limited by network, not abstraction

### Storage
- localStorage: < 1ms for read/write
- IndexedDB: < 10ms for read/write
- File-based: < 5ms for read/write

### Compute (Enhanced)
- Stream processing: Same as parallel (8x speedup on 8 cores)
- Pipeline: Linear scaling with stages
- Chunked: Memory-efficient for large datasets

---

## Success Metrics

### Phase 2 Success Criteria
✅ Backend proxy works for fs and process operations  
✅ `wj generate-backend` creates working server  
✅ Security features (whitelist, auth, rate limit) implemented  
✅ Fallback chain provides helpful error messages  
✅ Documentation complete

### Phase 3 Success Criteria
✅ Network operations work on native and WASM  
✅ Storage works with localStorage and IndexedDB  
✅ Enhanced compute features (stream, pipeline) work  
✅ All abstractions have zero leaks  
✅ Performance targets met

---

## Timeline

### Month 1: Foundation
- Week 1-2: Backend proxy
- Week 3-4: Network & storage

### Month 2: Enhancement
- Week 1-2: Enhanced compute
- Week 3-4: Additional abstractions (clipboard, notify)

### Month 3: Polish
- Week 1-2: Documentation and examples
- Week 3-4: Performance optimization and testing

---

## Conclusion

With Phases 2 and 3, Windjammer will have:

1. ✅ **Complete platform abstraction** - All OS operations work everywhere
2. ✅ **Transparent backend proxy** - WASM apps can use OS features via backend
3. ✅ **Rich standard library** - Network, storage, compute, and more
4. ✅ **Zero abstraction leaks** - Users never see platform details
5. ✅ **Production-ready** - Security, performance, documentation

**This makes Windjammer the ONLY language with truly transparent cross-platform abstractions.** 🚀

---

**Next Step**: Implement backend configuration parsing and proxy code generation!

