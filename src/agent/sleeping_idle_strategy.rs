// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::IdleStrategy;
use std::time::Duration;

/// Parks the thread for a nanosecond-scale duration when idle.
#[derive(Clone, Copy, Debug)]
pub struct SleepingIdleStrategy {
    duration: Duration,
}

impl SleepingIdleStrategy {
    /// Java-compatible default sleep period in nanoseconds.
    pub const DEFAULT_SLEEP_PERIOD_NS: u64 = 1_000;
    /// Creates a strategy with the supplied park duration.
    #[must_use]
    pub const fn new(duration: Duration) -> Self {
        Self { duration }
    }
    /// Returns the configured duration.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }
}

impl Default for SleepingIdleStrategy {
    fn default() -> Self {
        Self::new(Duration::from_nanos(Self::DEFAULT_SLEEP_PERIOD_NS))
    }
}

impl IdleStrategy for SleepingIdleStrategy {
    fn idle_once(&mut self) {
        std::thread::park_timeout(self.duration);
    }
    fn alias(&self) -> &'static str {
        "sleep-ns"
    }
}
