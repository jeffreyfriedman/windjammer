# Windjammer VSCode Extension

Official Visual Studio Code extension for the Windjammer programming language.

## Features

### 🎨 Syntax Highlighting
Beautiful syntax highlighting for Windjammer code with proper support for:
- Keywords (`fn`, `let`, `struct`, `enum`, `match`, `go`, `async`, etc.)
- Decorators (`@route`, `@timing`, `@cache`, etc.)
- Types (`int`, `string`, `Option<T>`, `Result<T, E>`)
- Comments and strings
- Operators

### 🔍 IntelliSense
Powered by the Windjammer Language Server:
- **Auto-completion** for keywords, types, functions, and decorators
- **Hover information** showing type signatures and documentation
- **Error diagnostics** for syntax and semantic errors
- **Ownership hints** showing inferred `&`, `&mut`, or owned parameters

### 🚀 Code Navigation
- **Document outline** showing all functions, structs, and enums
- **Go to definition** (coming soon)
- **Find all references** (coming soon)

### ⚡ Fast & Responsive
- Incremental compilation using Salsa
- Only recomputes what changed
- Instant feedback as you type

## Installation

### From VSIX (Development)

1. Build the extension:
```bash
cd editors/vscode
npm install
npm run compile
```

2. Package the extension:
```bash
npx vsce package
```

3. Install in VSCode:
   - Open VSCode
   - Press `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux)
   - Type "Extensions: Install from VSIX"
   - Select the generated `.vsix` file

### From Marketplace (Coming Soon)

Search for "Windjammer" in the VSCode Extensions marketplace.

## Requirements

The Windjammer Language Server must be installed and accessible in your PATH:

```bash
cargo install --path crates/windjammer-lsp
```

Or specify a custom path in settings:

```json
{
  "windjammer.server.path": "/path/to/windjammer-lsp"
}
```

## Configuration

### Settings

- `windjammer.server.path`: Path to the language server executable
- `windjammer.trace.server`: Enable LSP tracing for debugging
- `windjammer.inlayHints.enable`: Show inferred types and ownership inline

### Example Settings

```json
{
  "windjammer.server.path": "windjammer-lsp",
  "windjammer.trace.server": "verbose",
  "windjammer.inlayHints.enable": true
}
```

## Commands

- **Windjammer: Restart Language Server** - Restart the LSP
- **Windjammer: Show Generated Rust Code** - View transpiled Rust output

## File Association

The extension automatically activates for `.wj` files.

To manually associate files:

```json
{
  "files.associations": {
    "*.wj": "windjammer"
  }
}
```

## Themes

Windjammer syntax highlighting works with all VSCode themes. For the best experience, try:
- **Dark+** (default dark theme)
- **Light+** (default light theme)
- **Monokai**
- **Dracula**

## Troubleshooting

### Language Server Not Starting

1. Check that `windjammer-lsp` is in your PATH:
```bash
which windjammer-lsp
```

2. Check the Output panel:
   - View → Output
   - Select "Windjammer Language Server" from dropdown

3. Enable LSP tracing:
```json
{
  "windjammer.trace.server": "verbose"
}
```

### No Syntax Highlighting

1. Ensure the file has a `.wj` extension
2. Try reloading VSCode: `Cmd+Shift+P` → "Reload Window"
3. Check language mode in bottom right corner

### Performance Issues

The language server is designed to be fast, but if you experience slowness:

1. Check that incremental compilation is working (should see minimal recomputation)
2. Check the Output panel for errors
3. Try restarting the language server: `Cmd+Shift+P` → "Windjammer: Restart Language Server"

## Development

### Building from Source

```bash
cd editors/vscode
npm install
npm run compile
```

### Watching for Changes

```bash
npm run watch
```

### Running Extension

Press `F5` in VSCode to open an Extension Development Host window.

## Contributing

Contributions are welcome! Please see the main Windjammer repository for guidelines.

## License

MIT

