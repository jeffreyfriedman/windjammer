# Windjammer TODO List

**Last Updated:** 2025-12-04  
**Status:** Active Development - TDD + Dogfooding

---

## 🎉🎉🎉 MILESTONE: PLATFORMER IS PLAYABLE! 🎉🎉🎉

### Complete! First Playable Windjammer Game!
**Priority:** ✅ DONE  
**Status:** Complete!  
**Context:** Full stack working - compiler → game engine → rendering → input → playable game!

**What Works:**
- ✅ Platformer game compiles from Windjammer source
- ✅ Window opens (800x600 using winit)
- ✅ **wgpu 2D rendering!** (draw_rect, draw_circle)
- ✅ **Real keyboard input!** (WASD, Arrows, Space, Escape, etc.)
- ✅ Mouse input handling (position, clicks)
- ✅ Physics simulation (gravity, collision)
- ✅ **Player moves and jumps!**
- ✅ **Platforms render and collide!**

**To Play:**
```bash
cd windjammer-game/examples/build
cargo run --bin platformer_test
```
- WASD / Arrow keys: Move
- Space / W / Up: Jump
- Close window to exit

---

## 🎉 Previous Milestones

### windjammer-game-core BUILDS! (92→0 errors)
**Status:** ✅ Complete!  

**Major Wins:**
- ✅ Auto-generate mod.rs for multi-file projects
- ✅ Fix Copy type parameters in trait methods
- ✅ Added Copy derive to Color, Tile, TileType structs
- ✅ Remove std::ui and std::game from stdlib
- ✅ Fixed String literal conversion
- ✅ Fixed window types (int→u32)
- ✅ Fixed ownership/borrowing for all modules

### Trait Bound Inference (NEW!)
**Status:** ✅ Complete!  
**Context:** Automatically infers trait bounds for generic type parameters

**What Was Implemented:**
- ✅ Infers `Display` from `println!("{}", x)`
- ✅ Infers `Debug` from `println!("{:?}", x)`
- ✅ Infers `Clone` from `x.clone()`
- ✅ Infers `Add<Output = T>` from `x + x` (not just `Add`)
- ✅ Infers `Copy` when variable used twice in operators
- ✅ Infers `Sub`, `Mul`, `Div` with `Output = T` for same-type ops
- ✅ Automatically imports required traits (`std::fmt::Display`, `std::ops::Add`, etc.)

**Example:**
```windjammer
fn double<T>(x: T) -> T {
    x + x  // Infers T: Add<Output = T> + Copy
}
```

**Compiler Improvements:**
- Removed unnecessary `.to_string()` on string literals
- Better ownership inference for struct methods
- Proper `&str` handling in FFI calls
- Added `is_mutable` field to Parameter AST

**Test Status:**
- ✅ 206+ unit tests passing
- ✅ All integration tests passing
- ✅ Game engine builds cleanly

---

## 🔥 Next Priority: Enhance the Game!

### Text Rendering
**Priority:** HIGH  
**Status:** ✅ Complete!  
**Goal:** Display FPS, controls, status text in the platformer

**Implemented:**
- ✅ Bitmap font using 5x7 pixel characters
- ✅ `draw_text()` method on Renderer2D
- ✅ Supports A-Z, 0-9, punctuation
- ✅ Platformer displays FPS, controls, status!

### Sprite/Texture Rendering
**Priority:** MEDIUM  
**Status:** In Progress  
**Goal:** Load and render sprites for proper game graphics

**Implemented:**
- ✅ TextureManager for loading PNG/JPEG images
- ✅ Texture data accessible by handle
- ✅ Textured WGSL shader with UV coords
- ⏳ GPU texture upload (pending)
- ⏳ Renderer integration (pending)

**What we still need:
1. Texture loading (PNG support via `image` crate)
2. Implement `draw_sprite` with texture binding
3. Test with a sprite-based game

---

## 🛠️ Compiler Improvements (Ongoing)

### String → &str Automatic Borrow Inference
**Priority:** MEDIUM (partially done)  
**Status:** Basic implementation complete

