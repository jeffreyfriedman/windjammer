# Windjammer Game Engine Architecture

**Complete 2D/3D Game Engine with TDD Methodology**

## Core Philosophy

1. **80/20 Rule**: 80% of Rust's power, 20% of complexity
2. **Progressive Complexity**: Simple for 80% use case, powerful for 100%
3. **Dual Workflow**: Code-first OR Editor-first
4. **WindjammerScript**: Interpreted Windjammer for fast iteration, compiled for production
5. **3D-Ready**: Design 2D features with 3D extensibility

---

## Sprint 1: Texture & Sprite System ✅🔄

### 1.1 Texture Loading ✅ COMPLETE

**Files**: `src/ffi/texture.rs`, `tests_wj/texture_test.wj`

**Architecture**:
```
TextureManager (Thread-local)
├── textures: HashMap<u32, TextureData>
├── path_to_handle: HashMap<String, u32>
└── next_id: u32

TextureData
├── width: u32
├── height: u32
└── data: Vec<u8>  // RGBA8
```

**API**:
- `texture_load(path) -> u32` - Load with caching
- `texture_get_width(handle) -> u32`
- `texture_get_height(handle) -> u32`
- `texture_unload(handle)`

### 1.2 Sprite Rendering 🔄 IN PROGRESS

**Files**: `src/ffi/wgpu_renderer.rs`, `ffi/shaders/shader_textured.wgsl`

**Architecture**:
```
WgpuRenderer
├── texture_pipeline: RenderPipeline
├── texture_bind_groups: HashMap<u32, BindGroup>
├── sprite_batches: Vec<SpriteBatch>
└── current_batch: SpriteBatch

SpriteBatch
├── texture_handle: u32
├── vertices: Vec<VertexTextured>
└── indices: Vec<u16>

VertexTextured
├── position: [f32; 2]
├── uv: [f32; 2]
└── color: [f32; 4]  // Tint
```

**Implementation Steps**:
1. Create `VertexTextured` struct
2. Build wgpu::Texture from TextureData
3. Create bind group layout (texture + sampler)
4. Load shader_textured.wgsl
5. Implement sprite batching by texture
6. Render batches in `render()` call

**API**:
```rust
renderer_draw_sprite(
    texture: u32,
    x: f32, y: f32, w: f32, h: f32,
    rotation: f32,
    uv_x: f32, uv_y: f32, uv_w: f32, uv_h: f32,
    tint_r: f32, tint_g: f32, tint_b: f32, tint_a: f32
)
```

### 1.3 Sprite Batching (1000+ @ 60 FPS)

**Goal**: Render 1000+ sprites at 60 FPS (16.67ms budget)

**Optimization Strategies**:
1. **Sort by texture**: Minimize bind group changes
2. **Instance rendering**: Use wgpu instancing for same texture
3. **Vertex buffer pooling**: Reuse buffers across frames
4. **Frustum culling**: Only render visible sprites
5. **Z-sorting**: Depth sort for transparency

**Performance Metrics**:
- Batch creation: < 1ms
- GPU upload: < 2ms
- Draw calls: < 100 (ideally < 20)
- Total frame time: < 16.67ms

### 1.4 Sprite Atlas Support

**Architecture**:
```
SpriteAtlas
├── texture_handle: u32
├── regions: Vec<SpriteRegion>
└── name_to_index: HashMap<String, usize>

SpriteRegion
├── name: String
├── uv_x: f32, uv_y: f32
├── uv_width: f32, uv_height: f32
└── pivot: Vec2  // For rotation
```

**File Format** (JSON):
```json
{
  "texture": "assets/player_atlas.png",
  "regions": [
    {
      "name": "idle_01",
      "x": 0, "y": 0,
      "width": 32, "height": 32,
      "pivot": {"x": 16, "y": 28}
    }
  ]
}
```

---

## Sprint 2: Animation System

### 2.1 Frame-Based Animation

**Files**: `src_wj/animation/sprite_animation.wj`

**Architecture**:
```
Animation
├── frames: Vec<AnimationFrame>
├── fps: f32
├── loop_mode: LoopMode  // Once, Loop, PingPong
└── total_duration: f32

AnimationFrame
├── sprite_region: SpriteRegion
├── duration: f32  // Override fps if needed
└── events: Vec<String>  // Trigger events (e.g., "footstep")

AnimationState
├── current_frame: usize
├── time_accumulator: f32
└── playing: bool
```

