# E0282 Type Inference Analysis - 2026-03-15

## Summary

Analysis of 27 E0282 "type annotations needed" errors in windjammer-game-core, with implemented fixes and documentation of patterns.

## Error Distribution (27 total)

| File | Count | Patterns |
|------|-------|----------|
| achievement/manager.rs | 6 | for-loop var `ach`, match arm `a`, if-let `achievement` |
| animation/controller.rs | 2 | if-let `frame`, match arm `a` |
| dialogue/manager.rs | 7 | if-let `node`, `choice`, `next_id`, return `node.choices()` |
| inventory/inventory.rs | 7 | if-let `stack`, match arm |
| lod/lod_manager.rs | 1 | field access `mesh.value()` |
| save/manager.rs | 1 | match arm `data.clone()` |
| state_machine/machine.rs | 3 | match arm `state`, return `state.name()` |

## Categorization

### Pattern 1: Vec::new() with return type (FIXABLE) ✅ IMPLEMENTED

```windjammer
pub fn get_unlocked_achievements(self) -> Vec<&Achievement> {
    let mut result = Vec::new()
    for ach in self.achievements.values() {
        if ach.is_unlocked() { result.push(ach) }
    }
    result  // variable returned - infer Vec<&Achievement> from return type
}
```

**Fix:** `infer_vec_from_return_type()` - when variable is returned and function return type is `Vec<T>`, annotate with `Vec<T>`.

**Impact:** Fixes 6+ Vec::new() cases in achievement/manager, dialogue/manager, etc.

### Pattern 2: Vec::new() with .push() in loop (FIXABLE) ✅ ALREADY IMPLEMENTED

```windjammer
let mut particles = Vec::new()
while i < 10 {
    particles.push(Particle::new(...))
}
```

**Fix:** `extract_push_arg_type_from_statement` recurses into While/For/Loop bodies (E0282_REGRESSION_INVESTIGATION).

### Pattern 3: For-loop iterator variable (PARTIALLY FIXABLE)

```windjammer
for ach in self.achievements.values() {
    if ach.is_unlocked() { ... }  // ach needs type &Achievement
}
```

**Root cause:** Rust infers `ach` from `self.achievements.values()` which yields `&V`. The type flows from `self.achievements: HashMap<AchievementId, Achievement>`. If Vec::new() gets proper annotation from return type, the `result.push(ach)` constrains the loop - **Pattern 1 fix may resolve this**.

### Pattern 4: Match arm bindings (COMPLEX)

```windjammer
match self.achievements.get(&id) {
    Some(a) => a.is_unlocked(),  // a needs type &Achievement
    None => false,
}
```

**Root cause:** Type flows from `get()` return type `Option<&Achievement>`. The HashMap's type parameters must be known. Struct field type should provide this.

### Pattern 5: If-let bindings (COMPLEX)

```windjammer
if let Some(achievement) = self.achievements.get_mut(&id) {
    achievement.add_progress(amount)  // achievement needs type &mut Achievement
}
```

**Root cause:** Same as Pattern 4 - type flows from method return type.

## Implemented Fixes

### 1. Return Type Inference for Vec::new()

**Files:** `variable_analysis.rs`, `statement_generation.rs`

- Added `infer_vec_from_return_type(var_name)` - when variable is returned and fn returns `Vec<T>`, use that type
- Added `variable_is_returned_in_body(var_name)` - checks `return x` and last-expr `x`
- Recurses into If/For/While/Loop for return statements
- Fallback when push inference fails (e.g. push arg type from loop var unknown)

### 2. TDD Test

**File:** `tests/type_inference_ambiguity_test.rs`

- `test_vec_new_inferred_from_return_type` - `let mut result = Vec::new(); return result` with fn -> Vec<u32>

## Unfixable Patterns (Require User Annotation)

### Truly ambiguous Vec::new()

```windjammer
pub fn unclear() {
    let items = Vec::new()  // No push, no return, no usage
}
```

**Required:** User must add `let items: Vec<i32> = Vec::new()` or similar.

### Unused variables

```windjammer
let map = HashMap::new()  // Never used
```

**Required:** User annotation or remove variable.

## Philosophy Alignment

**"Correctness Over Speed"** - Only infer when sound (return type, push arg). Never guess.

**"Inference when possible, explicit when ambiguous"** - Use all available context.

## Verification

To verify the fix reduces E0282 count:

```bash
cd breach-protocol
wj build src/
cd runtime_host
cargo build --release 2>&1 | rg "E0282" | wc -l
```

Expected: Reduction from 27 toward 15-20 (return type inference helps Vec cases).

## Future Work

1. **Iterator element type propagation** - Track for-loop variable types from iterable (e.g. `values()` → `&V`)
2. **Match scrutinee type flow** - When match is on `self.map.get(&k)`, flow Option's inner type to arm bindings
3. **Turbofish for generic calls** - Emit `Vec::<T>::new()` when type known from context
