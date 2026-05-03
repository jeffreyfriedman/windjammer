use super::vec3::Vec3;
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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


#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Camera3D {
    pub position: Vec3,
}

