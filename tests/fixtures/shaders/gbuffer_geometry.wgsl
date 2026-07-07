// Deferred G-Buffer Geometry Pass
//
// Writes material properties to multiple render targets (MRT):
// RT0: Albedo.rgb + Metallic (RGBA16Float)
// RT1: Normal.xy (octahedral) + Roughness + MaterialFlags (RGBA16Float)
// RT2: Emission.rgb + AO (RGBA16Float)
// Depth: Depth32Float

struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    proj_matrix: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    position: vec3<f32>,
    _pad0: f32,
    screen_size: vec2<f32>,
    near_plane: f32,
    far_plane: f32,
}

struct ModelUniforms {
    model_matrix: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

struct MaterialUniforms {
    albedo: vec4<f32>,
    roughness: f32,
    metallic: f32,
    emission_strength: f32,
    has_albedo_texture: f32,
    has_normal_texture: f32,
    has_metallic_roughness_texture: f32,
    ao_strength: f32,
    material_flags: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> model: ModelUniforms;
@group(0) @binding(2) var<uniform> material: MaterialUniforms;

@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var t_normal: texture_2d<f32>;
@group(1) @binding(2) var t_metallic_roughness: texture_2d<f32>;
@group(1) @binding(3) var s_material: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) tangent: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) world_tangent: vec3<f32>,
    @location(5) tangent_w: f32,
}

struct GBufferOutput {
    @location(0) albedo_metallic: vec4<f32>,
    @location(1) normal_roughness: vec4<f32>,
    @location(2) emission_ao: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.model_matrix * vec4<f32>(in.position, 1.0);
    out.world_position = world_pos.xyz;
    out.clip_position = camera.proj_matrix * camera.view_matrix * world_pos;
    out.world_normal = normalize((model.normal_matrix * vec4<f32>(in.normal, 0.0)).xyz);
    out.vertex_color = in.color;
    out.texcoord = in.texcoord;
    out.world_tangent = normalize((model.model_matrix * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    out.tangent_w = in.tangent.w;
    return out;
}

// Octahedral normal encoding ([-1,1]^3 -> [0,1]^2)
fn octahedral_encode(n: vec3<f32>) -> vec2<f32> {
    var p = n.xy * (1.0 / (abs(n.x) + abs(n.y) + abs(n.z)));
    if (n.z < 0.0) {
        let signs = vec2<f32>(
            select(-1.0, 1.0, p.x >= 0.0),
            select(-1.0, 1.0, p.y >= 0.0)
        );
        p = (1.0 - abs(p.yx)) * signs;
    }
    return p * 0.5 + 0.5;
}

fn get_normal_from_map(texcoord: vec2<f32>, world_normal: vec3<f32>, world_tangent: vec3<f32>, tangent_w: f32) -> vec3<f32> {
    let tangent_normal = textureSample(t_normal, s_material, texcoord).xyz * 2.0 - 1.0;
    let N = normalize(world_normal);
    let T = normalize(world_tangent - dot(world_tangent, N) * N);
    let B = cross(N, T) * tangent_w;
    let TBN = mat3x3<f32>(T, B, N);
    return normalize(TBN * tangent_normal);
}

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    var out: GBufferOutput;

    // Albedo
    var albedo: vec3<f32>;
    if (material.has_albedo_texture > 0.5) {
        let tex_color = textureSample(t_albedo, s_material, in.texcoord);
        albedo = tex_color.rgb * material.albedo.rgb * in.vertex_color.rgb;
    } else {
        albedo = material.albedo.rgb * in.vertex_color.rgb;
    }

    // Metallic + Roughness
    var roughness = material.roughness;
    var metallic = material.metallic;
    if (material.has_metallic_roughness_texture > 0.5) {
        let mr = textureSample(t_metallic_roughness, s_material, in.texcoord);
        roughness = mr.g * material.roughness;
        metallic = mr.b * material.metallic;
    }

    // Normal
    var N: vec3<f32>;
    if (material.has_normal_texture > 0.5) {
        N = get_normal_from_map(in.texcoord, in.world_normal, in.world_tangent, in.tangent_w);
    } else {
        N = normalize(in.world_normal);
    }

    // Emission
    let emission = albedo * material.emission_strength;

    // AO (default 1.0 = no occlusion)
    let ao = material.ao_strength;

    // Pack into G-Buffer
    out.albedo_metallic = vec4<f32>(albedo, metallic);
    out.normal_roughness = vec4<f32>(octahedral_encode(N), roughness, material.material_flags);
    out.emission_ao = vec4<f32>(emission, ao);

    return out;
}
