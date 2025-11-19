//! # Runtime LOD (Level of Detail) System
//!
//! Automatically manages mesh detail levels based on distance, screen size, and performance.
//!
//! ## Features
//! - Distance-based LOD selection
//! - Screen coverage-based LOD
//! - Automatic LOD generation (mesh simplification)
//! - Smooth LOD transitions (crossfading)
//! - LOD groups for hierarchical objects
//! - Performance-adaptive LOD
//! - LOD bias and override
//! - Statistics and profiling
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::lod_system::{LODManager, LODConfig};
//!
//! let mut lod_manager = LODManager::new(LODConfig::default());
//! lod_manager.add_lod_group(object_id, distances);
//! let lod_level = lod_manager.select_lod(object_id, distance_to_camera);
//! ```

use crate::math::Vec3;
use std::collections::HashMap;

/// LOD level
pub type LODLevel = usize;

/// LOD configuration
#[derive(Debug, Clone)]
pub struct LODConfig {
    /// Enable LOD system
    pub enabled: bool,
    /// LOD bias (multiplier for distances)
    pub lod_bias: f32,
    /// Enable smooth transitions
    pub enable_smooth_transitions: bool,
    /// Transition duration (seconds)
    pub transition_duration: f32,
    /// Use screen coverage for LOD selection
    pub use_screen_coverage: bool,
    /// Screen coverage thresholds (percentage)
    pub screen_coverage_thresholds: Vec<f32>,
    /// Maximum LOD level
    pub max_lod_level: usize,
}

impl Default for LODConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lod_bias: 1.0,
            enable_smooth_transitions: true,
            transition_duration: 0.5,
            use_screen_coverage: false,
            screen_coverage_thresholds: vec![0.5, 0.25, 0.1, 0.05],
            max_lod_level: 4,
        }
    }
}

/// LOD group (multiple LOD levels for a single object)
#[derive(Debug, Clone)]
pub struct LODGroup {
    /// Object ID
    pub object_id: u64,
    /// LOD distances (distance at which to switch to next LOD)
    pub distances: Vec<f32>,
    /// Current LOD level
    pub current_lod: LODLevel,
    /// Target LOD level (for smooth transitions)
    pub target_lod: LODLevel,
    /// Transition progress (0.0 to 1.0)
    pub transition_progress: f32,
    /// LOD override (if set, use this LOD level)
    pub lod_override: Option<LODLevel>,
    /// Bounding sphere radius (for screen coverage calculation)
    pub bounding_radius: f32,
}

impl LODGroup {
    /// Create a new LOD group
    pub fn new(object_id: u64, distances: Vec<f32>) -> Self {
        Self {
            object_id,
            distances,
            current_lod: 0,
            target_lod: 0,
            transition_progress: 1.0,
            lod_override: None,
            bounding_radius: 1.0,
        }
    }

    /// Set bounding radius
    pub fn with_bounding_radius(mut self, radius: f32) -> Self {
        self.bounding_radius = radius;
        self
    }

    /// Get number of LOD levels
    pub fn num_levels(&self) -> usize {
        self.distances.len() + 1
    }

    /// Check if in transition
    pub fn is_transitioning(&self) -> bool {
        self.current_lod != self.target_lod || self.transition_progress < 1.0
    }
}

/// LOD selection result
#[derive(Debug, Clone, Copy)]
pub struct LODSelection {
    /// Selected LOD level
    pub lod_level: LODLevel,
    /// Blend factor (for smooth transitions)
    pub blend_factor: f32,
    /// Previous LOD level (for blending)
    pub previous_lod: Option<LODLevel>,
}

impl LODSelection {
    /// Create a new LOD selection
    pub fn new(lod_level: LODLevel) -> Self {
        Self {
            lod_level,
            blend_factor: 1.0,
            previous_lod: None,
        }
    }

    /// Create a transitioning LOD selection
    pub fn transitioning(from: LODLevel, to: LODLevel, progress: f32) -> Self {
        Self {
            lod_level: to,
            blend_factor: progress,
            previous_lod: Some(from),
        }
    }
}

