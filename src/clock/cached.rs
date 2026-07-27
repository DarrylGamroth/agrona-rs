// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use super::{EpochClock, NanoClock};

#[repr(align(128))]
#[derive(Debug)]
struct PaddedAtomicI64(AtomicI64);

impl PaddedAtomicI64 {
    fn new(value: i64) -> Self {
        Self(AtomicI64::new(value))
    }

    #[inline]
    fn load(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }

    #[inline]
    fn store(&self, value: i64) {
        self.0.store(value, Ordering::Release);
    }

    #[inline]
    fn advance(&self, delta: i64) -> i64 {
        let value = self.0.load(Ordering::Relaxed).wrapping_add(delta);
        self.0.store(value, Ordering::Release);
        value
    }
}

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
    value: Arc<PaddedAtomicI64>,
}

impl CachedEpochClock {
    /// Construct a cached epoch clock initialized to zero.
    pub fn new() -> Self {
        Self::with_initial_time(0)
    }

    /// Construct a cached epoch clock with an explicit initial value.
    pub fn with_initial_time(time_ms: i64) -> Self {
        Self {
            value: Arc::new(PaddedAtomicI64::new(time_ms)),
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
    value: Arc<PaddedAtomicI64>,
}

impl EpochClock for CachedEpochClockReader {
    #[inline]
    fn time(&self) -> i64 {
        self.value.load()
    }
}

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
    value: Arc<PaddedAtomicI64>,
}

impl CachedNanoClock {
    /// Construct a cached monotonic clock initialized to zero.
    pub fn new() -> Self {
        Self::with_initial_time(0)
    }

    /// Construct a cached monotonic clock with an explicit initial value.
    pub fn with_initial_time(time_ns: i64) -> Self {
        Self {
            value: Arc::new(PaddedAtomicI64::new(time_ns)),
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
    value: Arc<PaddedAtomicI64>,
}

impl NanoClock for CachedNanoClockReader {
    #[inline]
    fn nano_time(&self) -> i64 {
        self.value.load()
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn padded_value_occupies_its_alignment() {
        assert_eq!(128, align_of::<PaddedAtomicI64>());
        assert_eq!(128, size_of::<PaddedAtomicI64>());
    }
}
