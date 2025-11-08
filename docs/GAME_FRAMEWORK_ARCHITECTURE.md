# 🎮 Game Framework Architecture

**Goal:** Extensible architecture that supports 2D, 3D, physics, networking, and future features

**Design Principle:** Start simple (2D), but architect for growth (3D, physics, etc.)

---

## 🏗️ **Core Architecture**

### **Decorator-Based System (Extensible)**

```windjammer
// 2D Game (Simple)
@game
struct SimpleGame { }

@update
fn update(game: SimpleGame, delta: float) { }

@render
fn render(game: SimpleGame, renderer: Renderer) { }

// 3D Game (Extended)
@game
struct Game3D { }

@update
fn update(game: Game3D, delta: float) { }

@render3d
fn render(game: Game3D, renderer: Renderer3D, camera: Camera3D) { }

// Physics Game (Extended)
@game
struct PhysicsGame { }

@physics
fn physics(game: PhysicsGame, physics: PhysicsWorld) { }

@update
fn update(game: PhysicsGame, delta: float) { }

// Networked Game (Extended)
@game
struct MultiplayerGame { }

@network
fn network(game: MultiplayerGame, net: NetworkManager) { }

@update
fn update(game: MultiplayerGame, delta: float) { }
```

**Key Insight:** Decorators are **additive** - you can mix and match!

---

## 📦 **Module Structure**

### **Current (2D Foundation)**
```
std/game/
  ├── types.wj          # Vec2, Color, Rect
  ├── ecs.wj            # Entity-Component-System
  ├── input.wj          # Keyboard, Mouse
  ├── renderer.wj       # 2D rendering
  ├── runner.wj         # Game loop
  └── mod.wj            # Re-exports
```

### **Future (3D Extension)**
```
std/game/
  ├── types.wj          # Vec2, Color, Rect
  ├── types3d.wj        # Vec3, Vec4, Quat, Mat4 (NEW)
  ├── ecs.wj            # Entity-Component-System
  ├── input.wj          # Keyboard, Mouse
  ├── renderer.wj       # 2D rendering
  ├── renderer3d.wj     # 3D rendering (NEW)
  ├── camera.wj         # Camera2D
  ├── camera3d.wj       # Camera3D (NEW)
  ├── physics.wj        # 2D physics (NEW)
  ├── physics3d.wj      # 3D physics (NEW)
  ├── audio.wj          # Audio system (NEW)
  ├── network.wj        # Networking (NEW)
  ├── runner.wj         # Game loop
  └── mod.wj            # Re-exports
```

---

## 🎯 **Decorator System (Extensible)**

### **Core Decorators (Phase 1 - 2D)**
```windjammer
@game           // Marks game state struct
@init           // Initialize game (called once)
@update         // Update logic (called every frame)
@render         // Render 2D (called every frame)
@input          // Handle input events
@cleanup        // Cleanup on shutdown
```

### **Extended Decorators (Phase 2 - 3D)**
```windjammer
@render3d       // Render 3D (instead of @render)
@camera         // Setup camera
@lighting       // Setup lighting
@shadows        // Shadow rendering pass
```

### **Physics Decorators (Phase 3)**
```windjammer
@physics        // Physics simulation step
@collision      // Collision handling
@trigger        // Trigger events
```

### **Advanced Decorators (Phase 4)**
```windjammer
@network        // Network sync
@ai             // AI update
@audio          // Audio mixing
@particle       // Particle systems
@animation      // Animation update
```

**Key Design:** Each decorator is **optional** and **composable**

---

## 🔧 **Renderer Architecture (2D → 3D)**

### **2D Renderer (Phase 1)**
```windjammer
struct Renderer {
    // Hidden: wgpu 2D pipeline
}

impl Renderer {
    fn clear(color: Color)
    fn draw_rect(x: float, y: float, w: float, h: float, color: Color)
    fn draw_circle(x: float, y: float, radius: float, color: Color)
    fn draw_sprite(sprite: Sprite)
    fn draw_text(text: string, x: float, y: float, size: float, color: Color)
}
```

### **3D Renderer (Phase 2)**
```windjammer
struct Renderer3D {
    // Hidden: wgpu 3D pipeline
}

impl Renderer3D {
    fn clear(color: Color)
    fn draw_mesh(mesh: Mesh, transform: Transform3D, material: Material)
    fn draw_model(model: Model, transform: Transform3D)
    fn draw_skybox(skybox: Skybox)
    fn draw_particles(emitter: ParticleEmitter)
}
```