**Current State:**
- ✅ Function calls no longer add unnecessary `.to_string()` to string literals
- ⏳ Struct field initialization still needs explicit `.to_string()` in some cases

### Source Maps with Relative Paths
**Priority:** MEDIUM  
**Status:** Pending  
**Fix Needed:**
- Use relative paths from workspace root
- Detect workspace root dynamically
- Test across different machines

---

## 🎯 Game Engine Goals

### MVP: Platformer Demo ✅ COMPLETE!
**Status:** DONE!  
**Steps:**
1. ✅ windjammer-game-core compiles (0 errors)
2. ✅ Platformer game compiles
3. ✅ Window opens with input handling
4. ✅ wgpu rendering works
5. ✅ **Platformer is playable!**

### Next Games
- Breakout game (dogfooding test)
- Angry Birds clone (2D physics test)
- 3D FPS demo (3D rendering test)

---

## 🧪 Language Features (Planned)

### Ownership & Inference
- Closure capture analysis (by value/ref/mut)
- Local variable ownership tracking
- Move semantics for local variables

### Syntax Sugar
- Compound assignments (+=, -=, *=, /=)
- Destructuring assignment ((x, y) = (1, 2))
- Pattern matching in let (let Some(x) = opt else)

### Type System
- Trait bound inference
- Associated types as generics
- Smarter @auto derive inference

---

## 🔧 Developer Tools

### Error Handling
- Map Rust errors to Windjammer source lines
- Better error messages (colorized, helpful)

### Performance
- Performance benchmarks vs Rust
- Profiler integration

### Documentation
- Rust-style doctests in comments
- Language guide
- API reference
- Beginner tutorials
- 10+ example games

### IDE Support
- Language Server Protocol implementation
- VS Code extension (autocomplete, hover, etc.)
- Syntax highlighting

---

## 📦 Package Management

### Windjammer Package Manager
- Package manager (wj add serde)
- Dependency resolution
- Version management

### Standard Library
- std/fs file system operations
- std/http client/server
- std/json parsing
- std/testing framework

---

## 🎮 3D Game Engine (Long-term)

### 3D Rendering
- 3D rendering pipeline
- 3D camera system (FPS/orbit/follow)
- 3D lighting (directional/point/spot)
- 3D shadow mapping
- PBR materials

### Animation
- Skeletal animation system
- IK system
- Procedural animation
- Blend spaces

### Physics
- 3D physics integration (rapier)
- Ragdoll system
- Cloth simulation

### Environment
- Terrain system
- Water rendering
- Weather system
- Foliage

---

## 📊 Progress Metrics

**Compiler Tests:** 206+ passing (100%) ✅  
**Game Engine Errors:** 0 (down from 92!) 🎉  
**Platformer:** PLAYABLE! 🎮✅  
**Features Working:**
- ✅ Window creation (winit)
- ✅ 2D rendering (wgpu)
- ✅ Keyboard input
- ✅ Mouse input
- ✅ Physics simulation
- ✅ Collision detection
- ⏳ Text rendering (stub)
- ⏳ Sprite rendering (stub)
- ⏳ Audio (stub)

---

## 💡 Philosophy Reminders

**"80% of Rust's power with 20% of Rust's complexity"**

- Compiler does the hard work, not the developer
- Infer what doesn't matter (ownership, mutability, simple types)
- Be explicit about what matters (algorithms, business logic)
- No workarounds, no tech debt, only proper fixes
- TDD for all compiler changes
- Dogfooding reveals real bugs

---

**Remember:** Every bug is an opportunity to make the compiler better. Every test is documentation. Every commit is progress. No shortcuts. No tech debt. Only proper fixes.

---

## 🎊 Celebration!

We now have a **complete, working game** written in Windjammer:
1. Source code in `.wj` files
2. Compiled to Rust by Windjammer compiler
3. Uses wgpu for GPU rendering
4. Real-time input handling
5. Physics and collision
6. **IT'S PLAYABLE!**

This proves the Windjammer vision is viable. 🚀
