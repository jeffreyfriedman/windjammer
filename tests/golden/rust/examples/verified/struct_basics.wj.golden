#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Point {
    x: i64,
    y: i64,
}










impl Point {
#[inline]
fn new(x: i64, y: i64) -> Point {
        Point { x, y }
}
#[inline]
fn dist2(&self) -> i64 {
        self.x * self.x + self.y * self.y
}
#[inline]
fn translate(&mut self, dx: i64, dy: i64) {
        self.x += dx;
        self.y += dy;
}
#[inline]
fn show(&self) -> String {
        format!("Point({}, {})", self.x, self.y)
}
}

fn main() {
    let mut p = Point::new(3_i64, 4_i64);
    println!("{}", p.show());
    println!("{}", p.dist2());
    p.translate(1_i64, 2_i64);
    println!("{}", p.show());
}

