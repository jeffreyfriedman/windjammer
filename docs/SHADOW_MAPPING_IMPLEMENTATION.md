# Shadow Mapping Implementation Plan
# Production-Quality Shadows for Windjammer

**Date**: November 2025  
**Status**: IN PROGRESS  
**Goal**: Implement high-quality shadow mapping for all light types

---

## Overview

Shadow mapping is a critical feature for realistic 3D rendering. We're implementing:

1. **Directional Light Shadows** (sun/moon) - Cascaded Shadow Maps (CSM)
2. **Point Light Shadows** (bulbs) - Cube Shadow Maps
3. **Spot Light Shadows** (flashlights) - Standard Shadow Maps
4. **PCF (Percentage Closer Filtering)** - Soft shadows
5. **Shadow Bias** - Prevent shadow acne
6. **Shadow Cascades** - Better quality at different distances

---

## Architecture

### Shadow Map Generation Pass

```
For each shadow-casting light:
  1. Create shadow map texture (depth buffer)
  2. Set up light-space view-projection matrix
  3. Render scene from light's perspective
  4. Store depth values in shadow map
```

### Main Rendering Pass

```
For each fragment:
  1. Transform fragment position to light space
  2. Sample shadow map at projected position
  3. Compare fragment depth with shadow map depth
  4. Apply PCF for soft shadows
  5. Multiply lighting by shadow factor (0.0-1.0)
```

---

## Implementation Components

### 1. Shadow Map Shader (DONE ✅)

**File**: `shaders/shadow_map.wgsl`

**Purpose**: Render depth from light's perspective

**Features**:
- Simple vertex shader (transform to light space)
- No fragment shader needed (depth written automatically)
- Optimized for performance

### 2. PBR Shader Enhancement (TODO)

**File**: `shaders/pbr.wgsl`

**Additions Needed**:
```wgsl
// Add to bind groups
@group(3) @binding(0)
var shadow_map: texture_depth_2d;

@group(3) @binding(1)
var shadow_sampler: sampler_comparison;

@group(3) @binding(2)
var<uniform> light_view_proj: mat4x4<f32>;

// Shadow sampling function
fn sample_shadow(world_pos: vec3<f32>) -> f32 {
    // Transform to light space
    let light_space_pos = light_view_proj * vec4<f32>(world_pos, 1.0);
    
    // Perspective divide
    let proj_coords = light_space_pos.xyz / light_space_pos.w;
    
    // Transform to [0,1] range
    let shadow_coords = proj_coords * 0.5 + 0.5;
    
    // Sample shadow map with PCF
    var shadow = 0.0;
    let texel_size = 1.0 / 2048.0; // Shadow map resolution
    
    // 3x3 PCF
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                shadow_coords.xy + offset,
                shadow_coords.z - 0.005 // bias
            );
        }
    }
    shadow /= 9.0; // Average
    
    return shadow;
}
```

### 3. Shadow Map Pipeline (TODO)

**File**: `rendering/pipeline_shadow.rs`

**Structure**:
```rust
pub struct ShadowMapPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ShadowMapPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        // Create pipeline for shadow map generation
    }
    
    pub fn render_shadow_map(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shadow_texture: &wgpu::TextureView,
        light_view_proj: Mat4,
        meshes: &[Mesh],
    ) {
        // Render scene from light's perspective
    }
}
```

### 4. Shadow Map Manager (TODO)

**File**: `rendering/shadow_manager.rs`

**Purpose**: Manage shadow map resources

**Structure**:
```rust
pub struct ShadowManager {
    // Shadow map textures
    directional_shadow_maps: Vec<ShadowMap>,
    point_shadow_maps: Vec<CubeShadowMap>,
    spot_shadow_maps: Vec<ShadowMap>,
    
    // Shadow map pipeline
    pipeline: ShadowMapPipeline,
    
    // Configuration
    config: ShadowConfig,
}

pub struct ShadowMap {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    resolution: u32,
}

pub struct CubeShadowMap {
    texture: wgpu::Texture,
    views: [wgpu::TextureView; 6], // 6 faces
    resolution: u32,
}

pub struct ShadowConfig {
    pub resolution: u32,
    pub cascade_count: u32,
    pub cascade_splits: Vec<f32>,
    pub bias: f32,
    pub normal_bias: f32,
    pub pcf_radius: u32,
}
```

### 5. Public API (TODO)

**File**: `pbr.rs`

**Additions**:
```rust
impl DirectionalLight {
    pub fn with_shadows(mut self, enabled: bool) -> Self {
        self.cast_shadows = enabled;
        self
    }
}

impl PointLight {
    pub fn with_shadows(mut self, enabled: bool) -> Self {
        self.cast_shadows = enabled;
        self
    }
}

impl SpotLight {
    pub fn with_shadows(mut self, enabled: bool) -> Self {
        self.cast_shadows = enabled;
        self
    }
}

// Shadow configuration (public API)
pub struct ShadowConfig {
    pub resolution: u32,
    pub bias: f32,
    pub normal_bias: f32,
    pub pcf_samples: u32,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            resolution: 2048,
            bias: 0.005,
            normal_bias: 0.01,
            pcf_samples: 9, // 3x3
        }
    }
}
```

---

## Shadow Techniques

### 1. Directional Light Shadows (CSM)

**Cascaded Shadow Maps** for better quality at different distances:

```
Camera frustum split into cascades:
- Cascade 0: 0-10m (high resolution)
- Cascade 1: 10-50m (medium resolution)
- Cascade 2: 50-200m (low resolution)
- Cascade 3: 200m+ (very low resolution)

Each cascade has its own shadow map.
Fragment selects cascade based on distance from camera.
```

