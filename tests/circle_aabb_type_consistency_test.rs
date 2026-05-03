// TDD: CircleCollider/AABB f32 type consistency
//
// Bug: CircleCollider used f64 while AABB used f32, causing type errors in
// intersects_aabb (self.x < aabb.x etc - f64 vs f32 comparison).
//
// Fix: CircleCollider uses f32 throughout for consistency with AABB.
//
// Verifies: Code with CircleCollider + AABB compiles without f32/f64 mismatch.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_circle_aabb_intersection_compiles_without_type_error() {
    // CircleCollider and AABB both use f32 - no mixing
    let source = r#"
pub struct AABB {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl AABB {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> AABB {
        AABB { x, y, width, height }
    }
}

pub struct CircleCollider {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

impl CircleCollider {
    pub fn new(x: f32, y: f32, radius: f32) -> CircleCollider {
        CircleCollider { x, y, radius }
    }

    pub fn intersects_aabb(self, aabb: AABB) -> bool {
        let mut closest_x = self.x;
        let mut closest_y = self.y;
        if self.x < aabb.x {
            closest_x = aabb.x;
        } else if self.x > aabb.x + aabb.width {
            closest_x = aabb.x + aabb.width;
        }
        if self.y < aabb.y {
            closest_y = aabb.y;
        } else if self.y > aabb.y + aabb.height {
            closest_y = aabb.y + aabb.height;
        }
        let dx = self.x - closest_x;
        let dy = self.y - closest_y;
        let distance_squared = dx * dx + dy * dy;
        distance_squared <= self.radius * self.radius
    }
}

pub fn test_intersection() -> bool {
    let circle = CircleCollider::new(5.0, 5.0, 2.0);
    let aabb = AABB::new(3.0, 3.0, 4.0, 4.0);
    circle.intersects_aabb(aabb)
}
"#;

    let rust_code = test_utils::compile_single(source);
    test_utils::verify_rust_compiles(&rust_code).expect("Generated Rust should compile");
}
