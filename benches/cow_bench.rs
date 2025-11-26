// Benchmark: Cow vs String for conditional modification
//
// This benchmark validates Phase 9 optimization by comparing:
// - String (owned): Always clones when passed
// - Cow<'_, str>: Only clones when modified
//
// Expected result: Cow should be faster when modification is conditional

use criterion::{criterion_group, criterion_main, Criterion};
use std::borrow::Cow;
use std::hint::black_box;

// Scenario 1: Read-only path (no modification)
fn process_string_readonly(s: String) -> String {
    s // Just return it
}

fn process_cow_readonly(s: Cow<'_, str>) -> Cow<'_, str> {
    s // Just return it (zero-cost!)
}

// Scenario 2: Always modify
fn process_string_modify(s: String) -> String {
    s.to_uppercase()
}

fn process_cow_modify(s: Cow<'_, str>) -> Cow<'_, str> {
    Cow::Owned(s.to_uppercase())
}

// Scenario 3: Conditional modification (50% chance)
fn process_string_conditional(s: String, modify: bool) -> String {
    if modify {
        s.to_uppercase()
    } else {
        s
    }
}

fn process_cow_conditional(s: Cow<'_, str>, modify: bool) -> Cow<'_, str> {
    if modify {
        Cow::Owned(s.to_uppercase())
    } else {
        s
    }
}

fn bench_readonly(c: &mut Criterion) {
    let data = "hello world";

    c.bench_function("readonly_string", |b| {
        b.iter(|| {
            let s = data.to_string();
            black_box(process_string_readonly(s))
        });
    });

    c.bench_function("readonly_cow", |b| {
        b.iter(|| {
            let s = Cow::Borrowed(data);
            black_box(process_cow_readonly(s))
        });
    });
}

fn bench_always_modify(c: &mut Criterion) {
    let data = "hello world";

    c.bench_function("always_modify_string", |b| {
        b.iter(|| {
            let s = data.to_string();
            black_box(process_string_modify(s))
        });
    });

    c.bench_function("always_modify_cow", |b| {
        b.iter(|| {
            let s = Cow::Borrowed(data);
            black_box(process_cow_modify(s))
        });
    });
}

fn bench_conditional_modify(c: &mut Criterion) {
    let data = "hello world";

    // 50% modification rate
    c.bench_function("conditional_string_50pct", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let s = data.to_string();
            black_box(process_string_conditional(s, counter % 2 == 0))
        });
    });

    c.bench_function("conditional_cow_50pct", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let s = Cow::Borrowed(data);
            black_box(process_cow_conditional(s, counter % 2 == 0))
        });
    });

    // 10% modification rate (mostly read-only)
    c.bench_function("conditional_string_10pct", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let s = data.to_string();
            black_box(process_string_conditional(s, counter % 10 == 0))
        });
    });

    c.bench_function("conditional_cow_10pct", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let s = Cow::Borrowed(data);
            black_box(process_cow_conditional(s, counter % 10 == 0))
        });
    });

    // 90% modification rate (mostly write)
    c.bench_function("conditional_string_90pct", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let s = data.to_string();
            black_box(process_string_conditional(s, counter % 10 != 0))
        });
    });

    c.bench_function("conditional_cow_90pct", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let s = Cow::Borrowed(data);
            black_box(process_cow_conditional(s, counter % 10 != 0))
        });
    });
}

criterion_group!(
    benches,
    bench_readonly,
    bench_always_modify,
    bench_conditional_modify
);
criterion_main!(benches);