### **Unified Renderer (Phase 3 - Optional)**
```windjammer
struct Renderer {
    mode: RenderMode,  // 2D or 3D
}

impl Renderer {
    // 2D methods
    fn draw_rect(...)
    fn draw_sprite(...)
    
    // 3D methods
    fn draw_mesh(...)
    fn draw_model(...)
}
```

**Key Design:** Separate renderers initially, unified later if needed

---

## 🎮 **ECS Architecture (Scalable)**

### **Current ECS (Simple)**
```windjammer
struct World {
    entities: Vec<Entity>,
    components: HashMap<TypeId, Vec<Component>>,
}

impl World {
    fn spawn() -> EntityBuilder
    fn get<T>(entity: Entity) -> Option<T>
    fn query<T>() -> Vec<(Entity, T)>
}
```

### **Extended ECS (3D + Physics)**
```windjammer
// Components are just structs
struct Transform2D {
    position: Vec2,
    rotation: float,
    scale: Vec2,
}

struct Transform3D {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}

struct RigidBody {
    velocity: Vec3,
    angular_velocity: Vec3,
    mass: float,
}

struct Collider {
    shape: ColliderShape,
    is_trigger: bool,
}

// Usage
@game
struct Game3D {
    world: World,
}

@init
fn init(game: Game3D) {
    game.world.spawn()
        .with(Transform3D::new(Vec3::zero()))
        .with(Mesh::cube())
        .with(RigidBody::dynamic(1.0))
        .with(Collider::box(1.0, 1.0, 1.0))
        .build()
}
```

**Key Design:** Components are just data, systems process them

---

## 🌍 **Physics Integration**

### **2D Physics (Phase 3)**
```windjammer
use game::physics::*

@game
struct PhysicsGame {
    world: World,
    physics: PhysicsWorld2D,
}

@init
fn init(game: PhysicsGame) {
    game.physics = PhysicsWorld2D::new(Vec2::new(0.0, -9.81))
    
    // Spawn physics entity
    let entity = game.world.spawn()
        .with(Transform2D::new(Vec2::zero()))
        .with(RigidBody2D::dynamic(1.0))
        .with(Collider2D::circle(0.5))
        .build()
    
    game.physics.add_entity(entity)
}

@physics
fn physics(game: PhysicsGame, delta: float) {
    game.physics.step(delta)
    
    // Sync physics to transforms
    for (entity, body) in game.world.query2::<Transform2D, RigidBody2D>() {
        let pos = game.physics.get_position(entity)
        body.position = pos
    }
}

@update
fn update(game: PhysicsGame, delta: float) {
    // Game logic
}
```

### **3D Physics (Phase 4)**
```windjammer
@game
struct Game3D {
    world: World,
    physics: PhysicsWorld3D,
}

@physics
fn physics(game: Game3D, delta: float) {
    game.physics.step(delta)
    
    // Sync physics to transforms
    for (entity, transform, body) in game.world.query3::<Transform3D, RigidBody3D, Collider3D>() {
        transform.position = game.physics.get_position(entity)
        transform.rotation = game.physics.get_rotation(entity)
    }
}
```

**Key Design:** Physics is a separate system that syncs with ECS

---

## 🎨 **Material System (3D)**

### **Simple Materials (Phase 2)**
```windjammer
struct Material {
    color: Color,
    texture: Texture,
    shininess: float,
}

@render3d
fn render(game: Game3D, renderer: Renderer3D) {
    for (entity, mesh, material) in game.world.query3::<Transform3D, Mesh, Material>() {
        renderer.draw_mesh(mesh, entity.transform, material)
    }
}
```

### **PBR Materials (Phase 3)**
```windjammer
struct PBRMaterial {
    albedo: Color,
    metallic: float,
    roughness: float,
    normal_map: Texture,
    ao_map: Texture,
}
```

---

## 📡 **Networking (Future)**

### **Client-Server Architecture**
```windjammer
@game
struct MultiplayerGame {
    world: World,
    network: NetworkManager,
    is_server: bool,
}

@network
fn network(game: MultiplayerGame, delta: float) {
    if game.is_server {
        // Server: Send state to clients
        for (entity, transform) in game.world.query::<Transform3D>() {
            game.network.broadcast(NetworkMessage::EntityUpdate {
                entity: entity,
                position: transform.position,
            })
        }
    } else {
        // Client: Receive state from server
        for msg in game.network.receive() {
            match msg {
                NetworkMessage::EntityUpdate { entity, position } => {
                    if let Some(transform) = game.world.get_mut::<Transform3D>(entity) {
                        transform.position = position
                    }
                }
            }
        }
    }
}
```

---

## 🎵 **Audio System (Future)**

