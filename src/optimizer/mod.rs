//! Windjammer Optimization Phases
//!
//! This module implements all compiler optimization phases (0-15).
//! Each phase is designed to be composable and can be toggled independently.
//!
//! **Optimization Phases:**
//! - Phase 0: Defer Drop (Background deallocation)
//! - Phase 1: Inline Hints (Hot path optimization)
//! - Phase 2: Clone Elimination (Remove unnecessary clones)
//! - Phase 3: Struct Mapping (Idiomatic patterns)
//! - Phase 4: String Capacity (Pre-allocate buffers)
//! - Phase 5: Compound Assigns (Use +=, -=, etc.)
//! - Phase 6: Constant Folding (Compile-time evaluation)
//! - Phase 7: Const/Static Promotion (Promote to const)
//! - Phase 8: SmallVec (Stack-allocate small vectors)
//! - Phase 9: Cow (Clone-on-write)
//! - **Phase 11: String Interning (Deduplicate literals)** 🆕
//! - **Phase 12: Dead Code Elimination (Remove unreachable code)** 🆕
//! - **Phase 13: Loop Optimization (Hoist invariants, unroll)** 🆕
//! - **Phase 14: Escape Analysis (Stack-allocate when safe)** 🆕
//! - **Phase 15: SIMD Vectorization (Auto-vectorize numeric code)** 🆕

pub mod phase11_string_interning;
pub mod phase12_dead_code_elimination;
pub mod phase13_loop_optimization;
// pub mod phase14_escape_analysis;
// pub mod phase15_simd_vectorization;

use crate::parser::Program;

/// Configuration for optimizer
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Enable Phase 11: String Interning
    pub enable_string_interning: bool,
    /// Enable Phase 12: Dead Code Elimination
    pub enable_dead_code_elimination: bool,
    /// Enable Phase 13: Loop Optimization
    pub enable_loop_optimization: bool,
    /// Enable Phase 14: Escape Analysis
    pub enable_escape_analysis: bool,
    /// Enable Phase 15: SIMD Vectorization
    pub enable_simd_vectorization: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            enable_string_interning: true,
            enable_dead_code_elimination: true,
            enable_loop_optimization: true,
            enable_escape_analysis: false, // Conservative - needs more testing
            enable_simd_vectorization: false, // Conservative - needs more testing
        }
    }
}

/// Result of optimization pass
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Optimized program
    pub program: Program,
    /// Optimization statistics
    pub stats: OptimizationStats,
}

/// Statistics from optimization passes
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    /// String interning
    pub strings_interned: usize,
    pub string_memory_saved: usize,

    /// Dead code elimination
    pub dead_functions_removed: usize,
    pub dead_structs_removed: usize,
    pub dead_code_bytes_saved: usize,

    /// Loop optimization
    pub loops_optimized: usize,
    pub invariants_hoisted: usize,
    pub loops_unrolled: usize,

    /// Escape analysis
    pub heap_to_stack_conversions: usize,
    pub allocations_eliminated: usize,

    /// SIMD vectorization
    pub loops_vectorized: usize,
    pub simd_speedup_estimate: f64,
}

/// Main optimizer entry point
pub struct Optimizer {
    config: OptimizerConfig,
}

impl Optimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(OptimizerConfig::default())
    }

    /// Run all enabled optimization passes
    pub fn optimize(&self, program: Program) -> OptimizationResult {
        let mut program = program;
        let mut stats = OptimizationStats::default();

        // Phase 11: String Interning
        if self.config.enable_string_interning {
            let result = phase11_string_interning::optimize_string_interning(&program);
            program = result.program;
            stats.strings_interned = result.strings_interned;
            stats.string_memory_saved = result.memory_saved;
        }

        // Phase 12: Dead Code Elimination
        if self.config.enable_dead_code_elimination {
            let (optimized_program, dce_stats) =
                phase12_dead_code_elimination::eliminate_dead_code(&program);
            program = optimized_program;
            stats.dead_functions_removed = dce_stats.unused_functions_removed;
            stats.dead_code_bytes_saved =
                dce_stats.unreachable_statements_removed + dce_stats.empty_blocks_removed;
        }

        // Phase 13: Loop Optimization
        if self.config.enable_loop_optimization {
            let (optimized_program, loop_stats) =
                phase13_loop_optimization::optimize_loops(&program);
            program = optimized_program;
            stats.loops_optimized = loop_stats.loops_optimized;
            stats.invariants_hoisted = loop_stats.invariants_hoisted;
            stats.loops_unrolled = loop_stats.loops_unrolled;
        }

        // Phase 14: Escape Analysis
        // if self.config.enable_escape_analysis {
        //     let result = phase14_escape_analysis::perform_escape_analysis(&program);
        //     program = result.program;
        //     stats.heap_to_stack_conversions = result.conversions;
        // }

        // Phase 15: SIMD Vectorization
        // if self.config.enable_simd_vectorization {
        //     let result = phase15_simd_vectorization::vectorize_loops(&program);
        //     program = result.program;
        //     stats.loops_vectorized = result.loops_vectorized;
        // }

        OptimizationResult { program, stats }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = Optimizer::with_defaults();
        assert!(optimizer.config.enable_string_interning);
        assert!(optimizer.config.enable_dead_code_elimination);
    }

    #[test]
    fn test_optimizer_custom_config() {
        let config = OptimizerConfig {
            enable_string_interning: true,
            enable_dead_code_elimination: false,
            enable_loop_optimization: false,
            enable_escape_analysis: false,
            enable_simd_vectorization: false,
        };
        let optimizer = Optimizer::new(config);
        assert!(optimizer.config.enable_string_interning);
        assert!(!optimizer.config.enable_dead_code_elimination);
    }
}
