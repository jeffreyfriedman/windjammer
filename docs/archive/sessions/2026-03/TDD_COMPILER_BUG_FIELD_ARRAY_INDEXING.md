# TDD Compiler Bug: Field Array Indexing with i32

**Date**: 2026-03-21  
**Priority**: CRITICAL - Blocks 700+ game compilation errors  
**Status**: Identified, needs fix

## The Bug

The Windjammer compiler auto-casts i32→usize for direct variable indexing, but NOT for field access + indexing.

### Working Case ✅

**Windjammer**:
```windjammer
let items = [1, 2, 3]
let i = 0  // i32
let x = items[i]  // Works!
```

**Generated Rust**:
```rust
let items = [1, 2, 3];
let i = 0_i32;
let x = items[i as usize];  // ✅ AUTO-CAST!
```

### Broken Case ❌

**Windjammer**:
```windjammer
struct Agent {
    neighbors: Vec<u64>
}

let agent = Agent { neighbors: vec![1, 2, 3] }
let i = 0  // i32
let id = agent.neighbors[i]  // FAILS!
```

**Generated Rust (CURRENT)**:
```rust
let id = agent.neighbors[i];  // ❌ NO CAST! Error: can't index Vec<u64> with i32
```

**Expected Rust**:
```rust
let id = agent.neighbors[i as usize];  // ✅ AUTO-CAST NEEDED!
```

## Impact

**700+ compilation errors** in windjammer-game caused by this single bug:

```
error[E0277]: the type `[u64]` cannot be indexed by `i32`
error[E0277]: the type `[SteeringObstacle]` cannot be indexed by `i32`
error[E0277]: the type `[AStarNode]` cannot be indexed by `i32`
... 700+ more
```

## Root Cause

Looking at `expression_generation.rs`, the `generate_index` function likely handles direct variable indexing (items[i]) but NOT field access chains (agent.neighbors[i]).

The field access path probably generates:
1. Field access: `agent.neighbors`
2. Then indexing: `[i]`
3. But the indexing step doesn't have context that it's operating on an array, so no cast is added.

## The Fix (TDD)

### Step 1: Create failing test

```rust
// tests/field_array_indexing_test.rs
#[test]
fn test_field_array_indexing_with_i32() {
    let test_wj = r#"
struct Agent {
    neighbors: Vec<u64>
}

fn test() {
    let agent = Agent { neighbors: vec![1, 2, 3] }
    let i = 0
    let id = agent.neighbors[i]  // Should auto-cast i → usize
}
"#;
    // ... compile and verify `agent.neighbors[i as usize]` generated
}
```

### Step 2: Fix the compiler

In `expression_generation.rs`, in the `generate_index` or field access code:
- Detect when indexing follows field access
- Apply the same i32→usize cast logic as direct variable indexing
- Ensure ALL array/Vec indexing with integer types gets auto-cast to usize

### Step 3: Verify fix

- Run TDD test → should pass
- Rebuild game → 700+ errors should disappear
- Verify game compiles

## Expected Outcome

After fix:
- ✅ All `agent.field[i]` patterns auto-cast to usize
- ✅ Game compilation errors drop from 1600+ to ~900
- ✅ Consistent behavior: ALL integer indexing gets auto-cast

## Why This Matters

**"Compiler does the hard work, not the developer"** - Windjammer philosophy

Developers shouldn't think about Rust's usize requirement. The compiler should handle:
- i8, i16, i32, i64 → usize (all signed ints)
- u8, u16, u32, u64 → usize (all unsigned ints)  
- Automatically, everywhere, consistently

This is a CORE language feature, not a nice-to-have.

## Related Issues

Similar auto-cast needed for:
- Method call indexing: `get_items()[i]`
- Chained field access: `a.b.c[i]`
- Expression indexing: `(items)[i]`

All should auto-cast integer indices to usize.
