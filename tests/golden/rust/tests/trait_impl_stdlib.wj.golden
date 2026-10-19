use std::ops::Add;
use std::ops::Sub;
#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Point {
    pub x: f32,
    pub y: f32,
}










impl Add for Point {
    type Output = Point;

#[inline]
fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
}
}

impl Sub for Point {
    type Output = Point;

#[inline]
fn sub(self, other: Point) -> Point {
        Point { x: self.x - other.x, y: self.y - other.y }
}
}

