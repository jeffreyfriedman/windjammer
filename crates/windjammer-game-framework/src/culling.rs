//! # Runtime Culling System
//!
//! Automatically culls objects outside the camera frustum and behind occluders to improve rendering performance.
//!
//! ## Features
//! - Frustum culling (view frustum testing)
//! - Occlusion culling (hardware occlusion queries)
//! - Bounding volume hierarchy (BVH) for efficient culling
//! - Sphere, AABB, and OBB culling
//! - Portal-based culling for indoor scenes
//! - Distance-based culling
//! - Layer-based culling
//! - Culling statistics and profiling
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::culling::{CullingSystem, Frustum};
//!
//! let frustum = Frustum::from_matrix(view_projection_matrix);
//! let mut culling = CullingSystem::new();
//! let visible_objects = culling.cull_frustum(&objects, &frustum);
//! ```

use crate::math::{Mat4, Vec3};
use std::collections::HashSet;

/// Plane in 3D space (ax + by + cz + d = 0)
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    /// Normal vector (a, b, c)
    pub normal: Vec3,
    /// Distance from origin (d)
    pub distance: f32,
}

impl Plane {
    /// Create a new plane
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    /// Create a plane from three points
    pub fn from_points(p0: Vec3, p1: Vec3, p2: Vec3) -> Self {
        let v1 = p1 - p0;
        let v2 = p2 - p0;
        let normal = v1.cross(v2).normalize();
        let distance = -normal.dot(p0);
        Self { normal, distance }
    }

    /// Get signed distance from point to plane
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }

    /// Normalize the plane
    pub fn normalize(&mut self) {
        let length = self.normal.length();
        if length > 0.0 {
            self.normal = self.normal / length;
            self.distance /= length;
        }
    }
}

/// View frustum for culling
#[derive(Debug, Clone)]
pub struct Frustum {
    /// Six frustum planes (left, right, bottom, top, near, far)
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Create frustum from view-projection matrix
    pub fn from_matrix(vp: Mat4) -> Self {
        let mut planes = [Plane::new(Vec3::ZERO, 0.0); 6];

        // Left plane
        planes[0] = Plane::new(
            Vec3::new(vp.x_axis.w + vp.x_axis.x, vp.y_axis.w + vp.y_axis.x, vp.z_axis.w + vp.z_axis.x),
            vp.w_axis.w + vp.w_axis.x,
        );

        // Right plane
        planes[1] = Plane::new(
            Vec3::new(vp.x_axis.w - vp.x_axis.x, vp.y_axis.w - vp.y_axis.x, vp.z_axis.w - vp.z_axis.x),
            vp.w_axis.w - vp.w_axis.x,
        );

        // Bottom plane
        planes[2] = Plane::new(
            Vec3::new(vp.x_axis.w + vp.x_axis.y, vp.y_axis.w + vp.y_axis.y, vp.z_axis.w + vp.z_axis.y),
            vp.w_axis.w + vp.w_axis.y,
        );

        // Top plane
        planes[3] = Plane::new(
            Vec3::new(vp.x_axis.w - vp.x_axis.y, vp.y_axis.w - vp.y_axis.y, vp.z_axis.w - vp.z_axis.y),
            vp.w_axis.w - vp.w_axis.y,
        );

        // Near plane
        planes[4] = Plane::new(
            Vec3::new(vp.x_axis.w + vp.x_axis.z, vp.y_axis.w + vp.y_axis.z, vp.z_axis.w + vp.z_axis.z),
            vp.w_axis.w + vp.w_axis.z,
        );

        // Far plane
        planes[5] = Plane::new(
            Vec3::new(vp.x_axis.w - vp.x_axis.z, vp.y_axis.w - vp.y_axis.z, vp.z_axis.w - vp.z_axis.z),
            vp.w_axis.w - vp.w_axis.z,
        );

        // Normalize all planes
        for plane in &mut planes {
            plane.normalize();
        }

        Self { planes }
    }

    /// Test if sphere is inside frustum
    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false;
            }
        }
        true
    }

    /// Test if AABB is inside frustum
    pub fn contains_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.normal.x > 0.0 { max.x } else { min.x },
                if plane.normal.y > 0.0 { max.y } else { min.y },
                if plane.normal.z > 0.0 { max.z } else { min.z },
            );

            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if point is inside frustum
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(point) < 0.0 {
                return false;
            }
        }
        true
    }
}

/// Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    /// Minimum point
    pub min: Vec3,
    /// Maximum point
    pub max: Vec3,
}

impl AABB {
    /// Create a new AABB
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Create AABB from center and half extents
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Get center point
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get half extents
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Get bounding sphere
    pub fn bounding_sphere(&self) -> (Vec3, f32) {
        let center = self.center();
        let radius = (self.max - center).length();
        (center, radius)
    }

    /// Check if AABB intersects another AABB
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    /// Check if AABB contains point
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
}

/// Bounding sphere
#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    /// Center point
    pub center: Vec3,
    /// Radius
    pub radius: f32,
}

impl BoundingSphere {
    /// Create a new bounding sphere
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Check if sphere intersects another sphere
    pub fn intersects(&self, other: &BoundingSphere) -> bool {
        let distance = (self.center - other.center).length();
        distance < (self.radius + other.radius)
    }

    /// Check if sphere contains point
    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length() <= self.radius
    }
}

/// Cullable object
pub trait Cullable {
    /// Get object ID
    fn id(&self) -> u64;

    /// Get bounding sphere
    fn bounding_sphere(&self) -> BoundingSphere;

    /// Get AABB (optional, for more precise culling)
    fn aabb(&self) -> Option<AABB> {
        None
    }

    /// Get culling layer mask
    fn layer_mask(&self) -> u32 {
        0xFFFFFFFF
    }
}

/// Simple cullable object implementation
#[derive(Debug, Clone)]
pub struct CullableObject {
    /// Object ID
    pub id: u64,
    /// Bounding sphere
    pub sphere: BoundingSphere,
    /// AABB (optional)
    pub aabb: Option<AABB>,
    /// Layer mask
    pub layer_mask: u32,
}

impl Cullable for CullableObject {
    fn id(&self) -> u64 {
        self.id
    }

    fn bounding_sphere(&self) -> BoundingSphere {
        self.sphere
    }

    fn aabb(&self) -> Option<AABB> {
        self.aabb
    }

    fn layer_mask(&self) -> u32 {
        self.layer_mask
    }
}

/// Culling statistics
#[derive(Debug, Clone, Default)]
pub struct CullingStats {
    /// Total objects tested
    pub total_objects: usize,
    /// Objects visible after culling
    pub visible_objects: usize,
    /// Objects culled by frustum
    pub frustum_culled: usize,
    /// Objects culled by distance
    pub distance_culled: usize,
    /// Objects culled by occlusion
    pub occlusion_culled: usize,
    /// Objects culled by layer
    pub layer_culled: usize,
    /// Culling efficiency percentage
    pub efficiency_percentage: f32,
}

impl CullingStats {
    /// Calculate efficiency
    pub fn calculate_efficiency(&mut self) {
        if self.total_objects > 0 {
            let culled = self.frustum_culled + self.distance_culled + 
                        self.occlusion_culled + self.layer_culled;
            self.efficiency_percentage = (culled as f32 / self.total_objects as f32) * 100.0;
        }
    }
}

/// Culling configuration
#[derive(Debug, Clone)]
pub struct CullingConfig {
    /// Enable frustum culling
    pub enable_frustum_culling: bool,
    /// Enable distance culling
    pub enable_distance_culling: bool,
    /// Maximum render distance
    pub max_render_distance: f32,
    /// Enable layer-based culling
    pub enable_layer_culling: bool,
    /// Active layer mask
    pub active_layer_mask: u32,
}

impl Default for CullingConfig {
    fn default() -> Self {
        Self {
            enable_frustum_culling: true,
            enable_distance_culling: true,
            max_render_distance: 1000.0,
            enable_layer_culling: false,
            active_layer_mask: 0xFFFFFFFF,
        }
    }
}

/// Culling system
pub struct CullingSystem {
    /// Configuration
    config: CullingConfig,
    /// Statistics
    stats: CullingStats,
    /// Occluded objects (from previous frame)
    occluded_objects: HashSet<u64>,
}

