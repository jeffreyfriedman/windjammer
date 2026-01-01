# Arena Allocation Refactoring - Progress Report

**Status:** IN PROGRESS (Paused at 25% completion)  
**Started:** December 28, 2025  
**Estimated Completion:** 15-20 hours remaining

---

## Problem

Windows stack overflow due to recursive Drop of deeply nested AST structures.

Current workaround: 64MB stack (unsustainable long-term)  
Proper solution: Arena allocation (eliminates recursive Drop)

---

## Work Completed

### ✅ Phase 1: Add Lifetime Parameters to AST (Completed)

**Files Modified:**
- `Cargo.toml` - Added `typed-arena = "2.0"`  
- `src/parser/ast/core.rs` - Added `'ast` lifetime to all AST types:
  - `Expression<'ast>`
  - `Statement<'ast>`
  - `MatchArm<'ast>`
  - `Decorator<'ast>`
  - `FunctionDecl<'ast>`
  - `StructDecl<'ast>`, `StructField<'ast>`
  - `TraitDecl<'ast>`, `TraitMethod<'ast>`
  - `ImplBlock<'ast>`
  - `Item<'ast>`
  - `Program<'ast>`

**Changes:**
- All `Box<Expression>` → `&'ast Expression<'ast>`
- All `Box<Statement>` → `&'ast Statement<'ast>` (in Defer variant)
- All `Vec<Expression>` → `Vec<Expression<'ast>>`
- All `Vec<Statement>` → `Vec<Statement<'ast>>`

**Commit:** `1b8d8780` - "WIP: Add arena allocation - Part 1"

---

## Work Remaining

### ❌ Phase 2: Add Arena to Parser (IN PROGRESS)

**Current State:**
- Added `Arena<Expression>` and `Arena<Statement>` fields to Parser
- **Issue:** Lifetime management is complex
  - typed-arena requires lifetime parameters
  - Parser owns arenas but needs to return references with lifetime tied to Parser
  - This creates a self-referential lifetime issue

**Options:**
1. **Use unsafe to work around lifetimes** (pragmatic, works, but not ideal)
2. **Use Box<Arena>** and transmute lifetimes (hacky but effective)
3. **Redesign API** - Parser doesn't own arenas, caller creates them
4. **Use different arena** - bumpalo, id-arena, or typed-arena with different approach

### ❌ Phase 3: Update Allocation Sites (Not Started)

**Scope:** 900+ locations across 29 files

**Pattern:**
```rust
// Before
let expr = Box::new(Expression::Binary {
    left: Box::new(...),
    right: Box::new(...),
});

// After  
let expr = arena.alloc(Expression::Binary {
    left: arena.alloc(...),
    right: arena.alloc(...),
});
```

**Files Affected:** (from cargo check)
- `src/parser/expression_parser.rs` - 90 Expression:: references
- `src/codegen/rust/generator.rs` - 130 Expression:: references
- `src/analyzer.rs` - 152 Expression:: references
- `src/parser/statement_parser.rs` - Multiple Statement references
- `src/parser/item_parser.rs` - Multiple Item references
- ... 24 more files

### ❌ Phase 4: Fix Compilation Errors (Not Started)

**Current:** 396 compilation errors

**Categories:**
1. **Missing lifetime specifiers** (most common)
   - Functions that return AST types need lifetimes
   - Structs that contain AST types need lifetimes
   - Example: `AnalyzedFunction`, `Analyzer`, etc.

2. **Lifetime conflicts**
   - Functions borrowing AST with wrong lifetime
   - Structs storing AST with wrong lifetime

3. **Method signature changes**
   - All parser methods need to allocate from arena
   - All analyzer/codegen methods need lifetime parameters

### ❌ Phase 5: Update Tests (Not Started)

**Scope:** All 28 test files

**Pattern:**
```rust
// Before
let expr = Box::new(Expression::Literal { ... });

// After
let arena = Arena::new();
let expr = arena.alloc(Expression::Literal { ... });
```

### ❌ Phase 6: Verify & Reduce Stack Size (Not Started)

**Goal:** Reduce from 64MB → 8MB

**Steps:**
1. All tests passing
2. Update `.cargo/config.toml`:
   ```toml
   RUST_MIN_STACK = "8388608"  # 8MB instead of 64MB
   ```
3. Verify CI passes on all platforms
4. Remove 64MB workaround

---

## Estimated Time Remaining

- Phase 2 (Arena to Parser): 2-3 hours
- Phase 3 (Update allocations): 6-8 hours  
- Phase 4 (Fix errors): 4-6 hours
- Phase 5 (Update tests): 2-3 hours
- Phase 6 (Verify): 1 hour

**Total:** 15-20 hours

---

## Alternative Approach (Faster)

If arena allocation proves too complex, an alternative is **Custom Iterative Drop**:

```rust
impl Drop for Expression {
    fn drop(&mut self) {
        // Use iterative algorithm with Vec<Expression> stack
        // instead of recursive drop
        let mut stack = vec![self];
        while let Some(expr) = stack.pop() {
            // push children onto stack
            // manually drop non-recursive fields
        }
    }
}
```

**Pros:**
- Much simpler (2-3 hours)
- Solves stack overflow immediately  
- No API changes

**Cons:**
- Doesn't improve performance (cache locality, allocation speed)
- Still uses Box (not as efficient as arena)

---

## Decision Point

**Question for maintainer:** Given the massive scope (15-20 hours remaining), should we:

A. **Continue with arena allocation** (proper solution, better performance, but large effort)
B. **Implement custom iterative Drop first** (quick fix, then arena later)
C. **Keep 64MB stack temporarily** (works, revisit after MVP)

---

## Current Branch

`feature/fix-constructor-ownership`

**Last commit:** `1b8d8780` - WIP state, does not compile (396 errors expected)

**To continue:**
```bash
git checkout feature/fix-constructor-ownership
cargo check 2>&1 | less  # See all errors
```

---

## References

- `docs/TODO_ARENA_ALLOCATION.md` - Original plan
- `docs/CI_FIXES_2025_12_27.md` - Stack overflow history
- rustc arena: https://github.com/rust-lang/rust/tree/master/compiler/rustc_arena
- typed-arena: https://docs.rs/typed-arena

---

**Note:** This is a massive architectural change. It's the *right* solution but requires significant effort. Consider the trade-offs carefully.

