// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral and concurrency tests for `OffsetEpochNanoClock`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use agrona::clock::{
    EpochClock, EpochNanoClock, NanoClock, OffsetEpochNanoClock, OffsetEpochNanoClockConfig,
    OffsetEpochNanoClockError, SystemEpochClock,
};

#[derive(Debug)]
struct ScriptedEpochClock {
    values: Vec<i64>,
    index: AtomicUsize,
}

impl ScriptedEpochClock {
    fn new(values: impl IntoIterator<Item = i64>) -> Self {
        Self {
            values: values.into_iter().collect(),
            index: AtomicUsize::new(0),
        }
    }
}

impl EpochClock for ScriptedEpochClock {
    fn time(&self) -> i64 {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        self.values[index]
    }
}

#[derive(Debug)]
struct ScriptedNanoClock {
    values: Vec<i64>,
    index: AtomicUsize,
}

impl ScriptedNanoClock {
    fn new(values: impl IntoIterator<Item = i64>) -> Self {
        Self {
            values: values.into_iter().collect(),
            index: AtomicUsize::new(0),
        }
    }
}

impl NanoClock for ScriptedNanoClock {
    fn nano_time(&self) -> i64 {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        self.values[index]
    }
}

fn config(retries: usize, threshold_ns: u64, interval_ns: u64) -> OffsetEpochNanoClockConfig {
    OffsetEpochNanoClockConfig::new(
        retries,
        Duration::from_nanos(threshold_ns),
        Duration::from_nanos(interval_ns),
    )
    .expect("test configuration should be valid")
}

#[test]
fn accepts_first_sample_within_threshold() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000]),
        ScriptedNanoClock::new([100, 104, 112]),
        config(3, 5, 1_000),
    )
    .expect("sample should succeed");

    assert!(clock.is_within_threshold());
    assert_eq!(1_000_000_010, clock.nano_time());
}

#[test]
fn uses_narrowest_fallback_sample() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000, 3_000]),
        ScriptedNanoClock::new([100, 120, 200, 208, 300, 306, 310]),
        config(3, 5, 1_000),
    )
    .expect("fallback sample should succeed");

    assert!(!clock.is_within_threshold());
    assert_eq!(3_000_000_007, clock.nano_time());
}

#[test]
fn saturates_epoch_conversion_then_wraps_output() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([i64::MAX]),
        ScriptedNanoClock::new([0, 0, 1]),
        config(1, 1, 1_000),
    )
    .expect("sample should succeed");

    assert_eq!(i64::MIN, clock.nano_time());
}

#[test]
fn threshold_comparison_is_strict() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000]),
        ScriptedNanoClock::new([100, 105]),
        config(1, 5, 1_000),
    )
    .expect("fallback sample should succeed");

    assert!(!clock.is_within_threshold());
}

#[test]
fn rejects_invalid_configuration() {
    assert_eq!(
        Err(OffsetEpochNanoClockError::ZeroMeasurementRetries),
        OffsetEpochNanoClockConfig::new(0, Duration::ZERO, Duration::from_nanos(1))
    );
    assert_eq!(
        Err(OffsetEpochNanoClockError::ResampleIntervalOutOfRange),
        OffsetEpochNanoClockConfig::new(1, Duration::ZERO, Duration::ZERO)
    );

    let too_large = Duration::from_nanos(i64::MAX as u64 + 1);
    assert_eq!(
        Err(OffsetEpochNanoClockError::MeasurementThresholdOutOfRange),
        OffsetEpochNanoClockConfig::new(1, too_large, Duration::from_nanos(1))
    );
    assert_eq!(
        Err(OffsetEpochNanoClockError::ResampleIntervalOutOfRange),
        OffsetEpochNanoClockConfig::new(1, Duration::ZERO, too_large)
    );
}

#[test]
fn defaults_match_agrona() {
    let config = OffsetEpochNanoClockConfig::default();

    assert_eq!(100, config.max_measurement_retries());
    assert_eq!(Duration::from_nanos(250), config.measurement_threshold());
    assert_eq!(Duration::from_secs(60 * 60), config.resample_interval());
}

#[test]
fn reports_when_every_window_is_invalid() {
    let error = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1, 2]),
        ScriptedNanoClock::new([2, 1, 4, 3]),
        config(2, 250, 1_000),
    )
    .expect_err("all backward windows should fail");

    assert_eq!(OffsetEpochNanoClockError::NoValidSample, error);
}

#[test]
fn resamples_after_interval_and_backward_movement() {
    let expired = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000]),
        ScriptedNanoClock::new([0, 0, 11, 12, 12, 13]),
        config(1, 1, 10),
    )
    .expect("initial sample should succeed");
    assert_eq!(2_000_000_001, expired.nano_time());

    let backward = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000]),
        ScriptedNanoClock::new([100, 100, 99, 90, 90, 91]),
        config(1, 1, 10),
    )
    .expect("initial sample should succeed");
    assert_eq!(2_000_000_001, backward.nano_time());
}

#[test]
fn automatic_resample_failure_retains_last_sample() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000, 3_000]),
        ScriptedNanoClock::new([100, 100, 99, 10, 9, 20, 19, 98]),
        config(2, 1, 10),
    )
    .expect("initial sample should succeed");

    assert_eq!(999_999_998, clock.nano_time());
    assert!(clock.is_within_threshold());
}

#[test]
fn system_sources_track_epoch_time() {
    let before = SystemEpochClock.time();
    let clock = OffsetEpochNanoClock::new().expect("system sample should work");
    let value_ms = clock.nano_time() / 1_000_000;
    let after = SystemEpochClock.time();

    assert!(value_ms >= before - 1);
    assert!(value_ms <= after + 1);
}

#[derive(Debug)]
struct ConstantEpochClock(i64);

