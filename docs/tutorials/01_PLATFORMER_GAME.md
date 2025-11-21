# Tutorial: Building a 2D Platformer Game

## Overview

In this tutorial, you'll build a complete 2D platformer game from scratch using Windjammer. You'll learn:

- Setting up a game project
- Creating a player character with physics
- Implementing platformer controls (jump, run, double-jump)
- Building levels with tilemaps
- Adding enemies with AI
- Implementing collectibles and scoring
- Creating a complete game loop

**Estimated Time**: 60-90 minutes  
**Difficulty**: Beginner to Intermediate  
**Language**: Python (easily adaptable to other languages)

---

## Part 1: Project Setup (5 minutes)

### Step 1.1: Create Project Structure

```bash
mkdir my_platformer
cd my_platformer
mkdir assets
mkdir assets/sprites
mkdir assets/sounds
mkdir assets/levels
```

### Step 1.2: Create Main Game File

Create `main.py`:

```python
from windjammer_sdk import App, World, Vec2, Color

# Create the game application
app = App("My Platformer", 800, 600)

# Create the game world
world = World()

# Game state
class GameState:
    def __init__(self):
        self.score = 0
        self.lives = 3
        self.level = 1

game_state = GameState()

# Startup system
def startup(world: World):
    print("Game starting...")
    # We'll add initialization code here

# Update system
def update(world: World, delta: float):
    # We'll add game logic here
    pass

# Register systems
app.add_startup_system(startup)
app.add_system(update)

# Run the game
app.run()
```

### Step 1.3: Test the Setup

```bash
python main.py
```

You should see a black window with "Game starting..." in the console.

---

## Part 2: Creating the Player (15 minutes)

### Step 2.1: Define Player Component

Add to `main.py`:

```python
from dataclasses import dataclass

@dataclass
class Player:
    """Player component"""
    speed: float = 200.0
    jump_force: float = 400.0
    can_double_jump: bool = True
    has_double_jumped: bool = False
    is_grounded: bool = False

@dataclass
class Velocity:
    """Velocity component"""
    x: float = 0.0
    y: float = 0.0
```

### Step 2.2: Create Player Entity

Update the `startup` function:

```python
def startup(world: World):
    print("Game starting...")
    
    # Create player entity
    player = world.create_entity()
    
    # Add transform component
    world.add_component(player, Transform2D(
        position=Vec2(100, 300),
        scale=Vec2(32, 32)
    ))
    
    # Add sprite component
    world.add_component(player, Sprite(
        color=Color(0, 128, 255, 255),  # Blue player
        size=Vec2(32, 32)
    ))
    
    # Add physics components
    world.add_component(player, Player())
    world.add_component(player, Velocity())
    
    # Add physics body
    world.add_component(player, RigidBody2D(
        body_type=BodyType.Dynamic,
        gravity_scale=1.0
    ))
    
    # Add collider
    world.add_component(player, BoxCollider2D(
        size=Vec2(32, 32)
    ))
    
    print(f"Created player entity: {player}")
```

### Step 2.3: Implement Player Movement

Add a new system:

```python
def player_movement_system(world: World, delta: float):
    """Handle player input and movement"""
    
    # Query for player entities
    for entity in world.query([Player, Velocity, Transform2D]):
        player = world.get_component(entity, Player)
        velocity = world.get_component(entity, Velocity)
        transform = world.get_component(entity, Transform2D)
        
        # Horizontal movement
        if app.is_key_pressed(Key.A) or app.is_key_pressed(Key.Left):
            velocity.x = -player.speed
        elif app.is_key_pressed(Key.D) or app.is_key_pressed(Key.Right):
            velocity.x = player.speed
        else:
            velocity.x = 0.0
        
        # Jumping
        if app.is_key_just_pressed(Key.Space) or app.is_key_just_pressed(Key.W):
            if player.is_grounded:
                velocity.y = -player.jump_force
                player.is_grounded = False
                player.has_double_jumped = False
            elif player.can_double_jump and not player.has_double_jumped:
                velocity.y = -player.jump_force
                player.has_double_jumped = True
        
        # Apply velocity to position
        transform.position.x += velocity.x * delta
        transform.position.y += velocity.y * delta
        
        # Apply gravity
        if not player.is_grounded:
            velocity.y += 980.0 * delta  # Gravity acceleration
        
        # Update components
        world.update_component(entity, player)
        world.update_component(entity, velocity)
        world.update_component(entity, transform)

# Register the system
app.add_system(player_movement_system)
```

