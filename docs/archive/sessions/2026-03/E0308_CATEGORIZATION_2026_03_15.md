# E0308 Systematic Categorization - March 15, 2026

**Goal:** Reduce E0308 "mismatched types" to <100 through pattern-based fixes.

**Source:** build_errors.log (windjammer-game-core), E0308_PHASE7_CATEGORIZATION.md

## Executive Summary

| Pattern | Count | Root Cause | Fix Location | Status |
|---------|-------|------------|--------------|--------|
| **A: Struct tuple float fields** | ~20 | Keyframe/BoneTransform (0.0, 0.0, 0.0, 1.0) | float_inference.rs | ✅ WORKING |
| **B: Pattern binding &T→T** | ~12 | node_a, node_b from match &self.nodes[i] | expression_generation.rs | ✅ WORKING |
| **C: Vec push &path[k]** | 1 | rev.push(&path[k]) when Vec<(i32,i32)> | expression_generation.rs | 🔧 TODO |
| **D: Match arm f32/f64** | 2 | None => 999999.0_f64 vs Some(v)=>*v (f32) | float_inference.rs | 🔧 TODO |
| **E: if/else f32/f64** | ~2 | factor vs 1.0_f64 branch mismatch | float_inference.rs | 🔧 TODO |
| **F: Binary op literal** | ~2 | 6.28318_f64 when LHS is f32 | float_inference.rs | 🔧 TODO |
| **G: String→&str** | 1 | contains_key(animation_name()) | expression_generation.rs | 🔧 TODO |
| **H: Vec/&Vec clone** | 2 | clips expected Vec, found &Vec | expression_generation.rs | ⚠️ PARTIAL |
| **I: const f32 literal** | 1 | MAX_SPEED: f32 = 999999988.0_f64 | float_inference.rs | 🔧 TODO |
| **J: Method arg &roots[j]** | 1 | update_bone_recursive(&roots[j]) | expression_generation.rs | ✅ WORKING |
| **K: if let Some return** | ~4 | if let Some { ... } expected () | statement_generation.rs | Document |
| **L: f32/f64 misc** | ~270 | Various | float_inference | Mixed |

## Pattern Details

### Pattern A: Struct Literal Tuple Float Fields ✅
**Example:** `Keyframe { rotation: (0.0, 0.0, 0.0, 1.0), scale: (1.0, 1.0, 1.0) }`

**Fix:** float_inference StructLiteral handler passes field type to Tuple handler; collect_expression_constraints recurses with Type::Tuple([f32,f32,f32,f32]); each literal gets MustBeF32.

**Verification:** clip.rs line 15 shows `(0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32)`.

### Pattern B: Pattern Binding Deref ✅
**Example:** `match &self.nodes[i] { BlendNode::Lerp { node_a, node_b, .. } => BlendNode::Lerp { node_a, node_b, .. } }`

**Fix:** infer_match_bound_types marks Index scrutinee as ref; enum_variant_struct_fields provides field types; wrap_ref adds &u32 to bindings; struct literal generation emits *node_a when target expects u32 (Copy).

**Verification:** blend_tree.rs lines 76, 79 show `*node_a, *node_b`.

### Pattern C: Vec push &path[k] 🔧
**Example:** `rev.push(&path[k as usize])` when rev: Vec<(i32,i32)>

**Problem:** path[k] returns &(i32,i32). We emit &path[k] → &&(i32,i32). For Copy element type, should push path[k] or *path[k].

**Fix:** In MethodCall for push, when arg is Index and Vec element is Copy, emit *(index_expr) not &(index_expr).

### Pattern D: Match Arm Default f32 🔧
**Example:** `match g_score.get(&(x,y)) { Some(v) => *v, None => 999999.0 }`

**Problem:** *v yields f32, literal 999999.0 defaults to f64. Match arms must unify.

**Fix:** Match arm unification in float_inference - ensure None arm literal gets MustBeF32 from Some arm's *v type.

### Pattern E: if/else f32/f64 🔧
**Example:** `if factor > 1.0 { 1.0 } else { factor }` - factor is f32, 1.0 is f64

**Fix:** Unify if/else branch types; constrain both branches to same float type.

### Pattern F: Binary Op RHS Literal 🔧
**Example:** `member_index as f32 * 6.28318_f64` - LHS is f32, literal is f64

**Fix:** float_inference already has LHS→RHS propagation for binary ops. Verify 6.28318 gets constrained from LHS.

### Pattern G: String→&str for contains_key 🔧
**Example:** `self.animations.contains_key(state.animation_name())` - animation_name() returns String

**Fix:** When method param expects &Q where Q: Borrow<str>, and arg returns String, wrap with .as_ref() or &* in generated code (compiler emits, not user - no Rust leakage in .wj).

### Pattern H: Vec/&Vec Clone ⚠️
**Example:** `BlendNode::Blend1D { clips, parameter: value }` where clips from match is &Vec<Blend1DClip>

**Fix:** When pattern binding is &T and target expects T (T: !Copy), emit ident.clone(). Partial: set_parameter uses clips.clone(); add_1d_clip has `let mut new_clips = clips` - clips is &Vec, need clips.clone() for owned copy.

### Pattern I: const f32 Literal 🔧
**Example:** `pub const MAX_SPEED_UNLIMITED: f32 = 999999988.0_f64`

**Fix:** Item::Const handler already constrains value to type. Verify const type flows to literal.

## Test Suites

### float_inference_field_initializer_test.rs
- test_struct_tuple_field_f32: Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) } → f32

### pattern_binding_deref_codegen_test.rs
- Match-on-index bindings get * for Copy fields

### float_inference_match_arm_test (TODO)
- None => 999999.0 unifies with Some(v) => *v (f32)

### float_inference_if_else_test.rs
- if/else branch unification exists; extend for factor case

## Verification Commands

```bash
# Count E0308
cd windjammer-game && wj game build 2>&1 | grep -c "E0308"

# Or after transpilation:
cd windjammer-game-core && cargo build 2>&1 | grep -c "E0308"

# Target: <100 E0308
```

## Implementation Priority

1. **Pattern D** (Match arm) - 2 errors, high impact for astar
2. **Pattern E** (if/else) - 2 errors, blend_tree
3. **Pattern F** (Binary op) - 2 errors, squad_tactics
4. **Pattern I** (const) - 1 error, character_controller
5. **Pattern C** (Vec push) - 1 error, astar
6. **Pattern G** (String→&str) - 1 error, animation/controller
7. **Pattern H** (add_1d_clip) - Fix new_clips = clips when clips is &Vec
