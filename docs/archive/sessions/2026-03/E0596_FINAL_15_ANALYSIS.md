# E0596 Final 15 - Complete Analysis & Fixes

## Summary

This document analyzes all E0596 "cannot borrow as mutable" error patterns and documents the fixes applied.

## 15 E0596 Error Patterns (from build_errors.log)

| # | Pattern | File | Root Cause | Fix Status |
|---|---------|------|------------|------------|
| 1 | self.patrol.update_wait(dt) | npc_behavior.rs | Method on self field | ✓ is_mutating_method |
| 2 | inv.update(dt) | npc_behavior.rs | if let Some(inv) Option binding | ✓ variable_needs_mut + mut |
| 3 | search.update(dt) | npc_behavior.rs | if let Some(search) Option binding | ✓ variable_needs_mut + mut |
| 4 | squad.send_message/clear_old_messages | squad_tactics.rs | for squad in &self.squads | ✓ for_loop_mutates |
| 5 | self.channels[i].mute/unmute | mixer.rs | Indexed field method | ✓ expression_is_self_field_index |
| 6 | self.renderer.update_camera/render_frame | demos/*.rs | Nested field method | ✓ expression_traces_to_self |
| 7 | self.entities (for entity in &mut) | query_system.rs | For loop + assignment | ✓ statement_modifies_loop_var |
| 8 | self.delete_entity/duplicate_entity | scene_editor.rs | self.method() call | ✓ is_mutating_method |
| 9 | self.active_quests[i].start/complete/update_objective | quest.rs | Indexed field method | ✓ expression_is_self_field_index |
| 10 | self.voxel_renderer | game_renderer.rs | Nested field in trait impl | ⚠️ Trait impl self inference |
| 11 | *mesh in generate_merged_quad_x | mesh_generator.rs | mesh.add_quad() in match arm | ✓ Statement::Match + is_mutating_method |
| 12 | self.effects | post_processing.rs | for effect in &mut self.effects | ✓ expression_traces_to_self |
| 13 | self.inventory | trading.rs | Field method call | ✓ expression_traces_to_self |
| 14 | player_inventory.add_gold | trading.rs | Parameter mutation | ✓ infer_passthrough / is_mutated |
| 15 | self.scripts[i].reload | hot_reload.rs | Indexed field method | ✓ expression_is_self_field_index |

## Fixes Applied (2026-03-15)

### 1. mutation_detection.rs - Use comprehensive is_mutating_method
**Before:** Limited heuristic (push, insert, remove, set, etc.)
**After:** `return self.is_mutating_method(method)` - uses full optimization_detectors list

**Impact:** Parameter mutation detection now catches add_quad, add, add_gold, etc.

### 2. optimization_detectors.rs - Add mesh/inventory methods
**Added:** `add_quad`, `add` to is_mutating_method matches

**Impact:** mesh.add_quad() and stack.add() now recognized as mutating

### 3. mutation_detection.rs - Statement::Match handling
**Added:** Recurse into match arm bodies when checking parameter mutation
```rust
Statement::Match { arms, .. } => {
    for arm in arms {
        if self.has_mutable_method_call(name, arm.body, registry) {
            return true;
        }
    }
}
```

**Impact:** mesh.add_quad() inside `match direction { PosX => { mesh.add_quad(...) } }` now detected

## Patterns NOT Yet Implemented

### Pattern A: Closure mutations
```windjammer
pub fn process(self) {
    let f = || { self.field = 42 }
    f()  // Needs &mut self
}
```
**Status:** Not implemented (low frequency in game code)

### Pattern B: Trait impl self inference
When impl methods use `self` implicitly (trait has no self), the impl may need &mut self.
**Status:** Trait method analysis may need enhancement for RenderPort-style traits

### Pattern C: if let Some(x) = &mut self.field
When binding needs ref mut for mutation, pattern generation may need `Some(ref mut x)`.
**Status:** generate_pattern_with_mut_bindings produces `mut x`; may need `ref mut x` for Option elements

## Test Cases Added

- `test_param_mutating_method_direct` - mesh.add_quad() direct call
- `test_param_mutating_method_in_match_arm` - mesh.add_quad() inside match arms

## Verification

```bash
# Run transitive mutability tests
cargo test --release --test ownership_transitive_mut_test

# Rebuild game and count E0596
cd windjammer-game-core && wj build ... && cargo build 2>&1 | grep -c "E0596"
```

## Philosophy Alignment

**"Safety Without Ceremony"** - Complete mutability inference without requiring users to annotate &mut.

The compiler now:
1. Infers &mut for parameters mutated via method calls (including in match arms)
2. Uses comprehensive method registry for mutation detection
3. Handles nested contexts (match, if, for, while)
