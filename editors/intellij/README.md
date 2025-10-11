# Windjammer for IntelliJ IDEA

Language support for Windjammer in IntelliJ IDEA and other JetBrains IDEs using LSP.

## Requirements

- IntelliJ IDEA 2023.1+ (Community or Ultimate)
- Other JetBrains IDEs: CLion, PyCharm, WebStorm, etc.
- `windjammer-lsp` binary installed:
  ```bash
  cargo install windjammer
  ```

## Installation

### Step 1: Install LSP4IJ Plugin

1. Open IntelliJ IDEA
2. Go to **Settings** (Cmd+, / Ctrl+Alt+S)
3. Navigate to **Plugins**
4. Search for "LSP4IJ" or "LSP Support"
5. Click **Install** and restart IDE

> **Note**: LSP4IJ provides Language Server Protocol support for JetBrains IDEs

### Step 2: Configure File Type

1. Go to **Settings** → **Editor** → **File Types**
2. Click **+** to add new file type
3. Name: `Windjammer`
4. Description: `Windjammer Language`
5. Line comment: `//`
6. Block comment start: `/*`
7. Block comment end: `*/`
8. Add file pattern: `*.wj`
9. Click **OK**

### Step 3: Configure LSP Server

1. Go to **Settings** → **Languages & Frameworks** → **Language Server Protocol** → **Server Definitions**
2. Click **+** to add new server
3. Configure:
   - **Extension/File name pattern**: `*.wj`
   - **Command**: `windjammer-lsp`
   - **Configuration**: Leave empty or add:
     ```json
     {
       "inlayHints": {
         "enable": true
       }
     }
     ```
4. Click **OK**
5. Apply and close Settings

### Step 4: Verify Setup

1. Create a new file with `.wj` extension
2. Start typing Windjammer code
3. LSP server should start automatically (check status bar)

## Features

Once configured, you'll get:

- ✅ **Auto-completion** - Intelligent code completion
- ✅ **Go to Definition** - Navigate to symbol definitions (Cmd+B / Ctrl+B)
- ✅ **Find Usages** - Find all references (Alt+F7)
- ✅ **Rename Symbol** - Safe refactoring (Shift+F6)
- ✅ **Hover Information** - Type and signature information
- ✅ **Error Highlighting** - Real-time diagnostics
- ✅ **Inlay Hints** - Ownership inference annotations ✨

## Syntax Highlighting

For better syntax highlighting, you can create a custom TextMate bundle:

### Option 1: Use TextMate Bundle (Recommended)

1. Download the Windjammer TextMate bundle from the VSCode extension
2. Go to **Settings** → **Editor** → **TextMate Bundles**
3. Click **+** and select the bundle directory
4. Restart IDE

### Option 2: Manual Syntax Highlighting

1. Go to **Settings** → **Editor** → **Color Scheme** → **Language Defaults**
2. Customize colors for:
   - Keywords
   - Types
   - Strings
   - Comments
   - Numbers

## Key Bindings

Standard IntelliJ key bindings work with Windjammer:

| Action | Windows/Linux | macOS |
|--------|--------------|-------|
| Go to Definition | Ctrl+B | Cmd+B |
| Find Usages | Alt+F7 | Alt+F7 |
| Rename | Shift+F6 | Shift+F6 |
| Show Hover | Ctrl+Q | Ctrl+J |
| Code Completion | Ctrl+Space | Ctrl+Space |
| Parameter Info | Ctrl+P | Cmd+P |
| Quick Documentation | Ctrl+Q | F1 |

## Configuration Options

### Enable/Disable Inlay Hints

1. Go to **Settings** → **Editor** → **Inlay Hints**
2. Find **Windjammer** section
3. Toggle ownership inference hints

### Adjust LSP Timeout

If the LSP server is slow to start:

1. Go to **Settings** → **Languages & Frameworks** → **Language Server Protocol**
2. Increase **Server start timeout**
3. Default: 10 seconds, increase if needed

### Custom LSP Server Path

If `windjammer-lsp` is not in PATH:

1. Go to server configuration
2. Change **Command** to full path:
   - Windows: `C:\path\to\windjammer-lsp.exe`
   - macOS/Linux: `/full/path/to/windjammer-lsp`

## Troubleshooting

### LSP Server Not Starting

**Check logs:**
1. **Help** → **Show Log in Finder/Explorer**
2. Look for LSP4IJ or language server errors

**Verify binary:**
```bash
which windjammer-lsp  # macOS/Linux
where windjammer-lsp  # Windows
```

**Manual start test:**
```bash
windjammer-lsp
```

Should show: "Starting Windjammer Language Server"

### No Code Completion

1. Check LSP server is running (status bar)
2. Restart LSP server: **Tools** → **LSP** → **Restart Servers**
3. Invalidate caches: **File** → **Invalidate Caches / Restart**

### File Type Not Recognized

1. Right-click `.wj` file → **Associate with File Type**
2. Select "Windjammer"
3. Restart IDE if needed

### Inlay Hints Not Showing

1. Enable inlay hints globally: **Settings** → **Editor** → **Inlay Hints** → **Enable inlay hints**
2. Check Windjammer-specific hints are enabled
3. Restart LSP server

## Alternative: Use Rust Plugin

As a fallback, you can use the Rust plugin with custom file associations:

1. Install **Rust** plugin
2. Go to **Settings** → **Editor** → **File Types**
3. Find **Rust File** type
4. Add `*.wj` pattern
5. Limited support, but basic syntax highlighting

## Example Project Structure

```
my-windjammer-project/
├── src/
│   ├── main.wj
│   ├── lib.wj
│   └── utils/
│       └── helper.wj
└── wj.toml
```

## Features Comparison

| Feature | Support Level |
|---------|--------------|
| Syntax Highlighting | ⚠️ Basic (via TextMate) |
| Auto-completion | ✅ Full |
| Go to Definition | ✅ Full |
| Find Usages | ✅ Full |
| Rename Refactoring | ✅ Full |
| Inlay Hints | ✅ Full (Ownership!) |
| Debugging | ❌ Not yet supported |
| Run Configurations | ❌ Use terminal |

## Contributing

Want to help improve Windjammer support for IntelliJ?

- [Report issues](https://github.com/jeffreyfriedman/windjammer/issues)
- [Contribute to LSP server](https://github.com/jeffreyfriedman/windjammer)
- Create a full native IntelliJ plugin (Help wanted!)

## Future Plans

- ✨ Native IntelliJ plugin (no LSP4IJ dependency)
- 🎨 Custom syntax highlighter
- 🐛 Integrated debugging
- ▶️ Run configurations
- 🧪 Test integration

## License

MIT OR Apache-2.0

---

**Note**: IntelliJ support is currently provided through LSP4IJ. For the best experience, we recommend using VSCode or Vim/Neovim until a native plugin is available.

