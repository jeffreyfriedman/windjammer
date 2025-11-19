/// Rigid body type for physics
pub enum RigidBodyType {
    /// Dynamic body (affected by forces)
    Dynamic,
    /// Static body (never moves)
    Static,
    /// Kinematic body (moved by code)
    Kinematic,
}
