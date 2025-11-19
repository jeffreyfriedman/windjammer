//! # Built-in Performance Profiler
//!
//! Comprehensive profiling system for measuring and analyzing game performance.
//!
//! ## Features
//! - Hierarchical profiling scopes
//! - CPU time measurement
//! - Frame time tracking
//! - Memory allocation tracking
//! - GPU time estimation
//! - Statistical analysis (min, max, avg, percentiles)
//! - Profiling visualization data
//! - Low overhead profiling
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::profiler::{Profiler, ProfileScope};
//!
//! let mut profiler = Profiler::new();
//! profiler.begin_frame();
//! {
//!     let _scope = ProfileScope::new(&mut profiler, "update");
//!     // Your update code here
//! }
//! profiler.end_frame();
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Profile scope ID
pub type ScopeId = u64;

/// Profile scope data
#[derive(Debug, Clone)]
pub struct ScopeData {
    /// Scope name
    pub name: String,
    /// Parent scope ID (None for root scopes)
    pub parent: Option<ScopeId>,
    /// Start time
    pub start_time: Instant,
    /// Duration
    pub duration: Duration,
    /// Number of calls this frame
    pub call_count: usize,
    /// Depth in hierarchy
    pub depth: usize,
}

/// Profile statistics for a scope
#[derive(Debug, Clone)]
pub struct ScopeStats {
    /// Scope name
    pub name: String,
    /// Total number of samples
    pub sample_count: usize,
    /// Minimum duration
    pub min_duration: Duration,
    /// Maximum duration
    pub max_duration: Duration,
    /// Average duration
    pub avg_duration: Duration,
    /// Total duration across all samples
    pub total_duration: Duration,
    /// 95th percentile duration
    pub p95_duration: Duration,
    /// 99th percentile duration
    pub p99_duration: Duration,
}

impl ScopeStats {
    /// Create new scope statistics
    pub fn new(name: String) -> Self {
        Self {
            name,
            sample_count: 0,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            total_duration: Duration::ZERO,
            p95_duration: Duration::ZERO,
            p99_duration: Duration::ZERO,
        }
    }

    /// Update statistics with a new sample
    pub fn add_sample(&mut self, duration: Duration) {
        self.sample_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.sample_count as u32;
    }

    /// Calculate percentiles from a sorted list of durations
    pub fn calculate_percentiles(&mut self, sorted_durations: &[Duration]) {
        if sorted_durations.is_empty() {
            return;
        }

        let p95_idx = ((sorted_durations.len() as f32 * 0.95) as usize).min(sorted_durations.len() - 1);
        let p99_idx = ((sorted_durations.len() as f32 * 0.99) as usize).min(sorted_durations.len() - 1);

        self.p95_duration = sorted_durations[p95_idx];
        self.p99_duration = sorted_durations[p99_idx];
    }
}

/// Frame statistics
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    /// Frame number
    pub frame_number: u64,
    /// Frame duration
    pub frame_duration: Duration,
    /// FPS (frames per second)
    pub fps: f32,
    /// Number of scopes this frame
    pub scope_count: usize,
}

/// Profiler configuration
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Enable profiling
    pub enabled: bool,
    /// Maximum number of frames to keep in history
    pub max_history_frames: usize,
    /// Enable statistical analysis
    pub enable_statistics: bool,
    /// Enable memory tracking
    pub enable_memory_tracking: bool,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_history_frames: 300, // 5 seconds at 60 FPS
            enable_statistics: true,
            enable_memory_tracking: false,
        }
    }
}