```windjammer
@game
struct GameWithAudio {
    audio: AudioManager,
}

@init
fn init(game: GameWithAudio) {
    game.audio.load_sound("jump", "assets/jump.wav")
    game.audio.load_music("bgm", "assets/music.ogg")
    game.audio.play_music("bgm", true)  // loop
}

@update
fn update(game: GameWithAudio, delta: float) {
    if input.key_pressed(Key::Space) {
        game.audio.play_sound("jump")
    }
}
```

---

## 🔌 **Backend Abstraction**

### **Rendering Backends**
```rust
// Internal (hidden from user)
trait RenderBackend {
    fn create_window(&mut self, config: WindowConfig);
    fn clear(&mut self, color: Color);
    fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color);
    fn present(&mut self);
}

struct WgpuBackend { /* wgpu implementation */ }
struct OpenGLBackend { /* OpenGL implementation */ }
struct VulkanBackend { /* Vulkan implementation */ }
```

**User never sees this - they just use `Renderer`**

### **Physics Backends**
```rust
// Internal (hidden from user)
trait PhysicsBackend {
    fn step(&mut self, delta: f32);
    fn add_body(&mut self, body: RigidBodyDesc);
    fn get_position(&self, handle: BodyHandle) -> Vec3;
}

struct RapierBackend { /* rapier2d/rapier3d */ }
struct BulletBackend { /* bullet3 */ }
struct CustomBackend { /* custom physics */ }
```

**User never sees this - they just use `PhysicsWorld`**

---

## 📊 **Comparison: 2D vs 3D**

| Feature | 2D (Phase 1) | 3D (Phase 2+) |
|---------|--------------|---------------|
| **Vectors** | `Vec2` | `Vec3`, `Vec4` |
| **Transform** | `Transform2D` | `Transform3D` |
| **Rotation** | `float` (angle) | `Quat` |
| **Camera** | `Camera2D` | `Camera3D` |
| **Renderer** | `Renderer` | `Renderer3D` |
| **Physics** | `PhysicsWorld2D` | `PhysicsWorld3D` |
| **Colliders** | Circle, Rect | Box, Sphere, Mesh |
| **Lighting** | N/A | Point, Directional, Spot |
| **Materials** | Color, Texture | PBR (albedo, metallic, roughness) |

---

## ✅ **Design Principles**

### **1. Start Simple, Scale Up**
- Phase 1: 2D only
- Phase 2: Add 3D
- Phase 3: Add physics
- Phase 4: Add networking

### **2. Decorators are Additive**
- `@render` for 2D
- `@render3d` for 3D
- `@physics` for physics
- Mix and match as needed

### **3. Hide Implementation**
- User sees `Renderer`, not `wgpu`
- User sees `PhysicsWorld`, not `rapier`
- Backends are swappable

### **4. ECS is the Foundation**
- All game objects are entities
- Components are data
- Systems process components
- Scales to thousands of entities

### **5. Pure Windjammer**
- No `&` or `&mut` in user code
- No crate exposure
- Compiler infers everything
- Just game logic

---

## 🎯 **Implementation Priority**

### **Phase 1: 2D Foundation** (Current)
1. ✅ Decorators: `@game`, `@init`, `@update`, `@render`, `@input`
2. ✅ Types: `Vec2`, `Color`, `Rect`
3. ✅ Renderer: 2D primitives
4. ✅ Input: Keyboard, Mouse
5. ✅ ECS: Basic entity-component system

### **Phase 2: 3D Extension** (Future)
1. Types: `Vec3`, `Vec4`, `Quat`, `Mat4`
2. Decorators: `@render3d`, `@camera`
3. Renderer: 3D meshes, materials
4. Camera: Perspective, orthographic
5. Lighting: Point, directional, spot

### **Phase 3: Physics** (Future)
1. Decorators: `@physics`, `@collision`
2. Types: `RigidBody`, `Collider`
3. Physics: 2D and 3D
4. Collision: Detection and response

### **Phase 4: Advanced** (Future)
1. Networking: Client-server
2. Audio: Spatial audio
3. Particles: GPU particles
4. Animation: Skeletal animation

---

## 🚀 **Why This Architecture Works**

1. **Extensible** - Add features without breaking existing code
2. **Simple** - Start with 2D, add complexity as needed
3. **Composable** - Mix decorators as needed
4. **Scalable** - ECS handles thousands of entities
5. **Maintainable** - Clear separation of concerns
6. **Testable** - Each system can be tested independently

**This architecture supports everything from simple 2D games to complex 3D multiplayer games with physics.**

---

**Ready to implement Phase 1 (2D Foundation) with this architecture in mind!**

