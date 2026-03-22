# 🔍 Windjammer Philosophy Audit - TODO

**Priority:** 🔴 CRITICAL  
**Reason:** Multiple instances of Rust leakage discovered  
**Scope:** ALL Windjammer code

---

## 🎯 **What to Audit**

### 1. **Core Windjammer Language**
- [ ] All stdlib modules (`std/*`)
- [ ] All example code (`examples/*`)
- [ ] All documentation showing user-facing code
- [ ] README examples
- [ ] GUIDE.md examples

### 2. **windjammer-ui Crate**
- [ ] Public API exposed to users
- [ ] Example code
- [ ] Documentation
- [ ] Component definitions
- [ ] Event handlers

### 3. **windjammer-game-framework Crate**
- [ ] Public API exposed to users
- [ ] ECS API
- [ ] Rendering API
- [ ] Input API
- [ ] Example games
- [ ] Documentation

---

## 🚨 **What to Look For**

### **Rust Leakage (FORBIDDEN in user code)**

#### Borrowing Syntax
- ❌ `&` in function parameters
- ❌ `&mut` in function parameters
- ❌ `mut` keyword for parameters
- ❌ Lifetime annotations (`'a`, `'static`)
- ❌ Explicit borrowing in calls (`&x`, `&mut x`)

#### Crate Exposure
- ❌ `use axum::*`
- ❌ `use tokio::*`
- ❌ `use wgpu::*`
- ❌ `use winit::*`
- ❌ `use serde::*`
- ❌ ANY external crate in user code

#### Rust-Specific Concepts
- ❌ `unwrap()` / `expect()` in examples
- ❌ `Box<T>` / `Rc<T>` / `Arc<T>` exposed
- ❌ `Pin<T>` / `Unpin`
- ❌ Trait bounds in user code
- ❌ `dyn Trait` syntax
- ❌ `impl Trait` in user signatures

#### Complex Patterns
- ❌ Pattern matching on `Result<T, E>` (should use `?`)
- ❌ Manual `match` on `Option<T>` (should use `?` or methods)
- ❌ Explicit type annotations everywhere
- ❌ Turbofish (`::<T>`) in user code

---

## ✅ **What SHOULD Be There**

### **Pure Windjammer Patterns**

#### Clean Syntax
- ✅ `fn update(game: PongGame, delta: float)` (compiler infers &mut)
- ✅ `use std::http` (not `use axum`)
- ✅ `use std::db` (not `use sqlx`)
- ✅ `Result<T, E>` with `?` operator
- ✅ Simple struct definitions
- ✅ Clean function calls

#### Automatic Inference
- ✅ Compiler adds `&` / `&mut` automatically
- ✅ Compiler adds `.clone()` automatically
- ✅ Compiler converts `"string"` to `String` automatically
- ✅ No manual memory management

#### Decorators (Not Traits)
- ✅ `@derive(Clone, Debug)`
- ✅ `@get("/users")`
- ✅ `@game`, `@update`, `@render`
- ❌ NOT: `impl Trait for Struct`

---

## 📋 **Audit Checklist**

### **Phase 1: Core Language**

#### stdlib modules
- [ ] `std/http/mod.wj` - Check for axum leakage
- [ ] `std/db/mod.wj` - Check for sqlx leakage
- [ ] `std/json/mod.wj` - Check for serde leakage
- [ ] `std/fs/mod.wj` - Check for std::fs leakage
- [ ] `std/path/mod.wj` - Check for std::path leakage
- [ ] `std/io/mod.wj` - Check for std::io leakage
- [ ] `std/sync/mod.wj` - Check for Arc/Mutex leakage
- [ ] `std/thread/mod.wj` - Check for std::thread leakage
- [ ] `std/collections/mod.wj` - Check for HashMap leakage
- [ ] `std/game/mod.wj` - Check for wgpu/winit leakage

#### Examples
- [ ] `examples/http_server/main.wj`
- [ ] `examples/cli_tool/main.wj`
- [ ] `examples/hello_world/main.wj`
- [ ] `examples/wjfind/src/*.wj`
- [ ] `examples/wschat/src/*.wj`
- [ ] `examples/games/pong/main.wj`

