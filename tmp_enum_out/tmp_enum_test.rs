#[derive(Clone, Debug, PartialEq)]
enum Light {
    Point { color: String, intensity: f32, range: f32 },
    Directional { color: String, intensity: f32 },
}

#[inline]
fn get_intensity(light: Light) -> f32 {
    match light {
        Light::Point { intensity, .. } => intensity,
        Light::Directional { intensity, .. } => intensity,
    }
}

fn main() {
    let p = Light::Point { color: String::from("red"), intensity: 1.5_f32, range: 10.0_f32 };
    println!("{:.1}", get_intensity(*(&p)));
}

