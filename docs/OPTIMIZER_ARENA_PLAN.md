# Optimizer Arena Allocation Implementation Plan

**Status:** IN PROGRESS  
**Total Errors:** 161  
**Approach:** Systematic file-by-file refactoring

---

## Strategy

Following the parser arena allocation pattern established in Sessions 4-6:

1. **Optimizer struct owns arenas** (like Parser)
2. **Free lifetime `'ast`** decoupled from `&self` (like Parser)
3. **Alloc methods** for creating AST nodes
4. **Phase functions** accept `&Program<'ast>`, return `Program<'ast>` or `&'ast T<'ast>`

---

## Files to Fix

### 1. ✅ `mod.rs` (PARTIAL)
- [x] Add arena fields to `Optimizer` struct
- [x] Add `alloc_expr`, `alloc_stmt`, `alloc_pattern` methods  
- [x] Update `optimize()` signature to `<'ast>`
- [ ] Update calls to phase functions

### 2. `phase11_string_interning.rs`
- [x] Update `optimize_string_interning()` signature
- [ ] Update all helper functions
- [ ] Update string interning logic

### 3. `phase12_dead_code_elimination.rs`
- [ ] Update `eliminate_dead_code()` signature
- [ ] Update all helper functions (`eliminate_dead_code_in_*`)
- [ ] Update DCE logic

### 4. `phase13_loop_optimization.rs`
- [ ] Update `optimize_loops()` signature
- [ ] Update loop analysis functions
- [ ] Update loop transformation logic

### 5. `phase14_escape_analysis.rs`
- [ ] Update `optimize_escape_analysis()` signature  
- [ ] Update escape analysis functions

### 6. `phase15_simd_vectorization.rs`
- [ ] Update `optimize_simd_vectorization()` signature
- [ ] Update vectorization logic

---

## Pattern Examples

### Before:
```rust
fn process_expression(expr: &Expression) -> Expression {
    match expr {
        Expression::Literal { value, .. } => Expression::Literal { value: value.clone(), .. },
        _ => expr.clone(),
    }
}
```

### After:
```rust
fn process_expression<'ast>(expr: &'ast Expression<'ast>, optimizer: &Optimizer) -> &'ast Expression<'ast> {
    match expr {
        Expression::Literal { value, .. } => {
            optimizer.alloc_expr(Expression::Literal { value: value.clone(), .. })
        },
        _ => expr,
    }
}
```

---

## Progress Tracking

- **Errors Fixed:** 1 / 161
- **Files Complete:** 0 / 5
- **Current File:** phase11_string_interning.rs

---

## Next Steps

1. Finish phase11_string_interning.rs
2. Move to phase12_dead_code_elimination.rs
3. Continue through phase13, phase14, phase15
4. Test compilation after each file
5. Commit progress regularly

**No shortcuts. Full proper implementation.**



