#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Point {
    x: f32,
    y: f32,
}










impl Point {
#[inline]
fn double(self) -> Point {
        Point { x: self.x * 2.0_f32, y: self.y * 2.0_f32 }
}
}

fn main() {
    let p = Point { x: 1.0_f32, y: 2.0_f32 };
    let p2 = p.double();
    assert!(p2.x == 2.0_f32, "x should be doubled");
    assert!(p2.y == 4.0_f32, "y should be doubled");
    println!("✅ Method with self by value works");
}

