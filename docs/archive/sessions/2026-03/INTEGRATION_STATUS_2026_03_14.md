# Integration Status Report — 2026-03-14

**Task:** Cleanup and integration check after blit shader fix deployment.

---

## 1. Naming Issues: "windjammer-app" References

### Status: ⚠️ PARTIAL — Many references remain

| Location | Count | Action Required |
|----------|-------|-----------------|
| **breach-protocol** | | |
| `breach-protocol/src/Cargo.toml` | 2 | Fix: `name = "windjammer-app"` → `breach-protocol`, dependency path |
| `breach-protocol/runtime_host/Cargo.toml` | 1 | Fix: `windjammer-app = { path = ... }` → `breach-protocol` |
| **windjammer-game** | | |
| `windjammer-game-core/tests_build/Cargo.toml` | 1 | Fix crate name |
| `windjammer-game-core/csg/Cargo.toml` | 1 | Fix crate name |
| `windjammer-game-core/test_output/Cargo.toml` | 1 | Fix crate name |
| `windjammer-game-core/examples/generated/Cargo.toml` | 1 | Fix crate name |
| `windjammer-game-editor/src/Cargo.toml` | 1 | Fix crate name |
| `windjammer-game-editor/src/generated/Cargo.toml` | 1 | Fix crate name |
| `build_voxel_test/Cargo.toml` | 1 | Fix crate name |
| `build_engine/Cargo.toml` | 1 | Fix crate name |
| `build_minimal/Cargo.toml` | 1 | Fix crate name |
| `build_temp/Cargo.toml` | 1 | Fix crate name |
| **windjammer compiler** | | |
| `windjammer/src/main.rs` | 10+ | Intentional: fallback crate name for game projects |
| `windjammer/src/codegen/rust/backend.rs` | 1 | Intentional: default Cargo.toml template |
| `windjammer-game/wj-game/src/main.rs` | 1 | Auto-fix: replaces `windjammer-app` → `breach-protocol` in output |
| **test_scene.wj** | 1 | Fix: `use windjammer_app::` → `use breach_protocol::` or crate alias |

### Acceptable (docs/comments/tests)
- `windjammer/src/codegen/rust/generator.rs`, `type_analysis.rs` — comments about external crates
- `windjammer/tests/*.rs` — test fixtures and documentation
- `windjammer/tests/test_framework_lib_name_test.rs` — explicitly tests "not windjammer-app"

### Summary
- **Critical:** breach-protocol Cargo.toml files must use `breach-protocol` for correct builds
- **Generated files:** wj-game plugin auto-fixes `name = "windjammer-app"` → `breach-protocol` during sync
- **Compiler:** Uses `windjammer-app` as fallback when project name cannot be inferred

---

## 2. Guardrails Integration Status

### 2.1 Resolution Validator ✅ ACTIVE

| Check | Status | Location |
|-------|--------|----------|
| Called in `VoxelGPURenderer::initialize()` | ✅ | `voxel_gpu_renderer.wj:152-153` |
| Implementation | ✅ | `config.validate(w, h)` → `validate_ranges()` + `validate_surface_match()` |

```windjammer
// voxel_gpu_renderer.wj init_gpu():
let config = RenderConfig::new(w, h)
config.validate(w, h)
```

### 2.2 Buffer Size Validator ✅ ACTIVE

| Check | Status | Location |
|-------|--------|----------|
| Called after buffer creation | ✅ | `voxel_gpu_renderer.wj:161-169` |
| Validates gbuffer, color_buffer, history_buffer, denoise_output, ldr_output | ✅ | All 5 render buffers validated |

```windjammer
buffer_validator::validate_buffer_size(self.resources.gbuffer, w, h, 48)
buffer_validator::validate_buffer_size(self.resources.color_buffer, w, h, 16)
// ... etc
```

### 2.3 Workgroup Validator ❌ NOT INTEGRATED

| Check | Status | Location |
|-------|--------|----------|
| `gpu_validate_dispatch` FFI exists | ✅ | `gpu_compute.rs:537`, `api.wj:100` |
| Called before `dispatch_compute` | ❌ | **Never called** |
| `shader_graph_executor.wj` | ❌ | Calls `dispatch_compute` directly, no `validate_dispatch` |

**Gap:** `ShaderGraph::execute_pass()` and `execute_with_dispatch()` call `gpu::dispatch_compute(groups_x, groups_y, groups_z)` without first calling `gpu::validate_dispatch(groups_x, groups_y, 8, 8, screen_width, screen_height)`.

**Fix required:** Add `validate_dispatch` call in `shader_graph_executor.wj` before each `dispatch_compute`, passing workgroup size (8×8) and expected resolution.

---

## 3. SceneBuilder Integration

### Status: ❌ NOT INTEGRATED in breach-protocol

| Component | Expected | Actual |
|-----------|----------|--------|
| **test_scene.wj** | Returns `BuiltScene` from `SceneBuilder::new().add_voxel_grid(...).build()` | Returns `VoxelGrid` from manual `create_simple_test_scene()` |
| **level_loader.wj** | `scene.unpack()` → (voxel_grid, camera, materials, lighting) | Calls `create_simple_test_scene()` expecting BuiltScene API |
| **test_scene_test.wj** | Tests `scene.primary_voxel_grid()`, `scene.camera_count()`, etc. | Expects BuiltScene API |

**Mismatch:** `test_scene.wj` was documented as refactored to SceneBuilder in `SCENEBUILDER_VISUAL_VERIFICATION_INTEGRATION.md`, but the implementation still uses:
- `create_simple_test_scene() -> VoxelGrid`
- `create_test_camera() -> CameraData`

