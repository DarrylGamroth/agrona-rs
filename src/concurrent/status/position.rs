// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ReadablePosition;

/// Writable single-writer progress position.
pub trait Position: ReadablePosition {
    /// Return whether this local handle is closed.
    fn is_closed(&self) -> bool;

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

    /// Store with relaxed ordering, adapting Java plain semantics.
    fn set(&self, value: i64);

    /// Single-writer plain propose-max operation.
    fn propose_max(&self, proposed_value: i64) -> bool;

    /// Alias for [`Self::propose_max_release`].
    fn propose_max_ordered(&self, proposed_value: i64) -> bool {
        self.propose_max_release(proposed_value)
    }

    /// Single-writer propose-max operation with a release store.
    fn propose_max_release(&self, proposed_value: i64) -> bool;

    /// Single-writer propose-max operation with relaxed ordering.
    fn propose_max_opaque(&self, proposed_value: i64) -> bool;
}
