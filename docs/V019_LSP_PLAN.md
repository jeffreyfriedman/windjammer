# Windjammer v0.19.0: Language Server Protocol Implementation Plan

**Goal**: Build a world-class language server with real-time ownership inference hints and full editor support.

**Status**: In Progress  
**Target**: VSCode, Vim/Neovim, IntelliJ IDEA  
**Timeline**: 6-8 weeks

---

## 🎯 Vision

Create the most delightful developer experience for a systems programming language by:
1. **Real-time feedback** - See errors and type info as you type
2. **Ownership visibility** - Show inferred `&`, `&mut` inline (unique!)
3. **Universal support** - Works in all major editors
4. **Full IDE features** - Refactoring, debugging, everything

---

## 📦 Architecture

### Core Components

```
crates/
├── windjammer-lsp/          # LSP server (tower-lsp)
│   ├── src/
│   │   ├── main.rs          # LSP server entry point
│   │   ├── server.rs        # LSP request handlers
│   │   ├── analysis.rs      # Incremental analysis (Salsa)
│   │   ├── diagnostics.rs   # Error/warning generation
│   │   ├── completion.rs    # Auto-completion logic
│   │   ├── hover.rs         # Hover information
│   │   ├── goto.rs          # Go to definition/references
│   │   ├── refactor.rs      # Refactoring tools
│   │   └── inlay_hints.rs   # Ownership inference hints
│   └── Cargo.toml
│
editors/
├── vscode/                  # VSCode extension
│   ├── src/extension.ts     # LSP client
│   ├── syntaxes/            # Improved syntax highlighting
│   └── snippets/            # Code snippets
│
├── vim/                     # Vim/Neovim support
│   ├── syntax/              # Syntax files
│   ├── ftdetect/            # File type detection
│   └── lsp-config.lua       # Neovim LSP config
│
└── intellij/                # IntelliJ plugin
    ├── src/                 # Plugin code
    └── plugin.xml           # Plugin descriptor
```

### Technology Stack

**LSP Server:**
- `tower-lsp` - LSP protocol implementation
- `salsa` - Incremental compilation framework
- `lsp-types` - LSP type definitions
- `tokio` - Async runtime
- `dashmap` - Concurrent hash maps
- `notify` - File watching

**VSCode Extension:**
- TypeScript
- `vscode-languageclient` - LSP client

**Vim/Neovim:**
- Lua (Neovim LSP config)
- VimScript (syntax files)

**IntelliJ:**
- Kotlin
- IntelliJ Platform SDK
- LSP4IJ for LSP integration

---

## 🚀 Implementation Phases

### Phase 1: Core LSP Infrastructure (Weeks 1-2)

**Goal**: Basic LSP server with real-time diagnostics

#### 1.1 Project Setup
- [x] Create `crates/windjammer-lsp/` directory
- [ ] Add dependencies to `Cargo.toml`
- [ ] Set up `tower-lsp` server boilerplate
- [ ] Implement LSP lifecycle (initialize, shutdown)

#### 1.2 Salsa Integration
- [ ] Define Salsa database for incremental compilation
- [ ] Port existing compiler to Salsa queries
- [ ] Implement file watching with `notify`
- [ ] Cache parsed ASTs and analysis results

#### 1.3 Real-time Diagnostics
- [ ] Generate diagnostics from compiler errors
- [ ] Map Rust error spans to Windjammer source
- [ ] Publish diagnostics to LSP client
- [ ] Support incremental re-analysis on file changes

#### 1.4 Hover Information
- [ ] Extract type information for symbols
- [ ] Show inferred ownership (`&`, `&mut`, owned)
- [ ] Display function signatures
- [ ] Show documentation comments

**Deliverable**: Basic LSP with real-time errors and hover info

---

### Phase 2: Code Intelligence (Weeks 3-4)

**Goal**: Auto-completion, navigation, and ownership hints

#### 2.1 Auto-completion
- [ ] Keyword completion (fn, let, struct, etc.)
- [ ] Stdlib module/function completion
- [ ] User-defined types and functions
- [ ] Trait method completion
- [ ] Context-aware suggestions (after `.`, `::`)

#### 2.2 Go to Definition
- [ ] Navigate to function definitions
- [ ] Navigate to type definitions
- [ ] Navigate to trait definitions
- [ ] Navigate to module definitions
- [ ] Navigate to stdlib sources (if available)

#### 2.3 Find References
- [ ] Find all usages of a symbol
- [ ] Cross-file reference search
- [ ] Show usage context
- [ ] Filter by reference type (read/write/call)

