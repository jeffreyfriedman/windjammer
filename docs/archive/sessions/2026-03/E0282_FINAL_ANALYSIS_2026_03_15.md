# E0282 Final Fix - 2026-03-15

## Result

**E0282 reduced from 20 to 0** in windjammer-game-core.

## Implemented Fixes

### 1. infer_match_bound_types - Some(var) when scrutinee is &mut/&Option<T>

**File:** `windjammer/src/codegen/rust/type_analysis.rs`

When the AST has `Some(pos)` but the codegen emits `Some(ref mut pos)` (because scrutinee is `&mut self.field`), we now return `MutableReference(T)` instead of `T`. This ensures `local_var_types` has the correct type for the turbofish.

**Case added:** For `EnumVariant("Some", Single(var_name))` when `is_ref || is_mut_ref`, return `MutableReference(inner)` or `Reference(inner)`.

### 2. (*var).clone() turbofish

**File:** `windjammer/src/codegen/rust/expression_generation.rs`

When emitting `(*var).clone()` for `borrowed_mut_ref_vars`, add `::<T>` when we have `Type::MutableReference(inner)` in `local_var_types`: `(*pos).clone::<Vec3>()`.

### 3. Expanded type ascription for if-let and match arms

**File:** `windjammer/src/codegen/rust/statement_generation.rs`

Emit `let x: T = x` for ALL inferrable bindings except simple Copy types (i32, u32, usize, bool, f32, f64). Previously only Reference/MutableReference got ascription. Now owned types (DialogueNode, ItemStack, etc.) also get ascription, helping Rust infer method receiver types.

### 4. Some(ref mut var) / Some(ref var) explicit pattern

**File:** `windjammer/src/codegen/rust/type_analysis.rs`

Added case for `EnumPatternBinding::Tuple([RefMut(name)])` and `Tuple([Ref(name)])` when matching Option<T> - returns MutableReference/Reference.

## Error Categories Fixed

| Pattern | Count | Fix |
|---------|-------|-----|
| (*pos).clone() in struct constructor | 3 | infer_match_bound_types + turbofish |
| node.text(), node.choices(), etc. | 7 | Expanded type ascription |
| stack.item(), stack.clone() | 7 | Expanded type ascription |
| mesh.value() | 1 | Expanded type ascription |
| Match arm clone | 2 | Expanded type ascription |

## Verification

```bash
cd windjammer-game-core
wj build src_wj --no-cargo -o .
cargo build --release 2>&1 | rg "error\[E0282\]" | wc -l
# Result: 0
```

## TDD Tests Added

- `test_deref_clone_turbofish_when_type_known` - Verifies turbofish or type ascription for (*pos).clone()
- `test_match_arm_owned_type_ascription` - Verifies type ascription for match arm bindings

## Documentation

- `/tmp/e0282_final_analysis.md` - Full categorization
- `/tmp/e0282_unfixable_patterns.md` - Patterns requiring user annotation

## Philosophy

**"Compiler Does the Hard Work"** - Exploited all available context: struct fields, method returns, match scrutinees, ref/mut ref patterns. No shortcuts.
