# Dogfooding Win #34: Import Path Generation Bugs

**Date**: 2025-11-30  
**Status**: 🚧 **IDENTIFIED** (Not yet fixed)

---

## 🎯 **THE PROBLEM**

When building `game-host` platformer, discovered that **build.rs regenerates files**, undoing all manual fixes!

**Root Cause**: Compiler generates **incorrect import paths** in Rust code.

---

## 🔍 **BUG #1: Module-Qualified Type Imports**

### **The Bug**

When a Windjammer struct references a type from another module, the compiler generates wrong import paths.

**Example** (sprite.wj):
```windjammer
use rendering::Texture
use rendering::TextureAtlas
use rendering::SpriteRegion
```

**Generated Rust (WRONG)**:
```rust
use super::texture::Texture;            // ✅ Correct
use super::texture_atlas::TextureAtlas; // ❌ texture_atlas is NOT a module!
use super::sprite_region::SpriteRegion; // ❌ sprite_region is NOT a module!
```

**Should Generate (CORRECT)**:
```rust
use super::texture::{Texture, TextureAtlas, SpriteRegion};  // ✅ All in texture module!
```

### **Why This Happens**

The compiler sees `TextureAtlas` as a type name and assumes:
```
TextureAtlas → texture_atlas module
```

But actually, `TextureAtlas` is a **struct defined IN the texture module**, not a separate module!

**Correct Mapping**:
```
rendering::Texture       → super::texture::Texture        ✅
rendering::TextureAtlas  → super::texture::TextureAtlas   ✅
rendering::SpriteRegion  → super::texture::SpriteRegion   ✅
```

---

## 🔍 **BUG #2: Module-Qualified Type References**

### **The Bug**

When a Windjammer function parameter uses a module-qualified type, the compiler generates wrong paths.

**Example** (sprite.wj):
```windjammer
pub fn render(ctx: rendering::RenderContext) {
    // ...
}
```

**Generated Rust (WRONG)**:
```rust
pub fn render(&self, ctx: &rendering::RenderContext) {  // ❌ rendering doesn't exist!
```

**Should Generate (CORRECT)**:
```rust
pub fn render(&self, ctx: &super::render_context::RenderContext) {  // ✅ Correct path!
```

### **Why This Happens**

The compiler sees `rendering::RenderContext` and generates:
```
rendering::RenderContext → rendering::RenderContext
```

But `rendering` is not a crate or module in the generated code! It's a **logical grouping** in Windjammer that maps to multiple Rust modules.

**Correct Mapping**:
```
rendering::RenderContext → super::render_context::RenderContext  ✅
physics::RigidBody2D     → super::rigidbody2d::RigidBody2D       ✅
world::Tilemap           → super::tilemap::Tilemap               ✅
```

---

## 📊 **IMPACT**

### **Errors Caused**
- `sprite.rs`: 2 import errors
- All files with module-qualified types: Similar errors
- **Total**: ~20-30 import-related errors

### **Build.rs Regeneration**
- Every `cargo build` triggers `build.rs`
- `build.rs` runs `wj build` on all `.wj` files
- Generated `.rs` files overwrite manual fixes
- **Manual fixes are futile!** Must fix compiler!

---

## 🎯 **THE FIX** (TDD Approach)

### **Step 1: Create Failing Tests**

Test file: `module_qualified_types_test.wj`

```windjammer
// Define types in one module
pub mod math {
    pub struct Vec2 {
        pub x: f32,
        pub y: f32,
    }
    
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }
}

// Use types from that module in another struct
pub struct Sprite {
    pub position: math::Vec2,  // ← Test this!
    pub velocity: math::Vec2,
}

impl Sprite {
    pub fn new(pos: math::Vec2) -> Sprite {
        Sprite {
            position: pos,
            velocity: math::Vec2 { x: 0.0, y: 0.0 },
        }
    }
    
    // Method with module-qualified parameter
    pub fn set_pos(pos: math::Vec2) {  // ← Test this!
        self.position = pos
    }
}
```

**Expected Generated Rust**:
```rust
use super::math::{Vec2, Vec3};  // ✅ Correct import!

pub struct Sprite {
    pub position: Vec2,  // ✅ Not math::Vec2
    pub velocity: Vec2,
}

impl Sprite {
    pub fn set_pos(mut self, pos: Vec2) -> Sprite {  // ✅ Clean type!
        self.position = pos;
        self
    }
}
```

---

### **Step 2: Fix Compiler** (2 parts)

#### **Fix Part A: Import Generation** (`generator.rs`)

Find where `use` statements are generated for types.

**Current Logic** (WRONG):
```rust
// Generates: use super::texture_atlas::TextureAtlas
module_name = type_name.to_snake_case();  // TextureAtlas → texture_atlas
```

**New Logic** (CORRECT):
```rust
// Look up which module this type is ACTUALLY defined in
// Example: TextureAtlas is defined in texture.wj
// So generate: use super::texture::TextureAtlas

// Requires: Type Registry
// Maps: TypeName → ModuleName
// Example: TextureAtlas → texture
```

