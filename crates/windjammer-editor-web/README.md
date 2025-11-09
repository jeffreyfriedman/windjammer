# Windjammer Web Editor

A web-based code editor for the Windjammer programming language, built with Rust and WebAssembly.

## Features

- ✅ **Code Editor** - Write Windjammer code in your browser
- ✅ **Syntax Highlighting** - Clear, readable code
- ✅ **File Browser** - Navigate project files
- ✅ **Error Display** - World-class error messages
- ✅ **Local Storage** - Save projects in your browser
- ✅ **Live Compilation** - See errors in real-time
- ✅ **Small Bundle** - 2-10MB (vs 2GB+ for Unity/Unreal)

## Architecture

```
┌─────────────────────────────────────┐
│         Web Editor (WASM)           │
├─────────────────────────────────────┤
│  - Code Editor                      │
│  - File Browser                     │
│  - Error Display                    │
│  - Project Management               │
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│      Windjammer Compiler (WASM)     │
├─────────────────────────────────────┤
│  - Lexer                            │
│  - Parser                           │
│  - Analyzer                         │
│  - Codegen                          │
└─────────────────────────────────────┘
```

## Building

### Prerequisites

- Rust (latest stable)
- wasm-pack (`cargo install wasm-pack`)
- Node.js (for serving)

### Build Steps

```bash
# Build the WASM package
wasm-pack build --target web --out-dir pkg

# Serve locally
python3 -m http.server 8080
# Or use any other HTTP server

# Open in browser
open http://localhost:8080
```

### Development Build

```bash
# Build in development mode (faster, larger)
wasm-pack build --target web --dev --out-dir pkg
```

### Production Build

```bash
# Build in release mode (slower, smaller)
wasm-pack build --target web --release --out-dir pkg
```

## Usage

### Basic Usage

1. Open `index.html` in your browser
2. Write Windjammer code in the editor
3. Click "Run" to compile
4. See errors in the right panel
5. Click "Save" to save to local storage

### Keyboard Shortcuts

- `Ctrl+S` / `Cmd+S` - Save project
- `Ctrl+Enter` / `Cmd+Enter` - Run project
- `Ctrl+N` / `Cmd+N` - New project

## Project Structure

```
windjammer-editor-web/
├── src/
│   ├── lib.rs              # Main entry point
│   ├── editor.rs           # Code editor component
│   ├── file_browser.rs     # File browser component
│   ├── error_display.rs    # Error display component
│   ├── project.rs          # Project management
│   └── compiler_bridge.rs  # Compiler integration
├── index.html              # Main HTML file
├── styles.css              # Styles
├── Cargo.toml              # Rust dependencies
└── README.md               # This file
```

## Roadmap

### v0.1 (Current)
- [x] Basic code editor
- [x] File browser
- [x] Error display
- [x] Local storage
- [ ] Compiler integration

### v0.2 (Next)
- [ ] Syntax highlighting
- [ ] Auto-completion
- [ ] Live preview
- [ ] Multiple files
- [ ] Keyboard shortcuts

### v0.3 (Future)
- [ ] Debugging tools
- [ ] Profiling
- [ ] Git integration
- [ ] Collaborative editing
- [ ] Cloud storage

## Competitive Comparison

| Editor | Platform | Bundle Size | Offline | Open Source |
|--------|----------|-------------|---------|-------------|
| **Windjammer Web** | Web | 2-10MB | ✅ | ✅ |
| Unity Studio | Web | Browser | ❌ | ❌ |
| Babylon.js Editor | Web | Browser | ❌ | ✅ |
| VS Code Web | Web | Browser | ❌ | ✅ |

## Contributing

Contributions are welcome! Please see the main Windjammer repository for guidelines.

## License

MIT OR Apache-2.0

## Links

- [Windjammer Repository](https://github.com/windjammer-lang/windjammer)
- [Documentation](https://windjammer-lang.org/docs)
- [Discord Community](https://discord.gg/windjammer)

---

**"Code anywhere, anytime, in any browser!"** 🌐