**Update Logic**:
```rust
fn update(delta: f32) {
    if !playing { return; }
    
    time_accumulator += delta;
    let frame_duration = 1.0 / fps;
    
    while time_accumulator >= frame_duration {
        time_accumulator -= frame_duration;
        current_frame = (current_frame + 1) % frames.len();
        
        // Trigger events
        for event in frames[current_frame].events {
            trigger_event(event);
        }
    }
}
```

### 2.2 Animation State Machine

**Files**: `src_wj/animation/controller.wj`

**Architecture**:
```
AnimationController
├── animations: HashMap<String, Animation>
├── transitions: Vec<Transition>
├── current_state: String
└── next_state: Option<String>

Transition
├── from_state: String
├── to_state: String
├── condition: Fn() -> bool
└── blend_time: f32  // For smooth transitions
```

**Example Usage**:
```wj
let controller = AnimationController::new();
controller.add_animation("idle", idle_anim);
controller.add_animation("run", run_anim);
controller.add_animation("jump", jump_anim);

controller.add_transition("idle", "run", || input.is_moving());
controller.add_transition("run", "idle", || !input.is_moving());
controller.add_transition("*", "jump", || input.jump_pressed());
```

---

## Sprint 3: Tilemap System

### 3.1 Tilemap Data Structure

**Files**: `src_wj/world/tilemap.wj`

**Architecture**:
```
Tilemap
├── tiles: Vec<Vec<Tile>>  // [row][col]
├── tile_width: u32
├── tile_height: u32
├── tileset: SpriteAtlas
└── collision_map: Vec<Vec<TileCollision>>

Tile
├── id: u32  // Index into tileset
└── properties: HashMap<String, String>

TileCollision
├── solid: bool
├── one_way: bool  // Platform
└── collision_rect: Option<Rect>
```

**API**:
```wj
fn new(width: usize, height: usize, tileset: SpriteAtlas) -> Tilemap;
fn get_tile(x: usize, y: usize) -> Tile;
fn set_tile(x: usize, y: usize, tile: Tile);
fn world_to_tile(world_pos: Vec2) -> (usize, usize);
fn tile_to_world(tile_x: usize, tile_y: usize) -> Vec2;
```

### 3.2 Tilemap Rendering

**Optimization**: Batch all visible tiles into single draw call

**Algorithm**:
1. Calculate visible tile range (camera frustum)
2. For each visible tile:
   - Get sprite region from tileset
   - Calculate world position
   - Add to sprite batch
3. Render entire batch (1 draw call!)

**Performance**: 100x100 tilemap = 10,000 tiles → 1 draw call

### 3.3 Tilemap Collision

**Files**: `src_wj/world/tilemap.wj`

**API**:
```wj
fn collides_with_rect(rect: Rect) -> bool;
fn get_colliding_tiles(rect: Rect) -> Vec<(usize, usize)>;
fn raycast(start: Vec2, end: Vec2) -> Option<RaycastHit>;
```

