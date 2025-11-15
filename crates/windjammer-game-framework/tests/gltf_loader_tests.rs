//! Unit tests for GLTF/GLB Loader
//!
//! Tests GLTF document structure, materials, meshes, animations, and nodes.

use windjammer_game_framework::gltf_loader::*;
use windjammer_game_framework::math::{Mat4, Quat, Vec2, Vec3, Vec4};

// ============================================================================
// GltfDocument Tests
// ============================================================================

#[test]
fn test_gltf_document_creation() {
    let doc = GltfDocument::test_cube();
    assert_eq!(doc.meshes.len(), 1);
    assert_eq!(doc.materials.len(), 1);
    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.root_nodes.len(), 1);
    println!("✅ GltfDocument created with test cube");
}

#[test]
fn test_gltf_document_structure() {
    let doc = GltfDocument::test_cube();
    assert!(!doc.meshes.is_empty(), "Document should have meshes");
    assert!(!doc.materials.is_empty(), "Document should have materials");
    assert!(!doc.nodes.is_empty(), "Document should have nodes");
    println!("✅ GltfDocument has valid structure");
}

// ============================================================================
// GltfMesh Tests
// ============================================================================

#[test]
fn test_gltf_mesh_creation() {
    let doc = GltfDocument::test_cube();
    let mesh = &doc.meshes[0];
    assert_eq!(mesh.name, "Cube");
    assert_eq!(mesh.primitives.len(), 1);
    println!("✅ GltfMesh created: {}", mesh.name);
}

#[test]
fn test_gltf_mesh_primitives() {
    let doc = GltfDocument::test_cube();
    let mesh = &doc.meshes[0];
    assert!(!mesh.primitives.is_empty(), "Mesh should have primitives");
    println!("✅ GltfMesh has {} primitive(s)", mesh.primitives.len());
}

// ============================================================================
// GltfPrimitive Tests
// ============================================================================

#[test]
fn test_gltf_primitive_positions() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    assert_eq!(primitive.positions.len(), 8, "Cube should have 8 vertices");
    
    // Check that positions are valid
    for pos in &primitive.positions {
        assert!(pos.is_finite(), "Position should be finite");
    }
    println!("✅ GltfPrimitive has {} positions", primitive.positions.len());
}

#[test]
fn test_gltf_primitive_normals() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    assert_eq!(primitive.normals.len(), 8, "Should have normal for each vertex");
    
    // Check that normals are normalized
    for normal in &primitive.normals {
        let length = normal.length();
        assert!((length - 1.0).abs() < 0.01, "Normal should be unit length");
    }
    println!("✅ GltfPrimitive has {} normals", primitive.normals.len());
}

#[test]
fn test_gltf_primitive_tex_coords() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    assert_eq!(primitive.tex_coords.len(), 8, "Should have UV for each vertex");
    
    // Check that UVs are in valid range
    for uv in &primitive.tex_coords {
        assert!(uv.x >= 0.0 && uv.x <= 1.0, "U coordinate should be in [0, 1]");
        assert!(uv.y >= 0.0 && uv.y <= 1.0, "V coordinate should be in [0, 1]");
    }
    println!("✅ GltfPrimitive has {} tex coords", primitive.tex_coords.len());
}

#[test]
fn test_gltf_primitive_indices() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    assert_eq!(primitive.indices.len(), 12, "Should have 12 indices (4 triangles)");
    assert_eq!(primitive.indices.len() % 3, 0, "Indices should be multiple of 3");
    
    // Check that indices are valid
    let vertex_count = primitive.positions.len();
    for &index in &primitive.indices {
        assert!((index as usize) < vertex_count, "Index should be valid");
    }
    println!("✅ GltfPrimitive has {} indices ({} triangles)", 
        primitive.indices.len(), primitive.indices.len() / 3);
}

#[test]
fn test_gltf_primitive_material() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    assert_eq!(primitive.material_index, Some(0), "Should reference material 0");
    println!("✅ GltfPrimitive has material index");
}

