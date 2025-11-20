# Tutorial: Building a 3D First-Person Shooter

## Overview

In this tutorial, you'll build a complete 3D FPS game from scratch using Windjammer. You'll learn:

- Setting up a 3D game project
- Creating a first-person camera controller
- Implementing FPS movement (WASD + mouse look)
- Adding weapons and shooting mechanics
- Creating enemies with AI pathfinding
- Implementing health and damage systems
- Building a complete FPS experience

**Estimated Time**: 90-120 minutes  
**Difficulty**: Intermediate  
**Language**: Python (easily adaptable to other languages)

---

## Part 1: Project Setup (5 minutes)

### Step 1.1: Create Project Structure

```bash
mkdir fps_game
cd fps_game
mkdir assets
mkdir assets/models
mkdir assets/textures
mkdir assets/sounds
```

### Step 1.2: Create Main Game File

Create `main.py`:

```python
from windjammer_sdk import (
    App, World, Vec3, Quat, Color,
    Camera3D, Mesh, Material, PointLight
)

# Create the game application
app = App("FPS Game", 1280, 720)
world = World()

# Game state
class GameState:
    def __init__(self):
        self.score = 0
        self.ammo = 30
        self.health = 100
        self.enemies_killed = 0

game_state = GameState()

# Startup system
def startup(world: World):
    print("FPS Game starting...")
    setup_scene(world)

# Update system
def update(world: World, delta: float):
    pass

# Register systems
app.add_startup_system(startup)
app.add_system(update)

# Run the game
app.run()
```

---

## Part 2: First-Person Camera (15 minutes)

### Step 2.1: Create FPS Controller Component

```python
from dataclasses import dataclass

@dataclass
class FPSController:
    """First-person shooter controller"""
    move_speed: float = 5.0
    sprint_speed: float = 10.0
    mouse_sensitivity: float = 0.002
    yaw: float = 0.0  # Horizontal rotation
    pitch: float = 0.0  # Vertical rotation
    max_pitch: float = 1.5  # ~85 degrees
```

### Step 2.2: Setup Camera

```python
def setup_scene(world: World):
    """Initialize the game scene"""
    
    # Create player/camera entity
    player = world.create_entity()
    
    # Add transform
    world.add_component(player, Transform3D(
        position=Vec3(0, 1.7, 0),  # Eye height
        rotation=Quat.identity()
    ))
    
    # Add FPS controller
    world.add_component(player, FPSController())
    
    # Add camera
    camera = Camera3D(
        fov=70.0,
        near=0.1,
        far=1000.0
    )
    world.add_component(player, camera)
    app.set_active_camera(camera)
    
    # Add physics
    world.add_component(player, CharacterController3D(
        height=1.8,
        radius=0.3
    ))
    
    print(f"Created player entity: {player}")
    
    # Create ground
    create_ground(world)
    
    # Add lighting
    create_lighting(world)
```

### Step 2.3: Implement FPS Movement

```python
def fps_movement_system(world: World, delta: float):
    """Handle first-person movement and mouse look"""
    
    for entity in world.query([FPSController, Transform3D, CharacterController3D]):
        controller = world.get_component(entity, FPSController)
        transform = world.get_component(entity, Transform3D)
        char_controller = world.get_component(entity, CharacterController3D)
        
        # Mouse look
        mouse_delta = app.get_mouse_delta()
        controller.yaw -= mouse_delta.x * controller.mouse_sensitivity
        controller.pitch -= mouse_delta.y * controller.mouse_sensitivity
        
        # Clamp pitch to prevent over-rotation
        controller.pitch = max(-controller.max_pitch, 
                              min(controller.max_pitch, controller.pitch))
        
        # Apply rotation
        yaw_quat = Quat.from_axis_angle(Vec3(0, 1, 0), controller.yaw)
        pitch_quat = Quat.from_axis_angle(Vec3(1, 0, 0), controller.pitch)
        transform.rotation = yaw_quat * pitch_quat
        
        # Movement
        move_speed = controller.sprint_speed if app.is_key_pressed(Key.LeftShift) else controller.move_speed
        
        # Get forward and right vectors
        forward = transform.forward()
        right = transform.right()
        
        # Calculate movement direction
        move_dir = Vec3.zero()
        
        if app.is_key_pressed(Key.W):
            move_dir += forward
        if app.is_key_pressed(Key.S):
            move_dir -= forward
        if app.is_key_pressed(Key.A):
            move_dir -= right
        if app.is_key_pressed(Key.D):
            move_dir += right
        
        # Normalize and apply speed
        if move_dir.length() > 0:
            move_dir = move_dir.normalize() * move_speed * delta
            
        # Move character
        char_controller.move(move_dir)
        
        # Update components
        world.update_component(entity, controller)
        world.update_component(entity, transform)

# Register the system
app.add_system(fps_movement_system)

# Lock mouse cursor
app.set_cursor_locked(True)
```

