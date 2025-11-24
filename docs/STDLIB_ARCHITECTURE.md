# Windjammer Standard Library Architecture

**Version:** 0.14.0  
**Status:** Active  
**Last Updated:** October 8, 2025

---

## 🎯 Core Principle: Abstraction Over Implementation

**Windjammer stdlib modules MUST NOT leak underlying crate APIs.**

### Why This Matters

1. **API Stability** - We control breaking changes, not external crates
2. **Flexibility** - Can swap implementations without breaking user code
3. **Simplicity** - Users learn Windjammer APIs, not Rust crate APIs
4. **Curation** - We provide the 80% of functionality users need

### Bad Example (Leaking Implementation)

```windjammer
use std::http

// ❌ BAD: Exposing reqwest directly
let response = reqwest::get("https://api.example.com").await?
```

**Problems:**
- User depends on reqwest API
- Breaking changes in reqwest break user code
- Can't swap reqwest for another crate
- User needs to learn reqwest documentation

### Good Example (Proper Abstraction)

```windjammer
use std::http

// ✅ GOOD: Windjammer API
let response = http::get("https://api.example.com").await?
```

**Benefits:**
- User depends on Windjammer API
- We control API stability
- Can swap reqwest for hyper/curl/etc later
- User only needs Windjammer documentation

---

## 📋 Module Architecture Standards

### Template for All Stdlib Modules

```windjammer
// std/module.wj

// PUBLIC API - What users interact with
// All types, functions, and methods are Windjammer-defined

struct Connection {
    // PRIVATE: Implementation details
    // Never expose underlying crate types in public API
}

fn connect(url: string) -> Result<Connection, Error> {
    // IMPLEMENTATION: Wraps underlying crate
    // Users never see this
}

impl Connection {
    fn query(self, sql: string) -> QueryResult {
        // Wraps underlying crate functionality
    }
}

// RULE: Users should NEVER see the underlying crate name in their code
```

### Key Rules

1. **No Public Crate Exposure** - Underlying crates are implementation details
2. **Windjammer Types** - All public APIs use Windjammer-defined types
3. **Error Handling** - Wrap crate errors in Windjammer error types
4. **80/20 Coverage** - Provide the 80% of functionality users need
5. **Escape Hatch** - Advanced users can access underlying crate if needed

---

## 🏗️ Module-by-Module Architecture

### std/json

**Implementation:** serde + serde_json  
**Public API:**

```windjammer
// User-facing API
fn parse(s: string) -> Result<Value, JsonError>
fn stringify<T>(value: T) -> Result<string, JsonError>
fn pretty<T>(value: T) -> Result<string, JsonError>

struct Value {
    // Wraps serde_json::Value internally
}

impl Value {
    fn as_object(self) -> Option<Object>
    fn as_array(self) -> Option<Array>
    fn as_string(self) -> Option<string>
    fn get(self, key: string) -> Option<Value>
}
```

**Usage:**
```windjammer
use std::json

let data = json::parse("{\"name\": \"Alice\"}")?
let name = data.get("name")?.as_string()?
println!("Name: {}", name)
```

**NOT:**
```windjammer
// ❌ Don't expose serde_json
let data = serde_json::from_str("{\"name\": \"Alice\"}")?
```

---

### std/http

**Implementation:** reqwest + tokio  
**Public API:**

```windjammer
@async
fn get(url: string) -> Result<Response, HttpError>

@async
fn post(url: string) -> RequestBuilder

struct Response {
    status: int,
    headers: Headers,
}

impl Response {
    @async
    fn text(self) -> Result<string, HttpError>
    
    @async
    fn json<T>(self) -> Result<T, HttpError>
}

struct RequestBuilder {
    // Private: wraps reqwest::RequestBuilder
}

impl RequestBuilder {
    fn header(self, key: string, value: string) -> RequestBuilder
    fn json<T>(self, body: T) -> RequestBuilder
    @async
    fn send(self) -> Result<Response, HttpError>
}
```

**Usage:**
```windjammer
use std::http

@async
fn main() {
    let response = http::get("https://api.example.com").await?
    println!("Status: {}", response.status)
    let body = response.text().await?
}
```

---

### std/db

**Implementation:** sqlx + tokio  
**Public API:**

```windjammer
@async
fn connect(url: string) -> Result<Connection, DbError>

struct Connection {
    // Private: wraps sqlx::Pool
}

impl Connection {
    @async
    fn execute(self, sql: string) -> QueryBuilder
    
    @async
    fn query(self, sql: string) -> QueryBuilder
    
    @async
    fn transaction(self) -> Result<Transaction, DbError>
}

struct QueryBuilder {
    // Private: wraps sqlx::Query
}

impl QueryBuilder {
    fn bind<T>(self, value: T) -> QueryBuilder
    
    @async
    fn fetch_all(self) -> Result<Vec<Row>, DbError>
    
    @async
    fn fetch_one(self) -> Result<Row, DbError>
}

struct Row {
    // Private: wraps sqlx::Row
}

impl Row {
    fn get<T>(self, index: int) -> Result<T, DbError>
    fn get_by_name<T>(self, name: string) -> Result<T, DbError>
}
```

**Usage:**
```windjammer
use std::db

@async
fn main() {
    let conn = db.connect("sqlite::memory:").await?
    
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)").await?
    
    conn.execute("INSERT INTO users VALUES (?, ?)")
        .bind(1)
        .bind("Alice")
        .await?
    
    let rows = conn.query("SELECT * FROM users")
        .fetch_all()
        .await?
    
    for row in rows {
        let id = row.get::<int>(0)?
        let name = row.get::<string>(1)?
        println!("User: {} - {}", id, name)
    }
}
```

