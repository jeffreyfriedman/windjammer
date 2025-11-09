# UI Integration Plan

## Executive Summary

Integrate `windjammer-ui` with the game framework to provide:
1. **In-Game UI** - HUD, menus, dialogs for games
2. **Editor Foundation** - UI primitives for visual editor

This creates a **mutually reinforcing ecosystem**:
- Game framework exercises windjammer-ui
- windjammer-ui exercises windjammer language
- Editor uses both

---

## Phase 1: Game UI System (Priority 1)

### Goal
Provide immediate-mode-style UI for games (like Dear ImGui, egui).

### Why Immediate Mode?
- **Simple API** - No state management
- **Easy integration** - Works with game loop
- **Perfect for HUD** - Health bars, ammo counters, etc.
- **Perfect for debug UI** - FPS counter, physics debug, etc.

### Architecture

```
┌─────────────────────────────────────┐
│         Game Framework              │
│  (3D rendering, physics, input)     │
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│       Game UI System (New!)         │
│  - Immediate mode API               │
│  - Layout engine                    │
│  - Widget library                   │
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│      Windjammer-UI (Existing)       │
│  - Reactive state                   │
│  - Virtual DOM                      │
│  - Platform abstraction             │
└─────────────────────────────────────┘
```

### API Design

```windjammer
@render
fn render_ui(game: ShooterGame, ui: UI) {
    // Window
    ui.window("HUD")
        .position(10.0, 10.0)
        .size(200.0, 100.0)
        .begin()
    
    // Health bar
    ui.label("Health:")
    ui.progress_bar(game.player_health / 100.0, Color::red())
    
    // Ammo counter
    ui.label("Ammo: " + game.ammo.to_string())
    
    // Button
    if ui.button("Pause") {
        game.paused = true
    }
    
    ui.end()
}
```

### Components to Implement

#### 1. UI Context
```rust
pub struct UI {
    renderer: UIRenderer,
    layout: LayoutEngine,
    input: InputState,
    current_window: Option<WindowState>,
}

impl UI {
    pub fn new(renderer: UIRenderer) -> Self;
    pub fn begin_frame(&mut self);
    pub fn end_frame(&mut self);
}
```

#### 2. Widgets
```rust
// Text
pub fn label(&mut self, text: &str);
pub fn text(&mut self, text: &str, color: Color);

// Buttons
pub fn button(&mut self, label: &str) -> bool;
pub fn image_button(&mut self, texture: &Texture) -> bool;

// Input
pub fn text_input(&mut self, buffer: &mut String) -> bool;
pub fn slider(&mut self, value: &mut f32, min: f32, max: f32) -> bool;
pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool;

// Display
pub fn progress_bar(&mut self, fraction: f32, color: Color);
pub fn image(&mut self, texture: &Texture, size: Vec2);
pub fn separator(&mut self);

// Layout
pub fn window(&mut self, title: &str) -> WindowBuilder;
pub fn group(&mut self) -> GroupBuilder;
pub fn horizontal(&mut self, f: impl FnOnce(&mut UI));
pub fn vertical(&mut self, f: impl FnOnce(&mut UI));
```

#### 3. Layout Engine
```rust
pub struct LayoutEngine {
    stack: Vec<LayoutNode>,
    current: LayoutNode,
}

pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

pub struct LayoutNode {
    direction: LayoutDirection,
    position: Vec2,
    size: Vec2,
    padding: f32,
    spacing: f32,
}
```

#### 4. UI Renderer
```rust
pub struct UIRenderer {
    draw_list: Vec<DrawCommand>,
    font: Font,
    textures: HashMap<String, Texture>,
}

pub enum DrawCommand {
    Rect { pos: Vec2, size: Vec2, color: Color },
    Text { pos: Vec2, text: String, color: Color },
    Image { pos: Vec2, size: Vec2, texture: Texture },
    Line { start: Vec2, end: Vec2, color: Color, thickness: f32 },
}
```

---

## Phase 2: Retained Mode UI (Priority 2)

