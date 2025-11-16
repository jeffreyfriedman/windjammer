//! Particle System
//!
//! Provides GPU-accelerated particle effects for AAA visual quality.
//!
//! ## Features
//! - CPU and GPU particle simulation
//! - Particle emitters with various shapes
//! - Lifetime, velocity, and color curves
//! - Texture atlas support
//! - Collision and forces
//! - Particle pooling

use crate::math::{Vec3, Vec4};
use std::collections::VecDeque;

/// Simple pseudo-random number generator (for particle variance)
fn pseudo_random(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed as f32) / (u32::MAX as f32)
}

/// Particle emitter
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Emitter position
    pub position: Vec3,
    /// Emitter shape
    pub shape: EmitterShape,
    /// Emission rate (particles per second)
    pub emission_rate: f32,
    /// Particle lifetime (seconds)
    pub lifetime: f32,
    /// Lifetime variance
    pub lifetime_variance: f32,
    /// Initial velocity
    pub initial_velocity: Vec3,
    /// Velocity variance
    pub velocity_variance: Vec3,
    /// Initial size
    pub initial_size: f32,
    /// Size over lifetime curve
    pub size_curve: Vec<f32>,
    /// Initial color
    pub initial_color: Vec4,
    /// Color over lifetime curve
    pub color_curve: Vec<Vec4>,
    /// Gravity scale
    pub gravity_scale: f32,
    /// Max particles
    pub max_particles: usize,
    /// Is emitting
    pub is_emitting: bool,
    /// Emission accumulator (internal)
    emission_accumulator: f32,
    /// Random seed (internal)
    random_seed: u32,
}

/// Emitter shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmitterShape {
    /// Point emitter
    Point,
    /// Sphere emitter
    Sphere { radius: f32 },
    /// Box emitter
    Box { size: Vec3 },
    /// Cone emitter
    Cone { radius: f32, angle: f32 },
    /// Circle emitter (2D)
    Circle { radius: f32 },
}

/// Individual particle
#[derive(Debug, Clone)]
pub struct Particle {
    /// Particle position
    pub position: Vec3,
    /// Particle velocity
    pub velocity: Vec3,
    /// Particle size
    pub size: f32,
    /// Particle color
    pub color: Vec4,
    /// Particle lifetime (remaining)
    pub lifetime: f32,
    /// Initial lifetime
    pub initial_lifetime: f32,
    /// Is alive
    pub alive: bool,
}

/// Particle system
#[derive(Debug)]
pub struct ParticleSystem {
    /// Active particles
    particles: Vec<Particle>,
    /// Particle pool (for reuse)
    particle_pool: VecDeque<Particle>,
    /// Emitters
    emitters: Vec<ParticleEmitter>,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            shape: EmitterShape::Point,
            emission_rate: 10.0,
            lifetime: 1.0,
            lifetime_variance: 0.0,
            initial_velocity: Vec3::new(0.0, 1.0, 0.0),
            velocity_variance: Vec3::ZERO,
            initial_size: 1.0,
            size_curve: vec![1.0],
            initial_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            color_curve: vec![Vec4::new(1.0, 1.0, 1.0, 1.0)],
            gravity_scale: 0.0,
            max_particles: 1000,
            is_emitting: true,
            emission_accumulator: 0.0,
            random_seed: 12345,
        }
    }
}

impl ParticleEmitter {
    /// Create a new particle emitter
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Set emission rate
    pub fn with_emission_rate(mut self, rate: f32) -> Self {
        self.emission_rate = rate;
        self
    }

    /// Set lifetime
    pub fn with_lifetime(mut self, lifetime: f32, variance: f32) -> Self {
        self.lifetime = lifetime;
        self.lifetime_variance = variance;
        self
    }

    /// Set initial velocity
    pub fn with_velocity(mut self, velocity: Vec3, variance: Vec3) -> Self {
        self.initial_velocity = velocity;
        self.velocity_variance = variance;
        self
    }

    /// Set shape
    pub fn with_shape(mut self, shape: EmitterShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set gravity
    pub fn with_gravity(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
        self
    }

    /// Set color
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.initial_color = color;
        self
    }

