// Benchmark: SmallVec vs Vec for small allocations
//
// This benchmark validates Phase 8 optimization by comparing:
// - Vec<T>: Always heap allocates
// - SmallVec<[T; N]>: Stack allocates for small sizes, heap for large
//
// Expected result: SmallVec should be faster for small collections (< 8 elements)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smallvec::{smallvec, SmallVec};

fn bench_vec_creation(c: &mut Criterion) {
    // Small vec (3 elements)
    c.bench_function("vec_creation_small_vec", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..3 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    c.bench_function("vec_creation_small_smallvec", |b| {
        b.iter(|| {
            let mut v: SmallVec<[i32; 8]> = SmallVec::new();
            for i in 0..3 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    // Medium vec (8 elements)
    c.bench_function("vec_creation_medium_vec", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..8 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    c.bench_function("vec_creation_medium_smallvec", |b| {
        b.iter(|| {
            let mut v: SmallVec<[i32; 8]> = SmallVec::new();
            for i in 0..8 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    // Large vec (16 elements - exceeds SmallVec inline capacity)
    c.bench_function("vec_creation_large_vec", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..16 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    c.bench_function("vec_creation_large_smallvec", |b| {
        b.iter(|| {
            let mut v: SmallVec<[i32; 8]> = SmallVec::new();
            for i in 0..16 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });
}

fn bench_vec_literal(c: &mut Criterion) {
    // Small (3 elements) - should benefit from SmallVec
    c.bench_function("vec_literal_small_vec", |b| {
        b.iter(|| black_box(vec![1, 2, 3]));
    });

    c.bench_function("vec_literal_small_smallvec", |b| {
        b.iter(|| {
            let v: SmallVec<[i32; 8]> = smallvec![1, 2, 3];
            black_box(v)
        });
    });

    // Medium (5 elements) - should benefit from SmallVec
    c.bench_function("vec_literal_medium_vec", |b| {
        b.iter(|| black_box(vec![1, 2, 3, 4, 5]));
    });

    c.bench_function("vec_literal_medium_smallvec", |b| {
        b.iter(|| {
            let v: SmallVec<[i32; 8]> = smallvec![1, 2, 3, 4, 5];
            black_box(v)
        });
    });

    // Large (16 elements) - Vec might be competitive
    c.bench_function("vec_literal_large_vec", |b| {
        b.iter(|| black_box(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]));
    });

    c.bench_function("vec_literal_large_smallvec", |b| {
        b.iter(|| {
            let v: SmallVec<[i32; 8]> =
                smallvec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
            black_box(v)
        });
    });
}

fn bench_vec_iteration(c: &mut Criterion) {
    let vec_data: Vec<i32> = (0..8).collect();
    let smallvec_data: SmallVec<[i32; 8]> = (0..8).collect();

    c.bench_function("vec_iteration_vec", |b| {
        b.iter(|| {
            let sum: i32 = vec_data.iter().sum();
            black_box(sum)
        });
    });

    c.bench_function("vec_iteration_smallvec", |b| {
        b.iter(|| {
            let sum: i32 = smallvec_data.iter().sum();
            black_box(sum)
        });
    });
}

criterion_group!(
    benches,
    bench_vec_creation,
    bench_vec_literal,
    bench_vec_iteration
);
criterion_main!(benches);
