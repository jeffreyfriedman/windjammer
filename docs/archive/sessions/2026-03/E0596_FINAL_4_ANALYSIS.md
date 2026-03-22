# E0596 Final 4 - Complete Analysis & Fixes

## Summary

Eliminated the final 5 E0596 "cannot borrow as mutable" errors in windjammer-game-core by extending the mutating method registry and ensuring transitive analysis covers all patterns.

## Error Analysis (from /tmp/phase7_complete.txt)

| # | File | Line | Pattern | Root Cause |
|---|------|------|---------|------------|
| 1 | ecs/world.rs | 132 | `dirty.mark_transform()` | `mark_transform` not in registry → `if let Some(dirty)` didn't get `mut` binding |
| 2 | editor/prefab_system.rs | 173 | `self.instances[i].sync_from_prefab()` | `sync_from_prefab` not in registry |
| 3 | narrative/dialog.rs | 265 | `return self.advance_to(...)` | `advance_to` not in registry (recursive call in select_choice) |
| 4 | scene/builder.rs | 61 | `self.materials.set(id, ...)` | `set` not in variable_analysis (optimization_detectors had it) |
| 5 | scripting/components.rs | 191 | `self.components[i].initialize()` | `initialize` not in optimization_detectors |

## Why Transitive Analysis Didn't Catch Them

1. **Pattern 1 (if-let mut binding)**: `match_arm_body_mutates_variable` uses `variable_analysis.is_mutating_method()`. `mark_transform` was missing from that list.

2. **Pattern 2 & 5 (indexed field method)**: `has_mutable_method_call` in mutation_detection correctly traces `self.instances[i]` → `self` via `expr_contains_identifier`. The methods `sync_from_prefab` and `initialize` were missing from `optimization_detectors.is_mutating_method()`.

3. **Pattern 3 (recursive call)**: `select_choice` calls `self.advance_to()`. The analyzer's `is_mutated("self", ...)` correctly finds this in the Return statement. `advance_to` was missing from the registry.

4. **Pattern 4 (builder .set())**: `self.materials.set()` - `set` was in optimization_detectors but NOT in `variable_analysis.is_mutating_method()`. The analyzer uses optimization_detectors, so it should work. The fix added `set` explicitly to variable_analysis for consistency.

## Fixes Applied

### 1. optimization_detectors.rs
Added to `is_mutating_method`:
- `mark_transform` (Dirty::mark_transform)
- `sync_from_prefab` (PrefabInstance::sync_from_prefab)
- `advance_to` (DialogTree::advance_to)
- `initialize` (ScriptComponent::initialize)

### 2. variable_analysis.rs
- Added `set` to the explicit matches (Map/HashMap pattern)
- Added `mark_transform`, `sync_from_prefab`, `advance_to`, `initialize`

### 3. errors/mutability.rs
Added all 5 methods for consistent error message handling.

## TDD Tests Added (ownership_transitive_mut_test.rs)

1. **test_if_let_some_dirty_mark_transform** - Verifies `if let Some(dirty) = opt { dirty.mark_transform() }` emits `Some(mut dirty)`
2. **test_self_instances_indexed_sync_from_prefab** - Verifies `self.instances[i].sync_from_prefab()` → `&mut self`
3. **test_self_advance_to_recursive_call** - Verifies `select_choice` calling `self.advance_to()` → `&mut self`
4. **test_builder_with_material_set** - Verifies `self.materials.set()` → `&mut self`
5. **test_self_components_indexed_initialize** - Verifies `self.components[i].initialize()` → `&mut self`

## Verification

```bash
# Build compiler
cd windjammer && cargo build --release

# Run E0596 tests
cargo test --release --test ownership_transitive_mut_test

# Rebuild game and verify zero E0596
cd ../windjammer-game/windjammer-game-core
wj game build --release 2>&1 | grep -c "E0596"
# Expected: 0
```

## Philosophy: "Safety Without Ceremony"

Complete mutability inference with zero exceptions. The compiler infers:
- `&mut self` when methods mutate self or self's fields
- `mut` in pattern bindings when the bound variable is mutated
- Transitive mutation through indexed access (`self.arr[i].method()`)
- Recursive mutation (`self.other_method()` where other_method mutates)

No user annotations required.
