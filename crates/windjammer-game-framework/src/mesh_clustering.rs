//! Mesh Clustering System (Nanite-style)
//!
//! This module implements a mesh clustering system inspired by Unreal Engine 5's Nanite.
//! It breaks large meshes into small clusters of triangles that can be efficiently
//! culled and rendered.
//!
//! **Philosophy**: Zero crate leakage - no wgpu or glam types exposed.

use crate::math::Vec3;
use std::collections::HashMap;

/// Configuration for mesh clustering
pub struct ClusterConfig {
    /// Maximum triangles per cluster (typically 64-128)
    pub max_triangles: usize,
    
    /// Maximum vertices per cluster (typically 64-256)
    pub max_vertices: usize,
    
    /// Enable cluster bounds calculation
    pub calculate_bounds: bool,
    
    /// Enable cluster normal cone calculation (for backface culling)
    pub calculate_normal_cone: bool,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            max_triangles: 128,
            max_vertices: 256,
            calculate_bounds: true,
            calculate_normal_cone: true,
        }
    }
}

/// A single mesh cluster
///
/// Represents a small group of triangles that can be efficiently culled.
#[derive(Clone, Debug)]
pub struct MeshCluster {
    /// Cluster ID
    pub id: usize,
    
    /// Triangle indices (local to this cluster)
    pub triangles: Vec<[u32; 3]>,
    
    /// Vertex positions (local to this cluster)
    pub vertices: Vec<Vec3>,
    
    /// Vertex normals
    pub normals: Vec<Vec3>,
    
    /// Bounding sphere center
    pub bounds_center: Vec3,
    
    /// Bounding sphere radius
    pub bounds_radius: f32,
    
    /// Normal cone axis (for backface culling)
    pub normal_cone_axis: Vec3,
    
    /// Normal cone angle (in radians)
    pub normal_cone_angle: f32,
    
    /// LOD level this cluster belongs to
    pub lod_level: usize,
}

impl MeshCluster {
    /// Create a new empty cluster
    pub fn new(id: usize, lod_level: usize) -> Self {
        Self {
            id,
            triangles: Vec::new(),
            vertices: Vec::new(),
            normals: Vec::new(),
            bounds_center: Vec3::new(0.0, 0.0, 0.0),
            bounds_radius: 0.0,
            normal_cone_axis: Vec3::new(0.0, 1.0, 0.0),
            normal_cone_angle: std::f32::consts::PI,
            lod_level,
        }
    }
    
    /// Check if this cluster can fit more triangles
    pub fn can_add_triangle(&self, config: &ClusterConfig) -> bool {
        self.triangles.len() < config.max_triangles
            && self.vertices.len() + 3 <= config.max_vertices
    }
    
    /// Add a triangle to this cluster
    pub fn add_triangle(
        &mut self,
        vertices: [Vec3; 3],
        normals: [Vec3; 3],
    ) {
        let base_index = self.vertices.len() as u32;
        
        // Add vertices
        self.vertices.extend_from_slice(&vertices);
        self.normals.extend_from_slice(&normals);
        
        // Add triangle indices
        self.triangles.push([
            base_index,
            base_index + 1,
            base_index + 2,
        ]);
    }
    
    /// Calculate bounding sphere for this cluster
    pub fn calculate_bounds(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        
        // Calculate center as average of all vertices
        let mut center = Vec3::new(0.0, 0.0, 0.0);
        for vertex in &self.vertices {
            center = center + *vertex;
        }
        center = center * (1.0 / self.vertices.len() as f32);
        self.bounds_center = center;
        
        // Calculate radius as max distance from center
        let mut max_dist_sq = 0.0;
        for vertex in &self.vertices {
            let dist_sq = (*vertex - center).length_squared();
            if dist_sq > max_dist_sq {
                max_dist_sq = dist_sq;
            }
        }
        self.bounds_radius = max_dist_sq.sqrt();
    }
    
    /// Calculate normal cone for backface culling
    pub fn calculate_normal_cone(&mut self) {
        if self.normals.is_empty() {
            return;
        }
        
        // Calculate average normal as cone axis
        let mut avg_normal = Vec3::new(0.0, 0.0, 0.0);
        for normal in &self.normals {
            avg_normal = avg_normal + *normal;
        }
        avg_normal = avg_normal.normalize();
        self.normal_cone_axis = avg_normal;
        
        // Calculate cone angle as max deviation from average
        let mut max_angle = 0.0;
        for normal in &self.normals {
            let dot = avg_normal.dot(*normal);
            let angle = dot.acos();
            if angle > max_angle {
                max_angle = angle;
            }
        }
        self.normal_cone_angle = max_angle;
    }
    