---

## Part 3: Building the Level (15 minutes)

### Step 3.1: Create Ground

```python
def create_ground(world: World):
    """Create the ground plane"""
    
    ground = world.create_entity()
    
    # Transform
    world.add_component(ground, Transform3D(
        position=Vec3(0, 0, 0),
        scale=Vec3(50, 1, 50)
    ))
    
    # Mesh
    mesh = Mesh.create_plane(50, 50)
    world.add_component(ground, mesh)
    
    # Material
    material = Material(
        albedo=Color(100, 100, 100, 255),
        metallic=0.0,
        roughness=0.8
    )
    world.add_component(ground, material)
    
    # Physics
    world.add_component(ground, RigidBody3D(
        body_type=BodyType.Static
    ))
    
    world.add_component(ground, BoxCollider3D(
        size=Vec3(50, 1, 50)
    ))
```

### Step 3.2: Create Walls and Obstacles

```python
def create_walls(world: World):
    """Create walls and obstacles"""
    
    wall_positions = [
        # Outer walls
        (Vec3(-25, 2, 0), Vec3(1, 4, 50)),  # Left wall
        (Vec3(25, 2, 0), Vec3(1, 4, 50)),   # Right wall
        (Vec3(0, 2, -25), Vec3(50, 4, 1)),  # Back wall
        (Vec3(0, 2, 25), Vec3(50, 4, 1)),   # Front wall
        
        # Interior obstacles
        (Vec3(10, 1, 10), Vec3(2, 2, 2)),
        (Vec3(-10, 1, -10), Vec3(2, 2, 2)),
        (Vec3(15, 1, -15), Vec3(3, 2, 3)),
    ]
    
    for pos, scale in wall_positions:
        wall = world.create_entity()
        
        world.add_component(wall, Transform3D(
            position=pos,
            scale=scale
        ))
        
        mesh = Mesh.create_cube()
        world.add_component(wall, mesh)
        
        material = Material(
            albedo=Color(150, 150, 150, 255),
            metallic=0.0,
            roughness=0.7
        )
        world.add_component(wall, material)
        
        world.add_component(wall, RigidBody3D(
            body_type=BodyType.Static
        ))
        
        world.add_component(wall, BoxCollider3D(
            size=scale
        ))
```

### Step 3.3: Add Lighting

```python
def create_lighting(world: World):
    """Add lights to the scene"""
    
    # Main directional light (sun)
    sun = world.create_entity()
    world.add_component(sun, Transform3D(
        position=Vec3(0, 10, 0),
        rotation=Quat.from_euler(45, 45, 0)
    ))
    world.add_component(sun, DirectionalLight(
        color=Color(255, 255, 240, 255),
        intensity=1.0
    ))
    
    # Point lights
    light_positions = [
        Vec3(10, 3, 10),
        Vec3(-10, 3, -10),
        Vec3(10, 3, -10),
        Vec3(-10, 3, 10),
    ]
    
    for pos in light_positions:
        light = world.create_entity()
        world.add_component(light, Transform3D(position=pos))
        world.add_component(light, PointLight(
            color=Color(255, 200, 150, 255),
            intensity=2.0,
            range=15.0
        ))
```

---

## Part 4: Weapon System (20 minutes)

### Step 4.1: Create Weapon Component

```python
@dataclass
class Weapon:
    """Weapon component"""
    damage: int = 25
    fire_rate: float = 0.1  # Seconds between shots
    reload_time: float = 2.0
    magazine_size: int = 30
    current_ammo: int = 30
    time_since_last_shot: float = 0.0
    is_reloading: bool = False
    reload_timer: float = 0.0

@dataclass
class Bullet:
    """Bullet projectile"""
    speed: float = 50.0
    lifetime: float = 5.0
    damage: int = 25
    age: float = 0.0
```

### Step 4.2: Add Weapon to Player

Update `setup_scene`:

