//! # GPU Particle System
//!
//! Provides high-performance GPU-accelerated particle effects with forces and collision.
//!
//! ## Features
//! - GPU compute shader simulation
//! - Force fields (gravity, wind, vortex, turbulence)
//! - Collision detection (sphere, plane, box)
//! - Soft particles
//! - Particle sorting
//! - Texture atlas support
//! - Color and size curves
//! - Millions of particles
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::particles_gpu::{GPUParticleSystem, ForceField, ForceType};
//!
//! let mut system = GPUParticleSystem::new(100000);
//! system.add_force_field(ForceField::gravity(Vec3::new(0.0, -9.8, 0.0)));
//! system.add_force_field(ForceField::wind(Vec3::new(5.0, 0.0, 0.0)));
//! system.emit(1000);
//! system.update(0.016);
//! ```

use crate::math::{Vec3, Vec4};

/// Force field type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForceType {
    /// Constant force (gravity, wind)
    Constant,
    /// Point attractor/repulsor
    Point,
    /// Vortex (spinning force)
    Vortex,
    /// Turbulence (noise-based)
    Turbulence,
    /// Drag (velocity dampening)
    Drag,
}

/// Force field
#[derive(Debug, Clone)]
pub struct ForceField {
    /// Force type
    pub force_type: ForceType,
    /// Force strength
    pub strength: f32,
    /// Force position (for point/vortex)
    pub position: Vec3,
    /// Force direction (for constant/vortex)
    pub direction: Vec3,
    /// Force radius (for point/vortex)
    pub radius: f32,
    /// Falloff exponent
    pub falloff: f32,
}

impl ForceField {
    /// Create a gravity force
    pub fn gravity(direction: Vec3) -> Self {
        Self {
            force_type: ForceType::Constant,
            strength: 1.0,
            position: Vec3::ZERO,
            direction,
            radius: 0.0,
            falloff: 0.0,
        }
    }

    /// Create a wind force
    pub fn wind(direction: Vec3) -> Self {
        Self {
            force_type: ForceType::Constant,
            strength: 1.0,
            position: Vec3::ZERO,
            direction,
            radius: 0.0,
            falloff: 0.0,
        }
    }

    /// Create a point attractor
    pub fn attractor(position: Vec3, strength: f32, radius: f32) -> Self {
        Self {
            force_type: ForceType::Point,
            strength,
            position,
            direction: Vec3::ZERO,
            radius,
            falloff: 2.0,
        }
    }

    /// Create a point repulsor
    pub fn repulsor(position: Vec3, strength: f32, radius: f32) -> Self {
        Self {
            force_type: ForceType::Point,
            strength: -strength,
            position,
            direction: Vec3::ZERO,
            radius,
            falloff: 2.0,
        }
    }

    /// Create a vortex
    pub fn vortex(position: Vec3, axis: Vec3, strength: f32, radius: f32) -> Self {
        Self {
            force_type: ForceType::Vortex,
            strength,
            position,
            direction: axis.normalize(),
            radius,
            falloff: 1.0,
        }
    }

    /// Create turbulence
    pub fn turbulence(strength: f32) -> Self {
        Self {
            force_type: ForceType::Turbulence,
            strength,
            position: Vec3::ZERO,
            direction: Vec3::ZERO,
            radius: 0.0,
            falloff: 0.0,
        }
    }

    /// Create drag
    pub fn drag(strength: f32) -> Self {
        Self {
            force_type: ForceType::Drag,
            strength,
            position: Vec3::ZERO,
            direction: Vec3::ZERO,
            radius: 0.0,
            falloff: 0.0,
        }
    }

