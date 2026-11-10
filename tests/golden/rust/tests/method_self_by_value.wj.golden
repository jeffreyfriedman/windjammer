#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Mat4 {
    m00: f32,
}









impl Mat4 {
#[inline]
fn new(value: f32) -> Mat4 {
        Mat4 { m00: value }
}
#[inline]
fn multiply(self, other: Mat4) -> Mat4 {
        Mat4 { m00: self.m00 * other.m00 }
}
}

fn main() {
    let identity = Mat4::new(1.0_f32);
    let other = Mat4::new(2.0_f32);
    let result = identity.multiply(other);
    assert!(result.m00 == 2.0_f32, "Should multiply");
    println!("✅ Method with self by value works correctly");
}

