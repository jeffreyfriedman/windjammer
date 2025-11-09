# SSGI Integration Plan

## Overview
This document outlines the plan for integrating Screen-Space Global Illumination (SSGI) into the Windjammer game framework's Renderer3D.

## Current Status
✅ G-buffer shader created (`gbuffer.wgsl`)
✅ SSGI compute shader created (`ssgi_simple.wgsl`)
⏳ Integration into Renderer3D
⏳ Integration into shooter game

## Architecture

### Rendering Pipeline
The SSGI system uses a **deferred rendering** approach with three main passes:

```
1. G-Buffer Pass (Geometry)
   ├─ Render scene to multiple render targets (MRT)
   ├─ Output: Position, Normal, Albedo textures
   └─ Uses: gbuffer.wgsl shader

2. SSGI Compute Pass (Lighting)
   ├─ Read G-buffer textures
   ├─ Calculate indirect lighting per pixel
   ├─ Output: GI contribution texture
   └─ Uses: ssgi_simple.wgsl compute shader

3. Composite Pass (Final)
   ├─ Combine direct lighting + GI
   ├─ Output: Final frame
   └─ Uses: composite shader (to be created)
```

### Data Flow

```
Scene Geometry
     ↓
G-Buffer Pass → [Position] [Normal] [Albedo]
     ↓              ↓         ↓        ↓
SSGI Compute ← ─────┴─────────┴────────┘
     ↓
[GI Texture]
     ↓
Composite Pass → Final Frame
```

## Implementation Strategy

### Phase 1: Foundation (Simple Integration)
**Goal**: Get SSGI working without breaking existing renderer

**Approach**: Add SSGI as an **optional** feature that can be toggled on/off

**Changes to Renderer3D**:
1. Add `ssgi_enabled: bool` field
2. Add G-buffer textures (position, normal, albedo)
3. Add SSGI compute pipeline
4. Add composite pipeline
5. Modify `present()` to use multi-pass rendering when SSGI is enabled

**API**:
```rust
impl Renderer3D {
    pub fn enable_ssgi(&mut self, enabled: bool) { ... }
    pub fn set_ssgi_intensity(&mut self, intensity: f32) { ... }
    pub fn set_ssgi_samples(&mut self, samples: u32) { ... }
}
```

### Phase 2: Optimization
**Goal**: Make SSGI performant for real-time use

**Optimizations**:
1. **Temporal Reprojection**: Reuse previous frame's GI
2. **Spatial Denoising**: Blur GI to reduce noise
3. **Half-Resolution**: Compute GI at half resolution, upscale
4. **Adaptive Sampling**: More samples for complex areas

### Phase 3: Advanced Features
**Goal**: Match AAA quality

**Features**:
1. **Multi-Bounce GI**: Simulate multiple light bounces
2. **Emissive Materials**: Support glowing surfaces
3. **Sky Light**: Add ambient sky contribution
4. **Light Probes**: Bake static GI for performance

## Technical Details

### G-Buffer Layout
```
Texture 0 (Position): RGBA32Float
  - RGB: World position
  - A: Depth (for optimization)

Texture 1 (Normal): RGBA16Float
  - RGB: World normal (normalized)
  - A: Unused (future: roughness)

Texture 2 (Albedo): RGBA8Unorm
  - RGB: Base color
  - A: Metallic (future)
```

### SSGI Parameters
```rust
pub struct SSGIConfig {
    pub enabled: bool,
    pub num_samples: u32,      // 4-32 (default: 8)
    pub sample_radius: f32,    // 0.1-2.0 (default: 0.5)
    pub intensity: f32,        // 0.0-2.0 (default: 1.0)
    pub max_distance: f32,     // Max ray distance
    pub falloff: f32,          // Distance falloff
}
```

### Performance Considerations

**Memory Usage**:
- G-Buffer: 3 textures × screen resolution
  - 1920×1080: ~24 MB (uncompressed)
- SSGI Output: 1 texture × screen resolution
  - 1920×1080: ~8 MB