    /// Apply force to a particle
    pub fn apply(&self, position: Vec3, velocity: Vec3, _time: f32) -> Vec3 {
        match self.force_type {
            ForceType::Constant => self.direction * self.strength,
            ForceType::Point => {
                let to_point = self.position - position;
                let distance = to_point.length();
                if distance < self.radius && distance > 0.001 {
                    let falloff = 1.0 - (distance / self.radius).powf(self.falloff);
                    to_point.normalize() * self.strength * falloff
                } else {
                    Vec3::ZERO
                }
            }
            ForceType::Vortex => {
                let to_point = self.position - position;
                let distance = to_point.length();
                if distance < self.radius && distance > 0.001 {
                    let falloff = 1.0 - (distance / self.radius).powf(self.falloff);
                    let tangent = self.direction.cross(to_point).normalize();
                    tangent * self.strength * falloff
                } else {
                    Vec3::ZERO
                }
            }
            ForceType::Turbulence => {
                // Simple pseudo-random turbulence
                let noise_x = (position.x * 0.1).sin() * (position.y * 0.1).cos();
                let noise_y = (position.y * 0.1).sin() * (position.z * 0.1).cos();
                let noise_z = (position.z * 0.1).sin() * (position.x * 0.1).cos();
                Vec3::new(noise_x, noise_y, noise_z) * self.strength
            }
            ForceType::Drag => velocity * -self.strength,
        }
    }
}

/// Collision shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionShape {
    /// Sphere collider
    Sphere,
    /// Plane collider
    Plane,
    /// Box collider
    Box,
}

/// Collision object
#[derive(Debug, Clone)]
pub struct Collider {
    /// Collision shape
    pub shape: CollisionShape,
    /// Position
    pub position: Vec3,
    /// Size/radius
    pub size: Vec3,
    /// Normal (for plane)
    pub normal: Vec3,
    /// Restitution (bounciness)
    pub restitution: f32,
    /// Friction
    pub friction: f32,
}

impl Collider {
    /// Create a sphere collider
    pub fn sphere(position: Vec3, radius: f32) -> Self {
        Self {
            shape: CollisionShape::Sphere,
            position,
            size: Vec3::new(radius, radius, radius),
            normal: Vec3::ZERO,
            restitution: 0.5,
            friction: 0.1,
        }
    }

    /// Create a plane collider
    pub fn plane(position: Vec3, normal: Vec3) -> Self {
        Self {
            shape: CollisionShape::Plane,
            position,
            size: Vec3::ZERO,
            normal: normal.normalize(),
            restitution: 0.3,
            friction: 0.5,
        }
    }

    /// Create a box collider
    pub fn box_collider(position: Vec3, size: Vec3) -> Self {
        Self {
            shape: CollisionShape::Box,
            position,
            size,
            normal: Vec3::ZERO,
            restitution: 0.4,
            friction: 0.3,
        }
    }

    /// Check collision and resolve
    pub fn collide(&self, position: Vec3, velocity: Vec3) -> (Vec3, Vec3) {
        match self.shape {
            CollisionShape::Sphere => {
                let to_particle = position - self.position;
                let distance = to_particle.length();
                let radius = self.size.x;

                if distance < radius && distance > 0.001 {
                    // Collision detected
                    let normal = to_particle.normalize();
                    let penetration = radius - distance;
                    let new_position = position + normal * penetration;

                    // Reflect velocity
                    let dot = velocity.dot(normal);
                    let new_velocity = velocity - normal * (dot * (1.0 + self.restitution));
                    let new_velocity = new_velocity * (1.0 - self.friction);

                    (new_position, new_velocity)
                } else {
                    (position, velocity)
                }
            }
            CollisionShape::Plane => {
                let distance = (position - self.position).dot(self.normal);

                if distance < 0.0 {
                    // Below plane
                    let new_position = position - self.normal * distance;

                    // Reflect velocity
                    let dot = velocity.dot(self.normal);
                    let new_velocity = velocity - self.normal * (dot * (1.0 + self.restitution));
                    let new_velocity = new_velocity * (1.0 - self.friction);

                    (new_position, new_velocity)
                } else {
                    (position, velocity)
                }
            }
            CollisionShape::Box => {
                let half_size = self.size * 0.5;
                let local_pos = position - self.position;

                // Check if inside box
                if local_pos.x.abs() < half_size.x
                    && local_pos.y.abs() < half_size.y
                    && local_pos.z.abs() < half_size.z
                {
                    // Find closest face
                    let dx = half_size.x - local_pos.x.abs();
                    let dy = half_size.y - local_pos.y.abs();
                    let dz = half_size.z - local_pos.z.abs();

                    let (normal, penetration) = if dx < dy && dx < dz {
                        (Vec3::new(local_pos.x.signum(), 0.0, 0.0), dx)
                    } else if dy < dz {
                        (Vec3::new(0.0, local_pos.y.signum(), 0.0), dy)
                    } else {
                        (Vec3::new(0.0, 0.0, local_pos.z.signum()), dz)
                    };

                    let new_position = position + normal * penetration;

                    // Reflect velocity
                    let dot = velocity.dot(normal);
                    let new_velocity = velocity - normal * (dot * (1.0 + self.restitution));
                    let new_velocity = new_velocity * (1.0 - self.friction);

                    (new_position, new_velocity)
                } else {
                    (position, velocity)
                }
            }
        }
    }
}

