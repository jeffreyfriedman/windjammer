# E0308 Game Code Issues - Manual Fixes Required

**Date:** 2026-03-14  
**Context:** These are E0308 errors that require game code changes, not compiler fixes.

## 1. astar_grid.rs - Match Arm Type Mismatch

**Error:** `match` arms have incompatible types - Some(v) => *v (f32) vs None => (different type)

**Cause:** Default/None arm returns a different type than Some arm. Need explicit type for the match.

**Fix:** Add type annotation or ensure both arms return same type:
```rust
let current_g = match g_score.get(&(current_x, current_y)) {
    Some(v) => *v,
    None => f32::MAX,  // or appropriate default
};
```

## 2. astar_grid.rs - Vec Push Type

**Error:** rev.push(&path[k]) - Vec type mismatch (Vec<&(i32,i32)> vs Vec<(i32,i32)>)

**Cause:** Pushing reference when Vec expects owned.

**Fix:** Clone or dereference: `rev.push(path[k as usize].clone())` or fix rev's type.

## 3. asset_manager.rs - Insert Return Value

**Error:** expected `()`, found `Option<Asset>`

**Cause:** HashMap::insert returns Option, block expects ().

**Fix:** Use `let _ = self.assets.insert(...)` or `drop(self.assets.insert(...))`

## 4. pipeline.rs - add_texture Return Value

**Error:** expected `()`, found `bool`

**Cause:** add_texture returns bool, if-let block expects ().

**Fix:** Use `let _ = atlas.add_texture(...)` or handle return value explicitly.

## 5. animation/controller.rs - contains_key Argument

**Error:** expected `&_`, found `String` for contains_key(state.animation_name())

**Cause:** HashMap::contains_key expects &Q. animation_name() returns String.

**Compiler fix possible:** Auto-coerce String to &str for method args. Until then:
**Game fix:** Pass reference: `self.animations.contains_key(&state.animation_name())` - but that borrows. May need `state.animation_name().as_str()` (Rust leakage) or different API design.

## 6. skeleton.rs - update_bone_recursive Argument

**Error:** expected `u32`, found `&u32` for &roots[j]

**Cause:** roots[j] is &u32 from Vec index, function expects u32.

**Fix:** Pass *roots[j] or roots[j].clone() - compiler deref fix may cover this when in struct literal context. For direct function arg, may need explicit deref in game code.

## Summary

| File | Issue | Priority |
|------|-------|----------|
| astar_grid.rs | Match arm types, Vec push | P1 |
| asset_manager.rs | Insert return | P2 |
| pipeline.rs | add_texture return | P2 |
| animation/controller.rs | contains_key String | P1 (compiler fix preferred) |
| skeleton.rs | update_bone_recursive &u32 | P2 (compiler may fix) |

**Note:** Compiler fixes for pattern binding deref (blend_tree) and if/else float unification will reduce many errors. Re-run build after compiler updates to get accurate remaining count.