#### **Fix Part B: Type Reference Generation** (`type_to_rust()`)

Find where module-qualified types are converted to Rust.

**Current Logic** (WRONG):
```rust
// rendering::RenderContext → rendering::RenderContext
format!("{}::{}", module, type_name)
```

**New Logic** (CORRECT):
```rust
// rendering::RenderContext → super::render_context::RenderContext
// Requires: Module Mapping
// Maps: Windjammer logical module → Rust physical module
// Example: rendering → [texture, sprite, render_context, render_api]

// For a type like rendering::RenderContext:
// 1. Look up which Rust module has RenderContext: render_context
// 2. Generate: super::render_context::RenderContext
```

---

### **Step 3: Verify** (Run Tests)

1. Run `module_qualified_types_test.wj` - should PASS
2. Run full compiler test suite - should be 207 passing (NO regressions!)
3. Build windjammer-ui - should be 0 errors, 0 warnings
4. Build game-host - should be 0 errors, ~10 warnings
5. **BUILD PLATFORMER!** 🎮

---

## 🏗️ **IMPLEMENTATION PLAN**

### **Phase 1: Type Registry** (20 min)

Create a mapping of Type → Module during compilation:

```rust
// In ModuleCompiler or Generator
struct TypeRegistry {
    type_to_module: HashMap<String, String>,
}

// During module compilation:
// When we see: pub struct TextureAtlas in texture.wj
// Register: TextureAtlas → texture

impl TypeRegistry {
    fn register_type(&mut self, type_name: &str, module_name: &str) {
        self.type_to_module.insert(type_name.to_string(), module_name.to_string());
    }
    
    fn get_module_for_type(&self, type_name: &str) -> Option<&String> {
        self.type_to_module.get(type_name)
    }
}
```

### **Phase 2: Fix Import Generation** (15 min)

Update `generate_use_statement()` or similar:

```rust
fn generate_import_for_type(&self, module_qualified: &str) -> String {
    // Parse: rendering::TextureAtlas
    let parts: Vec<&str> = module_qualified.split("::").collect();
    
    if parts.len() == 2 {
        let (logical_module, type_name) = (parts[0], parts[1]);
        
        // Look up actual Rust module
        if let Some(rust_module) = self.type_registry.get_module_for_type(type_name) {
            return format!("use super::{}::{};", rust_module, type_name);
        }
    }
    
    // Fallback to current behavior
    // ...
}
```

### **Phase 3: Fix Type Reference Generation** (15 min)

Update `type_to_rust()` to handle module-qualified types:

```rust
fn type_to_rust(&self, ty: &Type) -> String {
    match ty {
        Type::ModuleQualified { module, type_name } => {
            // Don't generate rendering::RenderContext
            // Instead, look up actual module and generate super::module::Type
            
            if let Some(rust_module) = self.type_registry.get_module_for_type(type_name) {
                return format!("super::{}::{}", rust_module, type_name);
            }
            
            // Fallback
            format!("{}::{}", module, type_name)
        }
        // ... other cases
    }
}
```

---

## 📝 **ESTIMATED TIME**

| Task | Time | Status |
|------|------|--------|
| Create Type Registry | 20 min | Pending |
| Fix Import Generation | 15 min | Pending |
| Fix Type References | 15 min | Pending |
| Write Tests | 10 min | Pending |
| Run Full Test Suite | 5 min | Pending |
| Build Both Projects | 5 min | Pending |
| **TOTAL** | **70 min** | **~1 hour!** |

---

## 🚀 **AFTER THIS FIX**

- ✅ windjammer-ui: 0 errors, 0 warnings (ALREADY WORKING!)
- ✅ game-host: 0 errors, 0 warnings (WILL WORK!)
- ✅ build.rs regeneration: NO PROBLEM (generates correct code!)
- ✅ Compiler tests: 207+ passing (adding new tests!)
- ✅ **PLATFORMER RUNS!** 🎮

---

## 💡 **KEY INSIGHT**

**Manual fixes are futile when build.rs regenerates!**

Must fix the ROOT CAUSE (compiler import generation logic), not the SYMPTOM (generated file errors).

This is the Windjammer Philosophy in action:
> **"If it's worth doing, it's worth doing right."**

No workarounds. Fix the compiler. Then everything works forever!

---

## 🎯 **DECISION POINT**

Given that we've:
- ✅ Fixed 2 major compiler bugs (implicit self, parentheses)
- ✅ 206 tests passing (zero regressions)
- ✅ windjammer-ui FULLY WORKING (367 → 0 errors!)
- ✅ Validated TDD + Dogfooding process

**Options**:

### **Option A**: Fix Import Path Bugs NOW (~1 hour)
- Complete compiler fix
- Both projects compile cleanly
- **PLATFORMER RUNS!** 🎮

### **Option B**: Take a Break, Resume Next Session
- Excellent stopping point
- 2 major bugs fixed this session
- windjammer-ui working perfectly
- Clear plan for next session

**Recommendation**: Given the momentum and clear plan, **Option A** if energy permits!

---

**This is the final boss! One more hour and we WIN!** 💪

