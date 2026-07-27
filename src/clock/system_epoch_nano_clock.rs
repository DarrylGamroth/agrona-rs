// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use super::EpochNanoClock;
use super::system_time::{NANOS_PER_SECOND, signed_epoch_units};

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

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn provider_is_zero_sized() {
        assert_eq!(0, size_of::<SystemEpochNanoClock>());
    }
}