```python
def setup_scene(world: World):
    # ... (previous player setup)
    
    # Add weapon
    world.add_component(player, Weapon())
```

### Step 4.3: Implement Shooting

```python
def weapon_system(world: World, delta: float):
    """Handle weapon firing and reloading"""
    
    for entity in world.query([Weapon, FPSController, Transform3D]):
        weapon = world.get_component(entity, Weapon)
        transform = world.get_component(entity, Transform3D)
        
        # Update timers
        weapon.time_since_last_shot += delta
        
        # Handle reloading
        if weapon.is_reloading:
            weapon.reload_timer += delta
            if weapon.reload_timer >= weapon.reload_time:
                weapon.current_ammo = weapon.magazine_size
                weapon.is_reloading = False
                weapon.reload_timer = 0.0
                print("Reload complete!")
            continue
        
        # Reload input
        if app.is_key_just_pressed(Key.R) and weapon.current_ammo < weapon.magazine_size:
            weapon.is_reloading = True
            print("Reloading...")
            continue
        
        # Shooting
        if app.is_mouse_button_pressed(MouseButton.Left):
            if weapon.time_since_last_shot >= weapon.fire_rate and weapon.current_ammo > 0:
                # Fire bullet
                fire_bullet(world, transform.position, transform.forward(), weapon.damage)
                weapon.current_ammo -= 1
                weapon.time_since_last_shot = 0.0
                game_state.ammo = weapon.current_ammo
                
                # Auto-reload when empty
                if weapon.current_ammo == 0:
                    weapon.is_reloading = True
        
        # Update component
        world.update_component(entity, weapon)

def fire_bullet(world: World, position: Vec3, direction: Vec3, damage: int):
    """Create a bullet projectile"""
    
    bullet = world.create_entity()
    
    # Spawn slightly in front of player
    spawn_pos = position + direction * 0.5
    
    world.add_component(bullet, Transform3D(
        position=spawn_pos,
        scale=Vec3(0.1, 0.1, 0.5)
    ))
    
    world.add_component(bullet, Bullet(
        speed=50.0,
        damage=damage
    ))
    
    world.add_component(bullet, Velocity3D(
        velocity=direction * 50.0
    ))
    
    # Visual representation
    mesh = Mesh.create_sphere(0.1)
    world.add_component(bullet, mesh)
    
    material = Material(
        albedo=Color(255, 255, 0, 255),  # Yellow bullet
        emissive=Color(255, 255, 0, 255),
        emissive_strength=2.0
    ))
    world.add_component(bullet, material)

# Register the system
app.add_system(weapon_system)
```

### Step 4.4: Update Bullets

```python
def bullet_system(world: World, delta: float):
    """Update bullet movement and lifetime"""
    
    for entity in world.query([Bullet, Transform3D, Velocity3D]):
        bullet = world.get_component(entity, Bullet)
        transform = world.get_component(entity, Transform3D)
        velocity = world.get_component(entity, Velocity3D)
        
        # Update position
        transform.position += velocity.velocity * delta
        
        # Update lifetime
        bullet.age += delta
        if bullet.age >= bullet.lifetime:
            world.destroy_entity(entity)
            continue
        
        # Update components
        world.update_component(entity, bullet)
        world.update_component(entity, transform)

# Register the system
app.add_system(bullet_system)
```

---

## Part 5: Enemy AI (20 minutes)

### Step 5.1: Create Enemy Component

```python
@dataclass
class Enemy:
    """Enemy AI component"""
    health: int = 100
    max_health: int = 100
    move_speed: float = 3.0
    attack_range: float = 2.0
    detection_range: float = 20.0
    damage: int = 10
    attack_cooldown: float = 1.0
    time_since_attack: float = 0.0
    state: str = "patrol"  # patrol, chase, attack

@dataclass
class Velocity3D:
    """3D velocity component"""
    velocity: Vec3 = Vec3.zero()
```

### Step 5.2: Spawn Enemies

