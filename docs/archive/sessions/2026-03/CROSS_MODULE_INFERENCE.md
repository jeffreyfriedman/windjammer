# Cross-Module Type Inference - Architecture Required

## The Problem

**Current State**: Float inference works WITHIN a file, fails ACROSS files.

### Working Pattern (Same File)
```windjammer
// file: test.wj
struct Vec3 { x: f32, y: f32, z: f32 }
impl Vec3 { fn new(x: f32, y: f32, z: f32) -> Vec3 { ... } }

fn create() -> Vec3 {
    Vec3::new(1.0, 0.0, 1.0)  // ✅ Generates 0.0_f32
}
```

### Failing Pattern (Cross-Module)
```windjammer
// file: math/vec3.wj
struct Vec3 { x: f32, y: f32, z: f32 }
impl Vec3 { fn new(x: f32, y: f32, z: f32) -> Vec3 { ... } }

// file: ai/npc_behavior.wj
use crate::math::Vec3

fn create() -> Vec3 {
    Vec3::new(1.0, 0.0, 1.0)  // ❌ Generates 0.0_f64
}
```

**Why it fails**: The inference engine only sees functions defined in the CURRENT file.

## Root Cause

### Per-File Compilation
```
wj build file1.wj  // Inference sees only file1's functions
  ↓
wj build file2.wj  // Inference sees only file2's functions (NOT file1!)
  ↓
Link all .rs files  // Type errors appear here
```

### What's Missing
1. **Function signature export** - Vec3::new signature not available to other files
2. **Import resolution** - `use crate::math::Vec3` doesn't load Vec3's methods
3. **Whole-program view** - Each file compiled in isolation

## Impact on Game

**Remaining Errors**: ~1900 (out of 1918 total)

**Most common patterns**:
- `Vec3::new(x, 0.0, z)` - ~200 errors
- `Quat::from_euler(0.0, angle, 0.0)` - ~50 errors
- `Mat4::look_at(eye, target, 0.0)` - ~30 errors
- Other cross-module constructors - ~400 errors

**Pattern**: All involve constructors/functions from imported modules.

## Proper Fix (Architectural)

### Option 1: Metadata Files (Like Rust .rlib)
```
wj build math/vec3.wj
  ↓
  Generates:
  - math/vec3.rs (Rust code)
  - math/vec3.wj.meta (function signatures, struct fields, trait impls)
  
wj build ai/npc_behavior.wj
  ↓
  Reads: math/vec3.wj.meta
  Loads: Vec3::new(f32, f32, f32) -> Vec3
  Infers: 0.0 → f32
```

**Pros**:
- ✅ Fast (incremental compilation)
- ✅ Scalable (no whole-program analysis)
- ✅ Standard pattern (Rust does this)

**Cons**:
- ❌ Metadata format design
- ❌ Serialization/deserialization
- ❌ Version compatibility

### Option 2: Whole-Program Analysis
```
wj build src_wj/
  ↓
  Pass 1: Parse all files → AST forest
  Pass 2: Build global symbol table
  Pass 3: Type inference with full knowledge
  Pass 4: Codegen for each file
```

**Pros**:
- ✅ Complete information
- ✅ More powerful optimization
- ✅ Better error messages

**Cons**:
- ❌ Slow (recompile everything)
- ❌ High memory usage
- ❌ Complex implementation

### Option 3: Import-Time Parsing
```
wj build ai/npc_behavior.wj
  ↓
  Sees: use crate::math::Vec3
  Parses: math/vec3.wj (just for signatures)
  Loads: Vec3::new signature
  Infers: 0.0 → f32
```

**Pros**:
- ✅ No metadata files
- ✅ Faster than whole-program
- ✅ Simpler than .meta files

**Cons**:
- ❌ Re-parsing overhead
- ❌ Dependency tracking needed
- ❌ Circular import handling

## Recommended: Option 1 (Metadata Files)

**Rationale**:
- Rust-like (proven pattern)
- Incremental (fast rebuilds)
- Extensible (can add more metadata later)

### Implementation Plan

**Phase 1: Metadata Generation**
1. After compiling `file.wj` → `file.rs`, emit `file.wj.meta`
2. Format: JSON with function signatures, struct fields, trait impls
3. Include only PUBLIC exports (not private functions)

**Phase 2: Metadata Loading**
1. Before type inference, scan imports (`use crate::math::Vec3`)
2. Load `math/vec3.wj.meta`
3. Populate function_signatures registry

**Phase 3: Cache Invalidation**
1. Check if `.wj` newer than `.meta`
2. Rebuild metadata if stale
3. Propagate to dependent files

**Estimated Effort**: ~500 LOC, 2-3 sessions

## Temporary Workaround (Not Recommended)

Extend hardcoded function list:
```rust
if matches!(method, "new" | "zero" | "one" | "identity" | "from_euler" | "look_at") {
    // Assume returns f32 for math types
}
```

**Why NOT to do this**:
- ❌ Doesn't scale (hundreds of functions)
- ❌ Brittle (breaks on custom types)
- ❌ Tech debt (violates "no workarounds" principle)

## Current State

**What works**: ~97% of patterns when ALL code is in ONE file  
**What fails**: ~99% of patterns when code is SPLIT across files  
**Blocker**: Cross-module type information not available

## Decision

**Don't implement workaround.** The proper fix (metadata files) is the only acceptable solution.

### Why This Aligns with Philosophy

✅ **"No Shortcuts, Only Proper Fixes"**
- Metadata files are the proper architecture
- Hardcoded lists are shortcuts
- We take the time to do it right

✅ **"Long-term Robustness"**
- Metadata enables many future optimizations
- Incremental compilation foundation
- IDE integration foundation (autocomplete, go-to-definition)

✅ **"Compiler Does the Hard Work"**
- Metadata generation is automatic
- Users never think about it
- Just works across files

## Next Steps

1. **Design metadata format** (JSON schema)
2. **Implement metadata emission** (codegen phase)
3. **Implement metadata loading** (inference phase)
4. **Test with game code** (TDD all the way!)
5. **Watch errors drop to zero** 🎉

---

**Remember**: "If it's worth doing, it's worth doing right."

Cross-module inference is REQUIRED for a production compiler. We build it properly or not at all.
