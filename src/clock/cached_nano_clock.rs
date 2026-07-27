// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

use std::sync::Arc;

use super::NanoClock;
use super::atomic_clock_value::AtomicClockValue;

/// Single-writer cached provider of monotonic nanosecond ticks.
///
/// The writer is intentionally not cloneable. Create cloneable read-only
/// handles with [`Self::reader`].
///
/// ```compile_fail
/// use agrona::clock::CachedNanoClock;
///
/// let writer = CachedNanoClock::new();
/// let second_writer = writer.clone();
/// ```
#[derive(Debug)]
pub struct CachedNanoClock {
    value: Arc<AtomicClockValue>,
}

impl CachedNanoClock {
    /// Construct a cached monotonic clock initialized to zero.
    pub fn new() -> Self {
        Self::with_initial_time(0)
    }

    /// Construct a cached monotonic clock with an explicit initial value.
    pub fn with_initial_time(time_ns: i64) -> Self {
        Self {
            value: Arc::new(AtomicClockValue::new(time_ns)),
        }
    }

    /// Create a cloneable read-only handle.
    pub fn reader(&self) -> CachedNanoClockReader {
        CachedNanoClockReader {
            value: Arc::clone(&self.value),
        }
    }

    /// Publish an absolute monotonic-nanosecond value with release ordering.
    #[inline]
    pub fn update(&mut self, time_ns: i64) {
        self.value.store(time_ns);
    }

    /// Advance the cached value with wrapping arithmetic and release ordering.
    ///
    /// This is a single-writer operation, not a multi-writer atomic
    /// increment.
    #[inline]
    pub fn advance(&mut self, nanos: i64) -> i64 {
        self.value.advance(nanos)
    }
}

impl Default for CachedNanoClock {
    fn default() -> Self {
        Self::new()
    }
}

impl NanoClock for CachedNanoClock {
    #[inline]
    fn nano_time(&self) -> i64 {
        self.value.load()
    }
}

/// Cloneable read-only handle for a [`CachedNanoClock`].
#[derive(Clone, Debug)]
pub struct CachedNanoClockReader {
    value: Arc<AtomicClockValue>,
}

impl NanoClock for CachedNanoClockReader {
    #[inline]
    fn nano_time(&self) -> i64 {
        self.value.load()
    }
}
