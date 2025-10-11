# Windjammer for Visual Studio Code

The official Visual Studio Code extension for the Windjammer programming language.

## Features

### 🌊 **Syntax Highlighting**
- Full syntax highlighting for Windjammer (`.wj` files)
- Color-coded keywords, types, strings, comments, decorators
- Smart indentation and bracket matching

### 🧠 **Language Server Protocol (LSP)**
- **Real-time Diagnostics** - Errors and warnings as you type
- **Auto-completion** - Intelligent code completion for keywords, stdlib, and user code
- **Go to Definition** - Jump to function, struct, enum, and trait definitions
- **Find References** - Find all usages of a symbol
- **Rename Symbol** - Safe refactoring across your codebase
- **Hover Information** - See function signatures and type information
- **Code Actions** - Quick fixes, extract function, inline variable

### 🐛 **Debug Adapter Protocol (DAP)**
- **Breakpoints** - Set breakpoints in `.wj` files
- **Step Through Code** - Step over, step into, step out
- **Variable Inspection** - View local variables and their values
- **Call Stack** - Navigate the call stack
- **Watch Expressions** - Evaluate expressions in debug context
- **Source Mapping** - Maps Windjammer code to generated Rust for seamless debugging

### ✨ **Ownership Inference Hints** (Unique!)
- See inferred `&` (borrowed), `&mut` (mutable borrow), and `owned` (moved) annotations inline
- **No other language shows this!** - Makes Rust-like ownership intuitive
- Real-time feedback on ownership decisions

### 📝 **Code Snippets**
- Quick scaffolding for common patterns:
  - `fn` - Function declaration
  - `struct` - Struct definition
  - `enum` - Enum definition
  - `impl` - Implementation block
  - `match` - Match expression
  - `for` - For loop
  - `test` - Test function
  - And many more!

## Installation

### From VSCode Marketplace (coming soon)
1. Open VSCode
2. Go to Extensions (Cmd+Shift+X / Ctrl+Shift+X)
3. Search for "Windjammer"
4. Click Install

### From VSIX (manual)
1. Download the `.vsix` file from releases
2. In VSCode, go to Extensions
3. Click the `...` menu → "Install from VSIX..."
4. Select the downloaded `.vsix` file

### From Source
```bash
cd editors/vscode
npm install
npm run compile
npm run package
code --install-extension windjammer-0.19.0.vsix
```

## Requirements

- **Windjammer LSP Server**: Install the `windjammer-lsp` binary:
  ```bash
  cargo install windjammer
  ```
  
- The extension will automatically start the LSP server when you open a `.wj` file.

## Configuration

Configure the extension in VSCode settings:

```json
{
  // Path to the Windjammer LSP server binary
  "windjammer.lsp.serverPath": "windjammer-lsp",
  
  // Enable ownership inference hints (shows &, &mut, owned inline)
  "windjammer.inlayHints.enable": true,
  
  // Enable auto-completion
  "windjammer.completion.enable": true,
  
  // Trace LSP communication (for debugging)
  "windjammer.lsp.trace.server": "off" // "off" | "messages" | "verbose"
}
```

## Usage

1. Create a new file with `.wj` extension
2. Start typing Windjammer code
3. Enjoy real-time diagnostics, completion, and ownership hints!

### Example

```windjammer
fn greet(name: string) {
    println!("Hello, {}!", name)
}

fn main() {
    greet("World")
}
```

## Commands

- **Restart Language Server**: `Windjammer: Restart Language Server` (Cmd+Shift+P)
- **Extract Function**: Right-click selected code → "Extract Function"
- **Inline Variable**: Right-click variable usage → "Inline Variable"

## Debugging

### Quick Start
1. Open a `.wj` file
2. Click in the gutter to set a breakpoint (red dot)
3. Press F5 or go to Run → Start Debugging
4. Use the debug toolbar to step through your code

### Debug Configuration
Create or edit `.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "windjammer",
      "request": "launch",
      "name": "Debug Windjammer Program",
      "program": "${workspaceFolder}/build_output/target/debug/${workspaceFolderBasename}",
      "cwd": "${workspaceFolder}",
      "preLaunchTask": "windjammer: build"
    }
  ]
}
```

### Debug Features
- **Breakpoints**: Click gutter to set/unset
- **Conditional Breakpoints**: Right-click breakpoint → "Edit Breakpoint"
- **Step Over (F10)**: Execute current line, don't enter functions
- **Step Into (F11)**: Enter function calls
- **Step Out (Shift+F11)**: Return to caller
- **Continue (F5)**: Resume execution until next breakpoint
- **Variable Inspection**: Hover over variables or check Debug sidebar
- **Watch Expressions**: Add expressions to watch their values change

## Troubleshooting

### LSP server not starting
- Check that `windjammer-lsp` is in your PATH:
  ```bash
  which windjammer-lsp
  ```
- Set the full path in settings:
  ```json
  {
    "windjammer.lsp.serverPath": "/full/path/to/windjammer-lsp"
  }
  ```

### Syntax highlighting not working
- Make sure the file extension is `.wj`
- Reload VSCode: `Developer: Reload Window`

### Ownership hints not showing
- Enable inlay hints in settings:
  ```json
  {
    "windjammer.inlayHints.enable": true
  }
  ```
- Check VSCode's native inlay hints are enabled:
  ```json
  {
    "editor.inlayHints.enabled": "on"
  }
  ```

## Contributing

Found a bug or have a feature request? [Open an issue](https://github.com/jeffreyfriedman/windjammer/issues)!

## License

MIT OR Apache-2.0
