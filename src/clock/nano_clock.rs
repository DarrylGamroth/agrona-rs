// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

/// Provider of monotonic nanosecond ticks from an arbitrary origin.
///
/// Values are suitable for measuring elapsed time and are not epoch
/// timestamps. Arithmetic should use wrapping operations.
pub trait NanoClock {
    /// Return monotonic nanosecond ticks from an arbitrary origin.
    fn nano_time(&self) -> i64;
}
