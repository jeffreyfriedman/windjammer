# Windjammer Rendering API Architecture
# Public API Design for Implementation Flexibility

**Date**: November 2025  
**Goal**: Clean public API that hides implementation details, allowing easy backend swapping

---

## Core Principle: Public API vs. Implementation

**Philosophy**: "Users interact with pure Windjammer APIs, not wgpu/rendering internals"

### Current Architecture

```
┌─────────────────────────────────────────────────────────┐
│          Windjammer Public API (Pure Windjammer)        │
│   - Material (PBR properties, no wgpu types)            │
│   - Light (directional, point, spot)                    │
│   - Mesh (vertices, indices, no wgpu types)             │
│   - Texture (handle-based, no wgpu types)               │
│   - Camera (position, FOV, no wgpu types)               │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│          Internal Rendering Layer (Hidden)              │
│   - PipelinePBR (wgpu pipeline)                         │
│   - GraphicsBackend (wgpu device, queue)                │
│   - Vertex3D (wgpu vertex format)                       │
│   - Uniforms (wgpu buffer layout)                       │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│          Backend Implementation (Swappable)             │
│   - wgpu (current)                                      │
│   - Could be: OpenGL, Vulkan, Metal, DirectX, etc.     │
└─────────────────────────────────────────────────────────┘
```

---

## Public API Design

### ✅ Good: Clean Public API (What Users See)

```rust
// In pbr.rs - PUBLIC API
pub struct PBRMaterial {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: Vec3,
    pub emissive_strength: f32,
    pub normal_strength: f32,
    pub occlusion_strength: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: AlphaMode,
    pub base_color_texture: Option<TextureHandle>,
    pub metallic_roughness_texture: Option<TextureHandle>,
    pub normal_texture: Option<TextureHandle>,
    pub occlusion_texture: Option<TextureHandle>,
    pub emissive_texture: Option<TextureHandle>,
}

// TextureHandle is opaque - users don't know it's wgpu underneath
pub struct TextureHandle(pub u32);

// Light types - pure data, no wgpu
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
}
```

**Why this is good:**
- ✅ No wgpu types exposed
- ✅ Pure data structures
- ✅ Can swap backend without breaking user code
- ✅ Simple, intuitive API

---

### ❌ Bad: Leaky Abstraction (What We're Avoiding)

```rust
// DON'T DO THIS - exposes wgpu types
pub struct Material {
    pub base_color: Vec4,
    pub wgpu_texture: wgpu::Texture,  // ❌ wgpu type leaked!
    pub wgpu_bind_group: wgpu::BindGroup,  // ❌ wgpu type leaked!
}

// DON'T DO THIS - exposes backend details
pub fn render_mesh(
    mesh: &Mesh,
    device: &wgpu::Device,  // ❌ wgpu type leaked!
    queue: &wgpu::Queue,    // ❌ wgpu type leaked!
) { }
```

**Why this is bad:**
- ❌ Users must know about wgpu
- ❌ Can't swap backend without breaking user code
- ❌ Violates abstraction principle

---

## Implementation Strategy

### 1. Public API Layer (User-Facing)

**Location**: `crates/windjammer-game-framework/src/`

**Files:**
- `pbr.rs` - PBR material definitions (PUBLIC)
- `mesh.rs` - Mesh data structures (PUBLIC)
- `texture.rs` - Texture handles (PUBLIC)
- `camera3d.rs` - Camera definitions (PUBLIC)

**Characteristics:**
- ✅ No wgpu types
- ✅ No rendering backend types
- ✅ Pure data structures
- ✅ Builder patterns for ease of use
- ✅ Handle-based resource management

**Example:**
```rust
// PUBLIC API - users write this
let material = PBRMaterial::new()
    .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
    .with_metallic(0.8)
    .with_roughness(0.2)
    .with_base_color_texture(texture_handle);

let mesh = Mesh::cube();
let light = Light::sun();

// Render (backend is hidden)
renderer.draw_mesh(&mesh, &material, &light);
```

---

### 2. Internal Rendering Layer (Hidden from Users)

**Location**: `crates/windjammer-game-framework/src/rendering/`

**Files:**
- `backend.rs` - GraphicsBackend, Vertex3D (INTERNAL)
- `pipeline_pbr.rs` - PipelinePBR, uniforms (INTERNAL)
- `pipeline_3d.rs` - Pipeline3D (INTERNAL)
- `pipeline_2d.rs` - Pipeline2D (INTERNAL)

**Characteristics:**
- ✅ Contains wgpu types
- ✅ Not exposed in public API
- ✅ Converts public types to backend types
- ✅ Swappable implementation

