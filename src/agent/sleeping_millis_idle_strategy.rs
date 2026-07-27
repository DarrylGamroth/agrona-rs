// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::IdleStrategy;
use std::time::Duration;

/// Sleeps the thread for a millisecond-scale duration when idle.
#[derive(Clone, Copy, Debug)]
pub struct SleepingMillisIdleStrategy {
    duration: Duration,
}

impl SleepingMillisIdleStrategy {
    /// Java-compatible default sleep period.
    pub const DEFAULT_SLEEP_PERIOD: Duration = Duration::from_millis(1);
    /// Creates a strategy with the supplied sleep duration.
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

impl Default for SleepingMillisIdleStrategy {
    fn default() -> Self {
        Self::new(Self::DEFAULT_SLEEP_PERIOD)
    }
}

impl IdleStrategy for SleepingMillisIdleStrategy {
    fn idle_once(&mut self) {
        std::thread::sleep(self.duration);
    }
    fn alias(&self) -> &'static str {
        "sleep-ms"
    }
}
