# Windjammer Game Editor

A full-featured game editor for the Windjammer Game Framework, available in both desktop and browser versions.

## Features

### Desktop Editor (egui-based)
- Native performance
- Full file system access
- Advanced debugging tools
- Integrated profiler
- Scene editor
- Asset browser
- Code editor with syntax highlighting
- Property inspector
- Console output
- Game preview

### Browser Editor (WASM-based)
- Cross-platform (runs anywhere)
- No installation required
- Cloud-based projects
- Collaborative editing (future)
- Same features as desktop (with browser limitations)

## Architecture

Both editors share:
- Core editor logic
- UI components
- Project management
- Asset pipeline
- Scene management

Platform-specific:
- Desktop: Native file I/O, process spawning
- Browser: IndexedDB storage, Web Workers

## Usage

### Desktop Editor
```bash
wj editor
# or
wj editor --project path/to/project
```

### Browser Editor
Navigate to: `http://localhost:8080/editor.html`

## Development

The editor is built using:
- **Desktop**: egui for native UI
- **Browser**: egui + WASM for web UI
- **Shared**: windjammer-game-framework for game logic

## Status

- ✅ Desktop Editor: In development
- 🚧 Browser Editor: Planned
