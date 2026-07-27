// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral and concurrency acceptance tests for `DEC-CLOCK-001`.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agrona::clock::{
    CachedEpochClock, CachedNanoClock, EpochClock, EpochMicroClock, EpochNanoClock, NanoClock,
    OffsetEpochNanoClock, OffsetEpochNanoClockConfig, OffsetEpochNanoClockError, SystemEpochClock,
    SystemEpochMicroClock, SystemEpochNanoClock, SystemNanoClock,
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

fn offset_config(
    retries: usize,
    threshold_ns: u64,
    interval_ns: u64,
) -> OffsetEpochNanoClockConfig {
    OffsetEpochNanoClockConfig::new(
        retries,
        Duration::from_nanos(threshold_ns),
        Duration::from_nanos(interval_ns),
    )
    .expect("test configuration should be valid")
}

#[test]
fn provider_traits_are_object_safe_and_domains_are_distinct() {
    let epoch: Box<dyn EpochClock> = Box::new(SystemEpochClock);
    let epoch_micro: Box<dyn EpochMicroClock> = Box::new(SystemEpochMicroClock);
    let epoch_nano: Box<dyn EpochNanoClock> = Box::new(SystemEpochNanoClock);
    let monotonic: Box<dyn NanoClock> = Box::new(SystemNanoClock);

    assert!(epoch.time() > 0);
    assert!(epoch_micro.micro_time() > 0);
    assert!(epoch_nano.nano_time() > 0);
    assert!(monotonic.nano_time() >= 0);
}

#[test]
fn system_epoch_clocks_track_system_time() {
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should follow the Unix epoch");

    let millis = SystemEpochClock.time();
    let micros = SystemEpochMicroClock.micro_time();
    let nanos = SystemEpochNanoClock.nano_time();

    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should follow the Unix epoch");
    let tolerance = Duration::from_secs(1);

    assert!(millis >= before.saturating_sub(tolerance).as_millis() as i64);
    assert!(millis <= after.saturating_add(tolerance).as_millis() as i64);
    assert!(micros >= before.saturating_sub(tolerance).as_micros() as i64);
    assert!(micros <= after.saturating_add(tolerance).as_micros() as i64);
    assert!(nanos >= before.saturating_sub(tolerance).as_nanos() as i64);
    assert!(nanos <= after.saturating_add(tolerance).as_nanos() as i64);
}

#[test]
fn system_nano_clock_is_nondecreasing_and_measures_elapsed_time() {
    let clock = SystemNanoClock;
    let samples: Vec<_> = (0..1_000).map(|_| clock.nano_time()).collect();
    assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));

    let before = clock.nano_time();
    thread::sleep(Duration::from_millis(2));
    assert!(clock.nano_time() > before);
}

#[test]
fn cached_clocks_update_advance_and_wrap() {
    let mut epoch = CachedEpochClock::new();
    let epoch_reader = epoch.reader();
    let mut nano = CachedNanoClock::with_initial_time(i64::MAX);
    let nano_reader = nano.reader();

    assert_eq!(0, epoch.time());
    assert_eq!(0, epoch_reader.time());
    epoch.update(333);
    assert_eq!(333, epoch_reader.time());
    assert_eq!(340, epoch.advance(7));
    assert_eq!(340, epoch_reader.time());

    assert_eq!(i64::MAX, nano.nano_time());
    assert_eq!(i64::MIN, nano.advance(1));
    assert_eq!(i64::MIN, nano_reader.nano_time());
    nano.update(777);
    assert_eq!(777, nano_reader.nano_time());
}

#[test]
fn cached_clock_publishes_to_multiple_readers() {
    let mut writer = CachedNanoClock::new();
    let final_reader = writer.reader();
    let barrier = Arc::new(Barrier::new(4));
    let mut readers = Vec::new();

    for _ in 0..3 {
        let reader = writer.reader();
        let barrier = Arc::clone(&barrier);
        readers.push(thread::spawn(move || {
            barrier.wait();
            let mut previous = i64::MIN;
            for _ in 0..100_000 {
                let current = reader.nano_time();
                assert!(current >= previous);
                previous = current;
            }
        }));
    }

    barrier.wait();
    for value in 1..=100_000 {
        writer.update(value);
    }

    for reader in readers {
        reader.join().expect("reader should not panic");
    }
    assert_eq!(100_000, final_reader.nano_time());
}