impl EpochClock for ConstantEpochClock {
    fn time(&self) -> i64 {
        self.0
    }
}

#[derive(Debug)]
struct AtomicNanoClock(AtomicI64);

impl NanoClock for AtomicNanoClock {
    fn nano_time(&self) -> i64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

#[test]
fn supports_concurrent_reads_and_sampling() {
    let clock = Arc::new(
        OffsetEpochNanoClock::with_sources(
            ConstantEpochClock(1_000),
            AtomicNanoClock(AtomicI64::new(0)),
            config(1, 2, 1_000_000),
        )
        .expect("initial sample should succeed"),
    );
    let mut workers = Vec::new();

    for _ in 0..4 {
        let clock = Arc::clone(&clock);
        workers.push(thread::spawn(move || {
            for _ in 0..20_000 {
                let value = clock.nano_time();
                assert!((999_000_000..1_100_000_000).contains(&value));
            }
        }));
    }

    for _ in 0..2 {
        let clock = Arc::clone(&clock);
        workers.push(thread::spawn(move || {
            for _ in 0..1_000 {
                clock.sample().expect("concurrent sample should succeed");
            }
        }));
    }

    for worker in workers {
        worker.join().expect("clock worker should not panic");
    }
}

#[derive(Debug)]
struct OverlappingEpochClock {
    calls: AtomicUsize,
    concurrent_calls: AtomicUsize,
}

impl EpochClock for OverlappingEpochClock {
    fn time(&self) -> i64 {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        if call != 0 {
            self.concurrent_calls.fetch_add(1, Ordering::AcqRel);
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.concurrent_calls.load(Ordering::Acquire) != 2 {
                assert!(
                    Instant::now() < deadline,
                    "sampling calls were serialized instead of overlapping"
                );
                thread::yield_now();
            }
        }

        1_000
    }
}

#[test]
fn explicit_sampling_operations_overlap_without_serialization() {
    let clock = Arc::new(
        OffsetEpochNanoClock::with_sources(
            OverlappingEpochClock {
                calls: AtomicUsize::new(0),
                concurrent_calls: AtomicUsize::new(0),
            },
            AtomicNanoClock(AtomicI64::new(0)),
            config(1, 10, 1_000_000),
        )
        .expect("initial sample should succeed"),
    );

    let first = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || clock.sample())
    };
    let second = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || clock.sample())
    };

    first
        .join()
        .expect("first sampler should not panic")
        .expect("first sample should succeed");
    second
        .join()
        .expect("second sampler should not panic")
        .expect("second sample should succeed");
}

#[derive(Debug)]
struct OrderedPublicationNanoClock {
    calls: AtomicUsize,
    first_sampler_waiting: Arc<AtomicBool>,
    release_first_sampler: Arc<AtomicBool>,
}

impl NanoClock for OrderedPublicationNanoClock {
    fn nano_time(&self) -> i64 {
        match self.calls.fetch_add(1, Ordering::Relaxed) {
            0 | 1 => 0,
            2 => 100,
            3 => {
                self.first_sampler_waiting.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(2);
                while !self.release_first_sampler.load(Ordering::Acquire) {
                    assert!(
                        Instant::now() < deadline,
                        "second sampler did not complete in time"
                    );
                    thread::yield_now();
                }
                100
            }
            4 | 5 => 200,
            _ => 101,
        }
    }
}

#[test]
fn last_completed_atomic_replacement_becomes_current() {
    let first_sampler_waiting = Arc::new(AtomicBool::new(false));
    let release_first_sampler = Arc::new(AtomicBool::new(false));
    let clock = Arc::new(
        OffsetEpochNanoClock::with_sources(
            ScriptedEpochClock::new([1_000, 2_000, 3_000]),
            OrderedPublicationNanoClock {
                calls: AtomicUsize::new(0),
                first_sampler_waiting: Arc::clone(&first_sampler_waiting),
                release_first_sampler: Arc::clone(&release_first_sampler),
            },
            config(1, 1, 1_000_000),
        )
        .expect("initial sample should succeed"),
    );

    let first = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || clock.sample())
    };

    let deadline = Instant::now() + Duration::from_secs(2);
    while !first_sampler_waiting.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "first sampler did not reach its publication boundary"
        );
        thread::yield_now();
    }

    let second = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || clock.sample())
    };
    second
        .join()
        .expect("second sampler should not panic")
        .expect("second sample should succeed");

    release_first_sampler.store(true, Ordering::Release);
    first
        .join()
        .expect("first sampler should not panic")
        .expect("first sample should succeed");

    assert_eq!(2_000_000_001, clock.nano_time());
}

#[derive(Debug)]
struct PanicSwitchNanoClock {
    value: AtomicI64,
    should_panic: Arc<AtomicBool>,
}

impl NanoClock for PanicSwitchNanoClock {
    fn nano_time(&self) -> i64 {
        assert!(
            !self.should_panic.load(Ordering::Acquire),
            "scripted source panic"
        );
        self.value.fetch_add(1, Ordering::Relaxed)
    }
}

#[test]
fn a_panicking_sampler_does_not_poison_future_sampling() {
    let should_panic = Arc::new(AtomicBool::new(false));
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000]),
        PanicSwitchNanoClock {
            value: AtomicI64::new(0),
            should_panic: Arc::clone(&should_panic),
        },
        config(1, 2, 1_000),
    )
    .expect("initial sample should succeed");

    should_panic.store(true, Ordering::Release);
    let panic_result = std::panic::catch_unwind(|| {
        let _ = clock.sample();
    });
    assert!(panic_result.is_err());

    should_panic.store(false, Ordering::Release);
    clock
        .sample()
        .expect("sampling should recover without a poisoned lock");
}