---

## Part 3: Building the Level (15 minutes)

### Step 3.1: Create Platform Component

```python
@dataclass
class Platform:
    """Platform component"""
    width: float
    height: float
```

### Step 3.2: Add Platforms to the Level

Update `startup`:

```python
def startup(world: World):
    # ... (previous player code)
    
    # Create platforms
    platforms = [
        # Ground platform
        {"pos": Vec2(400, 550), "size": Vec2(800, 50)},
        # Floating platforms
        {"pos": Vec2(200, 400), "size": Vec2(150, 20)},
        {"pos": Vec2(500, 350), "size": Vec2(150, 20)},
        {"pos": Vec2(300, 250), "size": Vec2(150, 20)},
        {"pos": Vec2(600, 200), "size": Vec2(150, 20)},
    ]
    
    for platform_data in platforms:
        platform_entity = world.create_entity()
        
        world.add_component(platform_entity, Transform2D(
            position=platform_data["pos"],
            scale=platform_data["size"]
        ))
        
        world.add_component(platform_entity, Sprite(
            color=Color(100, 100, 100, 255),  # Gray platforms
            size=platform_data["size"]
        ))
        
        world.add_component(platform_entity, Platform(
            width=platform_data["size"].x,
            height=platform_data["size"].y
        ))
        
        world.add_component(platform_entity, RigidBody2D(
            body_type=BodyType.Static
        ))
        
        world.add_component(platform_entity, BoxCollider2D(
            size=platform_data["size"]
        ))
```

### Step 3.3: Implement Collision Detection

```python
def collision_system(world: World, delta: float):
    """Handle collisions between player and platforms"""
    
    # Get player
    player_entities = list(world.query([Player, Transform2D, Velocity]))
    if not player_entities:
        return
    
    player_entity = player_entities[0]
    player = world.get_component(player_entity, Player)
    player_transform = world.get_component(player_entity, Transform2D)
    player_velocity = world.get_component(player_entity, Velocity)
    
    # Check collision with platforms
    player.is_grounded = False
    
    for platform_entity in world.query([Platform, Transform2D]):
        platform = world.get_component(platform_entity, Platform)
        platform_transform = world.get_component(platform_entity, Transform2D)
        
        # Simple AABB collision
        if (player_transform.position.x < platform_transform.position.x + platform.width and
            player_transform.position.x + 32 > platform_transform.position.x and
            player_transform.position.y < platform_transform.position.y + platform.height and
            player_transform.position.y + 32 > platform_transform.position.y):
            
            # Player is on top of platform
            if player_velocity.y > 0:
                player_transform.position.y = platform_transform.position.y - 32
                player_velocity.y = 0
                player.is_grounded = True
    
    # Update components
    world.update_component(player_entity, player)
    world.update_component(player_entity, player_transform)
    world.update_component(player_entity, player_velocity)

# Register the system
app.add_system(collision_system)
```

---

## Part 4: Adding Enemies (15 minutes)

### Step 4.1: Create Enemy Component

```python
@dataclass
class Enemy:
    """Enemy component"""
    speed: float = 50.0
    direction: float = 1.0  # 1 = right, -1 = left
    patrol_distance: float = 100.0
    start_x: float = 0.0
```

### Step 4.2: Spawn Enemies

Add to `startup`:

```python
def startup(world: World):
    # ... (previous code)
    
    # Create enemies
    enemy_positions = [
        Vec2(300, 370),
        Vec2(550, 320),
        Vec2(350, 220),
    ]
    
    for pos in enemy_positions:
        enemy = world.create_entity()
        
        world.add_component(enemy, Transform2D(
            position=pos,
            scale=Vec2(24, 24)
        ))
        
        world.add_component(enemy, Sprite(
            color=Color(255, 0, 0, 255),  # Red enemies
            size=Vec2(24, 24)
        ))
        
        world.add_component(enemy, Enemy(
            start_x=pos.x
        ))
        
        world.add_component(enemy, Velocity())
```

### Step 4.3: Implement Enemy AI

```python
def enemy_ai_system(world: World, delta: float):
    """Simple patrol AI for enemies"""
    
    for entity in world.query([Enemy, Transform2D, Velocity]):
        enemy = world.get_component(entity, Enemy)
        transform = world.get_component(entity, Transform2D)
        velocity = world.get_component(entity, Velocity)
        
        # Move in current direction
        velocity.x = enemy.speed * enemy.direction
        transform.position.x += velocity.x * delta
        
        # Check if reached patrol limit
        distance_from_start = abs(transform.position.x - enemy.start_x)
        if distance_from_start >= enemy.patrol_distance:
            enemy.direction *= -1  # Reverse direction
        
        # Update components
        world.update_component(entity, enemy)
        world.update_component(entity, transform)
        world.update_component(entity, velocity)

# Register the system
app.add_system(enemy_ai_system)
```

