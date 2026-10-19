use crate::math::Vec3;
use std::collections::HashMap;
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct NavmeshPath {
    pub waypoints: Vec<Vec3>,
}

impl NavmeshPath {
#[inline]
pub fn new() -> NavmeshPath {
        NavmeshPath { waypoints: Vec::new() }
}
#[inline]
pub fn len(&self) -> i32 {
        self.waypoints.len() as i32
}
}

fn main() {
    let path = NavmeshPath::new();
    let count = path.len();
    println!("Path has {} waypoints", count);
    println!("✅ Navmesh imports work!");
}

