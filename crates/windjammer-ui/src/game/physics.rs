//! Simple 2D physics for games

use super::math::Vec2;

/// 2D axis-aligned bounding box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half_size = size * 0.5;
        Self {
            min: center - half_size,
            max: center + half_size,
        }
    }

    /// Check if this AABB intersects with another
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Check if a point is inside this AABB
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }
}

/// Rigidbody for 2D physics
#[derive(Debug, Clone, PartialEq)]
pub struct Rigidbody {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub mass: f32,
    pub drag: f32,
}

impl Rigidbody {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            mass: 1.0,
            drag: 0.99,
        }
    }

    /// Apply a force to the rigidbody
    pub fn apply_force(&mut self, force: Vec2) {
        self.acceleration += force * (1.0 / self.mass);
    }

    /// Update physics simulation
    pub fn update(&mut self, delta: f32) {
        self.velocity += self.acceleration * delta;
        self.velocity = self.velocity * self.drag;
        self.position += self.velocity * delta;
        self.acceleration = Vec2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_intersection() {
        let box1 = AABB::from_center_size(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let box2 = AABB::from_center_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0));
        let box3 = AABB::from_center_size(Vec2::new(100.0, 100.0), Vec2::new(10.0, 10.0));

        assert!(box1.intersects(&box2));
        assert!(!box1.intersects(&box3));
    }

    #[test]
    fn test_aabb_contains_point() {
        let bbox = AABB::from_center_size(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));

        assert!(bbox.contains_point(Vec2::new(0.0, 0.0)));
        assert!(bbox.contains_point(Vec2::new(4.0, 4.0)));
        assert!(!bbox.contains_point(Vec2::new(10.0, 10.0)));
    }

    #[test]
    fn test_rigidbody_update() {
        let mut body = Rigidbody::new(Vec2::ZERO);
        body.velocity = Vec2::new(10.0, 0.0);
        body.update(1.0);

        assert!(body.position.x > 0.0);
    }

    #[test]
    fn test_rigidbody_force() {
        let mut body = Rigidbody::new(Vec2::ZERO);
        body.apply_force(Vec2::new(100.0, 0.0));
        body.update(0.1);

        assert!(body.velocity.x > 0.0);
    }
}