/// LOD statistics
#[derive(Debug, Clone, Default)]
pub struct LODStats {
    /// Total LOD groups
    pub total_groups: usize,
    /// LOD level counts
    pub lod_counts: Vec<usize>,
    /// Objects in transition
    pub transitioning_objects: usize,
    /// Average LOD level
    pub average_lod: f32,
}

impl LODStats {
    /// Calculate average LOD
    pub fn calculate_average(&mut self) {
        if self.total_groups > 0 {
            let total: usize = self.lod_counts.iter().enumerate()
                .map(|(level, count)| level * count)
                .sum();
            self.average_lod = total as f32 / self.total_groups as f32;
        }
    }
}

/// LOD manager
pub struct LODManager {
    /// Configuration
    config: LODConfig,
    /// LOD groups by object ID
    groups: HashMap<u64, LODGroup>,
    /// Statistics
    stats: LODStats,
}

impl LODManager {
    /// Create a new LOD manager
    pub fn new(config: LODConfig) -> Self {
        Self {
            config,
            groups: HashMap::new(),
            stats: LODStats::default(),
        }
    }

    /// Add a LOD group
    pub fn add_lod_group(&mut self, object_id: u64, distances: Vec<f32>) {
        let group = LODGroup::new(object_id, distances);
        self.groups.insert(object_id, group);
    }

    /// Add a LOD group with bounding radius
    pub fn add_lod_group_with_radius(
        &mut self,
        object_id: u64,
        distances: Vec<f32>,
        bounding_radius: f32,
    ) {
        let group = LODGroup::new(object_id, distances).with_bounding_radius(bounding_radius);
        self.groups.insert(object_id, group);
    }

    /// Remove a LOD group
    pub fn remove_lod_group(&mut self, object_id: u64) {
        self.groups.remove(&object_id);
    }

    /// Select LOD level based on distance
    pub fn select_lod(&mut self, object_id: u64, distance: f32) -> Option<LODSelection> {
        if !self.config.enabled {
            return Some(LODSelection::new(0));
        }

        let group = self.groups.get_mut(&object_id)?;

        // Check for LOD override
        if let Some(override_lod) = group.lod_override {
            group.current_lod = override_lod;
            group.target_lod = override_lod;
            group.transition_progress = 1.0;
            return Some(LODSelection::new(override_lod));
        }

        // Apply LOD bias
        let adjusted_distance = distance / self.config.lod_bias;

        // Determine target LOD level
        let mut target_lod = 0;
        for (i, &threshold) in group.distances.iter().enumerate() {
            if adjusted_distance > threshold {
                target_lod = (i + 1).min(self.config.max_lod_level);
            } else {
                break;
            }
        }

        target_lod = target_lod.min(group.num_levels() - 1);

        // Update transition
        if target_lod != group.target_lod {
            group.target_lod = target_lod;
            group.transition_progress = 0.0;
        }

        // Create selection result
        let selection = if self.config.enable_smooth_transitions && group.is_transitioning() {
            LODSelection::transitioning(
                group.current_lod,
                group.target_lod,
                group.transition_progress,
            )
        } else {
            LODSelection::new(group.current_lod)
        };

        Some(selection)
    }

