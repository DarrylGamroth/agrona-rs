// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use super::{EpochClock, EpochNanoClock, NanoClock, SystemEpochClock, SystemNanoClock};

const NANOS_PER_MILLI: i64 = 1_000_000;
const DEFAULT_MAX_MEASUREMENT_RETRIES: usize = 100;
const DEFAULT_MEASUREMENT_THRESHOLD: Duration = Duration::from_nanos(250);
const DEFAULT_RESAMPLE_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Configuration for an [`OffsetEpochNanoClock`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OffsetEpochNanoClockConfig {
    max_measurement_retries: usize,
    measurement_threshold_ns: i64,
    resample_interval_ns: i64,
}

impl OffsetEpochNanoClockConfig {
    /// Construct and validate offset-clock configuration.
    pub fn new(
        max_measurement_retries: usize,
        measurement_threshold: Duration,
        resample_interval: Duration,
    ) -> Result<Self, OffsetEpochNanoClockError> {
        if max_measurement_retries == 0 {
            return Err(OffsetEpochNanoClockError::ZeroMeasurementRetries);
        }

        let measurement_threshold_ns = duration_as_i64_nanos(measurement_threshold)
            .ok_or(OffsetEpochNanoClockError::MeasurementThresholdOutOfRange)?;
        let resample_interval_ns = duration_as_i64_nanos(resample_interval)
            .ok_or(OffsetEpochNanoClockError::ResampleIntervalOutOfRange)?;

        if resample_interval_ns == 0 {
            return Err(OffsetEpochNanoClockError::ResampleIntervalOutOfRange);
        }

        Ok(Self {
            max_measurement_retries,
            measurement_threshold_ns,
            resample_interval_ns,
        })
    }

    /// Maximum number of attempts made by one sampling operation.
    pub fn max_measurement_retries(&self) -> usize {
        self.max_measurement_retries
    }

    /// Desired sampling-window threshold.
    pub fn measurement_threshold(&self) -> Duration {
        Duration::from_nanos(self.measurement_threshold_ns as u64)
    }

    /// Interval after which the offset is sampled again.
    pub fn resample_interval(&self) -> Duration {
        Duration::from_nanos(self.resample_interval_ns as u64)
    }
}

impl Default for OffsetEpochNanoClockConfig {
    fn default() -> Self {
        Self {
            max_measurement_retries: DEFAULT_MAX_MEASUREMENT_RETRIES,
            measurement_threshold_ns: DEFAULT_MEASUREMENT_THRESHOLD.as_nanos() as i64,
            resample_interval_ns: DEFAULT_RESAMPLE_INTERVAL.as_nanos() as i64,
        }
    }
}

/// Error returned while configuring or sampling an offset epoch clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OffsetEpochNanoClockError {
    /// The maximum measurement retry count was zero.
    ZeroMeasurementRetries,
    /// The threshold could not be represented as non-negative `i64`
    /// nanoseconds.
    MeasurementThresholdOutOfRange,
    /// The interval was zero or could not be represented as positive `i64`
    /// nanoseconds.
    ResampleIntervalOutOfRange,
    /// Every measurement window was invalid because monotonic time moved
    /// backward or overflowed.
    NoValidSample,
}

impl fmt::Display for OffsetEpochNanoClockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroMeasurementRetries => "max measurement retries must be positive",
            Self::MeasurementThresholdOutOfRange => {
                "measurement threshold must fit in non-negative i64 nanoseconds"
            }
            Self::ResampleIntervalOutOfRange => {
                "resample interval must fit in positive i64 nanoseconds"
            }
            Self::NoValidSample => "monotonic clock moved backward during every sampling attempt",
        })
    }
}

impl Error for OffsetEpochNanoClockError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Sample {
    initial_epoch_ns: i64,
    initial_nano_time: i64,
    within_threshold: bool,
}

/// Epoch-nanosecond clock derived from sampled epoch milliseconds and a
/// monotonic nanosecond source.
///
/// As in Agrona Java, each sampler independently builds an immutable sample
/// and atomically replaces the published snapshot. Sampling is not serialized,
/// and normal reads are lock-free and allocation-free.
#[derive(Debug)]
pub struct OffsetEpochNanoClock<E = SystemEpochClock, N = SystemNanoClock> {
    epoch_clock: E,
    nano_clock: N,
    config: OffsetEpochNanoClockConfig,
    sample: ArcSwap<Sample>,
}

impl OffsetEpochNanoClock<SystemEpochClock, SystemNanoClock> {
    /// Construct an offset clock using system epoch and monotonic sources.
    pub fn new() -> Result<Self, OffsetEpochNanoClockError> {
        Self::with_sources(
            SystemEpochClock,
            SystemNanoClock,
            OffsetEpochNanoClockConfig::default(),
        )
    }
}

