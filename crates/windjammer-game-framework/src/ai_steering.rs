//! AI Steering Behaviors
//!
//! Provides smooth, natural movement for AI agents using steering behaviors.
//! Based on Craig Reynolds' work on autonomous agents.

use crate::math::{Vec2, Vec3};
use std::collections::HashMap;

/// 2D Steering force
pub type SteeringForce2D = Vec2;

/// 3D Steering force
pub type SteeringForce3D = Vec3;

/// Agent properties for 2D steering
#[derive(Debug, Clone)]
pub struct SteeringAgent2D {
    /// Current position
    pub position: Vec2,
    /// Current velocity
    pub velocity: Vec2,
    /// Maximum speed
    pub max_speed: f32,
    /// Maximum force (steering power)
    pub max_force: f32,
    /// Agent radius (for obstacle avoidance)
    pub radius: f32,
    /// Mass (affects acceleration)
    pub mass: f32,
}

impl SteeringAgent2D {
    /// Create a new steering agent
    pub fn new(position: Vec2, max_speed: f32, max_force: f32) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            max_speed,
            max_force,
            radius: 0.5,
            mass: 1.0,
        }
    }

    /// Update agent position based on steering force
    pub fn update(&mut self, steering_force: SteeringForce2D, delta: f32) {
        // F = ma, so a = F/m
        let acceleration = steering_force / self.mass;
        
        // Update velocity
        self.velocity += acceleration * delta;
        
        // Limit to max speed
        if self.velocity.length() > self.max_speed {
            self.velocity = self.velocity.normalize() * self.max_speed;
        }
        
        // Update position
        self.position += self.velocity * delta;
    }

    /// Get current heading (normalized velocity)
    pub fn heading(&self) -> Vec2 {
        if self.velocity.length() > 0.0 {
            self.velocity.normalize()
        } else {
            Vec2::X
        }
    }
}

/// Agent properties for 3D steering
#[derive(Debug, Clone)]
pub struct SteeringAgent3D {
    /// Current position
    pub position: Vec3,
    /// Current velocity
    pub velocity: Vec3,
    /// Maximum speed
    pub max_speed: f32,
    /// Maximum force (steering power)
    pub max_force: f32,
    /// Agent radius (for obstacle avoidance)
    pub radius: f32,
    /// Mass (affects acceleration)
    pub mass: f32,
}

impl SteeringAgent3D {
    /// Create a new steering agent
    pub fn new(position: Vec3, max_speed: f32, max_force: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            max_speed,
            max_force,
            radius: 0.5,
            mass: 1.0,
        }
    }

    /// Update agent position based on steering force
    pub fn update(&mut self, steering_force: SteeringForce3D, delta: f32) {
        // F = ma, so a = F/m
        let acceleration = steering_force / self.mass;
        
        // Update velocity
        self.velocity += acceleration * delta;
        
        // Limit to max speed
        if self.velocity.length() > self.max_speed {
            self.velocity = self.velocity.normalize() * self.max_speed;
        }
        
        // Update position
        self.position += self.velocity * delta;
    }

    /// Get current heading (normalized velocity)
    pub fn heading(&self) -> Vec3 {
        if self.velocity.length() > 0.0 {
            self.velocity.normalize()
        } else {
            Vec3::X
        }
    }
}

/// 2D Steering Behaviors
pub struct SteeringBehaviors2D;

impl SteeringBehaviors2D {
    /// Seek: Move towards a target position
    pub fn seek(agent: &SteeringAgent2D, target: Vec2) -> SteeringForce2D {
        let desired_velocity = (target - agent.position).normalize() * agent.max_speed;
        let steering = desired_velocity - agent.velocity;
        Self::limit_force(steering, agent.max_force)
    }

