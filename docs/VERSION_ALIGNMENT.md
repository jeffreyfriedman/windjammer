# Version Alignment - Workspace Crate Management

**Status**: ✅ **IMPLEMENTED** - Automated checks in pre-commit hook and CI  
**Date**: 2026-01-01  
**Version**: 0.39.1

---

## 📋 Overview

Windjammer is a **workspace** with 4 crates that must maintain **version alignment**:

```
windjammer (workspace)
├── windjammer (main crate)          ← v0.39.1
├── crates/windjammer-lsp            ← v0.39.1 (depends on windjammer v0.39.1)
├── crates/windjammer-mcp            ← v0.39.1 (depends on windjammer v0.39.1, windjammer-lsp v0.39.1)
└── crates/windjammer-runtime        ← v0.39.1
```

**Critical Rule**: All crates must have the **same version number** at all times.

---

## 🎯 Why Version Alignment Matters

### **1. Semantic Versioning Consistency**

When we release `windjammer v0.39.1`, users expect:
- `windjammer-lsp v0.39.1` to be compatible with `windjammer v0.39.1`
- `windjammer-mcp v0.39.1` to be compatible with both `windjammer v0.39.1` and `windjammer-lsp v0.39.1`

Mismatched versions create confusion:
```bash
# BAD: What version is actually in use?
windjammer v0.39.1
windjammer-lsp v0.39.0  ← Older version?
windjammer-mcp v0.39.1  ← Same as workspace but depends on old LSP?
```

### **2. Dependency Resolution**

Each sub-crate declares dependencies on other workspace crates:

```toml
# crates/windjammer-lsp/Cargo.toml
[dependencies]
windjammer = { path = "../..", version = "0.39.1" }
```

If the dependency version doesn't match the actual workspace version:
- ❌ Confusing build errors
- ❌ Cargo.lock conflicts
- ❌ CI failures
- ❌ Publication failures to crates.io

### **3. Release Atomicity**

When we release a new version:
1. All crates are published together
2. All crates reference the same version
3. Users can install the full suite with consistent versions

```bash
# Users expect this to work:
cargo install windjammer --version 0.39.1
cargo install windjammer-lsp --version 0.39.1
cargo install windjammer-mcp --version 0.39.1
```

---

## 🔍 Automated Version Checks

### **1. Pre-commit Hook** (Local)

**Location**: `.git/hooks/pre-commit`

**When**: Every `git commit` (unless `--no-verify`)

**Checks**:
1. ✅ All sub-crate versions match workspace version
2. ✅ LSP's `windjammer` dependency matches workspace version
3. ✅ MCP's `windjammer` dependency matches workspace version
4. ✅ MCP's `windjammer-lsp` dependency matches workspace version

**Example Output**:
```bash
🔍 Pre-commit checks...
Checking version consistency... ✓
```

**On Failure**:
```bash
❌ ERROR: Sub-crate version mismatch!
Workspace version: 0.39.1

  ❌ windjammer-lsp: 0.39.0 (expected: 0.39.1)
  ❌ windjammer-mcp: 0.39.0 (expected: 0.39.1)

To fix, run:
  ./scripts/sync-version.sh
```

---

### **2. GitHub Actions CI** (Remote)

**Workflow**: `.github/workflows/test.yml`

**Job**: `version-check`

**When**: Every push and pull request

**Checks**:
1. ✅ Extracts workspace version from `[workspace.package]`
2. ✅ Compares each sub-crate's version
3. ✅ Fails if any mismatches found

**Example Output**:
```bash
🔍 Checking version alignment across all crates...
📦 Workspace version: 0.39.1

✅ windjammer-lsp: 0.39.1
✅ windjammer-mcp: 0.39.1
✅ windjammer-runtime: 0.39.1

✅ All crate versions are aligned with workspace version 0.39.1
```

**On Failure**:
```bash
📦 Workspace version: 0.39.1

❌ windjammer-lsp: 0.39.0 (expected: 0.39.1)
❌ windjammer-mcp: 0.39.0 (expected: 0.39.1)
❌ windjammer-runtime: 0.39.0 (expected: 0.39.1)

❌ ERROR: 3 crate(s) have mismatched versions!

To fix:
  1. Update version in each crate's Cargo.toml to match workspace version
  2. Or use 'version.workspace = true' to inherit from workspace
```