#### Documentation
- [ ] `README.md` - All code examples
- [ ] `docs/GUIDE.md` - All code examples
- [ ] `docs/COMPARISON.md` - All code examples

### **Phase 2: windjammer-ui**

#### Public API
- [ ] `crates/windjammer-ui/src/lib.rs` - Exported types
- [ ] Component API - Check for React/Vue leakage
- [ ] Event handlers - Check for &mut leakage
- [ ] State management - Check for Rc/RefCell leakage

#### Examples
- [ ] All `.wj` examples in `crates/windjammer-ui/examples/`
- [ ] Counter example
- [ ] Todo example
- [ ] Form validation example

### **Phase 3: windjammer-game-framework**

#### Public API
- [ ] `crates/windjammer-game-framework/src/lib.rs`
- [ ] `crates/windjammer-game-framework/src/ecs_windjammer.rs`
- [ ] `crates/windjammer-game-framework/src/game_app.rs`
- [ ] `crates/windjammer-game-framework/src/input.rs`
- [ ] `crates/windjammer-game-framework/src/rendering/mod.rs`

#### Check For
- [ ] No wgpu types in public API
- [ ] No winit types in public API
- [ ] No rapier types in public API
- [ ] No lifetimes in public API
- [ ] No trait bounds in public API

#### Examples
- [ ] All game examples should be `.wj` files
- [ ] No `.rs` game examples (those are internal tests)

---

## 🔧 **How to Fix Issues**

### **Pattern 1: Remove Borrowing Syntax**

**Before (WRONG):**
```windjammer
fn update(game: &mut PongGame, delta: f32) { }
```

**After (CORRECT):**
```windjammer
fn update(game: PongGame, delta: float) { }
// Compiler infers &mut automatically
```

### **Pattern 2: Hide Crate Implementations**

**Before (WRONG):**
```windjammer
use axum::Router
use axum::Json

@get("/users")
fn get_users() -> Json<Vec<User>> { }
```

**After (CORRECT):**
```windjammer
use std::http

@get("/users")
fn get_users() -> Vec<User> { }
// std::http hides axum internally
```

### **Pattern 3: Remove Trait Bounds**

**Before (WRONG):**
```windjammer
fn process<T: Clone + Debug>(item: T) { }
```

**After (CORRECT):**
```windjammer
fn process<T>(item: T) { }
// Compiler infers constraints from usage
```

### **Pattern 4: Use Decorators, Not Traits**

**Before (WRONG):**
```windjammer
impl GameLoop for PongGame {
    fn update(&mut self, delta: f32) { }
}
```

**After (CORRECT):**
```windjammer
@game
struct PongGame { }

@update
fn update(game: PongGame, delta: float) { }
```

---

## 📊 **Success Criteria**

### **A file/module passes audit if:**

1. ✅ No `&` or `&mut` in user-facing signatures
2. ✅ No external crate names in user code
3. ✅ No lifetime annotations
4. ✅ No trait bounds in signatures
5. ✅ No `unwrap()` / `expect()` in examples
6. ✅ Uses decorators instead of trait impls
7. ✅ Uses `std::*` instead of crate names
8. ✅ Compiler infers ownership automatically

### **The entire codebase passes if:**

1. ✅ All stdlib modules pass
2. ✅ All examples pass
3. ✅ All documentation passes
4. ✅ windjammer-ui public API passes
5. ✅ windjammer-game-framework public API passes

---

## 🎯 **Priority Order**

1. **CRITICAL:** Examples in README/GUIDE (users see these first)
2. **HIGH:** stdlib modules (core abstractions)
3. **HIGH:** Game framework public API (current focus)
4. **MEDIUM:** UI framework public API
5. **MEDIUM:** All other examples
6. **LOW:** Internal implementation (can use Rust)

---

## 📝 **Tracking**

### **Issues Found:**
- [ ] List each violation here as discovered
- [ ] Link to file and line number
- [ ] Describe the fix needed

### **Fixes Applied:**
- [ ] Track each fix
- [ ] Verify it follows philosophy
- [ ] Test that it still works

---

**This audit is CRITICAL to ensure Windjammer maintains its philosophy of simplicity.**

**Without this, we're just "Rust with different syntax" - which defeats the entire purpose.**

