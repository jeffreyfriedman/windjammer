#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct RigidBody2D {
    pub position: f32,
    pub collider: Collider2D,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Collider2D {
    Box { width: f32, height: f32 },
    Circle { radius: f32 },
}