    /// Flee: Move away from a target position
    pub fn flee(agent: &SteeringAgent2D, target: Vec2) -> SteeringForce2D {
        let desired_velocity = (agent.position - target).normalize() * agent.max_speed;
        let steering = desired_velocity - agent.velocity;
        Self::limit_force(steering, agent.max_force)
    }

    /// Arrive: Move towards target, slowing down as we approach
    pub fn arrive(agent: &SteeringAgent2D, target: Vec2, slowing_distance: f32) -> SteeringForce2D {
        let to_target = target - agent.position;
        let distance = to_target.length();
        
        if distance < 0.001 {
            return Vec2::ZERO;
        }
        
        let speed = if distance > slowing_distance {
            agent.max_speed
        } else {
            agent.max_speed * (distance / slowing_distance)
        };
        
        let desired_velocity = to_target.normalize() * speed;
        let steering = desired_velocity - agent.velocity;
        Self::limit_force(steering, agent.max_force)
    }

    /// Pursue: Predict target's future position and seek it
    pub fn pursue(agent: &SteeringAgent2D, target_pos: Vec2, target_vel: Vec2) -> SteeringForce2D {
        let to_target = target_pos - agent.position;
        let look_ahead_time = to_target.length() / (agent.max_speed + target_vel.length());
        let predicted_pos = target_pos + target_vel * look_ahead_time;
        Self::seek(agent, predicted_pos)
    }

    /// Evade: Predict target's future position and flee from it
    pub fn evade(agent: &SteeringAgent2D, target_pos: Vec2, target_vel: Vec2) -> SteeringForce2D {
        let to_target = target_pos - agent.position;
        let look_ahead_time = to_target.length() / (agent.max_speed + target_vel.length());
        let predicted_pos = target_pos + target_vel * look_ahead_time;
        Self::flee(agent, predicted_pos)
    }

    /// Wander: Random exploration
    /// 
    /// Note: Pass a changing value (like time or frame count) as `random_seed` for variation
    pub fn wander(
        agent: &SteeringAgent2D,
        wander_angle: &mut f32,
        wander_radius: f32,
        wander_distance: f32,
        wander_jitter: f32,
        random_seed: f32,
        delta: f32,
    ) -> SteeringForce2D {
        // Simple pseudo-random using sine (deterministic but varies with seed)
        let random_value = (random_seed * 12.9898).sin() * 43758.5453;
        let random_jitter = (random_value.fract() - 0.5) * wander_jitter * delta;
        
        // Add jitter to wander angle
        *wander_angle += random_jitter;
        
        // Calculate wander target
        let circle_center = agent.position + agent.heading() * wander_distance;
        let offset = Vec2::new(wander_angle.cos(), wander_angle.sin()) * wander_radius;
        let target = circle_center + offset;
        
        Self::seek(agent, target)
    }

    /// Obstacle Avoidance: Avoid circular obstacles
    pub fn obstacle_avoidance(
        agent: &SteeringAgent2D,
        obstacles: &[(Vec2, f32)], // (position, radius)
        detection_distance: f32,
    ) -> SteeringForce2D {
        let mut steering = Vec2::ZERO;
        let heading = agent.heading();
        
        for &(obstacle_pos, obstacle_radius) in obstacles {
            let to_obstacle = obstacle_pos - agent.position;
            let distance = to_obstacle.length();
            let combined_radius = agent.radius + obstacle_radius;
            
            // Check if obstacle is ahead and within detection range
            if distance < detection_distance + combined_radius {
                let forward_projection = to_obstacle.dot(heading);
                
                if forward_projection > 0.0 {
                    // Calculate lateral distance to obstacle
                    let lateral_distance = (to_obstacle - heading * forward_projection).length();
                    
                    if lateral_distance < combined_radius {
                        // Obstacle is in the way, steer away
                        let avoidance_force = (agent.position - obstacle_pos).normalize();
                        let strength = (1.0 - distance / detection_distance).max(0.0);
                        steering += avoidance_force * agent.max_force * strength;
                    }
                }
            }
        }
        
        Self::limit_force(steering, agent.max_force)
    }

