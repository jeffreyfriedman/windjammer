# v0.5.0 Development Plan - Expanded Standard Library

## 🎯 Goal
Expand the standard library with more essential modules and improve WASM/HTTP functionality to provide a truly "batteries included" experience.

## 📋 Features for v0.5.0

### Phase 1: Core Stdlib Modules (Week 1)

#### 1. **std/time** - Date/Time Operations
Wrap `chrono` with simple API:
```windjammer
use std.time

let now = time.now()
let formatted = time.format(now, "%Y-%m-%d %H:%M:%S")
let parsed = time.parse("2025-01-01", "%Y-%m-%d")
```

#### 2. **std/strings** - String Utilities
Common string operations:
```windjammer
use std.strings

let result = strings.split("hello,world", ",")
let joined = strings.join(vec!["a", "b"], "-")
let trimmed = strings.trim("  hello  ")
let upper = strings.to_uppercase("hello")
```

#### 3. **std/math** - Math Functions
Wrap common math operations:
```windjammer
use std.math

let result = math.sqrt(16.0)
let power = math.pow(2.0, 3.0)
let random = math.random()
```

### Phase 2: CLI and Logging (Week 2)

#### 4. **std/cli** - Command-Line Parsing
Wrap `clap` with declarative API:
```windjammer
use std.cli

@arg(short: "n", long: "name", help: "Your name")
struct Args {
    name: string,
    
    @arg(short: "v", long: "verbose")
    verbose: bool,
}

fn main() {
    let args = cli.parse<Args>()
    println!("Hello, {}!", args.name)
}
```

#### 5. **std/log** - Logging
Wrap `tracing` or `log`:
```windjammer
use std.log

fn main() {
    log.init()
    
    log.info("Application started")
    log.debug("Debug information")
    log.error("Something went wrong")
    log.warn("Warning message")
}
```

### Phase 3: Advanced Modules (Week 3)

#### 6. **std/regex** - Regular Expressions
Wrap `regex` crate:
```windjammer
use std.regex

let pattern = regex.compile(r"\d+")
let matches = regex.find_all(pattern, "abc 123 def 456")
let replaced = regex.replace_all(pattern, "text", "X")
```

#### 7. **std/encoding** - Base64, Hex, etc.
```windjammer
use std.encoding

let encoded = encoding.base64_encode("hello")
let decoded = encoding.base64_decode(encoded)

let hex = encoding.hex_encode(&[0xFF, 0xAA])
```

#### 8. **std/crypto** - Hashing and Crypto
Wrap `sha2`, `blake3` for hashing:
```windjammer
use std.crypto

let hash = crypto.sha256("hello world")
let blake = crypto.blake3("data")
```

### Phase 4: HTTP Server (Week 4)

#### 9. **std/http - Server Support**
Add HTTP server to existing http module:
```windjammer
use std.http

fn handle_request(req: Request) -> Response {
    Response {
        status: 200,
        body: "Hello, World!",
    }
}

fn main() {
    http.serve("127.0.0.1:8080", handle_request)
}
```

Or with routing:
```windjammer
use std.http

fn main() {
    let server = http.Server.new()
    
    server.get("/", |req| {
        Response.ok("Home page")
    })
    
    server.post("/api/users", |req| {
        let body = req.json()
        Response.created(body)
    })
    
    server.listen("127.0.0.1:8080")
}
```

## 🎨 Design Decisions

### 1. Sync vs Async
**Decision**: Provide both sync and async versions

```windjammer
// Sync (blocking)
use std.http

let response = http.get("https://api.example.com")

// Async
use std.http

async fn fetch_data() {
    let response = http.get_async("https://api.example.com").await
}
```

### 2. Error Handling
**Decision**: Use Rust's `Result<T, E>` pattern consistently

```windjammer
let result = fs.read_to_string("file.txt")

match result {
    Ok(content) => println!("{}", content),
    Err(e) => println!("Error: {}", e),
}
```

### 3. Type Aliases
**Decision**: Re-export Rust types for advanced use

```windjammer
// std/time.wj
type DateTime = chrono::DateTime<chrono::Utc>
type Duration = chrono::Duration

// Users can use them directly
let dt: DateTime = time.now()
```

### 4. Module Organization
**Decision**: Keep stdlib modules independent

Each `std/` module should:
- Be usable standalone
- Not depend on other std modules (except common types)
- Have clear, minimal dependencies

## 🔧 Implementation Strategy

### Step 1: Create Module Files
For each module:
1. Create `std/module_name.wj`
2. Define simple wrapper functions
3. Re-export types
4. Add examples in comments

### Step 2: Test Each Module
Create `examples/07_stdlib_advanced/` with:
- Time operations example
- String manipulation example
- CLI app example
- Logging example
- Regex example
- HTTP server example

