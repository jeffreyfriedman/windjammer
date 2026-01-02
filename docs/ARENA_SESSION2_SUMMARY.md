# Arena Allocation - Session 2 Summary (Dec 28, 2025)

## Progress: ~40% Complete (was 35%, now 40%)

**Branch:** `feature/fix-constructor-ownership`  
**Last Commit:** `a21d407d`  
**Error Count:** 427 (was 396, increased due to cascading)

---

## Completed This Session

### ✅ Helper Functions (Part 5)
- `codegen/rust/constant_folding.rs`: `try_fold_constant<'ast>()`
- `codegen/rust/string_analysis.rs`: `collect_concat_parts<'ast>()`
- `codegen/javascript/tree_shaker.rs`: `shake_tree<'ast>()`

### ✅ Parser Methods (Parts 6-7)
- `parser/item_parser.rs`:
  - `parse_decorator<'parser>()` → `Result<Decorator<'parser>>`
  - `parse_decorator_arguments<'parser>()` → `Vec<(String, Expression<'parser>)>`
  - `parse_function<'parser>()` → `Result<FunctionDecl<'parser>>`
  - `parse_impl<'parser>()` → `Result<ImplBlock<'parser>>`
  - `parse_trait<'parser>()` → `Result<TraitDecl<'parser>>`
  - `parse_struct<'parser>()` → `Result<StructDecl<'parser>>`

---

## Key Issue Discovered: AST Builders

**Problem:** `src/parser/ast/builders.rs` creates test AST nodes using `Box::new()`.

```rust
// Current (broken):
pub fn expr_binary(op: BinaryOp, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        left: Box::new(left),   // ← Expression now uses &'ast, not Box
        op,
        right: Box::new(right),
        location: None,
    }
}
```

**Impact:** ~100 builder functions broken, used extensively in tests.

**Options:**
1. **Skip builders, fix later** (recommended) - focus on main parser first
2. **Add arena parameter to builders** - `pub fn expr_binary<'ast>(arena: &'ast Arena, ...)` 
3. **Use Box for builders only** - special builder-only Expression type

**Recommendation:** Skip builders for now, come back after main parser works.

---

## Next Priority Tasks (In Order)

### 1. Expression Parser (~8-10 hours)
**File:** `src/parser/expression_parser.rs`

**What needs doing:**
- Add `<'parser>` lifetime to ALL parse_* methods
- Replace ALL `Box::new(Expression::...)` with `self.alloc_expr(Expression::...)`
- Handle nested allocations (e.g., Binary with Binary children)

**Example transformation:**
```rust
// Before
fn parse_binary(&mut self) -> Result<Expression, String> {
    let left = Box::new(self.parse_primary()?);
    let right = Box::new(self.parse_primary()?);
    Ok(Expression::Binary { left, op, right, location })
}

// After
fn parse_binary<'parser>(&'parser mut self) -> Result<Expression<'parser>, String> {
    let left = self.alloc_expr(self.parse_primary()?);
    let right = self.alloc_expr(self.parse_primary()?);
    Ok(Expression::Binary { left, op, right, location })
}
```

**Methods to update (partial list):**
- `parse_expression<'parser>()`
- `parse_binary_expression<'parser>()`
- `parse_primary_expression<'parser>()`
- `parse_postfix_expression<'parser>()`
- `parse_call<'parser>()`
- `parse_method_call<'parser>()`
- `parse_field_access<'parser>()`
- `parse_index<'parser>()`
- `parse_cast<'parser>()`
- `parse_range<'parser>()`
- `parse_closure<'parser>()`
- ... and ~20 more

### 2. Statement Parser (~4-6 hours)
**File:** `src/parser/statement_parser.rs`

**What needs doing:**
- Add `<'parser>` lifetime to ALL parse_* methods
- Replace `Box::new(Statement::...)` with `self.alloc_stmt(Statement::...)`
- Update Statement variants containing Expressions

**Methods to update (partial list):**
- `parse_statement<'parser>()`
- `parse_let<'parser>()`
- `parse_if<'parser>()`
- `parse_match<'parser>()`
- `parse_for<'parser>()`
- `parse_while<'parser>()`
- `parse_return<'parser>()`
- ... and ~15 more