### Step 4.4: Implement Player-Enemy Collision

```python
def player_enemy_collision_system(world: World, delta: float):
    """Handle collisions between player and enemies"""
    
    # Get player
    player_entities = list(world.query([Player, Transform2D, Velocity]))
    if not player_entities:
        return
    
    player_entity = player_entities[0]
    player_transform = world.get_component(player_entity, Transform2D)
    player_velocity = world.get_component(player_entity, Velocity)
    
    # Check collision with enemies
    for enemy_entity in world.query([Enemy, Transform2D]):
        enemy_transform = world.get_component(enemy_entity, Transform2D)
        
        # AABB collision
        if (player_transform.position.x < enemy_transform.position.x + 24 and
            player_transform.position.x + 32 > enemy_transform.position.x and
            player_transform.position.y < enemy_transform.position.y + 24 and
            player_transform.position.y + 32 > enemy_transform.position.y):
            
            # Player is above enemy (stomping)
            if player_velocity.y > 0:
                world.destroy_entity(enemy_entity)
                player_velocity.y = -200  # Bounce
                game_state.score += 100
            else:
                # Player hit from side - lose life
                game_state.lives -= 1
                # Reset player position
                player_transform.position = Vec2(100, 300)
                player_velocity.x = 0
                player_velocity.y = 0
                
                if game_state.lives <= 0:
                    print("Game Over!")
                    app.quit()
    
    # Update player components
    world.update_component(player_entity, player_transform)
    world.update_component(player_entity, player_velocity)

# Register the system
app.add_system(player_enemy_collision_system)
```

---

## Part 5: Adding Collectibles (10 minutes)

### Step 5.1: Create Coin Component

```python
@dataclass
class Coin:
    """Collectible coin"""
    value: int = 10
    collected: bool = False
```

### Step 5.2: Spawn Coins

Add to `startup`:

```python
def startup(world: World):
    # ... (previous code)
    
    # Create coins
    coin_positions = [
        Vec2(250, 350),
        Vec2(550, 300),
        Vec2(350, 200),
        Vec2(650, 150),
    ]
    
    for pos in coin_positions:
        coin = world.create_entity()
        
        world.add_component(coin, Transform2D(
            position=pos,
            scale=Vec2(16, 16)
        ))
        
        world.add_component(coin, Sprite(
            color=Color(255, 215, 0, 255),  # Gold coins
            size=Vec2(16, 16)
        ))
        
        world.add_component(coin, Coin())
```

### Step 5.3: Implement Coin Collection

```python
def coin_collection_system(world: World, delta: float):
    """Handle coin collection"""
    
    # Get player
    player_entities = list(world.query([Player, Transform2D]))
    if not player_entities:
        return
    
    player_entity = player_entities[0]
    player_transform = world.get_component(player_entity, Transform2D)
    
    # Check collision with coins
    for coin_entity in world.query([Coin, Transform2D]):
        coin = world.get_component(coin_entity, Coin)
        coin_transform = world.get_component(coin_entity, Transform2D)
        
        if coin.collected:
            continue
        
        # AABB collision
        if (player_transform.position.x < coin_transform.position.x + 16 and
            player_transform.position.x + 32 > coin_transform.position.x and
            player_transform.position.y < coin_transform.position.y + 16 and
            player_transform.position.y + 32 > coin_transform.position.y):
            
            # Collect coin
            coin.collected = True
            game_state.score += coin.value
            world.destroy_entity(coin_entity)
            print(f"Coin collected! Score: {game_state.score}")

# Register the system
app.add_system(coin_collection_system)
```

---

## Part 6: Adding UI and Polish (10 minutes)

### Step 6.1: Create UI System

```python
def ui_system(world: World, delta: float):
    """Render game UI"""
    
    # Draw score
    app.draw_text(
        f"Score: {game_state.score}",
        Vec2(10, 10),
        24,
        Color(255, 255, 255, 255)
    )
    
    # Draw lives
    app.draw_text(
        f"Lives: {game_state.lives}",
        Vec2(10, 40),
        24,
        Color(255, 255, 255, 255)
    )
    
    # Draw level
    app.draw_text(
        f"Level: {game_state.level}",
        Vec2(10, 70),
        24,
        Color(255, 255, 255, 255)
    )

# Register the system
app.add_system(ui_system)
```