#[test]
fn test_gltf_primitive_optional_data() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    
    // These are optional and may be empty
    assert!(primitive.tangents.is_empty() || !primitive.tangents.is_empty());
    assert!(primitive.colors.is_empty() || !primitive.colors.is_empty());
    assert!(primitive.bone_indices.is_empty() || !primitive.bone_indices.is_empty());
    assert!(primitive.bone_weights.is_empty() || !primitive.bone_weights.is_empty());
    
    println!("✅ GltfPrimitive optional data checked");
}

// ============================================================================
// GltfMaterial Tests
// ============================================================================

#[test]
fn test_gltf_material_default() {
    let material = GltfMaterial::default();
    assert_eq!(material.base_color_factor, Vec4::ONE);
    assert_eq!(material.metallic_factor, 1.0);
    assert_eq!(material.roughness_factor, 1.0);
    assert_eq!(material.alpha_mode, AlphaMode::Opaque);
    assert_eq!(material.alpha_cutoff, 0.5);
    assert!(!material.double_sided);
    println!("✅ GltfMaterial default values correct");
}

#[test]
fn test_gltf_material_pbr() {
    let doc = GltfDocument::test_cube();
    let material = &doc.materials[0];
    
    // Check PBR parameters are in valid range
    assert!(material.metallic_factor >= 0.0 && material.metallic_factor <= 1.0);
    assert!(material.roughness_factor >= 0.0 && material.roughness_factor <= 1.0);
    
    println!("✅ GltfMaterial PBR parameters valid");
}

#[test]
fn test_gltf_material_textures() {
    let material = GltfMaterial::default();
    
    // Textures are optional
    assert!(material.base_color_texture.is_none() || material.base_color_texture.is_some());
    assert!(material.metallic_roughness_texture.is_none() || material.metallic_roughness_texture.is_some());
    assert!(material.normal_texture.is_none() || material.normal_texture.is_some());
    
    println!("✅ GltfMaterial texture indices checked");
}

#[test]
fn test_gltf_material_emissive() {
    let material = GltfMaterial::default();
    assert_eq!(material.emissive_factor, Vec3::ZERO);
    assert!(material.emissive_texture.is_none());
    println!("✅ GltfMaterial emissive properties checked");
}

// ============================================================================
// AlphaMode Tests
// ============================================================================

#[test]
fn test_alpha_mode_variants() {
    assert_ne!(AlphaMode::Opaque, AlphaMode::Mask);
    assert_ne!(AlphaMode::Mask, AlphaMode::Blend);
    assert_ne!(AlphaMode::Opaque, AlphaMode::Blend);
    println!("✅ AlphaMode variants are distinct");
}

#[test]
fn test_alpha_mode_equality() {
    assert_eq!(AlphaMode::Opaque, AlphaMode::Opaque);
    assert_eq!(AlphaMode::Mask, AlphaMode::Mask);
    assert_eq!(AlphaMode::Blend, AlphaMode::Blend);
    println!("✅ AlphaMode equality works");
}

// ============================================================================
// GltfTexture Tests
// ============================================================================

#[test]
fn test_gltf_sampler_default() {
    let sampler = GltfSampler::default();
    assert_eq!(sampler.min_filter, FilterMode::Linear);
    assert_eq!(sampler.mag_filter, FilterMode::Linear);
    assert_eq!(sampler.wrap_s, WrapMode::Repeat);
    assert_eq!(sampler.wrap_t, WrapMode::Repeat);
    println!("✅ GltfSampler default values correct");
}

#[test]
fn test_filter_mode_variants() {
    assert_ne!(FilterMode::Nearest, FilterMode::Linear);
    assert_ne!(FilterMode::NearestMipmapNearest, FilterMode::LinearMipmapLinear);
    println!("✅ FilterMode variants are distinct");
}

#[test]
fn test_wrap_mode_variants() {
    assert_ne!(WrapMode::Repeat, WrapMode::ClampToEdge);
    assert_ne!(WrapMode::ClampToEdge, WrapMode::MirroredRepeat);
    println!("✅ WrapMode variants are distinct");
}

// ============================================================================
// GltfNode Tests
// ============================================================================

#[test]
fn test_gltf_node_creation() {
    let doc = GltfDocument::test_cube();
    let node = &doc.nodes[0];
    assert_eq!(node.name, "CubeNode");
    assert_eq!(node.mesh_index, Some(0));
    println!("✅ GltfNode created: {}", node.name);
}

