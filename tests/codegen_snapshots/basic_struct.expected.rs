#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
#[inline]
pub fn new(x: f32, y: f32) -> Point {
        Point { x, y }
}
#[inline]
pub fn distance_from_origin(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
}
}
