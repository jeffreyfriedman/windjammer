# Windjammer Game Editor

A production-grade game editor for the Windjammer Game Framework, built with AAA features and professional polish.

## Features

### Core Editor
- **Professional Docking System**: Fully customizable panel layout with drag-and-drop docking
- **File Management**: Project browser with file tree navigation
- **Code Editor**: Syntax highlighting for Windjammer code
- **Scene Editor**: 3D scene visualization and manipulation
- **Properties Panel**: Real-time property editing for game objects
- **Console**: Build output and runtime logs
- **Asset Browser**: Visual asset management (coming soon)

### Game Framework Panels (Dockable)
All game framework panels are fully dockable and can be arranged to your preference:

- **🎨 PBR Material Editor**: Physically-based rendering material configuration
- **✨ Post-Processing**: Visual effects and post-processing pipeline
- **📊 Performance Profiler**: Real-time performance analysis and optimization
- **✨ Particle System Editor**: Visual particle effect creation
- **🎬 Animation Editor**: Animation state machine (coming soon)
- **🏔️ Terrain Editor**: Heightmap-based terrain editing (coming soon)
- **🤖 AI Behavior Tree**: Visual AI behavior design (coming soon)
- **🔊 Audio Mixer**: 3D audio and mixing (coming soon)
- **🎮 Gamepad Config**: Controller configuration (coming soon)
- **🔫 Weapon Editor**: FPS/TPS weapon system (coming soon)
- **🗺️ NavMesh Editor**: Navigation mesh tools (coming soon)

### Platform Support
- **Desktop**: Native performance with egui (macOS, Linux, Windows)
- **Browser**: WASM-based editor (planned)

## Usage

### Running the Editor

```bash
# From the Windjammer workspace root
cargo run --package windjammer-game-editor --bin editor --features desktop

# Or in release mode for better performance
cargo run --package windjammer-game-editor --bin editor --features desktop --release
```

### Creating a New Project

1. Launch the editor
2. Click **File → New Project**
3. Choose project location
4. Start building your game!

### Opening Game Framework Panels

1. Click **View** in the menu bar
2. Select any game framework panel
3. Panel appears as a dockable tab
4. Drag to rearrange, dock, or undock

## Architecture

### Unified Design
- **Single Editor Binary**: One production-grade editor (`editor`)
- **Dockable Panels**: All panels use the same docking system
- **Shared Code**: Maximum code reuse between desktop and browser
- **Clean API**: No "professional" or "enhanced" variants - just one excellent editor

### Technology Stack
- **UI Framework**: egui with egui_dock for professional docking
- **Rendering**: wgpu for 3D scene visualization
- **Game Framework**: windjammer-game-framework for all game features
- **Code Sharing**: windjammer-ui for cross-platform components

## Development Status

- ✅ Core editor with docking system
- ✅ File management and code editor
- ✅ Scene editor with 3D visualization
- ✅ Properties panel and console
- ✅ Game framework panel integration (dockable)
- ✅ PBR Material Editor (implemented)
- ✅ Post-Processing Editor (implemented)
- ✅ Performance Profiler (implemented)
- ✅ Particle System Editor (implemented)
- 🚧 Animation, Terrain, AI, Audio, Gamepad, Weapon, NavMesh editors (in progress)
- 🚧 Browser/WASM version (planned)

## Contributing

The editor is designed to be:
- **Professional**: AAA-quality features and polish
- **Unified**: One way to do things, following Windjammer philosophy
- **Extensible**: Easy to add new panels and features
- **Cross-platform**: Desktop now, browser coming soon

All game framework panels are in `crates/windjammer-game-editor/src/panels/` and follow a consistent pattern for easy contribution.
