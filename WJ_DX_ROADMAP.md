# Windjammer Developer Experience (DX) Roadmap

## Vision

Windjammer developers should **never need to think about Rust**. The `wj` CLI should be the only tool needed for building, testing, and running Windjammer projects.

## Current State (v0.36.x)

### What Works
- ✅ `wj build` transpiles `.wj` → `.rs`
- ✅ `--library` flag strips `main()` functions
- ✅ `--module-file` flag generates `mod.rs`
- ✅ Auto-formats generated Rust code

### What's Suboptimal
- ❌ Developers must run `cargo build` after `wj build`
- ❌ Developers must understand `build.rs` and Cargo
- ❌ No `wj test` command
- ❌ No `wj watch` for auto-rebuild
- ❌ Manual flags (`--library`, `--module-file`) required

## Phase 1: Smart Build (v0.37.0)

### Goal
`wj build` should handle the entire build pipeline automatically.

### Features

#### 1.1 Auto-detect Project Type
```bash
# Automatically detect:
# - Library: has src/lib.wj or components_wj/
# - Application: has main.wj or src/main.wj
# - Hybrid: has both

wj build  # Just works, no flags needed
```

**Implementation**:
- Check for `src/lib.wj` → library mode
- Check for `main.wj` or `src/main.wj` → app mode
- Check for `components_wj/` or `src/components_wj/` → library mode
- Auto-enable `--library` and `--module-file` for libraries

#### 1.2 Integrated Cargo Build
```bash
wj build
# Internally:
# 1. Transpile .wj → .rs
# 2. Format generated code
# 3. Generate mod.rs
# 4. Run cargo build (if Cargo.toml exists)
# 5. Return unified errors
```

**Implementation**:
- After transpilation, check for `Cargo.toml`
- If found, run `cargo build` internally
- Capture output and map Rust errors back to `.wj` line numbers
- Return success/failure with Windjammer-focused error messages

#### 1.3 Error Mapping
```bash
# Bad: Rust error
error[E0308]: mismatched types
 --> src/generated/button.rs:42:9

# Good: Windjammer error
error: Type mismatch in button.wj
 --> src/components_wj/button.wj:42:9
   |
42 |         self
   |         ^^^^ expected Button, found &mut Button
   |
help: Builder methods should use 'mut self', not '&mut self'
```

**Implementation**:
- Parse Rust compiler errors
- Map file paths: `src/generated/button.rs` → `src/components_wj/button.wj`
- Map line numbers using source maps (if needed)
- Rewrite error messages to be Windjammer-focused

#### 1.4 Build Caching
```bash
wj build
# Only rebuilds changed .wj files
# Caches transpilation results
```

**Implementation**:
- Track `.wj` file mtimes
- Cache transpiled `.rs` files
- Only re-transpile changed files
- Invalidate cache on compiler version change

## Phase 2: Testing (v0.38.0)

### Goal
`wj test` should run tests without developers knowing about Cargo.

### Features

#### 2.1 Basic Test Command
```bash
wj test
# Transpiles all .wj files
# Runs cargo test
# Maps errors back to .wj files
```

**Implementation**:
- Run `wj build` first
- Execute `cargo test` internally
- Capture and map test failures to `.wj` files
- Display Windjammer-focused test output

#### 2.2 Test Filtering
```bash
wj test button           # Run tests matching "button"
wj test --file button.wj # Run tests in button.wj
wj test --lib            # Run library tests only
```

**Implementation**:
- Pass filters to `cargo test`
- Map test names back to `.wj` files

#### 2.3 Test Coverage
```bash
wj test --coverage
# Generates coverage report for .wj files
```

**Implementation**:
- Use `cargo-tarpaulin` or similar
- Map coverage data to `.wj` files
- Generate HTML report

## Phase 3: Watch Mode (v0.39.0)

### Goal
`wj watch` should auto-rebuild on file changes.

### Features

#### 3.1 Basic Watch
```bash
wj watch
# Watches .wj files
# Auto-rebuilds on save
# Shows errors in real-time
```

**Implementation**:
- Use `notify` crate for file watching
- Debounce changes (wait 100ms after last change)
- Run `wj build` on changes
- Display errors in terminal

#### 3.2 Test Watch
```bash
wj watch --test
# Watches .wj files
# Auto-runs tests on save
```

