# 🏗️ ECS Architecture - World-Class Design

## Vision: Performance + Ergonomics

**Goal**: Build an ECS that rivals Unity DOTS and Bevy, but with pure Windjammer API.

**Inspiration**:
- Unity DOTS (performance)
- Bevy (ergonomics)
- Flecs (features)
- EnTT (speed)

---

## 🎯 Core Design Principles

### 1. **Cache-Friendly Storage**
- Sparse sets for component storage
- Archetype-based storage for iteration
- SOA (Structure of Arrays) layout

### 2. **Parallel by Default**
- Systems run in parallel automatically
- Dependency graph for ordering
- No data races (compile-time checked)

### 3. **Zero-Cost Abstractions**
- Queries compile to tight loops
- No virtual dispatch
- Inline everything

### 4. **Pure Windjammer API**
```windjammer
// User writes THIS
let player = world.spawn()
    .with(Transform::at(0, 0, 0))
    .with(Mesh::cube())
    .with(RigidBody::dynamic())

// NOT this (Rust exposure)
let player = commands.spawn()
    .insert(Transform { ... })
    .insert(Mesh { ... });
```

---

## 🏗️ Architecture

### Core Types

```rust
// Entity: Just an ID (u64)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,      // Index in entity array
    generation: u32, // Generation for reuse detection
}

// World: The ECS container
pub struct World {
    entities: EntityAllocator,
    components: ComponentStorage,
    resources: ResourceStorage,
    systems: SystemScheduler,
    archetypes: ArchetypeStorage,
}

// Component: Marker trait
pub trait Component: Send + Sync + 'static {}

// System: Function that operates on components
pub trait System: Send + Sync {
    fn run(&mut self, world: &mut World);
}

// Query: Efficient component iteration
pub struct Query<'w, Q: QueryData> {
    world: &'w World,
    _marker: PhantomData<Q>,
}
```

---

## 📦 Component Storage

### Sparse Set Storage (Fast Add/Remove)

```rust
pub struct SparseSet<T> {
    sparse: Vec<Option<usize>>,  // Entity ID -> dense index
    dense: Vec<Entity>,           // Dense array of entities
    data: Vec<T>,                 // Dense array of components
}
```

**Performance**:
- Add: O(1)
- Remove: O(1)
- Get: O(1)
- Iteration: Cache-friendly (dense array)

### Archetype Storage (Fast Iteration)

```rust
pub struct Archetype {
    entities: Vec<Entity>,
    components: HashMap<TypeId, Box<dyn ComponentArray>>,
}

pub struct ArchetypeStorage {
    archetypes: Vec<Archetype>,
    entity_to_archetype: HashMap<Entity, ArchetypeId>,
}
```

**Benefits**:
- Iteration over entities with same components is blazing fast
- Moving entities between archetypes is expensive but rare
- Perfect for systems that iterate many entities

### Hybrid Approach (Best of Both)

```rust
pub struct ComponentStorage {
    sparse_sets: HashMap<TypeId, Box<dyn SparseSetTrait>>,
    archetypes: ArchetypeStorage,
    strategy: StorageStrategy,
}

enum StorageStrategy {
    SparseSets,   // Fast add/remove (use for frequently added/removed)
    Archetypes,   // Fast iteration (use for stable components)
    Hybrid,       // Automatic based on usage patterns
}
```

---

## 🔍 Query System

### Query Types

```rust
// Read-only query
Query<&Transform>

// Mutable query
Query<&mut Transform>

// Multiple components
Query<(&Transform, &mut Velocity)>

// With filters
Query<(&Transform, &Velocity), With<Player>>

// Without filters
Query<(&Transform, &Velocity), Without<Enemy>>

// Optional components
Query<(&Transform, Option<&Velocity>)>

// Changed detection
Query<(&Transform, &Velocity), Changed<Transform>>
```

### Query Implementation

```rust
pub trait QueryData {
    type Item<'w>;
    
    fn fetch<'w>(world: &'w World, entity: Entity) -> Option<Self::Item<'w>>;
    fn matches(world: &World, entity: Entity) -> bool;
}

impl<'w> QueryData for &'w Transform {
    type Item<'w> = &'w Transform;
    
    fn fetch<'w>(world: &'w World, entity: Entity) -> Option<Self::Item<'w>> {
        world.get_component::<Transform>(entity)
    }
    
    fn matches(world: &World, entity: Entity) -> bool {
        world.has_component::<Transform>(entity)
    }
}

// Query iteration
impl<'w, Q: QueryData> Query<'w, Q> {
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Q::Item<'_>)> {
        self.world.entities()
            .filter(|&e| Q::matches(self.world, e))
            .filter_map(|e| Q::fetch(self.world, e).map(|item| (e, item)))
    }
}
```

---

## ⚙️ System Scheduling

### System Types

```rust
// Simple system
fn move_system(query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (transform, velocity) in query.iter() {
        transform.position += velocity.value * time.delta();
    }
}

// System with commands
fn spawn_system(commands: Commands, time: Res<Time>) {
    if time.elapsed() > 1.0 {
        commands.spawn()
            .with(Transform::default())
            .with(Mesh::cube());
    }
}

// System with events
fn input_system(events: EventReader<KeyPressed>, query: Query<&mut Player>) {
    for event in events.read() {
        if event.key == Key::Space {
            // Handle jump
        }
    }
}
```

### Parallel Execution