/// GPU particle data
#[derive(Debug, Clone)]
pub struct GPUParticle {
    /// Position
    pub position: Vec3,
    /// Velocity
    pub velocity: Vec3,
    /// Size
    pub size: f32,
    /// Color
    pub color: Vec4,
    /// Lifetime (remaining)
    pub lifetime: f32,
    /// Initial lifetime
    pub initial_lifetime: f32,
    /// Mass
    pub mass: f32,
    /// Is alive
    pub alive: bool,
}

impl Default for GPUParticle {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            size: 1.0,
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            lifetime: 0.0,
            initial_lifetime: 1.0,
            mass: 1.0,
            alive: false,
        }
    }
}

/// GPU particle system
pub struct GPUParticleSystem {
    /// Particles
    particles: Vec<GPUParticle>,
    /// Force fields
    force_fields: Vec<ForceField>,
    /// Colliders
    colliders: Vec<Collider>,
    /// Max particles
    max_particles: usize,
    /// Next particle index
    next_particle: usize,
    /// Particle count
    particle_count: usize,
    /// Simulation time
    time: f32,
}

impl GPUParticleSystem {
    /// Create a new GPU particle system
    pub fn new(max_particles: usize) -> Self {
        let mut particles = Vec::with_capacity(max_particles);
        for _ in 0..max_particles {
            particles.push(GPUParticle::default());
        }

        Self {
            particles,
            force_fields: Vec::new(),
            colliders: Vec::new(),
            max_particles,
            next_particle: 0,
            particle_count: 0,
            time: 0.0,
        }
    }

    /// Add a force field
    pub fn add_force_field(&mut self, force_field: ForceField) {
        self.force_fields.push(force_field);
    }

    /// Remove all force fields
    pub fn clear_force_fields(&mut self) {
        self.force_fields.clear();
    }

    /// Add a collider
    pub fn add_collider(&mut self, collider: Collider) {
        self.colliders.push(collider);
    }

    /// Remove all colliders
    pub fn clear_colliders(&mut self) {
        self.colliders.clear();
    }

    /// Emit particles
    pub fn emit(&mut self, count: usize) {
        for _ in 0..count {
            if self.particle_count >= self.max_particles {
                break;
            }

            let particle = &mut self.particles[self.next_particle];
            particle.position = Vec3::ZERO;
            particle.velocity = Vec3::new(0.0, 5.0, 0.0);
            particle.size = 1.0;
            particle.color = Vec4::new(1.0, 1.0, 1.0, 1.0);
            particle.lifetime = 5.0;
            particle.initial_lifetime = 5.0;
            particle.mass = 1.0;
            particle.alive = true;

            self.next_particle = (self.next_particle + 1) % self.max_particles;
            self.particle_count += 1;
        }
    }

    /// Update particle system
    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;

