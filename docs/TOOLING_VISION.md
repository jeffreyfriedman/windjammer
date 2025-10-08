# Windjammer Tooling Vision

**Target**: v0.13.0+  
**Status**: Planning / RFC  
**Author**: Based on user feedback

---

## 🎯 Problem Statement

Currently, Windjammer users must juggle multiple tools:

```bash
# Today's workflow:
windjammer build --path main.wj --output ./out    # Transpile
cd out && cargo run                                 # Run
cargo test                                          # Test
cargo clippy                                        # Lint
cargo fmt                                           # Format
# Edit Cargo.toml manually                         # Dependencies
# Set up rustup, cargo, etc.                       # Installation
```

**Issues:**
- ❌ **Fragmented experience** - multiple CLIs, multiple mental models
- ❌ **Rust leakage** - users see Cargo.toml, Rust errors, `cargo` commands
- ❌ **Poor branding** - feels like "Rust with extra steps", not its own language
- ❌ **Onboarding friction** - users need rustup, cargo knowledge
- ❌ **Error confusion** - Rust compiler errors reference Rust code, not Windjammer

---

## 💡 Solution: Unified `wj` CLI

A single, cohesive CLI that **wraps and enhances** existing Rust tooling:

```bash
wj build main.wj              # Build and run (replaces: windjammer + cargo)
wj test                       # Run tests (replaces: cargo test)
wj lint                       # Run linter (replaces: cargo clippy)
wj fmt                        # Format code (replaces: cargo fmt)
wj new my-project             # Scaffold project (new!)
wj add reqwest                # Add dependency (hides Cargo.toml)
wj docs                       # Open docs (new!)
wj upgrade                    # Update Windjammer (new!)
wj run main.wj                # Quick run (compile + execute)
wj watch main.wj              # Watch mode (new!)
```

---

## 🏗️ Architecture

### Layer 1: CLI Interface (`wj` binary)

Single entry point with subcommands:

```rust
// src/cli/mod.rs
enum Command {
    Build { file: PathBuf, output: Option<PathBuf>, release: bool },
    Run { file: PathBuf, args: Vec<String> },
    Test { filter: Option<String> },
    Lint { fix: bool },
    Fmt { check: bool },
    New { name: String, template: String },
    Add { package: String, version: Option<String> },
    Docs,
    Upgrade,
    Watch { file: PathBuf },
    Lsp,  // For editor integration
}
```

### Layer 2: Tool Wrappers

**Wrap existing Rust tools:**

```rust
// src/tools/cargo.rs
pub fn run_cargo(args: &[&str]) -> Result<Output> {
    Command::new("cargo")
        .args(args)
        .output()
}

// src/tools/clippy.rs
pub fn run_clippy() -> Result<Vec<Diagnostic>> {
    let output = run_cargo(&["clippy", "--message-format=json"])?;
    parse_diagnostics(output)
}
```

### Layer 3: Enhancement Layer

**Add Windjammer-specific features:**

```rust
// src/enhancement/error_mapping.rs
pub fn translate_rust_error(rust_error: RustError) -> WindjammerError {
    // Map Rust line numbers to Windjammer source
    // Translate Rust types to Windjammer types
    // Provide Windjammer-specific hints
}

// src/enhancement/project_scaffold.rs
pub fn create_project(name: &str, template: &str) -> Result<()> {
    // Generate main.wj, wj.toml, README.md, etc.
    // Set up .gitignore
    // Initialize git repo
}
```

### Layer 4: Configuration

**New config file: `wj.toml`**

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2025"

[dependencies]
# Windjammer stdlib (auto-included)
# User dependencies:
reqwest = "0.11"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
criterion = "0.5"

[profile.release]
opt-level = 3

