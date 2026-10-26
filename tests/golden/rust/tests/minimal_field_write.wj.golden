#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Point {
    x: f32,
    y: f32,
}










impl Point {
#[inline]
fn set_x(&mut self, value: f32) {
        self.x = value;
}
#[inline]
fn get_x(&self) -> f32 {
        self.x
}
}

fn main() {
    println!("Testing field write detection...");
    let mut p = Point { x: 1.0_f32, y: 2.0_f32 };
    p.set_x(10.0_f32);
    println!("x = {}", p.x);
    assert!(p.x == 10.0_f32, "set_x should modify x");
    println!("✅ Test passed!");
}

