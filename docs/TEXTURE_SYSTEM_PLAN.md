# 🎨 Texture System Implementation Plan

**Goal:** Add comprehensive texture support to exercise Windjammer's capabilities

---

## 📋 **Requirements**

### Core Features
1. **Texture Loading** - Load PNG/JPEG from disk
2. **Texture Management** - Handle multiple textures
3. **Texture Binding** - Apply textures to 3D meshes
4. **UV Mapping** - Proper texture coordinates
5. **Texture Atlas** - Multiple textures in one image (optional)

### Windjammer API Design
```windjammer
// Simple, zero-crate-leakage API
struct Texture {
    id: int,
    width: int,
    height: int,
}

// Load texture from file
fn load_texture(path: string) -> Texture

// Draw textured cube
renderer.draw_textured_cube(pos, size, texture)

// Draw textured plane
renderer.draw_textured_plane(pos, size, texture)
```

---

## 🏗️ **Implementation Steps**

### Phase 1: Rust Backend (Framework)
1. Add `image` crate to `windjammer-game-framework`
2. Create `Texture` struct in renderer
3. Implement texture loading
4. Update shader to support textures
5. Add texture binding to render pipeline

### Phase 2: Windjammer API
1. Add `Texture` type to Windjammer
2. Add `load_texture()` function
3. Add textured drawing methods
4. Update codegen for texture types

### Phase 3: Game Integration
1. Create texture assets for shooter
2. Load textures in `@init`
3. Apply textures to walls
4. Apply textures to enemies
5. Apply textures to power-ups

---

## 🎯 **Language Gaps to Surface**

### 1. **Resource Management**
- How does Windjammer handle file I/O?
- How do we represent opaque handles (texture IDs)?
- How do we handle loading errors?

### 2. **Lifetime Management**
- Textures must outlive renderer
- How does Windjammer infer this?
- Do we need explicit lifetime annotations?

### 3. **Error Handling**
- File not found
- Invalid image format
- GPU allocation failure

### 4. **Performance**
- Texture caching
- Texture reuse
- Batch rendering with multiple textures

---

## 📝 **Implementation**

### Step 1: Add Dependencies
```toml
# crates/windjammer-game-framework/Cargo.toml
[dependencies]
image = "0.25"
```

### Step 2: Texture Struct
```rust
// crates/windjammer-game-framework/src/texture.rs
pub struct Texture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn from_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();
        
        // Create wgpu texture
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(path),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        Ok(Self {
            texture,
            view,
            sampler,
            width: dimensions.0,
            height: dimensions.1,
        })
    }
}
```

### Step 3: Update Renderer
```rust
// Add to Renderer3D
pub fn load_texture(&self, path: &str) -> Result<Texture, Box<dyn std::error::Error>> {
    Texture::from_file(&self.device, &self.queue, path)
}

pub fn draw_textured_cube(&mut self, position: Vec3, size: Vec3, texture: &Texture) {
    // Similar to draw_cube but with texture coordinates
}
```

### Step 4: Windjammer API
```windjammer
// In game code
@init
fn init(game: ShooterGame) {
    // Load textures
    game.wall_texture = load_texture("assets/wall.png")
    game.enemy_texture = load_texture("assets/enemy.png")
    game.floor_texture = load_texture("assets/floor.png")
}

@render3d
fn render(game: ShooterGame, renderer: Renderer3D, camera: Camera3D) {
    // Draw textured walls
    for wall in game.walls {
        renderer.draw_textured_cube(wall.pos, wall.size, game.wall_texture)
    }
}
```

---

## 🧪 **Testing Strategy**

### Unit Tests
- Load valid PNG
- Load valid JPEG
- Handle missing file
- Handle invalid format

### Integration Tests
- Render textured cube
- Render multiple textures
- Texture coordinates correct

### Game Tests
- All textures load
- Textures render correctly
- No performance regression

---

## 🚀 **Next Steps**

1. Implement texture loading in framework
2. Update shader for texture sampling
3. Add Windjammer API
4. Create texture assets
5. Integrate into shooter game
6. Test and document

---

**Status:** Ready to implement!

