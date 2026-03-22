# E0308 Mismatched Type Errors - Categorization Report

**Date:** 2026-03-14  
**Source:** windjammer-game-core build_errors.log  
**Total E0308 Errors:** ~304 (328 in log)

## Executive Summary

| Category | Count | Root Cause | Fix Location |
|----------|-------|------------|--------------|
| **f32 vs f64** | ~275 | Float literal inference gaps | Compiler: float_inference.rs |
| **u32 vs &u32** | ~10 | Pattern binding from match-on-ref | Compiler: expression_generation.rs |
| **Vec vs &Vec** | ~3 | Pattern binding from match-on-ref | Compiler or game code |
| **f32 vs f64 (if/else)** | ~22 | Branch type unification | Compiler: float_inference.rs |
| **String vs &str** | 1 | HashMap::contains_key expects &str | Compiler: method call coercion |
| **Other** | ~13 | Various (Option, bool, etc.) | Case-by-case |

## Pattern 1: f32 vs f64 (COMPILER BUG)

**Symptoms:** `expected f32, found f64` or `expected f64, found f32`

**Examples:**
- `Keyframe { rotation: (0.0_f64, ...) }` when struct expects `(f32, f32, f32, f32)`
- `6.28318_f64` in expression `member_index as f32 * 6.28318_f64`
- `0.3_f64` in comparison `survival_rate < 0.3_f64` when survival_rate is f32

**Root Cause:** Float inference doesn't propagate from:
1. Struct field types (tuple elements in `(f32, f32, f32, f32)`)
2. Binary operation LHS (when LHS is f32, RHS literal should be f32)
3. Comparison operands (unify both sides)
4. if/else branches (both branches must have same type)

**Fix:** Extend `float_inference.rs`:
- Struct literal: Recurse into tuple field values with tuple element types
- Binary op: Add MustMatch for both operands when one has known float type
- If/else: Add MustMatch between then and else block last expressions

## Pattern 2: u32 vs &u32 (COMPILER BUG)

**Symptoms:** `expected u32, found &u32` with "consider dereferencing the borrow"

**Examples:**
- `BlendNode::Lerp { node_a, node_b, blend_factor: value }` where node_a, node_b come from `match &self.nodes[i]`
- `update_bone_recursive(&roots[j], ...)` where roots[j] is &u32 but function expects u32

**Root Cause:** When matching on `&collection[i]`, pattern bindings are references. Using them in struct literals that expect owned Copy types requires dereference.

**Fix:** In codegen, when generating Identifier in context of struct literal field, and the identifier is a pattern binding from a match-on-reference arm, emit `*ident` for Copy types.

## Pattern 3: Vec vs &Vec (COMPILER BUG / GAME CODE)

**Symptoms:** `expected Vec<Blend1DClip>, found &Vec<Blend1DClip>`

**Examples:**
- `BlendNode::Blend1D { clips, parameter: value }` where clips from pattern match

**Root Cause:** Same as Pattern 2 - match on ref gives &Vec. Struct expects Vec. Need `.clone()`.

**Fix:** Codegen: When pattern binding is &T and target expects T (T: !Copy), emit `ident.clone()`.

## Pattern 4: if/else Incompatible Types (COMPILER BUG)

**Symptoms:** `` `if` and `else` have incompatible types ``

**Examples:**
```rust
let clamped = if factor > 1.0_f32 { 1.0_f64 } else { factor };
// if branch: f64, else branch: f32
```

**Root Cause:** Float inference doesn't unify branches. Each branch infers independently. Need MustMatch between branch expressions.

**Fix:** In `collect_statement_constraints` for If, add constraint that then_block last expr and else_block last expr must match when both produce values.

## Pattern 5: String vs &str (COMPILER BUG)

**Symptoms:** `expected &_, found String` for `contains_key(state.animation_name())`

**Root Cause:** HashMap::contains_key expects &Q where Q: Borrow<K>. For HashMap<String, V>, we need &str. Passing String - compiler should coerce to &str or pass &*s.

**Fix:** Method call argument coercion - when param expects &str and arg is String, emit `arg.as_str()` or `&*arg`. (Note: .as_str() is Rust leakage per no-rust-leakage.mdc - the codegen would emit it, not the user.)

## Pattern 6: Other (GAME CODE / DOCUMENT)

| File | Issue | Fix |
|------|-------|-----|
| astar_grid.rs | match arms Some(v)=>*v vs None=>f32 - f32 default | Add explicit type or fix default |
| astar_grid.rs | rev.push(&path[k]) - Vec type mismatch | Fix collection type |
| asset_manager.rs | insert returns Option, expected () | Use let _ = or ignore return |
| pipeline.rs | add_texture returns bool, expected () | Use let _ = or fix |
| camera2d.rs | if/else f32 vs f64 | Float inference |

## Recommended Fix Order

1. **Pattern 2 (u32/&u32)** - Small, clear fix, high impact on blend_tree
2. **Pattern 4 (if/else)** - Unifies 22 errors, extends float inference
3. **Pattern 1 (f32/f64)** - Largest category, struct/tuple/binary
4. **Pattern 3 (Vec/&Vec)** - Similar to Pattern 2
5. **Pattern 5 (String/&str)** - Single error
6. **Pattern 6** - Document for game code fixes

## TDD Test Files to Create

1. `pattern_binding_deref_codegen_test.rs` - Match on &enum, use bindings in struct
2. `float_inference_if_else_unification_test.rs` - if/else both return float
3. `float_inference_binary_op_propagation_test.rs` - f32 * 1.0 → 1.0_f32
