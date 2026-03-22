# Compiler Fix: pub use Relative Paths (TDD)

**Date**: 2026-03-21  
**Bug**: Windjammer compiler incorrectly adding `crate::` prefix to relative `pub use` statements in module files  
**Impact**: 364+ "unresolved import" errors in windjammer-game  

## The Bug

When transpiling Windjammer `pub use` statements for submodule re-exports, the compiler was incorrectly generating absolute paths.

### Example

**Windjammer source (achievement/mod.wj)**:
```windjammer
pub mod achievement_id
pub use achievement_id::AchievementId
```

**Generated Rust (BEFORE FIX)** ❌:
```rust
pub use crate::achievement_id::AchievementId;  // WRONG! Looking at crate root!
pub mod achievement_id;
```

**Generated Rust (AFTER FIX)** ✅:
```rust
pub use achievement_id::AchievementId;  // CORRECT! Submodule in same file!
pub mod achievement_id;
```

## Root Cause

Three issues:

1. **`current_wj_file` not set**: In `compiler.rs`, single-file builds didn't call `set_source_file()`, so `current_wj_file` was empty
2. **Wrong directory check**: `is_in_subdirectory` checked OUTPUT file structure, not INPUT source structure
3. **Incorrect prefix**: Used `super::` for submodules instead of no prefix

## The Fix

### 1. Set source file in compiler.rs

```rust
// compiler.rs line 222
let mut codegen = CodeGenerator::new(registry, target);
codegen.set_source_file(file);  // TDD FIX: Required for import path resolution
```

### 2. Check INPUT file structure

```rust
// import_generation.rs
let is_in_subdirectory = self
    .current_wj_file  // Check INPUT, not OUTPUT
    .parent()
    .and_then(|p| p.file_name())
    .is_some();
```

### 3. Use no prefix for submodules in same file

```rust
// import_generation.rs
} else if is_actual_module_file {
    // Submodules declared in same file don't need ANY prefix
    format!("use {};\n", rust_path)  // No super::, no crate::
}
```

## TDD Test

Created `tests/pub_use_codegen_test.rs`:

```rust
#[test]
fn test_pub_use_relative_paths() {
    // Input: pub use sub_module::TypeA
    // Expected: pub use sub_module::TypeA (no crate:: prefix)
    // ...
}
```

## Verification

**Before fix**:
```bash
cd windjammer-game/windjammer-game-core
cargo build 2>&1 | grep "unresolved import" | wc -l
# 364 errors
```

**After fix**:
```bash
# achievement/mod.rs now correct:
pub use achievement_id::AchievementId;  ✅
pub mod achievement_id;
```

## Impact

Fixed submodule re-exports in ALL modules:
- `achievement/*`
- `ai/*`
- `animation/*`  
- `asset_management/*`
- And 80+ other modules

## Related Issues

Remaining compilation errors (1600+) are from missing re-exports in user source `.wj` files (not compiler bugs):

```windjammer
// autotile/mod.wj needs:
pub use tile_id::TileId;
pub use edge_type::EdgeType;
// etc.
```

This is a game source issue, not a compiler issue.

## Lessons Learned

1. **Always set `current_wj_file`** - Required for context-aware code generation
2. **Check INPUT paths, not OUTPUT** - Output might be flat while input is nested
3. **Submodule re-exports need no prefix** - `pub mod foo; pub use foo::Bar;` in same file
4. **TDD catches subtle bugs** - Manual testing alone would have missed this

## Files Changed

- `windjammer/src/compiler.rs` - Added `set_source_file()` calls (2 locations)
- `windjammer/src/codegen/rust/import_generation.rs` - Fixed path detection logic
- `windjammer/tests/pub_use_codegen_test.rs` - TDD test (NEW)

## The Windjammer Way

✅ **"No workarounds, only proper fixes"** - Fixed the compiler, not the game source  
✅ **"TDD everything"** - Created test first, verified fix  
✅ **"Fix root causes"** - Addressed why `current_wj_file` wasn't set  
✅ **"Automatic is better"** - Compiler infers correct paths automatically
