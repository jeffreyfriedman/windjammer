//! Profile-Guided Optimization (PGO) System
//!
//! This module provides infrastructure for profile-guided optimization,
//! allowing the engine to collect runtime profiling data and use it to
//! optimize performance-critical code paths.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Profile-Guided Optimization manager
pub struct PGOManager {
    /// Function call counts
    call_counts: Arc<RwLock<HashMap<String, u64>>>,
    
    /// Function execution times
    execution_times: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    
    /// Hot path threshold (calls per second)
    hot_path_threshold: u64,
    
    /// Whether profiling is enabled
    enabled: bool,
    
    /// Start time for profiling session
    session_start: Instant,
}

impl PGOManager {
    /// Create a new PGO manager
    pub fn new() -> Self {
        Self {
            call_counts: Arc::new(RwLock::new(HashMap::new())),
            execution_times: Arc::new(RwLock::new(HashMap::new())),
            hot_path_threshold: 1000, // 1000 calls/sec = hot path
            enabled: cfg!(feature = "pgo"),
            session_start: Instant::now(),
        }
    }
    
    /// Enable profiling
    pub fn enable(&mut self) {
        self.enabled = true;
        self.session_start = Instant::now();
    }
    
    /// Disable profiling
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Record a function call
    pub fn record_call(&self, function_name: &str) {
        if !self.enabled {
            return;
        }
        
        if let Ok(mut counts) = self.call_counts.write() {
            *counts.entry(function_name.to_string()).or_insert(0) += 1;
        }
    }
    
    /// Record function execution time
    pub fn record_execution_time(&self, function_name: &str, duration: Duration) {
        if !self.enabled {
            return;
        }
        
        if let Ok(mut times) = self.execution_times.write() {
            times.entry(function_name.to_string())
                .or_insert_with(Vec::new)
                .push(duration);
        }
    }
    
    /// Get hot paths (frequently called functions)
    pub fn get_hot_paths(&self) -> Vec<(String, u64)> {
        let counts = match self.call_counts.read() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        
        let elapsed = self.session_start.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return Vec::new();
        }
        
        let mut hot_paths: Vec<_> = counts
            .iter()
            .filter(|(_, &count)| {
                let calls_per_sec = (count as f64 / elapsed) as u64;
                calls_per_sec >= self.hot_path_threshold
            })
            .map(|(name, &count)| (name.clone(), count))
            .collect();
        
        hot_paths.sort_by(|a, b| b.1.cmp(&a.1));
        hot_paths
    }
    
    /// Get slow functions (high average execution time)
    pub fn get_slow_functions(&self, threshold_ms: f64) -> Vec<(String, f64)> {
        let times = match self.execution_times.read() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        
        let mut slow_functions: Vec<_> = times
            .iter()
            .filter_map(|(name, durations)| {
                if durations.is_empty() {
                    return None;
                }
                
                let avg_ms = durations.iter()
                    .map(|d| d.as_secs_f64() * 1000.0)
                    .sum::<f64>() / durations.len() as f64;
                
                if avg_ms >= threshold_ms {
                    Some((name.clone(), avg_ms))
                } else {
                    None
                }
            })
            .collect();
        
        slow_functions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        slow_functions
    }
    
    /// Get profiling statistics
    pub fn get_statistics(&self) -> PGOStatistics {
        let counts = self.call_counts.read().unwrap();
        let times = self.execution_times.read().unwrap();
        
        let total_calls: u64 = counts.values().sum();
        let total_functions = counts.len();
        
        let elapsed = self.session_start.elapsed();
        
        PGOStatistics {
            total_calls,
            total_functions,
            session_duration: elapsed,
            hot_paths: self.get_hot_paths(),
            slow_functions: self.get_slow_functions(1.0), // 1ms threshold
        }
    }
    
    /// Export profiling data for PGO compilation
    pub fn export_profile_data(&self) -> String {
        let counts = self.call_counts.read().unwrap();
        let times = self.execution_times.read().unwrap();
        
        let mut data = String::new();
        data.push_str("# Windjammer PGO Profile Data\n");
        data.push_str(&format!("# Session Duration: {:?}\n", self.session_start.elapsed()));
        data.push_str("\n[call_counts]\n");
        
        for (name, count) in counts.iter() {
            data.push_str(&format!("{} = {}\n", name, count));
        }
        
        data.push_str("\n[execution_times]\n");
        for (name, durations) in times.iter() {
            let avg_ms = durations.iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum::<f64>() / durations.len() as f64;
            data.push_str(&format!("{} = {:.3}ms\n", name, avg_ms));
        }
        
        data
    }
    
    /// Clear all profiling data
    pub fn clear(&self) {
        if let Ok(mut counts) = self.call_counts.write() {
            counts.clear();
        }
        if let Ok(mut times) = self.execution_times.write() {
            times.clear();
        }
    }
    
    /// Generate optimization hints based on profiling data
    pub fn generate_optimization_hints(&self) -> Vec<OptimizationHint> {
        let mut hints = Vec::new();
        
        // Identify hot paths that should be inlined
        for (function, count) in self.get_hot_paths() {
            hints.push(OptimizationHint {
                function_name: function.clone(),
                hint_type: OptimizationHintType::Inline,
                reason: format!("Called {} times (hot path)", count),
                priority: OptimizationPriority::High,
            });
        }
        
        // Identify slow functions that need optimization
        for (function, avg_ms) in self.get_slow_functions(5.0) {
            hints.push(OptimizationHint {
                function_name: function.clone(),
                hint_type: OptimizationHintType::Optimize,
                reason: format!("Average execution time: {:.2}ms", avg_ms),
                priority: if avg_ms > 10.0 {
                    OptimizationPriority::Critical
                } else {
                    OptimizationPriority::High
                },
            });
        }
        
        hints
    }
}

