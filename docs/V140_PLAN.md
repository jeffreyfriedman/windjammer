# Windjammer v0.14.0 Development Plan

**Theme**: "Stdlib Abstraction & Project Management"  
**Status**: In Progress  
**Started**: October 8, 2025

---

## 🎯 Core Objective

**Fix the abstraction problem in stdlib and add project management tooling.**

### Critical Issue Identified

**v0.13.0 stdlib modules leaked implementation details:**
- ❌ Users had to call `sqlx::SqlitePool::connect()` directly
- ❌ Users had to call `reqwest::get()` directly  
- ❌ Users had to call `chrono::Utc::now()` directly
- ❌ Breaking changes in underlying crates break user code
- ❌ No API stability guarantees

**v0.14.0 fixes this with proper abstractions:**
- ✅ Users call `db.connect()` (Windjammer API)
- ✅ Users call `http::get()` (Windjammer API)
- ✅ Users call `time.utc_now()` (Windjammer API)
- ✅ Can swap underlying crates without breaking user code
- ✅ Windjammer controls API contracts and stability

---

## 📋 Phase 1: Stdlib Abstraction Layer (CRITICAL)

### 1.1 Architecture Documentation ✅

**Status**: Completed

- [x] Created `docs/STDLIB_ARCHITECTURE.md`
- [x] Defined abstraction principles
- [x] Documented API design patterns
- [x] Established stdlib standards

### 1.2 Abstraction Implementations ✅

**Status**: Completed

#### std/db ✅
- [x] Abstract over `sqlx`
- [x] Public API: `db.connect()`, `Connection.execute()`, `QueryBuilder.bind()`, etc.
- [x] Users never see `sqlx::` in their code

#### std/json ✅
- [x] Abstract over `serde_json`
- [x] Public API: `json::parse()`, `json::stringify()`, `json::pretty()`, `Value` type
- [x] Users never see `serde_json::` in their code

#### std/http ✅
- [x] Abstract over `reqwest`
- [x] Public API: `http::get()`, `http::post()`, `Response`, `RequestBuilder`
- [x] Users never see `reqwest::` in their code

#### std/time ✅
- [x] Abstract over `chrono`
- [x] Public API: `time.now()`, `time.utc_now()`, `DateTime`, formatting
- [x] Users never see `chrono::` in their code

#### std/crypto ✅
- [x] Abstract over `base64`, `bcrypt`, `sha2`
- [x] Public API: `crypto.base64_encode()`, `crypto.hash_password()`, `crypto.sha256()`
- [x] Users never see underlying crates in their code

#### std/random ✅
- [x] Abstract over `rand`
- [x] Public API: `random.range()`, `random.shuffle()`, `random.choice()`
- [x] Users never see `rand::` in their code

### 1.3 Parser Improvements (Required for Full Implementation)

**Status**: Pending

These parser improvements are needed to make the abstractions fully functional:

#### Nested Path Parsing
- [ ] Parse `sqlx::SqlitePool::connect()`
- [ ] Parse `chrono::DateTime::parse_from_str()`
- [ ] Handle multi-level `::` paths

#### Turbofish in Nested Paths
- [ ] Parse `response.json::<User>()`
- [ ] Parse `Value::from_str::<MyType>()`
- [ ] Handle turbofish after `::` qualified paths

#### Generic Type Annotations
- [ ] Parse `Result<T, E>` in function signatures
- [ ] Parse `Option<Vec<String>>` nested generics
- [ ] Improve type parameter parsing

**Implementation Location**: `src/parser.rs`

**Approach**:
1. Extend `parse_type()` to handle deeply nested `::` paths
2. Extend postfix operator loop to handle turbofish after `::`
3. Improve generic type parameter parsing recursively

---

## 📋 Phase 2: Project Management Tooling

### 2.1 `wj new` Command

**Status**: In Progress

**Goal**: Scaffold new Windjammer projects

**Templates**:
1. **cli** - Command-line application
2. **web** - Web service (using http server)
3. **lib** - Library crate
4. **wasm** - WebAssembly application

**Implementation**:
- [ ] Create `src/cli/new.rs`
- [ ] Create template files in `templates/`
- [ ] Implement project scaffolding logic
- [ ] Add to `src/cli/mod.rs`

**Usage**:
```bash
wj new my-app
wj new my-lib --template lib
wj new my-wasm --template wasm
```

**Generated Structure**:
```
my-app/
├── src/
│   └── main.wj
├── wj.toml
├── .gitignore
└── README.md
```

### 2.2 `wj.toml` Configuration Format

**Status**: Pending

**Goal**: Windjammer-native configuration file

