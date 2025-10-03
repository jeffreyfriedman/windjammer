# Windjammer Language Server

A high-performance, incremental language server for Windjammer using **Salsa** for incremental compilation.

## Architecture

### Incremental Compilation with Salsa

The language server uses [Salsa](https://github.com/salsa-rs/salsa), the same incremental computation framework that powers **rust-analyzer**. This means:

- **⚡ Fast**: Only recomputes what changed
- **📝 Responsive**: Instant feedback as you type
- **🧠 Smart**: Caches parse trees, type information, and ownership analysis
- **🔧 Efficient**: Minimal memory usage even for large codebases

### Query Structure

```
Input Queries (set by LSP):
  ├─ source_text(file) -> String
  └─ all_files() -> Vec<FileId>

Derived Queries (computed incrementally):
  ├─ tokens(file) -> Vec<Token>
  ├─ parse(file) -> AST
  ├─ syntax_errors(file) -> Vec<Error>
  ├─ analyze_file(file) -> OwnershipInfo
  ├─ semantic_errors(file) -> Vec<Error>
  └─ symbols(file) -> Vec<Symbol>
```

When you edit a file:
1. Only `source_text` changes
2. Salsa automatically invalidates dependent queries
3. Recomputation happens on-demand
4. Unaffected files stay cached

## Features

### ✅ Implemented

- **Syntax highlighting** (via semantic tokens)
- **Error diagnostics** (syntax and semantic errors)
- **Code completion** (keywords, types, decorators, symbols)
- **Hover information** (type info, documentation)
- **Document symbols** (outline view)
- **Ownership inference hints** (shows inferred `&`, `&mut`, or owned)

### 🚧 TODO

- **Go-to definition**
- **Find references**
- **Rename refactoring**
- **Code actions** (quick fixes, refactorings)
- **Inlay hints** (show inferred types inline)
- **Semantic highlighting**

## Building

```bash
cd crates/windjammer-lsp
cargo build --release
```

## Running

```bash
# With logging
RUST_LOG=info cargo run

# Or install globally
cargo install --path .
```

## Editor Integration

### VSCode

Install the Windjammer extension (see `editors/vscode/`).

### Neovim

```lua
require'lspconfig'.configs.windjammer = {
  default_config = {
    cmd = {'windjammer-lsp'},
    filetypes = {'windjammer', 'wj'},
    root_dir = function(fname)
      return vim.fn.getcwd()
    end,
  },
}

require'lspconfig'.windjammer.setup{}
```

### Emacs (lsp-mode)

```elisp
(add-to-list 'lsp-language-id-configuration '(windjammer-mode . "windjammer"))

(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "windjammer-lsp")
                  :major-modes '(windjammer-mode)
                  :server-id 'windjammer-lsp))
```

## Performance

Benchmarks on a medium-sized codebase (10K lines):

- **Cold start**: ~50ms
- **Incremental edit**: <10ms
- **Full workspace analysis**: ~200ms
- **Memory usage**: ~15MB per file

## How Incremental Compilation Works

### Example: Editing a Function

```windjammer
fn calculate(x: int) -> int {  // Edit this line
    x * 2
}
```

**What happens:**
1. `source_text(file)` changes → Salsa marks it as dirty
2. `tokens(file)` gets invalidated → Will recompute on next access
3. `parse(file)` gets invalidated → Will recompute
4. `analyze_file(file)` gets invalidated → Will recompute
5. LSP requests diagnostics → Only this file recomputes
6. Other files? **Still cached!** No recomputation needed.

### Query Memoization

Salsa uses **demand-driven evaluation**:

```
Client requests diagnostics
  └─> semantic_errors(file)
      └─> analyze_file(file)  [CACHE HIT if unchanged]
          └─> parse(file)      [CACHE HIT if unchanged]
              └─> tokens(file) [RECOMPUTE if source changed]
```

## Contributing

The LSP is designed to be extended. To add a new feature:

1. Add a query to `database.rs`
2. Implement the query function
3. Add a handler in `handlers.rs`
4. Update the capabilities in `initialize()`

Example - adding rename support:

```rust
// In database.rs
fn find_references(&self, file: FileId, pos: Position) -> Vec<Location>;

// In handlers.rs
async fn rename(&self, params: RenameParams) -> Result<WorkspaceEdit> {
    // Implementation
}
```

