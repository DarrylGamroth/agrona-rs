// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use super::EpochClock;
use super::system_time::signed_epoch_units;

/// Allocation-free provider of milliseconds since 1 January 1970 UTC.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEpochClock;

impl EpochClock for SystemEpochClock {
    #[inline]
    fn time(&self) -> i64 {
        signed_epoch_units(SystemTime::now(), 1_000)
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn provider_is_zero_sized() {
        assert_eq!(0, size_of::<SystemEpochClock>());
    }
}
