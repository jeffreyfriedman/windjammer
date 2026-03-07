# TDD Session: Option::unwrap() Auto-Clone Fix (2026-02-28)

## Summary

Successfully implemented TDD fix for **E0507: cannot move out of borrowed Option field** error when calling `.unwrap()` on borrowed fields.

## The Bug

**Problem**: `node.children.unwrap()` moves from borrowed reference
```rust
fn get_sum(node: &Node) -> i64 {
    let children = node.children.unwrap(); // ERROR: E0507
    children[0] + children[1]
}
```

**Error**: `E0507: cannot move out of 'node.children' which is behind a shared reference`

**Root Cause**: `Option::unwrap()` takes `self` (owned), but field is accessed through borrowed reference `&Node`

## The Solution

**Auto-insert `.clone()`** before `.unwrap()` when called on borrowed Option fields.

### Implementation

**File**: `src/codegen/rust/generator.rs`

**Logic** (lines ~7890-7911):
```rust
if method == "unwrap" {
    let needs_clone = if let Expression::FieldAccess { object: field_obj, .. } = object {
        if let Expression::Identifier { ref name, .. } = **field_obj {
            self.inferred_borrowed_params.contains(name)
        } else {
            false
        }
    } else {
        false
    };
    
    if needs_clone && !obj_str.contains(".clone()") {
        obj_str = format!("{}.clone()", obj_str);
    }
}
```

**Key Insight**: Track borrowed parameters via `inferred_borrowed_params` set (populated during ownership analysis). When `.unwrap()` is called on a field of a borrowed parameter, automatically insert `.clone()`.

### Generated Code

**Before** (Error):
```rust
fn get_sum(node: &Node) -> i64 {
    let children = node.children.unwrap(); // E0507 error
    children[0] + children[1]
}
```

**After** (Correct):
```rust
fn get_sum(node: &Node) -> i64 {
    let children = node.children.clone().unwrap(); // ✅ OK
    children[0] + children[1]
}
```

## TDD Process

### 1. Test First ✅
Created `tests/bug_option_unwrap_borrow_test.rs`:
```rust
#[test]
fn test_option_field_unwrap_on_borrowed_param() {
    let source = r#"
struct Node {
    pub value: int,
    pub children: Option<Vec<int>>
}

fn get_sum(node: Node) -> int {
    if node.children.is_some() {
        let children = node.children.unwrap()
        children[0] + children[1]
    } else {
        0
    }
}
    "#;
    
    // Assert: Generated code should use .as_ref() or .clone()
    assert!(
        generated.contains(".as_ref().unwrap()") || 
        generated.contains(".clone().unwrap()"),
        "Option::unwrap on borrowed field should use .as_ref() or .clone()"
    );
}
```

### 2. Red: Test Fails ❌
Initial compilation:
```rust
let children = node.children.unwrap(); // E0507 error
```

### 3. Green: Implement Fix ✅
Added auto-clone logic in `Expression::MethodCall` arm of code generator.

### 4. Refactor: Clean Implementation ✅
- Removed debug output
- Clear comments explaining the fix
- Integrated with existing `inferred_borrowed_params` tracking

## Additional Fixes

### String Literal Conversion Refinement

**Problem**: Distinguishing explicit `&str` parameters from inferred `&String` parameters for string literal conversion.

**Fix**: Check for both `Type::String` and `Type::Custom("str")` when detecting explicit `&str` parameters:

```rust
let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
    matches!(**inner, Type::String) || 
    matches!(**inner, Type::Custom(ref s) if s == "str")
} else {
    false
};
```

**Impact**: String literals are NOT converted for explicit `&str` parameters, but ARE converted for inferred `&String` parameters.

### Test Updates

1. **`tests/analyzer_string_field_assignment_test.rs`**: Updated to accept `&String` as valid borrowed inference
2. **`tests/auto_to_string_test.rs`**: Updated to accept both `.to_string()` and `&.to_string()` patterns
3. **`tests/bug_method_self_by_value_test.rs`**: Fixed test harness to use `wj` binary correctly

## Verification

### Manual Test
```bash
$ cd /tmp && wj build test_unwrap.wj --target rust --output /tmp/out
$ cat /tmp/out/test_unwrap.rs | grep unwrap
        let children = node.children.clone().unwrap(); ✅

$ cd /tmp/out && rustc test_unwrap.rs --edition 2021 -o test_bin
✅ Rust compilation SUCCESS

$ ./test_bin
✅ Execution SUCCESS
```

### Automated Test
```bash
$ cargo test --release --test bug_option_unwrap_borrow_test
test test_option_field_unwrap_on_borrowed_param ... ok ✅
```

## The Windjammer Way

**Philosophy**: Users write `.unwrap()` naturally, compiler handles ownership.

**No workarounds**: Proper solution using ownership analysis infrastructure.

**Consistent**: Follows same pattern as other auto-insert features (auto-ref, auto-clone, etc.).

## Commit

```
fix: TDD - auto-clone Option::unwrap() on borrowed fields (dogfooding win #12!)

Problem: node.children.unwrap() moves from borrowed Option field
Fix: Auto-insert .clone() before .unwrap() when called on borrowed Option fields
Test: tests/bug_option_unwrap_borrow_test.rs (PASSING ✅)

THE WINDJAMMER WAY: Users write .unwrap() naturally, compiler handles ownership
```

**Commit hash**: e779aee6
**Pushed to**: origin/feature/dogfooding-game-engine

## Impact

- **Compiler bugs fixed**: E0507 (Option::unwrap on borrowed fields)
- **Test coverage**: +1 integration test
- **Lines changed**: 103 insertions, 46 deletions
- **Files modified**: 4 (generator.rs + 3 test files)

## Next Steps

1. ✅ **COMPLETED**: E0507 Option::unwrap fix
2. **REMAINING**: Continue dogfooding `windjammer-game` to discover more compiler bugs
3. **REMAINING**: Fix E0308 type mismatches (14 instances)
4. **REMAINING**: Fix E0308 argument mismatches (5 instances)
5. **REMAINING**: Fix E0310 lifetime constraint (1 instance)

## Session Stats

- **Duration**: ~2 hours
- **TDD Cycles**: 1 complete cycle (red → green → refactor)
- **Commits**: 1
- **Tests**: 1 new, all passing
- **Disk cleanup**: 1.8GB freed (41% usage)
