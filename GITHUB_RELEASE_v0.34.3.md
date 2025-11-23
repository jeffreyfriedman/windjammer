# Windjammer v0.34.3 - CI Fixes & Immutable Builds

**Release Date:** November 23, 2025  
**Type:** Bug Fix Release (CI/CD Infrastructure)

---

## 🎯 Overview

This release fixes critical CI/CD issues that prevented v0.34.2 from publishing successfully to crates.io. These are infrastructure-only changes - no code or API changes.

---

## 🔧 Critical CI Fixes

### 1. **Fixed GitHub Actions Permissions**
   - ✅ Added `permissions: contents: write` to release workflow
   - ✅ Fixes "Resource not accessible by integration" error when creating releases
   - ✅ Enables automated GitHub release creation

### 2. **Immutable Builds During Publishing**
   - ✅ Removed duplicate `cargo test --all` from publish workflow
   - ✅ Removed duplicate `cargo fmt --check` from publish workflow  
   - ✅ Removed duplicate `cargo clippy` from publish workflow
   - ✅ Tests now only run once in the dedicated test job
   - ✅ `Cargo.lock` no longer modified during publish process

**Why this matters:** Running `cargo test` in the publish workflow was modifying `Cargo.lock` (for test dependencies), causing the publish to fail with "dirty working directory" errors.

### 3. **Removed `--allow-dirty` Band-Aid**
   - ❌ Was using `--allow-dirty` as a workaround
   - ✅ Now using proper fix: don't modify `Cargo.lock` during publish
   - ✅ Ensures truly reproducible, immutable builds

### 4. **Resilient Caching**
   - ✅ Added `continue-on-error: true` to all cargo cache steps
   - ✅ Intermittent `hashFiles()` failures no longer block entire CI pipeline
   - ✅ Caching is an optimization - if it fails, cargo downloads dependencies fresh

---

## 🚀 Impact

**Before v0.34.3:**
- ❌ Release creation failed due to permissions
- ❌ Publishing failed due to `Cargo.lock` changes
- ❌ Intermittent cache failures blocked entire CI

**After v0.34.3:**
- ✅ Releases auto-create successfully
- ✅ Publishing works with clean, immutable `Cargo.lock`
- ✅ Cache failures don't block CI (just slower builds)

---

## 📊 What Changed

### Modified Files
- **`.github/workflows/release.yml`**
  - Added `permissions: contents: write`

- **`.github/workflows/publish.yml`**
  - Removed duplicate testing (13 lines removed)
  - Removed `--allow-dirty` flags (proper fix applied)
  - Added `continue-on-error` to cache step

- **`.github/workflows/test.yml`**
  - Added `continue-on-error` to cache steps (2 locations)

- **`.github/workflows/test-examples.yml`**
  - Added `continue-on-error` to cache step

- **`Cargo.toml`** (workspace)
  - Bumped version from `0.34.2` → `0.34.3`

- **`crates/windjammer-mcp/Cargo.toml`**
  - Updated dependency versions to `0.34.3`

- **`CHANGELOG.md`**
  - Added entry for v0.34.3

---

## 🔄 Workflow Changes

### Old Publish Flow (Broken)
1. Checkout code
2. ❌ Run `cargo test --all` (modifies `Cargo.lock`)
3. ❌ Run `cargo fmt --check`
4. ❌ Run `cargo clippy`
5. Try to publish → **FAILS** (working directory is dirty)

### New Publish Flow (Fixed)
1. Checkout code (with committed `Cargo.lock`)
2. Verify CHANGELOG entry
3. Publish to crates.io ✅

All testing happens in the separate `test` job that runs before publish.

---

## 📦 Installation

```bash
# Via Cargo (once published)
cargo install windjammer

# Or from source
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer
cargo build --release
```

---

## 🔗 Links

- **Repository:** https://github.com/jeffreyfriedman/windjammer
- **Documentation:** https://github.com/jeffreyfriedman/windjammer/tree/main/docs
- **Related Projects:**
  - [windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui) - Cross-platform UI framework

---

## 📝 Migration Notes

**No action required for users.** This release only fixes CI/CD infrastructure. If you're already using Windjammer, everything continues to work exactly the same way.

---

## 🙏 Notes

This is a **CI/CD infrastructure fix release**. No changes to the language, compiler, or API. The fixes in this release enable:

1. ✅ Automated publishing to crates.io
2. ✅ Automated GitHub release creation
3. ✅ Truly immutable, reproducible builds

**What's Next:**
- 📦 This release will be automatically published to crates.io by CI
- 📦 Future releases will publish smoothly without CI failures
- 📝 Next language feature release will be v0.35.0

---

**Full Changelog:** https://github.com/jeffreyfriedman/windjammer/compare/v0.34.2...v0.34.3

**Contributors:** @jeffreyfriedman

---

🎉 **Thank you for using Windjammer!**

