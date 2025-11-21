# Windjammer Language Support for VS Code

**80% of Rust's power with 20% of its complexity**

This extension provides comprehensive language support for Windjammer, including:

## Features

### 🎯 World-Class Error Messages
- **Smart Error Translation** - Rust errors automatically translated to Windjammer terminology
- **Error Codes** - Unique `WJxxxx` codes for every error type
- **Contextual Help** - Actionable suggestions for every error
- **Auto-Fix** - Automatic fixes for common issues
- **Explain Command** - `wj explain WJ0001` for detailed help

### 💡 Intelligent Code Completion
- Context-aware suggestions
- Standard library completion
- User-defined symbols
- Method and field completion

### 🔍 Code Navigation
- Go to Definition (F12)
- Find All References
- Hover Information
- Symbol Search

### ✨ Inlay Hints (Unique!)
- See inferred ownership modes inline (`&`, `&mut`, `owned`)
- Educational for learners, validating for experts

### 🔧 Refactoring
- Extract Function
- Inline Variable
- Introduce Variable
- Change Signature
- Move Items
- Rename Symbol

### 🎨 Syntax Highlighting
- Beautiful, accurate syntax highlighting
- Semantic tokens for advanced coloring

### 🐛 Debugging
- Set breakpoints in `.wj` files
- Step through code
- Inspect variables
- Source mapping (Windjammer ↔ Rust)

## Requirements

- Windjammer compiler installed (`cargo install windjammer`)
- Windjammer LSP server (`windjammer-lsp` binary in PATH)

## Installation

1. Install the extension from the VS Code Marketplace
2. Install Windjammer: `cargo install windjammer`
3. Verify LSP server: `which windjammer-lsp`

## Extension Settings

This extension contributes the following settings:

* `windjammer.lsp.path`: Path to the windjammer-lsp executable
* `windjammer.lsp.trace.server`: Trace LSP communication (off/messages/verbose)
* `windjammer.diagnostics.enable`: Enable enhanced diagnostics
* `windjammer.inlayHints.enable`: Enable inlay hints for ownership modes
* `windjammer.autoFix.enable`: Enable auto-fix suggestions

## Commands

* `Windjammer: Restart Language Server` - Restart the LSP server
* `Windjammer: Explain Error Code` - Explain error at cursor
* `Windjammer: Show Error Catalog` - Open error documentation

## Usage

### Quick Start

1. Create a file with `.wj` extension
2. Start typing - autocomplete will activate
3. Hover over symbols for information
4. Press F12 to go to definition
5. Use Ctrl+Space for completions

### Error Codes

When you see an error like `error[WJ0002]`, you can:
- Hover to see contextual help
- Run `Windjammer: Explain Error Code` command
- Click the lightbulb for quick fixes

### Inlay Hints

Enable inlay hints to see ownership modes:

```windjammer
fn process(data: string /* & */, config: Config /* &mut */) {
    // Hints show the compiler's inference
}
```

## Known Issues

- LSP server requires Rust compiler for full functionality
- Some refactorings may require manual review

## Release Notes

### 0.35.0

- Initial release
- Full LSP support
- World-class error messages
- Inlay hints for ownership
- Advanced refactoring
- Debugging support

## More Information

- [Windjammer Website](https://windjammer.dev)
- [GitHub Repository](https://github.com/jeffreyfriedman/windjammer)
- [Documentation](https://windjammer.dev/docs)
- [Error Catalog](https://windjammer.dev/errors)

## License

MIT OR Apache-2.0