[target.wasm]
enabled = true
```

**Transpiles to `Cargo.toml` under the hood.**

---

## 🎨 User Experience

### Installing Windjammer

**Option 1: Standalone installer (ideal)**
```bash
# Downloads wj binary + manages Rust toolchain internally
curl -sSf https://windjammer.dev/install.sh | sh
wj --version  # Works immediately
```

**Option 2: Via cargo (current)**
```bash
cargo install windjammer-cli
wj --version
```

### Creating a New Project

```bash
$ wj new my-app
Creating Windjammer project: my-app
  ✓ Created src/main.wj
  ✓ Created wj.toml
  ✓ Created README.md
  ✓ Created .gitignore
  ✓ Initialized git repository

To get started:
  cd my-app
  wj run

$ cd my-app
$ wj run
   Compiling my-app v0.1.0
    Finished dev build in 1.2s
     Running target/debug/my-app
Hello, Windjammer!
```

### Adding Dependencies

```bash
$ wj add reqwest
    Adding reqwest ^0.11 to wj.toml
    Updated dependencies
    
$ wj add serde --features derive
    Adding serde ^1.0 (features: derive) to wj.toml
    Updated dependencies
```

**Behind the scenes:**
- Updates `wj.toml`
- Regenerates `Cargo.toml`
- Runs `cargo update` if needed

### Development Workflow

```bash
# Watch mode - auto-recompile on save
$ wj watch src/main.wj
   Watching src/**/*.wj
   Initial build... Done in 1.2s
   
   [file changed: src/main.wj]
   Recompiling... Done in 0.3s
   Running... 
   Hello, world!

# Quick iteration
$ wj run main.wj                    # Compile + run
$ wj test                            # Run all tests
$ wj test --filter user_tests       # Run specific tests
$ wj lint                            # Check for issues
$ wj fmt                             # Format code
```

### Better Error Messages

**Today (Rust errors):**
```
error[E0308]: mismatched types
  --> /tmp/out/main.rs:42:5
   |
42 |     "hello"
   |     ^^^^^^^ expected `i64`, found `&str`
```

**Tomorrow (Windjammer errors):**
```
error: Type mismatch
  --> src/main.wj:15:5
   |
15 |     "hello"
   |     ^^^^^^^ expected `int`, found `string`
   |
help: The function `add` expects to return an `int`, but you're returning a `string`.
      Perhaps you meant to parse the string?
      
      let result: int = "hello".parse()?
