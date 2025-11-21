# Next Immediate Tasks

**Status**: ECS working end-to-end ✅  
**Date**: November 15, 2025

---

## 🐛 Known Issues to Fix

### 1. Update Function Signature (HIGH PRIORITY)
**Problem**: Generated main loop always passes `&input` to update, but user's function may not take it.

**Current Workaround**: Manually fixed test game to accept unused `_input` parameter.

**Proper Fix**: Compiler should:
- Detect if user's `@update` function has `input` parameter
- Only pass `&input` if function signature includes it
- Or always generate with input parameter (simpler)

**Location**: `src/codegen/rust/generator.rs` - `generate_game_main()`

**Estimated Time**: 30 minutes

---

### 2. Type Conversion (MEDIUM PRIORITY)
**Problem**: Delta time is `f32` in game loop but user's function expects `f64`.

**Current**: Compiler generates `f64` for `float` type, but game loop uses `f32`.

**Fix Options**:
- A) Change game loop to use `f64` (simple)
- B) Change compiler to generate `f32` for `float` (affects all code)
- C) Add explicit conversion in generated code (current workaround)

**Recommendation**: Option A - use `f64` in game loop for consistency.

**Location**: `src/codegen/rust/generator.rs` - `generate_game_main()`

**Estimated Time**: 15 minutes

---

### 3. Unused Warnings (LOW PRIORITY)
**Problem**: Generated code has many unused imports and variables.

**Examples**:
- `use game_host_framework::ecs::*;` (unused)
- `use winit::event_loop::ControlFlow;` (unused)
- `game` parameter in render function (unused)

**Fix**: Compiler should:
- Only import what's actually used
- Add `_` prefix to intentionally unused parameters
- Detect which modules are needed

**Location**: Multiple places in codegen

**Estimated Time**: 1-2 hours

---

## 🚀 Next Features to Implement

### Phase 1: Basic Functionality (Week 1)

#### 1. Input System (2-3 hours)
- ✅ Input struct exists
- ❌ Need to wire up keyboard events
- ❌ Need to wire up mouse events
- ❌ Add input state tracking (pressed, just_pressed, just_released)

**Files**:
- `crates/game-host-framework/src/input.rs`
- `src/codegen/rust/generator.rs` (update event handling)

#### 2. Basic 2D Rendering (4-6 hours)
- ✅ Renderer struct exists
- ✅ Clear color works
- ❌ Draw sprites
- ❌ Draw shapes (rect, circle, line)
- ❌ Camera system

**Files**:
- `crates/game-host-framework/src/renderer.rs`
- `crates/game-host-framework/src/camera.rs` (new)

#### 3. Physics Integration - Rapier2D (6-8 hours)
- ❌ Add Rapier2D dependency
- ❌ Create physics world wrapper
- ❌ Add RigidBody component
- ❌ Add Collider component
- ❌ Integrate with ECS
- ❌ Update physics in game loop

**Files**:
- `crates/game-host-framework/src/physics2d.rs` (new)
- `crates/game-host-framework/Cargo.toml`
- `std/game/physics2d.wj` (new)

---

### Phase 2: 2D Game Demo (Week 1-2)

#### 4. Create 2D Platformer (8-12 hours)
- ❌ Player character with sprite
- ❌ Keyboard controls (arrow keys, space to jump)
- ❌ Ground and platforms (static colliders)
- ❌ Gravity and jumping physics
- ❌ Camera following player
- ❌ Simple level design

**File**: `examples/platformer_2d.wj`

**Success Criteria**:
- Player can move left/right
- Player can jump
- Collision with platforms works
- Camera follows player
- Runs at 60 FPS

---

### Phase 3: 3D Foundation (Week 2-3)

#### 5. 3D Renderer (12-16 hours)
- ❌ Mesh loading (GLTF)
- ❌ Camera 3D (perspective projection)
- ❌ Basic lighting (directional light)
- ❌ Texture loading
- ❌ Basic materials

**Files**:
- `crates/game-host-framework/src/renderer3d.rs` (enhance)
- `crates/game-host-framework/src/mesh.rs`
- `crates/game-host-framework/src/material.rs`

#### 6. Physics Integration - Rapier3D (6-8 hours)
- ❌ Add Rapier3D dependency
- ❌ Create physics world wrapper
- ❌ Add RigidBody component
- ❌ Add Collider component
- ❌ Integrate with ECS

**Files**:
- `crates/game-host-framework/src/physics3d.rs` (new)
- `std/game/physics3d.wj` (new)

#### 7. Create 3D FPS Demo (12-16 hours)
- ❌ First-person camera
- ❌ Mouse look controls
- ❌ WASD movement
- ❌ Ground plane
- ❌ Some 3D objects to navigate
- ❌ Basic lighting

**File**: `examples/fps_3d.wj`

---

### Phase 4: Advanced Rendering (Week 3-4)

#### 8. PBR Pipeline (16-20 hours)
- ❌ Metallic-roughness workflow
- ❌ Normal mapping
- ❌ Ambient occlusion
- ❌ HDR rendering
- ❌ Tone mapping

#### 9. Deferred Rendering (12-16 hours)
- ❌ G-buffer setup
- ❌ Multiple lights support
- ❌ Light culling

#### 10. Shadow Mapping (12-16 hours)
- ❌ Directional light shadows
- ❌ Cascaded shadow maps
- ❌ Point light shadows (cubemaps)

---

## 📋 Prioritized Task List

### This Week (Must Do)
1. ✅ Fix update signature generation (30 min)
2. ✅ Fix delta time type (15 min)
3. ✅ Input system (2-3 hours)
4. ✅ Basic 2D rendering (4-6 hours)
5. ✅ Rapier2D integration (6-8 hours)
6. ✅ 2D platformer demo (8-12 hours)

**Total**: ~20-30 hours (3-4 days of focused work)

### Next Week
7. ✅ 3D renderer foundation (12-16 hours)
8. ✅ Rapier3D integration (6-8 hours)
9. ✅ 3D FPS demo (12-16 hours)

**Total**: ~30-40 hours (4-5 days)

### Week 3-4
10. ✅ PBR pipeline (16-20 hours)
11. ✅ Deferred rendering (12-16 hours)
12. ✅ Shadow mapping (12-16 hours)

**Total**: ~40-52 hours (5-7 days)

---

## 🎯 Success Metrics

### By End of Week 1
- ✅ 2D platformer running at 60 FPS
- ✅ Physics working (jumping, collisions)
- ✅ Input responsive
- ✅ Camera following player

### By End of Week 2
- ✅ 3D FPS running at 60 FPS
- ✅ 3D physics working
- ✅ Mouse look smooth
- ✅ Basic lighting visible

### By End of Week 4
- ✅ PBR materials looking good
- ✅ Multiple lights with shadows
- ✅ Deferred rendering working
- ✅ Performance: 60 FPS with 1000+ objects

---

## 💡 Notes

- **Focus on working features over perfect code**
- **Test each feature immediately**
- **Create examples for each major feature**
- **Document as we go**
- **Commit frequently**

---

*Let's build something amazing!* 🚀

