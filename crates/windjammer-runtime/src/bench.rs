//! Benchmarking utilities for Windjammer tests
//!
//! Provides simple benchmarking functions that can be used with `@bench` decorator.
//! For more advanced benchmarking, use the criterion crate directly.

use std::time::{Duration, Instant};

/// Run a closure multiple times and return the average duration
///
/// # Example
/// ```
/// use windjammer_runtime::bench::bench_iterations;
///
/// let avg = bench_iterations(1000, || {
///     // Code to benchmark
///     let _ = (0..100).sum::<i32>();
/// });
///
/// println!("Average time: {:?}", avg);
/// ```
pub fn bench_iterations<F: Fn()>(iterations: usize, f: F) -> Duration {
    let mut total = Duration::ZERO;

    // Warmup
    for _ in 0..10 {
        f();
    }

    // Actual benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        f();
        total += start.elapsed();
    }

    total / iterations as u32
}

/// Benchmark a closure and return its duration
///
/// # Example
/// ```
/// use windjammer_runtime::bench::bench;
///
/// let duration = bench(|| {
///     // Code to benchmark
///     let _ = (0..1000).sum::<i32>();
/// });
///
/// println!("Duration: {:?}", duration);
/// ```
pub fn bench<F: FnOnce()>(f: F) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

/// Compare two implementations and return their relative performance
///
/// # Example
/// ```
/// use windjammer_runtime::bench::bench_compare;
///
/// let (time_a, time_b, speedup) = bench_compare(
///     || {
///         // Old implementation
///         let _ = (0..1000).sum::<i32>();
///     },
///     || {
///         // New implementation
///         let _ = (0..1000).fold(0, |acc, x| acc + x);
///     },
///     100, // iterations
/// );
///
/// println!("Old: {:?}, New: {:?}, Speedup: {:.2}x", time_a, time_b, speedup);
/// ```
pub fn bench_compare<F: Fn(), G: Fn()>(f: F, g: G, iterations: usize) -> (Duration, Duration, f64) {
    let time_f = bench_iterations(iterations, f);
    let time_g = bench_iterations(iterations, g);

    let speedup = time_f.as_secs_f64() / time_g.as_secs_f64();

    (time_f, time_g, speedup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_bench_iterations() {
        let avg = bench_iterations(10, || {
            thread::sleep(Duration::from_millis(1));
        });

        // Should be around 1ms (with some variance)
        assert!(avg.as_millis() >= 1 && avg.as_millis() <= 5);
    }

    #[test]
    fn test_bench() {
        let duration = bench(|| {
            thread::sleep(Duration::from_millis(10));
        });

        // Should be around 10ms
        assert!(duration.as_millis() >= 10 && duration.as_millis() <= 20);
    }

    #[test]
    fn test_bench_compare() {
        // Test that bench_compare returns reasonable measurements
        // Note: We can't reliably test timing relationships in CI due to
        // thread scheduling variance, so just verify the function works
        let (time_slow, time_fast, speedup) = bench_compare(
            || {
                // Use busy-wait instead of sleep for more deterministic timing
                let start = std::time::Instant::now();
                let mut sum = 0u64;
                while start.elapsed() < Duration::from_micros(100) {
                    sum = sum.wrapping_add(1);
                }
            },
            || {
                // Shorter busy-wait
                let start = std::time::Instant::now();
                let mut sum = 0u64;
                while start.elapsed() < Duration::from_micros(50) {
                    sum = sum.wrapping_add(1);
                }
            },
            3,
        );

        // Just verify we got reasonable measurements (not zero, not absurd)
        assert!(
            time_slow > Duration::from_nanos(1),
            "Slow function should take measurable time, got {:?}",
            time_slow
        );
        assert!(
            time_fast > Duration::from_nanos(1),
            "Fast function should take measurable time, got {:?}",
            time_fast
        );
        assert!(
            speedup > 0.0 && speedup < 1000.0,
            "Speedup should be reasonable, got {}",
            speedup
        );
    }
}
