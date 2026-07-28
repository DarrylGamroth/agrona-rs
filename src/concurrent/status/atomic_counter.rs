// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicBool, Ordering};

use super::{CountersReader, CountersReaderError};
use crate::concurrent::aligned_region::{AlignedRegion, MutableAlignedRegion};

/// Atomic counter backed by an Agrona-compatible values region.
///
/// Multi-writer operations use sequential consistency. Methods named
/// `release` are single-writer load-then-store operations and can lose updates
/// when called concurrently, matching Agrona. Java plain and opaque accesses
/// are represented by relaxed Rust atomics.
#[derive(Debug)]
pub struct AtomicCounter<'a> {
    values: AlignedRegion<'a>,
    offset: usize,
    id: i32,
    closed: AtomicBool,
}

impl<'a> AtomicCounter<'a> {
    /// Construct a checked counter handle over a borrowed values region.
    pub fn new(values: &'a mut [u8], counter_id: i32) -> Result<Self, CountersReaderError> {
        let max_counter_id = CountersReader::validate_values_region(values)?;
        CountersReader::validate_id(counter_id, max_counter_id)?;
        let values = MutableAlignedRegion::new("values", values)?;
        Ok(Self::from_region(values.atomic_view(), counter_id))
    }

    pub(super) fn from_region(values: AlignedRegion<'a>, counter_id: i32) -> Self {
        Self {
            values,
            offset: CountersReader::counter_offset(counter_id)
                .expect("manager supplied a validated counter ID"),
            id: counter_id,
            closed: AtomicBool::new(false),
        }
    }

    /// Return this counter's registry ID.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Mark this local handle closed.
    ///
    /// Registry reclamation is deliberately separate: call
    /// [`CountersManager::free`](super::CountersManager::free) on the
    /// single-owner manager.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Return whether this local handle has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Atomically increment and return the previous value.
    pub fn increment(&self) -> i64 {
        self.get_and_add(1)
    }

    /// Alias for [`Self::increment_release`].
    pub fn increment_ordered(&self) -> i64 {
        self.increment_release()
    }

    /// Single-writer increment with a release store.
    pub fn increment_release(&self) -> i64 {
        self.get_and_add_release(1)
    }

    /// Single-writer increment with relaxed ordering.
    pub fn increment_opaque(&self) -> i64 {
        self.get_and_add_opaque(1)
    }

    /// Single-writer increment with relaxed ordering.
    pub fn increment_plain(&self) -> i64 {
        self.get_and_add_plain(1)
    }

    /// Atomically decrement and return the previous value.
    pub fn decrement(&self) -> i64 {
        self.get_and_add(-1)
    }

    /// Alias for [`Self::decrement_release`].
    pub fn decrement_ordered(&self) -> i64 {
        self.decrement_release()
    }

    /// Single-writer decrement with a release store.
    pub fn decrement_release(&self) -> i64 {
        self.get_and_add_release(-1)
    }

    /// Single-writer decrement with relaxed ordering.
    pub fn decrement_opaque(&self) -> i64 {
        self.get_and_add_opaque(-1)
    }

    /// Single-writer decrement with relaxed ordering.
    pub fn decrement_plain(&self) -> i64 {
        self.get_and_add_plain(-1)
    }

    /// Store with sequentially consistent ordering.
    pub fn set(&self, value: i64) {
        self.store(value, Ordering::SeqCst);
    }

    /// Alias for [`Self::set_release`].
    pub fn set_ordered(&self, value: i64) {
        self.set_release(value);
    }

    /// Store with release ordering.
    pub fn set_release(&self, value: i64) {
        self.store(value, Ordering::Release);
    }

    /// Store with relaxed ordering, adapting Java opaque semantics.
    pub fn set_opaque(&self, value: i64) {
        self.store(value, Ordering::Relaxed);
    }

    /// Alias for [`Self::set_plain`].
    pub fn set_weak(&self, value: i64) {
        self.set_plain(value);
    }

    /// Store with relaxed ordering, adapting Java plain semantics.
    pub fn set_plain(&self, value: i64) {
        self.store(value, Ordering::Relaxed);
    }

    /// Atomically add and return the previous value.
    pub fn get_and_add(&self, increment: i64) -> i64 {
        self.values
            .fetch_add_i64(self.offset, increment, Ordering::SeqCst)
    }

    /// Alias for [`Self::get_and_add_release`].
    pub fn get_and_add_ordered(&self, increment: i64) -> i64 {
        self.get_and_add_release(increment)
    }

    /// Single-writer add with a release store.
    pub fn get_and_add_release(&self, increment: i64) -> i64 {
        self.add_single_writer(increment, Ordering::Release)
    }

    /// Single-writer add with relaxed ordering.
    pub fn get_and_add_opaque(&self, increment: i64) -> i64 {
        self.add_single_writer(increment, Ordering::Relaxed)
    }

    /// Single-writer add with relaxed ordering.
    pub fn get_and_add_plain(&self, increment: i64) -> i64 {
        self.add_single_writer(increment, Ordering::Relaxed)
    }

    /// Atomically replace the value and return its previous value.
    pub fn get_and_set(&self, value: i64) -> i64 {
        self.values.swap_i64(self.offset, value, Ordering::SeqCst)
    }

    /// Atomically update when the current value equals `expected`.
    pub fn compare_and_set(&self, expected: i64, update: i64) -> bool {
        self.values
            .compare_exchange_i64(
                self.offset,
                expected,
                update,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    /// Load with sequentially consistent ordering.
    pub fn get(&self) -> i64 {
        self.load(Ordering::SeqCst)
    }

    /// Load with acquire ordering.
    pub fn get_acquire(&self) -> i64 {
        self.load(Ordering::Acquire)
    }

    /// Load with relaxed ordering, adapting Java opaque semantics.
    pub fn get_opaque(&self) -> i64 {
        self.load(Ordering::Relaxed)
    }

    /// Alias for [`Self::get_plain`].
    pub fn get_weak(&self) -> i64 {
        self.get_plain()
    }

    /// Load with relaxed ordering, adapting Java plain semantics.
    pub fn get_plain(&self) -> i64 {
        self.load(Ordering::Relaxed)
    }

    /// Single-writer plain propose-max operation.
    pub fn propose_max(&self, proposed: i64) -> bool {
        self.propose_max_with(proposed, Ordering::Relaxed)
    }

    /// Alias for [`Self::propose_max_release`].
    pub fn propose_max_ordered(&self, proposed: i64) -> bool {
        self.propose_max_release(proposed)
    }

    /// Single-writer propose-max operation with a release store.
    pub fn propose_max_release(&self, proposed: i64) -> bool {
        self.propose_max_with(proposed, Ordering::Release)
    }

    /// Single-writer propose-max operation with relaxed ordering.
    pub fn propose_max_opaque(&self, proposed: i64) -> bool {
        self.propose_max_with(proposed, Ordering::Relaxed)
    }

    #[inline]
    fn load(&self, ordering: Ordering) -> i64 {
        self.values.load_i64(self.offset, ordering)
    }

    #[inline]
    fn store(&self, value: i64, ordering: Ordering) {
        self.values.store_i64(self.offset, value, ordering);
    }

    #[inline]
    fn add_single_writer(&self, increment: i64, ordering: Ordering) -> i64 {
        let current = self.load(Ordering::Relaxed);
        self.store(current.wrapping_add(increment), ordering);
        current
    }

    #[inline]
    fn propose_max_with(&self, proposed: i64, ordering: Ordering) -> bool {
        if self.load(Ordering::Relaxed) < proposed {
            self.store(proposed, ordering);
            true
        } else {
            false
        }
    }
}
