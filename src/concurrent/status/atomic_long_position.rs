// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use super::{Position, ReadablePosition};

/// Process-local atomic position, useful for tests and intra-process progress.
#[derive(Debug)]
pub struct AtomicLongPosition {
    closed: AtomicBool,
    id: i32,
    value: AtomicI64,
}

impl AtomicLongPosition {
    /// Construct ID zero with value zero.
    pub fn new() -> Self {
        Self::with_id_and_value(0, 0)
    }

    /// Construct an ID with value zero.
    pub fn with_id(id: i32) -> Self {
        Self::with_id_and_value(id, 0)
    }

    /// Construct an ID and initial value.
    pub fn with_id_and_value(id: i32, initial_value: i64) -> Self {
        Self {
            closed: AtomicBool::new(false),
            id,
            value: AtomicI64::new(initial_value),
        }
    }

    fn propose(&self, proposed: i64, store_ordering: Ordering) -> bool {
        // Agrona's AtomicLong implementation performs a volatile load even
        // for its release and opaque propose-max variants.
        if self.value.load(Ordering::SeqCst) < proposed {
            self.value.store(proposed, store_ordering);
            true
        } else {
            false
        }
    }
}

impl Default for AtomicLongPosition {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadablePosition for AtomicLongPosition {
    fn id(&self) -> i32 {
        self.id
    }

    fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    fn get_volatile(&self) -> i64 {
        self.value.load(Ordering::SeqCst)
    }

    fn get_acquire(&self) -> i64 {
        self.value.load(Ordering::Acquire)
    }

    fn get_opaque(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

impl Position for AtomicLongPosition {
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    fn set_volatile(&self, value: i64) {
        self.value.store(value, Ordering::SeqCst);
    }

    fn set_release(&self, value: i64) {
        self.value.store(value, Ordering::Release);
    }

    fn set_opaque(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }

    fn set(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }

    fn propose_max(&self, proposed_value: i64) -> bool {
        self.propose_max_release(proposed_value)
    }

    fn propose_max_release(&self, proposed_value: i64) -> bool {
        self.propose(proposed_value, Ordering::Release)
    }

    fn propose_max_opaque(&self, proposed_value: i64) -> bool {
        self.propose(proposed_value, Ordering::Relaxed)
    }
}
