# Transitive Mutability Inference - E0596 Analysis

## Summary

**Status:** Implementation exists and works for core patterns. Some E0596 errors remain due to:
1. Missing methods in `is_mutating_method` registry
2. Option pattern (`if let Some(x) = ...`) needing `mut x` when x is mutated
3. build_errors.log may be from older build - current generated code shows correct output

## Error Categories from build_errors.log (41 E0596 total)

### Category 1: self.field.method() - Transitive mut ✅ IMPLEMENTED
- `self.channels[i].mute()` → mute_channel needs &mut self
- `self.renderer.update_camera()` → render needs &mut self  
- `self.renderer.render_frame()` → render needs &mut self
- **Status:** mute, update_camera, render_frame ARE in optimization_detectors. Current mixer.rs, squad_tactics.rs show correct &mut self.

### Category 2: for item in self.field { item.mutate() } ✅ IMPLEMENTED
- `for squad in self.squads { squad.send_message(...) }` → needs &mut self.squads
- `for squad in self.squads { squad.clear_old_messages(...) }` → needs &mut self.squads
- **Status:** send_message, clear_old_messages in list. Current squad_tactics.rs has `for squad in &mut self.squads`.

### Category 3: self.field.method() - MISSING from registry
- `self.patrol.update_wait(dt)` → update_patrol needs &mut self
- **Fix:** Add `update_wait` to is_mutating_method

### Category 4: Option pattern - if let Some(x) = opt { x.mutate() }
- `if let Some(inv) = self.investigation { inv.update(dt) }` → need `mut inv`
- `if let Some(search) = self.search { search.update(dt) }` → need `mut search`
- **Fix:** Add `update` to is_mutating_method AND ensure variable_needs_mut infers mut for pattern bindings

### Category 5: Other patterns
- E0507 (move) - different issue
- Various self.field mutations - analyzer should catch via expression_traces_to_self

## Implementation Locations

| Component | File | is_mutating_method |
|-----------|------|-------------------|
| Analyzer | optimization_detectors.rs | Comprehensive (mute, clear_old_messages, update_camera, render_frame) |
| Codegen | variable_analysis.rs | For loop body (clear_, send_, mute, etc.) |
| Codegen | function_generation.rs | Limited inline - NOT used for impl methods |
| Errors | mutability.rs | For error suggestions |

## Key Finding

The **analyzer** determines &mut self via `function_modifies_self_fields_with_registry` which uses:
- expression_is_self_field_mutating_method_call (self.channels[i].mute())
- for_loop_mutates_self_field_elements (squad.clear_old_messages)
- is_mutating_method from optimization_detectors

The **codegen** uses `analyzed.inferred_ownership.get("self")` for impl methods - so analyzer result flows through.

The **for-loop** codegen uses variable_analysis.loop_body_modifies_variable → is_mutating_method. Both need same methods.

## Recommended Fixes

1. ✅ Add `update_wait` to is_mutating_method (update already present)
2. Option pattern mutability (if let Some(mut x) when x.mutate() in body) - separate investigation

## Changes Applied (2026-03-14)

1. **optimization_detectors.rs**: Added `update_wait` to is_mutating_method
2. **variable_analysis.rs**: Added `update_wait` and `update` to is_mutating_method  
3. **errors/mutability.rs**: Added `update_wait` and `update` to is_mutating_method
4. **ownership_transitive_mut_test.rs**: Added test_self_field_update_wait() for E0596 npc_behavior pattern

## Changes Applied (2026-03-15) - Phase 4: Complete E0596 Fix

### Extended Method Registry (all 3 files)
- **Quest/narrative**: start, complete, update_objective
- **Renderer**: set_lighting, set_exposure, set_gamma, set_bloom_threshold, set_vignette
- **Voxel GPU**: init_gpu, upload_svo, rebuild_shader_graph, do_render_frame, do_shutdown
- **Editor**: delete_entity, duplicate_entity
- **Inventory/RPG**: remove_item_by_id, add_gold, reload
- **For-loop**: clear_old_messages (variable_analysis)

### Option Pattern Mut Binding
- **Pattern**: `if let Some(x) = opt { x.mutate() }` → emit `if let Some(mut x)`
- **Implementation**: match_arm_body_mutates_variable() + generate_pattern_with_mut_bindings()
- **Files**: variable_analysis.rs, statement_generation.rs

### New Test Cases
- test_option_pattern_mut_binding: if let Some(mut inv) when body calls inv.update()
- test_self_field_indexed_method_start: self.active_quests[i].start() → &mut self
- test_self_field_reload: self.scripts[i].reload() → &mut self
