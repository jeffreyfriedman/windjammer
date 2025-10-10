# Windjammer v0.12.0 Development Plan

## 🎯 Theme: Web & Data - Batteries Included for Real Apps

**Goal**: Expand stdlib with practical modules for building real-world applications (web, JSON, databases)

**Timeline**: 2-3 weeks  
**Branch**: `feature/v0.12.0-stdlib-web`

---

## 📋 Features Overview

| Feature | Priority | Complexity | Status |
|---------|----------|------------|--------|
| std/json | High | Low | 🔜 Planned |
| std/http | High | Medium | 🔜 Planned |
| std/db | Medium | Medium | 🔜 Planned |
| std/time | Medium | Low | 🔜 Planned |
| std/crypto | Low | Low | 🔜 Planned |
| Pattern Matching Sugar | Medium | Low | 🔜 Planned |

---

## 🏗️ Phase 1: JSON Support (std/json)

### Goal

First-class JSON support for web APIs and configuration.

```windjammer
use std.json

fn main() {
    // Parse JSON
    let data = json.parse("{\"name\": \"Alice\", \"age\": 30}")
    match data {
        Ok(value) => println!("Parsed: {:?}", value),
        Err(err) => eprintln!("Error: {}", err)
    }
    
    // Serialize to JSON
    let user = User { name: "Bob", age: 25 }
    let json_str = json.stringify(user)
    
    // Pretty print
    let pretty = json.pretty(user)
}
```

### Implementation

**Wrap serde_json:**
- `parse(s: string) -> Result<Value, Error>` - Parse JSON string
- `stringify<T: Serialize>(value: T) -> string` - Serialize to JSON
- `pretty<T: Serialize>(value: T) -> string` - Pretty-print JSON
- `from_value<T: Deserialize>(value: Value) -> Result<T, Error>` - Convert Value to type
- `to_value<T: Serialize>(value: T) -> Value` - Convert type to Value

**Auto-add serde dependency** when `std.json` is imported.

---

## 🏗️ Phase 2: HTTP Client (std/http)

### Goal

Simple HTTP requests for APIs and web scraping.

```windjammer
use std.http

@async
fn main() {
    // GET request
    match http.get("https://api.example.com/users").await {
        Ok(response) => {
            println!("Status: {}", response.status)
            println!("Body: {}", response.text)
        }
        Err(err) => eprintln!("Error: {}", err)
    }
    
    // POST with JSON
    let data = json!({ "name": "Alice" })
    let response = http.post("https://api.example.com/users")
        .json(data)
        .send()
        .await?
    
    // Headers
    let response = http.get("https://api.example.com/data")
        .header("Authorization", "Bearer token")
        .send()
        .await?
}
```

### Implementation

**Wrap reqwest:**
- `get(url: string) -> Request` - Start GET request
- `post(url: string) -> Request` - Start POST request
- `Request` methods:
  - `header(key: string, value: string) -> Request`
  - `json<T: Serialize>(data: T) -> Request`
  - `send() -> Result<Response, Error>` (async)
- `Response` struct:
  - `status: int`
  - `text: string`
  - `json<T: Deserialize>() -> Result<T, Error>`

**Auto-add reqwest dependency** when `std.http` is imported.

---

## 🏗️ Phase 3: Database Access (std/db)

### Goal

Simple database queries with connection pooling.

```windjammer
use std.db

@async
fn main() {
    // Connect to database
    let db = db.connect("postgres://user:pass@localhost/mydb").await?
    
    // Query
    let users = db.query("SELECT * FROM users WHERE age > $1", vec![18]).await?
    for row in users {
        let name: string = row.get("name")
        let age: int = row.get("age")
        println!("{}: {}", name, age)
    }
    
    // Execute (INSERT, UPDATE, DELETE)
    db.execute("INSERT INTO users (name, age) VALUES ($1, $2)", 
               vec!["Alice", 30]).await?
    
    // Transaction
    let tx = db.transaction().await?
    tx.execute("UPDATE accounts SET balance = balance - 100 WHERE id = 1").await?
    tx.execute("UPDATE accounts SET balance = balance + 100 WHERE id = 2").await?
    tx.commit().await?
}
```

### Implementation