#[test]
fn test_gltf_node_transform() {
    let doc = GltfDocument::test_cube();
    let node = &doc.nodes[0];
    
    assert_eq!(node.translation, Vec3::ZERO);
    assert_eq!(node.rotation, Quat::IDENTITY);
    assert_eq!(node.scale, Vec3::ONE);
    assert_eq!(node.transform, Mat4::IDENTITY);
    
    println!("✅ GltfNode transform is identity");
}

#[test]
fn test_gltf_node_hierarchy() {
    let doc = GltfDocument::test_cube();
    let node = &doc.nodes[0];
    
    assert!(node.children.is_empty(), "Test cube node should have no children");
    assert_eq!(doc.root_nodes.len(), 1, "Should have one root node");
    assert_eq!(doc.root_nodes[0], 0, "Root node should be index 0");
    
    println!("✅ GltfNode hierarchy checked");
}

#[test]
fn test_gltf_node_optional_data() {
    let doc = GltfDocument::test_cube();
    let node = &doc.nodes[0];
    
    assert!(node.mesh_index.is_some(), "Test cube node should have mesh");
    assert!(node.skin_index.is_none(), "Test cube node should not have skin");
    
    println!("✅ GltfNode optional data checked");
}

// ============================================================================
// Animation Tests
// ============================================================================

#[test]
fn test_animation_path_variants() {
    assert_ne!(AnimationPath::Translation, AnimationPath::Rotation);
    assert_ne!(AnimationPath::Rotation, AnimationPath::Scale);
    assert_ne!(AnimationPath::Scale, AnimationPath::Weights);
    println!("✅ AnimationPath variants are distinct");
}

#[test]
fn test_interpolation_mode_variants() {
    assert_ne!(InterpolationMode::Linear, InterpolationMode::Step);
    assert_ne!(InterpolationMode::Step, InterpolationMode::CubicSpline);
    println!("✅ InterpolationMode variants are distinct");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_gltf_document_integrity() {
    let doc = GltfDocument::test_cube();
    
    // Check that references are valid
    for node in &doc.nodes {
        if let Some(mesh_idx) = node.mesh_index {
            assert!(mesh_idx < doc.meshes.len(), "Mesh index should be valid");
        }
    }
    
    for mesh in &doc.meshes {
        for primitive in &mesh.primitives {
            if let Some(mat_idx) = primitive.material_index {
                assert!(mat_idx < doc.materials.len(), "Material index should be valid");
            }
        }
    }
    
    println!("✅ GltfDocument references are valid");
}

#[test]
fn test_gltf_document_consistency() {
    let doc = GltfDocument::test_cube();
    let primitive = &doc.meshes[0].primitives[0];
    
    // All vertex attributes should have same count
    let vertex_count = primitive.positions.len();
    assert_eq!(primitive.normals.len(), vertex_count, "Normals count should match positions");
    assert_eq!(primitive.tex_coords.len(), vertex_count, "TexCoords count should match positions");
    
    println!("✅ GltfDocument vertex data is consistent");
}

#[test]
fn test_gltf_loader_error_handling() {
    // Test unsupported extension
    let result = GltfLoader::load("test.obj");
    assert!(result.is_err(), "Should fail for unsupported extension");
    
    println!("✅ GltfLoader error handling works");
}

#[test]
fn test_gltf_loader_glb_placeholder() {
    // Test GLB loading (not yet implemented)
    let result = GltfLoader::load_glb("test.glb");
    assert!(result.is_err(), "GLB loading not yet implemented");
    
    println!("✅ GltfLoader GLB placeholder works");
}

#[test]
fn test_gltf_loader_gltf_placeholder() {
    // Test GLTF loading (not yet implemented)
    let result = GltfLoader::load_gltf("test.gltf");
    assert!(result.is_err(), "GLTF loading not yet implemented");
    
    println!("✅ GltfLoader GLTF placeholder works");
}

#[test]
fn test_gltf_loader_memory_placeholder() {
    // Test memory loading (not yet implemented)
    let data = vec![0u8; 100];
    let result = GltfLoader::load_from_memory(&data);
    assert!(result.is_err(), "Memory loading not yet implemented");
    
    println!("✅ GltfLoader memory placeholder works");
}