```

---

## 🔧 Implementation Plan

### Phase 1: Basic CLI (v0.13.0)

**Scope:**
- `wj build` - Wrap windjammer + cargo build
- `wj run` - Compile and execute
- `wj test` - Run tests
- `wj fmt` - Format code
- `wj lint` - Run clippy

**Work:**
- Create new CLI crate (`windjammer-cli`)
- Implement command routing
- Wrap cargo commands
- Basic error mapping (reuse v0.7.0 work)

**Effort:** 1-2 weeks

---

### Phase 2: Project Management (v0.14.0)

**Scope:**
- `wj new` - Scaffold projects
- `wj add` / `wj remove` - Dependency management
- `wj.toml` configuration format
- Project templates (web, cli, wasm, lib)

**Work:**
- Project scaffolding system
- `wj.toml` parser
- `wj.toml` ↔ `Cargo.toml` translation
- Dependency resolver integration

**Effort:** 2-3 weeks

---

### Phase 3: Developer Experience (v0.15.0)

**Scope:**
- `wj watch` - File watcher with auto-reload
- `wj docs` - Documentation browser
- `wj upgrade` - Self-update mechanism
- Better error messages (enhanced mapping)
- Progress indicators, colored output

**Work:**
- File watching (notify crate)
- Documentation generation/serving
- Self-update mechanism (cargo-update pattern)
- Enhanced error translation
- Terminal UI improvements (indicatif, colored)

**Effort:** 2-3 weeks

---

### Phase 4: Editor Integration (v0.16.0)

**Scope:**
- `wj lsp` - Language Server Protocol
- VS Code extension
- Syntax highlighting
- Auto-completion
- Hover hints
- Go-to-definition

**Work:**
- LSP server (wrap rust-analyzer + custom layer)
- VS Code extension (TypeScript)
- Syntax definitions
- Salsa-based incremental compilation (already started)

**Effort:** 4-6 weeks

---

## 📊 Comparison: Before vs. After

| Task | Today (v0.11.0) | Future (v0.13.0+) |
|------|----------------|-------------------|
| **Install** | `cargo install windjammer` + know rustup | `curl install.sh \| sh` |
| **New project** | Manual file creation | `wj new my-app` |
| **Add dependency** | Edit Cargo.toml | `wj add reqwest` |
| **Build + run** | `windjammer build ...` + `cd` + `cargo run` | `wj run main.wj` |
| **Test** | `cargo test` | `wj test` |
| **Lint** | `cargo clippy` | `wj lint` |
| **Format** | `cargo fmt` | `wj fmt` |
| **Watch mode** | N/A (cargo-watch) | `wj watch` |
| **Errors** | Rust errors, confusing | Windjammer errors, helpful |
| **Docs** | GitHub README | `wj docs` |

---

## 💭 Open Questions

### 1. Cargo.toml Visibility

**Option A: Hide completely**
- Users never see `Cargo.toml`
- `wj.toml` is the source of truth
- Generated Cargo.toml is in `.wj/` directory

**Option B: Show but discourage editing**
- Generate `Cargo.toml` in project root
- Add comment: "Auto-generated, edit wj.toml instead"
- Users can manually tweak if needed (escape hatch)

**Option C: Hybrid**
- `wj.toml` for Windjammer-specific config
- `Cargo.toml` for advanced Rust features
- `wj` commands work with both

**Recommendation:** Start with **Option B** (show but discourage), evolve to **Option A** as tooling matures.

---

### 2. Rust Toolchain Management

**Option A: Bundle Rust**
- Ship `wj` with embedded rustc, cargo, etc.
- Easier installation, larger binary (~200MB)

**Option B: Require rustup**
- Users install rustup separately
- `wj` detects and uses system Rust
- Smaller binary, more setup

**Option C: Lazy install**
- Ship lightweight `wj` binary
- Downloads Rust toolchain on first use
- Like rustup, but Windjammer-branded

**Recommendation:** Start with **Option B** (require rustup), consider **Option C** post-v1.0.

---

### 3. Target Audience for Tooling

**Beginners:**
- Want simplicity, don't know Rust/Cargo
- Hide complexity, provide good defaults
- `wj run main.wj` should "just work"

**Intermediate:**
- Know some Rust, want ergonomics
- Appreciate unified CLI
- Still want escape hatches (edit Cargo.toml)

**Advanced:**
- Deep Rust knowledge, want control
- Use `wj` for convenience, drop to Cargo when needed
- Appreciate that Windjammer doesn't block them

**Recommendation:** Design for **beginners**, don't block **advanced** users. Provide escape hatches.

---

## 🎯 Success Metrics

### v0.13.0 Goals (Basic CLI)
- [ ] Users can `wj build`, `wj run`, `wj test` without touching `cargo`
- [ ] Error messages are 50% more helpful (subjective)
- [ ] Installation instructions mention `wj`, not `cargo`

### v0.14.0 Goals (Project Management)
- [ ] Users can create projects without knowing about Cargo.toml
- [ ] `wj new` templates cover 80% of use cases (cli, web, wasm)
- [ ] Dependency management is intuitive

### v0.15.0 Goals (Developer Experience)
- [ ] Watch mode enables rapid iteration
- [ ] Error messages reference Windjammer source, not Rust
- [ ] Documentation is accessible via `wj docs`

### v0.16.0 Goals (Editor Integration)
- [ ] VS Code extension provides syntax highlighting, completion
- [ ] LSP works with rust-analyzer features
- [ ] Developer survey: 80%+ satisfaction with tooling

---

## 🚀 Migration Strategy

### For Existing Users (v0.12.0 → v0.13.0)

**Gradual adoption:**
1. v0.13.0: Both `windjammer` and `wj` work
2. v0.14.0: `windjammer` deprecated, `wj` recommended
3. v0.15.0: `windjammer` shows migration warning
4. v1.0.0: Only `wj` supported

**Provide migration tool:**
```bash
$ wj migrate
Migrating project to Windjammer tooling...
  ✓ Created wj.toml from Cargo.toml
  ✓ Updated .gitignore
  ✓ Created scripts for compatibility
  
