// TDD: SVO Shader Compatibility Tests
//
// Validate that octree structure matches WGSL shader expectations

#[test]
fn test_shader_lookup_empty_grid() {
    // GIVEN: Empty SVO (single empty leaf)
    const LEAF_FLAG: u32 = 0x100;
    let svo = vec![LEAF_FLAG];  // Empty leaf at root
    
    // WHEN: Shader looks up any position
    let material = lookup_svo(&svo, 0, 0.5, 0.5, 0.5, 8.0, 3);
    
    // THEN: Should return 0 (empty)
    assert_eq!(material, 0, "Empty grid should return material 0");
}

#[test]
fn test_shader_lookup_single_voxel() {
    // GIVEN: SVO with one voxel at center
    const LEAF_FLAG: u32 = 0x100;
    
    // Build minimal octree: root -> children -> leaf with material
    // Root is interior, points to 8 children at index 1
    let root = 1u32 << 9;  // Interior, children at 1
    
    // 8 children (octants), octant 7 (+x,+y,+z) has the voxel
    let mut svo = vec![root];
    for i in 0..8 {
        if i == 7 {
            svo.push(5 | LEAF_FLAG);  // Material 5 leaf
        } else {
            svo.push(LEAF_FLAG);  // Empty leaves
        }
    }
    
    // WHEN: Shader looks up center position (in octant 7)
    let material = lookup_svo(&svo, 0, 6.0, 6.0, 6.0, 8.0, 3);
    
    // THEN: Should find material 5
    assert_eq!(material, 5, "Should find material at center");
}

#[test]
fn test_shader_lookup_out_of_bounds() {
    // GIVEN: Any SVO
    const LEAF_FLAG: u32 = 0x100;
    let svo = vec![LEAF_FLAG];
    
    // WHEN: Shader looks up position outside world
    let material = lookup_svo(&svo, 0, 10.0, 10.0, 10.0, 8.0, 3);
    
    // THEN: Should return 0 (empty)
    assert_eq!(material, 0, "Out of bounds should return 0");
}

// Simulate shader's lookup_svo function
fn lookup_svo(svo: &[u32], mut node_idx: usize, x: f32, y: f32, z: f32, world_size: f32, max_depth: u32) -> u32 {
    const LEAF_FLAG: u32 = 0x100;
    
    // Check bounds
    if x < 0.0 || x >= world_size || y < 0.0 || y >= world_size || z < 0.0 || z >= world_size {
        return 0;
    }
    
    if svo.is_empty() {
        return 0;
    }
    
    let mut node_min = (0.0f32, 0.0f32, 0.0f32);
    let mut node_size = world_size;
    
    for _depth in 0..max_depth {
        if node_idx >= svo.len() {
            return 0;
        }
        
        let node = svo[node_idx];
        let is_leaf = (node & LEAF_FLAG) != 0;
        
        if is_leaf {
            return (node & 0xFF) as u32;
        }
        
        // Interior node - descend
        let half = node_size * 0.5;
        let center = (
            node_min.0 + half,
            node_min.1 + half,
            node_min.2 + half,
        );
        
        // Get octant
        let mut octant = 0u32;
        if x >= center.0 { octant |= 1; }
        if y >= center.1 { octant |= 2; }
        if z >= center.2 { octant |= 4; }
        
        // Update bounds for next iteration
        if x >= center.0 { node_min.0 = center.0; } else { node_min.0 = node_min.0; }
        if y >= center.1 { node_min.1 = center.1; } else { node_min.1 = node_min.1; }
        if z >= center.2 { node_min.2 = center.2; } else { node_min.2 = node_min.2; }
        node_size = half;
        
        // Get child pointer and advance
        let child_ptr = (node >> 9) as usize;
        node_idx = child_ptr + octant as usize;
    }
    
    0  // Max depth reached
}

#[test]
fn test_shader_traversal_multiple_levels() {
    // GIVEN: Deep octree (3 levels)
    const LEAF_FLAG: u32 = 0x100;
    
    let mut svo = Vec::new();
    
    // Level 0: Root interior, children at 1
    svo.push(1u32 << 9);
    
    // Level 1: 8 children, octant 7 is interior pointing to 9
    for i in 0..8 {
        if i == 7 {
            svo.push(9u32 << 9);  // Interior
        } else {
            svo.push(LEAF_FLAG);  // Empty
        }
    }
    
    // Level 2: 8 more children, octant 7 has material 3
    for i in 0..8 {
        if i == 7 {
            svo.push(3 | LEAF_FLAG);
        } else {
            svo.push(LEAF_FLAG);
        }
    }
    
    // WHEN: Looking up deep position
    let material = lookup_svo(&svo, 0, 7.0, 7.0, 7.0, 8.0, 5);
    
    // THEN: Should traverse and find material
    assert_eq!(material, 3, "Should traverse deep octree");
}
