// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Closed-loop Clock microbenchmark for `DEC-CLOCK-001`.

use std::hint::black_box;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use agrona::clock::{
    CachedEpochClock, EpochClock, EpochNanoClock, NanoClock, OffsetEpochNanoClock,
    OffsetEpochNanoClockConfig, SystemEpochClock, SystemEpochNanoClock, SystemNanoClock,
};

const DEFAULT_ITERATIONS: u64 = 10_000_000;

struct FixedEpochClock(i64);

impl EpochClock for FixedEpochClock {
    fn time(&self) -> i64 {
        self.0
    }
}

struct IncrementingNanoClock(AtomicI64);

impl NanoClock for IncrementingNanoClock {
    fn nano_time(&self) -> i64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

fn measure(name: &str, iterations: u64, mut operation: impl FnMut() -> i64) {
    for _ in 0..10_000 {
        black_box(operation());
    }

    let started = Instant::now();
    for _ in 0..iterations {
        black_box(operation());
    }
    let elapsed = started.elapsed();
    let nanos_per_operation = elapsed.as_secs_f64() * 1_000_000_000.0 / iterations as f64;

    println!("{name:28} {nanos_per_operation:10.3} ns/op");
}

fn main() {
    let iterations = std::env::var("AGRONA_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_ITERATIONS);

    println!("agrona-rs Clock benchmark");
    println!("Agrona baseline: d4a47c67258f85b39910c4999da346ead655b736");
    println!(
        "target: {}-{}; iterations: {iterations}",
        std::env::consts::ARCH,
        std::env::consts::OS
    );

    measure("SystemEpochClock", iterations, || SystemEpochClock.time());
    measure("SystemEpochNanoClock", iterations, || {
        SystemEpochNanoClock.nano_time()
    });
    measure("SystemNanoClock", iterations, || {
        SystemNanoClock.nano_time()
    });

    let cached = CachedEpochClock::with_initial_time(1_000);
    let cached_reader = cached.reader();
    measure("CachedEpochClockReader", iterations, || {
        cached_reader.time()
    });

    let offset = OffsetEpochNanoClock::with_sources(
        FixedEpochClock(1_000),
        IncrementingNanoClock(AtomicI64::new(0)),
        OffsetEpochNanoClockConfig::new(1, Duration::from_nanos(10), Duration::from_secs(60 * 60))
            .expect("benchmark configuration should be valid"),
    )
    .expect("benchmark sample should succeed");
    measure("OffsetEpochNanoClock", iterations, || offset.nano_time());
}
