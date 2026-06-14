/// Microbenchmark: AoS `Vec<Entity>` vs SoA separate `Vec<f32>` columns.
///
/// Run: `cargo bench -p windjammer --bench cache_locality_aosoa_bench`
///
/// Compare cache behavior with:
/// `perf stat -r 10 -e cache-references,cache-misses ./target/release/deps/cache_locality_aosoa_bench-*`
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const N: usize = 10_000;

#[derive(Clone, Copy)]
struct Entity {
    x: f32,
    y: f32,
    z: f32,
    tag: i32,
}

fn sum_positions_aos(entities: &[Entity]) -> f32 {
    let mut s = 0.0f32;
    for e in entities {
        s += e.x + e.y + e.z;
    }
    black_box(s)
}

/// Loads every field including cold `tag` (typical AoS footprint per iteration).
fn sum_positions_aos_all_fields(entities: &[Entity]) -> f32 {
    let mut s = 0.0f32;
    for e in entities {
        s += e.x + e.y + e.z + (e.tag as f32);
    }
    black_box(s)
}

struct EntitySoA {
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
    tag: Vec<i32>,
}

fn sum_positions_soa(soa: &EntitySoA) -> f32 {
    black_box(soa.tag.as_slice());
    let mut s = 0.0f32;
    let n = soa.x.len();
    for i in 0..n {
        s += soa.x[i] + soa.y[i] + soa.z[i];
    }
    black_box(s)
}

fn bench_compare(c: &mut Criterion) {
    let entities: Vec<Entity> = (0..N)
        .map(|i| {
            let f = i as f32;
            Entity {
                x: f,
                y: f * 0.5,
                z: f * 0.25,
                tag: i as i32,
            }
        })
        .collect();
    let soa = EntitySoA {
        x: entities.iter().map(|e| e.x).collect(),
        y: entities.iter().map(|e| e.y).collect(),
        z: entities.iter().map(|e| e.z).collect(),
        tag: entities.iter().map(|e| e.tag).collect(),
    };

    let mut group = c.benchmark_group("cache_locality_ecs_10k");
    group.bench_function("aos_vec_entity_sum_xyz", |b| {
        b.iter(|| sum_positions_aos(black_box(entities.as_slice())))
    });
    group.bench_function("aos_vec_entity_sum_all_fields_xyz_tag", |b| {
        b.iter(|| sum_positions_aos_all_fields(black_box(entities.as_slice())))
    });
    group.bench_function("soa_vec_fields_sum_xyz", |b| {
        b.iter(|| sum_positions_soa(black_box(&soa)))
    });
    group.finish();
}

criterion_group!(benches, bench_compare);
criterion_main!(benches);