```python
def spawn_enemies(world: World):
    """Spawn enemy entities"""
    
    enemy_positions = [
        Vec3(15, 1, 15),
        Vec3(-15, 1, -15),
        Vec3(15, 1, -15),
        Vec3(-15, 1, 15),
        Vec3(0, 1, 20),
    ]
    
    for pos in enemy_positions:
        enemy = world.create_entity()
        
        world.add_component(enemy, Transform3D(
            position=pos,
            scale=Vec3(0.5, 1.8, 0.5)
        ))
        
        world.add_component(enemy, Enemy())
        world.add_component(enemy, Velocity3D())
        
        # Visual representation
        mesh = Mesh.create_capsule(0.5, 1.8)
        world.add_component(enemy, mesh)
        
        material = Material(
            albedo=Color(255, 0, 0, 255),  # Red enemies
            metallic=0.0,
            roughness=0.8
        )
        world.add_component(enemy, material)
        
        # Physics
        world.add_component(enemy, CharacterController3D(
            height=1.8,
            radius=0.5
        ))

# Call in setup_scene
def setup_scene(world: World):
    # ... (previous code)
    spawn_enemies(world)
```

### Step 5.3: Implement Enemy AI

```python
def enemy_ai_system(world: World, delta: float):
    """Enemy AI behavior"""
    
    # Find player
    player_entities = list(world.query([FPSController, Transform3D]))
    if not player_entities:
        return
    
    player_entity = player_entities[0]
    player_transform = world.get_component(player_entity, Transform3D)
    
    # Update each enemy
    for entity in world.query([Enemy, Transform3D, CharacterController3D]):
        enemy = world.get_component(entity, Enemy)
        transform = world.get_component(entity, Transform3D)
        char_controller = world.get_component(entity, CharacterController3D)
        
        # Calculate distance to player
        to_player = player_transform.position - transform.position
        distance = to_player.length()
        
        # Update attack cooldown
        enemy.time_since_attack += delta
        
        # State machine
        if distance <= enemy.attack_range:
            enemy.state = "attack"
        elif distance <= enemy.detection_range:
            enemy.state = "chase"
        else:
            enemy.state = "patrol"
        
        # Execute state behavior
        if enemy.state == "chase":
            # Move towards player
            direction = to_player.normalize()
            move = direction * enemy.move_speed * delta
            char_controller.move(move)
            
            # Face player
            look_dir = Vec3(to_player.x, 0, to_player.z).normalize()
            transform.rotation = Quat.look_rotation(look_dir, Vec3(0, 1, 0))
        
        elif enemy.state == "attack":
            # Attack player
            if enemy.time_since_attack >= enemy.attack_cooldown:
                # Deal damage to player
                game_state.health -= enemy.damage
                enemy.time_since_attack = 0.0
                print(f"Player hit! Health: {game_state.health}")
                
                if game_state.health <= 0:
                    print("Game Over!")
                    app.quit()
        
        # Update components
        world.update_component(entity, enemy)
        world.update_component(entity, transform)

# Register the system
app.add_system(enemy_ai_system)
```

### Step 5.4: Implement Bullet-Enemy Collision

```python
def bullet_enemy_collision_system(world: World, delta: float):
    """Handle bullet hits on enemies"""
    
    bullets_to_destroy = []
    enemies_to_destroy = []
    
    for bullet_entity in world.query([Bullet, Transform3D]):
        bullet = world.get_component(bullet_entity, Bullet)
        bullet_transform = world.get_component(bullet_entity, Transform3D)
        
        for enemy_entity in world.query([Enemy, Transform3D]):
            enemy = world.get_component(enemy_entity, Enemy)
            enemy_transform = world.get_component(enemy_entity, Transform3D)
            
            # Simple distance check
            distance = (bullet_transform.position - enemy_transform.position).length()
            
            if distance < 0.6:  # Hit radius
                # Damage enemy
                enemy.health -= bullet.damage
                bullets_to_destroy.append(bullet_entity)
                
                if enemy.health <= 0:
                    enemies_to_destroy.append(enemy_entity)
                    game_state.score += 100
                    game_state.enemies_killed += 1
                    print(f"Enemy killed! Score: {game_state.score}")
                else:
                    world.update_component(enemy_entity, enemy)
                
                break
    
    # Destroy bullets and enemies
    for entity in bullets_to_destroy:
        world.destroy_entity(entity)
    
    for entity in enemies_to_destroy:
        world.destroy_entity(entity)

# Register the system
app.add_system(bullet_enemy_collision_system)
```

---

## Part 6: UI and HUD (10 minutes)

### Step 6.1: Create HUD System

