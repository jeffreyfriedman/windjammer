# Arena Allocation - Next Steps for Continuation

## Current Progress (~35% Complete)

**Branch:** `feature/fix-constructor-ownership`  
**Last Commit:** `b22949d2`  
**Status:** WIP, does not compile (~380 errors remaining)

### ✅ Completed So Far

1. **AST Types** - All have `'ast` lifetime parameter
2. **Parser** - Has arenas, alloc_expr/alloc_stmt methods
3. **Analyzer** - Struct has `'ast` lifetime
4. **CodeGenerator** - Struct has `'ast` lifetime

### ❌ Remaining Work (~65%)

**Priority 1: Fix Core Method Signatures**

These need lifetime parameters added to ALL methods:

1. `src/analyzer.rs` (200+ methods)
   - Every method that touches AST needs `'ast`
   - Start with `analyze_program`, `analyze_function`, etc.

2. `src/codegen/rust/generator.rs` (150+ methods)
   - Every `generate_*` method needs `'ast`
   - Start with `generate_program`, `generate_item`, etc.

3. `src/parser_impl.rs` (50+ methods)
   - Every `parse_*` method needs `<'parser>`
   - Already started: `parse()`, `parse_item()`
   - TODO: All expression/statement/type parsing

**Priority 2: Update Parser Modules**

These need to use `alloc_expr/alloc_stmt` instead of `Box::new`:

1. `src/parser/expression_parser.rs`
   - Replace all `Box::new(Expression::...)` with `parser.alloc_expr(Expression::...)`
   - Every nested expression needs arena allocation

2. `src/parser/statement_parser.rs`
   - Replace `Box::new(Statement::...)` with `parser.alloc_stmt(Statement::...)`

3. `src/parser/item_parser.rs`
   - Update Item construction with lifetimes

4. `src/parser/type_parser.rs`, `src/parser/pattern_parser.rs`
   - Add lifetime parameters

**Priority 3: Update Helper Modules**

All these need `'ast` lifetimes added:

- `src/codegen/rust/constant_folding.rs`
- `src/codegen/rust/string_analysis.rs`
- `src/codegen/rust/self_analysis.rs`
- `src/codegen/rust/ast_utilities.rs`
- `src/codegen/rust/expression_helpers.rs`
- ... and 10 more codegen helper files

**Priority 4: Update Optimizer**

All 15 optimizer phase files:
- `src/optimizer/phase*.rs`
- `src/auto_clone.rs`
- `src/inference.rs`

**Priority 5: Update Tests**

All 28 test files need:
- Create arenas
- Use arena allocation
- Update assertions

**Priority 6: Update Database**

`src/compiler_database.rs` (Salsa integration)
- This is tricky - may need to clone AST into database
- Salsa has its own lifetime management

## How to Continue

### Step 1: Check Current Errors

```bash
cd /Users/jeffreyfriedman/src/wj/windjammer
cargo check 2>&1 | less
```

Look for patterns in errors to identify which files to fix first.

### Step 2: Fix Analyzer Methods

Start with the main analysis method:

```rust
// Before
pub fn analyze_program(&mut self, program: &Program) -> Result<...>

// After  
pub fn analyze_program(&mut self, program: &Program<'ast>) -> Result<...>
```

Then systematically add `'ast` to every method that touches AST.

### Step 3: Fix CodeGenerator Methods

Similarly for generator:

```rust
// Before
pub fn generate_program(&mut self, program: &Program) -> String

// After
pub fn generate_program(&mut self, program: &Program<'ast>) -> String
```

### Step 4: Fix Parser Methods

Update all parse_* methods to use arenas:

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

### Step 5: Run Tests After Each Module

After fixing a major module, run tests to verify:

```bash
cargo test --test specific_test -- --nocapture
```

### Step 6: Final Verification

Once all 380 errors are fixed:

1. Run full test suite: `cargo test`
2. Reduce stack size in `.cargo/config.toml`:
   ```toml
   RUST_MIN_STACK = "8388608"  # 8MB instead of 64MB
   ```
3. Run tests again
4. Verify CI passes on all platforms

## Key Technical Points

### Lifetime Transmute Safety

The Parser uses this pattern:

```rust
pub fn alloc_expr<'parser>(&'parser self, expr: Expression<'static>) 
    -> &'parser Expression<'parser> 
{
    unsafe { std::mem::transmute(self.expr_arena.alloc(expr)) }
}
```

**Why this is safe:**
- Parser owns the arena
- Arena contains `Expression<'static>` (arena's storage lifetime)
- We transmute to `Expression<'parser>` (Parser's borrow lifetime)
- Safe because references can't outlive Parser
- Arena is dropped when Parser is dropped

### Common Patterns

**Pattern 1: Struct with AST field**
```rust
// Before
struct Foo {
    expr: Expression,
}

// After
struct Foo<'ast> {
    expr: Expression<'ast>,
}
```

**Pattern 2: Function returning AST**
```rust
// Before
fn foo() -> Expression { ... }

// After
fn foo<'ast>() -> Expression<'ast> { ... }
```

**Pattern 3: Function with multiple lifetimes**
```rust
// Before
fn analyze(program: &Program, ctx: &Context) -> Result<...>

// After
fn analyze<'ast>(program: &Program<'ast>, ctx: &Context) -> Result<...>
```

Note: Only AST types need `'ast`, not Context or other types.

## Estimated Time Remaining

- Analyzer methods: 3-4 hours
- CodeGenerator methods: 3-4 hours
- Parser modules: 4-6 hours
- Helper modules: 2-3 hours
- Optimizer passes: 2-3 hours
- Tests: 2-3 hours
- Database/Salsa: 1-2 hours
- Verification: 1 hour

**Total:** 18-26 hours

## Success Criteria

1. `cargo check` passes (0 errors)
2. `cargo test` passes (all 28 test files)
3. Stack reduced from 64MB → 8MB
4. CI passes on Ubuntu, Windows, macOS
5. No performance regression

## Questions/Issues

1. **Salsa Integration:** May need special handling for database
2. **Test Fixtures:** Some tests construct AST directly
3. **Macro-generated Code:** May have lifetime issues

## Resources

- `docs/TODO_ARENA_ALLOCATION.md` - Original plan
- `docs/ARENA_ALLOCATION_PROGRESS.md` - Initial analysis
- `docs/ARENA_STATUS_DEC28.md` - Detailed current status
- rustc arena: https://github.com/rust-lang/rust/tree/master/compiler/rustc_arena

---

**Good luck! You're 35% done. The rest is mechanical but tedious.**

