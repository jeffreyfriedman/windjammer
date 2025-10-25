use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use windjammer_game_framework::ecs::World;

#[derive(Clone, Copy, Debug)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct Health {
    current: f32,
    max: f32,
}

fn bench_entity_spawn(c: &mut Criterion) {
    c.bench_function("entity_spawn", |b| {
        let mut world = World::new();

        b.iter(|| {
            let entity = world.spawn().build();
            black_box(entity);
        });
    });
}

fn bench_entity_spawn_with_components(c: &mut Criterion) {
    c.bench_function("entity_spawn_with_components", |b| {
        let mut world = World::new();

        b.iter(|| {
            let entity = world.spawn().build();
            world.add_component(entity, Position { x: 0.0, y: 0.0 });
            world.add_component(entity, Velocity { x: 1.0, y: 1.0 });
            world.add_component(
                entity,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            );
            black_box(entity);
        });
    });
}

fn bench_component_add(c: &mut Criterion) {
    c.bench_function("component_add", |b| {
        let mut world = World::new();
        let entity = world.spawn().build();

        b.iter(|| {
            world.add_component(
                entity,
                Position {
                    x: black_box(0.0),
                    y: black_box(0.0),
                },
            );
        });
    });
}

fn bench_component_get(c: &mut Criterion) {
    let mut world = World::new();
    let entity = world.spawn().build();
    world.add_component(entity, Position { x: 10.0, y: 20.0 });

    c.bench_function("component_get", |b| {
        b.iter(|| {
            let pos = world.get_component::<Position>(entity);
            black_box(pos);
        });
    });
}

fn bench_component_get_mut(c: &mut Criterion) {
    c.bench_function("component_get_mut", |b| {
        let mut world = World::new();
        let entity = world.spawn().build();
        world.add_component(entity, Position { x: 10.0, y: 20.0 });

        b.iter(|| {
            if let Some(pos) = world.get_component_mut::<Position>(entity) {
                pos.x += 1.0;
                black_box(pos);
            }
        });
    });
}

fn bench_query_single_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_single");

    for entity_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = World::new();

                // Create entities with Position component
                for i in 0..count {
                    let entity = world.spawn().build();
                    world.add_component(
                        entity,
                        Position {
                            x: i as f32,
                            y: i as f32,
                        },
                    );
                }

                b.iter(|| {
                    let results = world.query::<Position>();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn bench_query_two_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_two");

    for entity_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = World::new();

                // Create entities with Position and Velocity
                for i in 0..count {
                    let entity = world.spawn().build();
                    world.add_component(
                        entity,
                        Position {
                            x: i as f32,
                            y: i as f32,
                        },
                    );
                    world.add_component(entity, Velocity { x: 1.0, y: 1.0 });
                }

                b.iter(|| {
                    let results = world.query2::<Position, Velocity>();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn bench_system_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("system_iteration");

    for entity_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = World::new();

                // Create entities
                for i in 0..count {
                    let entity = world.spawn().build();
                    world.add_component(
                        entity,
                        Position {
                            x: i as f32,
                            y: i as f32,
                        },
                    );
                    world.add_component(entity, Velocity { x: 1.0, y: 1.0 });
                }

                b.iter(|| {
                    // Simulate a physics system
                    let results = world.query2_mut::<Position, Velocity>();
                    for (_entity, pos, vel) in results {
                        pos.x += vel.x * 0.016;
                        pos.y += vel.y * 0.016;
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_entity_despawn(c: &mut Criterion) {
    c.bench_function("entity_despawn", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let entity = world.spawn().build();
                world.add_component(entity, Position { x: 0.0, y: 0.0 });
                (world, entity)
            },
            |(mut world, entity)| {
                world.despawn(entity);
                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_component_remove(c: &mut Criterion) {
    c.bench_function("component_remove", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let entity = world.spawn().build();
                world.add_component(entity, Position { x: 0.0, y: 0.0 });
                (world, entity)
            },
            |(mut world, entity)| {
                let removed = world.remove_component::<Position>(entity);
                black_box(removed);
                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_sparse_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_entities");

    for entity_count in [1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = World::new();

                // Create entities, but only 10% have Position+Velocity
                for i in 0..count {
                    let entity = world.spawn().build();
                    if i % 10 == 0 {
                        world.add_component(
                            entity,
                            Position {
                                x: i as f32,
                                y: i as f32,
                            },
                        );
                        world.add_component(entity, Velocity { x: 1.0, y: 1.0 });
                    }
                }

                b.iter(|| {
                    let results = world.query2::<Position, Velocity>();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn bench_game_loop_simulation(c: &mut Criterion) {
    c.bench_function("game_loop_full", |b| {
        let mut world = World::new();

        // Create 1000 game entities
        for i in 0..1000 {
            let entity = world.spawn().build();
            world.add_component(
                entity,
                Position {
                    x: (i % 100) as f32,
                    y: (i / 100) as f32,
                },
            );
            world.add_component(
                entity,
                Velocity {
                    x: (i % 3) as f32 - 1.0,
                    y: (i % 5) as f32 - 2.0,
                },
            );
            world.add_component(
                entity,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            );
        }

        b.iter(|| {
            // Movement system
            let movement = world.query2_mut::<Position, Velocity>();
            for (_entity, pos, vel) in movement {
                pos.x += vel.x * 0.016;
                pos.y += vel.y * 0.016;
            }

            // Health system (read-only)
            let health_check = world.query::<Health>();
            for (_entity, health) in health_check {
                black_box(health.current / health.max);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_entity_spawn,
    bench_entity_spawn_with_components,
    bench_component_add,
    bench_component_get,
    bench_component_get_mut,
    bench_query_single_component,
    bench_query_two_components,
    bench_system_iteration,
    bench_entity_despawn,
    bench_component_remove,
    bench_sparse_entities,
    bench_game_loop_simulation
);

criterion_main!(benches);
