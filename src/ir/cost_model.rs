//! Compilation cost tracking for WJ-PERF-01 economic efficiency.
//!
//! Records per-phase timing, memory, and output size so the compiler can
//! report economics and drive optimization decisions.

/// Tracks compilation costs across the full pipeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilationCostTracker {
    pub parse_time_ms: f64,
    pub analyze_time_ms: f64,
    pub ir_time_ms: f64,
    pub codegen_time_ms: f64,
    pub total_time_ms: f64,
    pub peak_memory_bytes: usize,
    pub output_size_bytes: usize,
}

impl CompilationCostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recompute `total_time_ms` from phase timings.
    pub fn finalize(&mut self) {
        self.total_time_ms = self.parse_time_ms
            + self.analyze_time_ms
            + self.ir_time_ms
            + self.codegen_time_ms;
    }

    /// Record a phase duration in milliseconds.
    pub fn record_phase(&mut self, phase: CompilationPhase, duration_ms: f64) {
        match phase {
            CompilationPhase::Parse => self.parse_time_ms += duration_ms,
            CompilationPhase::Analyze => self.analyze_time_ms += duration_ms,
            CompilationPhase::Ir => self.ir_time_ms += duration_ms,
            CompilationPhase::Codegen => self.codegen_time_ms += duration_ms,
        }
    }

    /// Update peak memory if the current sample exceeds the prior peak.
    pub fn record_memory_sample(&mut self, bytes: usize) {
        if bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = bytes;
        }
    }

    /// Set generated output size (e.g. emitted Rust source or binary).
    pub fn set_output_size(&mut self, bytes: usize) {
        self.output_size_bytes = bytes;
    }

    /// Merge another tracker into this one (useful for multi-file builds).
    pub fn merge(&mut self, other: &CompilationCostTracker) {
        self.parse_time_ms += other.parse_time_ms;
        self.analyze_time_ms += other.analyze_time_ms;
        self.ir_time_ms += other.ir_time_ms;
        self.codegen_time_ms += other.codegen_time_ms;
        self.total_time_ms += other.total_time_ms;
        self.peak_memory_bytes = self.peak_memory_bytes.max(other.peak_memory_bytes);
        self.output_size_bytes += other.output_size_bytes;
    }
}

/// Pipeline phases tracked by [`CompilationCostTracker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationPhase {
    Parse,
    Analyze,
    Ir,
    Codegen,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_sums_phase_timings() {
        let mut tracker = CompilationCostTracker {
            parse_time_ms: 1.0,
            analyze_time_ms: 2.0,
            ir_time_ms: 3.0,
            codegen_time_ms: 4.0,
            ..Default::default()
        };
        tracker.finalize();
        assert_eq!(tracker.total_time_ms, 10.0);
    }

    #[test]
    fn merge_accumulates_costs() {
        let mut a = CompilationCostTracker {
            parse_time_ms: 1.0,
            peak_memory_bytes: 100,
            output_size_bytes: 50,
            ..Default::default()
        };
        let b = CompilationCostTracker {
            analyze_time_ms: 2.0,
            peak_memory_bytes: 200,
            output_size_bytes: 75,
            total_time_ms: 2.0,
            ..Default::default()
        };
        a.merge(&b);
        assert_eq!(a.analyze_time_ms, 2.0);
        assert_eq!(a.peak_memory_bytes, 200);
        assert_eq!(a.output_size_bytes, 125);
        assert_eq!(a.total_time_ms, 2.0);
    }
}
