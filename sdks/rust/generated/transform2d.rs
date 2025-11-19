/// 2D transformation (position, rotation, scale)
pub struct Transform2D {
    /// Position
    pub position: Vec2,
    /// Rotation in radians
    pub rotation: f32,
    /// Scale
    pub scale: Vec2,
}

impl Transform2D {
    pub fn new(position: Vec2, rotation: f32, scale: Vec2) -> Transform2D {
        todo!()
    }

    /// Translates the transform
    pub fn translate(&mut self, offset: Vec2) {
        todo!()
    }

    /// Rotates the transform
    pub fn rotate(&mut self, angle: f32) {
        todo!()
    }

}
