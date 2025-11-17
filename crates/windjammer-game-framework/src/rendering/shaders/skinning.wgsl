// Skeletal Animation Skinning Shader
// GPU-accelerated vertex skinning for skeletal animations

// Bone matrices (up to 256 bones)
struct BoneMatrices {
    matrices: array<mat4x4<f32>, 256>,
}

@group(3) @binding(0)
var<uniform> bone_matrices: BoneMatrices;

// Skinning function - transforms vertex by bone influences
fn skin_vertex(
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec4<f32>,
    bone_indices: vec4<u32>,
    bone_weights: vec4<f32>,
) -> SkinnedVertex {
    // Normalize weights (should sum to 1.0)
    let total_weight = bone_weights.x + bone_weights.y + bone_weights.z + bone_weights.w;
    let normalized_weights = bone_weights / max(total_weight, 0.0001);
    
    // Calculate skinned position
    var skinned_pos = vec3<f32>(0.0, 0.0, 0.0);
    var skinned_normal = vec3<f32>(0.0, 0.0, 0.0);
    var skinned_tangent = vec3<f32>(0.0, 0.0, 0.0);
    
    // Apply each bone influence
    for (var i = 0u; i < 4u; i = i + 1u) {
        let bone_index = bone_indices[i];
        let weight = normalized_weights[i];
        
        if (weight > 0.0001) {
            let bone_matrix = bone_matrices.matrices[bone_index];
            
            // Transform position
            let transformed_pos = (bone_matrix * vec4<f32>(position, 1.0)).xyz;
            skinned_pos = skinned_pos + transformed_pos * weight;
            
            // Transform normal (use 3x3 part of matrix, ignore translation)
            let bone_matrix_3x3 = mat3x3<f32>(
                bone_matrix[0].xyz,
                bone_matrix[1].xyz,
                bone_matrix[2].xyz
            );
            let transformed_normal = bone_matrix_3x3 * normal;
            skinned_normal = skinned_normal + transformed_normal * weight;
            
            // Transform tangent
            let transformed_tangent = bone_matrix_3x3 * tangent.xyz;
            skinned_tangent = skinned_tangent + transformed_tangent * weight;
        }
    }
    
    // Normalize normal and tangent
    skinned_normal = normalize(skinned_normal);
    skinned_tangent = normalize(skinned_tangent);
    
    var result: SkinnedVertex;
    result.position = skinned_pos;
    result.normal = skinned_normal;
    result.tangent = vec4<f32>(skinned_tangent, tangent.w); // Preserve handedness
    
    return result;
}

// Result of skinning operation
struct SkinnedVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec4<f32>,
}

// Vertex input for skinned meshes
struct SkinnedVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) bone_indices: vec4<u32>,
    @location(5) bone_weights: vec4<f32>,
}

// Vertex output
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
}

// Camera uniform
struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

// Model transform
struct Model {
    transform: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> model: Model;

// Vertex shader for skinned meshes
@vertex
fn vs_main(in: SkinnedVertexInput) -> VertexOutput {
    // Apply skinning
    let skinned = skin_vertex(
        in.position,
        in.normal,
        in.tangent,
        in.bone_indices,
        in.bone_weights
    );
    
    // Transform to world space
    let world_position = (model.transform * vec4<f32>(skinned.position, 1.0)).xyz;
    
    // Transform normal and tangent to world space
    let normal_matrix_3x3 = mat3x3<f32>(
        model.normal_matrix[0].xyz,
        model.normal_matrix[1].xyz,
        model.normal_matrix[2].xyz
    );
    let world_normal = normalize(normal_matrix_3x3 * skinned.normal);
    let world_tangent_xyz = normalize(normal_matrix_3x3 * skinned.tangent.xyz);
    let world_tangent = vec4<f32>(world_tangent_xyz, skinned.tangent.w);
    
    // Output
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;
    out.world_tangent = world_tangent;
    out.uv = in.uv;
    
    return out;
}

// Fragment shader (same as PBR)
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // This would use the same PBR fragment shader
    // For now, just return a simple color based on normal
    let color = (in.world_normal + vec3<f32>(1.0)) * 0.5;
    return vec4<f32>(color, 1.0);
}

