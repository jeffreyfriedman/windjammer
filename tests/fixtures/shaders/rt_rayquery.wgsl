// Hardware Ray Tracing - Ray Query (Inline RT in Compute)
// Requires wgpu 27+ with EXPERIMENTAL_RAY_QUERY, enable wgpu_ray_query
//
// Uses ray queries in compute shader instead of full ray tracing pipeline.
// One ray per pixel, trace against TLAS, write color to output.

enable wgpu_ray_query;

struct CameraParams {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _pad0: f32,
    screen_size: vec2<f32>,
    _pad1: vec2<f32>,
}

struct RayDesc {
    flags: u32,
    cull_mask: u32,
    t_min: f32,
    t_max: f32,
    origin: vec3<f32>,
    dir: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraParams;
@group(0) @binding(1) var acceleration_structure: acceleration_structure;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;

var<private> rq: ray_query;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = u32(camera.screen_size.x);
    let height = u32(camera.screen_size.y);
    if (id.x >= width || id.y >= height) {
        return;
    }

    // NDC from pixel
    let uv = (vec2<f32>(id.xy) + 0.5) / camera.screen_size;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

    // Ray from camera through pixel
    let clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    let world_pos = camera.inv_view_proj * clip_pos;
    let world_pos3 = world_pos.xyz / world_pos.w;

    var ray_desc: RayDesc;
    ray_desc.origin = camera.position;
    ray_desc.dir = normalize(world_pos3 - camera.position);
    ray_desc.t_min = 0.001;
    ray_desc.t_max = 1000.0;
    ray_desc.cull_mask = 0xFFu;
    ray_desc.flags = 0u;

    rayQueryInitialize(&rq, acceleration_structure, ray_desc);
    while (rayQueryProceed(&rq)) {
        // Candidate hit - for opaque geometry we don't need to confirm
    }

    var color = vec3<f32>(0.1, 0.2, 0.4); // Sky color (miss)
    let hit = rayQueryGetCommittedIntersection(&rq);
    if (hit.kind == 1u) { // RAY_QUERY_INTERSECTION_TRIANGLE
        // Hit - simple diffuse shading using transformed normal
        let N = vec3<f32>(hit.object_to_world[0].z, hit.object_to_world[1].z, hit.object_to_world[2].z);
        let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
        let diffuse = max(dot(N, light_dir), 0.0);
        color = vec3<f32>(0.8, 0.6, 0.4) * (0.3 + 0.7 * diffuse);
    }

    let idx = id.y * width + id.x;
    output[idx] = vec4<f32>(color, 1.0);
}
