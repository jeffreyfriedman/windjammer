use crate::math::Vec3;
#[derive(Debug, Clone)]

struct Triangle {
    pub v0: Vec3,
}

impl Triangle {
#[inline]
pub fn center(&self) -> Vec3 {
        Vec3::new(1.0_f64, 2.0_f64, 3.0_f64)
}
}

fn main() {
    let tri = Triangle { v0: Vec3::new(0.0_f64, 0.0_f64, 0.0_f64) };
    let _center = tri.center();
    println!("✅ Imported type qualified calls work!");
}