    /// Test if this cluster is visible from a camera position
    pub fn is_visible(&self, camera_pos: Vec3, camera_forward: Vec3) -> bool {
        // Frustum culling (simplified - just check if behind camera)
        let to_cluster = self.bounds_center - camera_pos;
        
        // If cluster is behind camera, cull it
        if to_cluster.dot(camera_forward) < -self.bounds_radius {
            return false;
        }
        
        // Backface culling using normal cone
        // If all normals point away from camera, cull the cluster
        let view_dir = to_cluster.normalize();
        let cone_dot = self.normal_cone_axis.dot(view_dir);
        
        // If cone axis points away and angle is small, entire cluster is backfacing
        if cone_dot < -self.normal_cone_angle.cos() {
            return false;
        }
        
        true
    }
}

/// Mesh clustering system
///
/// Manages the creation and culling of mesh clusters.
pub struct MeshClusteringSystem {
    config: ClusterConfig,
    clusters: HashMap<usize, Vec<MeshCluster>>, // LOD level -> clusters
}

impl MeshClusteringSystem {
    /// Create a new mesh clustering system
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            config,
            clusters: HashMap::new(),
        }
    }
    
    /// Create clusters from a mesh
    ///
    /// Takes a list of triangles and breaks them into clusters.
    ///
    /// # Arguments
    /// * `vertices` - Vertex positions
    /// * `normals` - Vertex normals
    /// * `indices` - Triangle indices (groups of 3)
    /// * `lod_level` - LOD level for these clusters
    ///
    /// # Returns
    /// Vector of mesh clusters
    pub fn create_clusters(
        &mut self,
        vertices: &[Vec3],
        normals: &[Vec3],
        indices: &[u32],
        lod_level: usize,
    ) -> Vec<MeshCluster> {
        let mut clusters = Vec::new();
        let mut current_cluster = MeshCluster::new(0, lod_level);
        
        // Process triangles
        for triangle_indices in indices.chunks(3) {
            if triangle_indices.len() != 3 {
                continue;
            }
            
            // Check if current cluster is full
            if !current_cluster.can_add_triangle(&self.config) {
                // Finalize current cluster
                if self.config.calculate_bounds {
                    current_cluster.calculate_bounds();
                }
                if self.config.calculate_normal_cone {
                    current_cluster.calculate_normal_cone();
                }
                
                clusters.push(current_cluster);
                current_cluster = MeshCluster::new(clusters.len(), lod_level);
            }
            
            // Add triangle to current cluster
            let v0 = vertices[triangle_indices[0] as usize];
            let v1 = vertices[triangle_indices[1] as usize];
            let v2 = vertices[triangle_indices[2] as usize];
            
            let n0 = normals[triangle_indices[0] as usize];
            let n1 = normals[triangle_indices[1] as usize];
            let n2 = normals[triangle_indices[2] as usize];
            
            current_cluster.add_triangle([v0, v1, v2], [n0, n1, n2]);
        }
        
        // Add final cluster if it has any triangles
        if !current_cluster.triangles.is_empty() {
            if self.config.calculate_bounds {
                current_cluster.calculate_bounds();
            }
            if self.config.calculate_normal_cone {
                current_cluster.calculate_normal_cone();
            }
            clusters.push(current_cluster);
        }
        
        // Store clusters
        self.clusters.insert(lod_level, clusters.clone());
        
        clusters
    }
    
    /// Cull clusters based on camera position and direction
    ///
    /// Returns indices of visible clusters for a given LOD level.
    pub fn cull_clusters(
        &self,
        lod_level: usize,
        camera_pos: Vec3,
        camera_forward: Vec3,
    ) -> Vec<usize> {
        let mut visible = Vec::new();
        
        if let Some(clusters) = self.clusters.get(&lod_level) {
            for cluster in clusters {
                if cluster.is_visible(camera_pos, camera_forward) {
                    visible.push(cluster.id);
                }
            }
        }
        
        visible
    }
    
    /// Get clusters for a specific LOD level
    pub fn get_clusters(&self, lod_level: usize) -> Option<&Vec<MeshCluster>> {
        self.clusters.get(&lod_level)
    }
    
    /// Get total number of clusters across all LOD levels
    pub fn total_clusters(&self) -> usize {
        self.clusters.values().map(|v| v.len()).sum()
    }
    
    /// Get total number of triangles across all clusters
    pub fn total_triangles(&self) -> usize {
        self.clusters
            .values()
            .flat_map(|clusters| clusters.iter())
            .map(|cluster| cluster.triangles.len())
            .sum()
    }
}