**Example:**
```rust
// INTERNAL - users never see this
impl Renderer3D {
    pub fn draw_mesh(&mut self, mesh: &Mesh, material: &PBRMaterial, light: &Light) {
        // Convert public API to backend types
        let material_uniform = MaterialUniform::from_material(material);
        let light_uniform = LightUniform::from_light(light);
        
        // Use wgpu internally (hidden from user)
        let material_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&[material_uniform]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // ... rest of rendering code
    }
}
```

---

### 3. Conversion Layer (Bridge)

**Purpose**: Convert public API types to backend types

**Pattern:**
```rust
// Public API type
pub struct PBRMaterial {
    pub base_color: Vec4,
    pub metallic: f32,
    // ... no wgpu types
}

// Internal uniform type (for wgpu)
#[repr(C)]
struct MaterialUniform {
    base_color: [f32; 4],
    metallic: f32,
    // ... wgpu-compatible layout
}

// Conversion function (internal)
impl MaterialUniform {
    fn from_material(material: &PBRMaterial) -> Self {
        Self {
            base_color: material.base_color.to_array(),
            metallic: material.metallic,
            // ...
        }
    }
}
```

**Benefits:**
- ✅ Public API stays clean
- ✅ Backend can change without affecting users
- ✅ Type safety maintained
- ✅ Performance (zero-cost conversion)

---

## Current Status: Verification

### ✅ What We're Doing Right

1. **PBRMaterial** - Clean public API
   - ✅ No wgpu types
   - ✅ Pure data structure
   - ✅ Builder methods
   - ✅ TextureHandle (opaque)

2. **Light** - Clean public API
   - ✅ No wgpu types
   - ✅ Enum for different light types
   - ✅ Pure data structures

3. **Conversion Layer** - Proper separation
   - ✅ `MaterialUniform::from_material()`
   - ✅ `LightUniform::from_light()`
   - ✅ Internal only

4. **Pipeline** - Hidden implementation
   - ✅ `PipelinePBR` is internal
   - ✅ Not exposed in public API
   - ✅ Users never see wgpu types

### ⚠️ What Needs Attention

1. **Vertex3D** - Currently in backend.rs
   - ⚠️ Has `tangent` field (good for PBR)
   - ⚠️ Need to ensure it's not exposed publicly
   - ✅ Should stay in `rendering::backend` (internal)

2. **TextureHandle** - Currently just `u32`
   - ✅ Opaque handle (good)
   - ⚠️ Need texture loading API (public)
   - ⚠️ Need texture management (internal)

3. **Renderer3D** - Public interface
   - ⚠️ Need to verify no wgpu types leaked
   - ⚠️ Need clean `draw_mesh()` API
   - ⚠️ Need clean `draw_pbr()` API

---

## Action Items (To Ensure Clean API)

### 1. Verify Public API Exports

**Check `lib.rs`:**
```rust
// GOOD - expose public API
pub use pbr::{PBRMaterial, Light, DirectionalLight, PointLight, SpotLight};
pub use texture::TextureHandle;
pub use mesh::Mesh;
pub use camera3d::Camera3D;

// BAD - don't expose internals
// pub use rendering::backend::Vertex3D;  // ❌ DON'T DO THIS
// pub use rendering::pipeline_pbr::PipelinePBR;  // ❌ DON'T DO THIS
```

### 2. Create Clean Renderer API

**Public renderer interface:**
```rust
// PUBLIC API
pub struct Renderer3D {
    // Internal fields are private
    backend: GraphicsBackend,  // private
    pipeline: PipelinePBR,     // private
    // ...
}

impl Renderer3D {
    // Clean public methods (no wgpu types)
    pub fn new(window: &Window) -> Self { }
    pub fn draw_mesh(&mut self, mesh: &Mesh, material: &PBRMaterial) { }
    pub fn set_light(&mut self, light: &Light) { }
    pub fn set_camera(&mut self, camera: &Camera3D) { }
    pub fn present(&mut self) { }
}
```

### 3. Texture Loading API

**Public texture API:**
```rust
// PUBLIC API
pub struct TextureManager {
    // Internal implementation hidden
}

impl TextureManager {
    pub fn load_texture(&mut self, path: &str) -> Result<TextureHandle, String> { }
    pub fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> TextureHandle { }
    pub fn unload_texture(&mut self, handle: TextureHandle) { }
}
```

### 4. Mesh API