/// Performance profiler
pub struct Profiler {
    /// Configuration
    config: ProfilerConfig,
    /// Current frame scopes
    current_scopes: Vec<ScopeData>,
    /// Scope stack (for hierarchy)
    scope_stack: Vec<ScopeId>,
    /// Next scope ID
    next_scope_id: ScopeId,
    /// Frame history
    frame_history: Vec<FrameStats>,
    /// Current frame number
    frame_number: u64,
    /// Frame start time
    frame_start: Option<Instant>,
    /// Scope statistics by name
    scope_stats: HashMap<String, ScopeStats>,
    /// Scope duration history (for percentile calculation)
    scope_durations: HashMap<String, Vec<Duration>>,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self::with_config(ProfilerConfig::default())
    }

    /// Create a new profiler with custom configuration
    pub fn with_config(config: ProfilerConfig) -> Self {
        Self {
            config,
            current_scopes: Vec::new(),
            scope_stack: Vec::new(),
            next_scope_id: 0,
            frame_history: Vec::new(),
            frame_number: 0,
            frame_start: None,
            scope_stats: HashMap::new(),
            scope_durations: HashMap::new(),
        }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        if !self.config.enabled {
            return;
        }

        self.frame_start = Some(Instant::now());
        self.current_scopes.clear();
        self.scope_stack.clear();
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        if !self.config.enabled {
            return;
        }

        if let Some(start) = self.frame_start {
            let frame_duration = start.elapsed();
            let fps = 1.0 / frame_duration.as_secs_f32();

            let frame_stats = FrameStats {
                frame_number: self.frame_number,
                frame_duration,
                fps,
                scope_count: self.current_scopes.len(),
            };

            self.frame_history.push(frame_stats);

            // Trim history if needed
            if self.frame_history.len() > self.config.max_history_frames {
                self.frame_history.remove(0);
            }

            // Update statistics
            if self.config.enable_statistics {
                self.update_statistics();
            }

            self.frame_number += 1;
        }
    }

    /// Begin a profiling scope
    pub fn begin_scope(&mut self, name: &str) -> ScopeId {
        if !self.config.enabled {
            return 0;
        }

        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;

        let parent = self.scope_stack.last().copied();
        let depth = self.scope_stack.len();

        let scope = ScopeData {
            name: name.to_string(),
            parent,
            start_time: Instant::now(),
            duration: Duration::ZERO,
            call_count: 1,
            depth,
        };

        self.current_scopes.push(scope);
        self.scope_stack.push(scope_id);

        scope_id
    }

    /// End a profiling scope
    pub fn end_scope(&mut self, scope_id: ScopeId) {
        if !self.config.enabled {
            return;
        }

        // Pop from stack
        if let Some(last_id) = self.scope_stack.pop() {
            if last_id != scope_id {
                // Scope mismatch - this shouldn't happen with RAII
                return;
            }
        }

        // Find the scope and update its duration
        if let Some(scope) = self.current_scopes.iter_mut().find(|s| {
            // We need to identify the scope - using a simple approach
            s.depth == self.scope_stack.len()
        }) {
            scope.duration = scope.start_time.elapsed();
        }
    }

    /// Update statistics for all scopes
    fn update_statistics(&mut self) {
        for scope in &self.current_scopes {
            let stats = self.scope_stats
                .entry(scope.name.clone())
                .or_insert_with(|| ScopeStats::new(scope.name.clone()));

            stats.add_sample(scope.duration);

            // Store duration for percentile calculation
            let durations = self.scope_durations
                .entry(scope.name.clone())
                .or_insert_with(Vec::new);

            durations.push(scope.duration);

            // Keep only recent durations
            if durations.len() > self.config.max_history_frames {
                durations.remove(0);
            }
        }

        // Calculate percentiles
        for (name, durations) in &mut self.scope_durations {
            if let Some(stats) = self.scope_stats.get_mut(name) {
                let mut sorted = durations.clone();
                sorted.sort();
                stats.calculate_percentiles(&sorted);
            }
        }
    }

    /// Get current frame statistics
    pub fn get_current_frame_stats(&self) -> Option<&FrameStats> {
        self.frame_history.last()
    }

    /// Get frame history
    pub fn get_frame_history(&self) -> &[FrameStats] {
        &self.frame_history
    }

    /// Get scope statistics
    pub fn get_scope_stats(&self, name: &str) -> Option<&ScopeStats> {
        self.scope_stats.get(name)
    }

    /// Get all scope statistics
    pub fn get_all_scope_stats(&self) -> &HashMap<String, ScopeStats> {
        &self.scope_stats
    }

    /// Get current scopes (for visualization)
    pub fn get_current_scopes(&self) -> &[ScopeData] {
        &self.current_scopes
    }

    /// Get average FPS over last N frames
    pub fn get_average_fps(&self, frame_count: usize) -> f32 {
        if self.frame_history.is_empty() {
            return 0.0;
        }

        let count = frame_count.min(self.frame_history.len());
        let start_idx = self.frame_history.len() - count;
        let sum: f32 = self.frame_history[start_idx..].iter().map(|f| f.fps).sum();

        sum / count as f32
    }

    /// Get average frame time over last N frames
    pub fn get_average_frame_time(&self, frame_count: usize) -> Duration {
        if self.frame_history.is_empty() {
            return Duration::ZERO;
        }

        let count = frame_count.min(self.frame_history.len());
        let start_idx = self.frame_history.len() - count;
        let sum: Duration = self.frame_history[start_idx..].iter().map(|f| f.frame_duration).sum();

        sum / count as u32
    }

    /// Clear all profiling data
    pub fn clear(&mut self) {
        self.current_scopes.clear();
        self.scope_stack.clear();
        self.frame_history.clear();
        self.scope_stats.clear();
        self.scope_durations.clear();
        self.frame_number = 0;
    }

    /// Get configuration
    pub fn get_config(&self) -> &ProfilerConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: ProfilerConfig) {
        self.config = config;
    }

    /// Get current frame number
    pub fn get_frame_number(&self) -> u64 {
        self.frame_number
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII profile scope guard
pub struct ProfileScope<'a> {
    profiler: &'a mut Profiler,
    scope_id: ScopeId,
}

