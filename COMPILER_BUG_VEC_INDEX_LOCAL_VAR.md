# Compiler Bug: Vector Indexing Through Local Variable Missing .clone()

## Status
🐛 **IDENTIFIED** - Not yet fixed

## Severity
**HIGH** - Breaks real game code (octree.wj)

## Description
The analyzer auto-inserts `.clone()` when indexing through `self.field[index]` but NOT when indexing through a local variable `local_var[index]`.

## Reproduction
### Works (through self.field):
```windjammer
pub fn get_child(self, index: usize) -> Node {
    self.children[index]  // ✅ .clone() auto-inserted
}
```

Generates:
```rust
self.children[index].clone()  // ✅ Correct
```

### Fails (through local variable):
```windjammer
pub fn get_recursive(node: OctreeNode, ...) -> u8 {
    let children = node.children.unwrap()
    let child = children[idx]  // ❌ NO .clone() inserted!
    Self::get_recursive(child, ...)
}
```

Generates:
```rust
let children = node.children.unwrap();
let child = children[idx];  // ❌ E0507: cannot move out of index
```

## Expected Behavior
When indexing a `Vec<T>` where `T: Clone` and the result is moved (not borrowed), the analyzer should auto-insert `.clone()` for BOTH cases:
- `self.field[index]` ✅ Already works
- `local_var[index]` ❌ Currently fails

## Root Cause
The ownership analyzer in `src/analyzer/mod.rs` handles `Expression::Index` differently depending on whether the base expression is `Expression::FieldAccess` (works) vs `Expression::Identifier` (fails).

## Fix Location
File: `src/analyzer/mod.rs`  
Function: Ownership inference for `Expression::Index`

Need to ensure that when:
1. Base is ANY expression that resolves to `Vec<T>`
2. Result type `T` implements `Clone`
3. The indexed value is being moved (not borrowed)

Then auto-insert `.clone()`.

## Test Case
File: `tests/vec_index_local_var.wj`
```windjammer
struct Node {
    children: Option<Vec<Node>>,
}

pub fn direct_access(self, index: usize) -> Node {
    let children = self.children.unwrap()
    children[index]  // Should compile with auto-clone
}
```

## Impact
**Blocking:**
- `voxel/octree.wj` - Cannot compile recursive octree traversal
- Any code using pattern: `let vec = ...; vec[i]`

## TDD Approach
1. ✅ Create failing test (`vec_index_local_var.wj`)
2. ⏳ Fix analyzer to handle local variable indexing
3. ⏳ Verify test passes
4. ⏳ Verify octree.wj compiles
5. ⏳ Run full test suite

## Related
- Working test: `tests/vector_indexing_ownership.wj` (through self.field)
- Failing code: `src_wj/voxel/octree.wj` line 130

## Windjammer Philosophy
This bug violates: **"Compiler Does the Hard Work, Not the Developer"**

The compiler should automatically handle the mechanical detail of cloning when moving out of a vector, regardless of whether it's accessed through `self.field` or a local variable.

---
**Date Identified:** 2026-02-24
**Reported By:** TDD Dogfooding Session
**Priority:** HIGH - Blocks game engine compilation
