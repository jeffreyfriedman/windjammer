# Fix Tuple Pattern Matching & AUTO-CLONE String Literals

## Summary

This PR fixes two critical codegen bugs discovered while dogfooding Windjammer to build `windjammer-ui` v0.2.0:

1. **Tuple pattern matching with string literals** - `.as_str()` was incorrectly added to tuple values
2. **AUTO-CLONE string literal optimization** - `.clone()` was unnecessarily added to `&str` variables

## Bug 1: Tuple Pattern Matching

### Problem
When matching on tuples containing string literals, the codegen incorrectly added `.as_str()` to the entire tuple value:

```rust
let colors = ("red", "green", "blue");
match colors.as_str() {  // ❌ Error: no method 'as_str' found for tuple
    ("red", g, b) => ...,
    _ => ...
}
```

### Root Cause
`pattern_has_string_literal()` returned `true` for tuple patterns containing string literals, which triggered `.as_str()` to be added to the match value regardless of whether it was a String or a Tuple.

### Fix
Added `is_tuple_match` check in `src/codegen/rust/generator.rs` (lines 2651-2673 and 3856-3878) to prevent `.as_str()` from being added when matching on tuples. Tuple patterns handle their own string matching internally.

### Impact
Enables full tuple destructuring with string literal patterns:
```windjammer
let colors = ("red", "green", "blue");
match colors {
    ("red", g, b) => format!("Red with {} and {}", g, b),
    (r, "green", b) => format!("{} with green and {}", r, b),
    _ => "Other".to_string()
}
```

## Bug 2: AUTO-CLONE String Literals

### Problem
Variables bound to string literals from match expressions were getting `.clone()` added, which is a no-op on `&str` and triggers clippy `noop_method_call` warnings:

```rust
let text_color = match variant {
    Default => "#2d3748",
    Primary => "white",
    _ => "..."
};
// Later: text_color.clone() <- no-op on &str, triggers warning
```

### Root Cause
The AUTO-CLONE analyzer didn't distinguish between owned types (which need `.clone()`) and `&str` references (which don't). It marked all multiply-used variables for cloning.

### Fix
Added `string_literal_vars` tracking to `AutoCloneAnalysis` in `src/auto_clone.rs`:
1. Detect variables bound to string literals (direct or from match/block expressions)
2. Skip `.clone()` for these variables in codegen (line 3089-3095 in `generator.rs`)
3. Maintains correct clone semantics for actual owned types

### Impact
- Eliminates `noop_method_call` clippy warnings in generated code
- Cleaner, more idiomatic Rust output
- No performance impact (`.clone()` on `&str` was already a no-op)
- Works with existing string interning optimization (Phase 11)

## Files Changed

### Core Changes
- `src/codegen/rust/generator.rs` - Tuple pattern matching fix and AUTO-CLONE skip logic
- `src/auto_clone.rs` - String literal variable tracking

### Testing
- All existing tests pass
- Verified with `windjammer-ui` v0.2.0 (58 components, 0 warnings)

## Dogfooding Impact

These bugs were discovered while building `windjammer-ui` v0.2.0 in pure Windjammer. This demonstrates the value of dogfooding: by using Windjammer to build real-world libraries, we identify and fix issues that would block production usage.

## Related PRs

- `windjammer-ui` PR: 10 new components built with these fixes
- Blocks: `windjammer-ui` v0.2.0 release

## Testing

```bash
# Build compiler
cargo build --release

# Test with windjammer-ui
cd ../windjammer-ui
cargo build  # Should succeed with 0 warnings
cargo clippy --all-targets --all-features -- -D warnings  # Should pass
```

## Checklist

- [x] All tests pass
- [x] Clippy checks pass
- [x] Code formatted with `rustfmt`
- [x] Pre-commit hooks pass
- [x] Tested with real-world code (`windjammer-ui`)
- [x] No breaking changes
- [x] Documentation updated (inline comments)

---

**Ready to merge!** This unblocks `windjammer-ui` v0.2.0 release.
