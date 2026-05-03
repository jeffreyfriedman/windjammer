# wgpu-ffi Analysis: Dead Code Removal

## Discovery

**Found:**
- `/Users/jeffreyfriedman/src/wj/wgpu-ffi/` (standalone)
- `/Users/jeffreyfriedman/src/wj/windjammer-game/wgpu-ffi/` (duplicate!)
- `/Users/jeffreyfriedman/src/wj/winit-ffi/` (related)

## Analysis

### What is wgpu-ffi?

**Purpose:** FFI layer for exposing wgpu to C/Windjammer
- Global static storage for GPU objects
- C-compatible API (cdylib)
- Built as dynamic library (libwgpu_ffi.dylib)

### Is it used?

**Declared in:**
- `windjammer-game/Cargo.toml` - dependency declared
- `windjammer-game/build.rs` - linked as dylib

**Used in:**
- ❌ **ZERO usage!** No `use wgpu_ffi` or `extern` declarations found
- ❌ breach-protocol doesn't reference it
- ❌ windjammer-runtime-host doesn't reference it

### Why does it exist?

**Historical context:** Old architecture from early prototyping

**Old approach:**
```
Windjammer → C FFI → wgpu-ffi → wgpu
```

**Current approach:**
```
Windjammer → windjammer-runtime-host → wgpu (direct)
```

**wgpu-ffi is superseded by windjammer-runtime-host!**

---

## Recommendation: DELETE ✅

### Reasons:

1. **Not used** - Zero references in codebase
2. **Superseded** - windjammer-runtime-host provides better architecture
3. **Technical debt** - Adds complexity, no value
4. **Duplicate** - Two copies (!), both unused
5. **Old architecture** - From experimental phase

### Impact of deletion:

**Breaking changes:** ❌ NONE (not used anywhere)

**Benefits:**
- ✅ Reduces confusion (what is this?)
- ✅ Removes technical debt
- ✅ Cleaner project structure
- ✅ Faster searches (less noise)

---

## Proper Architecture

### Current (Correct) Architecture

```
Windjammer (.wj files)
    ↓ (wj compiler)
Generated Rust (.rs files)
    ↓ (in windjammer-game-core)
windjammer-runtime-host (Rust FFI layer)
    ↓ (uses wgpu directly)
wgpu (GPU abstraction)
    ↓
GPU Hardware
```

**No FFI layer needed** - Windjammer compiles to Rust, Rust uses wgpu directly!

---

## Deletion Plan

### Step 1: Remove from windjammer-game

```bash
cd /Users/jeffreyfriedman/src/wj/windjammer-game

# Remove wgpu-ffi directory
rm -rf wgpu-ffi/

# Remove from Cargo.toml
sed -i.bak '/wgpu-ffi/d' Cargo.toml

# Remove from build.rs (or delete build.rs entirely)
# Check if build.rs has other important logic first
```

### Step 2: Check if build.rs is still needed

```bash
cat build.rs
# If only wgpu-ffi/winit-ffi linking, delete it
# If has other logic, remove FFI lines only
```

### Step 3: Remove standalone wgpu-ffi

```bash
cd /Users/jeffreyfriedman/src/wj

# Remove wgpu-ffi directory
rm -rf wgpu-ffi/

# Remove winit-ffi directory
rm -rf winit-ffi/
```

### Step 4: Verify builds

```bash
cd /Users/jeffreyfriedman/src/wj/windjammer-game
cargo check
# Expected: Success (no wgpu-ffi needed)

cd /Users/jeffreyfriedman/src/wj/breach-protocol
wj game build --release
# Expected: Success (never used wgpu-ffi)
```

---

## Conclusion

**Verdict: DELETE wgpu-ffi and winit-ffi** ✅

**Reason:** Technical debt from old architecture, completely unused.

**Current architecture (windjammer-runtime-host) is superior:**
- Direct wgpu usage (no FFI overhead)
- Better type safety
- Simpler to maintain
- No global statics

**Action:** Delete all 3 directories (wgpu-ffi, windjammer-game/wgpu-ffi, winit-ffi).