```rust
pub struct SystemScheduler {
    systems: Vec<Box<dyn System>>,
    dependency_graph: DependencyGraph,
    stages: Vec<SystemStage>,
}

impl SystemScheduler {
    pub fn run(&mut self, world: &mut World) {
        for stage in &mut self.stages {
            // Run systems in parallel within each stage
            stage.systems.par_iter_mut()
                .for_each(|system| system.run(world));
        }
    }
    
    fn build_dependency_graph(&mut self) {
        // Analyze system access patterns
        // Systems that access disjoint components can run in parallel
        // Systems that access same components must run sequentially
    }
}
```

---

## 🌳 Scene Graph Integration

### Transform Hierarchy

```rust
#[derive(Component)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    
    // Cached world transform
    world_matrix: Mat4,
    dirty: bool,
}

#[derive(Component)]
pub struct Parent(Entity);

#[derive(Component)]
pub struct Children(Vec<Entity>);

// System to update transform hierarchy
fn transform_propagation_system(
    query: Query<(Entity, &Transform, Option<&Parent>, Option<&Children>)>
) {
    // 1. Update all root transforms (no parent)
    // 2. Propagate to children recursively
    // 3. Cache world matrices
}
```

---

## 🎮 Windjammer API Design

### Pure Windjammer Code

```windjammer
use std::game::*

// Define components (automatically implements Component trait)
struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}

struct Velocity {
    value: Vec3,
}

struct Player {
    health: int,
    score: int,
}

// Define systems using decorators
@system
fn move_system(query: Query<(Transform, Velocity)>, time: Time) {
    for (transform, velocity) in query {
        transform.position += velocity.value * time.delta
    }
}

@system
fn player_input_system(query: Query<(Player, Transform)>, input: Input) {
    for (player, transform) in query {
        if input.key_down(Key::W) {
            transform.position.z += 5.0 * time.delta
        }
    }
}

// Game initialization
@game
struct MyGame {
    world: World,
}

@init
fn init(game: MyGame) {
    // Spawn player
    let player = game.world.spawn()
        .with(Transform::at(0, 0, 0))
        .with(Velocity::new(0, 0, 0))
        .with(Player { health: 100, score: 0 })
        .with(Mesh::from_asset("player.glb"))
    
    // Spawn enemies
    for i in 0..10 {
        game.world.spawn()
            .with(Transform::at(i * 5, 0, 0))
            .with(Enemy { ai_state: AIState::Patrol })
            .with(Mesh::from_asset("enemy.glb"))
    }
    
    // Register systems
    game.world.add_system(move_system)
    game.world.add_system(player_input_system)
}

@update
fn update(game: MyGame, delta: float) {
    // Run all systems
    game.world.run_systems(delta)
}
```

### Generated Rust Code

```rust
// Components (generated from Windjammer structs)
#[derive(Component)]
struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Component)]
struct Velocity {
    value: Vec3,
}

// Systems (generated from @system functions)
fn move_system(
    query: Query<(&mut Transform, &Velocity)>,
    time: Res<Time>
) {
    for (mut transform, velocity) in query.iter() {
        transform.position += velocity.value * time.delta();
    }
}

// Game struct
struct MyGame {
    world: World,
}

// Generated main
fn main() {
    let mut game = MyGame {
        world: World::new(),
    };
    
    init(&mut game);
    
    // Game loop
    loop {
        update(&mut game, delta);
    }
}
```

---

## 🚀 Performance Targets

### Benchmarks (Target)

| Operation | Target | Competitive With |
|-----------|--------|------------------|
| **Entity Spawn** | < 100ns | Unity DOTS |
| **Component Add** | < 50ns | Bevy |
| **Query Iteration** | 1M entities/ms | EnTT |
| **System Scheduling** | < 1ms overhead | Unity DOTS |
| **Transform Propagation** | 10K entities < 1ms | Godot |

### Memory Efficiency

- Entity: 8 bytes (u64)
- Component overhead: 16 bytes (TypeId + pointer)
- Archetype storage: 0 bytes per entity (SOA layout)

---

## 🎯 Implementation Plan

### Phase 1: Core ECS (Week 1)
1. Entity allocator with generations
2. Sparse set component storage
3. Basic query system
4. Simple system execution

**Deliverable**: 10,000 entities with Transform + Velocity

### Phase 2: Advanced Queries (Week 1)
1. Multi-component queries
2. Filters (With/Without)
3. Optional components
4. Changed detection

**Deliverable**: Complex queries with filters

### Phase 3: Parallel Systems (Week 2)
1. Dependency graph analysis
2. Parallel system execution
3. System stages
4. Commands for deferred operations

**Deliverable**: Systems running in parallel

### Phase 4: Scene Graph (Week 2)
1. Transform hierarchy
2. Parent-child relationships
3. Transform propagation
4. Scene serialization

**Deliverable**: Hierarchical scene with 1000+ nodes

---

## 📚 References

- **Unity DOTS**: https://unity.com/dots
- **Bevy ECS**: https://bevyengine.org/learn/book/getting-started/ecs/
- **Flecs**: https://github.com/SanderMertens/flecs
- **EnTT**: https://github.com/skypjack/entt
- **Specs**: https://github.com/amethyst/specs

---

## ✅ Success Criteria

1. **Performance**: 60 FPS with 10,000+ entities
2. **Ergonomics**: Pure Windjammer API (no Rust exposure)
3. **Features**: Queries, filters, parallel systems, scene graph
4. **Quality**: Production-ready, well-tested, documented

**Let's build the best ECS in the industry!** 🚀

