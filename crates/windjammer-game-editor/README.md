# Windjammer Game Editor

A desktop IDE for creating games with Windjammer, built with Tauri.

## Features

- 🎮 **Game Project Management** - Create and manage Windjammer game projects
- 📝 **Code Editor** - Edit Windjammer game code with monospace font
- 🗂️ **File Browser** - Navigate project files
- ▶️ **Run Games** - Compile and run games directly from the editor
- 🖥️ **Console Output** - View compilation results and errors
- 🎨 **VS Code Theme** - Dark theme inspired by VS Code

## Quick Start

### Build and Run

```bash
# From the windjammer-game-editor directory
cargo run
```

### Create Your First Game

1. Click **"New Project"**
2. Enter project name (e.g., "MyGame")
3. Choose project location (e.g., "/tmp")
4. Click on `main.wj` in the file tree
5. Edit your game code
6. Click **"Save"**
7. Click **"Run"** to compile and run

## Game Template

New projects are created with a complete 2D game template:

```windjammer
use std::game::*

@game(renderer = "2d")
struct MyGame {
    player_x: f32,
    player_y: f32,
}

@init
fn init() -> MyGame {
    MyGame {
        player_x: 400.0,
        player_y: 300.0,
    }
}

@update
fn update(mut game: MyGame, input: Input, dt: f32) {
    // Handle input
    if input.is_key_down(Key::Left) {
        game.player_x -= 200.0 * dt
    }
    // ... more input handling
}

@render
fn render(game: MyGame, renderer: Renderer) {
    renderer.clear(Color::rgb(0.1, 0.1, 0.15))
    renderer.draw_rect(
        game.player_x - 25.0,
        game.player_y - 25.0,
        50.0,
        50.0,
        Color::rgb(0.2, 0.8, 0.3)
    )
}
```

## Architecture

### Backend (Rust + Tauri)
- File system operations
- Project creation
- Compiler integration
- Process management

### Frontend (HTML/CSS/JS)
- User interface
- Code editing
- Console output
- File tree navigation

*Note: Future versions will use pure Windjammer + windjammer-ui for the frontend.*

## Development

### Build
```bash
cargo build
```

### Run Tests
```bash
cargo test
```

### Development Mode
```bash
cargo tauri dev
```

## Testing

The editor includes integration tests for:
- Project template creation
- File operations (read/write)
- Directory listing

Run tests with:
```bash
cargo test
```

## Documentation

See `docs/` for comprehensive documentation:
- `GAME_EDITOR_IMPLEMENTATION.md` - Full architecture and usage
- `GAME_EDITOR_TESTING_STRATEGY.md` - Testing plan
- `GAME_EDITOR_COMPLETE.md` - Implementation summary

## Requirements

- Rust 1.90+
- Tauri 2.1+
- Windjammer compiler (in PATH or `../../target/debug/windjammer`)

## Keyboard Shortcuts

*Coming soon*

## Known Issues

- Game process management needs improvement
- Syntax highlighting not yet implemented
- File tree doesn't support expand/collapse

See `docs/GAME_EDITOR_IMPLEMENTATION.md` for full list.

## Contributing

This is a dogfooding project to validate the windjammer-ui framework. Contributions welcome!

## License

MIT OR Apache-2.0

