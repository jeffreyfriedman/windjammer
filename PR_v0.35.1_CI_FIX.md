# Fix CI Release and Publish Workflows (v0.35.1)

## Problem

Two critical CI failures in the v0.35.0 release:

### 1. Release Workflow Failures
All platform builds failing at the `strip` step:
- **Linux**: `strip: 'target/x86_64-unknown-linux-gnu/release/windjammer': No such file`
- **macOS**: `can't open file: target/aarch64-apple-darwin/release/windjammer (No such file or directory)`
- **Windows**: `The system cannot find the file specified. windjammer.exe`

**Root Cause**: Cross-compilation with `cargo build --target` is failing silently, but we only detect the failure at the `strip` step.

### 2. Publish Workflow Dry-Run Failure
```
error: failed to select a version for the requirement `windjammer = ww"^0.35.0"`
candidate versions found which didn't match: 0.30.0, 0.29.0, 0.28.0, ...
location searched: crates.io index
required by package `windjammer-lsp v0.35.0`
```

**Root Cause**: `cargo publish --dry-run` for dependent crates tries to resolve dependencies from crates.io, but `windjammer 0.35.0` hasn't been published yet (chicken-and-egg problem).

---

## Solution

### 1. Release Workflow: Add Binary Verification
Added a verification step immediately after the build:
```yaml
- name: Verify binary exists
  shell: bash
  run: |
    if [ -f "target/${{ matrix.target }}/release/${{ matrix.artifact_name }}" ]; then
      echo "✅ Binary found at target/${{ matrix.target }}/release/${{ matrix.artifact_name }}"
      ls -lh "target/${{ matrix.target }}/release/${{ matrix.artifact_name }}"
    else
      echo "❌ Binary NOT found at target/${{ matrix.target }}/release/${{ matrix.artifact_name }}"
      echo "Contents of target/${{ matrix.target }}/release/:"
      ls -la "target/${{ matrix.target }}/release/" || echo "Directory doesn't exist"
      exit 1
    fi
```

**Benefits**:
- Fails fast if binary isn't built
- Provides diagnostic output (lists directory contents)
- Helps debug cross-compilation issues

### 2. Publish Workflow: Skip Dependent Crate Dry-Runs
Changed dry-run to only validate core crates:
```yaml
- name: Dry run publish (windjammer core only)
  run: |
    echo "Dry run: windjammer (core)"
    cargo publish --dry-run
    echo "Dry run: windjammer-runtime"
    cargo publish --dry-run -p windjammer-runtime
    # Skip dry-run for windjammer-lsp and windjammer-mcp
    # They depend on windjammer = "0.35.1" which isn't on crates.io yet
    echo "✓ Core crates validated, dependent crates will be validated during publish"
```

**Why this works**:
- Core crates (`windjammer`, `windjammer-runtime`) have no unpublished dependencies → dry-run succeeds
- Dependent crates (`windjammer-lsp`, `windjammer-mcp`) will be validated during **actual publish** after their dependencies are already live on crates.io
- Publish order: `windjammer` → wait 30s → `windjammer-runtime` → wait 30s → `windjammer-lsp` → `windjammer-mcp`

### 3. Test Workflow: Add Lockfile Validation + Publish Dry-Run
Added two new jobs to enforce best practices:

**Job 1: `lockfile-check`** - Validates Cargo.lock
```yaml
lockfile-check:
  steps:
    - name: Check Cargo.lock exists
      # Fails if Cargo.lock is not committed
    - name: Verify Cargo.lock is up-to-date
      # Fails if Cargo.lock is out of sync with Cargo.toml
```

**Job 2: `publish-dryrun`** - Validates publishing
```yaml
publish-dryrun:
  needs: lockfile-check  # Only runs if lockfile is valid
  steps:
    - name: Dry-run publish (core crates only)
      run: |
        cargo publish --dry-run
        cargo publish --dry-run -p windjammer-runtime
```

**Benefits**:
- ✅ Enforces that `Cargo.lock` is always committed (catches `--no-verify` bypasses)
- ✅ Enforces that `Cargo.lock` is always up-to-date
- ✅ Catches publish issues **before merge** (not after tag is created)
- ✅ No `--allow-dirty` needed (lockfile is guaranteed to be committed)
- ✅ Fast feedback loop (fails in CI, not during manual release)

---

## Changes

### Modified Files
- `.github/workflows/release.yml`: Added binary verification step
- `.github/workflows/publish.yml`: Restricted dry-run to core crates only
- `.github/workflows/test.yml`: **Added publish dry-run job to catch issues pre-merge**
- `Cargo.toml`: Bumped version to `0.35.1`
- `crates/windjammer-lsp/Cargo.toml`: Updated dependency version to `0.35.1`
- `crates/windjammer-mcp/Cargo.toml`: Updated dependency versions to `0.35.1`
- `CHANGELOG.md`: Added `0.35.1` entry with CI fixes

### Version
- **Old**: `0.35.0` (failed to release)
- **New**: `0.35.1`

---

## Testing

### Pre-commit Checks
✅ All pre-commit checks passed locally:
- Version consistency
- Version increment
- Code formatting
- Clippy lints
- Tests

### CI Validation
After merge, monitor:
1. **Test workflow**: Should pass (no changes to code)
2. **Publish workflow**: Dry-run should now succeed for core crates
3. **Release workflow**: Binary verification should reveal cross-compilation issues (or succeed)

---

## Next Steps

1. **If release still fails**: The new verification step will provide detailed diagnostics about why cross-compilation isn't working. We may need to:
   - Use `cross` tool for cross-compilation
   - Build natively on each platform instead of cross-compiling
   - Add additional build dependencies

2. **If release succeeds**: Tag and publish `v0.35.1` with these CI improvements.

---

## Risk Assessment

**Low Risk**:
- Only changes CI configuration (no code changes)
- Dry-run is still performed for core crates (catches most issues)
- Dependent crates are validated during actual publish (safe because dependencies are live)
- Pre-commit checks enforce quality (formatting, lints, tests)

**Immediate Value**:
- Unblocks publishing to crates.io
- Provides better diagnostics for release issues
- Reduces CI friction for future releases

