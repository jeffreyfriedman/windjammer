# SceneBuilder and Visual Verification Integration Summary

## Overview

Integrated SceneBuilder and Visual Verification with TDD across the Breach Protocol game and Windjammer engine. This document summarizes the changes, code simplification, and test coverage.

---

## Part 1: Refactor Test Scene with SceneBuilder

### Before (50+ lines, manual setup)

```windjammer
pub fn create_simple_test_scene() -> VoxelGrid {
    let mut grid = VoxelGrid::new(64, 64, 64)
    // ... 50+ lines of manual voxel loops ...
    // Manual camera: create_test_camera() with guessed coordinates
    // Manual materials: VoxelMaterial::diffuse() in game.initialize()
    grid
}
```

### After (declarative, validated)

```windjammer
pub fn create_simple_test_scene() -> BuiltScene {
    let mut grid = VoxelGrid::new(64, 64, 64)
    // ... voxel grid creation (same logic) ...
    
    let result = SceneBuilder::new()
        .add_voxel_grid(grid)
        .with_material(1, Color::green())
        .with_material(2, Color::red())
        .with_material(3, Color::blue())
        .add_camera_auto_frame()
        .with_default_lighting()
        .build()

    match result {
        BuildResult::Success(scene) => scene,
        BuildResult::Failure(msg) => panic(format!("Scene build failed: {}", msg)),
    }
}
```

### Key Improvements

- **Camera auto-framing**: No more manual coordinate guessing; `add_camera_auto_frame()` positions camera from scene bounds
- **Materials**: Declarative `with_material(id, Color::green())` instead of verbose `VoxelMaterial::diffuse()`
- **Validation**: SceneBuilder validates camera and voxel grid presence before build
- **Lighting**: `with_default_lighting()` adds directional + ambient

### SceneBuilder Extensions

- `BuiltScene::primary_voxel_grid()` - Get primary voxel grid from scene
- `BuiltScene::unpack()` - Extract (VoxelGrid, Camera, MaterialPalette, Option<LightingData>) for level loader
- `Camera::to_camera_data(screen_width, screen_height)` - Convert to RenderPort CameraData

### Level Loader Integration

- `load_test_scene()` now calls `create_simple_test_scene()`, unpacks scene, stores voxel_grid + camera
- `get_test_camera()` returns CameraData from SceneBuilder auto-frame

### Tests (9 in breach-protocol, 12 in windjammer-game-core)

- `test_simple_scene_has_ground`, `test_simple_scene_has_building`, `test_simple_scene_has_blue_marker`
- `test_simple_scene_ground_plane_full`, `test_simple_scene_building_hollow_interior`
- `test_scene_has_camera_from_builder`, `test_scene_has_lighting`, `test_scene_has_materials`
- `test_scene_camera_to_camera_data`
- SceneBuilder: `test_scene_validation_catches_missing_camera`, etc.

---

## Part 2: Visual Verification in Rendering Pipeline

### VoxelGPURenderer::render_frame_with_verification()

```windjammer
pub fn render_frame_with_verification(self) {
    self.render_frame()
    if crate::ffi::api::gpu_is_visual_verification_enabled() != 0 {
        let buffer = self.get_output_buffer()
        if buffer.len() > 0 {
            let img = VerificationImage::from_rgba8(
                self.screen_width, self.screen_height, buffer,
            )
            let verifier = VisualVerifier::new(img)
            let report = verifier.generate_report(0.01, 5000.0)
            if !report.is_valid() {
                panic(format!("Visual verification failed: {}", report.summary()))
            }
        }
    }
}
```

### VerificationReport Additions

- `is_valid()` - Returns true when quadrant_coverage && !has_stripes && pixel_range_valid && !is_solid_color
- `summary()` - Human-readable string for error messages

### VerificationImage::from_rgba8()

- Creates VerificationImage from RGBA8 frame buffer bytes (for GPU readback)

### FFI: gpu_is_visual_verification_enabled()

- Returns 1 when `cfg!(test)` or `VISUAL_VERIFICATION=1` env var
- Implemented in windjammer-runtime-host

### Tests (4 new in visual_verification_test.wj)

- `test_report_is_valid_passes_for_good_rendering`
- `test_report_is_valid_fails_for_solid_color`
- `test_report_summary_produces_readable_string`
- `test_from_rgba8_converts_bytes_to_pixels`

---

## Part 3: Guardrails in Game Initialization

### Game.initialize() Updates

