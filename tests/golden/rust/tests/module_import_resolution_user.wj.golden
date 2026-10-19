use module_import_resolution::RigidBody2D;
use module_import_resolution::Collider2D;
#[inline]
pub fn test_collision(a: &RigidBody2D, b: &RigidBody2D) -> bool {
    match &a.collider {
        Collider2D::Box { width: w1, height: h1 } => match &b.collider {
            Collider2D::Box { width: w2, height: h2 } => {
                w1 == w2 && h1 == h2
            },
            Collider2D::Circle { radius: _ } => false,
        },
        Collider2D::Circle { radius: r1 } => match &b.collider {
            Collider2D::Box { width: _, height: _ } => false,
            Collider2D::Circle { radius: r2 } => r1 == r2,
        },
    }
}