impl<E, N> OffsetEpochNanoClock<E, N>
where
    E: EpochClock,
    N: NanoClock,
{
    /// Construct an offset clock with injected sources and configuration.
    pub fn with_sources(
        epoch_clock: E,
        nano_clock: N,
        config: OffsetEpochNanoClockConfig,
    ) -> Result<Self, OffsetEpochNanoClockError> {
        let clock = Self {
            epoch_clock,
            nano_clock,
            config,
            sample: ArcSwap::from_pointee(Sample {
                initial_epoch_ns: 0,
                initial_nano_time: 0,
                within_threshold: false,
            }),
        };
        clock.sample()?;
        Ok(clock)
    }

    /// Return this clock's configuration.
    pub fn config(&self) -> OffsetEpochNanoClockConfig {
        self.config
    }

    /// Explicitly sample the relationship between epoch and monotonic time.
    pub fn sample(&self) -> Result<(), OffsetEpochNanoClockError> {
        let sample = self.measure_sample()?;
        self.sample.store(Arc::new(sample));
        Ok(())
    }

    /// Return whether the current sample met the configured threshold.
    #[inline]
    pub fn is_within_threshold(&self) -> bool {
        self.sample.load_full().within_threshold
    }

    fn measure_sample(&self) -> Result<Sample, OffsetEpochNanoClockError> {
        let mut best_sample = None;
        let mut best_window_ns = i64::MAX;

        for _ in 0..self.config.max_measurement_retries {
            let first_nano_time = self.nano_clock.nano_time();
            let epoch_ms = self.epoch_clock.time();
            let second_nano_time = self.nano_clock.nano_time();

            let Some(window_ns) = second_nano_time.checked_sub(first_nano_time) else {
                continue;
            };
            if window_ns < 0 {
                continue;
            }

            let sample = Sample {
                initial_epoch_ns: epoch_ms.saturating_mul(NANOS_PER_MILLI),
                initial_nano_time: first_nano_time.wrapping_add(window_ns >> 1),
                within_threshold: window_ns < self.config.measurement_threshold_ns,
            };

            if sample.within_threshold {
                return Ok(sample);
            }

            if window_ns < best_window_ns {
                best_window_ns = window_ns;
                best_sample = Some(sample);
            }
        }

        best_sample.ok_or(OffsetEpochNanoClockError::NoValidSample)
    }

    #[inline]
    fn time_from_sample(&self, sample: Sample, nano_time: i64) -> i64 {
        sample
            .initial_epoch_ns
            .wrapping_add(nano_time.wrapping_sub(sample.initial_nano_time))
    }
}

impl<E, N> EpochNanoClock for OffsetEpochNanoClock<E, N>
where
    E: EpochClock,
    N: NanoClock,
{
    #[inline]
    fn nano_time(&self) -> i64 {
        loop {
            let sample = *self.sample.load_full();
            let nano_time = self.nano_clock.nano_time();
            let adjustment = nano_time.wrapping_sub(sample.initial_nano_time);

            if adjustment < 0 || adjustment > self.config.resample_interval_ns {
                if self.sample().is_err() {
                    let current_nano_time = self.nano_clock.nano_time();
                    return self.time_from_sample(sample, current_nano_time);
                }
                continue;
            }

            return self.time_from_sample(sample, nano_time);
        }
    }
}

fn duration_as_i64_nanos(duration: Duration) -> Option<i64> {
    i64::try_from(duration.as_nanos()).ok()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    use super::*;

    #[test]
    fn immutable_sample_is_coherent_during_concurrent_replacement() {
        let published = Arc::new(ArcSwap::from_pointee(Sample {
            initial_epoch_ns: 0,
            initial_nano_time: !0,
            within_threshold: true,
        }));
        let running = Arc::new(AtomicBool::new(true));

        let writer_sample = Arc::clone(&published);
        let writer_running = Arc::clone(&running);
        let writer = thread::spawn(move || {
            for value in 1..100_000_i64 {
                writer_sample.store(Arc::new(Sample {
                    initial_epoch_ns: value,
                    initial_nano_time: !value,
                    within_threshold: value & 1 == 0,
                }));
            }
            writer_running.store(false, Ordering::Release);
        });

        while running.load(Ordering::Acquire) {
            let sample = *published.load_full();
            assert_eq!(!sample.initial_epoch_ns, sample.initial_nano_time);
            assert_eq!(sample.initial_epoch_ns & 1 == 0, sample.within_threshold);
        }

        writer.join().expect("writer should not panic");
    }
}