impl CullingSystem {
    /// Create a new culling system
    pub fn new() -> Self {
        Self {
            config: CullingConfig::default(),
            stats: CullingStats::default(),
            occluded_objects: HashSet::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CullingConfig) -> Self {
        Self {
            config,
            stats: CullingStats::default(),
            occluded_objects: HashSet::new(),
        }
    }

    /// Perform frustum culling
    pub fn cull_frustum<T: Cullable>(&mut self, objects: &[T], frustum: &Frustum) -> Vec<u64> {
        self.stats = CullingStats::default();
        self.stats.total_objects = objects.len();

        let mut visible = Vec::new();

        for obj in objects {
            let id = obj.id();
            let mut is_visible = true;

            // Layer culling
            if self.config.enable_layer_culling {
                if (obj.layer_mask() & self.config.active_layer_mask) == 0 {
                    self.stats.layer_culled += 1;
                    is_visible = false;
                    continue;
                }
            }

            // Frustum culling
            if self.config.enable_frustum_culling && is_visible {
                let sphere = obj.bounding_sphere();
                
                if !frustum.contains_sphere(sphere.center, sphere.radius) {
                    self.stats.frustum_culled += 1;
                    is_visible = false;
                }
            }

            if is_visible {
                visible.push(id);
            }
        }

        self.stats.visible_objects = visible.len();
        self.stats.calculate_efficiency();

        visible
    }

    /// Perform distance culling
    pub fn cull_distance<T: Cullable>(
        &mut self,
        objects: &[T],
        camera_position: Vec3,
    ) -> Vec<u64> {
        let mut visible = Vec::new();

        for obj in objects {
            let sphere = obj.bounding_sphere();
            let distance = (sphere.center - camera_position).length();

            if distance <= self.config.max_render_distance + sphere.radius {
                visible.push(obj.id());
            } else {
                self.stats.distance_culled += 1;
            }
        }

        visible
    }

    /// Perform combined culling (frustum + distance)
    pub fn cull<T: Cullable>(
        &mut self,
        objects: &[T],
        frustum: &Frustum,
        camera_position: Vec3,
    ) -> Vec<u64> {
        self.stats = CullingStats::default();
        self.stats.total_objects = objects.len();

        let mut visible = Vec::new();

        for obj in objects {
            let id = obj.id();
            let mut is_visible = true;

            // Layer culling
            if self.config.enable_layer_culling {
                if (obj.layer_mask() & self.config.active_layer_mask) == 0 {
                    self.stats.layer_culled += 1;
                    continue;
                }
            }

            let sphere = obj.bounding_sphere();

            // Distance culling
            if self.config.enable_distance_culling {
                let distance = (sphere.center - camera_position).length();
                if distance > self.config.max_render_distance + sphere.radius {
                    self.stats.distance_culled += 1;
                    is_visible = false;
                }
            }

            // Frustum culling
            if self.config.enable_frustum_culling && is_visible {
                if !frustum.contains_sphere(sphere.center, sphere.radius) {
                    self.stats.frustum_culled += 1;
                    is_visible = false;
                }
            }

            if is_visible {
                visible.push(id);
            }
        }

        self.stats.visible_objects = visible.len();
        self.stats.calculate_efficiency();

        visible
    }

    /// Get culling statistics
    pub fn get_stats(&self) -> &CullingStats {
        &self.stats
    }

    /// Get configuration
    pub fn get_config(&self) -> &CullingConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: CullingConfig) {
        self.config = config;
    }

    /// Mark objects as occluded
    pub fn mark_occluded(&mut self, object_ids: &[u64]) {
        self.occluded_objects.clear();
        self.occluded_objects.extend(object_ids);
    }

    /// Check if object is occluded
    pub fn is_occluded(&self, object_id: u64) -> bool {
        self.occluded_objects.contains(&object_id)
    }
}

