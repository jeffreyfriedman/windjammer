/// 2D physics rigid body
pub struct RigidBody2D {
    /// Body type
    pub body_type: RigidBodyType,
    /// Linear velocity
    pub velocity: Vec2,
    /// Mass
    pub mass: f32,
}

impl RigidBody2D {
    pub fn new(body_type: RigidBodyType) -> RigidBody2D {
        todo!()
    }

    /// Applies a force to the body
    pub fn apply_force(&mut self, force: Vec2) {
        todo!()
    }

    /// Applies an impulse to the body
    pub fn apply_impulse(&mut self, impulse: Vec2) {
        todo!()
    }

}
