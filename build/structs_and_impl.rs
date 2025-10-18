struct Point {
    x: i64,
    y: i64,
}

impl Point {
#[inline]
fn new(&self, x: i64, y: i64) -> Point {
        Point { x, y }
}
#[inline]
fn distance(&self) -> f64 {
        let dx = self.x as f64;
        let dy = self.y as f64(dx * dx + dy * dy).sqrt();
}
}

fn main() {
    let p = Point { x: 3, y: 4 };
    println!("Point: ({}, {})", p.x, p.y)
}