    /// Separation: Maintain distance from nearby agents
    pub fn separation(
        agent: &SteeringAgent2D,
        neighbors: &[Vec2],
        separation_distance: f32,
    ) -> SteeringForce2D {
        let mut steering = Vec2::ZERO;
        let mut count = 0;
        
        for &neighbor_pos in neighbors {
            let to_neighbor = agent.position - neighbor_pos;
            let distance = to_neighbor.length();
            
            if distance > 0.0 && distance < separation_distance {
                // Weight by distance (closer = stronger force)
                let weight = 1.0 - (distance / separation_distance);
                steering += to_neighbor.normalize() * weight;
                count += 1;
            }
        }
        
        if count > 0 {
            steering /= count as f32;
            steering = steering.normalize() * agent.max_speed - agent.velocity;
            Self::limit_force(steering, agent.max_force)
        } else {
            Vec2::ZERO
        }
    }

    /// Alignment: Match velocity with nearby agents
    pub fn alignment(
        agent: &SteeringAgent2D,
        neighbors: &[(Vec2, Vec2)], // (position, velocity)
        alignment_distance: f32,
    ) -> SteeringForce2D {
        let mut average_velocity = Vec2::ZERO;
        let mut count = 0;
        
        for &(neighbor_pos, neighbor_vel) in neighbors {
            let distance = (agent.position - neighbor_pos).length();
            
            if distance > 0.0 && distance < alignment_distance {
                average_velocity += neighbor_vel;
                count += 1;
            }
        }
        
        if count > 0 {
            average_velocity /= count as f32;
            let desired_velocity = average_velocity.normalize() * agent.max_speed;
            let steering = desired_velocity - agent.velocity;
            Self::limit_force(steering, agent.max_force)
        } else {
            Vec2::ZERO
        }
    }

    /// Cohesion: Move towards center of nearby agents
    pub fn cohesion(
        agent: &SteeringAgent2D,
        neighbors: &[Vec2],
        cohesion_distance: f32,
    ) -> SteeringForce2D {
        let mut center_of_mass = Vec2::ZERO;
        let mut count = 0;
        
        for &neighbor_pos in neighbors {
            let distance = (agent.position - neighbor_pos).length();
            
            if distance > 0.0 && distance < cohesion_distance {
                center_of_mass += neighbor_pos;
                count += 1;
            }
        }
        
        if count > 0 {
            center_of_mass /= count as f32;
            Self::seek(agent, center_of_mass)
        } else {
            Vec2::ZERO
        }
    }

    /// Path Following: Follow a path of waypoints
    pub fn path_following(
        agent: &SteeringAgent2D,
        path: &[Vec2],
        path_radius: f32,
        current_waypoint: &mut usize,
    ) -> SteeringForce2D {
        if path.is_empty() || *current_waypoint >= path.len() {
            return Vec2::ZERO;
        }
        
        let target = path[*current_waypoint];
        let distance_to_target = (target - agent.position).length();
        
        // Move to next waypoint if close enough
        if distance_to_target < path_radius {
            *current_waypoint += 1;
            if *current_waypoint >= path.len() {
                return Vec2::ZERO;
            }
        }
        
        Self::seek(agent, path[*current_waypoint])
    }

    /// Limit steering force to maximum
    fn limit_force(force: Vec2, max_force: f32) -> Vec2 {
        if force.length() > max_force {
            force.normalize() * max_force
        } else {
            force
        }
    }
}

/// 3D Steering Behaviors
pub struct SteeringBehaviors3D;

impl SteeringBehaviors3D {
    /// Seek: Move towards a target position
    pub fn seek(agent: &SteeringAgent3D, target: Vec3) -> SteeringForce3D {
        let desired_velocity = (target - agent.position).normalize() * agent.max_speed;
        let steering = desired_velocity - agent.velocity;
        Self::limit_force(steering, agent.max_force)
    }