### 3. Analyzer Methods (~3-4 hours)
**File:** `src/analyzer.rs`

**Status:** Struct has `<'ast>` but methods don't.

**What needs doing:**
- Add `<'ast>` to ~200 methods that touch AST
- Start with `analyze_program`, `analyze_function`, etc.
- Update all pattern matching on AST types

### 4. CodeGenerator Methods (~3-4 hours)
**File:** `src/codegen/rust/generator.rs`

**Status:** Struct has `<'ast>` but methods don't.

**What needs doing:**
- Add `<'ast>` to ~150 methods that touch AST
- Start with `generate_program`, `generate_item`, etc.
- Update all pattern matching on AST types

### 5. Optimizer Passes (~2-3 hours)
**Files:** `src/optimizer/phase*.rs` (15 files)

**What needs doing:**
- Add `<'ast>` to all optimization functions
- Update all Expression/Statement handling

### 6. Fix Builders (~2-3 hours)
**File:** `src/parser/ast/builders.rs`

**Options:**
- Add arena parameter to all builders
- Or use `Box::leak()` to create `'static` references for tests

### 7. Update Tests (~2-3 hours)
**Files:** All test files (28 files)

**What needs doing:**
- Create arenas in test setup
- Use arena allocation or updated builders
- Fix all assertions

### 8. Compiler Database (~1-2 hours)
**File:** `src/compiler_database.rs` (Salsa integration)

**Challenge:** Salsa has its own lifetime management.
**Likely solution:** Clone AST into database.

---

## Error Count Breakdown

**Current:** 427 errors (increased from 396)

**Why it increased:**
- Adding lifetimes to helper functions exposed downstream issues
- This is expected and normal
- Error count will fluctuate until all pieces connect

**Error distribution:**
- ~200: Missing lifetime specifiers
- ~100: Type mismatch (Box vs &'ast)
- ~80: Builder-related  
- ~47: Method signature conflicts

---

## Technical Notes

### Arena Allocation Pattern

**Parser helper methods:**
```rust
pub fn alloc_expr<'parser>(&'parser self, expr: Expression<'static>) 
    -> &'parser Expression<'parser> 
{
    unsafe { std::mem::transmute(self.expr_arena.alloc(expr)) }
}
```

**Why safe:**
- Parser owns arena
- Arena lives as long as Parser
- References can't outlive Parser
- Transmute just changes type-level lifetime

### Common Lifetime Patterns

**Pattern 1:** Function returning AST
```rust
fn foo<'parser>(&'parser mut self) -> Result<Expression<'parser>, String>
```

**Pattern 2:** Function taking AST reference
```rust
fn analyze<'ast>(expr: &Expression<'ast>) -> Result<...>
```

**Pattern 3:** Struct storing AST
```rust
struct Analyzer<'ast> {
    exprs: Vec<Expression<'ast>>,
}
```

---

## Estimated Time Remaining

- Expression parser: 8-10 hours
- Statement parser: 4-6 hours  
- Analyzer methods: 3-4 hours
- CodeGenerator methods: 3-4 hours
- Optimizer passes: 2-3 hours
- Builders: 2-3 hours
- Tests: 2-3 hours
- Database: 1-2 hours

**Total:** 25-35 hours remaining

---

## To Continue (Next Session)

```bash
cd /Users/jeffreyfriedman/src/wj/windjammer
git checkout feature/fix-constructor-ownership
cargo check 2>&1 | less  # Review errors

# Start with expression_parser.rs
# 1. Add <'parser> to all methods
# 2. Replace Box::new with self.alloc_expr
# 3. Test compile frequently
```

**Focus on getting parser working first, then analyzer, then codegen.**

**Skip builders and tests until parser/analyzer/codegen are done.**

---

## Success Criteria

1. `cargo check` passes (0 errors)
2. `cargo test` passes
3. Stack reduced from 64MB → 8MB in `.cargo/config.toml`
4. CI passes on all platforms
5. No performance regression

---

**Status:** Active WIP, ~40% complete, solid progress this session.  
**Recommendation:** Continue systematically, expression_parser.rs next.



