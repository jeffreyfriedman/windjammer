# Publishing Windjammer to crates.io

**Goal:** Make Windjammer installable via `cargo install windjammer`.

---

## Prerequisites

Before publishing to crates.io, you need:

1. ✅ **Crates.io Account**
   - Sign up at https://crates.io
   - Use GitHub for authentication (recommended)

2. ✅ **API Token**
   - Get token from https://crates.io/settings/tokens
   - Save it securely

3. ✅ **Cargo Login**
   ```bash
   cargo login <your-api-token>
   ```
   
   This saves your token to `~/.cargo/credentials.toml`.

---

## Pre-Publication Checklist

### 1. Verify `Cargo.toml` Metadata

Ensure all required fields are present:

```toml
[package]
name = "windjammer"
version = "0.16.0"  # Update this for each release!
edition = "2021"
authors = ["Your Name <your@email.com>"]
description = "A simple, high-level language that transpiles to Rust"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/windjammer"
homepage = "https://github.com/yourusername/windjammer"
documentation = "https://docs.rs/windjammer"
readme = "README.md"
keywords = ["transpiler", "rust", "language", "compiler"]
categories = ["compilers", "development-tools"]
rust-version = "1.90"

[lib]
name = "windjammer"
path = "src/lib.rs"

[[bin]]
name = "windjammer"
path = "src/main.rs"

# Or if you have a wj binary:
[[bin]]
name = "wj"
path = "src/bin/wj.rs"
default-run = "windjammer"  # Use windjammer as default for tests
```

**Check:**
```bash
cd /Users/jeffreyfriedman/src/windjammer
cargo package --list
```

This shows what files will be included in the package.

### 2. Update README.md

The README.md will be displayed on crates.io, so ensure it:
- Has a clear description
- Shows installation instructions
- Includes quick start examples
- Links to documentation

### 3. Verify LICENSE Files

You have dual licensing (MIT/Apache-2.0), so ensure:
- `LICENSE-MIT` exists
- `LICENSE-APACHE` exists
- `license = "MIT OR Apache-2.0"` in Cargo.toml

### 4. Test the Package Locally

```bash
# Create a test package (doesn't publish)
cargo package

# Test installation from the package
cargo install --path .

# Verify the binary works
wj --version
```

### 5. Run All Tests

```bash
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

## Publishing Steps

### Step 1: Dry Run

Test publishing without actually uploading:

```bash
cargo publish --dry-run
```

This will:
- Build the package
- Verify metadata
- Check dependencies
- Show what would be uploaded

**Common Issues:**
- Missing dependencies
- Path dependencies (not allowed on crates.io)
- Large files (>10 MB limit)
- Missing documentation

### Step 2: Publish to crates.io

Once dry-run succeeds:

```bash
cargo publish
```

**This is permanent!** You cannot:
- Delete a published version
- Modify a published version
- Re-publish the same version

You can only "yank" a version (hide it from new projects).

### Step 3: Verify Publication

```bash
# Wait 1-2 minutes for indexing
cargo install windjammer

