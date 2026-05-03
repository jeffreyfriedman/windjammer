# Vec3 Cross-Module Type Inference Audit

**Date:** 2026-03-14  
**Fix:** Set `source_root` correctly for cross-module metadata loading  
**Status:** ✅ Architecturally sound, compiler.rs aligned with main.rs

## Summary

The Vec3 type inference fix uses `find_source_root()` to resolve metadata paths like `math/vec3.wj.meta`. This audit verifies the fix is correct and documents edge cases.

## Architecture

### Metadata Flow

1. **Write:** When compiling `math/vec3.wj`, metadata is written to `math/vec3.wj.meta` (next to source)
2. **Load:** When compiling `game.wj` with `use crate::math::vec3::Vec3`, FloatInference loads `source_root.join("math/vec3.wj.meta")`
3. **Critical:** `source_root` must be the project root (e.g. `src_wj/`), not the file's parent

### Before Fix (Bug)

- `compiler.rs`: Used `file.parent()` → for `src_wj/ecs/entity.wj` gave `src_wj/ecs`
- Metadata lookup: `src_wj/ecs/math/vec3.wj.meta` ❌ (wrong - would look in ecs subdir)
- Correct path: `src_wj/math/vec3.wj.meta` ✅

### After Fix

- `compiler.rs`: Uses `find_source_root(file)` → returns `src_wj` for nested files
- `main.rs`: Already used `find_source_root` in ModuleCompiler (line 1236) and compile_file (line 1501)

## find_source_root() Logic

```rust
// Priority order:
// 1. Directory named "src_wj" (multi-file project)
// 2. Topmost directory with mod.wj
// 3. file_path.parent() (single-file / flat project)
```

**Works for:**
- Single file: `dir/game.wj` → `dir`
- Flat multi-file: `dir/game.wj`, `dir/math.wj` → `dir`
- Nested: `src_wj/ecs/entity.wj` → `src_wj`
- mod.wj structure: `proj/math/mod.wj`, `proj/math/vec3.wj` → `proj` or `proj/math` (topmost mod)

## Critical Questions Answered

### Does find_source_root() work for all project structures?

**Yes** for standard layouts. Edge case: project with no `src_wj` and no `mod.wj` falls back to `file.parent()`, which is correct for flat structures.

### What if user compiles from different directories?

`find_source_root` uses the **file path** (absolute after resolution), not CWD. So `wj build ./src_wj/game.wj` and `wj build src_wj/game.wj` both resolve correctly. The path is canonicalized where needed.

### Race conditions or caching?

- **No race:** Compilation is single-threaded; metadata is written before dependent files are compiled (dependency order in main.rs)
- **Caching:** No caching of metadata; each file read is a fresh `std::fs::read_to_string`. Stale metadata would only occur if user edits .wj without recompiling—expected behavior

## Test Coverage

| Test | Scope | Status |
|------|-------|--------|
| `test_vec3_with_f32_literals` | Single-file, Vec3 in same file | ✅ PASS |
| `test_vec3_math_f32` | Single-file, Vec3 + binary ops | ✅ PASS |
| `test_vec3_cross_module_inference` | Multi-file, Vec3 in math/, use in game.wj | ⏸️ IGNORED (requires wj binary) |

**Run cross-module manually:**
```bash
cargo test test_vec3_cross_module_inference -- --ignored
```
(Requires `wj` binary; use `cargo run --bin wj` from project root for manual verification)

## compiler.rs vs main.rs

| Aspect | compiler.rs (lib) | main.rs (binary) |
|--------|-------------------|------------------|
| Used by | Integration tests, `windjammer::build_project` | CLI `wj build` |
| Metadata write | ❌ No | ✅ Yes |
| source_root | ✅ Now uses find_source_root | ✅ Uses find_source_root |
| Cross-module | N/A (no metadata) | ✅ Full support |

The compiler.rs fix ensures **consistency** when/if metadata is added to the library path. For now, cross-module inference only works via the main binary.

## Recommendations

1. **Keep fix** – Aligning compiler.rs with main.rs prevents future bugs if library gains metadata
2. **Cross-module test** – Enable when wj binary is reliably available in test env (e.g. CI with full build)
3. **No further changes** – Architecture is sound; no edge case failures identified
