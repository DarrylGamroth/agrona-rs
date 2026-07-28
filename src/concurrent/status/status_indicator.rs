// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::StatusIndicatorReader;

/// Writable status value for a component.
pub trait StatusIndicator: StatusIndicatorReader {
    /// Store with sequentially consistent ordering.
    fn set_volatile(&self, value: i64);

    /// Alias for [`Self::set_release`].
    fn set_ordered(&self, value: i64) {
        self.set_release(value);
    }

    /// Store with release ordering.
    fn set_release(&self, value: i64);

    /// Store with relaxed ordering, adapting Java opaque semantics.
    fn set_opaque(&self, value: i64);
}
