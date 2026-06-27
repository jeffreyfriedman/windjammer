// PBR Mesh Rendering - Vertex + Fragment Shaders
//
// Renders triangle meshes with Cook-Torrance PBR shading.
// Supports textured and untextured materials via @group(2) material textures.
// Outputs to render targets for compositing with the voxel pipeline.

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
    _pad0: f32,
    _pad1: f32,
}

struct LightUniforms {
    sun_dir: vec3<f32>,
    _pad0: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
    sky_color: vec3<f32>,
    _pad1: f32,
    ground_color: vec3<f32>,
    ambient_intensity: f32,
}

struct ShadowCascadeUniforms {
    light_view_proj_0: mat4x4<f32>,
    light_view_proj_1: mat4x4<f32>,
    light_view_proj_2: mat4x4<f32>,
    light_view_proj_3: mat4x4<f32>,
    cascade_splits: vec4<f32>,
    shadow_map_size: f32,
    shadow_bias: f32,
    pcf_radius: f32,
    _pad: f32,
}

// Group 0: Per-frame uniforms
@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> model: ModelUniforms;
@group(0) @binding(2) var<uniform> material: MaterialUniforms;
@group(0) @binding(3) var<uniform> light: LightUniforms;

// Group 1: Shadow maps (wired in Phase 6)
@group(1) @binding(0) var<uniform> shadow_cascades: ShadowCascadeUniforms;
@group(1) @binding(1) var shadow_map_0: texture_depth_2d;
@group(1) @binding(2) var shadow_map_1: texture_depth_2d;
@group(1) @binding(3) var shadow_map_2: texture_depth_2d;
@group(1) @binding(4) var shadow_map_3: texture_depth_2d;
@group(1) @binding(5) var shadow_sampler: sampler_comparison;

// Group 2: Material textures
@group(2) @binding(0) var t_albedo: texture_2d<f32>;
@group(2) @binding(1) var t_normal: texture_2d<f32>;
@group(2) @binding(2) var t_metallic_roughness: texture_2d<f32>;
@group(2) @binding(3) var s_material: sampler;

struct VertexInputLegacy {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) tangent: vec4<f32>,
}

/// Per-instance model matrix columns (column-major mat4).
struct VertexInputInstanced {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) inst_m0: vec4<f32>,
    @location(6) inst_m1: vec4<f32>,
    @location(7) inst_m2: vec4<f32>,
    @location(8) inst_m3: vec4<f32>,
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

fn vertex_output_common(
    model_mat: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
    position: vec3<f32>,
    normal: vec3<f32>,
    color: vec4<f32>,
    texcoord: vec2<f32>,
    tangent: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model_mat * vec4<f32>(position, 1.0);
    out.world_position = world_pos.xyz;
    out.clip_position = camera.proj_matrix * camera.view_matrix * world_pos;
    out.world_normal = normalize((normal_mat * vec4<f32>(normal, 0.0)).xyz);
    out.vertex_color = color;
    out.texcoord = texcoord;
    out.world_tangent = normalize((model_mat * vec4<f32>(tangent.xyz, 0.0)).xyz);
    out.tangent_w = tangent.w;
    return out;
}

@vertex
fn vs_main_legacy(in: VertexInputLegacy) -> VertexOutput {
    return vertex_output_common(
        model.model_matrix,
        model.normal_matrix,
        in.position,
        in.normal,
        in.color,
        in.texcoord,
        in.tangent,
    );
}

@vertex
fn vs_main_instanced(in: VertexInputInstanced) -> VertexOutput {
    let model_mat = mat4x4<f32>(in.inst_m0, in.inst_m1, in.inst_m2, in.inst_m3);
    let n3 = mat3x3<f32>(model_mat[0].xyz, model_mat[1].xyz, model_mat[2].xyz);
    let inv_n = inverse(n3);
    let normal_mat3 = transpose(inv_n);
    let world_n = normalize(normal_mat3 * in.normal);
    var out: VertexOutput;
    let world_pos = model_mat * vec4<f32>(in.position, 1.0);
    out.world_position = world_pos.xyz;
    out.clip_position = camera.proj_matrix * camera.view_matrix * world_pos;
    out.world_normal = world_n;
    out.vertex_color = in.color;
    out.texcoord = in.texcoord;
    out.world_tangent = normalize((model_mat * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    out.tangent_w = in.tangent.w;
    return out;
}

// PBR functions

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    let t = 1.0 - cos_theta;
    let t2 = t * t;
    let t5 = t2 * t2 * t;
    return f0 + (vec3<f32>(1.0) - f0) * t5;
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * d * d + 0.0001);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = r * r / 8.0;
    let g1 = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let g2 = n_dot_l / (n_dot_l * (1.0 - k) + k);
    return g1 * g2;
}