#### 2.4 Rename Symbol
- [ ] Safe rename across all files
- [ ] Update imports and module references
- [ ] Preview changes before applying
- [ ] Rollback support

#### 2.5 Ownership Inference Hints ✨ (Unique Feature!)
- [ ] Detect inferred parameter ownership
- [ ] Show inline hints: `fn process(s: string /* inferred: & */) { ... }`
- [ ] Configurable visibility (on/off)
- [ ] Color-coded by ownership type
- [ ] Click to see inference reasoning

**Deliverable**: Full code intelligence with unique ownership hints

---

### Phase 3: Editor Extensions (Weeks 5-6)

**Goal**: Polished extensions for VSCode, Vim/Neovim, IntelliJ

#### 3.1 VSCode Extension (Priority 1)
- [ ] Improved syntax highlighting (semantic tokens)
- [ ] Integrate LSP client
- [ ] Code snippets (fn, struct, impl, match, etc.)
- [ ] Syntax theme support (dark/light)
- [ ] Extension icon and branding
- [ ] Publish to VSCode Marketplace

**Features:**
- Auto-completion with fuzzy matching
- Go to definition (Ctrl+Click / F12)
- Find references (Shift+F12)
- Rename (F2)
- Hover tooltips
- Inline ownership hints
- Real-time error checking
- Format on save (via `wj fmt`)

#### 3.2 Vim/Neovim Support (Priority 2)
- [ ] Syntax files (`syntax/windjammer.vim`)
- [ ] File type detection (`ftdetect/windjammer.vim`)
- [ ] Neovim LSP config (`lsp-config.lua`)
- [ ] CoC.nvim integration (for Vim 8+)
- [ ] Documentation and setup guide

**Features:**
- Native Neovim LSP support
- CoC.nvim for classic Vim
- All LSP features available
- Keyboard-friendly workflows

#### 3.3 IntelliJ Plugin (Priority 3)
- [ ] Basic syntax highlighting
- [ ] LSP integration via LSP4IJ
- [ ] File type registration (`.wj`)
- [ ] Icon and branding
- [ ] Publish to JetBrains Marketplace

**Features:**
- Syntax highlighting
- LSP features (completion, goto, etc.)
- IntelliJ UI integration
- Debug configuration templates

**Deliverable**: Published extensions for all three editors

---

### Phase 4: Advanced Features (Weeks 7-8)

**Goal**: Refactoring tools and debugging support

#### 4.1 Refactoring Tools
- [ ] **Extract Function** - Extract selection to new function
- [ ] **Inline Function** - Inline function calls
- [ ] **Move Module** - Reorganize module structure
- [ ] **Extract Variable** - Extract expression to variable
- [ ] **Introduce Parameter** - Pass value as parameter
- [ ] **Change Signature** - Update function signatures

**Code Actions (Quick Fixes):**
- [ ] Add missing import
- [ ] Implement missing trait methods
- [ ] Fix ownership annotation (`& -> &mut`)
- [ ] Convert `if` to `match`
- [ ] Add `@derive` decorator

#### 4.2 Debug Adapter Protocol (DAP)
- [ ] DAP server implementation
- [ ] Integration with `rust-lldb` / `gdb`
- [ ] Breakpoint support
- [ ] Variable inspection
- [ ] Step through Windjammer code (mapped to Rust)
- [ ] Watch expressions
- [ ] Debug console

**VSCode Debugging:**
- [ ] Launch configurations
- [ ] Debug toolbar integration
- [ ] Variable view
- [ ] Call stack view

#### 4.3 Additional LSP Features
- [ ] Document symbols (outline view)
- [ ] Workspace symbols (search by name)
- [ ] Code lens (show references, run tests)
- [ ] Semantic tokens (better syntax highlighting)
- [ ] Folding ranges (collapse functions/structs)
- [ ] Document formatting (via `wj fmt`)
- [ ] On-type formatting (auto-indent)

**Deliverable**: Full IDE experience with refactoring and debugging

---

### Phase 5: Polish & Documentation (Week 8+)

**Goal**: Production-ready LSP with excellent docs

#### 5.1 Testing
- [ ] Unit tests for all LSP features
- [ ] Integration tests with real projects
- [ ] Performance benchmarks
- [ ] Fuzz testing for edge cases
- [ ] Test against all 58 examples

#### 5.2 Performance Optimization
- [ ] Profile LSP response times
- [ ] Optimize Salsa queries
- [ ] Cache completion results
- [ ] Lazy loading for large projects
- [ ] Memory usage optimization
- [ ] Target: < 100ms for most operations

#### 5.3 Documentation
- [ ] LSP setup guide (all editors)
- [ ] API documentation
- [ ] Architecture document
- [ ] Contributing guide
- [ ] Demo videos (YouTube)
- [ ] Blog post announcing LSP

