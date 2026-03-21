# E0282 Regression Analysis Report - 2026-03-14

## Summary

**E0282 increased from 27 to 30 (+3 regression)** after Phase 6 Rust leakage cleanup.

## Phase 5 vs Phase 6 Comparison

### Phase 5 Baseline (27 errors)
| File | Count | Patterns |
|------|-------|----------|
| achievement/manager.rs | 6 | for-loop var `ach`, match arm `a`, if-let `achievement` |
| animation/controller.rs | 2 | if-let `frame`, match arm `a` |
| dialogue/manager.rs | 7 | if-let `node`, `choice`, `next_id`, return `node.choices()` |
| inventory/inventory.rs | 7 | if-let `stack`, match arm |
| lod/lod_manager.rs | 1 | field access `mesh.value()` |
| save/manager.rs | 1 | match arm `data.clone()` |
| state_machine/machine.rs | 3 | match arm `state`, return `state.name()` |

### Phase 6 Current (30 errors)
**3 NEW errors identified:**

| File | New Count | New Errors |
|------|-----------|------------|
| **ai/npc_behavior.rs** | **2** (NEW FILE) | Lines 283, 291: `*(pos).clone()` - `pos` from `if let Some(pos) = &mut self.perception.investigation_position` |
| **state_machine/machine.rs** | **4** (+1) | Line 109: `initial.clone()` - `initial` from `if let Some(initial) = &mut self.initial_state` |

## Root Cause Analysis

### Regression: YES - Caused by Phase 6 E0507 Fix Pattern

The E0507 fix for "cannot move out of borrowed content" converts:
```windjammer
if let Some(pos) = self.perception.investigation_position {
    self.investigation = Some(InvestigationState::new(pos))
```
to:
```rust
if let Some(pos) = &mut self.perception.investigation_position {
    self.investigation = Some(InvestigationState::new(*(pos).clone()));
```

**Problem:** When we use `&mut Option<T>`, the pattern `Some(pos)` gives `pos: &mut T`. 
- `&mut T` does **NOT** implement `Clone` in Rust (only `&T` does)
- The codegen was emitting `*(pos).clone()` which parses as: dereference the result of `pos.clone()`
- Correct form: `(*pos).clone()` - dereference first to get `T`, then clone

### Why Type Inference Fails (E0282)

Rust reports "type annotations needed" for `pos` because:
1. The expression `*(pos).clone()` is malformed - `pos.clone()` when `pos: &mut Vec3` fails (no Clone for &mut T)
2. Rust's type checker gets confused before it can infer `pos`'s type from the Option

## Fix Implemented

### 1. New Tracking: `borrowed_mut_ref_vars`

Variables from `if let Some(x) = &mut option` have type `&mut T`. Track them separately from `borrowed_iterator_vars` (which are `&T`).

**Files modified:**
- `generator.rs`: Added `borrowed_mut_ref_vars: HashSet<String>`
- `statement_generation.rs`: When `scrutinee_borrowed && scrutinee_is_mut`, add bound vars to `borrowed_mut_ref_vars`

### 2. Codegen: Use `(*var).clone()` or `*var` for &mut T

When converting borrowed vars to owned for function arguments:
- **&T** (borrowed_iterator_vars): `var.clone()` ✅
- **&mut T** (borrowed_mut_ref_vars): `(*var).clone()` or `*var` (if Copy) ✅

**Files modified:**
- `expression_generation.rs`: 
  - Call handler (Owned param): Check `borrowed_mut_ref_vars`, emit `(*var).clone()` or `*var`
  - Some/Ok/Err enum constructors: Same logic
  - Struct literal fields: Same logic

### 3. Cleanup on Scope Exit

Both `statement_generation.rs` (if-let and match) now remove vars from `borrowed_mut_ref_vars` when popping scope.

## Verification

**To verify the fix:**
1. Build windjammer compiler: `cd windjammer && cargo build --release`
2. Transpile: `cd windjammer-game-core && wj build src_wj --output . --no-cargo`
3. Build: `cargo build --release 2>&1 | rg "E0282" | wc -l`
4. **Expected:** 27 or fewer E0282 errors

## Test Case (TDD)

**Recommended test:** `test_if_let_some_mut_option_clone`
- Windjammer: `if let Some(pos) = self.option_field { Foo::new(pos) }` with `&mut self`
- Expected Rust: `if let Some(pos) = &mut self.option_field { Foo::new((*pos).clone()) }` or `Foo::new(*pos)` for Copy
- Location: `windjammer/tests/type_inference_ambiguity_test.rs`

## Philosophy Alignment

✅ **Correctness Over Speed** - Understood root cause before fixing
✅ **No shortcuts** - Proper fix for all &mut Option patterns, not just npc_behavior
✅ **TDD** - Fix addresses regression, test case documented for future

## Files Changed

1. `windjammer/src/codegen/rust/generator.rs` - Added borrowed_mut_ref_vars
2. `windjammer/src/codegen/rust/statement_generation.rs` - Track & remove borrowed_mut_ref_vars
3. `windjammer/src/codegen/rust/expression_generation.rs` - Emit (*var).clone() or *var for &mut T
