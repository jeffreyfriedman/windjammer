# Black Screen Fix Plan - TDD Approach

## 🐛 Root Cause: SVO Structure Mismatch

**Problem:** Voxel raymarch shader expects proper octree with parent/child pointers, but CPU generates flat list.

**Evidence:**
- Game renders sky color (shader runs)
- No voxels visible (traversal fails)
- 16,241 SVO nodes uploaded (data exists)
- Camera positioned correctly

---

## 🎯 TDD Fix Strategy

### Phase 1: Create Minimal Test Case ✅
```rust
#[test]
fn test_svo_structure_basic() {
    // GIVEN: 2x2x2 voxel grid with 1 filled voxel
    let grid = VoxelGrid::new(2);
    grid.set(0, 0, 0, 1); // One red voxel
    
    // WHEN: Convert to SVO
    let svo = voxelgrid_to_svo_flat(&grid);
    
    // THEN: Should have root + 1 leaf
    assert_eq!(svo.len(), 2);
    assert!(svo[0] & 0x100 == 0); // Root: not a leaf
    assert!(svo[1] & 0x100 != 0); // Child: is a leaf
    assert_eq!(svo[1] & 0xFF, 1); // Material = 1 (red)
}
```

### Phase 2: Fix SVO Builder (Proper Octree)

**Current (BROKEN):**
```rust
// Flat list - no hierarchy!
for voxel in voxels {
    svo_nodes.push((material & 0xFF) | 0x100); // LEAF_FLAG only
}
```

**Fixed (PROPER OCTREE):**
```rust
fn build_octree_recursive(
    grid: &VoxelGrid,
    origin: Vec3<u32>,
    size: u32,
    nodes: &mut Vec<u32>
) -> u32 {
    if size == 1 {
        // Leaf node: material + LEAF_FLAG
        let mat = grid.get(origin.x, origin.y, origin.z);
        let node_idx = nodes.len();
        nodes.push((mat & 0xFF) | 0x100);
        return node_idx as u32;
    }
    
    // Branch node: child mask + child pointer
    let half_size = size / 2;
    let mut child_mask = 0u8;
    let children_start = nodes.len() + 1; // Reserve space for this node
    
    nodes.push(0); // Placeholder
    
    // Build 8 octants
    for octant in 0..8 {
        let offset = octant_offset(octant) * half_size;
        let child_origin = origin + offset;
        
        if has_voxels(&grid, child_origin, half_size) {
            child_mask |= 1 << octant;
            build_octree_recursive(grid, child_origin, half_size, nodes);
        }
    }
    
    // Update parent node: child_mask (bits 0-7) + child_ptr (bits 9-31)
    let node = (child_mask as u32) | ((children_start as u32) << 9);
    nodes[nodes.len() - (children_start - nodes.len())] = node;
    
    (nodes.len() - 1) as u32
}
```

### Phase 3: Verify Shader Traversal

**Shader Code (voxel_raymarch.wgsl):**
```wgsl
fn lookup_svo(pos: vec3<f32>, world_size: f32) -> u32 {
    var node_idx: u32 = 0u; // Start at root
    var current_size = world_size;
    
    while current_size > 1.0 {
        let node = svo_data[node_idx];
        
        // Check if leaf
        if (node & 0x100u) != 0u {
            return node & 0xFFu; // Return material
        }
        
        // Branch: get octant and traverse
        let half_size = current_size * 0.5;
        let octant = compute_octant(pos, current_size);
        let child_mask = node & 0xFFu;
        
        // Check if octant has child
        if (child_mask & (1u << octant)) == 0u {
            return 0u; // Empty
        }
        
        // Get child pointer (bits 9-31)
        let child_base = (node >> 9u) & 0x7FFFFFu;
        node_idx = child_base + count_bits_before(child_mask, octant);
        
        current_size = half_size;
    }
    
    return 0u;
}
```

---

## 🧪 TDD Test Suite

