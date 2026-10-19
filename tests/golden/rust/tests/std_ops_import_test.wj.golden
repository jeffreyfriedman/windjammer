use std::ops::Add;
use std::ops::Sub;
use std::ops::Mul;
use std::ops::Div;
#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Vec2 {
    x: f32,
    y: f32,
}










impl Add for Vec2 {
    type Output = Vec2;

#[inline]
fn add(self, other: Vec2) -> Vec2 {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
}
}

fn main() {
    let v1 = Vec2 { x: 1.0_f32, y: 2.0_f32 };
    let v2 = Vec2 { x: 3.0_f32, y: 4.0_f32 };
    let v3 = v1 + v2;
    println!("v3 = ({}, {})", v3.x, v3.y);
}