**Public mesh API:**
```rust
// PUBLIC API
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub tex_coords: Vec<Vec2>,
    pub tangents: Vec<Vec4>,  // Exposed for user-generated meshes
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self { }
    pub fn cube() -> Self { }
    pub fn sphere(radius: f32) -> Self { }
    pub fn from_gltf(path: &str) -> Result<Self, String> { }
}
```

---

## Benefits of This Architecture

### 1. Backend Swappability

**Can easily swap from wgpu to:**
- OpenGL (for older systems)
- Vulkan (for direct control)
- Metal (for macOS/iOS optimization)
- DirectX 12 (for Windows optimization)
- WebGPU (for web)
- Custom renderer (for consoles)

**User code doesn't change!**

### 2. Testing

**Can create mock renderer for tests:**
```rust
struct MockRenderer {
    draw_calls: Vec<DrawCall>,
}

impl Renderer for MockRenderer {
    fn draw_mesh(&mut self, mesh: &Mesh, material: &PBRMaterial) {
        self.draw_calls.push(DrawCall { mesh, material });
    }
}
```

### 3. Performance Profiling

**Can wrap renderer for profiling:**
```rust
struct ProfilingRenderer {
    inner: Box<dyn Renderer>,
    stats: RenderStats,
}

impl Renderer for ProfilingRenderer {
    fn draw_mesh(&mut self, mesh: &Mesh, material: &PBRMaterial) {
        let start = Instant::now();
        self.inner.draw_mesh(mesh, material);
        self.stats.record_draw_time(start.elapsed());
    }
}
```

### 4. Future-Proofing

**Can add new backends without breaking existing code:**
- Ray tracing backend
- Path tracing backend
- Software renderer (for debugging)
- Distributed rendering (for cloud)

---

## Verification Checklist

### ✅ Public API (What Users See)

- [x] `PBRMaterial` - no wgpu types
- [x] `Light` - no wgpu types
- [x] `TextureHandle` - opaque handle
- [ ] `Mesh` - need to verify (TODO)
- [ ] `Renderer3D` - need to verify (TODO)
- [ ] `TextureManager` - need to create (TODO)

### ✅ Internal API (Hidden from Users)

- [x] `PipelinePBR` - internal only
- [x] `GraphicsBackend` - internal only
- [x] `Vertex3D` - internal only
- [x] `MaterialUniform` - internal only
- [x] `LightUniform` - internal only

### ✅ Conversion Layer

- [x] `MaterialUniform::from_material()` - implemented
- [x] `LightUniform::from_light()` - implemented
- [ ] `Vertex3D::from_mesh()` - need to implement (TODO)

---

## Next Steps

1. **Verify `lib.rs` exports** - Ensure no wgpu types leaked
2. **Create clean `Renderer3D` API** - No wgpu in public methods
3. **Implement `TextureManager`** - Public texture loading API
4. **Implement `Mesh` API** - Public mesh creation/loading
5. **Add integration tests** - Test public API only (no wgpu)

---

## Example: Complete User Code (No wgpu!)

```rust
// User code - pure Windjammer, no wgpu!
use windjammer::prelude::*;

fn main() {
    // Create renderer (backend hidden)
    let mut renderer = Renderer3D::new(&window);
    
    // Load texture (no wgpu types)
    let texture = renderer.load_texture("albedo.png").unwrap();
    
    // Create material (no wgpu types)
    let material = PBRMaterial::new()
        .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
        .with_metallic(0.8)
        .with_roughness(0.2)
        .with_base_color_texture(texture);
    
    // Create mesh (no wgpu types)
    let mesh = Mesh::sphere(1.0);
    
    // Create light (no wgpu types)
    let light = Light::sun();
    
    // Create camera (no wgpu types)
    let camera = Camera3D::new()
        .at(Vec3::new(0.0, 0.0, 5.0))
        .looking_at(Vec3::ZERO);
    
    // Render loop (no wgpu types)
    loop {
        renderer.begin_frame();
        renderer.set_camera(&camera);
        renderer.set_light(&light);
        renderer.draw_mesh(&mesh, &material);
        renderer.end_frame();
    }
}
```

**User never sees wgpu, can't accidentally depend on it!** ✅

---

## Conclusion

**Current Status**: ✅ Good foundation, needs verification

**Architecture**: ✅ Follows public API principle

**Next Steps**: 
1. Verify no leaks in `lib.rs`
2. Create clean renderer API
3. Add texture/mesh management APIs
4. Test with public API only

**Goal**: Users write pure Windjammer code, backend is 100% swappable! 🎯

---

*"Good API design is about what you hide, not what you show."*


