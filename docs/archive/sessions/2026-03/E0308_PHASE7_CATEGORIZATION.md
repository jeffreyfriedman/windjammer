# E0308 Phase 7 Categorization - 2026-03-15

**Source:** windjammer-game-core build_errors.log  
**Total E0308 Errors:** 328 (from build_errors.log)

## Executive Summary

| Category | Count | Root Cause | Fix Location | Status |
|----------|-------|------------|--------------|--------|
| **A: Struct tuple float fields** | ~20 | Keyframe/BoneTransform (0.0, 0.0, 0.0, 1.0) | float_inference.rs | ✅ Working (verified clip.wj) |
| **B: Pattern binding &T→T** | ~16 | node_a, node_b from match &self.nodes[i] | expression_generation.rs | 🔧 Implement |
| **C: Vec push &path[k]** | 1 | rev.push(&path[k]) when Vec<(i32,i32)> | expression_generation.rs | 🔧 Implement |
| **D: Match arm f32/f64** | 2 | None => 999999.0_f64 vs Some(v)=>*v (f32) | float_inference.rs | 🔧 Implement |
| **E: if/else f32/f64** | ~8 | factor vs 1.0_f64 branch mismatch | float_inference.rs | 🔧 Extend |
| **F: Binary op f32*literal** | ~2 | 6.28318_f64 when LHS is f32 | float_inference.rs | ✅ Has LHS→RHS |
| **G: Comparison f32<literal** | ~2 | survival_rate < 0.3_f64 | float_inference.rs | ✅ Has LHS→RHS |
| **H: Vec/&Vec in struct** | 2 | clips expected Vec, found &Vec | expression_generation.rs | 🔧 Implement |
| **I: String→&str** | 1 | contains_key(animation_name()) | expression_generation.rs | 🔧 Implement |
| **J: const f32 literal** | 1 | MAX_SPEED: f32 = 999999988.0_f64 | float_inference.rs | ✅ Has const handling |
| **K: Method arg &roots[j]** | 1 | update_bone_recursive(&roots[j]) | expression_generation.rs | ✅ Index deref |
| **L: Option/result return** | ~4 | if let Some { ... } expected () | statement_generation.rs | Document |
| **M: f32/f64 (misc)** | ~270 | Various struct/function args | float_inference | Mixed |

## Category B: Pattern Binding Deref (PRIORITY)

**Pattern:** `match &self.nodes[i] { BlendNode::Lerp { node_a, node_b, .. } => BlendNode::Lerp { node_a, node_b, .. } }`

**Problem:** node_a, node_b are &u32 from destructuring, but struct expects u32.

**Fix:** In struct literal field generation, when expr is Identifier from borrowed match binding and target expects Copy, emit *ident.

**Files:** animation/blend_tree.rs (12 errors)

## Category C: Vec push &path[k]

**Pattern:** `rev.push(&path[k])` when rev: Vec<(i32,i32)>

**Problem:** path[k] gives &(i32,i32). We add & making &&(i32,i32). For Copy element type, should push *path[k] or path[k].clone() - but (i32,i32) is Copy so just path[k] (deref, no &).

**Fix:** When pushing to Vec<T> where T is Copy, and arg is Index expression, emit *(index_expr) not &(index_expr).

## Category D: Match Arm Default f32

**Pattern:** `match g_score.get(&(x,y)) { Some(v) => *v, None => 999999.0 }`

**Problem:** *v yields f32, literal 999999.0 defaults to f64. MustMatch between arms should unify.

**Fix:** Match arm unification exists. Ensure None arm literal gets constrained from Some arm's *v type.

## Category H: Vec/&Vec Clone

**Pattern:** `BlendNode::Blend1D { clips, .. }` where clips from match is &Vec<Blend1DClip>

**Fix:** When pattern binding is &T and target expects T (T: !Copy), emit ident.clone().

## Category I: String→&str for contains_key

**Pattern:** `self.animations.contains_key(state.animation_name())` - animation_name() returns String, contains_key expects &str.

**Fix:** When method param expects &str (Borrow<Q>) and arg is String-returning method call, wrap with .as_str() or &*arg in generated code. (Codegen emits it, not user - no Rust leakage in .wj)

## Verification

```bash
# Count E0308 before/after
cd windjammer-game && wj game build 2>&1 | grep -c "E0308"

# Target: <100 E0308
```