#### 5.4 Release Preparation
- [ ] Update CHANGELOG.md
- [ ] Update README.md with LSP features
- [ ] Write release notes
- [ ] Create demo GIFs for features
- [ ] Publish extensions to marketplaces

**Deliverable**: Production-ready LSP with comprehensive documentation

---

## 🎨 Unique Features (Differentiators)

### 1. Real-time Ownership Inference Hints ✨

**The Big Idea**: Show compiler's ownership decisions inline

```windjammer
// User sees this in their editor:
fn process_user(user: User /* inferred: & */) {
    println!("{}", user.name)
}

fn update_user(user: User /* inferred: &mut */) {
    user.name = "New Name"
}

fn take_user(user: User /* inferred: owned */) -> User {
    user
}
```

**Hover for explanation:**
```
Parameter `user` inferred as `&User`
Reason: Only reads, no writes or returns

Click to see full inference reasoning
[Override] [Learn More]
```

**Why This Matters**:
- No other language shows inference in real-time
- Teaches ownership while coding
- Transparent compiler decisions
- Unique selling point

### 2. Ownership Debugging Mode

```
fn complex_function(data: Vec<User>) {
    // Hover shows full ownership flow:
    // 1. `data` owned (parameter)
    // 2. Moved to `process_users()`
    // 3. Returned and moved to `result`
    let result = process_users(data)
    // data is no longer valid here
}
```

### 3. Intelligent Error Recovery

Show what the user *probably* meant:

```
Error: Cannot mutate immutable variable `count`
      |
   12 | count = count + 1
      | ^^^^^^^^^^^^^^^^^ mutation of immutable variable
      |
Help: Did you mean to declare `count` as mutable?
   
   let mut count = 0  // Add `mut` here
   
[Quick Fix: Make mutable]
```

---

## 📊 Success Metrics

### Performance Targets
- LSP startup time: < 500ms
- Analysis time: < 100ms per file
- Completion latency: < 50ms
- Memory usage: < 100MB for medium projects
- CPU usage: < 10% idle, < 30% during typing

### Feature Coverage
- 100% stdlib completion
- 100% user code navigation
- 95%+ hover information accuracy
- Zero false-positive errors

### User Satisfaction
- VSCode extension: 4.5+ stars
- "Feels as good as Rust Analyzer" feedback
- 10+ community projects using Windjammer
- Twitter/Reddit buzz about ownership hints

---

## 🔧 Technical Challenges & Solutions

### Challenge 1: Mapping Rust Errors to Windjammer

**Problem**: We transpile to Rust, so errors are in Rust terms

**Solution**: 
- Build comprehensive source map during transpilation
- Translate Rust error spans back to Windjammer
- Rewrite error messages in Windjammer terms
- Filter out Rust-specific suggestions

### Challenge 2: Incremental Compilation

**Problem**: Re-analyzing entire codebase on every keystroke is slow

**Solution**:
- Use Salsa for query-based incremental compilation
- Cache parsed ASTs per file
- Invalidate only affected files
- Lazy evaluation of dependencies

### Challenge 3: Cross-file Analysis

**Problem**: Need global view for references, renames, etc.

**Solution**:
- Build module dependency graph
- Index all symbols in workspace
- Use parallel processing for analysis
- Cache symbol tables

### Challenge 4: Ownership Inference in LSP

**Problem**: Inference happens during analysis, need to expose results

**Solution**:
- Store inference decisions in analysis results
- Expose via Salsa queries
- Generate inlay hints from inference data
- Provide detailed reasoning on demand

---

## 🎯 Milestones

**Week 2**: Basic LSP with real-time errors  
**Week 4**: Code intelligence (completion, goto, hints)  
**Week 6**: All editors supported with polished extensions  
**Week 8**: Refactoring and debugging integration  
**Week 8+**: Production-ready, fully documented

---

## 📚 Resources

**LSP Spec**: https://microsoft.github.io/language-server-protocol/  
**tower-lsp**: https://github.com/ebkalderon/tower-lsp  
**Salsa**: https://github.com/salsa-rs/salsa  
**DAP Spec**: https://microsoft.github.io/debug-adapter-protocol/  
**Rust Analyzer**: https://github.com/rust-lang/rust-analyzer (inspiration)

---

## 🚀 Let's Build This!

This is the most ambitious and impactful release yet. When complete, Windjammer will have:
- Best-in-class developer experience
- Unique ownership visualization
- Universal editor support
- Full IDE features

**This will transform Windjammer from "interesting" to "must-use"!** 🎉

