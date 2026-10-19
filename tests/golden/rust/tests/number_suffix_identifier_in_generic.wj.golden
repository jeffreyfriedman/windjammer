#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Vec2 {
    x: f32,
    y: f32,
}
impl Vec2 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes
    }
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
    data: [f32; 16],
}
impl Mat4 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(64);
        for __el in &self.data {
            __bytes.extend_from_slice(&__el.to_ne_bytes());
        }
        __bytes
    }
}


#[derive(Debug, Clone, PartialEq, Default)]

struct Container {
    vec2s: Vec<Vec2>,
    vec3s: Vec<Vec3>,
    mat4s: Vec<Mat4>,
    items: Vec<Item2D>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Item2D {
    id: u32,
}
impl Item2D {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(4);
        __bytes.extend_from_slice(&self.id.to_ne_bytes());
        __bytes
    }
}


fn main() {
    let _c = Container { vec2s: Vec::new(), vec3s: Vec::new(), mat4s: Vec::new(), items: Vec::new() };
    println!("✅ Number suffix identifiers in generics work!");
}