    /// Flee: Move away from a target position
    pub fn flee(agent: &SteeringAgent3D, target: Vec3) -> SteeringForce3D {
        let desired_velocity = (agent.position - target).normalize() * agent.max_speed;
        let steering = desired_velocity - agent.velocity;
        Self::limit_force(steering, agent.max_force)
    }

    /// Arrive: Move towards target, slowing down as we approach
    pub fn arrive(agent: &SteeringAgent3D, target: Vec3, slowing_distance: f32) -> SteeringForce3D {
        let to_target = target - agent.position;
        let distance = to_target.length();
        
        if distance < 0.001 {
            return Vec3::ZERO;
        }
        
        let speed = if distance > slowing_distance {
            agent.max_speed
        } else {
            agent.max_speed * (distance / slowing_distance)
        };
        
        let desired_velocity = to_target.normalize() * speed;
        let steering = desired_velocity - agent.velocity;
        Self::limit_force(steering, agent.max_force)
    }

    /// Pursue: Predict target's future position and seek it
    pub fn pursue(agent: &SteeringAgent3D, target_pos: Vec3, target_vel: Vec3) -> SteeringForce3D {
        let to_target = target_pos - agent.position;
        let look_ahead_time = to_target.length() / (agent.max_speed + target_vel.length());
        let predicted_pos = target_pos + target_vel * look_ahead_time;
        Self::seek(agent, predicted_pos)
    }

    /// Evade: Predict target's future position and flee from it
    pub fn evade(agent: &SteeringAgent3D, target_pos: Vec3, target_vel: Vec3) -> SteeringForce3D {
        let to_target = target_pos - agent.position;
        let look_ahead_time = to_target.length() / (agent.max_speed + target_vel.length());
        let predicted_pos = target_pos + target_vel * look_ahead_time;
        Self::flee(agent, predicted_pos)
    }

    /// Separation: Maintain distance from nearby agents
    pub fn separation(
        agent: &SteeringAgent3D,
        neighbors: &[Vec3],
        separation_distance: f32,
    ) -> SteeringForce3D {
        let mut steering = Vec3::ZERO;
        let mut count = 0;
        
        for &neighbor_pos in neighbors {
            let to_neighbor = agent.position - neighbor_pos;
            let distance = to_neighbor.length();
            
            if distance > 0.0 && distance < separation_distance {
                let weight = 1.0 - (distance / separation_distance);
                steering += to_neighbor.normalize() * weight;
                count += 1;
            }
        }
        
        if count > 0 {
            steering /= count as f32;
            steering = steering.normalize() * agent.max_speed - agent.velocity;
            Self::limit_force(steering, agent.max_force)
        } else {
            Vec3::ZERO
        }
    }

    /// Alignment: Match velocity with nearby agents
    pub fn alignment(
        agent: &SteeringAgent3D,
        neighbors: &[(Vec3, Vec3)], // (position, velocity)
        alignment_distance: f32,
    ) -> SteeringForce3D {
        let mut average_velocity = Vec3::ZERO;
        let mut count = 0;
        
        for &(neighbor_pos, neighbor_vel) in neighbors {
            let distance = (agent.position - neighbor_pos).length();
            
            if distance > 0.0 && distance < alignment_distance {
                average_velocity += neighbor_vel;
                count += 1;
            }
        }
        
        if count > 0 {
            average_velocity /= count as f32;
            let desired_velocity = average_velocity.normalize() * agent.max_speed;
            let steering = desired_velocity - agent.velocity;
            Self::limit_force(steering, agent.max_force)
        } else {
            Vec3::ZERO
        }
    }

