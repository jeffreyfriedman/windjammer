# Windjammer v0.13.0 Development Plan

**Theme:** CLI Tooling + Database Support  
**Status:** In Progress  
**Branch:** `feature/v0.13.0-cli-and-db`

---

## 🎯 Overview

v0.13.0 brings two major improvements to make Windjammer more practical for real-world development:

1. **Unified `wj` CLI** - Single tool for build/run/test/lint/fmt (no more juggling cargo commands)
2. **std/db Module** - SQL database access with sqlx (PostgreSQL, SQLite, MySQL)

Both features address key pain points and move Windjammer closer to production readiness.

---

## 📋 Feature List

### Part 1: Unified CLI (`wj` command)

**Goal:** Single, cohesive CLI that wraps and enhances Rust tooling

**Commands to implement:**
- [x] `wj build <file>` - Build Windjammer project
- [x] `wj run <file>` - Compile and execute (combines build + run)
- [x] `wj test` - Run tests (wraps cargo test)
- [x] `wj fmt` - Format code (wraps cargo fmt)
- [x] `wj lint` - Run linter (wraps cargo clippy)
- [ ] `wj check` - Type check without building (wraps cargo check)

**Deferred to v0.14.0:**
- `wj new <name>` - Scaffold new project
- `wj add <package>` - Add dependency
- `wj.toml` configuration format

**Deferred to v0.15.0:**
- `wj watch` - File watcher with auto-reload
- `wj docs` - Documentation browser
- Enhanced error mapping

### Part 2: Database Support (std/db)

**Goal:** First-class SQL database support with automatic dependency injection

**Features:**
- [x] Automatic sqlx dependency injection (with tokio)
- [x] Support for PostgreSQL, SQLite, MySQL via feature flags
- [x] Example demonstrating database queries
- [x] Connection pooling support
- [x] Basic query building

**Deferred:**
- Migrations (future enhancement)
- Advanced ORM features (not in 80/20 scope)
- Database-specific optimizations

---

## 🏗️ Implementation Strategy

### Phase 1: CLI Foundation (Week 1)

**Goal:** Basic `wj` command working with essential subcommands

**Tasks:**
1. Create new binary crate structure
2. Implement command-line argument parsing (clap)
3. Implement `wj build` - wrap existing wj build logic
4. Implement `wj run` - compile + execute
5. Implement `wj test` - wrap `cargo test`
6. Implement `wj fmt` - wrap `cargo fmt`
7. Implement `wj lint` - wrap `cargo clippy`

**Deliverables:**
- New `src/cli/` module
- `wj` binary that replaces `windjammer` command
- Backward compatibility: `windjammer` still works (deprecated)

### Phase 2: Database Module (Week 1-2)

**Goal:** Working SQL database support via std/db

**Tasks:**
1. Create `std/db.wj` module
2. Update dependency mapping for sqlx + tokio
3. Create example using SQLite (simplest to demo)
4. Test connection pooling
5. Document usage patterns

**Deliverables:**
- `std/db.wj` module
- Example 45: Database queries (SQLite)
- Automatic sqlx dependency injection

### Phase 3: Testing & Documentation (Week 2)

**Goal:** Polish and document new features

**Tasks:**
1. Test all CLI commands
2. Update README.md with `wj` examples
3. Update GUIDE.md with database section
4. Update CHANGELOG.md
5. Write release notes
6. Run full test suite

**Deliverables:**
- Complete documentation
- All examples working
- Release-ready branch

---

## 📁 File Structure

### New CLI Structure

```
src/
├── main.rs           # CLI entry point (wj command)
├── cli/
│   ├── mod.rs        # CLI module
│   ├── build.rs      # wj build
│   ├── run.rs        # wj run
│   ├── test.rs       # wj test
│   ├── fmt.rs        # wj fmt
│   └── lint.rs       # wj lint
├── compiler/         # Rename from lib parts
│   ├── mod.rs
│   ├── lexer.rs
│   ├── parser.rs
│   ├── analyzer.rs
│   ├── codegen.rs
│   └── inference.rs
└── lib.rs            # Library interface
```

