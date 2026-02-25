# Disk Space Analysis & Cleanup Recommendations

**Analysis Date:** 2026-02-22
**Current Status:** Recently freed 4.2GB via `cargo clean`

---

## 📊 Current Disk Usage Breakdown

### Workspace (/Users/jeffreyfriedman/src/wj)
**Total:** ~10.1GB

| Directory | Size | Type | Safe to Remove? |
|-----------|------|------|-----------------|
| **windjammer-game** | 3.5GB | Project | ⚠️ Partial |
| **windjammer** | 3.1GB | Project | ⚠️ Partial |
| **build** | 2.3GB | Artifacts | ✅ YES |
| **windjammer-ui** | 1.2GB | Project | ⚠️ Partial |
| **windjammer-stream** | 408KB | Project | 🔒 Keep |

### Build Artifacts (Within Projects)

| Path | Size | Safe to Remove? |
|------|------|-----------------|
| `/build/target` | 2.3GB | ✅ **YES - High Priority** |
| `/windjammer-ui/target` | 1.0GB | ✅ **YES - High Priority** |
| `/windjammer-game/target` | 981MB | ✅ **YES - High Priority** |
| `/windjammer/target` | 691MB | ✅ **YES - High Priority** |
| `/windjammer/test_output` | 31MB | ✅ **YES - Test artifacts** |
| `/windjammer/build` | 10MB | ✅ **YES - Generated code** |

**Total Removable:** ~5.0GB (target directories + test artifacts)

### Git Repositories

| Path | Size | Safe to Remove? |
|------|------|-----------------|
| `/windjammer/.git` | 2.4GB | ⚠️ **NO - Version history** |
| `/windjammer-ui/.git` | 189MB | ⚠️ **NO - Version history** |
| `/windjammer-game/.git` | 9.7MB | ⚠️ **NO - Version history** |

**Note:** Git repos are large but critical. Consider `git gc --aggressive` if needed.

### System-Level Caches

| Path | Size | Safe to Remove? |
|------|------|-----------------|
| `~/.cargo` | 1.8GB | ⚠️ **Partial - Use cargo cache** |
| `~/.rustup` | 1.8GB | ⚠️ **Partial - Multiple toolchains?** |
| `~/.cursor/projects` | 219MB | ⚠️ **Partial - IDE metadata** |

---

## 🎯 RECOMMENDED CLEANUP PLAN

### ✅ High Priority (Safe, High Impact)

#### 1. Clean All Cargo Build Artifacts (~5GB)
```bash
# Clean all target directories
cd /Users/jeffreyfriedman/src/wj/windjammer && cargo clean
cd /Users/jeffreyfriedman/src/wj/windjammer-game && cargo clean
cd /Users/jeffreyfriedman/src/wj/windjammer-ui && cargo clean

# Remove standalone build directory
rm -rf /Users/jeffreyfriedman/src/wj/build/target

# Clean windjammer build artifacts
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/build/target
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/build/*.rs
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/build/Cargo.lock
```

**Expected Recovery:** ~5.0GB
**Risk:** None (all regenerable)
**Time to Rebuild:** 5-10 minutes on next `cargo build`

#### 2. Clean Test Output Artifacts (~31MB)
```bash
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/test_output
```

**Expected Recovery:** 31MB
**Risk:** None (test compilation artifacts)

#### 3. Clean Windjammer Build Directory (~10MB)
```bash
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/build
```

**Expected Recovery:** 10MB
**Risk:** None (generated Rust code, regenerated on compile)

**TOTAL HIGH PRIORITY:** ~5.04GB recovery

---

### ⚠️ Medium Priority (Safe, Moderate Impact)

#### 4. Clean Cargo Cache (~500MB-1GB potential)
```bash
# Remove old/unused dependencies
cargo cache --autoclean

# Or more aggressive (removes everything not in current Cargo.lock files)
cargo cache --remove-dir all
```

**Expected Recovery:** 500MB-1GB
**Risk:** Low (dependencies re-downloaded on next build)
**Recommended:** Use `--autoclean` first, not `--remove-dir all`

#### 5. Clean Rustup Old Toolchains (~500MB-1GB potential)
```bash
# List installed toolchains
rustup toolchain list

# Remove old/unused toolchains (if you have multiple)
rustup toolchain uninstall <toolchain-name>

# Example: Remove nightly if not needed
rustup toolchain uninstall nightly
```

**Expected Recovery:** Depends on number of toolchains
**Risk:** Low (can reinstall anytime)
**Check First:** Run `rustup toolchain list` to see what you have