```python
def hud_system(world: World, delta: float):
    """Render game HUD"""
    
    # Health bar
    health_percent = game_state.health / 100.0
    health_color = Color(
        int(255 * (1 - health_percent)),
        int(255 * health_percent),
        0,
        255
    )
    
    app.draw_rect(
        Vec2(10, 10),
        Vec2(200 * health_percent, 20),
        health_color
    )
    
    app.draw_text(
        f"Health: {game_state.health}",
        Vec2(10, 35),
        20,
        Color(255, 255, 255, 255)
    )
    
    # Ammo counter
    app.draw_text(
        f"Ammo: {game_state.ammo}/30",
        Vec2(10, 60),
        20,
        Color(255, 255, 255, 255)
    )
    
    # Score
    app.draw_text(
        f"Score: {game_state.score}",
        Vec2(10, 85),
        20,
        Color(255, 255, 255, 255)
    )
    
    # Crosshair
    screen_center = Vec2(640, 360)
    crosshair_size = 10
    app.draw_line(
        Vec2(screen_center.x - crosshair_size, screen_center.y),
        Vec2(screen_center.x + crosshair_size, screen_center.y),
        Color(255, 255, 255, 200)
    )
    app.draw_line(
        Vec2(screen_center.x, screen_center.y - crosshair_size),
        Vec2(screen_center.x, screen_center.y + crosshair_size),
        Color(255, 255, 255, 200)
    )

# Register the system
app.add_system(hud_system)
```

---

## Part 7: Polish and Effects (10 minutes)

### Step 7.1: Add Muzzle Flash

```python
def create_muzzle_flash(world: World, position: Vec3, direction: Vec3):
    """Create a temporary muzzle flash effect"""
    
    flash = world.create_entity()
    
    world.add_component(flash, Transform3D(
        position=position + direction * 0.3,
        scale=Vec3(0.2, 0.2, 0.2)
    ))
    
    world.add_component(flash, PointLight(
        color=Color(255, 200, 100, 255),
        intensity=5.0,
        range=5.0
    ))
    
    # Add lifetime component
    world.add_component(flash, Lifetime(duration=0.1))
```

### Step 7.2: Add Hit Markers

```python
@dataclass
class HitMarker:
    """Hit marker UI element"""
    lifetime: float = 0.5
    age: float = 0.0

def show_hit_marker(world: World):
    """Show a hit marker on successful hit"""
    marker = world.create_entity()
    world.add_component(marker, HitMarker())

def hit_marker_system(world: World, delta: float):
    """Update and render hit markers"""
    
    for entity in world.query([HitMarker]):
        marker = world.get_component(entity, HitMarker)
        marker.age += delta
        
        if marker.age >= marker.lifetime:
            world.destroy_entity(entity)
        else:
            # Draw X marker
            alpha = int(255 * (1 - marker.age / marker.lifetime))
            color = Color(255, 255, 255, alpha)
            
            center = Vec2(640, 360)
            size = 20
            
            app.draw_line(
                Vec2(center.x - size, center.y - size),
                Vec2(center.x + size, center.y + size),
                color
            )
            app.draw_line(
                Vec2(center.x + size, center.y - size),
                Vec2(center.x - size, center.y + size),
                color
            )
        
        world.update_component(entity, marker)

# Register the system
app.add_system(hit_marker_system)
```

---

## Conclusion

Congratulations! You've built a complete 3D FPS game with:

- ✅ First-person camera and movement
- ✅ Mouse look controls
- ✅ Weapon system with shooting and reloading
- ✅ Enemy AI with pathfinding and combat
- ✅ Health and damage systems
- ✅ HUD and UI elements
- ✅ Visual effects (muzzle flash, hit markers)

This tutorial covered advanced 3D game development concepts. You can now expand on this to create your own unique FPS experience!

---

## Next Steps

### Enhancements to Try:

1. **Multiple Weapons**
   - Pistol, rifle, shotgun
   - Weapon switching system
   - Different fire rates and damage

2. **Advanced AI**
   - Cover system
   - Flanking behavior
   - Different enemy types

3. **More Levels**
   - Level loading system
   - Checkpoints
   - Boss battles

4. **Power-ups**
   - Health packs
   - Ammo crates
   - Temporary buffs

5. **Multiplayer**
   - Network synchronization
   - Player vs player
   - Team deathmatch

---

## Resources

- [Windjammer API Reference](../API_REFERENCE.md)
- [3D Rendering Guide](../3D_RENDERING.md)
- [Previous Tutorial: 2D Platformer](01_PLATFORMER_GAME.md)
- [Next Tutorial: RPG Game](03_RPG_GAME.md)

Happy game making! 🎮✨

