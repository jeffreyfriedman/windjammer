use crate::math::Vec3;
#[derive(Debug, Clone, PartialEq, Default)]

struct Path {
    waypoints: Vec<Vec3>,
}

#[derive(Debug, Clone, PartialEq, Default)]

struct Matrix {
    data: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Default)]

struct Complex {
    vectors: Vec<Vec3>,
    matrices: Vec<Mat4>,
    items: Vec<Item>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}
impl Vec3 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(12);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes.extend_from_slice(&self.z.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, PartialEq)]

struct Mat4 {
    m: [f32; 16],
}
impl Mat4 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(64);
        for __el in &self.m {
            __bytes.extend_from_slice(&__el.to_ne_bytes());
        }
        __bytes
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Item {
    id: u32,
}
impl Item {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(4);
        __bytes.extend_from_slice(&self.id.to_ne_bytes());
        __bytes
    }
}


fn main() {
    let _path = Path { waypoints: Vec::new() };
    let _complex = Complex { vectors: Vec::new(), matrices: Vec::new(), items: Vec::new() };
    println!("✅ Generic types with number suffixes work!");
}