    /// Cohesion: Move towards center of nearby agents
    pub fn cohesion(
        agent: &SteeringAgent3D,
        neighbors: &[Vec3],
        cohesion_distance: f32,
    ) -> SteeringForce3D {
        let mut center_of_mass = Vec3::ZERO;
        let mut count = 0;
        
        for &neighbor_pos in neighbors {
            let distance = (agent.position - neighbor_pos).length();
            
            if distance > 0.0 && distance < cohesion_distance {
                center_of_mass += neighbor_pos;
                count += 1;
            }
        }
        
        if count > 0 {
            center_of_mass /= count as f32;
            Self::seek(agent, center_of_mass)
        } else {
            Vec3::ZERO
        }
    }

    /// Path Following: Follow a path of waypoints
    pub fn path_following(
        agent: &SteeringAgent3D,
        path: &[Vec3],
        path_radius: f32,
        current_waypoint: &mut usize,
    ) -> SteeringForce3D {
        if path.is_empty() || *current_waypoint >= path.len() {
            return Vec3::ZERO;
        }
        
        let target = path[*current_waypoint];
        let distance_to_target = (target - agent.position).length();
        
        // Move to next waypoint if close enough
        if distance_to_target < path_radius {
            *current_waypoint += 1;
            if *current_waypoint >= path.len() {
                return Vec3::ZERO;
            }
        }
        
        Self::seek(agent, path[*current_waypoint])
    }

    /// Limit steering force to maximum
    fn limit_force(force: Vec3, max_force: f32) -> Vec3 {
        if force.length() > max_force {
            force.normalize() * max_force
        } else {
            force
        }
    }
}

/// Steering behavior combiner with weights
#[derive(Debug, Clone)]
pub struct SteeringCombiner {
    behaviors: HashMap<String, f32>, // behavior name -> weight
}

impl SteeringCombiner {
    /// Create a new steering combiner
    pub fn new() -> Self {
        Self {
            behaviors: HashMap::new(),
        }
    }

    /// Add a weighted behavior
    pub fn add_behavior(&mut self, name: impl Into<String>, weight: f32) {
        self.behaviors.insert(name.into(), weight);
    }

    /// Combine multiple steering forces with weights
    pub fn combine_2d(&self, forces: &[(String, SteeringForce2D)]) -> SteeringForce2D {
        let mut result = Vec2::ZERO;
        
        for (name, force) in forces {
            if let Some(&weight) = self.behaviors.get(name) {
                result += *force * weight;
            }
        }
        
        result
    }

    /// Combine multiple steering forces with weights
    pub fn combine_3d(&self, forces: &[(String, SteeringForce3D)]) -> SteeringForce3D {
        let mut result = Vec3::ZERO;
        
        for (name, force) in forces {
            if let Some(&weight) = self.behaviors.get(name) {
                result += *force * weight;
            }
        }
        
        result
    }
}

