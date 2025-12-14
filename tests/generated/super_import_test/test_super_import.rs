use super::Vec3;


#[derive(Copy, Clone, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct Camera3D {
    pub position: Vec3,
}