**Wrap sqlx:**
- `connect(url: string) -> Result<Database, Error>` (async)
- `Database` methods:
  - `query(sql: string, params: Vec<Value>) -> Result<Vec<Row>, Error>` (async)
  - `execute(sql: string, params: Vec<Value>) -> Result<int, Error>` (async)
  - `transaction() -> Result<Transaction, Error>` (async)
- `Row` methods:
  - `get<T>(column: string) -> T`
- `Transaction` methods:
  - `execute(sql: string, params: Vec<Value>) -> Result<int, Error>` (async)
  - `commit()` (async)
  - `rollback()` (async)

**Auto-add sqlx dependency** when `std.db` is imported.

---

## 🏗️ Phase 4: Time & Date (std/time)

### Goal

Ergonomic time and date handling.

```windjammer
use std.time

fn main() {
    // Current time
    let now = time.now()
    println!("Now: {}", now)
    
    // Duration
    let duration = time.seconds(30)
    let later = now + duration
    
    // Parsing
    let date = time.parse("2025-10-07", "%Y-%m-%d")?
    
    // Formatting
    let formatted = now.format("%Y-%m-%d %H:%M:%S")
    
    // Unix timestamp
    let timestamp = now.timestamp()
}
```

### Implementation

**Wrap chrono:**
- `now() -> DateTime` - Current time
- `parse(s: string, format: string) -> Result<DateTime, Error>`
- `seconds(n: int) -> Duration`
- `minutes(n: int) -> Duration`
- `hours(n: int) -> Duration`
- `DateTime` methods:
  - `format(format: string) -> string`
  - `timestamp() -> int`
  - `+(duration: Duration) -> DateTime`
  - `-(duration: Duration) -> DateTime`

**Auto-add chrono dependency** when `std.time` is imported.

---

## 🏗️ Phase 5: Crypto & Hashing (std/crypto)

### Goal

Common cryptographic operations.

```windjammer
use std.crypto

fn main() {
    // Hash
    let hash = crypto.sha256("hello world")
    println!("SHA-256: {}", hash)
    
    // Password hashing (bcrypt)
    let hashed_password = crypto.hash_password("my_password")
    let is_valid = crypto.verify_password("my_password", hashed_password)
    
    // Random bytes
    let random_bytes = crypto.random_bytes(32)
    
    // Base64
    let encoded = crypto.base64_encode("hello")
    let decoded = crypto.base64_decode(encoded)?
}
```

### Implementation

**Wrap sha2, bcrypt, base64:**
- `sha256(data: string) -> string`
- `sha512(data: string) -> string`
- `hash_password(password: string) -> string`
- `verify_password(password: string, hash: string) -> bool`
- `random_bytes(n: int) -> Vec<u8>`
- `base64_encode(data: string) -> string`
- `base64_decode(data: string) -> Result<string, Error>`

---

## 🏗️ Phase 6: Pattern Matching Sugar

### Goal

More ergonomic patterns for common cases.

### If-Let Expression

```windjammer
// Current (verbose):
let value = match option {
    Some(x) => x,
    None => default
}

// New (concise):
let value = if let Some(x) = option { x } else { default }
```

### Match with Default

```windjammer
// Current:
match value {
    1 => "one",
    2 => "two",
    _ => "other"
}

// New (sugar):
match value {
    1 => "one",
    2 => "two",
    else => "other"  // More intuitive than _
}
```

---

## ✅ Success Criteria

### Must Have (Release Blockers)
- [ ] std/json working (parse, stringify, pretty)
- [ ] std/http working (get, post, headers)
- [ ] std/db working (basic queries, transactions)
- [ ] std/time working (now, parse, format)
- [ ] Examples 41-45 (one per new module)
- [ ] Zero test failures
- [ ] Zero clippy warnings
- [ ] Documentation updates

### Should Have (Nice to Have)
- [ ] std/crypto
- [ ] Pattern matching sugar (if-let, else)
- [ ] Performance benchmarks

---

## 🧪 Testing Strategy

### JSON Tests
```windjammer
use std.json

@test
fn test_parse_json() {
    let result = json.parse("{\"name\": \"Alice\"}")
    assert!(result.is_ok())
}

@test
fn test_stringify() {
    let user = User { name: "Bob" }
    let json = json.stringify(user)
    assert!(json.contains("Bob"))
}
```

