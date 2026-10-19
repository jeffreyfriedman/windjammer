use std::ops::Add;
use std::ops::Mul;
#[derive(Debug, Clone, Copy, Eq, Hash, Default)]

struct V2 {
    x: i64,
    y: i64,
}










impl Add for V2 {
#[inline]
fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
}
}

impl Mul<i64> for V2 {
#[inline]
fn mul(self, k: i64) -> V2 {
        V2 { x: self.x * k, y: self.y * k }
}
}

impl PartialEq for V2 {
#[inline]
fn eq(&self, o: &V2) -> bool {
        self.x == o.x && self.y == o.y
}
}

#[inline]
fn show(v: V2) -> String {
    format!("({}, {})", v.x, v.y)
}

fn main() {
    let a = V2 { x: 1_i64, y: 2_i64 };
    let b = V2 { x: 3_i64, y: 4_i64 };
    println!("{}", show(a + b));
    println!("{}", show(V2 { x: 1_i64, y: 2_i64 } * 3));
    println!("{}", a == V2 { x: 1_i64, y: 2_i64 });
    println!("{}", a == b);
}