**Compute Cost**:
- G-Buffer Pass: ~same as forward rendering
- SSGI Compute: ~1-3ms (8 samples, 1080p)
- Composite: ~0.1ms

**Total Overhead**: ~2-4ms per frame (30-60 FPS → 25-50 FPS)

## Implementation Steps

### Step 1: Add G-Buffer Resources
```rust
// In Renderer3D struct
gbuffer_position: wgpu::Texture,
gbuffer_normal: wgpu::Texture,
gbuffer_albedo: wgpu::Texture,
gbuffer_pipeline: wgpu::RenderPipeline,
```

### Step 2: Create SSGI Compute Pipeline
```rust
ssgi_pipeline: wgpu::ComputePipeline,
ssgi_output: wgpu::Texture,
ssgi_bind_group: wgpu::BindGroup,
ssgi_config: SSGIConfig,
```

### Step 3: Create Composite Pipeline
```rust
composite_pipeline: wgpu::RenderPipeline,
composite_bind_group: wgpu::BindGroup,
```

### Step 4: Modify Rendering Flow
```rust
pub fn present(&mut self) {
    if self.ssgi_enabled {
        // 1. Render to G-buffer
        self.render_gbuffer();
        
        // 2. Run SSGI compute
        self.compute_ssgi();
        
        // 3. Composite final image
        self.composite_final();
    } else {
        // Original forward rendering
        self.render_forward();
    }
}
```

## Windjammer Philosophy Compliance

### Zero Crate Leakage ✅
- All SSGI configuration uses Windjammer-friendly types
- No `wgpu` types exposed in public API
- Internal implementation details hidden

### Automatic Ownership Inference ✅
- SSGI methods follow existing patterns
- No manual `&mut` required in Windjammer code

### Simple API ✅
```windjammer
@render3d
fn render(game: ShooterGame, renderer: Renderer3D, camera: Camera3D) {
    // Enable SSGI with one line
    renderer.enable_ssgi(true)
    
    // Adjust quality
    renderer.set_ssgi_samples(16)
    
    // Normal rendering continues...
    renderer.draw_cube(pos, size, color)
}
```

## Testing Strategy

### Unit Tests
1. G-buffer creation and layout
2. SSGI compute shader execution
3. Composite pass correctness

### Integration Tests
1. SSGI on/off toggle
2. Parameter changes
3. Performance benchmarks

### Visual Tests
1. Compare with/without SSGI
2. Verify no artifacts
3. Check lighting accuracy

## Fallback Strategy

If SSGI is too complex or causes issues:
1. **Fallback 1**: Simplified SSAO (Ambient Occlusion only)
2. **Fallback 2**: Baked lightmaps
3. **Fallback 3**: Simple ambient term

## Timeline

**Phase 1 (Foundation)**: 2-3 hours
- Add G-buffer resources
- Create pipelines
- Basic integration

**Phase 2 (Optimization)**: 3-4 hours
- Temporal reprojection
- Spatial denoising
- Performance tuning

**Phase 3 (Advanced)**: 4-6 hours
- Multi-bounce GI
- Emissive materials
- Light probes

**Total**: 9-13 hours for full implementation

## Current Session Plan

For this session, we'll focus on **Phase 1 (Foundation)** with a simplified approach:

1. ✅ Create shaders (DONE!)
2. ⏳ Add basic SSGI toggle to Renderer3D
3. ⏳ Implement simple G-buffer rendering
4. ⏳ Add SSGI compute pass
5. ⏳ Test in shooter game

**Goal**: Demonstrate SSGI working, even if simplified, to showcase Windjammer's capability for cutting-edge features!

## Success Criteria

✅ SSGI can be toggled on/off
✅ Visible lighting improvement with SSGI enabled
✅ No performance regression when SSGI is disabled
✅ Clean Windjammer API (zero crate leakage)
✅ Documentation and examples

---

**Status**: Ready to implement Phase 1!
**Next**: Add G-buffer resources to Renderer3D

