// Benchmark to validate defer drop optimization speedup
// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
//
// This benchmark compares:
// 1. Normal drop (synchronous deallocation before return)
// 2. Defer drop (async deallocation via std::thread::spawn)
//
// Expected speedup: 10,000x for large collections (e.g., HashMap with 1M entries)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::thread;

// === HASHMAP BENCHMARKS ===

fn create_large_hashmap() -> HashMap<usize, Vec<usize>> {
    (0..1_000_000).map(|i| (i, vec![i])).collect()
}

// WITHOUT defer drop - normal synchronous drop
fn hashmap_normal_drop(data: HashMap<usize, Vec<usize>>) -> usize {
    let len = data.len();
    len
    // data is dropped here synchronously - SLOW!
}

// WITH defer drop - async drop in background thread
fn hashmap_defer_drop(data: HashMap<usize, Vec<usize>>) -> usize {
    let len = data.len();
    thread::spawn(move || drop(data)); // Drop in background - FAST!
    len
}

fn bench_hashmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_defer_drop");
    group.sample_size(10); // Fewer samples since each is expensive

    group.bench_function("normal_drop", |b| {
        b.iter(|| {
            let data = create_large_hashmap();
            black_box(hashmap_normal_drop(data))
        })
    });

    group.bench_function("defer_drop", |b| {
        b.iter(|| {
            let data = create_large_hashmap();
            black_box(hashmap_defer_drop(data))
        })
    });

    group.finish();
}

// === VEC BENCHMARKS ===

fn create_large_vec() -> Vec<Vec<usize>> {
    (0..10_000_000).map(|i| vec![i]).collect()
}

fn vec_normal_drop(data: Vec<Vec<usize>>) -> usize {
    let len = data.len();
    len
}

fn vec_defer_drop(data: Vec<Vec<usize>>) -> usize {
    let len = data.len();
    thread::spawn(move || drop(data));
    len
}

fn bench_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec_defer_drop");
    group.sample_size(10);

    group.bench_function("normal_drop", |b| {
        b.iter(|| {
            let data = create_large_vec();
            black_box(vec_normal_drop(data))
        })
    });

    group.bench_function("defer_drop", |b| {
        b.iter(|| {
            let data = create_large_vec();
            black_box(vec_defer_drop(data))
        })
    });

    group.finish();
}

// === STRING BENCHMARKS ===

fn create_large_string() -> String {
    "a".repeat(100_000_000) // 100MB string
}

fn string_normal_drop(data: String) -> usize {
    let len = data.len();
    len
}

fn string_defer_drop(data: String) -> usize {
    let len = data.len();
    thread::spawn(move || drop(data));
    len
}

fn bench_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_defer_drop");
    group.sample_size(10);

    group.bench_function("normal_drop", |b| {
        b.iter(|| {
            let data = create_large_string();
            black_box(string_normal_drop(data))
        })
    });

    group.bench_function("defer_drop", |b| {
        b.iter(|| {
            let data = create_large_string();
            black_box(string_defer_drop(data))
        })
    });

    group.finish();
}

// === REALISTIC SCENARIO: API REQUEST/RESPONSE ===

#[derive(Clone)]
struct ApiRequest {
    headers: HashMap<String, String>,
    body: Vec<u8>,
    metadata: HashMap<String, Vec<String>>,
}

impl ApiRequest {
    fn large() -> Self {
        let mut headers = HashMap::new();
        for i in 0..1000 {
            headers.insert(format!("header_{}", i), format!("value_{}", i));
        }

        let body = vec![0u8; 10_000_000]; // 10MB body

        let mut metadata = HashMap::new();
        for i in 0..1000 {
            metadata.insert(
                format!("meta_{}", i),
                (0..100).map(|j| format!("value_{}_{}", i, j)).collect(),
            );
        }

        ApiRequest {
            headers,
            body,
            metadata,
        }
    }
}

fn api_extract_user_id_normal(request: ApiRequest) -> Option<String> {
    request.headers.get("user-id").cloned()
    // request is dropped here synchronously
}

fn api_extract_user_id_defer(request: ApiRequest) -> Option<String> {
    let user_id = request.headers.get("user-id").cloned();
    thread::spawn(move || drop(request)); // Drop in background
    user_id
}

fn bench_realistic_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_request_defer_drop");
    group.sample_size(10);

    group.bench_function("normal_drop", |b| {
        b.iter(|| {
            let request = ApiRequest::large();
            black_box(api_extract_user_id_normal(request))
        })
    });

    group.bench_function("defer_drop", |b| {
        b.iter(|| {
            let request = ApiRequest::large();
            black_box(api_extract_user_id_defer(request))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hashmap,
    bench_vec,
    bench_string,
    bench_realistic_api
);
criterion_main!(benches);
