// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::IdleStrategy;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    NotIdle,
    Spinning,
    Yielding,
    Parking,
}

/// Spins, yields, then parks with capped exponential backoff.
#[derive(Clone, Debug)]
pub struct BackoffIdleStrategy {
    max_spins: u64,
    max_yields: u64,
    min_park: Duration,
    max_park: Duration,
    spins: u64,
    yields: u64,
    park: Duration,
    state: State,
}

impl BackoffIdleStrategy {
    /// Java-compatible maximum spin count.
    pub const DEFAULT_MAX_SPINS: u64 = 10;
    /// Java-compatible maximum yield count.
    pub const DEFAULT_MAX_YIELDS: u64 = 5;
    /// Java-compatible minimum park period.
    pub const DEFAULT_MIN_PARK: Duration = Duration::from_nanos(1_000);
    /// Java-compatible maximum park period.
    pub const DEFAULT_MAX_PARK: Duration = Duration::from_nanos(1_000_000);

    /// Creates a backoff strategy without adding policy validation.
    ///
    /// Java-equivalent arithmetic is claimed when both counts are at most
    /// `i64::MAX` and both durations are at most `i64::MAX / 2` nanoseconds.
    /// Larger Rust inputs remain safe and use wrapping counters and saturating
    /// duration doubling.
    #[must_use]
    pub const fn new(
        max_spins: u64,
        max_yields: u64,
        min_park: Duration,
        max_park: Duration,
    ) -> Self {
        Self {
            max_spins,
            max_yields,
            min_park,
            max_park,
            spins: 0,
            yields: 0,
            park: min_park,
            state: State::NotIdle,
        }
    }
}

impl Default for BackoffIdleStrategy {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_MAX_SPINS,
            Self::DEFAULT_MAX_YIELDS,
            Self::DEFAULT_MIN_PARK,
            Self::DEFAULT_MAX_PARK,
        )
    }
}

impl IdleStrategy for BackoffIdleStrategy {
    fn idle_once(&mut self) {
        match self.state {
            State::NotIdle => {
                self.state = State::Spinning;
                self.spins = self.spins.wrapping_add(1);
            }
            State::Spinning => {
                std::hint::spin_loop();
                self.spins = self.spins.wrapping_add(1);
                if self.spins > self.max_spins {
                    self.state = State::Yielding;
                    self.yields = 0;
                }
            }
            State::Yielding => {
                self.yields = self.yields.wrapping_add(1);
                if self.yields > self.max_yields {
                    self.state = State::Parking;
                    self.park = self.min_park;
                } else {
                    std::thread::yield_now();
                }
            }
            State::Parking => {
                std::thread::park_timeout(self.park);
                self.park = self.park.saturating_mul(2).min(self.max_park);
            }
        }
    }

    fn reset(&mut self) {
        self.spins = 0;
        self.yields = 0;
        self.park = self.min_park;
        self.state = State::NotIdle;
    }
    fn alias(&self) -> &'static str {
        "backoff"
    }
}

#[cfg(test)]
mod tests {
    use super::{BackoffIdleStrategy, IdleStrategy, State};
    use std::time::Duration;

    #[test]
    fn follows_java_transition_boundaries_and_reset() {
        let mut strategy =
            BackoffIdleStrategy::new(2, 1, Duration::from_nanos(1), Duration::from_nanos(4));

        strategy.idle_once();
        assert_eq!((State::Spinning, 1, 0), state(&strategy));
        strategy.idle_once();
        assert_eq!((State::Spinning, 2, 0), state(&strategy));
        strategy.idle_once();
        assert_eq!((State::Yielding, 3, 0), state(&strategy));
        strategy.idle_once();
        assert_eq!((State::Yielding, 3, 1), state(&strategy));
        strategy.idle_once();
        assert_eq!((State::Parking, 3, 2), state(&strategy));
        assert_eq!(Duration::from_nanos(1), strategy.park);
        strategy.idle_once();
        assert_eq!(Duration::from_nanos(2), strategy.park);
        strategy.idle_once();
        assert_eq!(Duration::from_nanos(4), strategy.park);
        strategy.idle_once();
        assert_eq!(Duration::from_nanos(4), strategy.park);

        strategy.idle(1);
        assert_eq!((State::NotIdle, 0, 0), state(&strategy));
        assert_eq!(Duration::from_nanos(1), strategy.park);
    }

    #[test]
    fn defaults_match_java() {
        let strategy = BackoffIdleStrategy::default();
        assert_eq!(10, strategy.max_spins);
        assert_eq!(5, strategy.max_yields);
        assert_eq!(Duration::from_nanos(1_000), strategy.min_park);
        assert_eq!(Duration::from_nanos(1_000_000), strategy.max_park);
    }

    fn state(strategy: &BackoffIdleStrategy) -> (State, u64, u64) {
        (strategy.state, strategy.spins, strategy.yields)
    }
}