**Format**:
```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2025"

[dependencies]
# Windjammer stdlib (auto-included)
# User dependencies:
reqwest = "0.11"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
criterion = "0.5"

[profile.release]
opt-level = 3

[target.wasm]
enabled = true
```

**Implementation**:
- [ ] Create `wj.toml` parser (use `toml` crate)
- [ ] Add `src/config.rs` for config handling
- [ ] Implement `wj.toml` → `Cargo.toml` translation
- [ ] Update `src/main.rs` to read `wj.toml` first

**Translation Logic**:
- Windjammer stdlib modules are auto-added as Rust crates
- User dependencies pass through
- Target-specific config translates to Cargo features

### 2.3 `wj add` Command

**Status**: Pending

**Goal**: Add dependencies to `wj.toml`

**Implementation**:
- [ ] Create `src/cli/add.rs`
- [ ] Parse dependency specification
- [ ] Update `wj.toml`
- [ ] Regenerate `Cargo.toml`
- [ ] Run `cargo update` if needed

**Usage**:
```bash
wj add reqwest
wj add serde --features derive
wj add tokio --version 1.0
```

### 2.4 `wj remove` Command

**Status**: Pending

**Goal**: Remove dependencies from `wj.toml`

**Implementation**:
- [ ] Create `src/cli/remove.rs`
- [ ] Find and remove dependency from `wj.toml`
- [ ] Regenerate `Cargo.toml`
- [ ] Run `cargo update`

**Usage**:
```bash
wj remove reqwest
```

---

## 📋 Phase 3: Examples and Documentation

### 3.1 Update Examples

**Status**: Pending

All examples must use **only Windjammer APIs**, not underlying crates.

#### Examples to Update:

- [ ] `examples/41_json/main.wj` - Use `json::parse()`, not `serde_json::`
- [ ] `examples/42_http_client/main.wj` - Use `http::get()`, not `reqwest::`
- [ ] `examples/43_time/main.wj` - Use `time.now()`, not `chrono::`
- [ ] `examples/44_crypto/main.wj` - Use `crypto.sha256()`, not `sha2::`
- [ ] `examples/45_database/main.wj` - Use `db.connect()`, not `sqlx::`

#### New Examples to Create:

- [ ] `examples/46_wj_new_demo/` - Demonstrate `wj new` command
- [ ] `examples/47_full_web_api/` - Full REST API using stdlib only
- [ ] `examples/48_cli_app/` - CLI app using stdlib only

### 3.2 Documentation Updates

**Status**: Pending

#### `docs/GUIDE.md`

- [ ] Add "Standard Library" section
- [ ] Document each stdlib module's API
- [ ] Show usage examples
- [ ] Explain abstraction philosophy

#### `README.md`

- [ ] Add stdlib API overview
- [ ] Update feature list (mention abstractions)
- [ ] Add `wj new`, `wj add`, `wj remove` to commands
- [ ] Update installation instructions

#### `CHANGELOG.md`

- [ ] Document v0.14.0 changes
- [ ] List breaking changes (old APIs removed)
- [ ] Explain abstraction rationale
- [ ] Migration guide

### 3.3 Project Templates

**Status**: In Progress

#### Template: CLI Application

```
templates/cli/
├── main.wj
├── wj.toml
├── .gitignore
└── README.md
```

**main.wj**:
```windjammer
use std::env
use std::process

fn main() {
    let args = env.args()
    if args.len() < 2 {
        println!("Usage: {} <name>", args[0])
        process.exit(1)
    }
    
    println!("Hello, {}!", args[1])
}
```

#### Template: Web Application

```
templates/web/
├── main.wj
├── wj.toml
├── .gitignore
└── README.md
```

**main.wj**:
```windjammer
use std::http
use std::json

@derive(Serialize, Deserialize)
struct User {
    id: int,
    name: string
}

@async
fn main() {
    // Start HTTP server (future feature)
    println!("Server started on http://localhost:8080")
}
```

#### Template: Library

```
templates/lib/
├── lib.wj
├── wj.toml
├── .gitignore
└── README.md
```

**lib.wj**:
```windjammer
// Public API for your library

pub fn hello(name: string) -> string {
    "Hello, ${name}!"
}

#[test]
fn test_hello() {
    assert_eq!(hello("World"), "Hello, World!")
}
```

#### Template: WASM

```
templates/wasm/
├── main.wj
├── wj.toml
├── .gitignore
├── README.md
└── www/
    └── index.html
```

**main.wj**:
```windjammer
@export
fn greet(name: string) -> string {
    "Hello from Windjammer, ${name}!"
}
```

---

## 📋 Phase 4: Testing and Validation

### 4.1 Stdlib Abstraction Tests

