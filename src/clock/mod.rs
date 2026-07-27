// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

//! Epoch and monotonic clock providers.
//!
//! The provider traits deliberately keep epoch timestamps separate from
//! arbitrary-origin monotonic ticks:
//!
//! ```
//! use agrona::clock::{
//!     EpochClock, NanoClock, SystemEpochClock, SystemNanoClock,
//! };
//!
//! let epoch_ms = SystemEpochClock.time();
//! let start_ns = SystemNanoClock.nano_time();
//! let elapsed_ns = SystemNanoClock.nano_time().wrapping_sub(start_ns);
//!
//! assert!(epoch_ms > 0);
//! assert!(elapsed_ns >= 0);
//! ```
//!
//! Epoch nanoseconds cannot be used where monotonic nanoseconds are required:
//!
//! ```compile_fail
//! use agrona::clock::{NanoClock, SystemEpochNanoClock};
//!
//! fn elapsed_source(_: &dyn NanoClock) {}
//!
//! elapsed_source(&SystemEpochNanoClock);
//! ```

#[cfg(not(target_has_atomic = "64"))]
compile_error!("the agrona-rs Clock module requires native 64-bit atomics");

mod cached;
mod offset;
mod system;

pub use cached::{
    CachedEpochClock, CachedEpochClockReader, CachedNanoClock, CachedNanoClockReader,
};
pub use offset::{OffsetEpochNanoClock, OffsetEpochNanoClockConfig, OffsetEpochNanoClockError};
pub use system::{SystemEpochClock, SystemEpochMicroClock, SystemEpochNanoClock, SystemNanoClock};

/// Provider of milliseconds since 1 January 1970 UTC.
pub trait EpochClock {
    /// Return milliseconds since 1 January 1970 UTC.
    fn time(&self) -> i64;
}

/// Provider of microseconds since 1 January 1970 UTC.
pub trait EpochMicroClock {
    /// Return microseconds since 1 January 1970 UTC.
    fn micro_time(&self) -> i64;
}

/// Provider of nanoseconds since 1 January 1970 UTC.
pub trait EpochNanoClock {
    /// Return nanoseconds since 1 January 1970 UTC.
    fn nano_time(&self) -> i64;
}

/// Provider of monotonic nanosecond ticks from an arbitrary origin.
///
/// Values are suitable for measuring elapsed time and are not epoch
/// timestamps. Arithmetic should use wrapping operations.
pub trait NanoClock {
    /// Return monotonic nanosecond ticks from an arbitrary origin.
    fn nano_time(&self) -> i64;
}
