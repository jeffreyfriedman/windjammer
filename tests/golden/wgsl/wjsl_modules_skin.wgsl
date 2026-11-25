// from brdf.wjsl:7
struct brdf__Lobe {
    weight: f32,
}

@group(0) @binding(0) var<uniform> lobe: brdf__Lobe;
// from brdf.wjsl:1
fn brdf__pow5(x: f32) -> f32 {
    return x * x * x * x * x;
}

// from brdf.wjsl:11
fn brdf__fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * brdf__pow5(1.0 - cos_theta);
}

// from brdf.wjsl:15
fn brdf__lobe_weight(l: brdf__Lobe) -> f32 {
    return l.weight;
}

@fragment
fn fs_skin(@location(0) n: vec3<f32>, @location(1) v: vec3<f32>) -> @location(0) vec4<f32> {
    let f = brdf__fresnel_schlick(clamp(dot(n, v), 0.0, 1.0), 0.028);
        return vec4<f32>(vec3<f32>(f * brdf__lobe_weight(lobe)), 1.0);
}