---

## 🔧 How to Update Versions

### **Option 1: Automated Script** (Recommended)

Use the provided sync script:

```bash
./scripts/sync-version.sh
```

**What it does**:
1. Extracts version from `[workspace.package]` in root `Cargo.toml`
2. Updates all sub-crate `Cargo.toml` files
3. Updates README.md version references
4. Updates ROADMAP.md version references
5. Shows git diff of changes

**Example**:
```bash
$ ./scripts/sync-version.sh
📦 Detected version from Cargo.toml: 0.39.1
🔄 Syncing version 0.39.1 across project files...
✅ Updated crates/windjammer-lsp/Cargo.toml
✅ Updated crates/windjammer-mcp/Cargo.toml
✅ Updated crates/windjammer-runtime/Cargo.toml
✅ Updated README.md
✨ Version sync complete!
```

---

### **Option 2: Manual Update**

If you prefer manual control:

#### **Step 1: Update Workspace Version**

Edit `Cargo.toml`:
```toml
[workspace.package]
version = "0.40.0"  # New version
```

#### **Step 2: Update Each Sub-crate**

Edit each crate's `Cargo.toml`:

**`crates/windjammer-lsp/Cargo.toml`**:
```toml
[package]
version = "0.40.0"

[dependencies]
windjammer = { path = "../..", version = "0.40.0" }
```

**`crates/windjammer-mcp/Cargo.toml`**:
```toml
[package]
version = "0.40.0"

[dependencies]
windjammer = { path = "../..", version = "0.40.0" }
windjammer-lsp = { path = "../windjammer-lsp", version = "0.40.0" }
```

**`crates/windjammer-runtime/Cargo.toml`**:
```toml
[package]
version = "0.40.0"
```

#### **Step 3: Update CHANGELOG**

Edit `CHANGELOG.md`:
```markdown
## [0.40.0] - 2026-01-15
### Added
- New feature...
```

#### **Step 4: Verify Alignment**

Run the version check:
```bash
./scripts/sync-version.sh  # Will detect version 0.40.0 and show current state
```

Or run the CI check locally:
```bash
.github/workflows/test.yml version-check  # (if you have act installed)
```

---

## 🚀 Release Process with Version Alignment

### **Complete Release Checklist**

1. **Decide new version** (e.g., `0.40.0`)
2. **Update workspace version**:
   ```bash
   # Edit Cargo.toml: [workspace.package] version = "0.40.0"
   ```
3. **Sync all crate versions**:
   ```bash
   ./scripts/sync-version.sh
   ```
4. **Update CHANGELOG**:
   ```bash
   # Edit CHANGELOG.md: ## [0.40.0] - 2026-01-15
   ```
5. **Commit version bump**:
   ```bash
   git add -A
   git commit -m "chore: bump version to 0.40.0"
   ```
6. **Run pre-commit checks** (automatic):
   - ✅ Version alignment verified
   - ✅ Code formatted
   - ✅ Clippy passes
   - ✅ Tests pass
   - ✅ Security audit clean
7. **Push and verify CI**:
   ```bash
   git push
   ```
   - ✅ `version-check` job passes
   - ✅ All platform tests pass
8. **Tag release**:
   ```bash
   git tag v0.40.0
   git push --tags
   ```
9. **Publish to crates.io** (if public):
   ```bash
   cargo publish -p windjammer-runtime
   cargo publish -p windjammer
   cargo publish -p windjammer-lsp
   cargo publish -p windjammer-mcp
   ```

---

## 🛠️ Troubleshooting

### **Issue 1: Pre-commit Hook Fails with Version Mismatch**

**Error**:
```bash
❌ ERROR: Sub-crate version mismatch!
Workspace version: 0.39.1
  ❌ windjammer-lsp: 0.39.0 (expected: 0.39.1)
```

**Fix**:
```bash
# Run sync script
./scripts/sync-version.sh

# Or manually edit crates/windjammer-lsp/Cargo.toml
# Change: version = "0.39.0"
# To:     version = "0.39.1"

# Then commit again
git add -A
git commit -m "chore: sync crate versions"
```

