// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::EpochClock;
use super::atomic_clock_value::AtomicClockValue;

/// Single-writer cached provider of epoch milliseconds.
///
/// The writer is intentionally not cloneable. Create cloneable read-only
/// handles with [`Self::reader`].
///
/// ```compile_fail
/// use agrona::clock::CachedEpochClock;
///
/// let writer = CachedEpochClock::new();
/// let second_writer = writer.clone();
/// ```
#[derive(Debug)]
pub struct CachedEpochClock {
    value: Arc<AtomicClockValue>,
}

impl CachedEpochClock {
    /// Construct a cached epoch clock initialized to zero.
    pub fn new() -> Self {
        Self::with_initial_time(0)
    }

    /// Construct a cached epoch clock with an explicit initial value.
    pub fn with_initial_time(time_ms: i64) -> Self {
        Self {
            value: Arc::new(AtomicClockValue::new(time_ms)),
        }
    }

    /// Create a cloneable read-only handle.
    pub fn reader(&self) -> CachedEpochClockReader {
        CachedEpochClockReader {
            value: Arc::clone(&self.value),
        }
    }

    /// Publish an absolute epoch-millisecond value with release ordering.
    #[inline]
    pub fn update(&mut self, time_ms: i64) {
        self.value.store(time_ms);
    }

    /// Advance the cached value with wrapping arithmetic and release ordering.
    ///
    /// This is a single-writer operation, not a multi-writer atomic
    /// increment.
    #[inline]
    pub fn advance(&mut self, millis: i64) -> i64 {
        self.value.advance(millis)
    }
}

impl Default for CachedEpochClock {
    fn default() -> Self {
        Self::new()
    }
}

impl EpochClock for CachedEpochClock {
    #[inline]
    fn time(&self) -> i64 {
        self.value.load()
    }
}

/// Cloneable read-only handle for a [`CachedEpochClock`].
#[derive(Clone, Debug)]
pub struct CachedEpochClockReader {
    value: Arc<AtomicClockValue>,
}

impl EpochClock for CachedEpochClockReader {
    #[inline]
    fn time(&self) -> i64 {
        self.value.load()
    }
}