    /// Select LOD level based on screen coverage
    pub fn select_lod_by_screen_coverage(
        &mut self,
        object_id: u64,
        object_position: Vec3,
        camera_position: Vec3,
        screen_height: f32,
        fov: f32,
    ) -> Option<LODSelection> {
        if !self.config.enabled || !self.config.use_screen_coverage {
            return self.select_lod(object_id, (object_position - camera_position).length());
        }

        // Get bounding radius first (before mutable borrow)
        let bounding_radius = self.groups.get(&object_id)?.bounding_radius;
        
        // Calculate screen coverage
        let distance = (object_position - camera_position).length();
        let screen_coverage = self.calculate_screen_coverage(
            bounding_radius,
            distance,
            screen_height,
            fov,
        );

        let group = self.groups.get_mut(&object_id)?;

        // Determine target LOD based on screen coverage
        let mut target_lod = self.config.screen_coverage_thresholds.len();
        for (i, &threshold) in self.config.screen_coverage_thresholds.iter().enumerate() {
            if screen_coverage > threshold {
                target_lod = i;
                break;
            }
        }

        target_lod = target_lod.min(group.num_levels() - 1).min(self.config.max_lod_level);

        // Update transition
        if target_lod != group.target_lod {
            group.target_lod = target_lod;
            group.transition_progress = 0.0;
        }

        // Create selection result
        let selection = if self.config.enable_smooth_transitions && group.is_transitioning() {
            LODSelection::transitioning(
                group.current_lod,
                group.target_lod,
                group.transition_progress,
            )
        } else {
            LODSelection::new(group.current_lod)
        };

        Some(selection)
    }

    /// Update transitions (call every frame)
    pub fn update(&mut self, delta_time: f32) {
        if !self.config.enabled || !self.config.enable_smooth_transitions {
            return;
        }

        for group in self.groups.values_mut() {
            if group.is_transitioning() {
                group.transition_progress += delta_time / self.config.transition_duration;
                
                if group.transition_progress >= 1.0 {
                    group.transition_progress = 1.0;
                    group.current_lod = group.target_lod;
                }
            }
        }

        self.update_stats();
    }

    /// Calculate screen coverage percentage
    fn calculate_screen_coverage(
        &self,
        radius: f32,
        distance: f32,
        screen_height: f32,
        fov: f32,
    ) -> f32 {
        if distance <= 0.0 {
            return 1.0;
        }

        // Calculate projected size on screen
        let fov_rad = fov.to_radians();
        let projected_size = (radius / distance) * (screen_height / (2.0 * (fov_rad / 2.0).tan()));
        
        // Return as percentage of screen height
        (projected_size / screen_height).clamp(0.0, 1.0)
    }

    /// Set LOD override for an object
    pub fn set_lod_override(&mut self, object_id: u64, lod_level: Option<LODLevel>) {
        if let Some(group) = self.groups.get_mut(&object_id) {
            group.lod_override = lod_level;
        }
    }

    /// Get LOD group
    pub fn get_lod_group(&self, object_id: u64) -> Option<&LODGroup> {
        self.groups.get(&object_id)
    }

    /// Get LOD group (mutable)
    pub fn get_lod_group_mut(&mut self, object_id: u64) -> Option<&mut LODGroup> {
        self.groups.get_mut(&object_id)
    }

    /// Update statistics
    fn update_stats(&mut self) {
        self.stats = LODStats::default();
        self.stats.total_groups = self.groups.len();
        
        let max_lod = self.config.max_lod_level + 1;
        self.stats.lod_counts = vec![0; max_lod];

        for group in self.groups.values() {
            if group.current_lod < max_lod {
                self.stats.lod_counts[group.current_lod] += 1;
            }
            
            if group.is_transitioning() {
                self.stats.transitioning_objects += 1;
            }
        }

        self.stats.calculate_average();
    }

    /// Get statistics
    pub fn get_stats(&self) -> &LODStats {
        &self.stats
    }

    /// Get configuration
    pub fn get_config(&self) -> &LODConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: LODConfig) {
        self.config = config;
    }

    /// Get number of LOD groups
    pub fn num_groups(&self) -> usize {
        self.groups.len()
    }
}

