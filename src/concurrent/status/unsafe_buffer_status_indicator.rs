// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{AtomicCounter, CountersReaderError, StatusIndicator, StatusIndicatorReader};

/// Status indicator backed by an Agrona-compatible values region.
#[derive(Debug)]
pub struct UnsafeBufferStatusIndicator<'a> {
    counter: AtomicCounter<'a>,
}

impl<'a> UnsafeBufferStatusIndicator<'a> {
    /// Construct a checked indicator over a borrowed values region.
    pub fn new(values: &'a mut [u8], counter_id: i32) -> Result<Self, CountersReaderError> {
        Ok(Self {
            counter: AtomicCounter::new(values, counter_id)?,
        })
    }

    /// Wrap an existing checked atomic counter handle as a status indicator.
    pub fn from_counter(counter: AtomicCounter<'a>) -> Self {
        Self { counter }
    }
}

impl StatusIndicatorReader for UnsafeBufferStatusIndicator<'_> {
    fn id(&self) -> i32 {
        self.counter.id()
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
}

impl StatusIndicator for UnsafeBufferStatusIndicator<'_> {
    fn set_volatile(&self, value: i64) {
        self.counter.set(value);
    }

    fn set_release(&self, value: i64) {
        self.counter.set_release(value);
    }

    fn set_opaque(&self, value: i64) {
        self.counter.set_opaque(value);
    }
}
