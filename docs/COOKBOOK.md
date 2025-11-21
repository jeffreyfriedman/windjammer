# Windjammer Cookbook - Common Patterns

A collection of common game development patterns and solutions in Windjammer.

---

## Table of Contents

1. [Player Movement](#player-movement)
2. [Camera Control](#camera-control)
3. [Shooting & Projectiles](#shooting--projectiles)
4. [Health & Damage](#health--damage)
5. [Collectibles](#collectibles)
6. [Enemy AI](#enemy-ai)
7. [Spawning & Pooling](#spawning--pooling)
8. [UI & HUD](#ui--hud)
9. [Save & Load](#save--load)
10. [Audio](#audio)
11. [Particle Effects](#particle-effects)
12. [Animation](#animation)
13. [Networking](#networking)
14. [Performance](#performance)

---

## Player Movement

### 2D Platformer Movement

**Python**:
```python
class PlayerMovementSystem:
    def __init__(self):
        self.speed = 200.0
        self.jump_force = 400.0
        self.gravity = 980.0
        self.max_fall_speed = 500.0
    
    def update(self, world, delta_time):
        for entity in world.query(Transform2D, RigidBody2D, PlayerTag):
            transform = entity.get(Transform2D)
            rb = entity.get(RigidBody2D)
            
            # Horizontal movement
            move_x = 0.0
            if Input.is_key_pressed(Key.Left) or Input.is_key_pressed(Key.A):
                move_x = -1.0
            if Input.is_key_pressed(Key.Right) or Input.is_key_pressed(Key.D):
                move_x = 1.0
            
            rb.velocity.x = move_x * self.speed
            
            # Apply gravity
            rb.velocity.y = min(rb.velocity.y + self.gravity * delta_time, self.max_fall_speed)
            
            # Jump
            if Input.is_key_just_pressed(Key.Space) and rb.is_on_ground():
                rb.velocity.y = -self.jump_force
```

**JavaScript**:
```javascript
class PlayerMovementSystem {
    constructor() {
        this.speed = 200.0;
        this.jumpForce = 400.0;
        this.gravity = 980.0;
        this.maxFallSpeed = 500.0;
    }
    
    update(world, deltaTime) {
        for (const entity of world.query(Transform2D, RigidBody2D, PlayerTag)) {
            const transform = entity.get(Transform2D);
            const rb = entity.get(RigidBody2D);
            
            // Horizontal movement
            let moveX = 0.0;
            if (Input.isKeyPressed(Key.Left) || Input.isKeyPressed(Key.A)) {
                moveX = -1.0;
            }
            if (Input.isKeyPressed(Key.Right) || Input.isKeyPressed(Key.D)) {
                moveX = 1.0;
            }
            
            rb.velocity.x = moveX * this.speed;
            
            // Apply gravity
            rb.velocity.y = Math.min(rb.velocity.y + this.gravity * deltaTime, this.maxFallSpeed);
            
            // Jump
            if (Input.isKeyJustPressed(Key.Space) && rb.isOnGround()) {
                rb.velocity.y = -this.jumpForce;
            }
        }
    }
}
```

### Top-Down Movement (8-directional)

**Python**:
```python
class TopDownMovementSystem:
    def __init__(self):
        self.speed = 150.0
    
    def update(self, world, delta_time):
        for entity in world.query(Transform2D, PlayerTag):
            transform = entity.get(Transform2D)
            
            # Get input
            move_x = 0.0
            move_y = 0.0
            
            if Input.is_key_pressed(Key.W): move_y = -1.0
            if Input.is_key_pressed(Key.S): move_y = 1.0
            if Input.is_key_pressed(Key.A): move_x = -1.0
            if Input.is_key_pressed(Key.D): move_x = 1.0
            
            # Normalize diagonal movement
            direction = Vec2(move_x, move_y).normalized()
            
            # Move
            transform.position += direction * self.speed * delta_time
```

### First-Person Controller (3D)

**Python**:
```python
class FirstPersonController:
    def __init__(self):
        self.move_speed = 5.0
        self.mouse_sensitivity = 0.002
        self.yaw = 0.0
        self.pitch = 0.0
    
    def update(self, world, delta_time):
        for entity in world.query(Transform3D, Camera3D, PlayerTag):
            transform = entity.get(Transform3D)
            camera = entity.get(Camera3D)
            
            # Mouse look
            mouse_delta = Input.get_mouse_delta()
            self.yaw -= mouse_delta.x * self.mouse_sensitivity
            self.pitch -= mouse_delta.y * self.mouse_sensitivity
            self.pitch = clamp(self.pitch, -1.57, 1.57)  # -90 to 90 degrees
            
            # Apply rotation
            transform.rotation = Quat.from_euler(self.pitch, self.yaw, 0.0)
            
            # Movement
            move_dir = Vec3(0, 0, 0)
            if Input.is_key_pressed(Key.W): move_dir.z -= 1.0
            if Input.is_key_pressed(Key.S): move_dir.z += 1.0
            if Input.is_key_pressed(Key.A): move_dir.x -= 1.0
            if Input.is_key_pressed(Key.D): move_dir.x += 1.0
            
            # Transform direction by camera rotation
            move_dir = transform.rotation * move_dir
            move_dir.y = 0.0  # No vertical movement
            move_dir = move_dir.normalized()
            
            # Move
            transform.position += move_dir * self.move_speed * delta_time
```

---

## Camera Control

### Smooth Follow Camera (2D)

**Python**:
```python
class CameraFollowSystem:
    def __init__(self):
        self.smoothness = 5.0
        self.offset = Vec2(0, -50)  # Offset from player
    
    def update(self, world, delta_time):
        # Find player
        player = world.query_single(PlayerTag, Transform2D)
        if not player:
            return
        
        player_pos = player.get(Transform2D).position
        
        # Find camera
        camera_entity = world.query_single(Camera2D)
        if not camera_entity:
            return
        
        camera = camera_entity.get(Camera2D)
        
        # Smooth follow
        target_pos = player_pos + self.offset
        camera.position = camera.position.lerp(target_pos, self.smoothness * delta_time)
```

### Third-Person Camera (3D)

**Python**:
```python
class ThirdPersonCameraSystem:
    def __init__(self):
        self.distance = 5.0
        self.height = 2.0
        self.smoothness = 10.0
    
    def update(self, world, delta_time):
        # Find player
        player = world.query_single(PlayerTag, Transform3D)
        if not player:
            return
        
        player_transform = player.get(Transform3D)
        
        # Find camera
        camera_entity = world.query_single(Camera3D)
        if not camera_entity:
            return
        
        camera_transform = camera_entity.get(Transform3D)
        
        # Calculate target position (behind and above player)
        forward = player_transform.forward()
        target_pos = player_transform.position - forward * self.distance
        target_pos.y += self.height
        
        # Smooth follow
        camera_transform.position = camera_transform.position.lerp(
            target_pos, 
            self.smoothness * delta_time
        )
        
        # Look at player
        camera_transform.look_at(player_transform.position + Vec3(0, 1, 0))
```

---

## Shooting & Projectiles

### Basic Shooting

**Python**:
```python
class ShootingSystem:
    def __init__(self):
        self.bullet_speed = 500.0
        self.fire_rate = 0.2  # Seconds between shots
        self.last_shot_time = 0.0
    
    def update(self, world, delta_time):
        self.last_shot_time += delta_time
        
        for entity in world.query(Transform2D, PlayerTag):
            if Input.is_mouse_button_pressed(MouseButton.Left):
                if self.last_shot_time >= self.fire_rate:
                    self.shoot(world, entity)
                    self.last_shot_time = 0.0
    
    def shoot(self, world, shooter):
        transform = shooter.get(Transform2D)
        
        # Get direction to mouse
        mouse_pos = Input.get_mouse_position()
        direction = (mouse_pos - transform.position).normalized()
        
        # Create bullet
        bullet = world.create_entity()
        bullet.add(Transform2D(position=transform.position))
        bullet.add(Sprite(color=Color(1, 1, 0, 1), size=Vec2(5, 5)))
        bullet.add(Velocity(x=direction.x * self.bullet_speed, y=direction.y * self.bullet_speed))
        bullet.add(BulletTag())
        bullet.add(Lifetime(duration=3.0))  # Auto-destroy after 3 seconds
```

### Homing Projectile

**Python**:
```python
class HomingProjectileSystem:
    def __init__(self):
        self.turn_speed = 5.0
    
    def update(self, world, delta_time):
        # Find target (first enemy)
        target = world.query_single(EnemyTag, Transform2D)
        if not target:
            return
        
        target_pos = target.get(Transform2D).position
        
        # Update homing projectiles
        for entity in world.query(Transform2D, Velocity, HomingTag):
            transform = entity.get(Transform2D)
            velocity = entity.get(Velocity)
            
            # Calculate direction to target
            to_target = (target_pos - transform.position).normalized()
            
            # Current direction
            current_dir = Vec2(velocity.x, velocity.y).normalized()
            
            # Rotate towards target
            new_dir = current_dir.lerp(to_target, self.turn_speed * delta_time).normalized()
            
            # Update velocity
            speed = Vec2(velocity.x, velocity.y).length()
            velocity.x = new_dir.x * speed
            velocity.y = new_dir.y * speed
```

---

## Health & Damage

### Health System

**Python**:
```python
class Health:
    def __init__(self, max_health=100):
        self.max_health = max_health
        self.current_health = max_health
    
    def take_damage(self, amount):
        self.current_health = max(0, self.current_health - amount)
        return self.current_health <= 0  # Returns True if dead
    
    def heal(self, amount):
        self.current_health = min(self.max_health, self.current_health + amount)
    
    def is_alive(self):
        return self.current_health > 0

class DamageSystem:
    def update(self, world, delta_time):
        # Check bullet-enemy collisions
        for collision in world.get_collisions():
            entity_a, entity_b = collision.entity_a, collision.entity_b
            
            # Bullet hits enemy
            if entity_a.has(BulletTag) and entity_b.has(EnemyTag, Health):
                health = entity_b.get(Health)
                if health.take_damage(10):
                    # Enemy died
                    world.destroy_entity(entity_b)
                
                # Destroy bullet
                world.destroy_entity(entity_a)
            
            # Enemy hits player
            elif entity_a.has(EnemyTag) and entity_b.has(PlayerTag, Health):
                health = entity_b.get(Health)
                if health.take_damage(5):
                    # Player died - game over
                    self.game_over(world)
```

### Invincibility Frames

**Python**:
```python
class Invincibility:
    def __init__(self, duration=1.0):
        self.duration = duration
        self.time_remaining = 0.0
    
    def is_invincible(self):
        return self.time_remaining > 0
    
    def activate(self):
        self.time_remaining = self.duration
    
    def update(self, delta_time):
        if self.time_remaining > 0:
            self.time_remaining -= delta_time

class InvincibilitySystem:
    def update(self, world, delta_time):
        for entity in world.query(Invincibility):
            invincibility = entity.get(Invincibility)
            invincibility.update(delta_time)
            
            # Flash sprite during invincibility
            if entity.has(Sprite):
                sprite = entity.get(Sprite)
                if invincibility.is_invincible():
                    # Flash effect
                    sprite.color.a = 0.5 if (int(invincibility.time_remaining * 10) % 2) else 1.0
                else:
                    sprite.color.a = 1.0

class DamageSystemWithInvincibility:
    def update(self, world, delta_time):
        for collision in world.get_collisions():
            entity_a, entity_b = collision.entity_a, collision.entity_b
            
            # Enemy hits player
            if entity_a.has(EnemyTag) and entity_b.has(PlayerTag, Health, Invincibility):
                invincibility = entity_b.get(Invincibility)
                
                # Only take damage if not invincible
                if not invincibility.is_invincible():
                    health = entity_b.get(Health)
                    health.take_damage(5)
                    invincibility.activate()
```

---

## Collectibles

### Coin Collection

**Python**:
```python
class Score:
    def __init__(self):
        self.coins = 0
        self.score = 0
    
    def add_coin(self):
        self.coins += 1
        self.score += 10

class CollectibleSystem:
    def update(self, world, delta_time):
        for collision in world.get_collisions():
            entity_a, entity_b = collision.entity_a, collision.entity_b
            
            # Player collects coin
            if entity_a.has(PlayerTag, Score) and entity_b.has(CoinTag):
                score = entity_a.get(Score)
                score.add_coin()
                
                # Play sound
                AudioManager.play_sound("coin_collect.wav")
                
                # Spawn particle effect
                self.spawn_collect_effect(world, entity_b.get(Transform2D).position)
                
                # Destroy coin
                world.destroy_entity(entity_b)
    
    def spawn_collect_effect(self, world, position):
        effect = world.create_entity()
        effect.add(Transform2D(position=position))
        effect.add(ParticleEmitter(
            count=10,
            lifetime=0.5,
            color=Color(1, 1, 0, 1)
        ))
        effect.add(Lifetime(duration=0.5))
```

### Power-Up System

**Python**:
```python
class PowerUp:
    def __init__(self, type, duration=5.0):
        self.type = type  # "speed", "invincibility", "double_damage"
        self.duration = duration

class ActivePowerUps:
    def __init__(self):
        self.power_ups = []  # List of (type, time_remaining)
    
    def add(self, power_up):
        self.power_ups.append([power_up.type, power_up.duration])
    
    def has(self, type):
        return any(pu[0] == type for pu in self.power_ups)
    
    def update(self, delta_time):
        # Update timers
        self.power_ups = [[pu[0], pu[1] - delta_time] for pu in self.power_ups]
        # Remove expired
        self.power_ups = [pu for pu in self.power_ups if pu[1] > 0]

class PowerUpSystem:
    def update(self, world, delta_time):
        # Update active power-ups
        for entity in world.query(ActivePowerUps):
            active = entity.get(ActivePowerUps)
            active.update(delta_time)
        
        # Check collisions
        for collision in world.get_collisions():
            entity_a, entity_b = collision.entity_a, collision.entity_b
            
            if entity_a.has(PlayerTag, ActivePowerUps) and entity_b.has(PowerUp):
                power_up = entity_b.get(PowerUp)
                active = entity_a.get(ActivePowerUps)
                active.add(power_up)
                
                # Visual feedback
                self.show_power_up_message(world, power_up.type)
                
                # Destroy power-up
                world.destroy_entity(entity_b)
```

---

## Enemy AI

### Chase Player AI

**Python**:
```python
class ChaseAISystem:
    def __init__(self):
        self.speed = 100.0
        self.chase_range = 300.0
    
    def update(self, world, delta_time):
        # Find player
        player = world.query_single(PlayerTag, Transform2D)
        if not player:
            return
        
        player_pos = player.get(Transform2D).position
        
        # Update enemies
        for entity in world.query(Transform2D, EnemyTag, ChaseAI):
            transform = entity.get(Transform2D)
            
            # Check distance
            distance = (player_pos - transform.position).length()
            
            if distance < self.chase_range:
                # Chase player
                direction = (player_pos - transform.position).normalized()
                transform.position += direction * self.speed * delta_time
```

### Patrol AI

**Python**:
```python
class PatrolAI:
    def __init__(self, waypoints):
        self.waypoints = waypoints  # List of Vec2
        self.current_waypoint = 0
        self.wait_time = 2.0
        self.wait_timer = 0.0

class PatrolAISystem:
    def __init__(self):
        self.speed = 50.0
        self.waypoint_threshold = 10.0
    
    def update(self, world, delta_time):
        for entity in world.query(Transform2D, PatrolAI):
            transform = entity.get(Transform2D)
            patrol = entity.get(PatrolAI)
            
            # Waiting at waypoint
            if patrol.wait_timer > 0:
                patrol.wait_timer -= delta_time
                continue
            
            # Get current waypoint
            target = patrol.waypoints[patrol.current_waypoint]
            
            # Move towards waypoint
            direction = (target - transform.position)
            distance = direction.length()
            
            if distance < self.waypoint_threshold:
                # Reached waypoint
                patrol.current_waypoint = (patrol.current_waypoint + 1) % len(patrol.waypoints)
                patrol.wait_timer = patrol.wait_time
            else:
                # Move
                direction = direction.normalized()
                transform.position += direction * self.speed * delta_time
```

---

## Spawning & Pooling

### Object Pooling

**Python**:
```python
class ObjectPool:
    def __init__(self, world, factory, initial_size=10):
        self.world = world
        self.factory = factory
        self.available = []
        self.active = []
        
        # Pre-create objects
        for _ in range(initial_size):
            entity = factory()
            entity.set_active(False)
            self.available.append(entity)
    
    def get(self):
        if self.available:
            entity = self.available.pop()
            entity.set_active(True)
            self.active.append(entity)
            return entity
        else:
            # Pool exhausted, create new
            entity = self.factory()
            self.active.append(entity)
            return entity
    
    def return_to_pool(self, entity):
        if entity in self.active:
            self.active.remove(entity)
            entity.set_active(False)
            self.available.append(entity)

# Usage
class BulletPoolSystem:
    def __init__(self, world):
        self.pool = ObjectPool(world, self.create_bullet, initial_size=50)
    
    def create_bullet(self):
        bullet = self.world.create_entity()
        bullet.add(Transform2D())
        bullet.add(Sprite(color=Color(1, 1, 0, 1), size=Vec2(5, 5)))
        bullet.add(Velocity())
        bullet.add(BulletTag())
        return bullet
    
    def shoot(self, position, direction, speed):
        bullet = self.pool.get()
        transform = bullet.get(Transform2D)
        velocity = bullet.get(Velocity)
        
        transform.position = position
        velocity.x = direction.x * speed
        velocity.y = direction.y * speed
    
    def return_bullet(self, bullet):
        self.pool.return_to_pool(bullet)
```

### Wave Spawner

**Python**:
```python
class WaveSpawner:
    def __init__(self):
        self.current_wave = 0
        self.enemies_per_wave = 5
        self.enemies_remaining = 0
        self.spawn_interval = 2.0
        self.spawn_timer = 0.0
        self.wave_delay = 5.0
        self.wave_timer = 0.0
    
    def update(self, world, delta_time):
        # Wave delay between waves
        if self.wave_timer > 0:
            self.wave_timer -= delta_time
            return
        
        # Start new wave if no enemies left
        if self.enemies_remaining == 0:
            self.start_wave(world)
        
        # Spawn enemies
        if self.enemies_remaining > 0:
            self.spawn_timer -= delta_time
            if self.spawn_timer <= 0:
                self.spawn_enemy(world)
                self.enemies_remaining -= 1
                self.spawn_timer = self.spawn_interval
    
    def start_wave(self, world):
        self.current_wave += 1
        self.enemies_remaining = self.enemies_per_wave + (self.current_wave * 2)
        self.spawn_timer = 0.0
        
        # Show wave message
        self.show_wave_message(world, self.current_wave)
    
    def spawn_enemy(self, world):
        # Spawn at random position
        spawn_pos = self.get_random_spawn_position()
        
        enemy = world.create_entity()
        enemy.add(Transform2D(position=spawn_pos))
        enemy.add(Sprite(color=Color(1, 0, 0, 1), size=Vec2(30, 30)))
        enemy.add(Health(max_health=50 + self.current_wave * 10))
        enemy.add(EnemyTag())
        enemy.add(ChaseAI())
```

---

## UI & HUD

### Health Bar

**Python**:
```python
class HealthBarUI:
    def __init__(self, world):
        self.world = world
        self.bar_entity = None
        self.create_ui()
    
    def create_ui(self):
        # Background
        bg = self.world.create_entity()
        bg.add(Transform2D(position=Vec2(20, 20)))
        bg.add(UIRect(size=Vec2(200, 20), color=Color(0.2, 0.2, 0.2, 1)))
        
        # Health bar
        self.bar_entity = self.world.create_entity()
        self.bar_entity.add(Transform2D(position=Vec2(20, 20)))
        self.bar_entity.add(UIRect(size=Vec2(200, 20), color=Color(1, 0, 0, 1)))
    
    def update(self, world, delta_time):
        # Find player health
        player = world.query_single(PlayerTag, Health)
        if not player:
            return
        
        health = player.get(Health)
        health_percent = health.current_health / health.max_health
        
        # Update bar width
        rect = self.bar_entity.get(UIRect)
        rect.size.x = 200 * health_percent
```

### Score Display

**Python**:
```python
class ScoreUI:
    def __init__(self, world):
        self.world = world
        self.text_entity = None
        self.create_ui()
    
    def create_ui(self):
        self.text_entity = self.world.create_entity()
        self.text_entity.add(Transform2D(position=Vec2(10, 50)))
        self.text_entity.add(UIText(
            text="Score: 0",
            font_size=24,
            color=Color(1, 1, 1, 1)
        ))
    
    def update(self, world, delta_time):
        # Find player score
        player = world.query_single(PlayerTag, Score)
        if not player:
            return
        
        score = player.get(Score)
        
        # Update text
        text = self.text_entity.get(UIText)
        text.text = f"Score: {score.score}"
```

---

## Save & Load

### Simple Save System

**Python**:
```python
import json

class SaveData:
    def __init__(self):
        self.player_position = [0, 0]
        self.player_health = 100
        self.score = 0
        self.level = 1

class SaveSystem:
    def __init__(self, filename="savegame.json"):
        self.filename = filename
    
    def save(self, world):
        # Find player
        player = world.query_single(PlayerTag, Transform2D, Health, Score)
        if not player:
            return
        
        # Create save data
        save_data = SaveData()
        
        transform = player.get(Transform2D)
        save_data.player_position = [transform.position.x, transform.position.y]
        
        health = player.get(Health)
        save_data.player_health = health.current_health
        
        score = player.get(Score)
        save_data.player_score = score.score
        
        # Write to file
        with open(self.filename, 'w') as f:
            json.dump(save_data.__dict__, f, indent=2)
        
        print("Game saved!")
    
    def load(self, world):
        try:
            # Read from file
            with open(self.filename, 'r') as f:
                data = json.load(f)
            
            # Find player
            player = world.query_single(PlayerTag, Transform2D, Health, Score)
            if not player:
                return False
            
            # Apply loaded data
            transform = player.get(Transform2D)
            transform.position = Vec2(data['player_position'][0], data['player_position'][1])
            
            health = player.get(Health)
            health.current_health = data['player_health']
            
            score = player.get(Score)
            score.score = data.get('player_score', 0)
            
            print("Game loaded!")
            return True
        
        except FileNotFoundError:
            print("No save file found")
            return False
```

---

## Audio

### Background Music Manager

**Python**:
```python
class MusicManager:
    def __init__(self):
        self.current_track = None
        self.tracks = {}
        self.volume = 0.7
    
    def load_track(self, name, path):
        self.tracks[name] = AudioClip.load(path)
    
    def play(self, name, loop=True):
        if name in self.tracks:
            if self.current_track:
                self.current_track.stop()
            
            self.current_track = self.tracks[name]
            self.current_track.set_volume(self.volume)
            self.current_track.set_loop(loop)
            self.current_track.play()
    
    def stop(self):
        if self.current_track:
            self.current_track.stop()
    
    def set_volume(self, volume):
        self.volume = clamp(volume, 0.0, 1.0)
        if self.current_track:
            self.current_track.set_volume(self.volume)

# Usage
music = MusicManager()
music.load_track("menu", "assets/music/menu.ogg")
music.load_track("gameplay", "assets/music/gameplay.ogg")
music.play("menu")
```

### Sound Effect Pool

**Python**:
```python
class SoundEffectManager:
    def __init__(self, max_concurrent=10):
        self.sounds = {}
        self.playing = []
        self.max_concurrent = max_concurrent
    
    def load_sound(self, name, path):
        self.sounds[name] = AudioClip.load(path)
    
    def play(self, name, volume=1.0):
        if name not in self.sounds:
            return
        
        # Limit concurrent sounds
        if len(self.playing) >= self.max_concurrent:
            # Stop oldest sound
            self.playing[0].stop()
            self.playing.pop(0)
        
        # Play sound
        sound = self.sounds[name].clone()
        sound.set_volume(volume)
        sound.play()
        self.playing.append(sound)
    
    def update(self):
        # Remove finished sounds
        self.playing = [s for s in self.playing if s.is_playing()]

# Usage
sfx = SoundEffectManager()
sfx.load_sound("jump", "assets/sfx/jump.wav")
sfx.load_sound("shoot", "assets/sfx/shoot.wav")
sfx.load_sound("hit", "assets/sfx/hit.wav")

# Play sounds
sfx.play("jump")
sfx.play("shoot", volume=0.5)
```

---

## Particle Effects

### Explosion Effect

**Python**:
```python
def create_explosion(world, position, color=Color(1, 0.5, 0, 1)):
    explosion = world.create_entity()
    explosion.add(Transform2D(position=position))
    explosion.add(ParticleEmitter(
        count=50,
        lifetime=1.0,
        speed_min=100,
        speed_max=300,
        spread=360,  # degrees
        color_start=color,
        color_end=Color(0.5, 0.5, 0.5, 0),
        size_start=10,
        size_end=2,
        gravity=Vec2(0, 100)
    ))
    explosion.add(Lifetime(duration=1.0))
    
    # Play sound
    AudioManager.play_sound("explosion.wav")
```

### Trail Effect

**Python**:
```python
class TrailSystem:
    def __init__(self):
        self.spawn_interval = 0.05
        self.spawn_timer = 0.0
    
    def update(self, world, delta_time):
        self.spawn_timer += delta_time
        
        if self.spawn_timer >= self.spawn_interval:
            self.spawn_timer = 0.0
            
            # Spawn trail particles for entities with TrailTag
            for entity in world.query(Transform2D, TrailTag):
                transform = entity.get(Transform2D)
                self.spawn_trail_particle(world, transform.position)
    
    def spawn_trail_particle(self, world, position):
        particle = world.create_entity()
        particle.add(Transform2D(position=position))
        particle.add(Sprite(
            color=Color(1, 1, 1, 0.5),
            size=Vec2(5, 5)
        ))
        particle.add(FadeOut(duration=0.5))
        particle.add(Lifetime(duration=0.5))
```

---

## Animation

### Sprite Animation

**Python**:
```python
class SpriteAnimation:
    def __init__(self, frames, frame_duration=0.1, loop=True):
        self.frames = frames  # List of texture paths
        self.frame_duration = frame_duration
        self.loop = loop
        self.current_frame = 0
        self.time_accumulator = 0.0
        self.is_playing = True
    
    def update(self, delta_time):
        if not self.is_playing:
            return
        
        self.time_accumulator += delta_time
        
        if self.time_accumulator >= self.frame_duration:
            self.time_accumulator = 0.0
            self.current_frame += 1
            
            if self.current_frame >= len(self.frames):
                if self.loop:
                    self.current_frame = 0
                else:
                    self.current_frame = len(self.frames) - 1
                    self.is_playing = False
    
    def get_current_texture(self):
        return self.frames[self.current_frame]

class SpriteAnimationSystem:
    def update(self, world, delta_time):
        for entity in world.query(Sprite, SpriteAnimation):
            sprite = entity.get(Sprite)
            animation = entity.get(SpriteAnimation)
            
            animation.update(delta_time)
            sprite.texture = animation.get_current_texture()
```

---

## Networking

### Simple Multiplayer

**Python**:
```python
class MultiplayerSystem:
    def __init__(self, is_server=False):
        self.is_server = is_server
        self.network = NetworkManager()
        
        if is_server:
            self.network.start_server(port=7777)
        else:
            self.network.connect("127.0.0.1", 7777)
    
    def update(self, world, delta_time):
        # Server: Send player positions to all clients
        if self.is_server:
            for entity in world.query(Transform2D, NetworkedEntity):
                transform = entity.get(Transform2D)
                networked = entity.get(NetworkedEntity)
                
                self.network.broadcast({
                    'type': 'position_update',
                    'entity_id': networked.id,
                    'x': transform.position.x,
                    'y': transform.position.y
                })
        
        # Process incoming messages
        for message in self.network.receive_messages():
            if message['type'] == 'position_update':
                self.handle_position_update(world, message)
    
    def handle_position_update(self, world, message):
        # Find entity by network ID
        for entity in world.query(NetworkedEntity, Transform2D):
            networked = entity.get(NetworkedEntity)
            if networked.id == message['entity_id']:
                transform = entity.get(Transform2D)
                transform.position = Vec2(message['x'], message['y'])
                break
```

---

## Performance

### FPS Counter

**Python**:
```python
class FPSCounter:
    def __init__(self):
        self.frame_times = []
        self.max_samples = 60
        self.fps = 60.0
    
    def update(self, delta_time):
        self.frame_times.append(delta_time)
        
        if len(self.frame_times) > self.max_samples:
            self.frame_times.pop(0)
        
        if self.frame_times:
            avg_frame_time = sum(self.frame_times) / len(self.frame_times)
            self.fps = 1.0 / avg_frame_time if avg_frame_time > 0 else 60.0
    
    def get_fps(self):
        return int(self.fps)
```

### Debug Overlay

**Python**:
```python
class DebugOverlay:
    def __init__(self, world):
        self.world = world
        self.enabled = True
        self.fps_counter = FPSCounter()
        self.text_entity = None
        self.create_ui()
    
    def create_ui(self):
        self.text_entity = world.create_entity()
        self.text_entity.add(Transform2D(position=Vec2(10, 10)))
        self.text_entity.add(UIText(
            text="",
            font_size=16,
            color=Color(0, 1, 0, 1)
        ))
    
    def update(self, world, delta_time):
        if not self.enabled:
            return
        
        self.fps_counter.update(delta_time)
        
        # Gather stats
        entity_count = len(world.get_all_entities())
        fps = self.fps_counter.get_fps()
        
        # Update text
        text = self.text_entity.get(UIText)
        text.text = f"FPS: {fps}\nEntities: {entity_count}"
        
        # Toggle with F3
        if Input.is_key_just_pressed(Key.F3):
            self.enabled = not self.enabled
            self.text_entity.set_active(self.enabled)
```

---

## Conclusion

This cookbook covers the most common patterns in game development. All patterns work with **automatic optimization** - Windjammer handles batching, instancing, and parallelization automatically!

**Key Takeaways**:
- ✅ Write clean, simple code
- ✅ Let Windjammer optimize automatically
- ✅ Focus on game logic, not performance
- ✅ Use these patterns as starting points
- ✅ Customize for your specific game

**Need more patterns?** Check out:
- [Tutorials](TUTORIALS.md) (TODO)
- [API Documentation](api/index.md) (TODO)
- [Examples](../examples/) ✅

**Happy game development!** 🎮

