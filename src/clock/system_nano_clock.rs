// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::OnceLock;
use std::time::Instant;

use super::NanoClock;

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

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn provider_is_zero_sized() {
        assert_eq!(0, size_of::<SystemNanoClock>());
    }
}
