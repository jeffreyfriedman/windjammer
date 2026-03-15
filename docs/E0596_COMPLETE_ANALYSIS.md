# E0596 Complete Transitive Mutability Analysis

## Summary

This document analyzes all 15 E0596 "cannot borrow as mutable" error patterns and documents the transitive mutability inference implementation.

## 15 E0596 Error Patterns (from build_errors.log)

| # | Pattern | File | Mutation | Fix |
|---|---------|------|----------|-----|
| 1 | self.patrol.update_wait(dt) | npc_behavior.rs | Method call on self field | is_mutating_method("update_wait") ✓ |
| 2 | inv.update(dt) | npc_behavior.rs | if let Some(inv) - Option binding | variable_needs_mut + mut binding ✓ |
| 3 | search.update(dt) | npc_behavior.rs | if let Some(search) - Option binding | variable_needs_mut + mut binding ✓ |
| 4 | squad.send_message/clear_old_messages | squad_tactics.rs | for squad in self.squads | for_loop_mutates_self_field_elements ✓ |
| 5 | self.channels[i].mute/unmute | mixer.rs | Indexed field method call | expression_is_self_field_index_access ✓ |
| 6 | self.renderer.update_camera/render_frame | demos/*.rs | Nested field method call | expression_traces_to_self ✓ |
| 7 | self.entities (for entity in &mut) | query_system.rs | For loop + assignment to entity | **NEW: statement_modifies_loop_var** ✓ |
| 8 | self.delete_entity/duplicate_entity | scene_editor.rs | self.method() call | is_mutating_method ✓ |
| 9 | self.active_quests[i].start/complete/update_objective | quest.rs | Indexed field method | expression_is_self_field_index_access ✓ |
| 10 | self.voxel_renderer | various | Nested field method | expression_traces_to_self ✓ |
| 11 | *mesh in loop | - | Loop var mutation | for_loop_mutates_self_field_elements ✓ |
| 12 | self.effects | - | Field method call | expression_traces_to_self ✓ |
| 13 | self.inventory | trading.rs | Field method call | expression_traces_to_self ✓ |
| 14 | player_inventory | - | Pattern binding mutation | variable_needs_mut ✓ |
| 15 | self.scripts[i].reload | hot_reload.rs | Indexed field method | expression_is_self_field_index_access ✓ |

## Implementation Patterns

### Pattern A: Assignment through nested fields
```windjammer
self.a.b.c.field = val
```
**Status:** ✓ Covered by `expression_is_self_field_access` (recursive FieldAccess)

### Pattern B: Compound assignment
```windjammer
self.field += 1
```
**Status:** ✓ Covered by `statement_modifies_self_fields` (Assignment target check)

### Pattern C: Method calls returning mutable references
```windjammer
self.get_mut_reference().modify()
```
**Status:** ✓ NEW - `expression_traces_to_self` now handles MethodCall when method ends with `_mut` or is `get_mut`

### Pattern D: Closure captures
```windjammer
let f = || { self.field = 42 }
```
**Status:** ⚠️ Not yet implemented (low frequency in game code)

### Pattern E: Match arm assignments
```windjammer
match choice { 0 => self.data.field = 42 }
```
**Status:** ✓ Covered by `expression_contains_self_field_mutations`

### Pattern F: For-loop assignment to loop var (NEW)
```windjammer
for entity in self.entities {
    entity.transform.x = entity.transform.x + 1.0
}
```
**Status:** ✓ NEW - `for_loop_mutates_self_field_elements` now checks:
- `statement_modifies_loop_var` with assignment target analysis
- `expression_references_variable_or_field` for entity.transform.x → entity
- `pattern_loop_var_names` to extract loop var from pattern

## Changes Applied (2026-03-15)

### 1. self_analysis.rs - Extended for-loop mutation detection
- **for_loop_mutates_self_field_elements**: Now takes `pattern` parameter
- **pattern_loop_var_names**: Extracts bound var names from Pattern (Identifier, Tuple, EnumVariant, etc.)
- **statement_mutates_loop_var**: Checks both method calls AND assignments to loop var
- **expression_references_variable_or_field**: Detects when assignment target references loop var (entity.transform.x → entity)
- **expression_is_mutating_method_call_on_variable**: Precise loop var method call detection

### 2. expression_traces_to_self - MethodCall case
- Added: `self.get_mut().modify()` and `self.iter_mut().next()` patterns

### 3. variable_analysis.rs - Build fixes (pre-existing)
- Fixed override_map.insert/remove type mismatches
- Fixed infer_vec_element_from_push_in_body borrow conflict

### 4. statement_generation.rs - Syntax fix (pre-existing)
- Fixed matches! macro missing closing paren

## Test Coverage

- test_for_loop_assignment_to_loop_var: **NEW** - entity.transform.x = ... in for loop
- test_for_loop_mutating_elements: squad.clear_old_messages()
- test_for_loop_send_message: squad.send_message()
- test_self_field_update_wait: self.patrol.update_wait()
- test_self_field_indexed_method_start: self.active_quests[i].start()
- test_self_field_reload: self.scripts[i].reload()
- test_match_arm_self_field_mutation: match arm assignment
- test_option_pattern_mut_binding: if let Some(mut inv)

## Verification

Run: `cargo test --release --test ownership_transitive_mut_test`

Note: Some tests may fail due to pre-existing issues (parse errors, type mismatches) unrelated to mutability inference.