**Implementation**:
- Same as basic watch
- Run `wj test` instead of `wj build`

#### 3.3 Hot Reload (Future)
```bash
wj watch --hot-reload
# For web apps: auto-refresh browser
# For desktop apps: reload without restart
```

**Implementation**:
- Inject hot-reload runtime into generated code
- WebSocket connection for browser refresh
- Dynamic library reloading for desktop

## Phase 4: Pure Windjammer Projects (v0.40.0)

### Goal
No `Cargo.toml` required for pure Windjammer projects.

### Features

#### 4.1 Windjammer.toml
```toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
windjammer-ui = "0.1"

[dev-dependencies]
# ...
```

**Implementation**:
- Define Windjammer-native manifest format
- Auto-generate `Cargo.toml` internally
- Support Windjammer and Rust dependencies

#### 4.2 Package Management
```bash
wj add windjammer-ui      # Add dependency
wj remove windjammer-ui   # Remove dependency
wj update                 # Update dependencies
```

**Implementation**:
- Modify `Windjammer.toml`
- Regenerate `Cargo.toml`
- Run `cargo update` internally

#### 4.3 Publishing
```bash
wj publish
# Publishes to crates.io (for now)
# Future: Windjammer package registry
```

**Implementation**:
- Ensure all `.wj` files are transpiled
- Run `cargo publish` internally
- Handle authentication

## Phase 5: Advanced Features (v0.41.0+)

### 5.1 Language Server Protocol (LSP)
- Real-time error checking in editors
- Auto-completion
- Go-to-definition
- Rename refactoring

### 5.2 Debugger Integration
- Debug `.wj` files directly (not `.rs`)
- Breakpoints in Windjammer code
- Variable inspection

### 5.3 Profiler
```bash
wj profile
# Profiles Windjammer code
# Shows hotspots in .wj files
```

### 5.4 Formatter
```bash
wj fmt
# Formats .wj files
```

### 5.5 Linter
```bash
wj lint
# Lints .wj files
# Windjammer-specific rules
```

## Implementation Priority

### Immediate (v0.36.1)
1. ✅ Fix builder pattern bug (`&mut self` → `mut self`)
2. Document current limitations in README

### Short-term (v0.37.0 - Q1 2025)
1. Auto-detect project type
2. Integrated cargo build
3. Error mapping

### Medium-term (v0.38.0 - Q2 2025)
1. `wj test` command
2. Test filtering
3. Basic watch mode

### Long-term (v0.39.0+ - Q3+ 2025)
1. Hot reload
2. Pure Windjammer projects
3. LSP support

## Success Metrics

### Developer Experience
- **Time to first build**: < 10 seconds (from clone to running app)
- **Rebuild time**: < 1 second for single file change
- **Error clarity**: 90%+ of developers understand errors without Rust knowledge
- **Documentation**: 100% of common tasks documented with Windjammer-only commands

### Adoption
- **Windjammer-only projects**: 50%+ of projects use only `wj` commands
- **Community feedback**: 4.5+ stars on satisfaction surveys
- **Contributor growth**: 10+ contributors to `wj` CLI

## Breaking Changes

### v0.37.0
- `--library` and `--module-file` flags deprecated (auto-detected)
- `wj build` now runs `cargo build` by default (use `--transpile-only` to skip)

### v0.40.0
- `Cargo.toml` optional for pure Windjammer projects
- `Windjammer.toml` becomes the standard manifest

## Migration Guide

### From v0.36.x to v0.37.0
```bash
# Old way
wj build src/ -o generated/ --library --module-file
cargo build

# New way
wj build  # Just works!
```

### From v0.37.x to v0.40.0
```bash
# Old way
# Cargo.toml required

# New way
# Create Windjammer.toml
wj init
wj build
```

## Community Input

We welcome feedback on this roadmap! Please:
- Open issues for feature requests
- Comment on RFCs for major changes
- Join discussions in Discord/GitHub Discussions

## Related Issues

- #TBD: Auto-detect project type
- #TBD: `wj test` command
- #TBD: `wj watch` command
- #TBD: Error mapping
- #TBD: Windjammer.toml format

---

**Last Updated**: November 24, 2024
**Status**: Draft - Open for Community Feedback

