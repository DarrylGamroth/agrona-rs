// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

/// Read-only progress position for a component.
pub trait ReadablePosition: Send + Sync {
    /// Return the position's counter ID.
    fn id(&self) -> i32;

    /// Load with relaxed ordering, adapting Java plain semantics.
    fn get(&self) -> i64;

    /// Load with sequentially consistent ordering.
    fn get_volatile(&self) -> i64;

    /// Load with acquire ordering.
    fn get_acquire(&self) -> i64;

    /// Load with relaxed ordering, adapting Java opaque semantics.
    fn get_opaque(&self) -> i64;

    /// Close the local position handle.
    fn close(&self);
}