```windjammer
pub fn initialize(self) {
    println("[GAME] Initializing with guardrails...")
    
    // 1. Validate resolution config (ranges 1-8192)
    let config = RenderConfig::new(1280, 720)
    match config.validate_ranges() {
        Ok(()) => {},
        Err(msg) => panic(format!("Invalid render config: {}", msg)),
    }
    
    // 2. Load test scene (uses SceneBuilder)
    self.level_loader.load_level("test_scene")
    // ... rest of init ...
    
    println("[GAME] Initialization complete with all validations passing!")
}
```

**Note**: Full surface validation (`config.validate(surface_w, surface_h)`) runs inside `VoxelGPURenderer::initialize()` when GPU is ready.

---

## Part 4: CI Integration

### Updated `.github/workflows/rendering-tests.yml`

- **Triggers**: windjammer-game/**, breach-protocol/**
- **Steps**:
  1. Run SceneBuilder tests
  2. Run Visual Verification tests
  3. Run rendering regression tests (shader, buffer, pipeline)
  4. Run SOLID_RED diagnostic test
  5. Run Guardrail tests (resolution, workgroup, buffer validators)
  6. Save screenshots on failure (`/tmp/breach_protocol_frame_*.png`)

---

## File Changes Summary

| File | Changes |
|------|---------|
| `breach-protocol/src/world/test_scene.wj` | Refactored to use SceneBuilder, returns BuiltScene |
| `breach-protocol/src/world/level_loader.wj` | Unpacks BuiltScene, stores test_camera |
| `breach-protocol/src/world/test_scene_test.wj` | New tests for SceneBuilder integration |
| `breach-protocol/src/game.wj` | Uses level_loader.get_test_camera(), config validation |
| `windjammer-game-core/src_wj/scene/builder.wj` | Added primary_voxel_grid, get_lighting, get_materials, unpack |
| `windjammer-game-core/src_wj/rendering/camera.wj` | Added to_camera_data(screen_width, screen_height) |
| `windjammer-game-core/src_wj/rendering/visual_verification.wj` | Added is_valid(), summary(), from_rgba8() |
| `windjammer-game-core/src_wj/rendering/voxel_gpu_renderer.wj` | Added render_frame_with_verification() |
| `windjammer-game-core/src_wj/ffi/api.wj` | Added gpu_is_visual_verification_enabled() |
| `windjammer-game-core/ffi/api.rs` | Added gpu_is_visual_verification_enabled declaration |
| `windjammer-runtime-host/src/gpu_compute.rs` | Implemented gpu_is_visual_verification_enabled |
| `windjammer-game-core/tests_wj/visual_verification_test.wj` | Added 4 new tests |
| `windjammer-game/.github/workflows/rendering-tests.yml` | Added SceneBuilder, VisualVerification, Guardrail steps |

---

## Usage

### Enable Visual Verification

```bash
# In tests (automatically enabled when cfg!(test))
cargo test

# In production (manual)
VISUAL_VERIFICATION=1 ./breach-protocol-host
```

### Use render_frame_with_verification

```windjammer
// In game render loop - use when verification is needed
self.renderer.render_frame_with_verification()
```

---

## Test Count Summary

- **Part 1 (SceneBuilder)**: 9 breach-protocol + 12 windjammer-game-core = 21 tests
- **Part 2 (Visual Verification)**: 4 new + 16 existing = 20 tests
- **Part 3 (Guardrails)**: Integrated into existing game_init_test flow
- **Part 4 (CI)**: All rendering tests run on push/PR

---

## Before/After Code Comparison

```windjammer
// BEFORE: Manual
self.voxel_grid = create_simple_test_scene()  // Returns VoxelGrid
// Manual camera: create_test_camera() with guessed (50, 20, 50)
// Manual materials in game.initialize(): palette.set(1, VoxelMaterial::diffuse(...))

// AFTER: Declarative
let scene = create_simple_test_scene()  // Returns BuiltScene
let (voxel_grid, camera, materials, lighting) = scene.unpack()
self.voxel_grid = voxel_grid
self.test_camera = Some(camera.to_camera_data(1280.0, 720.0))
// Materials and lighting from scene
```

---

## Next Steps

1. **Implement get_output_buffer()**: Currently returns `Vec::new()`. Add GPU readback to enable verification in render loop.
2. **Fix pre-existing build errors**: windjammer-game-core has Rust compile errors (gpu_safe.rs, resolution_validator.rs, etc.) that predate this integration.
3. **Optional**: Use `render_frame_with_verification()` in game render loop when `VISUAL_VERIFICATION=1`.
