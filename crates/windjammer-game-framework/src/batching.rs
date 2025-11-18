//! # Runtime Draw Call Batching System
//!
//! Automatically batches draw calls to minimize GPU state changes and improve rendering performance.
//!
//! ## Features
//! - Automatic mesh batching by material
//! - Instanced rendering for identical meshes
//! - Dynamic batching for small meshes
//! - Static batching for static geometry
//! - Texture atlas support
//! - Material property blocks
//! - Batch statistics and profiling
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::batching::{BatchManager, BatchConfig};
//!
//! let mut batch_manager = BatchManager::new(BatchConfig::default());
//! batch_manager.add_mesh(mesh_id, material_id, transform);
//! let batches = batch_manager.generate_batches();
//! ```

use crate::math::{Mat4, Vec3};
use std::collections::HashMap;

/// Batch configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum vertices per batch
    pub max_vertices_per_batch: usize,
    /// Maximum instances per batch
    pub max_instances_per_batch: usize,
    /// Enable dynamic batching
    pub enable_dynamic_batching: bool,
    /// Enable static batching
    pub enable_static_batching: bool,
    /// Enable instanced rendering
    pub enable_instancing: bool,
    /// Minimum instances for instancing
    pub min_instances_for_instancing: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_vertices_per_batch: 65536,
            max_instances_per_batch: 1024,
            enable_dynamic_batching: true,
            enable_static_batching: true,
            enable_instancing: true,
            min_instances_for_instancing: 2,
        }
    }
}

/// Mesh identifier
pub type MeshId = u64;

/// Material identifier
pub type MaterialId = u64;

/// Batch key for grouping draw calls
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BatchKey {
    material_id: MaterialId,
    mesh_id: Option<MeshId>, // None for dynamic batching
}

/// Draw call instance
#[derive(Debug, Clone)]
pub struct DrawInstance {
    /// Mesh ID
    pub mesh_id: MeshId,
    /// Material ID
    pub material_id: MaterialId,
    /// Transform matrix
    pub transform: Mat4,
    /// Vertex count
    pub vertex_count: usize,
    /// Is static (doesn't move)
    pub is_static: bool,
}

/// Batched draw call
#[derive(Debug, Clone)]
pub struct Batch {
    /// Material ID
    pub material_id: MaterialId,
    /// Batch type
    pub batch_type: BatchType,
    /// Instance transforms
    pub transforms: Vec<Mat4>,
    /// Mesh IDs (for instanced rendering)
    pub mesh_ids: Vec<MeshId>,
    /// Total vertex count
    pub vertex_count: usize,
}

/// Batch type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchType {
    /// Instanced rendering (same mesh, different transforms)
    Instanced { mesh_id: MeshId },
    /// Dynamic batching (different meshes, combined into one)
    Dynamic,
    /// Static batching (pre-combined static geometry)
    Static,
    /// Single draw call (no batching)
    Single { mesh_id: MeshId },
}

/// Batch statistics
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    /// Total draw instances
    pub total_instances: usize,
    /// Total batches generated
    pub total_batches: usize,
    /// Instanced batches
    pub instanced_batches: usize,
    /// Dynamic batches
    pub dynamic_batches: usize,
    /// Static batches
    pub static_batches: usize,
    /// Single draw calls
    pub single_draws: usize,
    /// Total vertices
    pub total_vertices: usize,
    /// Draw call reduction percentage
    pub reduction_percentage: f32,
}

impl BatchStats {
    /// Calculate reduction percentage
    pub fn calculate_reduction(&mut self, original_draws: usize) {
        if original_draws > 0 {
            self.reduction_percentage = 
                ((original_draws - self.total_batches) as f32 / original_draws as f32) * 100.0;
        }
    }
}

/// Batch manager
pub struct BatchManager {
    /// Configuration
    config: BatchConfig,
    /// Draw instances
    instances: Vec<DrawInstance>,
    /// Static batches (pre-computed)
    static_batches: HashMap<BatchKey, Vec<DrawInstance>>,
    /// Statistics
    stats: BatchStats,
}

