#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Triangle {
    pub id: u32,
}









impl Triangle {
#[inline]
pub fn new(id: u32) -> Triangle {
        Triangle { id }
}
}

#[derive(Debug, Clone, PartialEq, Default)]

struct Navmesh {
    pub triangles: Vec<Triangle>,
}

impl Navmesh {
#[inline]
pub fn new() -> Navmesh {
        Navmesh { triangles: Vec::new() }
}
#[inline]
pub fn add_triangle(&mut self, id: u32) -> u32 {
        self.triangles.push(Triangle::new(id));
        id
}
}

fn main() {
    let mut nav = Navmesh::new();
    nav.add_triangle(1_u32);
    println!("✅ Qualified paths in method calls work!");
}

