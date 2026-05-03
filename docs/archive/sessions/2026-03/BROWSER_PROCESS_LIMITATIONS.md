# Browser Process Limitations - Detailed Explanation

## Overview

When you run Windjammer code compiled to WASM in a browser, **process execution is completely unavailable** due to fundamental browser security restrictions.

---

## Why Process Execution Doesn't Work in Browsers

### 1. **Security Sandbox**
Browsers run in a strict security sandbox that prevents:
- Direct access to the operating system
- Execution of arbitrary system commands
- Spawning of processes
- Access to the file system (except through specific APIs)

**Reason**: If browsers could execute arbitrary processes, malicious websites could:
- Run viruses/malware on your computer
- Mine cryptocurrency without permission
- Steal sensitive data
- Take control of your system

### 2. **No System API Access**
Unlike native applications, browser JavaScript (and WASM) cannot:
- Call `fork()` or `exec()` system calls
- Access `std::process::Command` in Rust
- Run shell commands
- Interact with the operating system directly

### 3. **Different Execution Model**
Browsers use a **single-threaded event loop** model:
- All code runs in the main thread or Web Workers
- No concept of "child processes"
- No inter-process communication (IPC)

---

## What `std::process` Does in WASM

When you write:
```windjammer
use std::process::*

process::execute("ls", vec!["-la"])
```

And compile to WASM, the generated code calls:
```rust
use windjammer_runtime::platform::wasm::process;

process::execute("ls".to_string(), vec!["-la".to_string()])
```

Which returns:
```rust
Err("Process execution not available in browser. Use Web Workers for background tasks.")
```

---

## Alternatives in the Browser

### 1. **Web Workers** (Background Computation)
For CPU-intensive tasks:
```javascript
// Create a Web Worker
const worker = new Worker('worker.js');

// Send data to worker
worker.postMessage({ task: 'compute', data: [1, 2, 3] });

// Receive results
worker.onmessage = (e) => {
    console.log('Result:', e.data);
};
```

**Use cases**:
- Heavy computations
- Data processing
- Parallel algorithms
- Background tasks

**Limitations**:
- No access to DOM
- No shared memory (except SharedArrayBuffer)
- Communication via message passing only

### 2. **fetch() API** (Network Requests)
For external services:
```javascript
// Call an API
const response = await fetch('https://api.example.com/data');
const data = await response.json();
```

**Use cases**:
- REST API calls
- GraphQL queries
- File uploads/downloads
- External service integration

### 3. **WebAssembly Threads** (Parallel Computation)
For true parallelism:
```rust
// Rust with wasm-bindgen
use wasm_bindgen::prelude::*;
use rayon::prelude::*; // Parallel iterator library

#[wasm_bindgen]
pub fn parallel_sum(data: Vec<i32>) -> i32 {
    data.par_iter().sum()
}
```

**Use cases**:
- Parallel data processing
- Matrix operations
- Image/video processing

**Limitations**:
- Requires SharedArrayBuffer
- Browser support varies
- More complex setup

### 4. **Server-Side Execution** (Backend API)
For actual process execution:
```javascript
// Frontend calls backend
const result = await fetch('/api/execute', {
    method: 'POST',
    body: JSON.stringify({ command: 'ls', args: ['-la'] })
});
```

**Backend** (Node.js, Rust, etc.):
```rust
// Actual process execution on server
use std::process::Command;

let output = Command::new("ls")
    .args(&["-la"])
    .output()?;
```

**Use cases**:
- File system operations
- System commands
- Database access
- Sensitive operations

---

## Comparison Table

| Feature | Native | WASM (Browser) | Solution |
|---------|--------|----------------|----------|
| **Process Execution** | ✅ Full access | ❌ Not available | Use backend API |
| **File System** | ✅ Full access | ⚠️ Limited (File API) | Use IndexedDB or backend |
| **Network** | ✅ Full access | ✅ fetch() API | Works well |
| **Threads** | ✅ OS threads | ⚠️ Web Workers | Different model |
| **System Calls** | ✅ All syscalls | ❌ None | Use Web APIs |
| **Performance** | ✅ Native speed | ✅ Near-native | Good |
| **Security** | ⚠️ User's responsibility | ✅ Sandboxed | Very secure |

---

## Real-World Examples

### ❌ Won't Work in Browser
```windjammer
// This will FAIL in WASM
use std::process::*

// Try to run a compiler
let result = process::execute("rustc", vec!["main.rs"])

// Try to list files
let files = process::execute("ls", vec!["-la"])

// Try to install packages
let output = process::execute("npm", vec!["install", "package"])
```

### ✅ Will Work in Browser
```windjammer
// Use Web APIs instead
use std::http::*

// Fetch data from API
let data = http::get("https://api.github.com/repos/rust-lang/rust")

// Call backend to execute process
let result = http::post("https://myapi.com/execute", json!({
    "command": "rustc",
    "args": ["main.rs"]
}))
```

---

## Architecture Pattern: Backend + Frontend

### Frontend (WASM)
```windjammer
use std::http::*

fn compile_code(code: string) -> Result<string, string> {
    // Send to backend for actual compilation
    let response = http::post("/api/compile", json!({
        "code": code,
        "language": "rust"
    }))?
    
    Ok(response.body)
}
```

### Backend (Native Rust/Node.js/etc.)
```rust
use std::process::Command;

fn compile_code(code: &str) -> Result<String, String> {
    // Write code to temp file
    std::fs::write("/tmp/code.rs", code)?;
    
    // Actually compile it
    let output = Command::new("rustc")
        .args(&["/tmp/code.rs"])
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

---

## Summary

### What Works
- ✅ Computation (WASM is fast!)
- ✅ Network requests (fetch API)
- ✅ Background tasks (Web Workers)
- ✅ User interaction (DOM APIs)
- ✅ Graphics (Canvas, WebGL)

### What Doesn't Work
- ❌ Process execution
- ❌ Direct file system access
- ❌ System commands
- ❌ Native OS integration

### The Solution
**Hybrid Architecture**: Frontend (WASM) + Backend (Native)
- Frontend handles UI, computation, user interaction
- Backend handles file system, processes, system integration
- Communication via HTTP/WebSocket

---

## Windjammer's Approach

Windjammer makes this **explicit and clear**:

```windjammer
use std::process::*

// In native code: Works perfectly
// In WASM: Returns helpful error message
let result = process::execute("ls", vec![])

match result {
    Ok(output) => println!("Output: {}", output),
    Err(e) => println!("Error: {}", e)
    // In browser: "Process execution not available in browser. Use Web Workers for background tasks."
}
```

**Philosophy**: Same API, platform-specific behavior, clear error messages.

This is **platform abstraction done right**! ✅

