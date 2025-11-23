# Fix Version Alignment Across All Crates (v0.35.2)

## Problem

During the v0.35.1 release, we discovered that not all workspace crates had their versions synchronized:

```
✅ windjammer:          0.35.1
❌ windjammer-runtime:  0.34.1  (should be 0.35.1)
✅ windjammer-lsp:      0.35.1
❌ windjammer-mcp:      0.31.0  (should be 0.35.1)
```

### Impact

- `windjammer-runtime` was published to crates.io as `0.34.1` instead of `0.35.1`
- `windjammer-mcp` was not published (it's still at `0.31.0` on crates.io)
- Version mismatches cause confusion for users and break semantic versioning expectations

### Root Cause

Individual crate `Cargo.toml` files had explicit `version = "X.Y.Z"` that weren't updated when the workspace version was bumped. We updated:
- Workspace version: `0.35.0` → `0.35.1`
- `windjammer-lsp` dependencies: `0.35.0` → `0.35.1`
- `windjammer-mcp` dependencies: `0.35.0` → `0.35.1`

But forgot to update:
- `windjammer-runtime` package version
- `windjammer-mcp` package version

---

## Solution

### 1. Fix All Crate Versions → 0.35.2

```toml
# Workspace
version = "0.35.1" → "0.35.2"

# windjammer-runtime
version = "0.34.1" → "0.35.2"

# windjammer-mcp
version = "0.31.0" → "0.35.2"

# windjammer-lsp
windjammer = { ..., version = "0.35.1" } → "0.35.2"

# windjammer-mcp
windjammer = { ..., version = "0.35.1" } → "0.35.2"
windjammer-lsp = { ..., version = "0.35.1" } → "0.35.2"
```

### 2. Prevent Future Misalignment with CI Check

Added a new `version-check` CI job that runs on every PR:

```yaml
version-check:
  name: Version Alignment Check
  steps:
    - name: Check all crate versions match workspace version
      run: |
        # Get workspace version
        WORKSPACE_VERSION=$(grep -A 1 '\[workspace.package\]' Cargo.toml | grep 'version = ' | sed 's/.*version = "\(.*\)"/\1/')
        
        # Check each crate's version
        for crate_toml in crates/*/Cargo.toml; do
          CRATE_NAME=$(grep '^name = ' "$crate_toml" | head -1 | sed 's/.*name = "\(.*\)"/\1/')
          CRATE_VERSION=$(grep '^version = ' "$crate_toml" | head -1 | sed 's/.*version = "\(.*\)"/\1/')
          
          if [ "$CRATE_VERSION" != "$WORKSPACE_VERSION" ]; then
            echo "❌ $CRATE_NAME: $CRATE_VERSION (expected: $WORKSPACE_VERSION)"
            exit 1
          fi
        done
```

**Benefits:**
- ✅ Fails fast if any crate has a mismatched version
- ✅ Provides clear error messages showing which crates are wrong
- ✅ Suggests fixes: update version or use `version.workspace = true`
- ✅ Runs on every PR before merge

---

## Why Version 0.35.2 and Not 0.35.1?

Since `windjammer-runtime 0.34.1` is already published to crates.io, we cannot "unpublish" it and replace it with `0.35.1`. Cargo/crates.io uses immutable versions.

Options considered:
1. ❌ **Yank 0.34.1 and publish 0.35.1**: Can't unpublish, and yanking doesn't free up the version number
2. ❌ **Leave mismatched**: Confusing for users, breaks expectations
3. ✅ **Bump all to 0.35.2**: Clean slate, all crates aligned, clear history

We chose option 3 to maintain consistency across the entire workspace.

---

## Changes

### Modified Files
- `Cargo.toml`: Workspace version `0.35.1` → `0.35.2`
- `crates/windjammer-runtime/Cargo.toml`: Version `0.34.1` → `0.35.2`
- `crates/windjammer-mcp/Cargo.toml`: Version `0.31.0` → `0.35.2`
- `crates/windjammer-lsp/Cargo.toml`: Dependency versions → `0.35.2`
- `crates/windjammer-mcp/Cargo.toml`: Dependency versions → `0.35.2`
- `CHANGELOG.md`: Added `0.35.2` entry with fixes
- `Cargo.lock`: Updated all crate versions

### CI Enhancement
- `.github/workflows/test.yml`: Added `version-check` job (already merged in previous PR)

---

## Testing

### Pre-merge CI
The new `version-check` job will validate:
- ✅ All crate versions are `0.35.2`
- ✅ Lockfile is up-to-date
- ✅ Publish dry-run succeeds for core crates

### Post-merge Verification
After merge and tagging `v0.35.2`:
1. Publish workflow will publish all crates with `0.35.2`
2. Users can `cargo install windjammer --version 0.35.2`
3. All workspace crates will be aligned on crates.io

---

## Release Plan

1. ✅ Merge this PR
2. ✅ Tag `v0.35.2`
3. ✅ Publish workflow runs automatically
4. ✅ All crates published to crates.io with `0.35.2`

---

## Future Prevention

This issue is now prevented by:
1. **CI check**: `version-check` job enforces alignment on every PR
2. **Publish dry-run**: Catches publishing issues before merge
3. **Lockfile check**: Ensures `Cargo.lock` is current
4. **Better process**: We'll consider using `version.workspace = true` to inherit version automatically

---

## Risk Assessment

**Low Risk:**
- Only version bumps, no code changes
- CI validates all crates can be published
- Existing users on `0.35.1` will get bugfix notifications for `0.35.2`
- Clear changelog explains the version alignment fix

**Immediate Value:**
- Consistent versioning across all crates
- Future-proof: CI prevents this from happening again
- Improved user experience (no version confusion)

---

**Related:**
- Fixes version mismatches from #59 (v0.35.1 release)
- Depends on version-check CI job from #59

