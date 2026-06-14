//! Scalar vs handwritten SIMD kernels (~10M `f32` elements).
//!
//! Validates the shape of SIMD code the Windjammer Rust backend emits (`std::arch`
//! chunks + scalar tail). Run: `cargo bench -p windjammer --bench simd_vectorization_bench`.
//! On `x86_64` with AVX, compare `simd_sse_4wide`, `simd_avx256_8wide`, and `scalar`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const N: usize = 10_000_000;

fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut s = 0.0_f32;
    let n = a.len().min(b.len());
    for i in 0..n {
        s += a[i] * b[i];
    }
    black_box(s)
}

#[cfg(target_arch = "x86_64")]
fn dot_simd_x86(a: &[f32], b: &[f32]) -> f32 {
    unsafe {
        use std::arch::x86_64::*;
        let n = a.len().min(b.len());
        let chunks = n / 4;
        let mut vacc = _mm_setzero_ps();
        let ap = a.as_ptr();
        let bp = b.as_ptr();
        for c in 0..chunks {
            let o = c * 4;
            let va = _mm_loadu_ps(ap.add(o));
            let vb = _mm_loadu_ps(bp.add(o));
            vacc = _mm_add_ps(vacc, _mm_mul_ps(va, vb));
        }
        let parts: [f32; 4] = std::mem::transmute(vacc);
        let mut s = parts[0] + parts[1] + parts[2] + parts[3];
        for i in (chunks * 4)..n {
            s += a[i] * b[i];
        }
        black_box(s)
    }
}

#[cfg(target_arch = "aarch64")]
fn dot_simd_neon(a: &[f32], b: &[f32]) -> f32 {
    unsafe {
        use std::arch::aarch64::*;
        let n = a.len().min(b.len());
        let chunks = n / 4;
        let mut vacc = vdupq_n_f32(0.0);
        let ap = a.as_ptr();
        let bp = b.as_ptr();
        for c in 0..chunks {
            let o = c * 4;
            let va = vld1q_f32(ap.add(o));
            let vb = vld1q_f32(bp.add(o));
            vacc = vfmaq_f32(vacc, va, vb);
        }
        let parts: [f32; 4] = std::mem::transmute(vacc);
        let mut s = parts[0] + parts[1] + parts[2] + parts[3];
        for i in (chunks * 4)..n {
            s += a[i] * b[i];
        }
        black_box(s)
    }
}

#[cfg(target_arch = "x86_64")]
fn dot_simd_avx256(a: &[f32], b: &[f32]) -> f32 {
    unsafe {
        use std::arch::x86_64::*;
        let n = a.len().min(b.len());
        let chunks = n / 8;
        let mut vacc = _mm256_setzero_ps();
        let ap = a.as_ptr();
        let bp = b.as_ptr();
        for c in 0..chunks {
            let o = c * 8;
            let va = _mm256_loadu_ps(ap.add(o));
            let vb = _mm256_loadu_ps(bp.add(o));
            vacc = _mm256_add_ps(vacc, _mm256_mul_ps(va, vb));
        }
        let parts: [f32; 8] = std::mem::transmute(vacc);
        let mut s =
            parts[0] + parts[1] + parts[2] + parts[3] + parts[4] + parts[5] + parts[6] + parts[7];
        for i in (chunks * 8)..n {
            s += a[i] * b[i];
        }
        black_box(s)
    }
}

fn bench_dots(c: &mut Criterion) {
    let a: Vec<f32> = (0..N).map(|i| (i % 997) as f32 * 0.001).collect();
    let b: Vec<f32> = (0..N).map(|i| (i % 1009) as f32 * 0.002).collect();
    let a = black_box(a);
    let b = black_box(b);

    let mut g = c.benchmark_group("f32_dot_10m");
    g.bench_function("scalar", |bench| {
        bench.iter(|| dot_scalar(black_box(&a), black_box(&b)))
    });
    #[cfg(target_arch = "x86_64")]
    g.bench_function("simd_sse_4wide", |bench| {
        bench.iter(|| dot_simd_x86(black_box(&a), black_box(&b)))
    });
    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("avx") {
        g.bench_function("simd_avx256_8wide", |bench| {
            bench.iter(|| dot_simd_avx256(black_box(&a), black_box(&b)))
        });
    }
    #[cfg(target_arch = "aarch64")]
    g.bench_function("simd_neon", |bench| {
        bench.iter(|| dot_simd_neon(black_box(&a), black_box(&b)))
    });
    g.finish();
}

criterion_group!(simd_benches, bench_dots);
criterion_main!(simd_benches);