### Step 3: Documentation
Update:
- `std/README.md` with new modules
- `docs/GUIDE.md` with stdlib usage
- Create `docs/STDLIB_REFERENCE.md` with full API docs

## 📊 Success Metrics

- [ ] 8+ stdlib modules implemented
- [ ] Each module has examples
- [ ] HTTP server working
- [ ] All tests passing
- [ ] Documentation complete
- [ ] Real-world example app built

## 🚧 Challenges & Solutions

### Challenge 1: Async/Await
**Problem**: Windjammer doesn't have async/await syntax yet

**Solution**: Use Rust's async/await in generated code
```windjammer
async fn fetch() {
    let data = http.get_async("url").await
}
```
Transpiles to:
```rust
async fn fetch() {
    let data = reqwest::get("url").await;
}
```

### Challenge 2: Macro-Heavy Crates
**Problem**: Crates like `clap` use derive macros heavily

**Solution**: Provide simplified wrappers
```windjammer
// User writes simple structs
@arg(long: "name")
struct Args {
    name: string,
}

// We generate clap code
```

### Challenge 3: Error Types
**Problem**: Each crate has different error types

**Solution**: Re-export as module-specific errors
```windjammer
// std/http.wj
type Error = reqwest::Error

// std/fs.wj
type Error = std::io::Error
```

## 🎯 Priority Order

1. **HIGH**: `time`, `strings`, `math` (common utilities)
2. **HIGH**: `log` (essential for debugging)
3. **MEDIUM**: `cli` (for building tools)
4. **MEDIUM**: `regex` (common need)
5. **MEDIUM**: `encoding`, `crypto` (security)
6. **LOW**: HTTP server (complex, can defer to v0.6.0)

## 📝 Examples to Build

### Example 1: CLI Tool
```windjammer
// examples/cli_todo/main.wj
use std.cli
use std.fs
use std.json

@arg(subcommand)
enum Command {
    Add { task: string },
    List,
    Done { id: int },
}

fn main() {
    let cmd = cli.parse<Command>()
    
    match cmd {
        Command.Add { task } => add_task(task),
        Command.List => list_tasks(),
        Command.Done { id } => mark_done(id),
    }
}
```

### Example 2: Web Server
```windjammer
// examples/hello_server/main.wj
use std.http
use std.log

fn main() {
    log.init()
    
    let server = http.Server.new()
    
    server.get("/", |req| {
        Response.ok("Hello, World!")
    })
    
    server.get("/health", |req| {
        Response.json({"status": "ok"})
    })
    
    log.info("Server starting on port 8080")
    server.listen("127.0.0.1:8080")
}
```

### Example 3: Data Processing
```windjammer
// examples/data_processor/main.wj
use std.fs
use std.json
use std.log
use std.strings

fn main() {
    log.init()
    
    // Read CSV file
    let content = fs.read_to_string("data.csv")
    
    // Process lines
    let lines = strings.split(content, "\n")
    let mut results = Vec.new()
    
    for line in lines {
        let fields = strings.split(line, ",")
        results.push(process_row(fields))
    }
    
    // Write JSON output
    let json = json.to_string_pretty(&results)
    fs.write("output.json", json)
    
    log.info("Processing complete")
}
```

## 🔄 Optional Enhancements

### Auto-Import for Stdlib (Future)
Instead of:
```windjammer
use std.json
use std.fs
```

Could have:
```windjammer
// std.* automatically available
let data = json.parse("{}")
fs.write("file.txt", "content")
```

**Decision**: Defer to v0.6.0, keep explicit imports for now

### Prelude Module (Future)
```windjammer
use std.prelude  // Imports common types/functions

// Now have access to:
// - Result, Option, Vec, HashMap
// - println!, format!
// - Common traits
```

**Decision**: Defer to v0.6.0

## 📅 Timeline

- **Week 1**: time, strings, math modules + tests
- **Week 2**: cli, log modules + examples
- **Week 3**: regex, encoding, crypto modules
- **Week 4**: HTTP server (if time permits) + documentation

**Total**: 3-4 weeks for v0.5.0 release

## ✅ Definition of Done

For each module:
- [ ] `.wj` file created in `std/`
- [ ] Wraps appropriate Rust crate
- [ ] Simple, intuitive API
- [ ] Types re-exported
- [ ] Example created
- [ ] Tests written
- [ ] Documented in `std/README.md`

For release:
- [ ] All priority modules implemented
- [ ] Examples build and run
- [ ] Documentation complete
- [ ] All tests passing
- [ ] Real-world app demonstrates usefulness

---

**Status**: Planning complete, ready to begin implementation  
**Target**: v0.5.0 release in 3-4 weeks  
**Branch**: `feature/v0.5.0-expanded-stdlib`
