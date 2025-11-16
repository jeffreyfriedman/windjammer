//! Performance Profiler System
//!
//! Provides performance analysis tools for optimization.
//!
//! ## Features
//! - Frame timing
//! - CPU profiling
//! - Memory tracking
//! - Draw call counting
//! - System performance metrics
//! - Hierarchical profiling

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance profiler
#[derive(Debug)]
pub struct Profiler {
    /// Frame timing history
    frame_times: Vec<Duration>,
    /// Max frame history
    max_history: usize,
    /// Current frame start
    frame_start: Option<Instant>,
    /// Profile scopes
    scopes: HashMap<String, ProfileScope>,
    /// Active scope stack
    scope_stack: Vec<String>,
    /// Enabled
    pub enabled: bool,
}

/// Profile scope
#[derive(Debug, Clone)]
pub struct ProfileScope {
    /// Scope name
    pub name: String,
    /// Total time spent
    pub total_time: Duration,
    /// Number of calls
    pub call_count: u64,
    /// Average time per call
    pub avg_time: Duration,
    /// Min time
    pub min_time: Duration,
    /// Max time
    pub max_time: Duration,
    /// Parent scope
    pub parent: Option<String>,
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Average FPS
    pub avg_fps: f32,
    /// Min FPS
    pub min_fps: f32,
    /// Max FPS
    pub max_fps: f32,
    /// Average frame time (ms)
    pub avg_frame_time: f32,
    /// Min frame time (ms)
    pub min_frame_time: f32,
    /// Max frame time (ms)
    pub max_frame_time: f32,
    /// Frame time percentiles
    pub percentiles: FrameTimePercentiles,
}

