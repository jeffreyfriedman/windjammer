# Quick Start Guide - Windjammer 2025

## 🚀 Getting Started

This guide will get you up and running with Windjammer in **5 minutes**!

---

## 📦 Installation

### **Option 1: From Source (Recommended)**

```bash
# Clone the repository
git clone https://github.com/windjammer-lang/windjammer.git
cd windjammer

# Build the compiler
cargo build --release

# Add to PATH
export PATH="$PWD/target/release:$PATH"

# Verify installation
wj --version
```

### **Option 2: From Crates.io** (Coming Soon)

```bash
cargo install windjammer
```

---

## 🎮 Your First Game (PONG)

### **Step 1: Create a new project**

```bash
mkdir my-pong-game
cd my-pong-game
```

### **Step 2: Create `main.wj`**

```windjammer
@game
struct PongGame {
    ball_x: float,
    ball_y: float,
    ball_vx: float,
    ball_vy: float,
    paddle_y: float,
}

@init
fn init(game: PongGame) {
    game.ball_x = 400.0
    game.ball_y = 300.0
    game.ball_vx = 200.0
    game.ball_vy = 150.0
    game.paddle_y = 250.0
}

@update
fn update(game: PongGame, delta: float, input: Input) {
    // Move ball
    game.ball_x += game.ball_vx * delta
    game.ball_y += game.ball_vy * delta
    
    // Bounce off walls
    if game.ball_y < 0.0 || game.ball_y > 600.0 {
        game.ball_vy = -game.ball_vy
    }
    
    // Move paddle
    if input.held(Key::W) {
        game.paddle_y -= 300.0 * delta
    }
    if input.held(Key::S) {
        game.paddle_y += 300.0 * delta
    }
}

@render
fn render(game: PongGame, renderer: Renderer) {
    renderer.clear(Color::black())
    
    // Draw ball
    renderer.draw_circle(game.ball_x, game.ball_y, 10.0, Color::white())
    
    // Draw paddle
    renderer.draw_rect(50.0, game.paddle_y, 10.0, 100.0, Color::white())
}
```

### **Step 3: Run your game**

```bash
wj run main.wj
```

**That's it!** 🎉 You just created your first Windjammer game!

---

## 🎯 Your First 3D Game (Shooter)

### **Create `shooter.wj`**

```windjammer
@game
struct ShooterGame {
    player_pos: Vec3,
    player_yaw: float,
    player_pitch: float,
}

@init
fn init(game: ShooterGame) {
    game.player_pos = Vec3::new(0.0, 1.0, 0.0)
    game.player_yaw = 0.0
    game.player_pitch = 0.0
}

@update
fn update(game: ShooterGame, delta: float, input: Input) {
    // Mouse look
    let dx = input.mouse_delta_x() as f32
    let dy = input.mouse_delta_y() as f32
    
    game.player_yaw -= dx * 0.1
    game.player_pitch -= dy * 0.1
    
    // Movement
    let speed = 5.0
    if input.held(Key::W) {
        game.player_pos.z += speed * delta
    }
    if input.held(Key::S) {
        game.player_pos.z -= speed * delta
    }
}

@render3d
fn render(game: ShooterGame, renderer: Renderer3D, camera: Camera3D) {
    // Set camera
    camera.position = game.player_pos
    camera.yaw = game.player_yaw
    camera.pitch = game.player_pitch
    
    // Draw floor
    renderer.draw_cube(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.1, 10.0),
        Color::gray()
    )
}
```

### **Run your 3D game**

```bash
wj run shooter.wj
```

---

## 🎨 Adding UI (HUD)

```windjammer
@render_ui
fn render_ui(game: ShooterGame, ui: UI) {
    ui.window("HUD", Vec2::new(10.0, 10.0), Vec2::new(200.0, 100.0))
    
    ui.label("Health:")
    ui.progress_bar(game.health / 100.0, Color::red())
    
    ui.label("Ammo: " + game.ammo.to_string())
    
    if ui.button("Pause") {
        game.paused = true
    }
    
    ui.end_window()
}
```

---

## 📚 Learn More

