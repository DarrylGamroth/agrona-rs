// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

/// Provider of milliseconds since 1 January 1970 UTC.
pub trait EpochClock {
    /// Return milliseconds since 1 January 1970 UTC.
    fn time(&self) -> i64;
}
