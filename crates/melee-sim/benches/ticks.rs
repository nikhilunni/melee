//! Regression measurements, deliberately separate from the bit-exact gates.
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::{hint::black_box, path::Path, time::Duration};

fn ticks(c: &mut Criterion) {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/start_fd_fox.toml"),
    )
    .unwrap();
    assert_eq!(scenario.frames, 600);
    for path in scenario.required_files() {
        assert!(
            melee_test_support::trace::exists(&path),
            "benchmark requires {}",
            path.display()
        );
    }
    let load = || {
        Simulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            trace::pad_script(&scenario).unwrap(),
        )
    };
    let mut group = c.benchmark_group("start_fd_fox");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    // Return the imported scene so its destruction is outside the timed interval.
    group.bench_function("load", |b| {
        b.iter_batched(|| (), |()| black_box(load()), BatchSize::PerIteration)
    });
    group.throughput(Throughput::Elements(600));
    // Fresh savestate for every sample: setup and destruction are not tick time.
    group.bench_function("ticks_600", |b| {
        b.iter_batched_ref(
            load,
            |simulation| {
                for _ in 0..600 {
                    black_box(simulation.tick_without_snapshot()).unwrap();
                }
                black_box(simulation);
            },
            BatchSize::PerIteration,
        )
    });
    group.finish();
}
criterion_group!(benches, ticks);
criterion_main!(benches);