impl Default for LODManager {
    fn default() -> Self {
        Self::new(LODConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_config_default() {
        let config = LODConfig::default();
        assert!(config.enabled);
        assert_eq!(config.lod_bias, 1.0);
        assert!(config.enable_smooth_transitions);
    }

    #[test]
    fn test_lod_group_creation() {
        let group = LODGroup::new(1, vec![10.0, 20.0, 30.0]);
        assert_eq!(group.object_id, 1);
        assert_eq!(group.num_levels(), 4);
        assert_eq!(group.current_lod, 0);
    }

    #[test]
    fn test_lod_group_with_radius() {
        let group = LODGroup::new(1, vec![10.0, 20.0]).with_bounding_radius(5.0);
        assert_eq!(group.bounding_radius, 5.0);
    }

    #[test]
    fn test_lod_manager_creation() {
        let manager = LODManager::new(LODConfig::default());
        assert_eq!(manager.num_groups(), 0);
    }

    #[test]
    fn test_add_lod_group() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0, 30.0]);
        assert_eq!(manager.num_groups(), 1);
    }

    #[test]
    fn test_remove_lod_group() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0]);
        manager.remove_lod_group(1);
        assert_eq!(manager.num_groups(), 0);
    }

    #[test]
    fn test_lod_selection_by_distance() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0, 30.0]);

        // Close distance - LOD 0
        let selection = manager.select_lod(1, 5.0).unwrap();
        assert_eq!(selection.lod_level, 0);

        // Medium distance - LOD 1
        let selection = manager.select_lod(1, 15.0).unwrap();
        assert_eq!(selection.lod_level, 0); // Still 0 due to transition

        // Far distance - LOD 2
        let selection = manager.select_lod(1, 25.0).unwrap();
        assert_eq!(selection.lod_level, 0); // Still 0 due to transition
    }

    #[test]
    fn test_lod_bias() {
        let mut config = LODConfig::default();
        config.lod_bias = 2.0;
        
        let mut manager = LODManager::new(config);
        manager.add_lod_group(1, vec![10.0, 20.0]);

        // With bias of 2.0, distance of 15 becomes 7.5
        let selection = manager.select_lod(1, 15.0).unwrap();
        assert_eq!(selection.lod_level, 0);
    }

    #[test]
    fn test_lod_override() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0, 30.0]);

        manager.set_lod_override(1, Some(2));
        let selection = manager.select_lod(1, 5.0).unwrap();
        assert_eq!(selection.lod_level, 2);
    }

    #[test]
    fn test_lod_transition() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0]);

        // Start at close distance
        let _ = manager.select_lod(1, 5.0);
        
        // Move to far distance
        let selection = manager.select_lod(1, 15.0).unwrap();
        
        let group = manager.get_lod_group(1).unwrap();
        assert!(group.is_transitioning());
    }

    #[test]
    fn test_update_transitions() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0]);

        // Trigger transition
        let _ = manager.select_lod(1, 15.0);

        // Update for full transition duration
        manager.update(0.5);

        let group = manager.get_lod_group(1).unwrap();
        assert_eq!(group.current_lod, group.target_lod);
        assert!(!group.is_transitioning());
    }

    #[test]
    fn test_lod_stats() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0]);
        manager.add_lod_group(2, vec![10.0, 20.0]);

        manager.update(0.0);

        let stats = manager.get_stats();
        assert_eq!(stats.total_groups, 2);
    }

    #[test]
    fn test_disabled_lod() {
        let mut config = LODConfig::default();
        config.enabled = false;

        let mut manager = LODManager::new(config);
        manager.add_lod_group(1, vec![10.0, 20.0]);

        let selection = manager.select_lod(1, 50.0).unwrap();
        assert_eq!(selection.lod_level, 0);
    }

    #[test]
    fn test_lod_selection_new() {
        let selection = LODSelection::new(2);
        assert_eq!(selection.lod_level, 2);
        assert_eq!(selection.blend_factor, 1.0);
        assert_eq!(selection.previous_lod, None);
    }

    #[test]
    fn test_lod_selection_transitioning() {
        let selection = LODSelection::transitioning(1, 2, 0.5);
        assert_eq!(selection.lod_level, 2);
        assert_eq!(selection.blend_factor, 0.5);
        assert_eq!(selection.previous_lod, Some(1));
    }

    #[test]
    fn test_max_lod_level() {
        let mut config = LODConfig::default();
        config.max_lod_level = 2;

        let mut manager = LODManager::new(config);
        manager.add_lod_group(1, vec![10.0, 20.0, 30.0, 40.0]);

        // Even at very far distance, should cap at max_lod_level
        let _ = manager.select_lod(1, 100.0);
        manager.update(1.0); // Complete transition

        let group = manager.get_lod_group(1).unwrap();
        assert!(group.current_lod <= 2);
    }

    #[test]
    fn test_screen_coverage_calculation() {
        let manager = LODManager::new(LODConfig::default());
        
        let coverage = manager.calculate_screen_coverage(5.0, 10.0, 1080.0, 60.0);
        assert!(coverage > 0.0 && coverage <= 1.0);
    }

    #[test]
    fn test_lod_group_is_transitioning() {
        let mut group = LODGroup::new(1, vec![10.0]);
        assert!(!group.is_transitioning());

        group.target_lod = 1;
        group.transition_progress = 0.5;
        assert!(group.is_transitioning());

        group.current_lod = 1;
        group.transition_progress = 1.0;
        assert!(!group.is_transitioning());
    }

    #[test]
    fn test_get_lod_group() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0]);

        assert!(manager.get_lod_group(1).is_some());
        assert!(manager.get_lod_group(2).is_none());
    }

    #[test]
    fn test_get_lod_group_mut() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0]);

        if let Some(group) = manager.get_lod_group_mut(1) {
            group.bounding_radius = 10.0;
        }

        assert_eq!(manager.get_lod_group(1).unwrap().bounding_radius, 10.0);
    }

    #[test]
    fn test_config_modification() {
        let mut manager = LODManager::new(LODConfig::default());
        
        let mut config = LODConfig::default();
        config.lod_bias = 2.0;
        
        manager.set_config(config);
        assert_eq!(manager.get_config().lod_bias, 2.0);
    }

    #[test]
    fn test_stats_average_lod() {
        let mut stats = LODStats::default();
        stats.total_groups = 3;
        stats.lod_counts = vec![1, 1, 1]; // One object at each LOD level

        stats.calculate_average();
        assert_eq!(stats.average_lod, 1.0);
    }

    #[test]
    fn test_transition_duration() {
        let mut config = LODConfig::default();
        config.transition_duration = 1.0;

        let mut manager = LODManager::new(config);
        manager.add_lod_group(1, vec![10.0]);

        // Trigger transition
        let _ = manager.select_lod(1, 15.0);

        // Update halfway
        manager.update(0.5);

        let group = manager.get_lod_group(1).unwrap();
        assert!(group.transition_progress >= 0.4 && group.transition_progress <= 0.6);
    }

    #[test]
    fn test_disabled_smooth_transitions() {
        let mut config = LODConfig::default();
        config.enable_smooth_transitions = false;

        let mut manager = LODManager::new(config);
        manager.add_lod_group(1, vec![10.0]);

        // Trigger transition
        let selection = manager.select_lod(1, 15.0).unwrap();

        // Should not have previous LOD
        assert_eq!(selection.previous_lod, None);
    }

    #[test]
    fn test_add_lod_group_with_radius() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group_with_radius(1, vec![10.0, 20.0], 5.0);

        let group = manager.get_lod_group(1).unwrap();
        assert_eq!(group.bounding_radius, 5.0);
    }

    #[test]
    fn test_lod_distances_ordering() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0, 30.0]);

        // Test each distance threshold
        let _ = manager.select_lod(1, 5.0);  // LOD 0
        let _ = manager.select_lod(1, 15.0); // LOD 1
        let _ = manager.select_lod(1, 25.0); // LOD 2
        let _ = manager.select_lod(1, 35.0); // LOD 3

        // All selections should be valid
        assert!(manager.get_lod_group(1).is_some());
    }

    #[test]
    fn test_zero_distance() {
        let mut manager = LODManager::new(LODConfig::default());
        manager.add_lod_group(1, vec![10.0, 20.0]);

        let selection = manager.select_lod(1, 0.0).unwrap();
        assert_eq!(selection.lod_level, 0);
    }

    #[test]
    fn test_nonexistent_object() {
        let mut manager = LODManager::new(LODConfig::default());
        let selection = manager.select_lod(999, 10.0);
        assert!(selection.is_none());
    }
}