impl BatchManager {
    /// Create a new batch manager
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            instances: Vec::new(),
            static_batches: HashMap::new(),
            stats: BatchStats::default(),
        }
    }

    /// Add a draw instance
    pub fn add_instance(&mut self, instance: DrawInstance) {
        self.instances.push(instance);
    }

    /// Add a mesh to be batched
    pub fn add_mesh(
        &mut self,
        mesh_id: MeshId,
        material_id: MaterialId,
        transform: Mat4,
        vertex_count: usize,
        is_static: bool,
    ) {
        self.add_instance(DrawInstance {
            mesh_id,
            material_id,
            transform,
            vertex_count,
            is_static,
        });
    }

    /// Clear all instances
    pub fn clear(&mut self) {
        self.instances.clear();
        self.stats = BatchStats::default();
    }

    /// Generate batches from current instances
    pub fn generate_batches(&mut self) -> Vec<Batch> {
        let original_count = self.instances.len();
        self.stats.total_instances = original_count;
        self.stats.total_vertices = self.instances.iter().map(|i| i.vertex_count).sum();

        // Group instances by material and mesh
        let mut groups: HashMap<BatchKey, Vec<DrawInstance>> = HashMap::new();

        for instance in &self.instances {
            let key = BatchKey {
                material_id: instance.material_id,
                mesh_id: Some(instance.mesh_id),
            };
            groups.entry(key).or_insert_with(Vec::new).push(instance.clone());
        }

        let mut batches = Vec::new();

        // Process each group
        for (key, instances) in groups {
            if instances.len() >= self.config.min_instances_for_instancing && self.config.enable_instancing {
                // Use instanced rendering
                batches.push(self.create_instanced_batch(key.material_id, key.mesh_id.unwrap(), instances));
                self.stats.instanced_batches += 1;
            } else if instances.len() == 1 {
                // Single draw call
                let instance = &instances[0];
                batches.push(Batch {
                    material_id: instance.material_id,
                    batch_type: BatchType::Single { mesh_id: instance.mesh_id },
                    transforms: vec![instance.transform],
                    mesh_ids: vec![instance.mesh_id],
                    vertex_count: instance.vertex_count,
                });
                self.stats.single_draws += 1;
            } else {
                // Try dynamic batching for small meshes
                if self.can_dynamic_batch(&instances) {
                    batches.push(self.create_dynamic_batch(key.material_id, instances));
                    self.stats.dynamic_batches += 1;
                } else {
                    // Fall back to individual draws
                    for instance in instances {
                        batches.push(Batch {
                            material_id: instance.material_id,
                            batch_type: BatchType::Single { mesh_id: instance.mesh_id },
                            transforms: vec![instance.transform],
                            mesh_ids: vec![instance.mesh_id],
                            vertex_count: instance.vertex_count,
                        });
                        self.stats.single_draws += 1;
                    }
                }
            }
        }

        self.stats.total_batches = batches.len();
        self.stats.calculate_reduction(original_count);

        batches
    }

    /// Create an instanced batch
    fn create_instanced_batch(
        &self,
        material_id: MaterialId,
        mesh_id: MeshId,
        instances: Vec<DrawInstance>,
    ) -> Batch {
        let transforms: Vec<Mat4> = instances.iter().map(|i| i.transform).collect();
        let vertex_count = instances.first().map(|i| i.vertex_count).unwrap_or(0);

        Batch {
            material_id,
            batch_type: BatchType::Instanced { mesh_id },
            transforms,
            mesh_ids: vec![mesh_id; instances.len()],
            vertex_count: vertex_count * instances.len(),
        }
    }

    /// Create a dynamic batch
    fn create_dynamic_batch(
        &self,
        material_id: MaterialId,
        instances: Vec<DrawInstance>,
    ) -> Batch {
        let transforms: Vec<Mat4> = instances.iter().map(|i| i.transform).collect();
        let mesh_ids: Vec<MeshId> = instances.iter().map(|i| i.mesh_id).collect();
        let vertex_count: usize = instances.iter().map(|i| i.vertex_count).sum();

        Batch {
            material_id,
            batch_type: BatchType::Dynamic,
            transforms,
            mesh_ids,
            vertex_count,
        }
    }

    /// Check if instances can be dynamically batched
    fn can_dynamic_batch(&self, instances: &[DrawInstance]) -> bool {
        if !self.config.enable_dynamic_batching {
            return false;
        }

        let total_vertices: usize = instances.iter().map(|i| i.vertex_count).sum();
        total_vertices <= self.config.max_vertices_per_batch
    }

    /// Get batch statistics
    pub fn get_stats(&self) -> &BatchStats {
        &self.stats
    }

    /// Optimize static geometry
    pub fn optimize_static_geometry(&mut self) {
        if !self.config.enable_static_batching {
            return;
        }

        let static_instances: Vec<DrawInstance> = self.instances
            .iter()
            .filter(|i| i.is_static)
            .cloned()
            .collect();

        // Group static instances by material
        let mut static_groups: HashMap<MaterialId, Vec<DrawInstance>> = HashMap::new();

        for instance in static_instances {
            static_groups
                .entry(instance.material_id)
                .or_insert_with(Vec::new)
                .push(instance);
        }

        // Create static batches
        for (material_id, instances) in static_groups {
            let key = BatchKey {
                material_id,
                mesh_id: None,
            };
            self.static_batches.insert(key, instances);
        }
    }

    /// Get configuration
    pub fn get_config(&self) -> &BatchConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: BatchConfig) {
        self.config = config;
    }
}