impl Default for SteeringCombiner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_2d_creation() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        assert_eq!(agent.position, Vec2::ZERO);
        assert_eq!(agent.velocity, Vec2::ZERO);
        assert_eq!(agent.max_speed, 10.0);
        assert_eq!(agent.max_force, 5.0);
    }

    #[test]
    fn test_agent_2d_update() {
        let mut agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let force = Vec2::new(5.0, 0.0);
        agent.update(force, 0.1);
        
        assert!(agent.velocity.x > 0.0);
        assert!(agent.position.x > 0.0);
    }

    #[test]
    fn test_agent_2d_max_speed() {
        let mut agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let large_force = Vec2::new(1000.0, 0.0);
        agent.update(large_force, 1.0);
        
        assert!(agent.velocity.length() <= agent.max_speed);
    }

    #[test]
    fn test_seek_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let target = Vec2::new(10.0, 0.0);
        let force = SteeringBehaviors2D::seek(&agent, target);
        
        assert!(force.x > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_flee_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let target = Vec2::new(10.0, 0.0);
        let force = SteeringBehaviors2D::flee(&agent, target);
        
        assert!(force.x < 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_arrive_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let target = Vec2::new(5.0, 0.0);
        let force = SteeringBehaviors2D::arrive(&agent, target, 10.0);
        
        assert!(force.x > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_pursue_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let target_pos = Vec2::new(10.0, 0.0);
        let target_vel = Vec2::new(5.0, 0.0);
        let force = SteeringBehaviors2D::pursue(&agent, target_pos, target_vel);
        
        assert!(force.x > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_evade_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let target_pos = Vec2::new(10.0, 0.0);
        let target_vel = Vec2::new(-5.0, 0.0);
        let force = SteeringBehaviors2D::evade(&agent, target_pos, target_vel);
        
        assert!(force.x < 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_separation_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let neighbors = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let force = SteeringBehaviors2D::separation(&agent, &neighbors, 5.0);
        
        assert!(force.length() > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_alignment_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let neighbors = vec![
            (Vec2::new(1.0, 0.0), Vec2::new(5.0, 0.0)),
            (Vec2::new(0.0, 1.0), Vec2::new(0.0, 5.0)),
        ];
        let force = SteeringBehaviors2D::alignment(&agent, &neighbors, 5.0);
        
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_cohesion_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let neighbors = vec![Vec2::new(5.0, 0.0), Vec2::new(0.0, 5.0)];
        let force = SteeringBehaviors2D::cohesion(&agent, &neighbors, 10.0);
        
        assert!(force.length() > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_path_following_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let path = vec![Vec2::new(5.0, 0.0), Vec2::new(10.0, 0.0), Vec2::new(10.0, 5.0)];
        let mut waypoint = 0;
        let force = SteeringBehaviors2D::path_following(&agent, &path, 1.0, &mut waypoint);
        
        assert!(force.x > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_agent_3d_creation() {
        let agent = SteeringAgent3D::new(Vec3::ZERO, 10.0, 5.0);
        assert_eq!(agent.position, Vec3::ZERO);
        assert_eq!(agent.velocity, Vec3::ZERO);
        assert_eq!(agent.max_speed, 10.0);
        assert_eq!(agent.max_force, 5.0);
    }

    #[test]
    fn test_seek_3d() {
        let agent = SteeringAgent3D::new(Vec3::ZERO, 10.0, 5.0);
        let target = Vec3::new(10.0, 0.0, 0.0);
        let force = SteeringBehaviors3D::seek(&agent, target);
        
        assert!(force.x > 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_flee_3d() {
        let agent = SteeringAgent3D::new(Vec3::ZERO, 10.0, 5.0);
        let target = Vec3::new(10.0, 0.0, 0.0);
        let force = SteeringBehaviors3D::flee(&agent, target);
        
        assert!(force.x < 0.0);
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_steering_combiner() {
        let mut combiner = SteeringCombiner::new();
        combiner.add_behavior("seek", 1.0);
        combiner.add_behavior("flee", 0.5);
        
        let forces = vec![
            ("seek".to_string(), Vec2::new(5.0, 0.0)),
            ("flee".to_string(), Vec2::new(-2.0, 0.0)),
        ];
        
        let combined = combiner.combine_2d(&forces);
        assert!(combined.x > 0.0); // seek (1.0) > flee (0.5)
    }

    #[test]
    fn test_obstacle_avoidance_2d() {
        let agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        let obstacles = vec![(Vec2::new(5.0, 0.0), 1.0)];
        let force = SteeringBehaviors2D::obstacle_avoidance(&agent, &obstacles, 10.0);
        
        // Force should steer away from obstacle
        assert!(force.length() <= agent.max_force);
    }

    #[test]
    fn test_agent_heading() {
        let mut agent = SteeringAgent2D::new(Vec2::ZERO, 10.0, 5.0);
        agent.velocity = Vec2::new(5.0, 0.0);
        let heading = agent.heading();
        
        assert!((heading.x - 1.0).abs() < 0.001);
        assert!((heading.y - 0.0).abs() < 0.001);
    }
}