### New Stdlib Module

```
std/
├── db.wj             # Database module (NEW)
├── json.wj
├── http.wj
├── time.wj
├── crypto.wj
└── ...
```

### New Example

```
examples/
├── 45_database/
│   └── main.wj       # SQLite demo
```

---

## 🎨 User Experience

### Before v0.13.0 (v0.12.0)

```bash
# Complex workflow
$ wj build --path main.wj --output ./build
$ cd build && cargo run
$ cd .. && cargo test
$ cargo fmt
$ cargo clippy
```

### After v0.13.0 (New Workflow)

```bash
# Simple workflow
$ wj run main.wj          # Build + run in one command
$ wj test                 # Run tests
$ wj fmt                  # Format code
$ wj lint                 # Lint code
```

### Database Example

```windjammer
use std.db

@derive(Serialize, Deserialize)
struct User {
    id: int,
    name: string,
    email: string
}

@async
fn main() {
    // SQLite example
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap()
    
    // Create table
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
        .execute(&pool)
        .await
        .unwrap()
    
    // Insert data
    sqlx::query("INSERT INTO users (name, email) VALUES (?, ?)")
        .bind("Alice")
        .bind("alice@example.com")
        .execute(&pool)
        .await
        .unwrap()
    
    // Query data
    let users = sqlx::query_as::<_, (i64, String, String)>("SELECT * FROM users")
        .fetch_all(&pool)
        .await
        .unwrap()
    
    for user in users {
        println!("User: {} - {}", user.1, user.2)
    }
}
```

---

## 🔧 Technical Details

### CLI Implementation

**Approach:** Thin wrapper around existing tools

```rust
// src/cli/run.rs
pub fn run(file: &Path) -> Result<()> {
    // 1. Compile Windjammer to Rust
    let output_dir = tempdir()?;
    build::compile(file, &output_dir)?;
    
    // 2. Run with cargo
    let status = Command::new("cargo")
        .arg("run")
        .current_dir(&output_dir)
        .status()?;
    
    if !status.success() {
        bail!("Execution failed");
    }
    
    Ok(())
}
```

**Benefits:**
- Reuses all existing build logic
- No code duplication
- Consistent with current implementation

### Database Module

**sqlx Integration:**

```toml
# Auto-generated dependencies for std.db
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite"] }
tokio = { version = "1", features = ["full"] }
```

**Feature flags** (future):
```bash
# User can specify database backend
wj build --features postgres  # PostgreSQL
wj build --features sqlite    # SQLite (default)
wj build --features mysql     # MySQL
```

---

## 🚦 Success Criteria

### CLI Success Metrics

- [ ] Users can build+run with `wj run main.wj` (one command)
- [ ] All cargo commands wrapped (test, fmt, lint)
- [ ] Error messages are clear and helpful
- [ ] Performance: CLI overhead < 100ms

### Database Success Metrics

- [ ] `std.db` import automatically adds sqlx
- [ ] Example 45 demonstrates CRUD operations
- [ ] Works with SQLite out of the box
- [ ] Connection pooling documented
- [ ] Users can build real database apps

---

## ⏱️ Timeline

**Total Estimated Time:** 1.5-2 weeks

| Phase | Tasks | Time | Status |
|-------|-------|------|--------|
| **CLI Foundation** | Basic commands | 3-4 days | 🟡 Planned |
| **Database Module** | sqlx integration | 3-4 days | 🟡 Planned |
| **Testing & Docs** | Polish + release | 2-3 days | 🟡 Planned |

---

## 🎯 Scope Decisions

### ✅ IN SCOPE (v0.13.0)

**CLI:**
- ✅ `wj build`, `wj run`, `wj test`, `wj fmt`, `wj lint`
- ✅ Wrap existing cargo commands
- ✅ Basic error handling
- ✅ Help text and usage

**Database:**
- ✅ std/db module (sqlx wrapper)
- ✅ SQLite support
- ✅ Example demonstrating queries
- ✅ Automatic dependency injection

### ⏸️ DEFERRED (v0.14.0+)