# Check on crates.io
open https://crates.io/crates/windjammer
```

---

## Troubleshooting

### Issue: "error: api errors: Not Found"

**Cause:** Package name is already taken.

**Solution:** Choose a different name:
```toml
name = "windjammer-lang"  # or windjammer-rs, wj-lang, etc.
```

### Issue: "error: failed to verify package tarball"

**Cause:** Package doesn't build cleanly from the tarball.

**Solution:**
```bash
# Extract and test the package
cargo package
tar -xzf target/package/windjammer-0.16.0.crate -C /tmp
cd /tmp/windjammer-0.16.0
cargo build --release
```

Fix any issues in your source, then try again.

### Issue: "error: path dependencies are not allowed"

**Cause:** Cargo.toml has path dependencies.

**Example:**
```toml
[dependencies]
some-crate = { path = "../some-crate" }  # ❌ Not allowed
```

**Solution:** Publish dependencies first, then use version:
```toml
[dependencies]
some-crate = "0.1.0"  # ✅ From crates.io
```

### Issue: "error: package is too large"

**Cause:** Package > 10 MB.

**Solution:** Add exclusions to `Cargo.toml`:
```toml
[package]
exclude = [
    "target/",
    "examples/*/target/",
    "*.wasm",
    "*.so",
    "*.dylib",
    "*.dll",
    ".git/",
    ".github/",
    "docs/images/",
]
```

---

## Version Management

### Semantic Versioning

Follow [semver](https://semver.org/):

- **MAJOR.MINOR.PATCH** (e.g., 1.0.0)
- **0.x.y** - Before 1.0.0, any version can have breaking changes
- **1.0.0** - First stable release
- **1.1.0** - Add features (backward compatible)
- **1.0.1** - Bug fixes (backward compatible)
- **2.0.0** - Breaking changes

### Publishing a New Version

1. Update version in `Cargo.toml`:
   ```toml
   version = "0.17.0"
   ```

2. Update `CHANGELOG.md` with changes.

3. Commit and tag:
   ```bash
   git commit -am "Release v0.17.0"
   git tag v0.17.0
   git push origin main --tags
   ```

4. Publish:
   ```bash
   cargo publish
   ```

### Yanking a Version (Emergency)

If you published a broken version:

```bash
cargo yank --vers 0.16.0  # Hide from new installs
cargo yank --vers 0.16.0 --undo  # Undo yank
```

**Note:** Yanking doesn't delete; it just prevents new projects from using it.

---

## Post-Publication

### 1. Update Documentation

Your docs will be automatically built at https://docs.rs/windjammer.

**Verify:**
```bash
open https://docs.rs/windjammer/0.16.0
```

If docs fail to build, check the build log at docs.rs.

### 2. Update README.md

Add the crates.io badge:

```markdown
[![Crates.io](https://img.shields.io/crates/v/windjammer.svg)](https://crates.io/crates/windjammer)
[![Documentation](https://docs.rs/windjammer/badge.svg)](https://docs.rs/windjammer)
[![License](https://img.shields.io/crates/l/windjammer.svg)](https://github.com/yourusername/windjammer)
```

### 3. Announce

- Post on Reddit: r/rust
- Post on Twitter/X with #rustlang
- Post on Hacker News
- Post on your blog

### 4. Monitor

Check for:
- Issues on GitHub
- Questions on docs.rs
- Download statistics on crates.io

---

## Best Practices

### 1. Don't Rush

- Test thoroughly before publishing
- Have someone else review the package
- Wait until you're confident

### 2. Document Everything

- Write good README.md
- Add rustdoc comments
- Include examples

### 3. Keep Dependencies Minimal

- Only include necessary dependencies
- Use `[dev-dependencies]` for testing
- Consider `default-features = false`

### 4. Test Installation

Before publishing:
```bash
cargo install --path . --force
wj --version
wj --help
```

### 5. Plan Releases

- Don't publish broken versions
- Have a release checklist
- Maintain CHANGELOG.md

---

## Release Checklist

Before running `cargo publish`:

- [ ] All tests pass (`cargo test --all`)
- [ ] No clippy warnings (`cargo clippy --all-targets -- -D warnings`)
- [ ] Code is formatted (`cargo fmt --all -- --check`)
- [ ] Version bumped in `Cargo.toml`
- [ ] `CHANGELOG.md` updated
- [ ] README.md is current
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] Local installation works (`cargo install --path .`)
- [ ] Dry run succeeds (`cargo publish --dry-run`)
- [ ] Git committed and tagged
- [ ] GitHub release created (optional but recommended)

---

## Your Next Steps

### For Initial Publication (v0.16.0):

1. **Login to crates.io:**
   ```bash
   # Get token from https://crates.io/settings/tokens
   cargo login <your-token>
   ```

2. **Verify package:**
   ```bash
   cd /Users/jeffreyfriedman/src/windjammer
   cargo package --list
   ```

3. **Test dry run:**
   ```bash
   cargo publish --dry-run
   ```

4. **Publish:**
   ```bash
   cargo publish
   ```

5. **Verify:**
   ```bash
   # Wait 1-2 minutes
   cargo install windjammer
   wj --version
   ```

### For Future Releases:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit and tag: `git tag v0.17.0`
4. Push: `git push origin main --tags`
5. Publish: `cargo publish`

---

## Common Questions

**Q: Can I delete a published version?**  
A: No. You can only "yank" it to hide from new installs.

**Q: What if I publish with a typo?**  
A: Publish a patch version (e.g., 0.16.1) immediately.

**Q: How long does publishing take?**  
A: Upload: ~30 seconds. Indexing: ~2 minutes. Docs build: ~5-10 minutes.

**Q: Can I reserve a crate name?**  
A: Publish a minimal "placeholder" version with a warning in README.

**Q: What if the name is taken?**  
A: Choose a different name or contact the current owner.

**Q: Should I publish pre-1.0.0?**  
A: Yes! Use 0.x.y versions. Many crates stay 0.x for years.

---

*Last Updated: October 10, 2025*  
*For more info: https://doc.rust-lang.org/cargo/reference/publishing.html*

