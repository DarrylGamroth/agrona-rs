// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

/// Read-only status value for a component.
pub trait StatusIndicatorReader: Send + Sync {
    /// Return the status counter ID.
    fn id(&self) -> i32;

    /// Load with sequentially consistent ordering.
    fn get_volatile(&self) -> i64;

    /// Load with acquire ordering.
    fn get_acquire(&self) -> i64;

    /// Load with relaxed ordering, adapting Java opaque semantics.
    fn get_opaque(&self) -> i64;
}
