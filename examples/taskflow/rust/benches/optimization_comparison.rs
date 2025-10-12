// Comprehensive optimization benchmarks comparing naive vs optimized implementations
// Demonstrates: SmallVec, Cow, defer drop, and const/static optimizations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use smallvec::{smallvec, SmallVec};
use std::borrow::Cow;
use std::collections::HashMap;

// ============================================================================
// Phase 7: Const vs Static Optimization
// ============================================================================

// Naive: Runtime static initialization
static mut NAIVE_STATUS_CODES: Option<HashMap<&'static str, i32>> = None;

fn naive_status_codes() -> &'static HashMap<&'static str, i32> {
    unsafe {
        NAIVE_STATUS_CODES.get_or_insert_with(|| {
            let mut map = HashMap::new();
            map.insert("open", 1);
            map.insert("in_progress", 2);
            map.insert("review", 3);
            map.insert("done", 4);
            map.insert("closed", 5);
            map
        })
    }
}

// Optimized: Compile-time const array
const OPTIMIZED_STATUS_CODES: &[(&str, i32)] = &[
    ("open", 1),
    ("in_progress", 2),
    ("review", 3),
    ("done", 4),
    ("closed", 5),
];

fn optimized_status_lookup(status: &str) -> Option<i32> {
    OPTIMIZED_STATUS_CODES
        .iter()
        .find(|(k, _)| *k == status)
        .map(|(_, v)| *v)
}

// ============================================================================
// Phase 8: SmallVec Optimization
// ============================================================================

#[derive(Clone)]
struct Task {
    id: i32,
    title: String,
    priority: u8,
}

// Naive: Always heap-allocate Vec
fn naive_filter_high_priority_tasks(tasks: &[Task]) -> Vec<Task> {
    let mut result = Vec::new();
    for task in tasks {
        if task.priority >= 8 {
            result.push(task.clone());
        }
    }
    result
}

// Optimized: Use SmallVec for typical small result sets
fn optimized_filter_high_priority_tasks(tasks: &[Task]) -> SmallVec<[Task; 8]> {
    let mut result = SmallVec::new();
    for task in tasks {
        if task.priority >= 8 {
            result.push(task.clone());
        }
    }
    result
}

// Naive: Vec for tag collection
fn naive_collect_tags(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("tag{}", i)).collect()
}

// Optimized: SmallVec for typical small tag lists
fn optimized_collect_tags(count: usize) -> SmallVec<[String; 4]> {
    (0..count).map(|i| format!("tag{}", i)).collect()
}

// ============================================================================
// Phase 9: Cow Optimization
// ============================================================================

// Naive: Always clone strings
fn naive_normalize_status(status: String, uppercase: bool) -> String {
    if uppercase {
        status.to_uppercase()
    } else {
        status
    }
}