#[test]
fn offset_clock_accepts_first_sample_within_threshold() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000]),
        ScriptedNanoClock::new([100, 104, 112]),
        offset_config(3, 5, 1_000),
    )
    .expect("sample should succeed");

    assert!(clock.is_within_threshold());
    assert_eq!(1_000_000_010, clock.nano_time());
}

#[test]
fn offset_clock_uses_narrowest_fallback_sample() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000, 3_000]),
        ScriptedNanoClock::new([100, 120, 200, 208, 300, 306, 310]),
        offset_config(3, 5, 1_000),
    )
    .expect("fallback sample should succeed");

    assert!(!clock.is_within_threshold());
    assert_eq!(3_000_000_007, clock.nano_time());
}

#[test]
fn offset_clock_saturates_epoch_conversion_then_wraps_output() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([i64::MAX]),
        ScriptedNanoClock::new([0, 0, 1]),
        offset_config(1, 1, 1_000),
    )
    .expect("sample should succeed");

    assert_eq!(i64::MIN, clock.nano_time());
}

#[test]
fn offset_clock_threshold_comparison_is_strict() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000]),
        ScriptedNanoClock::new([100, 105]),
        offset_config(1, 5, 1_000),
    )
    .expect("fallback sample should succeed");

    assert!(!clock.is_within_threshold());
}

#[test]
fn offset_clock_rejects_invalid_configuration() {
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
fn offset_clock_defaults_match_agrona() {
    let config = OffsetEpochNanoClockConfig::default();

    assert_eq!(100, config.max_measurement_retries());
    assert_eq!(Duration::from_nanos(250), config.measurement_threshold());
    assert_eq!(Duration::from_secs(60 * 60), config.resample_interval());
}

#[test]
fn offset_clock_reports_when_every_window_is_invalid() {
    let error = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1, 2]),
        ScriptedNanoClock::new([2, 1, 4, 3]),
        offset_config(2, 250, 1_000),
    )
    .expect_err("all backward windows should fail");

    assert_eq!(OffsetEpochNanoClockError::NoValidSample, error);
}

#[test]
fn offset_clock_resamples_after_interval_and_backward_movement() {
    let expired = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000]),
        ScriptedNanoClock::new([0, 0, 11, 12, 12, 13]),
        offset_config(1, 1, 10),
    )
    .expect("initial sample should succeed");
    assert_eq!(2_000_000_001, expired.nano_time());

    let backward = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000]),
        ScriptedNanoClock::new([100, 100, 99, 90, 90, 91]),
        offset_config(1, 1, 10),
    )
    .expect("initial sample should succeed");
    assert_eq!(2_000_000_001, backward.nano_time());
}

#[test]
fn automatic_resample_failure_retains_last_sample() {
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000, 3_000]),
        ScriptedNanoClock::new([100, 100, 99, 10, 9, 20, 19, 98]),
        offset_config(2, 1, 10),
    )
    .expect("initial sample should succeed");

    assert_eq!(999_999_998, clock.nano_time());
    assert!(clock.is_within_threshold());
}

#[test]
fn offset_system_clock_tracks_epoch_time() {
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
fn offset_clock_supports_concurrent_reads_and_sampling() {
    let clock = Arc::new(
        OffsetEpochNanoClock::with_sources(
            ConstantEpochClock(1_000),
            AtomicNanoClock(AtomicI64::new(0)),
            offset_config(1, 2, 1_000_000),
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
fn explicit_sample_reports_a_poisoned_sampling_lock() {
    let should_panic = Arc::new(AtomicBool::new(false));
    let clock = OffsetEpochNanoClock::with_sources(
        ScriptedEpochClock::new([1_000, 2_000]),
        PanicSwitchNanoClock {
            value: AtomicI64::new(0),
            should_panic: Arc::clone(&should_panic),
        },
        offset_config(1, 2, 1_000),
    )
    .expect("initial sample should succeed");

    should_panic.store(true, Ordering::Release);
    let panic_result = std::panic::catch_unwind(|| {
        let _ = clock.sample();
    });
    assert!(panic_result.is_err());

    should_panic.store(false, Ordering::Release);
    assert_eq!(
        Err(OffsetEpochNanoClockError::SamplingLockPoisoned),
        clock.sample()
    );
}
