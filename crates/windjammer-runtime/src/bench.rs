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
        let (time_slow, time_fast, speedup) = bench_compare(
            || {
                thread::sleep(Duration::from_millis(2));
            },
            || {
                thread::sleep(Duration::from_millis(1));
            },
            5,
        );

        // Slow should be slower than fast
        assert!(
            time_slow > time_fast,
            "Slow function should take more time than fast function"
        );

        // Speedup should be roughly 2x, but CI environments can have high variance
        // due to CPU throttling, virtualization, and load. Just verify it's faster.
        assert!(
            speedup > 1.0,
            "Fast function should be at least somewhat faster (speedup > 1.0), got {}",
            speedup
        );

        // Sanity check: speedup shouldn't be absurdly high
        assert!(
            speedup < 10.0,
            "Speedup seems unrealistic (> 10x), got {}",
            speedup
        );
    }
}