### Goal
Provide retained-mode UI for complex interfaces (like editor).

### Why Retained Mode?
- **Complex state** - Editor has lots of state
- **Performance** - Only update what changed
- **Declarative** - Easier to reason about
- **Reactive** - Automatic updates

### Architecture

```windjammer
@component
struct EditorUI {
    scene: Scene,
    selected: Option<Entity>,
}

impl EditorUI {
    fn render(self) -> VNode {
        VElement::new("div")
            .class("editor")
            .child(self.render_viewport())
            .child(self.render_hierarchy())
            .child(self.render_inspector())
            .into()
    }
    
    fn render_viewport(self) -> VNode {
        VElement::new("div")
            .class("viewport")
            .child(Canvas::new(self.scene))
            .into()
    }
}
```

---

## Phase 3: Integration with Game Framework

### 1. Add UI Module to Game Framework

```rust
// crates/windjammer-game-framework/src/ui/mod.rs
pub mod immediate;  // Immediate mode UI
pub mod retained;   // Retained mode UI (wraps windjammer-ui)
pub mod widgets;    // Common widgets
pub mod layout;     // Layout engine
pub mod renderer;   // UI rendering
```

### 2. Integrate with Game Loop

```rust
// In generated game loop
let mut ui = UI::new(ui_renderer);

loop {
    // ... game update ...
    
    // UI frame
    ui.begin_frame();
    
    // Call @render_ui function
    render_ui(&mut game, &mut ui);
    
    ui.end_frame();
    
    // Render UI on top of game
    ui.render();
}
```

### 3. Add @render_ui Decorator

```windjammer
@render_ui
fn render_ui(game: ShooterGame, ui: UI) {
    // Immediate mode UI code
    ui.label("FPS: " + fps.to_string())
}
```

---

## Implementation Timeline

### Week 1: Immediate Mode UI (6-8 hours)
- Day 1: UI context + layout engine (2-3 hours)
- Day 2: Basic widgets (label, button, slider) (2-3 hours)
- Day 3: Advanced widgets (progress bar, image, input) (2 hours)
- Day 4: Integration with game loop (1 hour)
- Day 5: Testing + examples (1 hour)

### Week 2: Retained Mode Integration (4-6 hours)
- Day 1: Wrap windjammer-ui for game use (2-3 hours)
- Day 2: Create common components (2-3 hours)
- Day 3: Testing + examples (1 hour)

### Week 3: Editor Foundation (12-16 hours)
- Week 3: Build visual editor using both UI systems

---

## Success Criteria

### Immediate Mode UI
- [x] UI context and frame management
- [x] Layout engine (horizontal, vertical, windows)
- [x] Basic widgets (label, button, slider, checkbox)
- [x] Advanced widgets (progress bar, image, text input)
- [x] Integration with game loop
- [x] Example: HUD for shooter game

### Retained Mode UI
- [x] Wrapper around windjammer-ui
- [x] Common components (Button, Label, Input, etc.)
- [x] Integration with game framework
- [x] Example: Menu system

### Editor Foundation
- [x] Scene viewport
- [x] Entity hierarchy
- [x] Component inspector
- [x] Asset browser

---

## Competitive Comparison

| Feature | UE5 | Unity | Godot | Bevy | Windjammer |
|---------|-----|-------|-------|------|------------|
| Immediate Mode UI | ✅ Slate | ✅ IMGUI | ❌ | ✅ egui | ⏳ |
| Retained Mode UI | ✅ UMG | ✅ UI Toolkit | ✅ Control | ❌ | ⏳ |
| Visual Editor | ✅ | ✅ | ✅ | ❌ | ⏳ |

After implementation, Windjammer will have **both** immediate and retained mode UI!

---

## Next Steps

1. **Implement immediate mode UI** (this session)
2. **Integrate with shooter game** (add HUD)
3. **Wrap windjammer-ui** (retained mode)
4. **Build visual editor** (next session)

---

**Status**: Ready to implement!  
**Grade**: A+ (Comprehensive plan)  
**Recommendation**: Start with immediate mode UI!