impl Default for PGOManager {
    fn default() -> Self {
        Self::new()
    }
}

/// PGO statistics
#[derive(Debug, Clone)]
pub struct PGOStatistics {
    /// Total number of function calls
    pub total_calls: u64,
    
    /// Total number of profiled functions
    pub total_functions: usize,
    
    /// Session duration
    pub session_duration: Duration,
    
    /// Hot paths (function name, call count)
    pub hot_paths: Vec<(String, u64)>,
    
    /// Slow functions (function name, avg time in ms)
    pub slow_functions: Vec<(String, f64)>,
}

/// Optimization hint generated from profiling data
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    /// Function name
    pub function_name: String,
    
    /// Type of optimization hint
    pub hint_type: OptimizationHintType,
    
    /// Reason for the hint
    pub reason: String,
    
    /// Priority level
    pub priority: OptimizationPriority,
}

/// Type of optimization hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationHintType {
    /// Function should be inlined
    Inline,
    
    /// Function needs general optimization
    Optimize,
    
    /// Function should use SIMD
    Vectorize,
    
    /// Function should be parallelized
    Parallelize,
    
    /// Function has cache misses
    CacheOptimize,
}

/// Optimization priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationPriority {
    /// Low priority
    Low,
    
    /// Medium priority
    Medium,
    
    /// High priority
    High,
    
    /// Critical priority
    Critical,
}

/// Macro to profile a function call
#[macro_export]
macro_rules! profile_function {
    ($pgo:expr, $name:expr, $body:expr) => {{
        $pgo.record_call($name);
        let start = std::time::Instant::now();
        let result = $body;
        let duration = start.elapsed();
        $pgo.record_execution_time($name, duration);
        result
    }};
}

/// Scope-based profiler for automatic timing
pub struct ProfileScope<'a> {
    pgo: &'a PGOManager,
    function_name: String,
    start: Instant,
}

impl<'a> ProfileScope<'a> {
    /// Create a new profile scope
    pub fn new(pgo: &'a PGOManager, function_name: impl Into<String>) -> Self {
        let function_name = function_name.into();
        pgo.record_call(&function_name);
        
        Self {
            pgo,
            function_name,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for ProfileScope<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.pgo.record_execution_time(&self.function_name, duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_pgo_manager_creation() {
        let pgo = PGOManager::new();
        assert!(!pgo.is_enabled() || cfg!(feature = "pgo"));
    }
    
    #[test]
    fn test_record_call() {
        let mut pgo = PGOManager::new();
        pgo.enable();
        
        pgo.record_call("test_function");
        pgo.record_call("test_function");
        pgo.record_call("test_function");
        
        let stats = pgo.get_statistics();
        assert_eq!(stats.total_calls, 3);
    }
    
    #[test]
    fn test_record_execution_time() {
        let mut pgo = PGOManager::new();
        pgo.enable();
        
        pgo.record_execution_time("test_function", Duration::from_millis(10));
        pgo.record_execution_time("test_function", Duration::from_millis(20));
        
        let slow_functions = pgo.get_slow_functions(5.0);
        assert!(!slow_functions.is_empty());
    }
    
    #[test]
    fn test_profile_scope() {
        let mut pgo = PGOManager::new();
        pgo.enable();
        
        {
            let _scope = ProfileScope::new(&pgo, "test_function");
            thread::sleep(Duration::from_millis(10));
        }
        
        let stats = pgo.get_statistics();
        assert_eq!(stats.total_calls, 1);
    }
    
    #[test]
    fn test_optimization_hints() {
        let mut pgo = PGOManager::new();
        pgo.enable();
        
        // Simulate hot path
        for _ in 0..10000 {
            pgo.record_call("hot_function");
        }
        
        // Simulate slow function
        pgo.record_execution_time("slow_function", Duration::from_millis(10));
        
        let hints = pgo.generate_optimization_hints();
        assert!(!hints.is_empty());
    }
}

