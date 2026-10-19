#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes
    }
}


impl Vec2 {
#[inline]
pub fn test(&self, other: Vec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
}
}