- [ ] Test that examples compile without direct crate usage
- [ ] Test that generated Rust code is correct
- [ ] Test that abstractions work end-to-end
- [ ] Test error handling in abstractions

### 4.2 Tooling Tests

- [ ] Test `wj new` for all templates
- [ ] Test `wj add` / `wj remove`
- [ ] Test `wj.toml` parsing and translation
- [ ] Test that generated projects work

### 4.3 Parser Tests

- [ ] Test nested path parsing (`sqlx::Pool::connect`)
- [ ] Test turbofish in paths (`value.json::<T>()`)
- [ ] Test complex generic types (`Result<Vec<T>, Error>`)

### 4.4 Integration Tests

- [ ] Build and run all examples
- [ ] Verify no crate names leak
- [ ] Check documentation accuracy
- [ ] Run formatters and linters

---

## 🎯 Success Criteria

### Must-Have for v0.14.0 Release

1. ✅ **All stdlib modules have proper abstractions**
   - No `sqlx::`, `reqwest::`, `chrono::`, etc. in user code
   - Clean Windjammer APIs for all modules

2. ⏳ **Project management tooling works**
   - `wj new` scaffolds projects
   - `wj add` / `wj remove` manage dependencies
   - `wj.toml` is the source of truth

3. ⏳ **Parser improvements complete**
   - Nested path parsing works
   - Turbofish in paths works
   - Generic type annotations improved

4. ⏳ **All examples updated**
   - Use only Windjammer APIs
   - Demonstrate proper abstractions
   - Work end-to-end

5. ⏳ **Documentation complete**
   - `docs/GUIDE.md` has stdlib API docs
   - `README.md` updated
   - `CHANGELOG.md` documents breaking changes

### Nice-to-Have (Can Defer to v0.15.0)

- Escape hatches for direct crate access (`conn.inner_sqlx_pool()`)
- HTTP server abstraction (currently only client)
- More stdlib modules (regex, cli, etc.)
- `wj migrate` command for updating old projects

---

## 🚧 Breaking Changes

**v0.14.0 introduces intentional breaking changes** to fix abstraction leaks.

### Migration Required

**Old (v0.13.0):**
```windjammer
let pool = sqlx::SqlitePool::connect("...").await?
let response = reqwest::get("...").await?
let now = chrono::Utc::now()
```

**New (v0.14.0):**
```windjammer
let conn = db.connect("...").await?
let response = http::get("...").await?
let now = time.utc_now()
```

### Rationale

- **Stability**: Windjammer controls API, not external crates
- **Flexibility**: Can swap implementations later
- **Simplicity**: Users learn one API, not many crate APIs
- **Philosophy**: 80/20 - provide curated functionality

---

## 📅 Timeline Estimate

| Phase | Estimated Time | Status |
|-------|---------------|--------|
| Phase 1: Stdlib Abstractions | 2-3 days | ✅ Completed |
| Phase 2: Project Management | 2-3 days | ⏳ In Progress |
| Phase 3: Examples & Docs | 1-2 days | ⏳ Pending |
| Phase 4: Testing & Validation | 1 day | ⏳ Pending |
| **Total** | **6-9 days** | |

---

## 🎉 Expected Impact

### For Users

- ✅ **Simpler APIs** - Learn Windjammer, not 5+ Rust crates
- ✅ **Better stability** - API changes controlled by Windjammer
- ✅ **Faster onboarding** - `wj new my-app` and start coding
- ✅ **Dependency management** - `wj add reqwest` instead of editing files

### For Windjammer Project

- ✅ **Professional tooling** - Feels like a real language, not a toy
- ✅ **API control** - We own the contracts, not external crates
- ✅ **Future flexibility** - Can swap implementations as needed
- ✅ **80/20 philosophy** - Curated, simple APIs for common cases

---

## 📊 Post-Release Validation

After v0.14.0 release, validate:

1. **No Crate Leakage**: Search all examples for `::` - should only see `std::` or Windjammer modules
2. **User Feedback**: Are abstractions sufficient? What's missing?
3. **Documentation Quality**: Can new users understand stdlib APIs?
4. **Tooling Adoption**: Are users using `wj new`, `wj add`?

---

## 🔮 Looking Ahead: v0.15.0

After v0.14.0, focus areas:

- **HTTP Server Abstraction** - `http.serve()` for web apps
- **Advanced Tooling** - `wj watch`, `wj docs`
- **More Stdlib Modules** - regex, cli, log (properly abstracted)
- **Performance** - Optimize compilation, reduce binary size
- **Editor Integration** - LSP improvements

---

**Status**: In Progress  
**Expected Completion**: Mid-October 2025  
**Current Priority**: Implement `wj new` command and templates