### HTTP Tests (integration)
```windjammer
use std.http

@async
@test
fn test_http_get() {
    let response = http.get("https://httpbin.org/get").await?
    assert_eq!(response.status, 200)
}
```

---

## 📊 Development Phases

### Week 1: JSON & HTTP
- **Days 1-2**: std/json (parse, stringify, integration)
- **Days 3-5**: std/http (GET, POST, headers, async)
- **Milestone**: JSON and HTTP working, examples 41-42

### Week 2: Database & Time
- **Days 6-8**: std/db (queries, transactions, connection pooling)
- **Days 9-10**: std/time (now, parse, format, durations)
- **Milestone**: Database and time working, examples 43-44

### Week 3: Polish & Ship
- **Days 11-12**: std/crypto (optional), pattern sugar
- **Days 13-15**: Examples, documentation, testing
- **Milestone**: Ready to merge

---

## 🔮 Looking Ahead: Post-v0.12.0

### v0.13.0: Tooling & Developer Experience

**Idea: Unified Windjammer CLI** (from user feedback)

Instead of juggling `cargo`, `rustc`, `clippy`, `rustfmt`, wrap them in a branded Windjammer experience:

```bash
# Current (multiple tools):
wj build --path main.wj --output ./out
cd out && cargo run
cargo test
cargo clippy

# Proposed (unified CLI):
wj build main.wj              # Build and run
wj test                       # Run tests
wj lint                       # Run clippy
wj fmt                        # Format code
wj new my-project             # Scaffold new project
wj add serde                  # Add dependency
wj docs                       # Open documentation
wj upgrade                    # Update Windjammer
```

**Benefits:**
- ✅ **Branding**: "Windjammer" identity, not just "transpiler to Rust"
- ✅ **Simplicity**: One tool instead of many
- ✅ **Better errors**: Translate Rust errors to Windjammer context
- ✅ **Project scaffolding**: `wj new` creates idiomatic projects
- ✅ **Dependency management**: Hide Cargo.toml complexity
- ✅ **LSP integration**: `wj lsp` for editor support

**Implementation:**
- Wrap existing Rust tools (cargo, rustc, clippy, rustfmt)
- Enhance output with Windjammer-friendly messages
- Add project templates and scaffolding
- Consider rust-analyzer integration for LSP

**Complexity:** Medium (mostly CLI work, less compiler work)

**Value:** High (better DX, stronger brand, easier onboarding)

**ROI:** ⭐⭐⭐⭐⭐

---

### Other Future Ideas (v0.13.0+)

**Macro System:**
```windjammer
macro debug_print(expr) {
    println!("{} = {:?}", stringify!(expr), expr)
}

debug_print!(my_variable)  // Expands to println!
```

**More Stdlib:**
- `std/regex` - Regular expressions
- `std/csv` - CSV parsing
- `std/xml` - XML parsing
- `std/log` - Structured logging
- `std/config` - Configuration files (TOML, YAML)

**Language Features:**
- Default parameters: `fn greet(name: string = "World") { ... }`
- Spread operator: `let combined = [...list1, ...list2]`
- String methods: `"hello".uppercase()`, `"test".contains("es")`

---

## 📚 References

- `docs/V110_PLAN.md` - Previous version plan
- Rust crates: serde_json, reqwest, sqlx, chrono, sha2, bcrypt
- Similar languages: TypeScript (tooling), Python (batteries included)

---

## 🚀 Release Checklist

Before merging to main:

- [ ] All stdlib modules implemented and tested
- [ ] Examples 41-45 work
- [ ] Zero test failures
- [ ] Zero clippy warnings
- [ ] `cargo fmt --all` clean
- [ ] Documentation updated:
  - [ ] README.md
  - [ ] CHANGELOG.md
  - [ ] GUIDE.md
- [ ] Dependency management working (auto-add serde, reqwest, etc.)
- [ ] PR comment prepared
- [ ] Release notes written

---

**Status**: Planning Complete ✅  
**Next Step**: Begin Day 1 - std/json  
**Branch**: `feature/v0.12.0-stdlib-web`  
**Target Release**: 2-3 weeks from start

**Core Philosophy**: Continue **batteries-included** approach - make building real apps easy out of the box. Web APIs, databases, and JSON are essential for 80% of modern applications.