#### 6. Clean Old Cursor Project Metadata (~50-100MB)
```bash
# Clean old agent transcripts (if very large)
find ~/.cursor/projects -name "*.txt" -mtime +30 -delete
```

**Expected Recovery:** 50-100MB
**Risk:** Low (old session transcripts only)

**TOTAL MEDIUM PRIORITY:** ~1-2GB potential

---

### 🔍 Low Priority (Investigate First)

#### 7. Git Repository Optimization
```bash
# Compress git history (run in each repo)
cd /Users/jeffreyfriedman/src/wj/windjammer
git gc --aggressive --prune=now
```

**Expected Recovery:** 100-500MB (variable)
**Risk:** None (just compresses, doesn't lose data)
**Time:** 5-15 minutes per repo
**Note:** Windjammer .git is 2.4GB - unusually large!

**Potential Issue:** Large binary files or many commits?
```bash
# Check for large files in history
cd /Users/jeffreyfriedman/src/wj/windjammer
git rev-list --objects --all | \
  git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' | \
  sed -n 's/^blob //p' | sort --numeric-sort --key=2 | tail -20
```

---

## 📋 QUICK REFERENCE: Non-Critical Removable Items

### ✅ Always Safe to Remove (Regenerable)
- `*/target` directories (Rust build artifacts)
- `*/build` directories (generated code)
- `*/test_output` directories (test artifacts)
- `*.log` files (application logs)
- `*.tmp` files (temporary files)

### ⚠️ Safe with Caveats (Re-downloadable)
- `~/.cargo/registry` (dependency cache)
- `~/.cargo/git` (git dependency cache)
- Old rustup toolchains (if multiple installed)

### 🔒 NEVER Remove (Critical)
- `.git` directories (version control history)
- `src/`, `src_wj/` directories (source code)
- `Cargo.toml`, `Cargo.lock` files (dependency definitions)
- Documentation `.md` files (project docs)

---

## 🚀 RECOMMENDED ACTION PLAN

### Immediate (Run Now)
```bash
# 1. Clean all cargo builds (~5GB)
cd /Users/jeffreyfriedman/src/wj/windjammer && cargo clean
cd /Users/jeffreyfriedman/src/wj/windjammer-game && cargo clean
cd /Users/jeffreyfriedman/src/wj/windjammer-ui && cargo clean

# 2. Remove standalone build artifacts
rm -rf /Users/jeffreyfriedman/src/wj/build/target

# 3. Clean test artifacts
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/test_output
rm -rf /Users/jeffreyfriedman/src/wj/windjammer/build
```

**Expected Recovery:** ~5.04GB
**Time:** 1-2 minutes

### Optional (If More Space Needed)
```bash
# 4. Check installed Rust toolchains
rustup toolchain list

# 5. Remove unused toolchains (if any)
# rustup toolchain uninstall <name>

# 6. Clean cargo cache (conservative)
cargo install cargo-cache
cargo cache --autoclean

# 7. Optimize git repos (time-consuming)
cd /Users/jeffreyfriedman/src/wj/windjammer && git gc --aggressive
```

**Expected Additional Recovery:** 1-2GB
**Time:** 10-30 minutes

---

## 💾 Current Disk Status

**Before Cleanup:** Unknown (analysis run after recent 4.2GB cleanup)
**After Recent Cleanup:** 4.2GB freed via `cargo clean`
**Potential Additional:** 5.04GB (high priority items)
**Total Potential:** ~9.24GB recoverable space

---

## 🔔 Maintenance Recommendations

### Weekly
- Run `cargo clean` after major work sessions
- Delete old test artifacts

### Monthly
- Run `cargo cache --autoclean`
- Check `rustup toolchain list` and remove unused

### Quarterly
- Run `git gc --aggressive` on large repos
- Audit `.cargo` and `.rustup` sizes

### Before Major Work
- Free up 10-15GB buffer space
- Prevents build failures from disk exhaustion
- Enables faster compilation (SSD performance)

---

## 📌 Summary

**Quick Win:** Clean all `target` directories → ~5GB freed in 2 minutes
**Best Practice:** Run `cargo clean` regularly, especially on laptops
**Investigation Needed:** Why is `windjammer/.git` 2.4GB? (unusually large)

**Next Steps:**
1. Run immediate cleanup commands above
2. Check rustup toolchains (`rustup toolchain list`)
3. Consider `git gc` on windjammer repo (investigate large .git)
4. Install `cargo-cache` for future maintenance
