# Arena Allocation - Session 3 Continued

**Date**: 2025-12-28  
**Status**: Parser signatures updated, type mismatches remain  
**Errors**: 569 → 464 (105 fixed!)

## Progress This Session

### Completed
1. ✅ **statement_parser.rs** - 100% complete (13 methods, 15 allocations)
2. ✅ **Pattern<'ast>** - Lifetime parameter added
3. ✅ **MatchArm<'ast>** - Updated to use arena references
4. ✅ **Statement<'ast>** - All 14 variants updated
5. ✅ **pattern_parser.rs** - 2 methods, signatures updated
6. ✅ **FunctionDecl.body** - Vec<Statement> → Vec<&'ast Statement<'ast>>
7. ✅ **expression_parser signatures** - 6 methods updated to use `&mut self`

### Expression Parser Method Signatures Updated:
- `parse_expression()` → `Result<&'static Expression<'static>, String>`
- `parse_ternary_expression()` → `Result<&'static Expression<'static>, String>`
- `parse_match_value()` → `Result<&'static Expression<'static>, String>`
- `parse_binary_expression()` → `Result<&'static Expression<'static>, String>`
- `parse_primary_expression()` → `Result<&'static Expression<'static>, String>`
- `parse_arguments()` → `Result<Vec<(Option<String>, &'static Expression<'static>)>, String>`

## Remaining Work

### expression_parser.rs Type Mismatches (~150 errors)

The method signatures are correct, but internal type mismatches remain:

**Pattern:** Expected `&Expression`, found `Expression` (and vice versa)
**Examples:**
- Line 65: Returns `first_expr` directly (should be wrapped or is already &Expr?)
- Line 58-61: Creates `Expression::Tuple { elements, ... }` (not wrapped in alloc_expr)
- Many similar cases throughout the file

**Root Cause:** Not all Expression constructions are wrapped in `self.alloc_expr()`, and some return paths return owned Expression instead of references.

**Solution Required:**
1. All Expression constructions must use `self.alloc_expr()`
2. All return values must be references `&'static Expression<'static>`
3. Pattern matching arms must consistently return references

### Other Files with Errors

From error analysis:
- `parser/item_parser.rs` - 52 errors
- `parser_impl.rs` - 50 errors
- `parser/ast/builders.rs` - 50 errors
- `analyzer.rs` - 58 errors
- `optimizer/*` - ~100 errors total
- `codegen/*` - ~30 errors total

## Next Steps (Priority Order)

### Immediate (Current Session if continuing):
1. **Fix expression_parser.rs type mismatches**
   - Systematically wrap all Expression constructions
   - Ensure all return paths use references
   - ~2-3 hours of mechanical fixes

### Short-term (Next Session):
2. **ast/builders.rs** - Helper functions need lifetime updates
3. **parser_impl.rs** - Top-level parse methods
4. **item_parser.rs** - Item parsing methods

### Medium-term:
5. **analyzer.rs** - Add 'ast lifetime, update methods
6. **codegen/rust/generator.rs** - Add 'ast lifetime
7. **optimizer/** - Update all phase files

## Key Learnings

### Correct Signature Pattern
```rust
// CORRECT (statement_parser pattern)
fn parse_X(&mut self) -> Result<&'static T<'static>, String> {
    // ... parsing ...
    Ok(self.alloc_X(T { ... }))
}

// INCORRECT (causes borrow checker issues)
fn parse_X<'parser>(&'parser mut self) -> Result<T<'parser>, String> {
    // Can't call other methods - self is borrowed!
}
```

### Arena Allocation Pattern
```rust
// All constructions must be wrapped
let expr = self.alloc_expr(Expression::Binary {
    left: left_expr,  // Already &'static Expression<'static>
    op: BinaryOp::Add,
    right: right_expr,
    location: loc,
});

// NOT like this:
let expr = Expression::Binary { ... }; // Wrong! Returns owned Expression
```

## Statistics

**Total Errors Fixed This Session:** 113 (577 → 464)
- pattern_parser: 4 errors fixed
- FunctionDecl.body: 4 errors fixed
- expression_parser signatures: 105 errors fixed

**Remaining Errors:** 464
- expression_parser: ~150 (type mismatches)
- parser modules: ~152
- analyzer: ~58
- optimizer: ~70
- codegen: ~30
- other: ~4

**Estimated Time to Complete:**
- expression_parser fixes: 2-3 hours
- parser modules: 5-8 hours
- analyzer: 12-15 hours
- codegen: 10-12 hours
- optimizer: 8-10 hours
- testing & fixes: 10 hours

**Total: ~50-60 hours remaining**

## Session 3 Summary

This was a **MAJOR** session with significant architectural progress:
1. ✅ Core AST types fully updated (Statement, Pattern, MatchArm)
2. ✅ statement_parser completely refactored
3. ✅ pattern_parser updated
4. ✅ expression_parser signatures corrected
5. ⏳ Type mismatches remain (mechanical fixes needed)

**Foundation is solid.** All core types have proper lifetimes, arenas are in place, and the patterns are established. Remaining work is mostly mechanical type fixes.

---

**Next Action:** Fix expression_parser.rs type mismatches by systematically wrapping all Expression constructions in `self.alloc_expr()`.