/// Statistics for mesh clustering
pub struct ClusterStats {
    pub total_clusters: usize,
    pub total_triangles: usize,
    pub visible_clusters: usize,
    pub culled_clusters: usize,
    pub triangles_rendered: usize,
    pub triangles_culled: usize,
}

impl ClusterStats {
    pub fn new() -> Self {
        Self {
            total_clusters: 0,
            total_triangles: 0,
            visible_clusters: 0,
            culled_clusters: 0,
            triangles_rendered: 0,
            triangles_culled: 0,
        }
    }
    
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    
    pub fn record_cluster(&mut self, cluster: &MeshCluster, visible: bool) {
        self.total_clusters += 1;
        self.total_triangles += cluster.triangles.len();
        
        if visible {
            self.visible_clusters += 1;
            self.triangles_rendered += cluster.triangles.len();
        } else {
            self.culled_clusters += 1;
            self.triangles_culled += cluster.triangles.len();
        }
    }
    
    pub fn culling_efficiency(&self) -> f32 {
        if self.total_clusters == 0 {
            return 0.0;
        }
        (self.culled_clusters as f32 / self.total_clusters as f32) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cluster_creation() {
        let config = ClusterConfig {
            max_triangles: 2,
            max_vertices: 6,
            ..Default::default()
        };
        
        let mut system = MeshClusteringSystem::new(config);
        
        // Create a simple mesh (2 triangles)
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.5, 1.0, 0.0),
        ];
        
        let normals = vec![Vec3::new(0.0, 0.0, 1.0); 6];
        let indices = vec![0, 1, 2, 3, 4, 5];
        
        let clusters = system.create_clusters(&vertices, &normals, &indices, 0);
        
        assert_eq!(clusters.len(), 1); // Should fit in one cluster
        assert_eq!(clusters[0].triangles.len(), 2);
    }
    
    #[test]
    fn test_cluster_bounds() {
        let mut cluster = MeshCluster::new(0, 0);
        
        cluster.add_triangle(
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
            ],
            [Vec3::new(0.0, 0.0, 1.0); 3],
        );
        
        cluster.calculate_bounds();
        
        // Center should be roughly (0.5, 0.33, 0.0)
        assert!(cluster.bounds_center.x > 0.4 && cluster.bounds_center.x < 0.6);
        assert!(cluster.bounds_radius > 0.0);
    }
    
    #[test]
    fn test_cluster_visibility() {
        let mut cluster = MeshCluster::new(0, 0);
        
        cluster.add_triangle(
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
            ],
            [Vec3::new(0.0, 0.0, 1.0); 3],
        );
        
        cluster.calculate_bounds();
        cluster.calculate_normal_cone();
        
        // Camera looking at cluster
        let camera_pos = Vec3::new(0.5, 0.5, 5.0);
        let camera_forward = Vec3::new(0.0, 0.0, -1.0);
        
        assert!(cluster.is_visible(camera_pos, camera_forward));
        
        // Camera behind cluster
        let camera_pos_behind = Vec3::new(0.5, 0.5, -5.0);
        assert!(!cluster.is_visible(camera_pos_behind, camera_forward));
    }
    
    #[test]
    fn test_cluster_culling() {
        let config = ClusterConfig::default();
        let mut system = MeshClusteringSystem::new(config);
        
        // Create a simple mesh
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        
        let normals = vec![Vec3::new(0.0, 0.0, 1.0); 3];
        let indices = vec![0, 1, 2];
        
        system.create_clusters(&vertices, &normals, &indices, 0);
        
        // Camera looking at cluster
        let camera_pos = Vec3::new(0.5, 0.5, 5.0);
        let camera_forward = Vec3::new(0.0, 0.0, -1.0);
        
        let visible = system.cull_clusters(0, camera_pos, camera_forward);
        assert_eq!(visible.len(), 1);
    }
}