**CLI:**
- ⏸️ `wj new` (project scaffolding)
- ⏸️ `wj add` (dependency management)
- ⏸️ `wj.toml` configuration
- ⏸️ `wj watch` (file watcher)

**Database:**
- ⏸️ Migrations system
- ⏸️ Query builder DSL
- ⏸️ Advanced ORM features
- ⏸️ Database-agnostic abstractions

---

## 📊 Progress Tracking

### Milestones

- [ ] **M1:** CLI commands working (build, run, test, fmt, lint)
- [ ] **M2:** std/db module with automatic dependencies
- [ ] **M3:** Example 45 (database) working
- [ ] **M4:** Documentation complete
- [ ] **M5:** All tests passing, ready for release

### Implementation Checklist

**CLI:**
- [ ] Create `src/cli/` module structure
- [ ] Implement command parsing (clap)
- [ ] Implement `wj build`
- [ ] Implement `wj run`
- [ ] Implement `wj test`
- [ ] Implement `wj fmt`
- [ ] Implement `wj lint`
- [ ] Add help text for all commands
- [ ] Test error handling

**Database:**
- [ ] Create `std/db.wj`
- [ ] Add sqlx to dependency mapping (both locations)
- [ ] Create Example 45 (SQLite demo)
- [ ] Test connection pooling
- [ ] Document query patterns
- [ ] Test compilation with auto-deps

**Documentation:**
- [ ] Update README.md (replace `windjammer` with `wj`)
- [ ] Update GUIDE.md (add database section)
- [ ] Update CHANGELOG.md (v0.13.0 entry)
- [ ] Write release notes
- [ ] Update installation instructions

**Quality:**
- [ ] Run `cargo fmt --all`
- [ ] Run `cargo clippy` (0 warnings)
- [ ] Run `cargo test` (all passing)
- [ ] Test all examples (1-45)
- [ ] Manual CLI testing

---

## 🤔 Open Questions

### 1. CLI Binary Name

**Options:**
- A) `wj` (short, memorable)
- B) `windjammer` (explicit, but longer)
- C) Both (wj as alias)

**Decision:** Go with `wj` as primary, keep `windjammer` for backward compat (deprecated).

### 2. Database Default

**Options:**
- A) SQLite only (simplest)
- B) PostgreSQL (most common in production)
- C) All three (too complex)

**Decision:** SQLite as default (easiest setup), document how to use others.

### 3. Temp Directory for `wj run`

**Options:**
- A) `/tmp/wj-build-<hash>/` (ephemeral)
- B) `.wj/build/` in project (persistent)
- C) User-specified via flag

**Decision:** Option A for `wj run` (quick iteration), Option B for `wj build` (explicit output).

---

## 🎉 Expected Impact

### Developer Experience

**Before:**
```bash
# 5 commands, context switching
wj build --path main.wj --output ./build
cd build
cargo run
cd ..
cargo test
cargo fmt
```

**After:**
```bash
# 1 command for most workflows
wj run main.wj
wj test
wj fmt
```

**Result:** 80% reduction in command complexity!

### Database Development

**Before:**
- Manual Cargo.toml editing
- Complex sqlx setup
- No guidance

**After:**
```windjammer
use std.db
// SQLite works automatically!
```

**Result:** Zero-config database development!

---

## 📚 References

- **TOOLING_VISION.md** - Full CLI design doc
- **V120_PLAN.md** - Previous version's plan (std/db was deferred)
- **Rust cargo** - Command structure inspiration
- **Deno** - Single CLI tool inspiration
- **sqlx** - Database library we're wrapping

---

## 🚀 Let's Build!

v0.13.0 combines developer experience improvements (CLI) with practical functionality (databases). This is a high-value release that makes Windjammer significantly more pleasant to use.

**Philosophy alignment:**
- ✅ **80/20:** Essential CLI commands, skip advanced features
- ✅ **Batteries included:** Database support without manual setup
- ✅ **Progressive disclosure:** Simple default (SQLite), advanced options available

**Ready to implement!** 🎯