fn cook_torrance(
    N: vec3<f32>, V: vec3<f32>, L: vec3<f32>,
    albedo: vec3<f32>, roughness: f32, metallic: f32,
) -> vec3<f32> {
    let H = normalize(V + L);
    let n_dot_v = max(dot(N, V), 0.001);
    let n_dot_l = max(dot(N, L), 0.0);
    let n_dot_h = max(dot(N, H), 0.0);
    let v_dot_h = max(dot(V, H), 0.0);

    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let F = fresnel_schlick(v_dot_h, f0);
    let D = distribution_ggx(n_dot_h, roughness);
    let G = geometry_smith(n_dot_v, n_dot_l, roughness);

    let specular = (D * F * G) / (4.0 * n_dot_v * n_dot_l + 0.0001);
    let kd = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let diffuse = kd * albedo / 3.14159265;

    return (diffuse + specular) * n_dot_l;
}

fn hemisphere_ambient(normal: vec3<f32>, albedo: vec3<f32>) -> vec3<f32> {
    let sky_factor = normal.y * 0.5 + 0.5;
    let sky_ambient = mix(light.ground_color, light.sky_color, sky_factor);
    return albedo * sky_ambient * light.ambient_intensity;
}

fn sample_shadow_pcf(shadow_map: texture_depth_2d, light_space_pos: vec4<f32>, bias: f32) -> f32 {
    let proj = light_space_pos.xyz / light_space_pos.w;
    let uv = vec2<f32>(proj.x * 0.5 + 0.5, 1.0 - (proj.y * 0.5 + 0.5));
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return 1.0;
    }
    let depth = proj.z - bias;
    let texel_size = 1.0 / shadow_cascades.shadow_map_size;

    var shadow = 0.0;
    for (var x = -1i; x <= 1i; x++) {
        for (var y = -1i; y <= 1i; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, depth);
        }
    }
    return shadow / 9.0;
}

fn compute_shadow(world_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    let view_pos = camera.view_matrix * vec4<f32>(world_pos, 1.0);
    let view_depth = -view_pos.z;
    let bias = shadow_cascades.shadow_bias;

    if (view_depth < shadow_cascades.cascade_splits.x) {
        let lsp = shadow_cascades.light_view_proj_0 * vec4<f32>(world_pos, 1.0);
        return sample_shadow_pcf(shadow_map_0, lsp, bias);
    } else if (view_depth < shadow_cascades.cascade_splits.y) {
        let lsp = shadow_cascades.light_view_proj_1 * vec4<f32>(world_pos, 1.0);
        return sample_shadow_pcf(shadow_map_1, lsp, bias);
    } else if (view_depth < shadow_cascades.cascade_splits.z) {
        let lsp = shadow_cascades.light_view_proj_2 * vec4<f32>(world_pos, 1.0);
        return sample_shadow_pcf(shadow_map_2, lsp, bias);
    } else if (view_depth < shadow_cascades.cascade_splits.w) {
        let lsp = shadow_cascades.light_view_proj_3 * vec4<f32>(world_pos, 1.0);
        return sample_shadow_pcf(shadow_map_3, lsp, bias);
    }
    return 1.0;
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
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample albedo from texture or use uniform color
    var albedo: vec3<f32>;
    if (material.has_albedo_texture > 0.5) {
        let tex_color = textureSample(t_albedo, s_material, in.texcoord);
        albedo = tex_color.rgb * material.albedo.rgb * in.vertex_color.rgb;
    } else {
        albedo = material.albedo.rgb * in.vertex_color.rgb;
    }

    // Sample metallic/roughness from texture or use uniforms
    var roughness = material.roughness;
    var metallic = material.metallic;
    if (material.has_metallic_roughness_texture > 0.5) {
        let mr = textureSample(t_metallic_roughness, s_material, in.texcoord);
        roughness = mr.g * material.roughness;
        metallic = mr.b * material.metallic;
    }

    // Get normal from normal map or vertex normal
    var N: vec3<f32>;
    if (material.has_normal_texture > 0.5) {
        N = get_normal_from_map(in.texcoord, in.world_normal, in.world_tangent, in.tangent_w);
    } else {
        N = normalize(in.world_normal);
    }

    let V = normalize(camera.position - in.world_position);
    let L = normalize(-light.sun_dir);

    let shadow_factor = compute_shadow(in.world_position, N);

    let direct = cook_torrance(N, V, L, albedo, roughness, metallic)
                 * light.sun_color * light.sun_intensity
                 * shadow_factor;

    let ambient = hemisphere_ambient(N, albedo);

    let emissive = albedo * material.emission_strength;

    let color = direct + ambient + emissive;
    return vec4<f32>(color, material.albedo.a * in.vertex_color.a);
}

// Depth-only variant for shadow passes
@vertex
fn vs_depth_legacy(in: VertexInputLegacy) -> @builtin(position) vec4<f32> {
    let world_pos = model.model_matrix * vec4<f32>(in.position, 1.0);
    return camera.proj_matrix * camera.view_matrix * world_pos;
}

@vertex
fn vs_depth_instanced(in: VertexInputInstanced) -> @builtin(position) vec4<f32> {
    let model_mat = mat4x4<f32>(in.inst_m0, in.inst_m1, in.inst_m2, in.inst_m3);
    let world_pos = model_mat * vec4<f32>(in.position, 1.0);
    return camera.proj_matrix * camera.view_matrix * world_pos;
}