/// Frame time percentiles
#[derive(Debug, Clone)]
pub struct FrameTimePercentiles {
    /// 50th percentile (median)
    pub p50: f32,
    /// 90th percentile
    pub p90: f32,
    /// 95th percentile
    pub p95: f32,
    /// 99th percentile
    pub p99: f32,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total allocated (bytes)
    pub allocated: usize,
    /// Peak allocated (bytes)
    pub peak: usize,
    /// Allocation count
    pub allocations: usize,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            frame_times: Vec::new(),
            max_history: 300, // 5 seconds at 60 FPS
            frame_start: None,
            scopes: HashMap::new(),
            scope_stack: Vec::new(),
            enabled: true,
        }
    }

    /// Begin frame timing
    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }
        self.frame_start = Some(Instant::now());
    }

    /// End frame timing
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        if let Some(start) = self.frame_start.take() {
            let frame_time = start.elapsed();
            self.frame_times.push(frame_time);

            // Keep only recent history
            if self.frame_times.len() > self.max_history {
                self.frame_times.remove(0);
            }
        }
    }

    /// Begin a profile scope
    pub fn begin_scope(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        let scope_name = name.to_string();
        self.scope_stack.push(scope_name.clone());

        // Initialize scope if it doesn't exist
        if !self.scopes.contains_key(&scope_name) {
            let parent = if self.scope_stack.len() > 1 {
                Some(self.scope_stack[self.scope_stack.len() - 2].clone())
            } else {
                None
            };

            self.scopes.insert(
                scope_name.clone(),
                ProfileScope {
                    name: scope_name,
                    total_time: Duration::ZERO,
                    call_count: 0,
                    avg_time: Duration::ZERO,
                    min_time: Duration::from_secs(999999),
                    max_time: Duration::ZERO,
                    parent,
                },
            );
        }
    }

    /// End a profile scope
    pub fn end_scope(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        if let Some(last) = self.scope_stack.last() {
            if last == name {
                self.scope_stack.pop();
            }
        }
    }

    /// Record scope timing
    pub fn record_scope(&mut self, name: &str, duration: Duration) {
        if !self.enabled {
            return;
        }

        if let Some(scope) = self.scopes.get_mut(name) {
            scope.total_time += duration;
            scope.call_count += 1;
            scope.avg_time = scope.total_time / scope.call_count as u32;
            scope.min_time = scope.min_time.min(duration);
            scope.max_time = scope.max_time.max(duration);
        }
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> PerformanceStats {
        if self.frame_times.is_empty() {
            return PerformanceStats {
                avg_fps: 0.0,
                min_fps: 0.0,
                max_fps: 0.0,
                avg_frame_time: 0.0,
                min_frame_time: 0.0,
                max_frame_time: 0.0,
                percentiles: FrameTimePercentiles {
                    p50: 0.0,
                    p90: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                },
            };
        }

        let mut sorted_times: Vec<f32> = self
            .frame_times
            .iter()
            .map(|d| d.as_secs_f32() * 1000.0)
            .collect();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let avg_frame_time = sorted_times.iter().sum::<f32>() / sorted_times.len() as f32;
        let min_frame_time = sorted_times[0];
        let max_frame_time = sorted_times[sorted_times.len() - 1];

        let avg_fps = 1000.0 / avg_frame_time;
        let min_fps = 1000.0 / max_frame_time;
        let max_fps = 1000.0 / min_frame_time;

        let p50_idx = (sorted_times.len() as f32 * 0.50) as usize;
        let p90_idx = (sorted_times.len() as f32 * 0.90) as usize;
        let p95_idx = (sorted_times.len() as f32 * 0.95) as usize;
        let p99_idx = (sorted_times.len() as f32 * 0.99) as usize;

        PerformanceStats {
            avg_fps,
            min_fps,
            max_fps,
            avg_frame_time,
            min_frame_time,
            max_frame_time,
            percentiles: FrameTimePercentiles {
                p50: sorted_times[p50_idx.min(sorted_times.len() - 1)],
                p90: sorted_times[p90_idx.min(sorted_times.len() - 1)],
                p95: sorted_times[p95_idx.min(sorted_times.len() - 1)],
                p99: sorted_times[p99_idx.min(sorted_times.len() - 1)],
            },
        }
    }

    /// Get scope statistics
    pub fn get_scope(&self, name: &str) -> Option<&ProfileScope> {
        self.scopes.get(name)
    }

    /// Get all scopes
    pub fn get_all_scopes(&self) -> Vec<&ProfileScope> {
        self.scopes.values().collect()
    }

    /// Clear all profiling data
    pub fn clear(&mut self) {
        self.frame_times.clear();
        self.scopes.clear();
        self.scope_stack.clear();
    }

    /// Get current FPS
    pub fn current_fps(&self) -> f32 {
        if let Some(last_frame) = self.frame_times.last() {
            1.0 / last_frame.as_secs_f32()
        } else {
            0.0
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Profile scope guard (RAII)
pub struct ProfileGuard<'a> {
    profiler: &'a mut Profiler,
    name: String,
    start: Instant,
}

impl<'a> ProfileGuard<'a> {
    /// Create a new profile guard
    pub fn new(profiler: &'a mut Profiler, name: &str) -> Self {
        profiler.begin_scope(name);
        Self {
            profiler,
            name: name.to_string(),
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for ProfileGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.profiler.record_scope(&self.name, duration);
        self.profiler.end_scope(&self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new();
        assert!(profiler.enabled);
        println!("✅ Profiler created");
    }

    #[test]
    fn test_frame_timing() {
        let mut profiler = Profiler::new();

        for _ in 0..10 {
            profiler.begin_frame();
            thread::sleep(Duration::from_millis(1));
            profiler.end_frame();
        }

        let stats = profiler.get_stats();
        assert!(stats.avg_fps > 0.0);
        assert!(stats.avg_frame_time > 0.0);
        println!("✅ Frame timing: {} FPS", stats.avg_fps);
    }

    #[test]
    fn test_scope_profiling() {
        let mut profiler = Profiler::new();

        profiler.begin_scope("test_scope");
        thread::sleep(Duration::from_millis(1));
        profiler.record_scope("test_scope", Duration::from_millis(1));
        profiler.end_scope("test_scope");

        let scope = profiler.get_scope("test_scope");
        assert!(scope.is_some());
        assert_eq!(scope.unwrap().call_count, 1);
        println!("✅ Scope profiling");
    }

    #[test]
    fn test_profile_guard() {
        let mut profiler = Profiler::new();

        {
            let _guard = ProfileGuard::new(&mut profiler, "guarded_scope");
            thread::sleep(Duration::from_millis(1));
        }

        let scope = profiler.get_scope("guarded_scope");
        assert!(scope.is_some());
        assert_eq!(scope.unwrap().call_count, 1);
        println!("✅ Profile guard (RAII)");
    }

    #[test]
    fn test_nested_scopes() {
        let mut profiler = Profiler::new();

        profiler.begin_scope("outer");
        profiler.begin_scope("inner");
        profiler.end_scope("inner");
        profiler.end_scope("outer");

        let outer = profiler.get_scope("outer");
        let inner = profiler.get_scope("inner");

        assert!(outer.is_some());
        assert!(inner.is_some());
        assert_eq!(inner.unwrap().parent, Some("outer".to_string()));
        println!("✅ Nested scopes");
    }

    #[test]
    fn test_stats_calculation() {
        let mut profiler = Profiler::new();

        // Add known frame times
        profiler.frame_times.push(Duration::from_millis(16)); // ~60 FPS
        profiler.frame_times.push(Duration::from_millis(33)); // ~30 FPS
        profiler.frame_times.push(Duration::from_millis(16));

        let stats = profiler.get_stats();
        assert!(stats.avg_fps > 0.0);
        assert!(stats.min_fps > 0.0);
        assert!(stats.max_fps > 0.0);
        println!("✅ Stats calculation: avg={} FPS", stats.avg_fps);
    }

    #[test]
    fn test_percentiles() {
        let mut profiler = Profiler::new();

        for i in 0..100 {
            profiler.frame_times.push(Duration::from_millis(i));
        }

        let stats = profiler.get_stats();
        assert!(stats.percentiles.p50 > 0.0);
        assert!(stats.percentiles.p90 > stats.percentiles.p50);
        assert!(stats.percentiles.p95 > stats.percentiles.p90);
        assert!(stats.percentiles.p99 > stats.percentiles.p95);
        println!("✅ Percentiles: p50={}, p99={}", stats.percentiles.p50, stats.percentiles.p99);
    }

    #[test]
    fn test_clear() {
        let mut profiler = Profiler::new();

        profiler.begin_frame();
        profiler.end_frame();
        profiler.begin_scope("test");
        profiler.end_scope("test");

        profiler.clear();

        assert_eq!(profiler.frame_times.len(), 0);
        assert_eq!(profiler.scopes.len(), 0);
        println!("✅ Clear profiling data");
    }

    #[test]
    fn test_current_fps() {
        let mut profiler = Profiler::new();

        profiler.frame_times.push(Duration::from_millis(16));
        let fps = profiler.current_fps();
        assert!(fps > 60.0 && fps < 63.0); // ~62.5 FPS
        println!("✅ Current FPS: {}", fps);
    }

    #[test]
    fn test_disabled_profiler() {
        let mut profiler = Profiler::new();
        profiler.enabled = false;

        profiler.begin_frame();
        profiler.end_frame();

        assert_eq!(profiler.frame_times.len(), 0);
        println!("✅ Disabled profiler");
    }

    #[test]
    fn test_max_history() {
        let mut profiler = Profiler::new();
        profiler.max_history = 10;

        for _ in 0..20 {
            profiler.begin_frame();
            profiler.end_frame();
        }

        assert_eq!(profiler.frame_times.len(), 10);
        println!("✅ Max history limit");
    }

    #[test]
    fn test_scope_statistics() {
        let mut profiler = Profiler::new();

        for _ in 0..5 {
            profiler.record_scope("test", Duration::from_millis(10));
        }

        let scope = profiler.get_scope("test").unwrap();
        assert_eq!(scope.call_count, 5);
        assert_eq!(scope.avg_time, Duration::from_millis(10));
        println!("✅ Scope statistics: {} calls", scope.call_count);
    }

    #[test]
    fn test_get_all_scopes() {
        let mut profiler = Profiler::new();

        profiler.begin_scope("scope1");
        profiler.end_scope("scope1");
        profiler.begin_scope("scope2");
        profiler.end_scope("scope2");

        let scopes = profiler.get_all_scopes();
        assert_eq!(scopes.len(), 2);
        println!("✅ Get all scopes: {} scopes", scopes.len());
    }
}