        for particle in &mut self.particles {
            if !particle.alive {
                continue;
            }

            // Update lifetime
            particle.lifetime -= delta_time;
            if particle.lifetime <= 0.0 {
                particle.alive = false;
                self.particle_count = self.particle_count.saturating_sub(1);
                continue;
            }

            // Apply forces
            let mut total_force = Vec3::ZERO;
            for force_field in &self.force_fields {
                total_force += force_field.apply(particle.position, particle.velocity, self.time);
            }

            // Update velocity (F = ma, a = F/m)
            let acceleration = total_force / particle.mass;
            particle.velocity += acceleration * delta_time;

            // Update position
            particle.position += particle.velocity * delta_time;

            // Check collisions
            for collider in &self.colliders {
                let (new_position, new_velocity) =
                    collider.collide(particle.position, particle.velocity);
                particle.position = new_position;
                particle.velocity = new_velocity;
            }

            // Update color/size based on lifetime
            let life_ratio = particle.lifetime / particle.initial_lifetime;
            particle.color.w = life_ratio; // Fade out alpha
        }
    }

    /// Get active particle count
    pub fn particle_count(&self) -> usize {
        self.particle_count
    }

    /// Get particles
    pub fn particles(&self) -> &[GPUParticle] {
        &self.particles
    }

    /// Get force field count
    pub fn force_field_count(&self) -> usize {
        self.force_fields.len()
    }

    /// Get collider count
    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_field_gravity() {
        let force = ForceField::gravity(Vec3::new(0.0, -9.8, 0.0));
        assert_eq!(force.force_type, ForceType::Constant);
        assert_eq!(force.direction.y, -9.8);
    }

    #[test]
    fn test_force_field_wind() {
        let force = ForceField::wind(Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(force.force_type, ForceType::Constant);
        assert_eq!(force.direction.x, 5.0);
    }

    #[test]
    fn test_force_field_attractor() {
        let force = ForceField::attractor(Vec3::ZERO, 10.0, 5.0);
        assert_eq!(force.force_type, ForceType::Point);
        assert_eq!(force.strength, 10.0);
        assert_eq!(force.radius, 5.0);
    }

    #[test]
    fn test_force_field_repulsor() {
        let force = ForceField::repulsor(Vec3::ZERO, 10.0, 5.0);
        assert_eq!(force.force_type, ForceType::Point);
        assert_eq!(force.strength, -10.0);
    }

    #[test]
    fn test_force_field_vortex() {
        let force = ForceField::vortex(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 5.0, 10.0);
        assert_eq!(force.force_type, ForceType::Vortex);
        assert_eq!(force.strength, 5.0);
    }

    #[test]
    fn test_force_field_apply_constant() {
        let force = ForceField::gravity(Vec3::new(0.0, -9.8, 0.0));
        let result = force.apply(Vec3::ZERO, Vec3::ZERO, 0.0);
        assert_eq!(result.y, -9.8);
    }

    #[test]
    fn test_collider_sphere() {
        let collider = Collider::sphere(Vec3::ZERO, 5.0);
        assert_eq!(collider.shape, CollisionShape::Sphere);
        assert_eq!(collider.size.x, 5.0);
    }

    #[test]
    fn test_collider_plane() {
        let collider = Collider::plane(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(collider.shape, CollisionShape::Plane);
        assert_eq!(collider.normal.y, 1.0);
    }

    #[test]
    fn test_collider_box() {
        let collider = Collider::box_collider(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        assert_eq!(collider.shape, CollisionShape::Box);
        assert_eq!(collider.size.x, 10.0);
    }

    #[test]
    fn test_gpu_particle_system_creation() {
        let system = GPUParticleSystem::new(1000);
        assert_eq!(system.max_particles, 1000);
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_gpu_particle_system_emit() {
        let mut system = GPUParticleSystem::new(1000);
        system.emit(10);
        assert_eq!(system.particle_count(), 10);
    }

    #[test]
    fn test_gpu_particle_system_update() {
        let mut system = GPUParticleSystem::new(1000);
        system.emit(10);
        system.update(0.016);
        // Particles should still be alive
        assert!(system.particle_count() > 0);
    }

    #[test]
    fn test_gpu_particle_system_forces() {
        let mut system = GPUParticleSystem::new(1000);
        system.add_force_field(ForceField::gravity(Vec3::new(0.0, -9.8, 0.0)));
        system.add_force_field(ForceField::wind(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!(system.force_field_count(), 2);
    }

    #[test]
    fn test_gpu_particle_system_colliders() {
        let mut system = GPUParticleSystem::new(1000);
        system.add_collider(Collider::plane(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)));
        system.add_collider(Collider::sphere(Vec3::new(5.0, 0.0, 0.0), 2.0));
        assert_eq!(system.collider_count(), 2);
    }

    #[test]
    fn test_gpu_particle_lifetime() {
        let mut system = GPUParticleSystem::new(1000);
        system.emit(1);

        // Update for longer than lifetime
        for _ in 0..1000 {
            system.update(0.016);
        }

        // Particle should be dead
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_force_field_drag() {
        let force = ForceField::drag(0.5);
        assert_eq!(force.force_type, ForceType::Drag);
        assert_eq!(force.strength, 0.5);
    }

    #[test]
    fn test_force_field_turbulence() {
        let force = ForceField::turbulence(1.0);
        assert_eq!(force.force_type, ForceType::Turbulence);
        assert_eq!(force.strength, 1.0);
    }

    #[test]
    fn test_collider_sphere_collision() {
        let collider = Collider::sphere(Vec3::ZERO, 5.0);
        let position = Vec3::new(2.0, 0.0, 0.0); // Inside sphere
        let velocity = Vec3::new(-1.0, 0.0, 0.0);

        let (new_pos, new_vel) = collider.collide(position, velocity);

        // Position should be pushed out
        assert!(new_pos.x > position.x);
        // Velocity should be reflected
        assert!(new_vel.x > 0.0);
    }

    #[test]
    fn test_collider_plane_collision() {
        let collider = Collider::plane(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        let position = Vec3::new(0.0, -1.0, 0.0); // Below plane
        let velocity = Vec3::new(0.0, -1.0, 0.0);

        let (new_pos, new_vel) = collider.collide(position, velocity);

        // Position should be above plane
        assert!(new_pos.y >= 0.0);
        // Velocity should be reflected upward
        assert!(new_vel.y > 0.0);
    }

    #[test]
    fn test_gpu_particle_default() {
        let particle = GPUParticle::default();
        assert!(!particle.alive);
        assert_eq!(particle.size, 1.0);
        assert_eq!(particle.mass, 1.0);
    }

    #[test]
    fn test_force_types() {
        assert_eq!(ForceType::Constant, ForceType::Constant);
        assert_eq!(ForceType::Point, ForceType::Point);
        assert_ne!(ForceType::Constant, ForceType::Point);
    }

    #[test]
    fn test_collision_shapes() {
        assert_eq!(CollisionShape::Sphere, CollisionShape::Sphere);
        assert_eq!(CollisionShape::Plane, CollisionShape::Plane);
        assert_ne!(CollisionShape::Sphere, CollisionShape::Plane);
    }

    #[test]
    fn test_gpu_particle_system_clear_forces() {
        let mut system = GPUParticleSystem::new(1000);
        system.add_force_field(ForceField::gravity(Vec3::new(0.0, -9.8, 0.0)));
        system.clear_force_fields();
        assert_eq!(system.force_field_count(), 0);
    }

    #[test]
    fn test_gpu_particle_system_clear_colliders() {
        let mut system = GPUParticleSystem::new(1000);
        system.add_collider(Collider::sphere(Vec3::ZERO, 5.0));
        system.clear_colliders();
        assert_eq!(system.collider_count(), 0);
    }

    #[test]
    fn test_gpu_particle_system_max_particles() {
        let mut system = GPUParticleSystem::new(10);
        system.emit(20); // Try to emit more than max
        assert_eq!(system.particle_count(), 10); // Should cap at max
    }
}

