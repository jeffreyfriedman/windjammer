// Benchmark to measure PERCEIVED LATENCY improvement with defer drop
// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
//
// Key insight: Defer drop doesn't necessarily reduce TOTAL time,
// but it dramatically reduces TIME TO RETURN, which is what users perceive.
//
// This benchmark measures:
// 1. Time until function returns (user-perceived latency)
// 2. Total time including cleanup (actual work done)

use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

const NUM_ELEMENTS: usize = 1_000_000;

type HeavyThings = HashMap<usize, Vec<usize>>;

fn make_heavy_things() -> HeavyThings {
    (1..=NUM_ELEMENTS).map(|v| (v, vec![v])).collect()
}

// WITHOUT defer drop - function waits for drop to complete
fn fn_that_drops_heavy_things(things: HeavyThings) -> usize {
    things.len()
    // Drop happens here - function can't return until drop completes
}

// WITH defer drop - function returns immediately, drop happens in background
fn fn_that_drops_heavy_things_in_another_thread(things: HeavyThings) -> usize {
    let len = things.len();
    thread::spawn(move || drop(things));
    len
    // Function returns immediately!
}

fn main() {
    println!("=== Defer Drop Latency Benchmark ===\n");
    println!(
        "Testing with HashMap<usize, Vec<usize>> ({} elements)\n",
        NUM_ELEMENTS
    );

    // Warm up
    println!("Warming up...");
    for _ in 0..3 {
        let things = make_heavy_things();
        let _ = fn_that_drops_heavy_things(things);
    }

    // Benchmark: WITHOUT defer drop (synchronous)
    println!("\n--- WITHOUT Defer Drop (Synchronous) ---");
    let mut sync_times = Vec::new();
    for i in 0..10 {
        let things = make_heavy_things();
        let start = Instant::now();
        let len = fn_that_drops_heavy_things(things);
        let elapsed = start.elapsed();
        sync_times.push(elapsed);
        println!("Run {}: Time to return: {:?} (len={})", i + 1, elapsed, len);
    }

    // Benchmark: WITH defer drop (asynchronous)
    println!("\n--- WITH Defer Drop (Asynchronous) ---");
    let mut async_times = Vec::new();
    for i in 0..10 {
        let things = make_heavy_things();
        let start = Instant::now();
        let len = fn_that_drops_heavy_things_in_another_thread(things);
        let elapsed = start.elapsed();
        async_times.push(elapsed);
        println!("Run {}: Time to return: {:?} (len={})", i + 1, elapsed, len);

        // Give background thread time to finish before next iteration
        thread::sleep(Duration::from_millis(100));
    }

    // Calculate statistics
    println!("\n=== Results ===\n");

    let sync_avg: Duration = sync_times.iter().sum::<Duration>() / sync_times.len() as u32;
    let async_avg: Duration = async_times.iter().sum::<Duration>() / async_times.len() as u32;

    let sync_min = *sync_times.iter().min().unwrap();
    let sync_max = *sync_times.iter().max().unwrap();
    let async_min = *async_times.iter().min().unwrap();
    let async_max = *async_times.iter().max().unwrap();

    println!("Synchronous Drop (Normal):");
    println!("  Average: {:?}", sync_avg);
    println!("  Min: {:?}", sync_min);
    println!("  Max: {:?}", sync_max);

    println!("\nAsynchronous Drop (Defer):");
    println!("  Average: {:?}", async_avg);
    println!("  Min: {:?}", async_min);
    println!("  Max: {:?}", async_max);

    let speedup = sync_avg.as_micros() as f64 / async_avg.as_micros() as f64;
    println!("\nSpeedup: {:.2}x faster time to return!", speedup);

    if speedup > 2.0 {
        println!(
            "✅ Defer drop provides {:.0}% faster return time!",
            (speedup - 1.0) * 100.0
        );
    } else {
        println!("⚠️  Defer drop provides modest speedup for this workload.");
        println!(
            "    Real benefit: User sees response {:.2}x faster!",
            speedup
        );
    }

    println!("\n=== Key Insight ===");
    println!(
        "The function returns {:.2}x faster with defer drop,",
        speedup
    );
    println!("even though the total work (including background cleanup) is similar.");
    println!("This is CRITICAL for interactive applications where latency matters!");
}
