use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use windjammer_ui::reactivity::Signal;

fn bench_signal_creation(c: &mut Criterion) {
    c.bench_function("signal_new", |b| {
        b.iter(|| {
            let signal = Signal::new(black_box(42));
            black_box(signal);
        });
    });
}

fn bench_signal_get(c: &mut Criterion) {
    let signal = Signal::new(42);

    c.bench_function("signal_get", |b| {
        b.iter(|| {
            let value = signal.get();
            black_box(value);
        });
    });
}

fn bench_signal_set(c: &mut Criterion) {
    c.bench_function("signal_set", |b| {
        let signal = Signal::new(0);
        let mut counter = 0;

        b.iter(|| {
            signal.set(black_box(counter));
            counter += 1;
        });
    });
}

fn bench_signal_update(c: &mut Criterion) {
    c.bench_function("signal_update", |b| {
        let signal = Signal::new(0);

        b.iter(|| {
            signal.update(|val| *val += 1);
        });
    });
}

fn bench_signal_update_closure(c: &mut Criterion) {
    c.bench_function("signal_update_closure", |b| {
        let signal = Signal::new(0);

        b.iter(|| {
            signal.update(|val| {
                *val += 1;
            });
        });
    });
}

fn bench_multiple_signals(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_signals");

    for signal_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(signal_count),
            signal_count,
            |b, &count| {
                b.iter(|| {
                    let signals: Vec<_> = (0..count).map(Signal::new).collect();

                    for signal in &signals {
                        signal.set(black_box(42));
                    }

                    black_box(signals);
                });
            },
        );
    }

    group.finish();
}

fn bench_computed_signal(c: &mut Criterion) {
    c.bench_function("computed_signal", |b| {
        let signal_a = Signal::new(10);
        let signal_b = Signal::new(20);

        b.iter(|| {
            let sum = signal_a.get() + signal_b.get();
            black_box(sum);
        });
    });
}

fn bench_signal_chain(c: &mut Criterion) {
    c.bench_function("signal_chain", |b| {
        let signal1 = Signal::new(1);
        let signal2 = Signal::new(2);
        let signal3 = Signal::new(3);
        let signal4 = Signal::new(4);
        let signal5 = Signal::new(5);

        b.iter(|| {
            let result =
                signal1.get() + signal2.get() + signal3.get() + signal4.get() + signal5.get();
            black_box(result);
        });
    });
}

fn bench_signal_memory(c: &mut Criterion) {
    c.bench_function("signal_memory_footprint", |b| {
        b.iter(|| {
            let signals: Vec<_> = (0..1000).map(Signal::new).collect();
            black_box(signals);
        });
    });
}

criterion_group!(
    benches,
    bench_signal_creation,
    bench_signal_get,
    bench_signal_set,
    bench_signal_update,
    bench_signal_update_closure,
    bench_multiple_signals,
    bench_computed_signal,
    bench_signal_chain,
    bench_signal_memory
);

criterion_main!(benches);