**Algorithm** (Tile-based AAB`B):
```
1. Convert rect to tile coordinates
2. Check tiles in rect bounds
3. For each solid tile:
   - Check AABB overlap
   - Calculate penetration depth
4. Return collision info
```

---

## Sprint 4: Character Controller

### 4.1 Ground Detection

**Files**: `src_wj/physics/character_controller.wj`

**Methods**:
1. **Raycast Down**: Cast ray from feet
2. **Overlap Check**: Check AABB below character
3. **Tile Query**: Check tilemap at feet position

**API**:
```wj
fn is_grounded() -> bool;
fn get_ground_normal() -> Vec2;
fn get_ground_velocity() -> Vec2;  // For moving platforms
```

### 4.2 Jump Mechanics

**Features**:
- Variable jump height (hold = higher)
- Coyote time (grace period after leaving ground)
- Jump buffering (input before landing)

**Parameters**:
```wj
struct JumpConfig {
    jump_velocity: f32,       // 400.0
    gravity: f32,             // 980.0
    terminal_velocity: f32,   // -600.0
    coyote_time: f32,         // 0.1s
    jump_buffer_time: f32,    // 0.1s
    jump_cut_multiplier: f32, // 0.5 (when release jump)
}
```

**Algorithm**:
```wj
fn update(delta: f32) {
    // Coyote time
    if was_grounded && !is_grounded {
        coyote_timer = coyote_time;
    }
    coyote_timer -= delta;
    
    // Jump buffering
    if jump_pressed {
        jump_buffer = jump_buffer_time;
    }
    jump_buffer -= delta;
    
    // Execute jump
    if jump_buffer > 0.0 && (is_grounded || coyote_timer > 0.0) {
        velocity.y = jump_velocity;
        jump_buffer = 0.0;
        coyote_timer = 0.0;
    }
    
    // Variable jump height
    if jump_released && velocity.y > 0.0 {
        velocity.y *= jump_cut_multiplier;
    }
    
    // Apply gravity
    velocity.y -= gravity * delta;
    velocity.y = velocity.y.max(-terminal_velocity);
}
```

### 4.3 Wall Mechanics

**Features**:
- Wall detection (raycast horizontally)
- Wall slide (reduced gravity when touching wall)
- Wall jump (jump away from wall)

**Parameters**:
```wj
struct WallConfig {
    wall_slide_speed: f32,    // -100.0
    wall_jump_horizontal: f32, // 300.0
    wall_jump_vertical: f32,   // 400.0
    wall_stick_time: f32,      // 0.05s
}
```

---

## Sprint 5: Camera System

### 5.1 Camera Follow

**Files**: `src_wj/rendering/camera2d.wj`

**Algorithm** (Lerp-based):
```wj
fn follow(target: Vec2, lerp_factor: f32, delta: f32) {
    let desired_pos = target - Vec2::new(viewport_width / 2.0, viewport_height / 2.0);
    position = position.lerp(desired_pos, lerp_factor * delta * 60.0);
}
```

**Advanced** (Dead Zone):
```wj
struct CameraDeadZone {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

fn follow_with_deadzone(target: Vec2) {
    let target_screen = world_to_screen(target);
    
    if target_screen.x < deadzone.left {
        position.x += target_screen.x - deadzone.left;
    } else if target_screen.x > deadzone.right {
        position.x += target_screen.x - deadzone.right;
    }
    
    // Same for Y
}
```

### 5.2 Camera Bounds

**API**:
```wj
fn set_bounds(min: Vec2, max: Vec2);
fn clamp_to_bounds();
```

**Algorithm**:
```wj
fn clamp_to_bounds() {
    let half_width = viewport_width / 2.0;
    let half_height = viewport_height / 2.0;
    
    position.x = position.x.clamp(bounds_min.x + half_width, bounds_max.x - half_width);
    position.y = position.y.clamp(bounds_min.y + half_height, bounds_max.y - half_height);
}
```

---

## Sprint 6: Particles & Polish

### 6.1 Particle Emitter

**Files**: `src_wj/effects/particle_system.wj`

**Architecture**:
```
ParticleEmitter
├── particles: Vec<Particle>
├── max_particles: usize
├── emission_rate: f32  // Particles per second
├── particle_lifetime: f32
├── spawn_position: Vec2
├── spawn_velocity: Vec2Range
├── spawn_color: ColorRange
└── gravity: Vec2

Particle
├── position: Vec2
├── velocity: Vec2
├── color: Color
├── size: f32
├── lifetime: f32
└── age: f32
```

**Update**:
```wj
fn update(delta: f32) {
    // Emit new particles
    emission_accumulator += emission_rate * delta;
    while emission_accumulator >= 1.0 && particles.len() < max_particles {
        spawn_particle();
        emission_accumulator -= 1.0;
    }
    
    // Update existing particles
    for particle in particles {
        particle.velocity += gravity * delta;
        particle.position += particle.velocity * delta;
        particle.age += delta;
        
        if particle.age >= particle.lifetime {
            remove(particle);
        }
    }
}
```

### 6.2 Particle Rendering

**Optimization**: Batch all particles into single draw call

**Implementation**:
1. Create particle vertex buffer (instanced or batched)
2. Upload all particle data to GPU
3. Render in single draw call
4. Use additive blending for glowy effects

**Target**: 1000 particles at 60 FPS

---

## Sprint 7: Audio System

### 7.1 Audio Loading & Playback

**Files**: `src/ffi/audio.rs`

**Dependencies**: `rodio` crate

**Architecture**:
```
AudioManager
├── output_stream: rodio::OutputStream
├── stream_handle: rodio::OutputStreamHandle
├── sounds: HashMap<u32, Arc<Vec<u8>>>  // Cached audio data
├── playing_sounds: Vec<rodio::Sink>
└── music_sink: Option<rodio::Sink>
```

**API**:
```rust
pub fn audio_load(path: String) -> u32;
pub fn audio_play(handle: u32, volume: f32, looping: bool);
pub fn audio_stop(sound_id: u32);
pub fn audio_set_master_volume(volume: f32);
pub fn audio_set_music_volume(volume: f32);
pub fn audio_set_sfx_volume(volume: f32);
```

### 7.2 Spatial Audio (2D)

**API**:
```rust
pub fn audio_play_at_position(
    handle: u32,
    x: f32, y: f32,
    max_distance: f32,
    listener_x: f32, listener_y: f32
) -> u32;
```

**Algorithm**:
```rust
fn calculate_spatial_audio(emitter_pos: Vec2, listener_pos: Vec2) -> (f32, f32) {
    let diff = emitter_pos - listener_pos;
    let distance = diff.length();
    let direction = diff.normalize();
    
    // Volume attenuation
    let attenuation = (1.0 - (distance / max_distance).min(1.0)).max(0.0);
    let volume = base_volume * attenuation;
    
    // Stereo panning (-1.0 = left, 1.0 = right)
    let pan = direction.x.clamp(-1.0, 1.0);
    
    (volume, pan)
}
```

---

## Phase 2: Editor & Designer Tools

### Sprint 8: Visual Editor (windjammer-ui dogfooding)

**Goal**: Build Godot-style visual editor using windjammer-ui

**Components**:
1. **Scene Viewport** - Render game scene with pan/zoom
2. **Entity Hierarchy** - Tree view of scene entities
3. **Property Inspector** - Edit component properties
4. **Asset Browser** - Import/manage assets
5. **Tilemap Editor** - Visual tilemap editing
6. **Animation Timeline** - Visual animation editing

**Architecture**:
```
Editor (windjammer-ui app)
├── Viewport Panel
│   ├── Game Renderer (embedded)
│   ├── Gizmos (move/rotate/scale)
│   └── Grid Overlay
├── Hierarchy Panel
│   └── Tree<Entity>
├── Inspector Panel
│   └── PropertyGrid<Component>
├── Asset Browser
│   └── FileTree
└── Toolbar
    ├── Play/Pause/Stop
    ├── Transform Tools
    └── Scene Management
```

### Sprint 9: WindjammerScript Interpreter

**Architecture**:
```
Windjammer Interpreter
├── Parser (reuse compiler parser)
├── Type Checker (reuse compiler analyzer)
├── VM (bytecode or tree-walking)
└── FFI Bindings (call native functions)
```

**Execution Modes**:
1. **Dev Mode**: `wj run main.wj` → Interpreted, hot reload
2. **Prod Mode**: `wj build main.wj` → Compiled to Rust, optimized

**Hot Reload**:
```
1. File watcher detects change
2. Re-parse changed file
3. Update VM bytecode
4. Preserve game state
5. Resume execution
```

---

## 3D Extensions

### 3D Rendering Pipeline

**Components**:
1. **Mesh Rendering** - Load .obj/.gltf, vertex/index buffers
2. **Material System** - PBR shaders, textures, properties
3. **Lighting** - Point, directional, spot lights
4. **Shadows** - Shadow mapping, PCF filtering
5. **Camera 3D** - Perspective projection, FPS controls

**Existing Shaders**:
- `shader_3d.wgsl` - Basic 3D rendering
- `shader_3d_pbr.wgsl` - PBR materials
- `shader_shadow.wgsl` - Shadow mapping
- `shader_skinned.wgsl` - Skeletal animation
- `shader_terrain.wgsl` - Terrain rendering
- `shader_particles.wgsl` - 3D particles

**Architecture** (extends 2D):
```
WgpuRenderer
├── sprite_pipeline (2D)
├── mesh_pipeline (3D)
├── pbr_pipeline (3D PBR)
└── shadow_pipeline (3D shadows)
```

---

## Testing Strategy

### Unit Tests (Rust)
- FFI functions (`tests/texture_test_runner.rs`)
- Renderer methods (`tests/renderer_test.rs`)
- Math utilities (`tests/math_test.rs`)

### Integration Tests (Windjammer)
- Game loop (`tests_wj/game_loop_test.wj`)
- Physics simulation (`tests_wj/physics_test.wj`)
- Animation system (`tests_wj/animation_test.wj`)

### Dogfooding (Games)
- Breakout (basic 2D, completed)
- Platformer (advanced 2D, in progress)
- 3D FPS (3D rendering, planned)

---

## Success Criteria

### Phase 1 Complete (2D Engine MVP)
- ✅ Platformer runs at 60 FPS
- ✅ Sprites render with textures
- ✅ Animations play smoothly
- ✅ Tilemap levels load
- ✅ Character controller feels responsive
- ✅ Camera follows player
- ✅ Particle effects look good
- ✅ Audio plays correctly

### World-Class Status
- 2D capabilities = Godot 4
- 3D capabilities = Godot 4 (basic)
- Performance > Unity 2D
- Safety > all (compile-time checks)
- Simplicity > Bevy (auto-inference)
- Unique: WindjammerScript (interpreted + compiled)

---

**Next**: Implement sprite rendering, then systematically work through remaining features! 🚀