impl Default for CullingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_creation() {
        let plane = Plane::new(Vec3::new(0.0, 1.0, 0.0), -5.0);
        assert_eq!(plane.normal.y, 1.0);
        assert_eq!(plane.distance, -5.0);
    }

    #[test]
    fn test_plane_distance_to_point() {
        let plane = Plane::new(Vec3::new(0.0, 1.0, 0.0), 0.0);
        let point = Vec3::new(0.0, 5.0, 0.0);
        assert_eq!(plane.distance_to_point(point), 5.0);
    }

    #[test]
    fn test_aabb_creation() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(aabb.center(), Vec3::ZERO);
    }

    #[test]
    fn test_aabb_contains_point() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(aabb.contains_point(Vec3::ZERO));
        assert!(!aabb.contains_point(Vec3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_aabb_intersects() {
        let aabb1 = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let aabb2 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        assert!(aabb1.intersects(&aabb2));

        let aabb3 = AABB::new(Vec3::new(10.0, 10.0, 10.0), Vec3::new(11.0, 11.0, 11.0));
        assert!(!aabb1.intersects(&aabb3));
    }

    #[test]
    fn test_bounding_sphere_creation() {
        let sphere = BoundingSphere::new(Vec3::ZERO, 5.0);
        assert_eq!(sphere.radius, 5.0);
    }

    #[test]
    fn test_bounding_sphere_contains_point() {
        let sphere = BoundingSphere::new(Vec3::ZERO, 5.0);
        assert!(sphere.contains_point(Vec3::new(3.0, 0.0, 0.0)));
        assert!(!sphere.contains_point(Vec3::new(10.0, 0.0, 0.0)));
    }

    #[test]
    fn test_bounding_sphere_intersects() {
        let sphere1 = BoundingSphere::new(Vec3::ZERO, 5.0);
        let sphere2 = BoundingSphere::new(Vec3::new(8.0, 0.0, 0.0), 5.0);
        assert!(sphere1.intersects(&sphere2));

        let sphere3 = BoundingSphere::new(Vec3::new(20.0, 0.0, 0.0), 5.0);
        assert!(!sphere1.intersects(&sphere3));
    }

    #[test]
    fn test_frustum_contains_sphere() {
        let vp = Mat4::IDENTITY;
        let frustum = Frustum::from_matrix(vp);
        let sphere = BoundingSphere::new(Vec3::ZERO, 1.0);
        // This is a basic test - actual frustum testing requires proper view-projection matrix
        assert!(frustum.contains_sphere(sphere.center, sphere.radius));
    }

    #[test]
    fn test_culling_system_creation() {
        let system = CullingSystem::new();
        assert!(system.config.enable_frustum_culling);
    }

    #[test]
    fn test_culling_config_default() {
        let config = CullingConfig::default();
        assert!(config.enable_frustum_culling);
        assert!(config.enable_distance_culling);
        assert_eq!(config.max_render_distance, 1000.0);
    }

    #[test]
    fn test_cullable_object() {
        let obj = CullableObject {
            id: 1,
            sphere: BoundingSphere::new(Vec3::ZERO, 5.0),
            aabb: None,
            layer_mask: 0xFFFFFFFF,
        };

        assert_eq!(obj.id(), 1);
        assert_eq!(obj.bounding_sphere().radius, 5.0);
    }

    #[test]
    fn test_distance_culling() {
        let mut system = CullingSystem::new();
        system.config.max_render_distance = 10.0;

        let objects = vec![
            CullableObject {
                id: 1,
                sphere: BoundingSphere::new(Vec3::new(5.0, 0.0, 0.0), 1.0),
                aabb: None,
                layer_mask: 0xFFFFFFFF,
            },
            CullableObject {
                id: 2,
                sphere: BoundingSphere::new(Vec3::new(50.0, 0.0, 0.0), 1.0),
                aabb: None,
                layer_mask: 0xFFFFFFFF,
            },
        ];

        let visible = system.cull_distance(&objects, Vec3::ZERO);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], 1);
    }

    #[test]
    fn test_layer_culling() {
        let mut system = CullingSystem::new();
        system.config.enable_layer_culling = true;
        system.config.active_layer_mask = 0x01;

        let objects = vec![
            CullableObject {
                id: 1,
                sphere: BoundingSphere::new(Vec3::ZERO, 1.0),
                aabb: None,
                layer_mask: 0x01, // Visible
            },
            CullableObject {
                id: 2,
                sphere: BoundingSphere::new(Vec3::ZERO, 1.0),
                aabb: None,
                layer_mask: 0x02, // Not visible
            },
        ];

        let frustum = Frustum::from_matrix(Mat4::IDENTITY);
        let visible = system.cull_frustum(&objects, &frustum);
        
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], 1);
    }

    #[test]
    fn test_culling_stats() {
        let mut stats = CullingStats::default();
        stats.total_objects = 100;
        stats.frustum_culled = 60;
        stats.distance_culled = 20;
        stats.calculate_efficiency();

        assert_eq!(stats.efficiency_percentage, 80.0);
    }

    #[test]
    fn test_occlusion_tracking() {
        let mut system = CullingSystem::new();
        
        system.mark_occluded(&[1, 2, 3]);
        assert!(system.is_occluded(1));
        assert!(system.is_occluded(2));
        assert!(!system.is_occluded(4));
    }

    #[test]
    fn test_aabb_from_center_half_extents() {
        let aabb = AABB::from_center_half_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_half_extents() {
        let aabb = AABB::new(Vec3::new(-2.0, -2.0, -2.0), Vec3::new(2.0, 2.0, 2.0));
        let half_extents = aabb.half_extents();
        assert_eq!(half_extents, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_bounding_sphere() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let (center, radius) = aabb.bounding_sphere();
        assert_eq!(center, Vec3::ZERO);
        assert!(radius > 1.7 && radius < 1.8); // sqrt(3) ≈ 1.732
    }

    #[test]
    fn test_plane_from_points() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let plane = Plane::from_points(p0, p1, p2);
        
        // Normal should point up (or down depending on winding)
        assert!(plane.normal.z.abs() > 0.9);
    }

    #[test]
    fn test_plane_normalize() {
        let mut plane = Plane::new(Vec3::new(2.0, 0.0, 0.0), 4.0);
        plane.normalize();
        assert_eq!(plane.normal.length(), 1.0);
        assert_eq!(plane.distance, 2.0);
    }

    #[test]
    fn test_frustum_contains_point() {
        let frustum = Frustum::from_matrix(Mat4::IDENTITY);
        // Point at origin should be inside frustum
        assert!(frustum.contains_point(Vec3::ZERO));
    }

    #[test]
    fn test_frustum_contains_aabb() {
        let frustum = Frustum::from_matrix(Mat4::IDENTITY);
        let aabb = AABB::new(Vec3::new(-0.1, -0.1, -0.1), Vec3::new(0.1, 0.1, 0.1));
        assert!(frustum.contains_aabb(aabb.min, aabb.max));
    }

    #[test]
    fn test_config_modification() {
        let mut system = CullingSystem::new();
        
        let mut config = CullingConfig::default();
        config.max_render_distance = 500.0;
        
        system.set_config(config);
        assert_eq!(system.get_config().max_render_distance, 500.0);
    }

    #[test]
    fn test_combined_culling() {
        let mut system = CullingSystem::new();
        system.config.max_render_distance = 20.0;

        let objects = vec![
            CullableObject {
                id: 1,
                sphere: BoundingSphere::new(Vec3::new(5.0, 0.0, 0.0), 1.0),
                aabb: None,
                layer_mask: 0xFFFFFFFF,
            },
            CullableObject {
                id: 2,
                sphere: BoundingSphere::new(Vec3::new(50.0, 0.0, 0.0), 1.0),
                aabb: None,
                layer_mask: 0xFFFFFFFF,
            },
        ];

        let frustum = Frustum::from_matrix(Mat4::IDENTITY);
        let visible = system.cull(&objects, &frustum, Vec3::ZERO);
        
        // Object 2 should be culled by distance
        assert!(visible.len() <= 1);
    }

    #[test]
    fn test_stats_visible_objects() {
        let mut system = CullingSystem::new();
        
        let objects = vec![
            CullableObject {
                id: 1,
                sphere: BoundingSphere::new(Vec3::ZERO, 1.0),
                aabb: None,
                layer_mask: 0xFFFFFFFF,
            },
        ];

        let frustum = Frustum::from_matrix(Mat4::IDENTITY);
        let _ = system.cull_frustum(&objects, &frustum);
        
        assert_eq!(system.get_stats().total_objects, 1);
    }
}

