# Windjammer v0.35.1 - CI/CD Infrastructure Improvements

## Overview

This patch release focuses on improving CI/CD reliability and catching issues earlier in the development cycle. No functional changes to the compiler or runtime.

---

## 🔧 Fixed

### Release Workflow
- **Added binary verification step** after cross-compilation builds
  - Fails fast if binary isn't produced (instead of failing at `strip` step)
  - Provides detailed diagnostics (file size, directory contents)
  - Helps debug cross-compilation issues on Linux, macOS, and Windows

### Publish Workflow
- **Fixed dry-run dependency resolution** for dependent crates
  - Skip dry-run for `windjammer-lsp` and `windjammer-mcp` (they depend on unreleased versions)
  - Only validate core crates (`windjammer`, `windjammer-runtime`) in dry-run
  - Dependent crates are validated during actual publish after dependencies are live

---

## ✨ Enhanced

### Test Workflow (Pre-merge Validation)

Added two new CI jobs to catch issues **before merge**:

#### 1. `lockfile-check` - Cargo.lock Validation
```yaml
- Enforces that Cargo.lock is committed
- Enforces that Cargo.lock is up-to-date with Cargo.toml
- Catches --no-verify bypasses of pre-commit hooks
```

**Why this matters:**
- Ensures reproducible builds across all contributors
- Prevents "works on my machine" dependency issues
- No longer relies on pre-commit hooks (which can be bypassed)

#### 2. `publish-dryrun` - Pre-merge Publish Validation
```yaml
- Validates crates can be published before merge
- Catches missing metadata, invalid dependencies, etc.
- Runs automatically on every PR
```

**Why this matters:**
- Catches publish issues in PR review, not during release
- Fast feedback loop (fails in minutes, not hours)
- Reduces manual release friction

---

## 📦 What's Changed

### Modified Files
- `.github/workflows/release.yml` - Added binary verification
- `.github/workflows/publish.yml` - Fixed dry-run for dependent crates
- `.github/workflows/test.yml` - Added lockfile and publish validation
- `Cargo.toml` - Bumped version to 0.35.1
- `crates/windjammer-lsp/Cargo.toml` - Updated dependency version
- `crates/windjammer-mcp/Cargo.toml` - Updated dependency versions
- `CHANGELOG.md` - Added 0.35.1 entry

### Commits
- `fix(ci): Fix release and publish workflows for v0.35.1`
- `test(ci): Add publish dry-run to test workflow for pre-merge validation`
- `fix(ci): Add --allow-dirty flag to publish dry-run`
- `fix(ci): Add Cargo.lock validation job to enforce committed and up-to-date lockfile`

---

## 🎯 Benefits

### For Contributors
- ✅ Faster feedback on CI issues (fails in PR, not post-merge)
- ✅ Clear error messages when something is wrong
- ✅ No more "Oops, forgot to update Cargo.lock"

### For Maintainers
- ✅ Confident releases (publish issues caught early)
- ✅ Less time debugging CI failures
- ✅ Better diagnostics for cross-compilation problems

### For Users
- ✅ More frequent, reliable releases
- ✅ Reproducible builds (lockfile always current)
- ✅ Faster bug fixes (less CI friction)

---

## 📊 CI Status

All workflows passing:
- ✅ Tests (unit, integration, examples)
- ✅ Lints (clippy, formatting)
- ✅ Lockfile validation (new!)
- ✅ Publish dry-run (new!)

---

## 🔄 Upgrade Path

No action required. This is a CI/CD infrastructure release with no breaking changes.

If you're on `v0.35.0`:
```bash
# Via cargo
cargo install windjammer --version 0.35.1

# Via git
git pull origin main
cargo build --release
```

---

## 📝 Notes

- This release does **not** include functional changes to the compiler
- All improvements are CI/CD infrastructure
- If you experienced issues with `v0.35.0` release artifacts, this addresses the root causes
- The release workflow will now provide better diagnostics if cross-compilation fails

---

## 🙏 Acknowledgments

Thanks for reporting the CI issues! This release makes the development process more robust for everyone.

---

**Full Changelog**: https://github.com/jeffreyfriedman/windjammer/compare/v0.35.0...v0.35.1