### **Documentation**
- [Language Guide](../GUIDE.md)
- [Game Framework Guide](GAME_FRAMEWORK_ARCHITECTURE.md)
- [Cross-Platform Vision](CROSS_PLATFORM_VISION.md)
- [2025 Roadmap](WINDJAMMER_2025_ROADMAP.md)

### **Examples**
- PONG: `examples/games/pong/main.wj`
- 3D Shooter: `examples/games/shooter/main.wj`

### **Community**
- GitHub: [github.com/windjammer-lang/windjammer](https://github.com/windjammer-lang/windjammer)
- Discord: Coming soon!
- Reddit: r/windjammer (coming soon!)

---

## 🎯 Next Steps

### **Try the Examples**

```bash
# Run PONG
wj run examples/games/pong/main.wj

# Run 3D Shooter
wj run examples/games/shooter/main.wj
```

### **Explore Features**

1. **Animation System**
   - Skeletal animation
   - Animation blending
   - Inverse kinematics

2. **Physics System**
   - Rigid body dynamics
   - Collision detection
   - Raycasting

3. **Advanced Rendering**
   - SSGI (global illumination)
   - LOD (level of detail)
   - VSM (virtual shadow maps)

### **Join the Community**

1. Star us on GitHub ⭐
2. Join our Discord (coming soon!)
3. Share your games!

---

## 🚀 Coming in 2025

### **Q1 2025: Editor Foundation**
- 🌐 Web editor (browser-based)
- 💻 Desktop editor (native)
- 🎨 Visual tools

### **Q2 2025: Mobile Editor**
- 📱 iOS editor
- 📱 Android editor
- ☁️ Cloud sync

### **Q3 2025: Advanced Features**
- 🎬 Visual scripting
- ✨ Particle system
- 🏔️ Terrain editor

### **Q4 2025: 1.0 Release!**
- 🏪 Asset marketplace
- 👥 Community features
- 🎉 Launch event

---

## 💡 Tips & Tricks

### **Performance**

```bash
# Release build (optimized)
wj build --release main.wj

# Profile your game
wj profile main.wj
```

### **Debugging**

```bash
# Run with debug info
wj run --debug main.wj

# Check for errors
wj check main.wj
```

### **Hot Reload**

```bash
# Watch for changes
wj watch main.wj
```

---

## 🆘 Troubleshooting

### **Build Errors**

```bash
# Clean build
cargo clean
cargo build --release

# Update dependencies
cargo update
```

### **Game Not Running**

1. Check `wj --version`
2. Verify file path
3. Check for syntax errors: `wj check main.wj`

### **Performance Issues**

1. Use release build: `wj build --release`
2. Enable SSGI only if needed
3. Use LOD for complex scenes

---

## 🌟 Why Windjammer?

### **1. Simple & Powerful**
- Clean syntax (like Python)
- Rust performance
- Zero crate leakage

### **2. World-Class Errors**
- Helpful error messages
- Auto-fix suggestions
- Interactive TUI

### **3. Cross-Platform Editor**
- 🌐 Web (browser-based)
- 💻 Desktop (native)
- 📱 Mobile (iOS/Android)
- **UNIQUE!** No other engine has all three!

### **4. AAA Rendering**
- SSGI (global illumination)
- LOD (level of detail)
- VSM (virtual shadow maps)
- Competitive with UE5/Unity

### **5. 100% Free**
- No runtime fees
- No royalties
- Open source (MIT/Apache)
- Community-driven

---

## 🎉 Welcome to Windjammer!

**You're now part of the future of game development!**

### **What's Next?**

1. ⭐ Star us on GitHub
2. 🎮 Build your first game
3. 💬 Join the community
4. 🚀 Share your creations!

---

**"The ONLY game engine you can edit on your phone!"** 📱

**Let's change game development together!** 🚀

---

## 📞 Get Help

- **GitHub Issues**: [Report bugs](https://github.com/windjammer-lang/windjammer/issues)
- **Discussions**: [Ask questions](https://github.com/windjammer-lang/windjammer/discussions)
- **Discord**: Coming soon!
- **Email**: support@windjammer.dev (coming soon!)

---

**Happy coding!** 🎮✨