**Benefits**:
- High quality shadows near camera
- Acceptable quality shadows far away
- Efficient use of shadow map resolution

### 2. Point Light Shadows (Cube Maps)

**Cube Shadow Maps** for omnidirectional shadows:

```
6 shadow maps (one per cube face):
- +X, -X, +Y, -Y, +Z, -Z

Fragment direction determines which face to sample.
```

**Optimization**:
- Only render faces that are visible
- Use lower resolution for distant lights
- Limit number of shadow-casting point lights

### 3. Spot Light Shadows (Standard)

**Standard Shadow Maps** (simplest case):

```
Single shadow map per spot light.
Similar to directional, but with perspective projection.
```

---

## Quality Improvements

### 1. PCF (Percentage Closer Filtering)

**Soft shadows** by sampling multiple texels:

```wgsl
// 3x3 PCF (9 samples)
for (var x = -1; x <= 1; x++) {
    for (var y = -1; y <= 1; y++) {
        shadow += sample_shadow_map(coords + offset);
    }
}
shadow /= 9.0;
```

**Variants**:
- 3x3 PCF: 9 samples (fast, medium quality)
- 5x5 PCF: 25 samples (medium, good quality)
- 7x7 PCF: 49 samples (slow, excellent quality)
- Poisson disk: Variable samples (good quality, less banding)

### 2. Shadow Bias

**Prevent shadow acne** (self-shadowing artifacts):

```wgsl
let bias = 0.005; // Constant bias
let normal_bias = 0.01 * (1.0 - dot(N, L)); // Slope-based bias
let depth_bias = bias + normal_bias;

let in_shadow = depth_compare(shadow_depth, fragment_depth - depth_bias);
```

### 3. Shadow Fading

**Fade shadows at distance** for better performance:

```wgsl
let fade_start = 100.0;
let fade_end = 200.0;
let fade_factor = smoothstep(fade_start, fade_end, distance);
shadow = mix(shadow, 1.0, fade_factor);
```

---

## Performance Considerations

### 1. Shadow Map Resolution

**Trade-off**: Quality vs. Memory/Performance

```
Resolution | Memory (per map) | Quality
-----------|------------------|--------
512x512    | 0.25 MB          | Low
1024x1024  | 1 MB             | Medium
2048x2048  | 4 MB             | High
4096x4096  | 16 MB            | Very High
```

**Recommendation**: 2048x2048 for directional, 1024x1024 for others

### 2. Shadow Casting Limits

**Limit number of shadow-casting lights**:

```
- Directional: 1 (sun)
- Point: 4 (nearby lights)
- Spot: 4 (nearby lights)
```

**Dynamic selection**:
- Sort lights by importance (distance, intensity)
- Only cast shadows for most important lights
- Other lights use no shadows

### 3. Update Frequency

**Not all shadows need to update every frame**:

```
- Static objects: Update once, cache shadow map
- Dynamic objects: Update every frame
- Distant lights: Update every N frames
```

---

## Implementation Phases

### Phase 1: Basic Shadow Maps (Current)
- [x] Shadow map generation shader
- [ ] Shadow map pipeline
- [ ] Directional light shadows (single cascade)
- [ ] Shadow sampling in PBR shader
- [ ] Basic PCF (3x3)

### Phase 2: Quality Improvements
- [ ] Cascaded shadow maps (CSM)
- [ ] Better PCF (5x5 or Poisson)
- [ ] Shadow bias tuning
- [ ] Shadow fading

### Phase 3: All Light Types
- [ ] Point light shadows (cube maps)
- [ ] Spot light shadows
- [ ] Multiple shadow-casting lights

### Phase 4: Optimization
- [ ] Shadow map caching
- [ ] Dynamic light selection
- [ ] LOD for shadow maps
- [ ] Frustum culling for shadow rendering

---

## Testing Strategy

### Visual Tests
1. Shadow acne (should be minimal)
2. Peter panning (shadows detached from objects)
3. Shadow aliasing (jagged edges)
4. Shadow softness (PCF quality)
5. Shadow distance (fade properly)

### Performance Tests
1. Frame time with shadows on/off
2. Memory usage (shadow map textures)
3. Shadow map generation time
4. Multiple lights performance

---

## Example Usage (Public API)

```rust
// Create light with shadows
let sun = DirectionalLight::new(
    Vec3::new(-0.3, -1.0, -0.3),
    Vec3::new(1.0, 0.95, 0.9),
    1.0,
).with_shadows(true);

// Configure shadow quality
let shadow_config = ShadowConfig {
    resolution: 2048,
    bias: 0.005,
    normal_bias: 0.01,
    pcf_samples: 9,
};

// Render with shadows
renderer.set_shadow_config(shadow_config);
renderer.set_light(&sun);
renderer.draw_mesh(&mesh, &material);
```

**User never sees wgpu types!** ✅

---

## Current Status

**Completed**:
- ✅ Shadow map generation shader
- ✅ Architecture design
- ✅ Implementation plan

**In Progress**:
- ⏳ Shadow map pipeline
- ⏳ PBR shader enhancement
- ⏳ Shadow manager

**TODO**:
- ⏳ Integration with Renderer3D
- ⏳ Testing and tuning
- ⏳ Documentation

---

## Next Steps

1. Implement `ShadowMapPipeline`
2. Enhance PBR shader with shadow sampling
3. Create `ShadowManager`
4. Integrate with `Renderer3D`
5. Test with sample scenes
6. Tune bias and PCF parameters
7. Add cascaded shadow maps
8. Optimize performance

**Estimated Time**: 4-6 hours for basic shadows, 8-12 hours for full implementation

---

*"Shadows are what give depth to light."*


