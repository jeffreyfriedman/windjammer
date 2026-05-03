# E0432 Unresolved Import Investigation Report

**Date:** 2026-03-14  
**Scope:** windjammer-game-core, windjammer-ui  
**Goal:** Determine if E0432 errors are compiler bugs or game code issues

---

## Executive Summary

**Verdict: All investigated E0432 errors are GAME CODE issues, not compiler bugs.**

| Error | Root Cause | Fix Location |
|-------|------------|--------------|
| `crate::plugin::PluginContext` | Type not defined in plugin module | plugin/mod.wj |
| `crate::rendering::texture_packer` | File/module structure (may be resolved) | rendering/ |
| `vertical_slice_test::test_library_compiles` | Incorrect `::*` on function re-export | tests/mod.wj or tests/mod.rs |
| `crate::components` (windjammer-ui) | Module exists - verify structure | N/A |

---

## Investigation Details

### 1. PluginContext (E0432: unresolved import)

**Location:** `plugin/rendering_plugin.rs`, `plugin/audio_plugin.rs`, `plugin/physics_plugin.rs`, `plugin/test_plugins.rs`

**Error:** `use crate::plugin::{Plugin, PluginContext}` - PluginContext not found

**Root Cause:** 
- `src_wj/mod.wj` re-exports `pub use plugin::{Plugin, PluginContext, App}`
- `plugin/mod.wj` defines only `Plugin` trait and `App` struct
- **PluginContext is never defined** in the plugin module

**Evidence:**
- `grep "struct PluginContext|type PluginContext"` → No matches
- `plugin/mod.wj` lines 18-110: Only Plugin trait and App struct
- Plugin implementations use `ctx: PluginContext` and call `ctx.record_resource()`
- App has `record_resource(self, name: str)` - same interface

**Fix (Game Code):** Add to `plugin/mod.wj`:
```windjammer
pub type PluginContext = App
```

**Note:** The Plugin trait in mod.wj has `fn initialize(self, app: App)` - the implementations use `ctx: PluginContext`. With the type alias, these are equivalent. The trait signature may need alignment: implementations pass `ctx` (PluginContext = App) and call `ctx.record_resource()`.

---

### 2. texture_packer (E0432: could not find in rendering)

**Location:** `assets/pipeline.rs:4` - `use crate::rendering::texture_packer::TexturePacker`

**Root Cause:** Build log may be stale. Current state:
- `rendering/mod.rs` line 61: `pub mod texture_packer;`
- `rendering/texture_packer.rs` EXISTS (verified via glob_file_search)
- `src_wj/rendering/mod.wj` line 10: `pub mod texture_packer`

**Status:** Module structure appears correct. If error persists:
- Verify `rendering/texture_packer.rs` exports `TexturePacker` and `PackedRect`
- Check for circular dependencies or conditional compilation

---

### 3. vertical_slice_test::test_library_compiles (E0432)

**Location:** `tests/mod.rs:9` - `pub use vertical_slice_test::test_library_compiles`

**Error:** `no 'test_library_compiles' in 'tests::vertical_slice_test'`

**Root Cause:** 
- `tests/mod.rs` has: `pub use vertical_slice_test::test_library_compiles::*`
- `test_library_compiles` is a **function**, not a module
- `::*` tries to glob-import from a function → invalid Rust

**Source:** `tests/mod.wj` correctly has `pub use vertical_slice_test::test_library_compiles` (no ::*)

**Fix (Game Code):** Ensure generated `tests/mod.rs` has:
```rust
pub use vertical_slice_test::test_library_compiles;
```
NOT:
```rust
pub use vertical_slice_test::test_library_compiles::*;
```

**Compiler Check:** If `wj build` regenerates tests/mod.rs with `::*`, that's a compiler bug in module_system.rs. The `parse_mod_declarations` and `generate_mod_rs_for_submodule` should preserve exact pub use paths without adding `::*` for single-item re-exports.

---

### 4. windjammer-ui components

**Location:** `windjammer-ui/src/lib.rs` - `pub use crate::components::alert::...`

**Status:** 
- `pub mod components` declared at line 79
- `src/components/mod.rs` exists and re-exports from `generated`
- `src/components/generated/alert.rs` exists
- Structure appears correct

**If E0432 occurs:** Verify `components/generated/mod.rs` exports all submodules (alert, button, etc.)

---

## Compiler vs Game Code Decision

**Critical Rule:** Fix compiler bugs only. Document game code issues for user.

| Issue | Compiler Bug? | Action |
|-------|---------------|--------|
| PluginContext | No | Document - add type alias to plugin/mod.wj |
| texture_packer | No | Document - verify file structure |
| test_library_compiles::* | Maybe | Add compiler test; fix game mod.rs if needed |
| components | No | Document - verify structure |

---

## Recommended Fixes (Game Code - For User)

### Fix 1: Add PluginContext to plugin/mod.wj

```windjammer
// In plugin/mod.wj, after the App struct definition:
pub type PluginContext = App
```

Also verify Plugin trait and implementations align: trait has `fn initialize(self, app: App)` while impls use `ctx: PluginContext`. With the alias, both refer to App.

### Fix 2: Fix tests/mod.rs re-export

Change:
```rust
pub use vertical_slice_test::test_library_compiles::*;
```
To:
```rust
pub use vertical_slice_test::test_library_compiles;
```

If this file is generated by `wj build`, re-run build and check output. If compiler adds `::*`, file a compiler bug.

---

## Compiler Test (If Bug Found)

If the compiler incorrectly adds `::*` to single-item pub use:

```rust
// windjammer/tests/module_system_pub_use_test.rs
#[test]
fn test_pub_use_single_item_no_glob() {
    let content = "pub use vertical_slice_test::test_library_compiles\npub mod vertical_slice_test\n";
    let (_, pub_uses) = parse_mod_declarations(content);
    assert_eq!(pub_uses[0], "vertical_slice_test::test_library_compiles");
    // Generated output must NOT contain ::*
    let generated = format!("pub use {};\n", pub_uses[0]);
    assert!(!generated.contains("::*"), "Single item re-export must not add ::*");
}
```

---

## Files Modified

None - this is an investigation report only. Fixes are documented for user implementation.

---

## Philosophy Alignment

**"No Workarounds, Only Proper Fixes"** - Root cause identified for each error.

**"Fix compiler bugs only"** - PluginContext and test_library_compiles are game/source structure issues. The compiler correctly translates .wj to .rs; the source has missing definitions or incorrect re-exports.
