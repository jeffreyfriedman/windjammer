# 3D Doom-like Game Design

## Game Concept: "Greybox Shooter"

A simple first-person shooter inspired by Doom, using greyboxing (simple geometric shapes) for rapid prototyping and to fully exercise the Windjammer game framework.

---

## Core Features

### 1. **First-Person Movement**
- WASD movement (forward, back, strafe)
- Mouse look (pitch and yaw)
- Smooth camera control
- Collision detection with walls

### 2. **3D Rendering**
- Greybox environment (cubes, planes, simple geometry)
- Textured walls, floors, ceilings
- Basic lighting
- Skybox

### 3. **Physics**
- Gravity
- Collision detection (player vs walls)
- Projectile physics (bullets/rockets)
- Simple enemy AI movement

### 4. **Combat**
- Weapon system (pistol, shotgun, rocket launcher)
- Shooting mechanics
- Enemy AI (chase player, shoot back)
- Health system
- Damage numbers

### 5. **Level Design**
- Procedurally generated or hand-crafted maze
- Multiple rooms
- Doors (optional)
- Spawn points for enemies
- Pickups (health, ammo, weapons)

---

## Technical Architecture

### Decorators Used
```windjammer
@game           // Main game state
@init           // Initialize level, player, enemies
@update         // Game logic, AI, physics
@render3d       // 3D rendering
@input          // Mouse + keyboard input
@cleanup        // Cleanup resources
```

### Game State
```windjammer
@game
struct ShooterGame {
    // Player
    player_pos: Vec3,
    player_velocity: Vec3,
    player_yaw: float,
    player_pitch: float,
    player_health: int,
    player_weapon: int,  // 0=pistol, 1=shotgun, 2=rocket
    
    // Enemies
    enemies: Vec<Enemy>,
    
    // Projectiles
    bullets: Vec<Bullet>,
    
    // Level
    walls: Vec<Wall>,
    floor_y: float,
    
    // Game state
    score: int,
    ammo: int,
}

struct Enemy {
    pos: Vec3,
    health: int,
    state: int,  // 0=idle, 1=chase, 2=attack, 3=dead
}

struct Bullet {
    pos: Vec3,
    velocity: Vec3,
    damage: int,
}

struct Wall {
    pos: Vec3,
    size: Vec3,
    color: Color,
}
```

### Input Mapping
```windjammer
// Movement
W/S - Forward/Backward
A/D - Strafe Left/Right
Space - Jump
Shift - Sprint

// Combat
Mouse Left - Shoot
1/2/3 - Switch weapon
R - Reload

// Camera
Mouse - Look around
```

---

## Implementation Plan

### Phase 1: Basic 3D Rendering ✅
- [x] Set up 3D camera
- [x] Render simple cubes (walls)
- [x] Render floor plane
- [x] Basic lighting

### Phase 2: First-Person Controller
- [ ] WASD movement
- [ ] Mouse look (yaw/pitch)
- [ ] Collision detection
- [ ] Gravity

### Phase 3: Combat System
- [ ] Weapon switching
- [ ] Shooting mechanics
- [ ] Bullet physics
- [ ] Hit detection

### Phase 4: Enemy AI
- [ ] Spawn enemies
- [ ] Chase player
- [ ] Attack player
- [ ] Take damage/die

### Phase 5: Polish
- [ ] Health/ammo UI
- [ ] Damage numbers
- [ ] Sound effects
- [ ] Particle effects

---

## Windjammer Philosophy Validation

### Zero Crate Leakage ✅
- No `wgpu`, `winit`, or `nalgebra` types in game code
- All rendering through `Renderer3D` API
- All input through `Input` API
- All math through `Vec3`, `Mat4`, `Quat`

### Automatic Ownership ✅
- No `&`, `&mut`, or `mut` in user code
- Decorators handle ownership automatically
- `@update` gets `&mut game`
- `@render3d` gets `&game` and `&mut renderer`

### Simple, Declarative API ✅
```windjammer
@render3d
fn render(game: ShooterGame, renderer: Renderer3D) {
    renderer.clear(Color::black())
    
    // Draw walls
    for wall in game.walls {
        renderer.draw_cube(wall.pos, wall.size, wall.color)
    }
    
    // Draw enemies
    for enemy in game.enemies {
        renderer.draw_cube(enemy.pos, Vec3::new(1.0, 2.0, 1.0), Color::red())
    }
}
```

---

## File Structure

```
examples/games/shooter/
├── main.wj              # Main game file with decorators
├── level.wj             # Level generation/loading
├── enemy.wj             # Enemy AI logic
└── weapons.wj           # Weapon system
```

---

## Greybox Aesthetic

The game uses simple geometric shapes with solid colors:
- **Walls**: Grey cubes
- **Floor**: Dark grey plane
- **Ceiling**: Light grey plane
- **Player**: Green cube (for debugging)
- **Enemies**: Red cubes
- **Bullets**: Yellow spheres
- **Pickups**: Blue/green cubes

This "programmer art" aesthetic:
1. Focuses on gameplay mechanics
2. Exercises the 3D rendering pipeline
3. Is easy to iterate on
4. Looks intentionally minimalist (not "bad graphics")

---

## Success Criteria

✅ **Functional**:
- Player can move and look around
- Player can shoot enemies
- Enemies chase and attack player
- Collision detection works
- Game has win/lose conditions

✅ **Philosophy**:
- Zero Rust types in game code
- Automatic ownership inference
- Simple, declarative API
- Ergonomic input handling

✅ **Framework Exercise**:
- 3D rendering
- Physics simulation
- Input handling (mouse + keyboard)
- Game state management
- Multiple entities
- Collision detection

---

## Next Steps

1. Create `Renderer3D` API (similar to `Renderer` for 2D)
2. Implement 3D camera system
3. Create basic level with walls
4. Implement first-person controller
5. Add shooting mechanics
6. Implement enemy AI
7. Polish and playtest

Let's build it! 🎮

