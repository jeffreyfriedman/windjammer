use crate::math::Vec3;
use std::collections::HashMap;
#[derive(Debug, Clone)]

struct Triangle {
    pub id: u32,
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct OffMeshLink {
    pub start: Vec3,
    pub end: Vec3,
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct Navmesh {
    pub triangles: Vec<Triangle>,
    pub off_mesh_links: Vec<OffMeshLink>,
    pub agent_radius: f32,
}

impl Navmesh {
#[inline]
pub fn new() -> Navmesh {
        Navmesh { triangles: Vec::new(), off_mesh_links: Vec::new(), agent_radius: 0.5_f32 }
}
#[inline]
pub fn add_triangle(&mut self, v0: Vec3, v1: Vec3, v2: Vec3) -> u32 {
        let id = self.triangles.len() as u32;
        self.triangles.push(Triangle { id, v0, v1, v2 });
        id
}
}

fn main() {
    let mut _nav = Navmesh::new();
    println!("✅ Navmesh structs compile!");
}

