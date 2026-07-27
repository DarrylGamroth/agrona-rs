// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

/// Provider of nanoseconds since 1 January 1970 UTC.
pub trait EpochNanoClock {
    /// Return nanoseconds since 1 January 1970 UTC.
    fn nano_time(&self) -> i64;
}