---

### **Issue 2: CI `version-check` Job Fails**

**Error**:
```bash
❌ ERROR: 3 crate(s) have mismatched versions!
```

**Fix**:
```bash
# Pull latest changes
git pull

# Run sync script locally
./scripts/sync-version.sh

# Verify locally
git diff

# Commit and push
git add -A
git commit -m "chore: align crate versions"
git push
```

---

### **Issue 3: Dependency Version Mismatch**

**Error**:
```bash
❌ ERROR: windjammer-mcp depends on wrong windjammer version!
Workspace version: 0.39.1
MCP dependency:    0.39.0
```

**Fix**:
Edit `crates/windjammer-mcp/Cargo.toml`:
```toml
[dependencies]
windjammer = { path = "../..", version = "0.39.1" }
```

**Pro Tip**: Always run `./scripts/sync-version.sh` instead of manual edits!

---

### **Issue 4: Cargo.lock Out of Sync**

**Error**:
```bash
error: failed to select a version for `windjammer`
```

**Fix**:
```bash
# Update Cargo.lock
cargo update -p windjammer
cargo update -p windjammer-lsp
cargo update -p windjammer-mcp
cargo update -p windjammer-runtime

# Or regenerate
rm Cargo.lock
cargo generate-lockfile
```

---

## 📊 Version Alignment Status

### **Current State** (v0.39.1)

| Crate | Version | Status |
|-------|---------|--------|
| windjammer (workspace) | 0.39.1 | ✅ |
| windjammer (main) | 0.39.1 | ✅ |
| windjammer-lsp | 0.39.1 | ✅ |
| windjammer-mcp | 0.39.1 | ✅ |
| windjammer-runtime | 0.39.1 | ✅ |

**Dependencies**:
- ✅ LSP depends on windjammer 0.39.1
- ✅ MCP depends on windjammer 0.39.1
- ✅ MCP depends on windjammer-lsp 0.39.1

---

## 🎓 Best Practices

### **DO** ✅

1. **Use the sync script** for version updates
   ```bash
   ./scripts/sync-version.sh
   ```

2. **Let pre-commit hooks run** (don't use `--no-verify` unless necessary)

3. **Check CI before merging** (verify `version-check` passes)

4. **Bump all versions together** (never leave crates mismatched)

5. **Test locally** before pushing:
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

---

### **DON'T** ❌

1. **Don't manually edit versions** without running sync script

2. **Don't use `version.workspace = true`** (we use explicit versions for clarity)

3. **Don't skip pre-commit checks** with `--no-verify` habitually

4. **Don't mix version numbers** across crates

5. **Don't forget CHANGELOG** when bumping versions

---

## 🔮 Future Improvements

### **Planned**

1. **`version.workspace = true`**: Consider using workspace inheritance
   - Pro: Single source of truth
   - Con: Less explicit in each crate's Cargo.toml

2. **Cargo workspace version command**:
   ```bash
   cargo workspace version 0.40.0
   ```

3. **Automated CHANGELOG updates**: Parse commit messages to generate entries

4. **Pre-release version support**: Handle `-alpha`, `-beta`, `-rc` suffixes

---

## 📚 Related Documentation

- **Pre-commit Hook**: `.git/hooks/pre-commit`
- **Sync Script**: `scripts/sync-version.sh`
- **CI Workflow**: `.github/workflows/test.yml` (job: `version-check`)
- **CHANGELOG**: `CHANGELOG.md`
- **Workspace Config**: `Cargo.toml` (`[workspace.package]`)

---

## 🎉 Summary

**Version alignment is critical for**:
- ✅ Consistent releases
- ✅ Clear dependency management
- ✅ User expectations
- ✅ Publication to crates.io

**Automated checks catch mismatches**:
- ✅ Pre-commit hook (local)
- ✅ CI `version-check` job (remote)

**Easy to fix**:
```bash
./scripts/sync-version.sh
git add -A
git commit -m "chore: sync versions"
```

**Current status**: ✅ **All 4 crates aligned at v0.39.1**

---

**Last Updated**: 2026-01-01  
**Version**: 0.39.1  
**Status**: **AUTOMATED** ✅