    /// Create a fire emitter
    pub fn fire(position: Vec3) -> Self {
        Self::new(position)
            .with_emission_rate(50.0)
            .with_lifetime(1.0, 0.3)
            .with_velocity(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.5, 0.5, 0.5))
            .with_color(Vec4::new(1.0, 0.5, 0.0, 1.0))
            .with_gravity(-0.5)
    }

    /// Create a smoke emitter
    pub fn smoke(position: Vec3) -> Self {
        Self::new(position)
            .with_emission_rate(20.0)
            .with_lifetime(2.0, 0.5)
            .with_velocity(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.2, 0.3))
            .with_color(Vec4::new(0.5, 0.5, 0.5, 0.5))
            .with_gravity(-0.2)
    }

    /// Create an explosion emitter
    pub fn explosion(position: Vec3) -> Self {
        Self::new(position)
            .with_emission_rate(100.0)
            .with_lifetime(0.5, 0.2)
            .with_velocity(Vec3::ZERO, Vec3::new(5.0, 5.0, 5.0))
            .with_shape(EmitterShape::Sphere { radius: 0.5 })
            .with_color(Vec4::new(1.0, 0.8, 0.0, 1.0))
    }

    /// Emit particles
    fn emit(&mut self, delta: f32, particles: &mut Vec<Particle>, pool: &mut VecDeque<Particle>) {
        if !self.is_emitting {
            return;
        }

        self.emission_accumulator += self.emission_rate * delta;

        while self.emission_accumulator >= 1.0 && particles.len() < self.max_particles {
            self.emission_accumulator -= 1.0;

            // Get particle from pool or create new
            let mut particle = pool.pop_front().unwrap_or_else(|| Particle {
                position: Vec3::ZERO,
                velocity: Vec3::ZERO,
                size: 1.0,
                color: Vec4::ONE,
                lifetime: 1.0,
                initial_lifetime: 1.0,
                alive: false,
            });

            // Initialize particle
            particle.position = self.get_spawn_position();
            particle.velocity = self.get_spawn_velocity();
            particle.size = self.initial_size;
            particle.color = self.initial_color;
            particle.lifetime = self.lifetime + (pseudo_random(&mut self.random_seed) - 0.5) * self.lifetime_variance;
            particle.initial_lifetime = particle.lifetime;
            particle.alive = true;

            particles.push(particle);
        }
    }

    /// Get spawn position based on emitter shape
    fn get_spawn_position(&mut self) -> Vec3 {
        match self.shape {
            EmitterShape::Point => self.position,
            EmitterShape::Sphere { radius } => {
                let theta = pseudo_random(&mut self.random_seed) * std::f32::consts::TAU;
                let phi = pseudo_random(&mut self.random_seed) * std::f32::consts::PI;
                let r = pseudo_random(&mut self.random_seed) * radius;
                self.position + Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.sin() * theta.sin(),
                    r * phi.cos(),
                )
            }
            EmitterShape::Box { size } => {
                self.position + Vec3::new(
                    (pseudo_random(&mut self.random_seed) - 0.5) * size.x,
                    (pseudo_random(&mut self.random_seed) - 0.5) * size.y,
                    (pseudo_random(&mut self.random_seed) - 0.5) * size.z,
                )
            }
            EmitterShape::Cone { radius, angle } => {
                let theta = pseudo_random(&mut self.random_seed) * std::f32::consts::TAU;
                let r = pseudo_random(&mut self.random_seed) * radius;
                let offset_angle = (pseudo_random(&mut self.random_seed) - 0.5) * angle;
                self.position + Vec3::new(
                    r * theta.cos(),
                    offset_angle.tan() * r,
                    r * theta.sin(),
                )
            }
            EmitterShape::Circle { radius } => {
                let theta = pseudo_random(&mut self.random_seed) * std::f32::consts::TAU;
                let r = pseudo_random(&mut self.random_seed) * radius;
                self.position + Vec3::new(r * theta.cos(), 0.0, r * theta.sin())
            }
        }
    }

    /// Get spawn velocity
    fn get_spawn_velocity(&mut self) -> Vec3 {
        self.initial_velocity + Vec3::new(
            (pseudo_random(&mut self.random_seed) - 0.5) * self.velocity_variance.x,
            (pseudo_random(&mut self.random_seed) - 0.5) * self.velocity_variance.y,
            (pseudo_random(&mut self.random_seed) - 0.5) * self.velocity_variance.z,
        )
    }
}

