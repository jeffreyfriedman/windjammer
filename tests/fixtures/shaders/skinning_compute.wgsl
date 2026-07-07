// GPU Skinning Compute Shader
// Transforms vertices by weighted bone matrices.
// Input: unskinned vertex buffer + bone index/weight buffer + bone matrices
// Output: skinned vertex buffer (standard GpuVertex layout)

struct SkinVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    texcoord: vec2<f32>,
    bone_indices: vec4<u32>,
    bone_weights: vec4<f32>,
}

struct GpuVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    color: vec4<f32>,
    texcoord: vec2<f32>,
    tangent: vec4<f32>,
}

struct SkinningUniforms {
    vertex_count: u32,
    bone_count: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<uniform> uniforms: SkinningUniforms;
@group(0) @binding(1) var<storage, read> bone_matrices: array<mat4x4<f32>>;

// Input vertex data as raw f32/u32 arrays (position + normal + texcoord + bone_indices + bone_weights)
// Layout per vertex: 3f pos, 3f normal, 2f texcoord, 4u bone_indices, 4f bone_weights = 16 elements
@group(0) @binding(2) var<storage, read> input_positions: array<f32>;
@group(0) @binding(3) var<storage, read> input_normals: array<f32>;
@group(0) @binding(4) var<storage, read> input_texcoords: array<f32>;
@group(0) @binding(5) var<storage, read> input_bone_indices: array<u32>;
@group(0) @binding(6) var<storage, read> input_bone_weights: array<f32>;

// Output: flat f32 array matching GpuVertex layout (16 floats = 64 bytes per vertex)
// pos(3) + normal(3) + color(4) + texcoord(2) + tangent(4) = 16
@group(0) @binding(7) var<storage, read_write> output_vertices: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let vertex_idx = id.x;
    if vertex_idx >= uniforms.vertex_count {
        return;
    }

    let vi = vertex_idx;
    let pos = vec3<f32>(
        input_positions[vi * 3u + 0u],
        input_positions[vi * 3u + 1u],
        input_positions[vi * 3u + 2u],
    );
    let norm = vec3<f32>(
        input_normals[vi * 3u + 0u],
        input_normals[vi * 3u + 1u],
        input_normals[vi * 3u + 2u],
    );

    let texcoord = vec2<f32>(
        input_texcoords[vi * 2u + 0u],
        input_texcoords[vi * 2u + 1u],
    );

    let bi = vec4<u32>(
        input_bone_indices[vi * 4u + 0u],
        input_bone_indices[vi * 4u + 1u],
        input_bone_indices[vi * 4u + 2u],
        input_bone_indices[vi * 4u + 3u],
    );
    let bw = vec4<f32>(
        input_bone_weights[vi * 4u + 0u],
        input_bone_weights[vi * 4u + 1u],
        input_bone_weights[vi * 4u + 2u],
        input_bone_weights[vi * 4u + 3u],
    );

    // Weighted bone matrix blend
    var skin_matrix = mat4x4<f32>(
        vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0)
    );
    let total_weight = bw.x + bw.y + bw.z + bw.w;
    if total_weight > 0.001 {
        if bw.x > 0.0 { skin_matrix += bone_matrices[bi.x] * bw.x; }
        if bw.y > 0.0 { skin_matrix += bone_matrices[bi.y] * bw.y; }
        if bw.z > 0.0 { skin_matrix += bone_matrices[bi.z] * bw.z; }
        if bw.w > 0.0 { skin_matrix += bone_matrices[bi.w] * bw.w; }
    } else {
        skin_matrix = mat4x4<f32>(
            vec4<f32>(1.0, 0.0, 0.0, 0.0),
            vec4<f32>(0.0, 1.0, 0.0, 0.0),
            vec4<f32>(0.0, 0.0, 1.0, 0.0),
            vec4<f32>(0.0, 0.0, 0.0, 1.0),
        );
    }

    // Transform position and normal
    let skinned_pos = (skin_matrix * vec4<f32>(pos, 1.0)).xyz;
    let normal_matrix = mat3x3<f32>(
        skin_matrix[0].xyz,
        skin_matrix[1].xyz,
        skin_matrix[2].xyz,
    );
    let skinned_normal = normalize(normal_matrix * norm);

    // Write output GpuVertex (16 floats, 64 bytes)
    let out_base = vertex_idx * 16u;
    output_vertices[out_base + 0u]  = skinned_pos.x;
    output_vertices[out_base + 1u]  = skinned_pos.y;
    output_vertices[out_base + 2u]  = skinned_pos.z;
    output_vertices[out_base + 3u]  = skinned_normal.x;
    output_vertices[out_base + 4u]  = skinned_normal.y;
    output_vertices[out_base + 5u]  = skinned_normal.z;
    output_vertices[out_base + 6u]  = 1.0; // color.r
    output_vertices[out_base + 7u]  = 1.0; // color.g
    output_vertices[out_base + 8u]  = 1.0; // color.b
    output_vertices[out_base + 9u]  = 1.0; // color.a
    output_vertices[out_base + 10u] = texcoord.x;
    output_vertices[out_base + 11u] = texcoord.y;
    output_vertices[out_base + 12u] = 1.0; // tangent.x
    output_vertices[out_base + 13u] = 0.0; // tangent.y
    output_vertices[out_base + 14u] = 0.0; // tangent.z
    output_vertices[out_base + 15u] = 1.0; // tangent.w
}
