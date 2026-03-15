# Breach Protocol Black Screen - TDD Analysis

## Problem Statement

Game renders nearly pure black screen despite:
- ✅ Windjammer octree generating 16,241 nodes
- ✅ Camera positioned correctly (32, 6, 22) → (32, 1, 32)
- ✅ 6,181 non-empty voxels in grid
- ✅ SVO uploaded to GPU successfully

## Screenshot Evidence

**Frame 60 Analysis:**
```
Shape: (720, 1280, 4)
Top-left pixel: [12, 19, 36] (very dark blue)
Center pixel: [0, 0, 0] (pure black)
Bottom-right: [0, 0, 0] (pure black)
Unique colors: 2
Average RGB: [0.009, 0.014, 0.027] (nearly black)
Bright pixels: 0/921600 (0.0%)
```

**Conclusion:** Shader is running but returning **0 (empty)** for all voxel lookups!

## Root Cause Hypotheses

### 🎯 **Hypothesis 1: Octree Traversal Mismatch (MOST LIKELY)**

**Evidence:**
- Old flat SVO: 6,181 nodes → gray screen (sky color)
- New octree: 16,241 nodes → black screen
- Shader expects hierarchical traversal, we're providing hierarchy, but...

**Potential Issues:**
1. **Child pointer calculation incorrect**
   - Shader: `child_ptr = node >> 9`
   - Our code: `result[0] = (child_base_idx as u32) << 9`
   - These SHOULD match, but need verification

2. **Octant ordering mismatch**
   - Shader expects: x, y, z bit flags (0b00XYZ)
   - Our code builds children in order 0-7
   - Need to verify order matches

3. **Empty node representation**
   - We use `LEAF_FLAG` (0x100) for empty
   - Shader checks `(node & 0x100) != 0` for leaf
   - Material 0 means empty
   - This should work, but...

### 🔍 **Hypothesis 2: Homogeneous Collapse Issue**

**Evidence:**
```windjammer
// In svo_convert.wj
if is_homogeneous {
    let node = (first_material as u32) | LEAF_FLAG
    let mut result = Vec::new()
    result.push(node)
    return result
}
```

**Problem:** If large regions collapse to single nodes, shader traversal might skip them!

**Example:**
- Grid has solid 32x32 ground at y=0
- Octree collapses this to 1 node at level 2
- Shader might not know to check collapsed nodes?

### 🐛 **Hypothesis 3: Bounds/Coordinate System**

**Evidence:**
- Grid is 64x64x64
- World size passed as 128
- Voxels at positions like (32, 1, 32)

**Potential Issue:**
- Shader might be looking up wrong coordinates
- Off-by-one in octree subdivision
- Size mismatch (grid 64 vs world 128)

## TDD Test Plan

### ✅ **Created Tests**

1. **`svo_octree_integration_test.rs`** - Validates Windjammer compilation
2. **`svo_shader_compat_test.rs`** - Simulates shader traversal
3. **`svo_debug.wj`** - Debug utilities (Windjammer!)

### 🔬 **Next Tests Needed**

1. **Test actual game octree structure**
   ```rust
   #[test]
   fn test_breach_protocol_octree_structure() {
       // Build actual game grid (64x64 with ground)
       // Generate octree
       // Validate structure matches shader expectations
   }
   ```

2. **Test shader lookup with real data**
   ```rust
   #[test]
   fn test_shader_lookup_ground_voxel() {
       // Use actual game octree
       // Simulate shader looking up (32, 0, 32)
       // Should find material, not 0
   }
   ```

3. **Test homogeneous collapse behavior**
   ```rust
   #[test]
   fn test_collapsed_regions_are_traversable() {
       // Create homogeneous region
       // Verify shader can still find voxels
   }
   ```

## Action Items

### 🚀 **Immediate (RED Phase - Tests First)**

1. ✅ Create integration tests
2. ✅ Create shader compatibility tests
3. ⏳ Run tests and find failures
4. ⏳ Fix Windjammer octree based on test failures

### 🔧 **Debug Tools**

1. ✅ Add SVO inspector to game
2. ⏳ Run game with inspector output
3. ⏳ Examine actual octree structure
4. ⏳ Compare with shader expectations

### 🎨 **Shader Debug**

1. ⏳ Add debug shader that visualizes octree depth
2. ⏳ Add shader that shows octant IDs
3. ⏳ Verify traversal path visually

## Expected Fix

**Once tests reveal the issue, likely fix will be:**

Option A: **Child pointer base off-by-one**
```windjammer
// Wrong:
result[0] = (child_base_idx as u32) << 9

// Right:
result[0] = ((result.len()) as u32) << 9
```

Option B: **Octant ordering mismatch**
```windjammer
// Need to match shader's exact order:
// 0:(-x,-y,-z) 1:(+x,-y,-z) 2:(-x,+y,-z) 3:(+x,+y,-z)
// 4:(-x,-y,+z) 5:(+x,-y,+z) 6:(-x,+y,+z) 7:(+x,+y,+z)
```

Option C: **Homogeneous nodes need subdivision info**
```windjammer
// Don't collapse if it would break traversal
// OR: Add metadata for shader to handle collapsed nodes
```

## Philosophy Adherence

✅ **TDD** - Tests written before fix attempts
✅ **Dogfooding** - Debug tools in Windjammer
✅ **No workarounds** - Finding root cause, not patching
✅ **Parallel work** - Multiple tests running simultaneously

---

**Status:** 🔴 RED Phase (tests failing, root cause investigation)
**Next:** 🟢 GREEN Phase (fix implementation based on test results)
