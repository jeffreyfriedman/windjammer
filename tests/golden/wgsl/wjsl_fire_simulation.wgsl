struct FireParams {
    source_position: vec3<f32>,
    _pad0: f32,
    source_radius: f32,
    max_height: f32,
    intensity: f32,
    time: f32,
    wind_x: f32,
    wind_z: f32,
    turbulence: f32,
    cooling_rate: f32,
}

@group(0) @binding(0) var<uniform> fire_params: FireParams;
@group(0) @binding(1) var<storage, read_write> temperature_grid: array<f32>;
fn hash31(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
        let r = dot(q, vec3(12.9898, 78.233, 45.54321));
        return fract(sin(r) * 43758.5453);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var q = vec3(
            dot(p, vec3(127.1, 311.7, 74.7)),
            dot(p, vec3(269.5, 183.3, 246.1)),
            dot(p, vec3(113.5, 271.9, 124.6))
        );
        return fract(sin(q) * 43758.5453123);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
        let f = fract(p);
        let u = f * f * (3.0 - 2.0 * f);
    
        let n000 = hash31(i + vec3(0.0, 0.0, 0.0));
        let n100 = hash31(i + vec3(1.0, 0.0, 0.0));
        let n010 = hash31(i + vec3(0.0, 1.0, 0.0));
        let n110 = hash31(i + vec3(1.0, 1.0, 0.0));
        let n001 = hash31(i + vec3(0.0, 0.0, 1.0));
        let n101 = hash31(i + vec3(1.0, 0.0, 1.0));
        let n011 = hash31(i + vec3(0.0, 1.0, 1.0));
        let n111 = hash31(i + vec3(1.0, 1.0, 1.0));
    
        let x0 = mix(n000, n100, u.x);
        let x1 = mix(n010, n110, u.x);
        let x2 = mix(n001, n101, u.x);
        let x3 = mix(n011, n111, u.x);
        let y0 = mix(x0, x1, u.y);
        let y1 = mix(x2, x3, u.y);
        return mix(y0, y1, u.z);
}

fn fbm(p: vec3<f32>, octaves: i32) -> f32 {
    var value = 0.0;
        var amplitude = 0.5;
        var frequency = 1.0;
        var pos = p;
        for (var i = 0; i < octaves; i = i + 1) {
            value = value + amplitude * noise3d(pos * frequency);
            amplitude = amplitude * 0.5;
            frequency = frequency * 2.0;
        }
        return value;
}

fn grid_index(x: u32, y: u32, z: u32) -> u32 {
    return z * 64u * 64u + y * 64u + x;
}

fn is_in_bounds(x: u32, y: u32, z: u32) -> bool {
    return x < 64u && y < 64u && z < 64u;
}

fn heat_source(world_pos: vec3<f32>) -> f32 {
    let rel = world_pos - fire_params.source_position;
        let horizontal_dist = length(vec2(rel.x, rel.z));
        let height = rel.y;
    
        if (horizontal_dist > fire_params.source_radius || height > 1.0 || height < -0.5) {
            return 0.0;
        }
    
        let radial = 1.0 - smoothstep(0.0, fire_params.source_radius, horizontal_dist);
        let vertical = 1.0 - smoothstep(-0.2, 1.0, height);
        let noise_val = noise3d(world_pos * 3.0 + vec3(fire_params.time * 2.0, 0.0, 0.0));
        let flicker = 0.7 + 0.3 * noise_val;
    
        return radial * vertical * fire_params.intensity * flicker;
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (!is_in_bounds(id.x, id.y, id.z)) { return; }
    
        let idx = grid_index(id.x, id.y, id.z);
        let cell_size = fire_params.max_height / 64.0;
        let offset_x = (f32(id.x) - 32.0) * cell_size;
        let offset_y = f32(id.y) * cell_size;
        let offset_z = (f32(id.z) - 32.0) * cell_size;
        let world_pos = fire_params.source_position + vec3(offset_x, offset_y, offset_z);
    
        var temp = temperature_grid[idx];
    
        // Upward advection: sample from below
        if (id.y > 0u) {
            let below_idx = grid_index(id.x, id.y - 1u, id.z);
            let advection = temperature_grid[below_idx] * 0.3;
            temp = temp + advection;
        }
    
        // Wind advection
        let wind = vec2(fire_params.wind_x, fire_params.wind_z);
        let wind_offset = wind * fire_params.time;
        let noise_pos = world_pos + vec3(wind_offset.x, 0.0, wind_offset.y);
        let turb = fbm(noise_pos * 2.0 + vec3(fire_params.time, 0.0, 0.0), 4);
        temp = temp + turb * fire_params.turbulence * temp;
    
        // Cooling: temperature decreases with height and over time
        let height_factor = f32(id.y) / 64.0;
        let cooling = fire_params.cooling_rate * (1.0 + height_factor * 2.0);
        temp = max(temp - cooling * 0.016, 0.0);
    
        // Heat source injection
        temp = temp + heat_source(world_pos);
    
        // Clamp
        temp = clamp(temp, 0.0, 5.0);
    
        temperature_grid[idx] = temp;
}