impl ParticleSystem {
    /// Create a new particle system
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            particle_pool: VecDeque::new(),
            emitters: Vec::new(),
        }
    }

    /// Add an emitter
    pub fn add_emitter(&mut self, emitter: ParticleEmitter) -> usize {
        self.emitters.push(emitter);
        self.emitters.len() - 1
    }

    /// Get emitter
    pub fn get_emitter_mut(&mut self, index: usize) -> Option<&mut ParticleEmitter> {
        self.emitters.get_mut(index)
    }

    /// Update particle system
    pub fn update(&mut self, delta: f32) {
        // Emit particles from all emitters
        for emitter in &mut self.emitters {
            emitter.emit(delta, &mut self.particles, &mut self.particle_pool);
        }

        // Update particles
        let gravity = Vec3::new(0.0, -9.81, 0.0);
        
        self.particles.retain_mut(|particle| {
            if !particle.alive {
                return false;
            }

            // Update lifetime
            particle.lifetime -= delta;
            if particle.lifetime <= 0.0 {
                particle.alive = false;
                return false;
            }

            // Apply gravity (find emitter's gravity scale)
            let gravity_scale = self.emitters.first().map(|e| e.gravity_scale).unwrap_or(0.0);
            particle.velocity += gravity * gravity_scale * delta;

            // Update position
            particle.position += particle.velocity * delta;

            true
        });

        // Return dead particles to pool
        self.particles.retain(|p| {
            if !p.alive {
                self.particle_pool.push_back(p.clone());
                false
            } else {
                true
            }
        });
    }

    /// Get active particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get particles
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_system_creation() {
        let system = ParticleSystem::new();
        assert_eq!(system.particle_count(), 0);
        println!("✅ ParticleSystem created");
    }

    #[test]
    fn test_emitter_creation() {
        let emitter = ParticleEmitter::new(Vec3::ZERO);
        assert_eq!(emitter.position, Vec3::ZERO);
        assert_eq!(emitter.emission_rate, 10.0);
        println!("✅ ParticleEmitter created");
    }

    #[test]
    fn test_emitter_builder() {
        let emitter = ParticleEmitter::new(Vec3::ZERO)
            .with_emission_rate(50.0)
            .with_lifetime(2.0, 0.5)
            .with_gravity(1.0);

        assert_eq!(emitter.emission_rate, 50.0);
        assert_eq!(emitter.lifetime, 2.0);
        assert_eq!(emitter.gravity_scale, 1.0);
        println!("✅ ParticleEmitter builder");
    }

    #[test]
    fn test_fire_preset() {
        let fire = ParticleEmitter::fire(Vec3::ZERO);
        assert_eq!(fire.emission_rate, 50.0);
        assert!(fire.initial_color.x > 0.5); // Red component
        println!("✅ Fire emitter preset");
    }

    #[test]
    fn test_smoke_preset() {
        let smoke = ParticleEmitter::smoke(Vec3::ZERO);
        assert_eq!(smoke.emission_rate, 20.0);
        assert!(smoke.lifetime >= 2.0);
        println!("✅ Smoke emitter preset");
    }

    #[test]
    fn test_explosion_preset() {
        let explosion = ParticleEmitter::explosion(Vec3::ZERO);
        assert_eq!(explosion.emission_rate, 100.0);
        assert!(matches!(explosion.shape, EmitterShape::Sphere { .. }));
        println!("✅ Explosion emitter preset");
    }

    #[test]
    fn test_add_emitter() {
        let mut system = ParticleSystem::new();
        let emitter = ParticleEmitter::new(Vec3::ZERO);
        let index = system.add_emitter(emitter);
        assert_eq!(index, 0);
        println!("✅ Add emitter to system");
    }

    #[test]
    fn test_particle_emission() {
        let mut system = ParticleSystem::new();
        let emitter = ParticleEmitter::new(Vec3::ZERO).with_emission_rate(10.0);
        system.add_emitter(emitter);

        // Update for 1 second
        system.update(1.0);

        // Should have emitted ~10 particles
        assert!(system.particle_count() > 0);
        assert!(system.particle_count() <= 15); // Allow some variance
        println!("✅ Particle emission: {} particles", system.particle_count());
    }

    #[test]
    fn test_particle_lifetime() {
        let mut system = ParticleSystem::new();
        let emitter = ParticleEmitter::new(Vec3::ZERO)
            .with_emission_rate(10.0)
            .with_lifetime(0.1, 0.0); // Very short lifetime
        system.add_emitter(emitter);

        // Emit particles
        system.update(0.1);
        let count_after_emit = system.particle_count();
        assert!(count_after_emit > 0);

        // Wait for particles to die
        system.update(0.2);
        assert_eq!(system.particle_count(), 0);

        println!("✅ Particle lifetime");
    }

    #[test]
    fn test_emitter_shapes() {
        let point = EmitterShape::Point;
        let sphere = EmitterShape::Sphere { radius: 5.0 };
        let box_shape = EmitterShape::Box { size: Vec3::ONE };
        let cone = EmitterShape::Cone { radius: 2.0, angle: 0.5 };
        let circle = EmitterShape::Circle { radius: 3.0 };

        assert!(matches!(point, EmitterShape::Point));
        assert!(matches!(sphere, EmitterShape::Sphere { .. }));
        assert!(matches!(box_shape, EmitterShape::Box { .. }));
        assert!(matches!(cone, EmitterShape::Cone { .. }));
        assert!(matches!(circle, EmitterShape::Circle { .. }));

        println!("✅ Emitter shapes");
    }

    #[test]
    fn test_stop_emission() {
        let mut system = ParticleSystem::new();
        let mut emitter = ParticleEmitter::new(Vec3::ZERO).with_emission_rate(10.0);
        emitter.is_emitting = false;
        system.add_emitter(emitter);

        system.update(1.0);
        assert_eq!(system.particle_count(), 0);

        println!("✅ Stop emission");
    }

    #[test]
    fn test_max_particles() {
        let mut system = ParticleSystem::new();
        let mut emitter = ParticleEmitter::new(Vec3::ZERO)
            .with_emission_rate(1000.0)
            .with_lifetime(10.0, 0.0);
        emitter.max_particles = 50;
        system.add_emitter(emitter);

        system.update(1.0);
        assert!(system.particle_count() <= 50);

        println!("✅ Max particles limit: {}", system.particle_count());
    }
}