// Optimized: Use Cow to avoid unnecessary cloning
fn optimized_normalize_status(status: Cow<'_, str>, uppercase: bool) -> Cow<'_, str> {
    if uppercase {
        Cow::Owned(status.to_uppercase())
    } else {
        status
    }
}

// Naive: Always clone for validation
fn naive_validate_and_sanitize(input: String, needs_sanitization: bool) -> String {
    if needs_sanitization {
        input.trim().to_string()
    } else {
        input
    }
}

// Optimized: Use Cow for conditional modification
fn optimized_validate_and_sanitize(input: Cow<'_, str>, needs_sanitization: bool) -> Cow<'_, str> {
    if needs_sanitization {
        Cow::Owned(input.trim().to_string())
    } else {
        input
    }
}

// ============================================================================
// Phase 0: Defer Drop Optimization
// ============================================================================

// Naive: Synchronous drop of large data structures
fn naive_cleanup_large_cache() {
    let mut cache: HashMap<i32, Vec<String>> = HashMap::new();
    for i in 0..1000 {
        let mut entries = Vec::new();
        for j in 0..100 {
            entries.push(format!("task_{}_{}", i, j));
        }
        cache.insert(i, entries);
    }
    // Synchronous drop happens here
    drop(cache);
}

// Optimized: Defer drop to background thread
fn optimized_cleanup_large_cache() {
    let mut cache: HashMap<i32, Vec<String>> = HashMap::new();
    for i in 0..1000 {
        let mut entries = Vec::new();
        for j in 0..100 {
            entries.push(format!("task_{}_{}", i, j));
        }
        cache.insert(i, entries);
    }
    // Defer drop to background thread
    std::thread::spawn(move || {
        drop(cache);
    });
}

// ============================================================================
// Combined Real-World Scenario
// ============================================================================

#[derive(Clone)]
struct TaskRequest {
    title: String,
    description: String,
    tags: Vec<String>,
    priority: u8,
}

// Naive: No optimizations
fn naive_process_task_batch(requests: Vec<TaskRequest>) -> Vec<String> {
    let mut results = Vec::new();
    
    for req in requests {
        // Always clone for validation
        let title = req.title.clone();
        let validated_title = if title.len() > 100 {
            title[..100].to_string()
        } else {
            title
        };
        
        // Always use Vec for tags
        let mut filtered_tags: Vec<String> = Vec::new();
        for tag in req.tags {
            if tag.len() > 0 {
                filtered_tags.push(tag);
            }
        }
        
        // Lookup status code from runtime static
        let status_code = naive_status_codes().get("open").unwrap();
        
        results.push(format!("{} - {} tags - status {}", validated_title, filtered_tags.len(), status_code));
    }
    
    results
}

// Optimized: All optimizations applied
fn optimized_process_task_batch(requests: Vec<TaskRequest>) -> Vec<String> {
    let mut results = Vec::new();
    
    for req in requests {
        // Use Cow for conditional modification
        let title = Cow::Borrowed(req.title.as_str());
        let validated_title = if title.len() > 100 {
            Cow::Owned(title[..100].to_string())
        } else {
            title
        };
        
        // Use SmallVec for typical small tag lists
        let mut filtered_tags: SmallVec<[String; 4]> = SmallVec::new();
        for tag in req.tags {
            if !tag.is_empty() {
                filtered_tags.push(tag);
            }
        }
        
        // Lookup status code from const
        let status_code = optimized_status_lookup("open").unwrap();
        
        results.push(format!("{} - {} tags - status {}", validated_title, filtered_tags.len(), status_code));
    }
    
    results
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_const_static(c: &mut Criterion) {
    let mut group = c.benchmark_group("const_static");
    
    group.bench_function("naive_static_lookup", |b| {
        b.iter(|| {
            black_box(naive_status_codes().get("in_progress"));
        });
    });
    
    group.bench_function("optimized_const_lookup", |b| {
        b.iter(|| {
            black_box(optimized_status_lookup("in_progress"));
        });
    });
    
    group.finish();
}

fn bench_smallvec(c: &mut Criterion) {
    let mut group = c.benchmark_group("smallvec");
    
    let tasks: Vec<Task> = (0..20)
        .map(|i| Task {
            id: i,
            title: format!("Task {}", i),
            priority: (i % 10) as u8,
        })
        .collect();
    
    group.bench_function("naive_vec_filter", |b| {
        b.iter(|| {
            black_box(naive_filter_high_priority_tasks(&tasks));
        });
    });
    
    group.bench_function("optimized_smallvec_filter", |b| {
        b.iter(|| {
            black_box(optimized_filter_high_priority_tasks(&tasks));
        });
    });
    
    for size in [2, 4, 8].iter() {
        group.bench_with_input(BenchmarkId::new("naive_vec_collect", size), size, |b, &size| {
            b.iter(|| {
                black_box(naive_collect_tags(size));
            });
        });
        
        group.bench_with_input(BenchmarkId::new("optimized_smallvec_collect", size), size, |b, &size| {
            b.iter(|| {
                black_box(optimized_collect_tags(size));
            });
        });
    }
    
    group.finish();
}

fn bench_cow(c: &mut Criterion) {
    let mut group = c.benchmark_group("cow");
    
    let status = "in_progress".to_string();
    
    group.bench_function("naive_string_readonly", |b| {
        b.iter(|| {
            black_box(naive_normalize_status(status.clone(), false));
        });
    });
    
    group.bench_function("optimized_cow_readonly", |b| {
        b.iter(|| {
            black_box(optimized_normalize_status(Cow::Borrowed(&status), false));
        });
    });
    
    group.bench_function("naive_string_modify", |b| {
        b.iter(|| {
            black_box(naive_normalize_status(status.clone(), true));
        });
    });
    
    group.bench_function("optimized_cow_modify", |b| {
        b.iter(|| {
            black_box(optimized_normalize_status(Cow::Borrowed(&status), true));
        });
    });
    
    let input = "  task title  ".to_string();
    
    group.bench_function("naive_validate_no_sanitize", |b| {
        b.iter(|| {
            black_box(naive_validate_and_sanitize(input.clone(), false));
        });
    });
    
    group.bench_function("optimized_validate_no_sanitize", |b| {
        b.iter(|| {
            black_box(optimized_validate_and_sanitize(Cow::Borrowed(&input), false));
        });
    });
    
    group.finish();
}

fn bench_defer_drop(c: &mut Criterion) {
    let mut group = c.benchmark_group("defer_drop");
    group.sample_size(10);
    
    group.bench_function("naive_sync_drop", |b| {
        b.iter(|| {
            black_box(naive_cleanup_large_cache());
        });
    });
    
    group.bench_function("optimized_defer_drop", |b| {
        b.iter(|| {
            black_box(optimized_cleanup_large_cache());
        });
    });
    
    group.finish();
}

fn bench_combined_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_real_world");
    
    let requests: Vec<TaskRequest> = (0..50)
        .map(|i| TaskRequest {
            title: format!("Task number {} with a description", i),
            description: format!("This is a detailed description for task {}", i),
            tags: vec!["urgent".to_string(), "backend".to_string(), "api".to_string()],
            priority: (i % 10) as u8,
        })
        .collect();
    
    group.bench_function("naive_no_optimizations", |b| {
        b.iter(|| {
            black_box(naive_process_task_batch(requests.clone()));
        });
    });
    
    group.bench_function("optimized_all_phases", |b| {
        b.iter(|| {
            black_box(optimized_process_task_batch(requests.clone()));
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_const_static,
    bench_smallvec,
    bench_cow,
    bench_defer_drop,
    bench_combined_scenario
);
criterion_main!(benches);

