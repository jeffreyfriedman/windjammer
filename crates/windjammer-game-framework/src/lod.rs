//! Level of Detail (LOD) system for Windjammer games
//!
//! This module provides automatic LOD management for 3D meshes,
//! allowing games to render high-detail meshes up close and
//! lower-detail meshes at a distance for better performance.
//!
//! **Philosophy**: Zero crate leakage - no wgpu or mesh processing types exposed.

use crate::math::Vec3;

/// A single level of detail for a mesh
#[derive(Clone, Debug)]
pub struct LODLevel {
    /// Minimum distance from camera (in world units)
    pub min_distance: f32,
    /// Maximum distance from camera (in world units)
    pub max_distance: f32,
    /// Scale factor for this LOD (1.0 = full detail, 0.5 = half detail, etc.)
    pub detail_scale: f32,
}

impl LODLevel {
    /// Create a new LOD level
    pub fn new(min_distance: f32, max_distance: f32, detail_scale: f32) -> Self {
        Self {
            min_distance,
            max_distance,
            detail_scale,
        }
    }
}

/// LOD configuration for automatic level generation
#[derive(Clone, Debug)]
pub struct LODConfig {
    /// Number of LOD levels to generate
    pub num_levels: usize,
    /// Distance multiplier between levels (e.g., 2.0 = each level is 2x farther)
    pub distance_multiplier: f32,
    /// Detail reduction per level (e.g., 0.5 = each level has half the detail)
    pub detail_reduction: f32,
    /// Base distance for LOD 0 (highest detail)
    pub base_distance: f32,
}

impl Default for LODConfig {
    fn default() -> Self {
        Self {
            num_levels: 4,
            distance_multiplier: 2.0,
            detail_reduction: 0.5,
            base_distance: 10.0,
        }
    }
}

impl LODConfig {
    /// Generate LOD levels from this configuration
    pub fn generate_levels(&self) -> Vec<LODLevel> {
        let mut levels = Vec::with_capacity(self.num_levels);

        for i in 0..self.num_levels {
            let min_distance = if i == 0 {
                0.0
            } else {
                self.base_distance * self.distance_multiplier.powi(i as i32 - 1)
            };

            let max_distance = if i == self.num_levels - 1 {
                f32::INFINITY
            } else {
                self.base_distance * self.distance_multiplier.powi(i as i32)
            };

            let detail_scale = self.detail_reduction.powi(i as i32);

            levels.push(LODLevel::new(min_distance, max_distance, detail_scale));
        }

        levels
    }
}

/// LOD selector for choosing appropriate detail level
pub struct LODSelector {
    levels: Vec<LODLevel>,
}

impl LODSelector {
    /// Create a new LOD selector with the given levels
    pub fn new(levels: Vec<LODLevel>) -> Self {
        Self { levels }
    }

    /// Create a LOD selector from a configuration
    pub fn from_config(config: &LODConfig) -> Self {
        Self::new(config.generate_levels())
    }

    /// Select the appropriate LOD level based on distance from camera
    ///
    /// Returns the index of the selected LOD level.
    pub fn select_lod(&self, distance: f32) -> usize {
        for (i, level) in self.levels.iter().enumerate() {
            if distance >= level.min_distance && distance < level.max_distance {
                return i;
            }
        }

        // Fallback to lowest detail
        self.levels.len().saturating_sub(1)
    }

    /// Get the detail scale for a given distance
    pub fn get_detail_scale(&self, distance: f32) -> f32 {
        let lod_index = self.select_lod(distance);
        self.levels
            .get(lod_index)
            .map(|level| level.detail_scale)
            .unwrap_or(1.0)
    }

    /// Calculate distance from camera to a point
    pub fn calculate_distance(camera_pos: Vec3, object_pos: Vec3) -> f32 {
        (object_pos - camera_pos).length()
    }

    /// Get the number of LOD levels
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    /// Get a specific LOD level
    pub fn get_level(&self, index: usize) -> Option<&LODLevel> {
        self.levels.get(index)
    }
}

/// LOD statistics for debugging and profiling
#[derive(Default, Clone, Debug)]
pub struct LODStats {
    /// Number of objects rendered at each LOD level
    pub objects_per_lod: Vec<usize>,
    /// Total objects rendered
    pub total_objects: usize,
    /// Average LOD level (0 = highest detail)
    pub average_lod: f32,
}

impl LODStats {
    /// Create new LOD statistics
    pub fn new(num_levels: usize) -> Self {
        Self {
            objects_per_lod: vec![0; num_levels],
            total_objects: 0,
            average_lod: 0.0,
        }
    }

    /// Record an object at a specific LOD level
    pub fn record(&mut self, lod_level: usize) {
        if lod_level < self.objects_per_lod.len() {
            self.objects_per_lod[lod_level] += 1;
        }
        self.total_objects += 1;
    }

    /// Calculate statistics
    pub fn calculate(&mut self) {
        if self.total_objects == 0 {
            self.average_lod = 0.0;
            return;
        }

        let mut weighted_sum = 0.0;
        for (lod_level, &count) in self.objects_per_lod.iter().enumerate() {
            weighted_sum += lod_level as f32 * count as f32;
        }

        self.average_lod = weighted_sum / self.total_objects as f32;
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        for count in &mut self.objects_per_lod {
            *count = 0;
        }
        self.total_objects = 0;
        self.average_lod = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_config_default() {
        let config = LODConfig::default();
        assert_eq!(config.num_levels, 4);
        assert_eq!(config.distance_multiplier, 2.0);
        assert_eq!(config.detail_reduction, 0.5);
    }

    #[test]
    fn test_lod_level_generation() {
        let config = LODConfig::default();
        let levels = config.generate_levels();

        assert_eq!(levels.len(), 4);

        // LOD 0: 0 to 10
        assert_eq!(levels[0].min_distance, 0.0);
        assert_eq!(levels[0].max_distance, 10.0);
        assert_eq!(levels[0].detail_scale, 1.0);

        // LOD 1: 10 to 20
        assert_eq!(levels[1].min_distance, 10.0);
        assert_eq!(levels[1].max_distance, 20.0);
        assert_eq!(levels[1].detail_scale, 0.5);

        // LOD 3: 40 to infinity
        assert_eq!(levels[3].min_distance, 40.0);
        assert!(levels[3].max_distance.is_infinite());
    }

    #[test]
    fn test_lod_selection() {
        let config = LODConfig::default();
        let selector = LODSelector::from_config(&config);

        assert_eq!(selector.select_lod(5.0), 0); // Close: LOD 0
        assert_eq!(selector.select_lod(15.0), 1); // Medium: LOD 1
        assert_eq!(selector.select_lod(35.0), 2); // Far: LOD 2
        assert_eq!(selector.select_lod(100.0), 3); // Very far: LOD 3
    }

    #[test]
    fn test_lod_stats() {
        let mut stats = LODStats::new(4);

        stats.record(0);
        stats.record(0);
        stats.record(1);
        stats.record(2);

        stats.calculate();

        assert_eq!(stats.total_objects, 4);
        assert_eq!(stats.objects_per_lod[0], 2);
        assert_eq!(stats.objects_per_lod[1], 1);
        assert_eq!(stats.objects_per_lod[2], 1);
        assert_eq!(stats.average_lod, 0.75); // (0+0+1+2) / 4
    }
}
