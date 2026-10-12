use windjammer_runtime::test::*;
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes
    }
}


impl Point {
#[inline]
pub fn new(x: f32, y: f32) -> Point {
        Point { x, y }
}
#[inline]
pub fn from_coords(x: f32, y: f32) -> Point {
        Point { x, y }
}
#[inline]
pub fn zero() -> Point {
        Point { x: 0.0_f32, y: 0.0_f32 }
}
#[inline]
pub fn distance(&self, other: Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
}
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
    pub data: Vec<i32>,
}

impl Grid {
#[inline]
pub fn new(width: i32, height: i32) -> Grid {
        let mut data: Vec<i32> = Vec::new();
        for _i in 0_i32..width * height {
            data.push(0_i32);
        }
        Grid { width, height, data }
}
}

#[test]

pub fn test_static_methods() {
    let p1 = Point::new(1.0_f32, 2.0_f32);
    let p2 = Point::from_coords(3.0_f32, 4.0_f32);
    let _p3 = Point::zero();
    let _dist = p1.distance(p2);
    let _grid = Grid::new(10_i32, 10_i32);
}