---

### std/time

**Implementation:** chrono  
**Public API:**

```windjammer
fn now() -> DateTime
fn utc_now() -> DateTime

struct DateTime {
    // Private: wraps chrono::DateTime
}

impl DateTime {
    fn timestamp(self) -> i64
    fn format(self, fmt: string) -> string
    fn year(self) -> int
    fn month(self) -> int
    fn day(self) -> int
    fn hour(self) -> int
    fn minute(self) -> int
    fn second(self) -> int
}

fn parse(s: string, format: string) -> Result<DateTime, TimeError>
```

**Usage:**
```windjammer
use std::time

fn main() {
    let now = time.now()
    println!("Current time: {}", now.format("%Y-%m-%d %H:%M:%S"))
    println!("Timestamp: {}", now.timestamp())
}
```

---

### std/crypto

**Implementation:** sha2, bcrypt, base64  
**Public API:**

```windjammer
// Base64
fn base64_encode(data: string) -> string
fn base64_decode(data: string) -> Result<string, CryptoError>

// Password hashing
fn hash_password(password: string) -> Result<string, CryptoError>
fn verify_password(password: string, hash: string) -> Result<bool, CryptoError>

// SHA-256
fn sha256(data: string) -> string
```

**Usage:**
```windjammer
use std::crypto

fn main() {
    let encoded = crypto.base64_encode("Hello, World!")
    println!("Encoded: {}", encoded)
    
    let hash = crypto.hash_password("my_password")?
    let valid = crypto.verify_password("my_password", &hash)?
    println!("Password valid: {}", valid)
}
```

---

### std/random

**Implementation:** rand  
**Public API:**

```windjammer
fn random<T>() -> T
fn range(min: int, max: int) -> int
fn float() -> float
fn bool() -> bool
fn shuffle<T>(list: Vec<T>) -> Vec<T>
fn choice<T>(list: Vec<T>) -> Option<T>
```

**Usage:**
```windjammer
use std::random

fn main() {
    let num = random.range(1, 100)
    let coin = random.bool()
    let items = vec![1, 2, 3, 4, 5]
    let shuffled = random.shuffle(items)
}
```

---

## 🔧 Implementation Strategy

### Phase 1: Create Abstract Types (v0.14.0)

For each module:
1. Define Windjammer types (Connection, Response, DateTime, etc.)
2. Define Windjammer functions (connect, get, now, etc.)
3. Write wrapper implementations

### Phase 2: Update Examples (v0.14.0)

- Update all examples to use Windjammer APIs only
- Remove all direct crate usage
- Test that abstractions work

### Phase 3: Documentation (v0.14.0)

- Document each module's API
- Add to GUIDE.md
- Update README.md

---

## 🎯 Escape Hatch (Advanced Users)

For users who need direct access to underlying crates:

```windjammer
use std::db

// Standard abstraction (recommended)
let conn = db.connect("...").await?

// Advanced: Access underlying sqlx (not recommended)
let pool = conn.inner_sqlx_pool()  // Returns sqlx::Pool
```

**Guidelines:**
- Escape hatches should be explicitly named (`inner_*`)
- Documentation should warn about stability
- 95% of users should never need this

---

## 📊 Success Metrics

### Abstraction Quality Checklist

For each stdlib module:

- [ ] ✅ No crate names in user code
- [ ] ✅ All public types are Windjammer-defined
- [ ] ✅ All public functions use Windjammer types
- [ ] ✅ Errors are wrapped in Windjammer error types
- [ ] ✅ Examples use only Windjammer APIs
- [ ] ✅ Can theoretically swap implementation
- [ ] ✅ 80% use case coverage
- [ ] ✅ Documented in GUIDE.md

---

## 🚀 Migration Path (v0.13.0 → v0.14.0)

### Breaking Changes

v0.14.0 will introduce breaking changes to fix abstraction leaks:

**Old (v0.13.0):**
```windjammer
let response = reqwest::get("...").await?
let pool = sqlx::SqlitePool::connect("...").await?
let now = chrono::Utc::now()
```

**New (v0.14.0):**
```windjammer
let response = http::get("...").await?
let conn = db.connect("...").await?
let now = time.utc_now()
```

### Migration Strategy

1. **v0.14.0 Alpha** - Both APIs work (deprecated warnings)
2. **v0.14.0 Beta** - Old APIs removed, new APIs stable
3. **v0.14.0 Release** - Clean, abstracted APIs only

---

## 📝 Future Considerations

### When to Add New Stdlib Modules

Only add modules when:
1. ✅ Can provide clean abstraction
2. ✅ Covers 80% of use cases
3. ✅ Clear best-in-class underlying crate
4. ✅ Stable, well-maintained implementation

### When to Expose More Functionality

Only expand APIs when:
1. ✅ Clear user demand
2. ✅ Fits within abstraction model
3. ✅ Maintains simplicity
4. ✅ Doesn't leak implementation

---

## 🎉 Conclusion

**Proper abstractions are not optional - they're foundational.**

By v0.14.0, every stdlib module will:
- ✅ Hide implementation details
- ✅ Provide clean Windjammer APIs
- ✅ Allow future flexibility
- ✅ Maintain the 80/20 philosophy

**This is what makes Windjammer different from just "Rust with simpler syntax" - we provide a curated, stable, simple API surface.**

---

**Status:** Active  
**Enforcement:** All stdlib PRs must follow this architecture  
**Review:** Architecture team must approve API changes

