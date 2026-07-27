// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use super::{EpochClock, EpochMicroClock, EpochNanoClock, NanoClock};

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Allocation-free provider of milliseconds since 1 January 1970 UTC.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEpochClock;

impl EpochClock for SystemEpochClock {
    #[inline]
    fn time(&self) -> i64 {
        signed_epoch_units(SystemTime::now(), 1_000)
    }
}

/// Allocation-free provider of microseconds since 1 January 1970 UTC.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEpochMicroClock;

impl EpochMicroClock for SystemEpochMicroClock {
    #[inline]
    fn micro_time(&self) -> i64 {
        signed_epoch_units(SystemTime::now(), 1_000_000)
    }
}

/// Allocation-free provider of nanoseconds since 1 January 1970 UTC.
///
/// The returned unit does not imply that the operating system clock has
/// nanosecond resolution.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEpochNanoClock;

impl EpochNanoClock for SystemEpochNanoClock {
    #[inline]
    fn nano_time(&self) -> i64 {
        signed_epoch_units(SystemTime::now(), NANOS_PER_SECOND)
    }
}

/// Allocation-free provider of process-local monotonic nanosecond ticks.
///
/// The origin is initialized on the first read and deliberately unspecified.
/// Values can wrap and are suitable only for elapsed-time measurement.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemNanoClock;

impl NanoClock for SystemNanoClock {
    #[inline]
    fn nano_time(&self) -> i64 {
        static ORIGIN: OnceLock<Instant> = OnceLock::new();

        let origin = ORIGIN.get_or_init(Instant::now);
        Instant::now().duration_since(*origin).as_nanos() as i64
    }
}

#[inline]
fn signed_epoch_units(time: SystemTime, units_per_second: u128) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration_units(duration, units_per_second) as i64,
        Err(error) => {
            (duration_units_ceil(error.duration(), units_per_second) as i64).wrapping_neg()
        }
    }
}

#[inline]
fn duration_units(duration: std::time::Duration, units_per_second: u128) -> u128 {
    let whole = u128::from(duration.as_secs()).wrapping_mul(units_per_second);
    let fraction =
        u128::from(duration.subsec_nanos()).wrapping_mul(units_per_second) / NANOS_PER_SECOND;

    whole.wrapping_add(fraction)
}

#[inline]
fn duration_units_ceil(duration: std::time::Duration, units_per_second: u128) -> u128 {
    let whole = u128::from(duration.as_secs()).wrapping_mul(units_per_second);
    let fraction_numerator = u128::from(duration.subsec_nanos()).wrapping_mul(units_per_second);
    let fraction = fraction_numerator.div_ceil(NANOS_PER_SECOND);

    whole.wrapping_add(fraction)
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    #[test]
    fn system_providers_are_zero_sized() {
        assert_eq!(0, size_of::<SystemEpochClock>());
        assert_eq!(0, size_of::<SystemEpochMicroClock>());
        assert_eq!(0, size_of::<SystemEpochNanoClock>());
        assert_eq!(0, size_of::<SystemNanoClock>());
    }

    #[test]
    fn signed_epoch_conversion_handles_both_sides_of_epoch() {
        // Windows represents SystemTime in 100-nanosecond intervals.
        let representable_duration = Duration::new(2, 345_678_900);
        let after = UNIX_EPOCH + representable_duration;
        let before = UNIX_EPOCH - representable_duration;

        assert_eq!(2_345, signed_epoch_units(after, 1_000));
        assert_eq!(-2_346, signed_epoch_units(before, 1_000));
        assert_eq!(2_345_678, signed_epoch_units(after, 1_000_000));
        assert_eq!(-2_345_679, signed_epoch_units(before, 1_000_000));
        assert_eq!(2_345_678_900, signed_epoch_units(after, NANOS_PER_SECOND));
        assert_eq!(-2_345_678_900, signed_epoch_units(before, NANOS_PER_SECOND));

        let exact_duration = Duration::new(2, 345_678_901);
        assert_eq!(
            2_345_678_901,
            duration_units(exact_duration, NANOS_PER_SECOND)
        );
    }

    #[test]
    fn unit_conversion_wraps_at_i64_boundary() {
        let duration = Duration::from_secs(i64::MAX as u64 + 1);

        assert_eq!(i64::MIN, duration_units(duration, 1) as i64);
    }
}