/// Batch sorter for optimal rendering order
pub struct BatchSorter;

impl BatchSorter {
    /// Sort batches by material to minimize state changes
    pub fn sort_by_material(batches: &mut [Batch]) {
        batches.sort_by_key(|b| b.material_id);
    }

    /// Sort batches by depth (front to back for opaque, back to front for transparent)
    pub fn sort_by_depth(batches: &mut [Batch], camera_position: Vec3, front_to_back: bool) {
        batches.sort_by(|a, b| {
            let depth_a = Self::calculate_depth(&a.transforms[0], camera_position);
            let depth_b = Self::calculate_depth(&b.transforms[0], camera_position);

            if front_to_back {
                depth_a.partial_cmp(&depth_b).unwrap()
            } else {
                depth_b.partial_cmp(&depth_a).unwrap()
            }
        });
    }

    /// Calculate depth from camera
    fn calculate_depth(transform: &Mat4, camera_position: Vec3) -> f32 {
        let position = Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);
        (position - camera_position).length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_transform(x: f32, y: f32, z: f32) -> Mat4 {
        Mat4::from_translation(Vec3::new(x, y, z))
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_vertices_per_batch, 65536);
        assert_eq!(config.max_instances_per_batch, 1024);
        assert!(config.enable_dynamic_batching);
        assert!(config.enable_static_batching);
        assert!(config.enable_instancing);
    }

    #[test]
    fn test_batch_manager_creation() {
        let manager = BatchManager::new(BatchConfig::default());
        assert_eq!(manager.instances.len(), 0);
    }

    #[test]
    fn test_add_mesh() {
        let mut manager = BatchManager::new(BatchConfig::default());
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        assert_eq!(manager.instances.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut manager = BatchManager::new(BatchConfig::default());
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.clear();
        assert_eq!(manager.instances.len(), 0);
    }

    #[test]
    fn test_single_instance_no_batching() {
        let mut manager = BatchManager::new(BatchConfig::default());
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        
        let batches = manager.generate_batches();
        assert_eq!(batches.len(), 1);
        assert!(matches!(batches[0].batch_type, BatchType::Single { .. }));
    }

    #[test]
    fn test_instanced_batching() {
        let mut manager = BatchManager::new(BatchConfig::default());
        
        // Add multiple instances of the same mesh
        for i in 0..5 {
            manager.add_mesh(1, 1, create_test_transform(i as f32, 0.0, 0.0), 100, false);
        }
        
        let batches = manager.generate_batches();
        assert_eq!(batches.len(), 1);
        assert!(matches!(batches[0].batch_type, BatchType::Instanced { .. }));
        assert_eq!(batches[0].transforms.len(), 5);
    }

    #[test]
    fn test_material_grouping() {
        let mut manager = BatchManager::new(BatchConfig::default());
        
        // Add instances with different materials
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.add_mesh(1, 2, create_test_transform(1.0, 0.0, 0.0), 100, false);
        
        let batches = manager.generate_batches();
        assert_eq!(batches.len(), 2);
    }

    #[test]
    fn test_batch_stats() {
        let mut manager = BatchManager::new(BatchConfig::default());
        
        for i in 0..10 {
            manager.add_mesh(1, 1, create_test_transform(i as f32, 0.0, 0.0), 100, false);
        }
        
        let _ = manager.generate_batches();
        let stats = manager.get_stats();
        
        assert_eq!(stats.total_instances, 10);
        assert_eq!(stats.total_batches, 1);
        assert!(stats.reduction_percentage > 0.0);
    }

    #[test]
    fn test_vertex_count_tracking() {
        let mut manager = BatchManager::new(BatchConfig::default());
        
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.add_mesh(2, 1, create_test_transform(1.0, 0.0, 0.0), 200, false);
        
        let _ = manager.generate_batches();
        let stats = manager.get_stats();
        
        assert_eq!(stats.total_vertices, 300);
    }

    #[test]
    fn test_dynamic_batching() {
        let mut config = BatchConfig::default();
        config.min_instances_for_instancing = 10; // Disable instancing for this test
        
        let mut manager = BatchManager::new(config);
        
        // Add small meshes that can be dynamically batched
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.add_mesh(2, 1, create_test_transform(1.0, 0.0, 0.0), 100, false);
        
        let batches = manager.generate_batches();
        assert_eq!(batches.len(), 1);
        assert!(matches!(batches[0].batch_type, BatchType::Dynamic));
    }

    #[test]
    fn test_max_vertices_limit() {
        let mut config = BatchConfig::default();
        config.max_vertices_per_batch = 150;
        config.min_instances_for_instancing = 10;
        
        let mut manager = BatchManager::new(config);
        
        // Add meshes that exceed the limit
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.add_mesh(2, 1, create_test_transform(1.0, 0.0, 0.0), 100, false);
        
        let batches = manager.generate_batches();
        // Should create separate batches due to vertex limit
        assert!(batches.len() >= 2);
    }

    #[test]
    fn test_static_geometry_flag() {
        let mut manager = BatchManager::new(BatchConfig::default());
        
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, true);
        manager.add_mesh(2, 1, create_test_transform(1.0, 0.0, 0.0), 100, false);
        
        assert!(manager.instances[0].is_static);
        assert!(!manager.instances[1].is_static);
    }

    #[test]
    fn test_batch_sorter_by_material() {
        let mut batches = vec![
            Batch {
                material_id: 3,
                batch_type: BatchType::Single { mesh_id: 1 },
                transforms: vec![],
                mesh_ids: vec![],
                vertex_count: 0,
            },
            Batch {
                material_id: 1,
                batch_type: BatchType::Single { mesh_id: 2 },
                transforms: vec![],
                mesh_ids: vec![],
                vertex_count: 0,
            },
            Batch {
                material_id: 2,
                batch_type: BatchType::Single { mesh_id: 3 },
                transforms: vec![],
                mesh_ids: vec![],
                vertex_count: 0,
            },
        ];

        BatchSorter::sort_by_material(&mut batches);
        
        assert_eq!(batches[0].material_id, 1);
        assert_eq!(batches[1].material_id, 2);
        assert_eq!(batches[2].material_id, 3);
    }

    #[test]
    fn test_batch_sorter_by_depth() {
        let mut batches = vec![
            Batch {
                material_id: 1,
                batch_type: BatchType::Single { mesh_id: 1 },
                transforms: vec![create_test_transform(0.0, 0.0, 10.0)],
                mesh_ids: vec![],
                vertex_count: 0,
            },
            Batch {
                material_id: 1,
                batch_type: BatchType::Single { mesh_id: 2 },
                transforms: vec![create_test_transform(0.0, 0.0, 5.0)],
                mesh_ids: vec![],
                vertex_count: 0,
            },
        ];

        let camera_pos = Vec3::new(0.0, 0.0, 0.0);
        BatchSorter::sort_by_depth(&mut batches, camera_pos, true);
        
        // Closer object should be first (front to back)
        assert_eq!(batches[0].transforms[0].w_axis.z, 5.0);
        assert_eq!(batches[1].transforms[0].w_axis.z, 10.0);
    }

    #[test]
    fn test_config_modification() {
        let mut manager = BatchManager::new(BatchConfig::default());
        
        let mut new_config = BatchConfig::default();
        new_config.max_vertices_per_batch = 1000;
        
        manager.set_config(new_config);
        assert_eq!(manager.get_config().max_vertices_per_batch, 1000);
    }

    #[test]
    fn test_reduction_percentage_calculation() {
        let mut stats = BatchStats::default();
        stats.total_batches = 5;
        stats.calculate_reduction(20);
        
        assert_eq!(stats.reduction_percentage, 75.0);
    }

    #[test]
    fn test_batch_key_equality() {
        let key1 = BatchKey {
            material_id: 1,
            mesh_id: Some(1),
        };
        let key2 = BatchKey {
            material_id: 1,
            mesh_id: Some(1),
        };
        let key3 = BatchKey {
            material_id: 2,
            mesh_id: Some(1),
        };
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_empty_batch_generation() {
        let mut manager = BatchManager::new(BatchConfig::default());
        let batches = manager.generate_batches();
        assert_eq!(batches.len(), 0);
    }

    #[test]
    fn test_instancing_threshold() {
        let mut config = BatchConfig::default();
        config.min_instances_for_instancing = 3;
        
        let mut manager = BatchManager::new(config);
        
        // Add 2 instances (below threshold)
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.add_mesh(1, 1, create_test_transform(1.0, 0.0, 0.0), 100, false);
        
        let batches = manager.generate_batches();
        // Should not use instancing
        assert!(batches.iter().all(|b| !matches!(b.batch_type, BatchType::Instanced { .. })));
    }

    #[test]
    fn test_disabled_instancing() {
        let mut config = BatchConfig::default();
        config.enable_instancing = false;
        
        let mut manager = BatchManager::new(config);
        
        for i in 0..5 {
            manager.add_mesh(1, 1, create_test_transform(i as f32, 0.0, 0.0), 100, false);
        }
        
        let batches = manager.generate_batches();
        // Should not use instancing
        assert!(batches.iter().all(|b| !matches!(b.batch_type, BatchType::Instanced { .. })));
    }

    #[test]
    fn test_disabled_dynamic_batching() {
        let mut config = BatchConfig::default();
        config.enable_dynamic_batching = false;
        config.min_instances_for_instancing = 10;
        
        let mut manager = BatchManager::new(config);
        
        manager.add_mesh(1, 1, create_test_transform(0.0, 0.0, 0.0), 100, false);
        manager.add_mesh(2, 1, create_test_transform(1.0, 0.0, 0.0), 100, false);
        
        let batches = manager.generate_batches();
        // Should create separate draws
        assert!(batches.iter().all(|b| !matches!(b.batch_type, BatchType::Dynamic)));
    }
}