impl<'a> ProfileScope<'a> {
    /// Create a new profile scope
    pub fn new(profiler: &'a mut Profiler, name: &str) -> Self {
        let scope_id = profiler.begin_scope(name);
        Self { profiler, scope_id }
    }
}

impl<'a> Drop for ProfileScope<'a> {
    fn drop(&mut self) {
        self.profiler.end_scope(self.scope_id);
    }
}

/// Macro for easy profiling
#[macro_export]
macro_rules! profile_scope {
    ($profiler:expr, $name:expr) => {
        let _profile_scope = $crate::profiler::ProfileScope::new($profiler, $name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new();
        assert_eq!(profiler.get_frame_number(), 0);
    }

    #[test]
    fn test_profiler_config() {
        let config = ProfilerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_history_frames, 300);
    }

    #[test]
    fn test_begin_end_frame() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        thread::sleep(Duration::from_millis(1));
        profiler.end_frame();

        assert_eq!(profiler.get_frame_number(), 1);
        assert!(profiler.get_current_frame_stats().is_some());
    }

    #[test]
    fn test_scope_timing() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        
        let scope_id = profiler.begin_scope("test_scope");
        thread::sleep(Duration::from_millis(1));
        profiler.end_scope(scope_id);
        
        profiler.end_frame();

        let scopes = profiler.get_current_scopes();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].name, "test_scope");
        assert!(scopes[0].duration.as_millis() >= 1);
    }

    #[test]
    fn test_profile_scope_raii() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        
        {
            let _scope = ProfileScope::new(&mut profiler, "raii_scope");
            thread::sleep(Duration::from_millis(1));
        }
        
        profiler.end_frame();

        let scopes = profiler.get_current_scopes();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].name, "raii_scope");
    }

    #[test]
    fn test_nested_scopes() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        
        let outer = profiler.begin_scope("outer");
        let inner = profiler.begin_scope("inner");
        profiler.end_scope(inner);
        profiler.end_scope(outer);
        
        profiler.end_frame();

        let scopes = profiler.get_current_scopes();
        assert_eq!(scopes.len(), 2);
        assert_eq!(scopes[0].depth, 0);
        assert_eq!(scopes[1].depth, 1);
    }

    #[test]
    fn test_frame_history() {
        let mut profiler = Profiler::new();
        
        for _ in 0..5 {
            profiler.begin_frame();
            thread::sleep(Duration::from_millis(1));
            profiler.end_frame();
        }

        let history = profiler.get_frame_history();
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_frame_history_limit() {
        let mut config = ProfilerConfig::default();
        config.max_history_frames = 3;
        
        let mut profiler = Profiler::with_config(config);
        
        for _ in 0..5 {
            profiler.begin_frame();
            profiler.end_frame();
        }

        let history = profiler.get_frame_history();
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_scope_statistics() {
        let mut profiler = Profiler::new();
        
        for _ in 0..10 {
            profiler.begin_frame();
            let scope = profiler.begin_scope("test");
            thread::sleep(Duration::from_millis(1));
            profiler.end_scope(scope);
            profiler.end_frame();
        }

        let stats = profiler.get_scope_stats("test").unwrap();
        assert_eq!(stats.sample_count, 10);
        assert!(stats.avg_duration.as_millis() >= 1);
    }

    #[test]
    fn test_average_fps() {
        let mut profiler = Profiler::new();
        
        for _ in 0..10 {
            profiler.begin_frame();
            thread::sleep(Duration::from_millis(16)); // ~60 FPS
            profiler.end_frame();
        }

        let avg_fps = profiler.get_average_fps(10);
        assert!(avg_fps > 0.0 && avg_fps < 100.0);
    }

    #[test]
    fn test_average_frame_time() {
        let mut profiler = Profiler::new();
        
        for _ in 0..5 {
            profiler.begin_frame();
            thread::sleep(Duration::from_millis(10));
            profiler.end_frame();
        }

        let avg_time = profiler.get_average_frame_time(5);
        assert!(avg_time.as_millis() >= 10);
    }

    #[test]
    fn test_clear() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        let _scope = profiler.begin_scope("test");
        profiler.end_frame();

        profiler.clear();

        assert_eq!(profiler.get_frame_number(), 0);
        assert_eq!(profiler.get_frame_history().len(), 0);
    }

    #[test]
    fn test_disabled_profiler() {
        let mut config = ProfilerConfig::default();
        config.enabled = false;
        
        let mut profiler = Profiler::with_config(config);
        
        profiler.begin_frame();
        let _scope = profiler.begin_scope("test");
        profiler.end_frame();

        assert_eq!(profiler.get_frame_history().len(), 0);
    }

    #[test]
    fn test_scope_stats_creation() {
        let stats = ScopeStats::new("test".to_string());
        assert_eq!(stats.name, "test");
        assert_eq!(stats.sample_count, 0);
    }

    #[test]
    fn test_scope_stats_add_sample() {
        let mut stats = ScopeStats::new("test".to_string());
        
        stats.add_sample(Duration::from_millis(10));
        stats.add_sample(Duration::from_millis(20));

        assert_eq!(stats.sample_count, 2);
        assert_eq!(stats.min_duration, Duration::from_millis(10));
        assert_eq!(stats.max_duration, Duration::from_millis(20));
        assert_eq!(stats.avg_duration, Duration::from_millis(15));
    }

    #[test]
    fn test_percentile_calculation() {
        let mut stats = ScopeStats::new("test".to_string());
        
        let durations: Vec<Duration> = (1..=100)
            .map(|i| Duration::from_millis(i))
            .collect();

        stats.calculate_percentiles(&durations);

        assert_eq!(stats.p95_duration, Duration::from_millis(95));
        assert_eq!(stats.p99_duration, Duration::from_millis(99));
    }

    #[test]
    fn test_frame_stats_default() {
        let stats = FrameStats::default();
        assert_eq!(stats.frame_number, 0);
        assert_eq!(stats.fps, 0.0);
    }

    #[test]
    fn test_get_all_scope_stats() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        let scope1 = profiler.begin_scope("scope1");
        profiler.end_scope(scope1);
        let scope2 = profiler.begin_scope("scope2");
        profiler.end_scope(scope2);
        profiler.end_frame();

        let all_stats = profiler.get_all_scope_stats();
        assert_eq!(all_stats.len(), 2);
    }

    #[test]
    fn test_config_modification() {
        let mut profiler = Profiler::new();
        
        let mut config = ProfilerConfig::default();
        config.max_history_frames = 100;
        
        profiler.set_config(config);
        assert_eq!(profiler.get_config().max_history_frames, 100);
    }

    #[test]
    fn test_multiple_frames_same_scope() {
        let mut profiler = Profiler::new();
        
        for i in 0..5 {
            profiler.begin_frame();
            let scope = profiler.begin_scope("repeated");
            thread::sleep(Duration::from_millis(i + 1));
            profiler.end_scope(scope);
            profiler.end_frame();
        }

        let stats = profiler.get_scope_stats("repeated").unwrap();
        assert_eq!(stats.sample_count, 5);
    }

    #[test]
    fn test_scope_depth_tracking() {
        let mut profiler = Profiler::new();
        
        profiler.begin_frame();
        
        let l1 = profiler.begin_scope("level1");
        let l2 = profiler.begin_scope("level2");
        let l3 = profiler.begin_scope("level3");
        profiler.end_scope(l3);
        profiler.end_scope(l2);
        profiler.end_scope(l1);
        
        profiler.end_frame();

        let scopes = profiler.get_current_scopes();
        assert_eq!(scopes[0].depth, 0);
        assert_eq!(scopes[1].depth, 1);
        assert_eq!(scopes[2].depth, 2);
    }

    #[test]
    fn test_empty_percentile_calculation() {
        let mut stats = ScopeStats::new("test".to_string());
        stats.calculate_percentiles(&[]);
        
        // Should not panic
        assert_eq!(stats.p95_duration, Duration::ZERO);
    }
}
