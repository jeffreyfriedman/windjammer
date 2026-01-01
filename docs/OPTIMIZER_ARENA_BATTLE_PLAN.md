# Optimizer Arena Allocation - Battle Plan

## Status: In Progress

**Date**: 2025-12-29  
**Errors Remaining**: 473 (down from 478)

## Error Breakdown by Phase

| Phase | File | Errors | Status |
|-------|------|--------|--------|
| 11 | string_interning.rs | 16 | Signatures done, bodies need work |
| 12 | dead_code_elimination.rs | 62 | Partially done (statements ✓, expressions 50%) |
| 13 | loop_optimization.rs | **301** | Signatures done, bodies untouched |
| 14 | escape_analysis.rs | 52 | Signatures done, bodies untouched |
| 15 | simd_vectorization.rs | 42 | Signatures done, bodies untouched |

**Total**: 473 errors

## What's Been Done ✅

### Phase 12 (dead_code_elimination.rs)
- ✅ All function signatures updated with `'ast` lifetimes
- ✅ `eliminate_dead_code` entry point updated
- ✅ `eliminate_dead_code_in_statements` updated to accept `&[&Statement]`
- ✅ `eliminate_dead_code_in_statement` - ALL statement allocations fixed
- ✅ `eliminate_dead_code_in_expression` - Partially done (Call, MethodCall)
- ❌ Still need: ~60 more Expression constructions in `eliminate_dead_code_in_expression`

### All Phases
- ✅ All entry point signatures updated (`optimize_xxx(program, optimizer)`)
- ✅ All helper function signatures updated with lifetimes
- ✅ All call sites in `optimizer/mod.rs` updated

## What Needs to Be Done ❌

### The Pattern

Every function body that constructs an AST node needs:

**Statements:**
```rust
// OLD:
Statement::Expression { expr, location }

// NEW:
optimizer.alloc_stmt(Statement::Expression { expr, location })
```

**Expressions:**
```rust
// OLD:
Expression::Call { function: Box::new(...), ... }

// NEW:
optimizer.alloc_expr(Expression::Call { function: ..., ... })
```

**Key Change**: `Box::new(expr)` → just `expr` (functions now return `&'ast Expression<'ast>`)

### Systematic Approach

For each phase file:

1. Fix `Expression` constructions in main optimization functions
2. Fix `Statement` constructions in main optimization functions
3. Fix helper functions that build AST nodes
4. Remove `Box::new` wrapping (no longer needed with arena allocation)

### Estimated Work

- phase11: ~16 Expression/Statement constructions to fix
- phase12: ~60 remaining Expression constructions
- phase13: **~300 constructions** (MASSIVE - this is the bulk of the work)
- phase14: ~52 constructions
- phase15: ~42 constructions

**Total**: ~470 AST node constructions to wrap with `optimizer.alloc_xxx(...)`

## Strategy for phase13 (the 300-error monster)

phase13_loop_optimization.rs is HUGE and has the most errors. Here's the plan:

1. **Identify all functions that return owned AST nodes**:
   - `optimize_loops_in_item` → returns `Item`
   - `optimize_loops_in_statements` → returns `Vec<Statement>`
   - `optimize_loops_in_statement` → returns `Statement`
   - `optimize_loops_in_expression` → returns `Expression`
   - `replace_variable_in_statement` → returns `Statement`
   - `replace_variable_in_expression` → returns `Expression`
   - `try_unroll_loop` → likely returns AST nodes
   - `try_strength_reduction` → likely returns AST nodes

2. **Fix them in order from deepest (expressions) to highest (items)**:
   - Start with `optimize_loops_in_expression` (bottom of call tree)
   - Then `replace_variable_in_expression`
   - Then `optimize_loops_in_statement`
   - Then `replace_variable_in_statement`
   - Then `optimize_loops_in_statements`
   - Finally `optimize_loops_in_item`

3. **For each function**:
   - Wrap every `Expression::...` with `optimizer.alloc_expr(...)`
   - Wrap every `Statement::...` with `optimizer.alloc_stmt(...)`
   - Change `Box::new(expr)` to just `expr` (already a reference)

## Commit Strategy

Commit after each file is complete:
- ✅ phase12 statement fixes
- 🔄 phase12 expression fixes (in progress)
- ⏳ phase11 complete
- ⏳ phase13 complete (will take multiple commits)
- ⏳ phase14 complete
- ⏳ phase15 complete
- 🎯 FINAL: All phases complete, 0 errors!

## Time Estimate

At current pace (~5-10 errors fixed per focused session):
- phase11: 16 errors → 2-3 focused edits
- phase12: 62 errors → 6-10 focused edits
- phase13: 301 errors → **30-60 focused edits** (this is the grind)
- phase14: 52 errors → 5-10 focused edits
- phase15: 42 errors → 4-8 focused edits

**Total**: 47-91 focused search_replace operations remaining

## Token Budget

Current remaining: ~918K tokens  
Estimated per fix: ~2K tokens (including search, read, replace)  
Can handle: ~450 fixes  
Need: ~470 fixes

**Conclusion**: Doable but tight. Need to be efficient and systematic.

## Next Immediate Actions

1. Finish phase12 `eliminate_dead_code_in_expression` (~30 more Expression wrappings)
2. Commit phase12 complete
3. Tackle phase11 (smallest remaining, 16 errors)
4. Commit phase11 complete
5. Break for status check and token assessment
6. If tokens allow, start the phase13 grind

## Success Criteria

- ✅ Lib compiles (`cargo build --lib` = 0 errors)
- ✅ Tests compile (`cargo test --no-run` = 0 errors)
- ✅ Clippy happy (`cargo clippy --all-targets` = 0 warnings)
- ✅ All tests pass (`cargo test`)
- ✅ Coverage runs (`cargo tarpaulin`)