### Test 1: Single Voxel ✅
```rust
#[test]
fn test_single_voxel_svo() {
    let mut grid = VoxelGrid::new(2);
    grid.set(0, 0, 0, 1);
    
    let svo = build_svo(&grid);
    
    // Root node: has 1 child in octant 0
    assert_eq!(svo[0] & 0xFF, 0x01); // child_mask = 0b00000001
    
    // Child node: leaf with material 1
    assert_eq!(svo[1], 0x101); // LEAF_FLAG | material
}
```

### Test 2: Full 2x2x2 Grid ✅
```rust
#[test]
fn test_full_grid_svo() {
    let mut grid = VoxelGrid::new(2);
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                grid.set(x, y, z, x + y*2 + z*4 + 1);
            }
        }
    }
    
    let svo = build_svo(&grid);
    
    // Root node: all 8 octants filled
    assert_eq!(svo[0] & 0xFF, 0xFF); // child_mask = 0b11111111
    
    // 8 leaf nodes
    assert_eq!(svo.len(), 9); // 1 root + 8 leaves
}
```

### Test 3: Sparse Grid (Real World) ✅
```rust
#[test]
fn test_sparse_grid_svo() {
    let mut grid = VoxelGrid::new(64);
    // Place voxels in corners
    grid.set(0, 0, 0, 1);
    grid.set(63, 63, 63, 2);
    
    let svo = build_svo(&grid);
    
    // Should have minimal nodes (not 64^3!)
    assert!(svo.len() < 100); // Much less than full grid
    
    // Verify corner voxels are retrievable
    let mat1 = lookup_svo_cpu(&svo, Vec3::new(0.5, 0.5, 0.5), 64.0);
    assert_eq!(mat1, 1);
    
    let mat2 = lookup_svo_cpu(&svo, Vec3::new(63.5, 63.5, 63.5), 64.0);
    assert_eq!(mat2, 2);
}
```

---

## 🚀 Implementation Steps

### Step 1: Write Failing Tests
- [x] Document current broken behavior
- [ ] Write test_single_voxel_svo
- [ ] Write test_full_grid_svo
- [ ] Write test_sparse_grid_svo
- [ ] Run tests (FAIL expected)

### Step 2: Implement Proper SVO Builder
- [ ] Create `build_octree_recursive()`
- [ ] Implement octant calculation
- [ ] Implement child pointer logic
- [ ] Replace flat list generation

### Step 3: Verify with CPU Traversal
- [ ] Implement `lookup_svo_cpu()` (mirrors GPU code)
- [ ] Test that CPU lookup works
- [ ] Ensures GPU will work the same way

### Step 4: GPU Integration
- [ ] Upload new SVO structure
- [ ] Verify shader still compiles
- [ ] Run game, check for voxels
- [ ] Take screenshot, verify rendering

### Step 5: Iterate Until Working
- [ ] If still broken, add more logging
- [ ] Debug with smaller grid (2x2x2)
- [ ] Verify each octree level
- [ ] Fix shader if needed

---

## 📊 Success Criteria

✅ **Tests pass:**
- Single voxel lookup works
- Full grid lookup works
- Sparse grid lookup works

✅ **Game renders:**
- Voxels visible on screen
- Not just sky color
- Correct materials/colors

✅ **Performance:**
- SVO size reasonable (<< N^3)
- Shader traversal fast
- 60fps maintained

---

## 🎯 Expected Results

### Before Fix:
- **Screen:** Black/gray (sky only)
- **SVO:** 16,241 flat nodes (no hierarchy)
- **Traversal:** Fails (no child pointers)

### After Fix:
- **Screen:** Voxel world visible
- **SVO:** ~1,000-5,000 nodes (sparse octree)
- **Traversal:** Works (proper hierarchy)

---

## 🔧 Tools & Commands

### Build & Test:
```bash
cd /Users/jeffreyfriedman/src/wj/windjammer-game
cargo test svo --lib
```

### Run Game:
```bash
cd /Users/jeffreyfriedman/src/wj/breach-protocol/runtime_host
./target/release/breach-protocol-host
```

### Take Screenshot:
```bash
screencapture -x /tmp/breach_protocol_fix.png
open /tmp/breach_protocol_fix.png
```

---

**The Windjammer Way: TDD-driven, proper fixes only, no workarounds!** 🚀
