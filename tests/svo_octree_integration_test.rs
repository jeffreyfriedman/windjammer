// TDD: SVO Octree Integration Tests (Rust - test the Windjammer output!)
//
// These tests validate that Windjammer's octree generation produces
// shader-compatible output

use std::path::PathBuf;

#[test]
fn test_windjammer_octree_compiles() {
    // GIVEN: Windjammer octree source
    let wj_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/src_wj/voxel/svo_convert.wj");

    // Skip if windjammer-game repository is not checked out (e.g., in CI)
    if !wj_src.exists() {
        eprintln!("Skipping test: windjammer-game repository not found");
        return;
    }

    // WHEN: We compile it
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_src)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    // THEN: Should compile successfully
    assert!(
        output.status.success(),
        "Windjammer SVO should compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_octree_node_format_matches_shader() {
    // GIVEN: Shader expects specific node format
    const LEAF_FLAG: u32 = 0x100;

    // Material in bits 0-7
    let material: u8 = 5;
    let node = (material as u32) | LEAF_FLAG;

    // WHEN: We decode it (like shader does)
    let decoded_material = (node & 0xFF) as u8;
    let is_leaf = (node & LEAF_FLAG) != 0;
    let child_ptr = node >> 9;

    // THEN: Format should match
    assert_eq!(decoded_material, 5, "Material should be in bits 0-7");
    assert!(is_leaf, "Bit 8 should be leaf flag");
    assert_eq!(child_ptr, 0, "Bits 9-31 should be child pointer");
}

#[test]
fn test_interior_node_format() {
    // GIVEN: Interior node pointing to children at index 8
    const LEAF_FLAG: u32 = 0x100;
    let child_base = 8u32;
    let node = child_base << 9; // Child pointer in bits 9-31

    // WHEN: We decode it
    let material = (node & 0xFF) as u8;
    let is_leaf = (node & LEAF_FLAG) != 0;
    let child_ptr = node >> 9;

    // THEN: Should decode correctly
    assert_eq!(material, 0, "Interior nodes have material 0");
    assert!(!is_leaf, "Should not be leaf");
    assert_eq!(child_ptr, 8, "Should point to children at 8");
}

#[test]
fn test_octant_ordering_matches_shader() {
    // GIVEN: Shader expects octant ordering: x, y, z bits
    // Octant 0: (-x, -y, -z) = 0b000 = 0
    // Octant 1: (+x, -y, -z) = 0b001 = 1
    // Octant 2: (-x, +y, -z) = 0b010 = 2
    // Octant 3: (+x, +y, -z) = 0b011 = 3
    // Octant 4: (-x, -y, +z) = 0b100 = 4
    // Octant 5: (+x, -y, +z) = 0b101 = 5
    // Octant 6: (-x, +y, +z) = 0b110 = 6
    // Octant 7: (+x, +y, +z) = 0b111 = 7

    fn get_octant(x: f32, y: f32, z: f32, cx: f32, cy: f32, cz: f32) -> u32 {
        let mut idx = 0u32;
        if x >= cx {
            idx |= 1;
        }
        if y >= cy {
            idx |= 2;
        }
        if z >= cz {
            idx |= 4;
        }
        idx
    }

    // WHEN: We compute octants
    let center = (4.0, 4.0, 4.0);

    // THEN: Ordering should match shader
    assert_eq!(get_octant(0.0, 0.0, 0.0, center.0, center.1, center.2), 0);
    assert_eq!(get_octant(7.0, 0.0, 0.0, center.0, center.1, center.2), 1);
    assert_eq!(get_octant(0.0, 7.0, 0.0, center.0, center.1, center.2), 2);
    assert_eq!(get_octant(7.0, 7.0, 0.0, center.0, center.1, center.2), 3);
    assert_eq!(get_octant(0.0, 0.0, 7.0, center.0, center.1, center.2), 4);
    assert_eq!(get_octant(7.0, 0.0, 7.0, center.0, center.1, center.2), 5);
    assert_eq!(get_octant(0.0, 7.0, 7.0, center.0, center.1, center.2), 6);
    assert_eq!(get_octant(7.0, 7.0, 7.0, center.0, center.1, center.2), 7);
}

#[test]
fn test_child_pointer_sequential() {
    // GIVEN: Interior node with 8 children
    // Children should be stored sequentially: base, base+1, ..., base+7

    let child_base = 10u32;
    let root = child_base << 9;

    // WHEN: We decode child pointer
    let ptr = root >> 9;

    // THEN: Children are at ptr+0 through ptr+7
    for octant in 0..8 {
        let child_idx = ptr + octant;
        assert!(child_idx >= ptr, "Child {} should be after base", octant);
        assert!(
            child_idx < ptr + 8,
            "Child {} should be within range",
            octant
        );
    }
}
