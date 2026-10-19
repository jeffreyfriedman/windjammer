use std::ops::Sub;
#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Vec2 {
    pub x: f32,
    pub y: f32,
}










impl Vec2 {
#[inline]
pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
}
}

impl Sub for Vec2 {
    type Output = Vec2;

#[inline]
fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new((self.x - other.x) as f32, (self.y - other.y) as f32)
}
}

#[inline]
pub fn distance(a: Vec2, b: Vec2) -> f32 {
    let diff = a - b;
    (diff.x * diff.x + diff.y * diff.y).sqrt()
}

#[inline]
pub fn midpoint(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(((a.x + b.x) / 2.0_f32) as f32, ((a.y + b.y) / 2.0_f32) as f32)
}

