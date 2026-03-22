# E0507 Complete Categorization & Fix Plan (2026-03-15)

**Goal:** Eliminate remaining "cannot move out of" (E0507) errors through ownership inference.  
**Philosophy:** "Compiler Does the Hard Work" - ownership should be invisible.

## Complete E0507 Error Categorization (from build_errors.log)

| # | Pattern | File:Line | Example | Root Cause | Fix |
|---|---------|-----------|---------|------------|-----|
| 1 | **Pattern B: Option if-let with &mut self** | npc_behavior.rs:255 | `if let Some(search) = self.search` | Move from &mut self | `&mut self.search` |
| 2 | **Pattern D: Vec index + method(owned self)** | clip.rs:125 | `self.tracks[i].sample(time)` | sample(self) consumes | `tracks[i].clone().sample(time)` |
| 3 | **Pattern D: Option match iterator var** | query_system.rs:200 | `match entity.health` in for loop | entity is &Entity | `&entity.health` |
| 4 | **Pattern D: Option if-let iterator var** | query_system.rs:252 | `if let Some(vel) = entity.velocity` | entity is &Entity | `&entity.velocity` |
| 5 | **Pattern C: Builder pattern self.method()** | hierarchy_panel.rs:124 | `self.build_node(*entity_id, 0)` | &self, method takes owned | `self.clone().build_node(...)` |
| 6 | **Pattern A: Vec index let binding** | particle_editor.rs:136 | `let preset = self.available_presets[i]` | Move from index | `&self.available_presets[i]` |
| 7-9 | **Pattern C: Builder pattern** | shader_graph.rs:340,344,348 | `self.input_uniform(buffer)` | &self, method takes owned | `self.clone().input_uniform(...)` |
| 10-11 | **Pattern C: Struct literal self fields** | shader_graph.rs:402 | `bindings: self.bindings` | Move from &self | `self.bindings.clone()` |
| 12 | **Pattern C: Let binding nested field** | shader_graph.rs:403 | `let mut new_passes = self.graph.passes` | Move from &self | `self.graph.passes.clone()` |
| 13-14 | **Pattern D: Option match param** | octree.rs:133,152 | `match node.children { Some(c) => c }` | node is &OctreeNode | `&node.children` |

## Pattern Taxonomy

### Pattern A: Vec Iteration Consuming
```windjammer
for item in self.items {  // &self
    process(item)  // Needs &item or clone
}
```
**Fix:** `for item in &self.items` → item is &T, add to borrowed_iterator_vars.

### Pattern B: Option Unwrap/If-Let with &self/&mut self
```windjammer
if let Some(x) = self.opt { ... }  // &mut self
```
**Fix:** `if let Some(x) = &mut self.opt` when base in inferred_mut_borrowed_params.

### Pattern C: Field Returns / Struct Literal / Let Binding
```windjammer
pub fn get_data(self) -> Vec<T> { self.data }  // &self
PassDef { bindings: self.bindings }  // &self
let mut x = self.graph.passes  // &self
```
**Fix:** Add `.clone()` when root in inferred_borrowed_params and in owned context.

### Pattern D: Match Arms with Borrowed Base
```windjammer
match entity.health { Some(h) => ... }  // entity from for entity in &self.entities
match node.children { Some(c) => c }    // node is &OctreeNode param
```
**Fix:** `&expr.field` or `&mut expr.field` when base in borrowed params or borrowed_iterator_vars.

## Implemented Fixes (Codebase)

1. **match_scrutinee_is_borrowed_field** - Checks inferred_borrowed_params, inferred_mut_borrowed_params, borrowed_iterator_vars
2. **match_scrutinee_base_identifier** - Extracts base (self, node, entity) from field chain
3. **is_iterating_over_borrowed** - Now checks inferred_borrowed_params for self/param.field (ENHANCED)
4. **FieldAccess clone** - When root borrowed + (in_struct_literal_field OR in_owned_value_context) + not Copy → .clone()
5. **MethodCall object_is_borrowed** - When method takes owned self and object borrowed → .clone()
6. **Index handler** - Vec index non-Copy → &vec[i] for read, vec[i].clone() for owned method
7. **Let handler** - Sets in_owned_value_context for field clone inference

## Decision Framework

| Context | Borrow | Clone |
|---------|--------|-------|
| Vec index, read-only | `&vec[i]` | - |
| Vec index, method(owned self) | - | `vec[i].clone().method()` |
| Option match/if-let, borrowed base | `&expr` or `&mut expr` | - |
| Struct literal, borrowed base | - | `expr.clone()` |
| Let binding, borrowed base | - | `expr.clone()` |
| self.method() owned, self borrowed | - | `self.clone().method()` |

## Test Suite (e0507_ownership_inference_test.rs)

20 tests covering all patterns. Run:
```bash
cd windjammer && cargo test --test e0507_ownership_inference_test --features cli
```

## Verification

```bash
# Compile single file that had E0507
cd windjammer-game-core
wj build src_wj/ai/npc_behavior.wj --output /tmp/e0507_test --library --no-cargo
# Check generated Rust for &mut self.search in if-let
```

## Files Changed (This Session)

- statement_generation.rs: Removed debug eprintln!, match/if-let borrow logic
- variable_analysis.rs: is_iterating_over_borrowed checks inferred_borrowed_params
- expression_generation.rs: (existing) FieldAccess clone, MethodCall object_is_borrowed, Index handler
- generator.rs: (existing) in_owned_value_context
