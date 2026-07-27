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

mod atomic_clock_value;
mod cached_epoch_clock;
mod cached_nano_clock;
mod epoch_clock;
mod epoch_micro_clock;
mod epoch_nano_clock;
mod nano_clock;
mod offset_epoch_nano_clock;
mod system_epoch_clock;
mod system_epoch_micro_clock;
mod system_epoch_nano_clock;
mod system_nano_clock;
mod system_time;

pub use cached_epoch_clock::{CachedEpochClock, CachedEpochClockReader};
pub use cached_nano_clock::{CachedNanoClock, CachedNanoClockReader};
pub use epoch_clock::EpochClock;
pub use epoch_micro_clock::EpochMicroClock;
pub use epoch_nano_clock::EpochNanoClock;
pub use nano_clock::NanoClock;
pub use offset_epoch_nano_clock::{
    OffsetEpochNanoClock, OffsetEpochNanoClockConfig, OffsetEpochNanoClockError,
};
pub use system_epoch_clock::SystemEpochClock;
pub use system_epoch_micro_clock::SystemEpochMicroClock;
pub use system_epoch_nano_clock::SystemEpochNanoClock;
pub use system_nano_clock::SystemNanoClock;
