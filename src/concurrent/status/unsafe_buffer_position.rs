// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{AtomicCounter, CountersReaderError, Position, ReadablePosition};

/// Position backed by an Agrona-compatible counter values region.
///
/// Closing the handle does not reclaim the registry record. Use the
/// single-owner counter manager explicitly for reclamation.
#[derive(Debug)]
pub struct UnsafeBufferPosition<'a> {
    counter: AtomicCounter<'a>,
}

impl<'a> UnsafeBufferPosition<'a> {
    /// Construct a checked position over a borrowed values region.
    pub fn new(values: &'a mut [u8], counter_id: i32) -> Result<Self, CountersReaderError> {
        Ok(Self {
            counter: AtomicCounter::new(values, counter_id)?,
        })
    }

    /// Wrap an existing checked atomic counter handle as a position.
    pub fn from_counter(counter: AtomicCounter<'a>) -> Self {
        Self { counter }
    }
}

impl ReadablePosition for UnsafeBufferPosition<'_> {
    fn id(&self) -> i32 {
        self.counter.id()
    }

    fn get(&self) -> i64 {
        self.counter.get_plain()
    }

    fn get_volatile(&self) -> i64 {
        self.counter.get()
    }

    fn get_acquire(&self) -> i64 {
        self.counter.get_acquire()
    }

    fn get_opaque(&self) -> i64 {
        self.counter.get_opaque()
    }

    fn close(&self) {
        self.counter.close();
    }
}

impl Position for UnsafeBufferPosition<'_> {
    fn is_closed(&self) -> bool {
        self.counter.is_closed()
    }

    fn set_volatile(&self, value: i64) {
        self.counter.set(value);
    }

    fn set_release(&self, value: i64) {
        self.counter.set_release(value);
    }

    fn set_opaque(&self, value: i64) {
        self.counter.set_opaque(value);
    }

    fn set(&self, value: i64) {
        self.counter.set_plain(value);
    }

    fn propose_max(&self, proposed_value: i64) -> bool {
        self.counter.propose_max(proposed_value)
    }

    fn propose_max_release(&self, proposed_value: i64) -> bool {
        self.counter.propose_max_release(proposed_value)
    }

    fn propose_max_opaque(&self, proposed_value: i64) -> bool {
        self.counter.propose_max_opaque(proposed_value)
    }
}