### Step 6.2: Add Camera Follow

```python
def camera_follow_system(world: World, delta: float):
    """Make camera follow player"""
    
    # Get player
    player_entities = list(world.query([Player, Transform2D]))
    if not player_entities:
        return
    
    player_entity = player_entities[0]
    player_transform = world.get_component(player_entity, Transform2D)
    
    # Get camera
    camera = app.get_camera()
    
    # Smooth follow
    target_x = player_transform.position.x - 400  # Center player
    target_y = player_transform.position.y - 300
    
    camera.position.x += (target_x - camera.position.x) * 0.1
    camera.position.y += (target_y - camera.position.y) * 0.1
    
    # Clamp camera to level bounds
    camera.position.x = max(0, min(camera.position.x, 800))
    camera.position.y = max(0, min(camera.position.y, 600))

# Register the system
app.add_system(camera_follow_system)
```

---

## Part 7: Complete Game Code

Here's the complete `main.py` with all systems integrated:

```python
from windjammer_sdk import (
    App, World, Vec2, Color, Transform2D, Sprite,
    RigidBody2D, BoxCollider2D, BodyType, Key
)
from dataclasses import dataclass

# Create the game application
app = App("My Platformer", 800, 600)
world = World()

# Game state
class GameState:
    def __init__(self):
        self.score = 0
        self.lives = 3
        self.level = 1

game_state = GameState()

# Components
@dataclass
class Player:
    speed: float = 200.0
    jump_force: float = 400.0
    can_double_jump: bool = True
    has_double_jumped: bool = False
    is_grounded: bool = False

@dataclass
class Velocity:
    x: float = 0.0
    y: float = 0.0

@dataclass
class Platform:
    width: float
    height: float

@dataclass
class Enemy:
    speed: float = 50.0
    direction: float = 1.0
    patrol_distance: float = 100.0
    start_x: float = 0.0

@dataclass
class Coin:
    value: int = 10
    collected: bool = False

# Systems (all the systems from above)
# ... (include all system functions here)

# Register all systems
app.add_startup_system(startup)
app.add_system(player_movement_system)
app.add_system(collision_system)
app.add_system(enemy_ai_system)
app.add_system(player_enemy_collision_system)
app.add_system(coin_collection_system)
app.add_system(camera_follow_system)
app.add_system(ui_system)

# Run the game
app.run()
```

---

## Next Steps

### Enhancements to Try:

1. **Add Sound Effects**
   - Jump sound
   - Coin collection sound
   - Enemy defeat sound

2. **Add Animations**
   - Player walk/jump animations
   - Enemy patrol animations
   - Coin spin animation

3. **Add More Levels**
   - Level transition system
   - Level loading from files
   - Different themes per level

4. **Add Power-ups**
   - Speed boost
   - Invincibility
   - Extra lives

5. **Add Particles**
   - Coin sparkle effect
   - Enemy defeat explosion
   - Jump dust particles

6. **Add Menus**
   - Main menu
   - Pause menu
   - Game over screen

---

## Troubleshooting

### Player Falls Through Platforms

**Problem**: Collision detection isn't working properly.

**Solution**: Make sure the collision system runs after the movement system. Check the order of `app.add_system()` calls.

### Player Moves Too Fast/Slow

**Problem**: Movement speed isn't right.

**Solution**: Adjust the `Player.speed` value. Try values between 100-300.

### Enemies Don't Patrol

**Problem**: Enemy AI isn't working.

**Solution**: Make sure `enemy_ai_system` is registered and the `patrol_distance` is set correctly.

---

## Conclusion

Congratulations! You've built a complete 2D platformer game with:

- ✅ Player movement and physics
- ✅ Platform collision detection
- ✅ Enemy AI with patrol behavior
- ✅ Collectible coins
- ✅ Score and lives system
- ✅ Camera following
- ✅ UI display

This tutorial covered the fundamentals of game development with Windjammer. You can now expand on this foundation to create your own unique platformer!

---

## Resources

- [Windjammer API Reference](../API_REFERENCE.md)
- [Cookbook](../COOKBOOK.md)
- [Next Tutorial: 3D FPS Game](02_FPS_GAME.md)
- [Community Forum](https://forum.windjammer.dev)

Happy game making! 🎮✨