**level_loader.wj** expects:
```windjammer
let scene = create_simple_test_scene()
let (voxel_grid, camera, _materials, _lighting) = scene.unpack()
```

**Actual test_scene.wj** has no `unpack()` — it returns `VoxelGrid` and has a separate `create_test_camera()`.

**SceneBuilder exists in windjammer-game-core** (`scene/builder.wj`, `scene/builder_test.wj`) with 12+ tests, but breach-protocol does not use it for test_scene.

---

## 4. Visual Verification Integration

### Status: ⚠️ PARTIAL — Available but not used by default

| Component | Status | Notes |
|-----------|--------|------|
| `VisualVerifier` | ✅ | `visual_verification.wj`, used in tests |
| `render_frame_with_verification()` | ✅ | `voxel_gpu_renderer.wj:403-420` |
| breach-protocol render loop | ❌ | Uses `render_frame()` not `render_frame_with_verification()` |
| Env var gate | ✅ | Runs when `VISUAL_VERIFICATION=1` |

**Current behavior:** `render_frame_with_verification()` calls `render_frame()` then, if `gpu_is_visual_verification_enabled()`, captures buffer and runs VisualVerifier. breach-protocol never calls it.

**Integration options:**
1. Switch breach-protocol to `render_frame_with_verification()` when `VISUAL_VERIFICATION=1`
2. Or keep as-is for optional manual testing

---

## 5. CI Configuration

### Status: ✅ CONFIGURED

**Location:** `windjammer-game/.github/workflows/rendering-tests.yml`

**Note:** No `rendering-tests.yml` at repo root (`.github/workflows/`). The workflow lives under `windjammer-game/`.

| Step | Present | Notes |
|------|---------|------|
| SceneBuilder tests | ✅ | `cargo test scene_builder` |
| Visual Verification tests | ✅ | `cargo test visual_verification` |
| Rendering regression tests | ✅ | shader_output_validation, buffer_format, rendering_pipeline, visual_output |
| SOLID_RED diagnostic | ✅ | `continue-on-error: true` |
| Guardrail tests | ✅ | resolution_validator, workgroup_validator, buffer_validator (`continue-on-error`) |
| Screenshot artifact on failure | ✅ | `/tmp/breach_protocol_frame_*.png` |

---

## 6. Integration Test Results

### Status: ❌ BUILD FAILURE — Tests cannot run

**windjammer-game-core:** Fails to compile with **2414+ errors** (e.g. octree.rs, vgs/lod_generator_test.rs — ownership/borrow issues).

**windjammer-runtime-host:** Depends on windjammer-game-core; build fails before tests run.

**Test counts:** N/A — no tests executed due to build failure.

---

## 7. Documentation Check

| Document | Exists | Location |
|----------|--------|----------|
| RENDERING_GUARDRAILS_DESIGN.md | ✅ | `/Users/jeffreyfriedman/src/wj/RENDERING_GUARDRAILS_DESIGN.md` |
| ARCHITECTURE_COMPARISON.md | ✅ | `windjammer-game-core/docs/ARCHITECTURE_COMPARISON.md` |
| ENGINE_COMPARISON.md | ✅ | `windjammer-game-core/docs/ENGINE_COMPARISON.md` |
| BEVY_SCENE_PATTERNS.md | ❌ | Not found |
| VISUAL_VERIFICATION_SYSTEM.md | ✅ | `windjammer-game-core/docs/VISUAL_VERIFICATION_SYSTEM.md` |

---

## 8. Summary: Success Criteria

| Criterion | Status |
|-----------|--------|
| No stale "windjammer-app" references | ❌ Many remain in breach-protocol, windjammer-game |
| Guardrails actively used | ⚠️ Resolution ✅, Buffer ✅, Workgroup ❌ |
| SceneBuilder used in test scene | ❌ test_scene uses manual VoxelGrid |
| Visual verification integrated | ⚠️ Implemented but not used in breach-protocol loop |
| CI configured correctly | ✅ |
| Integration tests passing | ❌ Build fails |
| Documentation complete | ⚠️ BEVY_SCENE_PATTERNS.md missing |

---

## 9. Next Steps for Full Integration

### P0 — Critical
1. **Fix windjammer-game-core build** — Resolve 2414+ compile errors (octree, vgs, etc.) so tests can run.
2. **Integrate workgroup validator** — Call `validate_dispatch` before each `dispatch_compute` in `shader_graph_executor.wj`.
3. **Fix breach-protocol naming** — Update `breach-protocol/src/Cargo.toml` and `runtime_host/Cargo.toml` to use `breach-protocol` crate name.

### P1 — SceneBuilder
4. **Refactor test_scene to SceneBuilder** — Implement `create_simple_test_scene() -> BuildResult` using `SceneBuilder::new().add_voxel_grid(...).add_camera_auto_frame().with_default_lighting().build()`.
5. **Align level_loader** — Ensure `load_test_scene()` works with the new BuiltScene API.

### P2 — Polish
6. **Optional visual verification** — Use `render_frame_with_verification()` in breach-protocol when `VISUAL_VERIFICATION=1`.
7. **Create BEVY_SCENE_PATTERNS.md** — Document SceneBuilder patterns and Bevy alignment.
8. **Bulk rename windjammer-app** — Update remaining Cargo.toml files in windjammer-game build/config crates.

---

*Report generated: 2026-03-14*