Your project is ready!
  - Use `wj build` instead of `cargo run`
  - Edit wj.toml for dependencies
  - See MIGRATION_GUIDE.md for details
```

---

## 📚 Related Work

### Inspiration from Other Languages

**Deno (JavaScript/TypeScript):**
- Single executable, built-in tools
- `deno run`, `deno test`, `deno fmt`, `deno lint`
- No `package.json` noise (can use URLs)
- **Lesson:** Unified tooling is powerful for branding

**Go:**
- `go build`, `go test`, `go fmt`
- Simple, consistent CLI
- **Lesson:** Simplicity wins, fewer tools to learn

**Rust:**
- `cargo` is excellent, but also complex
- Many subcommands, configuration options
- **Lesson:** We can wrap and simplify

**Zig:**
- Single `zig` binary, no external build system
- `zig build`, `zig test`
- **Lesson:** Self-contained tooling improves DX

---

## 🤔 Philosophical Alignment

### 80/20 Philosophy

**Tooling should embody 80/20:**
- ✅ **80% case:** `wj run main.wj` - trivial
- ✅ **20% case:** Advanced Cargo features - possible via escape hatches

### Progressive Disclosure

**Beginners see:**
```bash
wj run main.wj
```

**Intermediate users see:**
```bash
wj build --release
wj test --filter integration
```

**Advanced users see:**
```bash
wj build --cargo-args "--features experimental"
# Or drop to cargo directly
cargo build --features experimental
```

### Transparency

**Don't hide Rust completely:**
- Show generated Rust code if users want (`wj build --show-rust`)
- Explain that Windjammer transpiles to Rust
- Encourage learning Rust for advanced use cases

**But simplify the common path:**
- Default workflow doesn't require Rust knowledge
- Cargo.toml is auto-generated, not hand-written
- Error messages are Windjammer-first

---

## 📋 TODO: Tooling Roadmap

- [ ] v0.13.0: Basic CLI (`wj build`, `wj run`, `wj test`, `wj lint`, `wj fmt`)
- [ ] v0.14.0: Project management (`wj new`, `wj add`, `wj.toml`)
- [ ] v0.15.0: Developer experience (`wj watch`, `wj docs`, better errors)
- [ ] v0.16.0: Editor integration (LSP, VS Code extension)
- [ ] v1.0.0: Polish, stabilize, production-ready

**Post-v1.0.0:**
- Package registry? (wj-pkg.dev, like crates.io)
- CI/CD integration (GitHub Actions templates)
- Cloud deployment helpers (wj deploy)
- GUI tools? (project manager, debugger)

---

## 🎉 Conclusion

Unified `wj` CLI tooling is a **high-value, moderate-complexity** investment that will:

1. **Strengthen brand identity** - Windjammer feels like its own language
2. **Improve onboarding** - Lower barrier to entry for new users
3. **Enhance ergonomics** - Single tool, consistent interface
4. **Better error messages** - Windjammer-aware, not Rust-aware
5. **Enable future features** - Foundation for LSP, watch mode, etc.

**Recommendation:** Start with **v0.13.0 basic CLI**, iterate based on user feedback, aim for full tooling suite by **v0.16.0**.

---

**Status**: RFC / Planning  
**Feedback Welcome**: Should we prioritize this for v0.13.0?  
**Alternative**: Continue stdlib expansion, defer tooling to v0.14.0+

