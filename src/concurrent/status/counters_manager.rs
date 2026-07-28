// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::convert::Infallible;
use std::sync::atomic::Ordering;

use crate::clock::{CachedEpochClock, EpochClock};
use crate::concurrent::aligned_region::MutableAlignedRegion;

use super::{
    AtomicCounter, CounterAllocationError, CountersManagerError, CountersReader,
    CountersReaderError,
};

/// Single-owner allocator and mutator for Agrona-compatible counter regions.
///
/// Registry operations are deliberately not synchronized. A manager can move
/// between threads but must have only one active owner. Counter value handles
/// created by the manager remain independently shareable.
///
/// ```compile_fail
/// use agrona::concurrent::status::CountersManager;
///
/// fn require_sync<T: Sync>() {}
/// require_sync::<CountersManager<'static>>();
/// ```
#[derive(Debug)]
pub struct CountersManager<'a, C = CachedEpochClock> {
    metadata: MutableAlignedRegion<'a>,
    values: MutableAlignedRegion<'a>,
    max_counter_id: i32,
    high_water_mark_id: i32,
    free_list: Vec<i32>,
    epoch_clock: C,
    free_to_reuse_timeout_ms: i64,
}

impl<'a> CountersManager<'a, CachedEpochClock> {
    /// Construct a manager with a zero-valued cached clock and no reuse delay.
    pub fn new(metadata: &'a mut [u8], values: &'a mut [u8]) -> Result<Self, CountersManagerError> {
        Self::with_clock(metadata, values, CachedEpochClock::new(), 0)
    }
}

impl<'a, C: EpochClock> CountersManager<'a, C> {
    /// Construct a manager with an epoch-millisecond clock and reuse timeout.
    pub fn with_clock(
        metadata: &'a mut [u8],
        values: &'a mut [u8],
        epoch_clock: C,
        free_to_reuse_timeout_ms: i64,
    ) -> Result<Self, CountersManagerError> {
        let max_counter_id = CountersReader::new(metadata, values)?.max_counter_id();
        Ok(Self {
            metadata: MutableAlignedRegion::new("metadata", metadata)?,
            values: MutableAlignedRegion::new("values", values)?,
            max_counter_id,
            high_water_mark_id: CountersReader::NULL_COUNTER_ID,
            free_list: Vec::new(),
            epoch_clock,
            free_to_reuse_timeout_ms,
        })
    }

    /// Return the number of counter records supported by the values region.
    pub fn capacity(&self) -> usize {
        (self.max_counter_id as i64 + 1) as usize
    }

    /// Return the number of immediately allocatable records.
    pub fn available(&self) -> usize {
        let unused = (i64::from(self.max_counter_id) - i64::from(self.high_water_mark_id)) as usize;
        if self.free_list.is_empty() {
            return unused;
        }

        let now_ms = self.epoch_clock.time();
        let metadata = self.metadata.view();
        unused
            + self
                .free_list
                .iter()
                .filter(|counter_id| {
                    let offset = CountersReader::metadata_offset(**counter_id)
                        .expect("free-list counter ID was validated");
                    now_ms
                        >= metadata.load_i64(
                            offset + CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
                            Ordering::Relaxed,
                        )
                })
                .count()
    }

    /// Return a read-only view whose byte borrows cannot outlive this borrow.
    pub fn reader(&self) -> CountersReader<'_> {
        CountersReader::from_aligned(
            self.metadata.view(),
            self.values.view(),
            self.max_counter_id,
        )
    }

    /// Return a checked atomic handle for any in-range counter record.
    ///
    /// As in Agrona's direct `AtomicCounter` constructor, this validates the
    /// values-region location but does not require the metadata record to be
    /// allocated.
    pub fn counter_handle(
        &self,
        counter_id: i32,
    ) -> Result<AtomicCounter<'a>, CountersManagerError> {
        CountersReader::validate_id(counter_id, self.max_counter_id)?;
        Ok(AtomicCounter::from_region(
            self.values.atomic_view(),
            counter_id,
        ))
    }

    /// Allocate a default-type counter with label bytes.
    pub fn allocate(&mut self, label: &[u8]) -> Result<i32, CountersManagerError> {
        self.allocate_with_type(label, CountersReader::DEFAULT_TYPE_ID)
    }

    /// Allocate a counter with label bytes and an explicit type ID.
    pub fn allocate_with_type(
        &mut self,
        label: &[u8],
        type_id: i32,
    ) -> Result<i32, CountersManagerError> {
        match self.allocate_with_key(label, type_id, |_| Ok::<_, Infallible>(())) {
            Ok(counter_id) => Ok(counter_id),
            Err(CounterAllocationError::Manager(error)) => Err(error),
            Err(CounterAllocationError::KeyInitializer(error)) => match error {},
        }
    }

    /// Allocate a counter and initialize its complete 112-byte key field.
    ///
    /// If the initializer returns an error, the selected ID is returned to the
    /// manager free list and can be allocated again.
    pub fn allocate_with_key<E>(
        &mut self,
        label: &[u8],
        type_id: i32,
        key_initializer: impl FnOnce(&mut [u8]) -> Result<(), E>,
    ) -> Result<i32, CounterAllocationError<E>> {
        self.allocate_with_key_and_label_order(label, type_id, key_initializer, Ordering::Release)
    }

    fn allocate_with_key_and_label_order<E>(
        &mut self,
        label: &[u8],
        type_id: i32,
        key_initializer: impl FnOnce(&mut [u8]) -> Result<(), E>,
        label_ordering: Ordering,
    ) -> Result<i32, CounterAllocationError<E>> {
        let counter_id = self.next_counter_id()?;
        let record_offset =
            CountersReader::metadata_offset(counter_id).expect("selected counter ID is valid");

        self.metadata.view().store_i32(
            record_offset + CountersReader::TYPE_ID_OFFSET,
            type_id,
            Ordering::Relaxed,
        );

        if let Err(error) = key_initializer(self.metadata.bytes_mut(
            record_offset + CountersReader::KEY_OFFSET,
            CountersReader::MAX_KEY_LENGTH,
        )) {
            self.free_list.push(counter_id);
            return Err(CounterAllocationError::KeyInitializer(error));
        }

        self.metadata.view().store_i64(
            record_offset + CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
            CountersReader::NOT_FREE_TO_REUSE,
            Ordering::Relaxed,
        );
        self.put_label(record_offset, label, label_ordering);
        self.metadata.view().store_i32(
            record_offset + CountersReader::STATE_OFFSET,
            CountersReader::RECORD_ALLOCATED,
            Ordering::Release,
        );
        Ok(counter_id)
    }

    /// Allocate by copying optional key bytes and mandatory label bytes.
    ///
    /// Both fields are truncated to their fixed Agrona ABI capacities.
    pub fn allocate_raw(
        &mut self,
        type_id: i32,
        key: Option<&[u8]>,
        label: &[u8],
    ) -> Result<i32, CountersManagerError> {
        let result = self.allocate_with_key_and_label_order(
            label,
            type_id,
            |target| {
                if let Some(key) = key {
                    let length = key.len().min(target.len());
                    target[..length].copy_from_slice(&key[..length]);
                }
                Ok::<_, Infallible>(())
            },
            Ordering::Relaxed,
        );
        match result {
            Ok(counter_id) => Ok(counter_id),
            Err(CounterAllocationError::Manager(error)) => Err(error),
            Err(CounterAllocationError::KeyInitializer(error)) => match error {},
        }
    }

    /// Allocate and return an atomic value handle.
    pub fn new_counter(&mut self, label: &[u8]) -> Result<AtomicCounter<'a>, CountersManagerError> {
        self.new_counter_with_type(label, CountersReader::DEFAULT_TYPE_ID)
    }

    /// Allocate an explicit-type counter and return its atomic value handle.
    pub fn new_counter_with_type(
        &mut self,
        label: &[u8],
        type_id: i32,
    ) -> Result<AtomicCounter<'a>, CountersManagerError> {
        let counter_id = self.allocate_with_type(label, type_id)?;
        Ok(AtomicCounter::from_region(
            self.values.atomic_view(),
            counter_id,
        ))
    }

    /// Allocate with a caller key initializer and return an atomic handle.
    pub fn new_counter_with_key<E>(
        &mut self,
        label: &[u8],
        type_id: i32,
        key_initializer: impl FnOnce(&mut [u8]) -> Result<(), E>,
    ) -> Result<AtomicCounter<'a>, CounterAllocationError<E>> {
        let counter_id = self.allocate_with_key(label, type_id, key_initializer)?;
        Ok(AtomicCounter::from_region(
            self.values.atomic_view(),
            counter_id,
        ))
    }

    /// Allocate by copying optional key and label bytes and return a handle.
    pub fn new_counter_raw(
        &mut self,
        type_id: i32,
        key: Option<&[u8]>,
        label: &[u8],
    ) -> Result<AtomicCounter<'a>, CountersManagerError> {
        let counter_id = self.allocate_raw(type_id, key, label)?;
        Ok(AtomicCounter::from_region(
            self.values.atomic_view(),
            counter_id,
        ))
    }

    /// Free an allocated counter for delayed reuse.
    pub fn free(&mut self, counter_id: i32) -> Result<(), CountersManagerError> {
        CountersReader::validate_id(counter_id, self.max_counter_id)?;
        let offset = CountersReader::metadata_offset(counter_id)
            .expect("validated counter ID has a metadata offset");
        let state = self
            .metadata
            .view()
            .load_i32(offset + CountersReader::STATE_OFFSET, Ordering::Acquire);
        if state != CountersReader::RECORD_ALLOCATED {
            return Err(CountersManagerError::CounterNotAllocated { counter_id, state });
        }

        self.metadata.view().store_i32(
            offset + CountersReader::STATE_OFFSET,
            CountersReader::RECORD_RECLAIMED,
            Ordering::Release,
        );
        self.metadata
            .bytes_mut(
                offset + CountersReader::KEY_OFFSET,
                CountersReader::MAX_KEY_LENGTH,
            )
            .fill(0);
        let deadline = self
            .epoch_clock
            .time()
            .wrapping_add(self.free_to_reuse_timeout_ms);
        self.metadata.view().store_i64(
            offset + CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
            deadline,
            Ordering::Relaxed,
        );
        self.free_list.push(counter_id);
        Ok(())
    }

    /// Release-store a counter value.
    pub fn set_counter_value(
        &self,
        counter_id: i32,
        value: i64,
    ) -> Result<(), CountersManagerError> {
        let offset = self.checked_counter_offset(counter_id)?;
        self.values
            .view()
            .store_i64(offset, value, Ordering::Release);
        Ok(())
    }

    /// Release-store a counter registration ID.
    pub fn set_counter_registration_id(
        &self,
        counter_id: i32,
        registration_id: i64,
    ) -> Result<(), CountersManagerError> {
        let offset = self.checked_counter_offset(counter_id)?;
        self.values.view().store_i64(
            offset + CountersReader::REGISTRATION_ID_OFFSET,
            registration_id,
            Ordering::Release,
        );
        Ok(())
    }

    /// Relaxed-store a counter owner ID, adapting upstream plain semantics.
    pub fn set_counter_owner_id(
        &self,
        counter_id: i32,
        owner_id: i64,
    ) -> Result<(), CountersManagerError> {
        let offset = self.checked_counter_offset(counter_id)?;
        self.values.view().store_i64(
            offset + CountersReader::OWNER_ID_OFFSET,
            owner_id,
            Ordering::Relaxed,
        );
        Ok(())
    }

    /// Relaxed-store a counter reference ID, adapting upstream plain semantics.
    pub fn set_counter_reference_id(
        &self,
        counter_id: i32,
        reference_id: i64,
    ) -> Result<(), CountersManagerError> {
        let offset = self.checked_counter_offset(counter_id)?;
        self.values.view().store_i64(
            offset + CountersReader::REFERENCE_ID_OFFSET,
            reference_id,
            Ordering::Relaxed,
        );
        Ok(())
    }

    /// Replace and release-publish a counter label, truncating to 380 bytes.
    pub fn set_counter_label(
        &mut self,
        counter_id: i32,
        label: &[u8],
    ) -> Result<(), CountersManagerError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        self.put_label(offset, label, Ordering::Release);
        Ok(())
    }

    /// Replace a key prefix without modifying the remainder of the key field.
    pub fn set_counter_key(
        &mut self,
        counter_id: i32,
        key: &[u8],
    ) -> Result<(), CountersManagerError> {
        if key.len() > CountersReader::MAX_KEY_LENGTH {
            return Err(CountersManagerError::KeyTooLong {
                length: key.len(),
                maximum_length: CountersReader::MAX_KEY_LENGTH,
            });
        }
        let offset = self.checked_metadata_offset(counter_id)?;
        self.metadata
            .bytes_mut(offset + CountersReader::KEY_OFFSET, key.len())
            .copy_from_slice(key);
        Ok(())
    }

    /// Mutate the complete fixed key field in place.
    pub fn update_counter_key(
        &mut self,
        counter_id: i32,
        update: impl FnOnce(&mut [u8]),
    ) -> Result<(), CountersManagerError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        update(self.metadata.bytes_mut(
            offset + CountersReader::KEY_OFFSET,
            CountersReader::MAX_KEY_LENGTH,
        ));
        Ok(())
    }

    /// Append bytes to a counter label and return the appended byte count.
    pub fn append_to_label(
        &mut self,
        counter_id: i32,
        suffix: &[u8],
    ) -> Result<usize, CountersManagerError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        let existing = self.metadata.view().load_i32(
            offset + CountersReader::LABEL_LENGTH_OFFSET,
            Ordering::Acquire,
        );
        let existing =
            usize::try_from(existing).map_err(|_| CountersReaderError::MalformedLabelLength {
                counter_id,
                label_length: existing,
                maximum_length: CountersReader::MAX_LABEL_LENGTH,
            })?;
        if existing > CountersReader::MAX_LABEL_LENGTH {
            return Err(CountersReaderError::MalformedLabelLength {
                counter_id,
                label_length: i32::try_from(existing).unwrap_or(i32::MAX),
                maximum_length: CountersReader::MAX_LABEL_LENGTH,
            }
            .into());
        }

        let length = suffix
            .len()
            .min(CountersReader::MAX_LABEL_LENGTH - existing);
        self.metadata
            .bytes_mut(
                offset + CountersReader::LABEL_VALUE_OFFSET + existing,
                length,
            )
            .copy_from_slice(&suffix[..length]);
        self.metadata.view().store_i32(
            offset + CountersReader::LABEL_LENGTH_OFFSET,
            i32::try_from(existing + length).expect("label capacity fits i32"),
            Ordering::Release,
        );
        Ok(length)
    }

    fn next_counter_id(&mut self) -> Result<i32, CountersManagerError> {
        if !self.free_list.is_empty() {
            let now_ms = self.epoch_clock.time();
            let metadata = self.metadata.view();
            if let Some(index) = self.free_list.iter().position(|counter_id| {
                let offset = CountersReader::metadata_offset(*counter_id)
                    .expect("free-list counter ID was validated");
                now_ms
                    >= metadata.load_i64(
                        offset + CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
                        Ordering::Relaxed,
                    )
            }) {
                let counter_id = self.free_list.remove(index);
                let offset = CountersReader::counter_offset(counter_id)
                    .expect("free-list counter ID was validated");
                let values = self.values.view();
                values.store_i64(
                    offset + CountersReader::REGISTRATION_ID_OFFSET,
                    CountersReader::DEFAULT_REGISTRATION_ID,
                    Ordering::Release,
                );
                values.store_i64(
                    offset + CountersReader::OWNER_ID_OFFSET,
                    CountersReader::DEFAULT_OWNER_ID,
                    Ordering::Relaxed,
                );
                values.store_i64(
                    offset + CountersReader::REFERENCE_ID_OFFSET,
                    CountersReader::DEFAULT_REFERENCE_ID,
                    Ordering::Relaxed,
                );
                values.store_i64(offset, 0, Ordering::Release);
                return Ok(counter_id);
            }
        }

        if self.high_water_mark_id >= self.max_counter_id {
            return Err(CountersManagerError::Full {
                max_counter_id: self.max_counter_id,
            });
        }
        let next = self.high_water_mark_id + 1;
        self.high_water_mark_id = next;
        Ok(next)
    }

    fn put_label(&mut self, record_offset: usize, label: &[u8], ordering: Ordering) {
        let length = label.len().min(CountersReader::MAX_LABEL_LENGTH);
        self.metadata
            .bytes_mut(record_offset + CountersReader::LABEL_VALUE_OFFSET, length)
            .copy_from_slice(&label[..length]);
        self.metadata.view().store_i32(
            record_offset + CountersReader::LABEL_LENGTH_OFFSET,
            i32::try_from(length).expect("label capacity fits i32"),
            ordering,
        );
    }

    fn checked_counter_offset(&self, counter_id: i32) -> Result<usize, CountersManagerError> {
        CountersReader::validate_id(counter_id, self.max_counter_id)?;
        Ok(CountersReader::counter_offset(counter_id)?)
    }

    fn checked_metadata_offset(&self, counter_id: i32) -> Result<usize, CountersManagerError> {
        CountersReader::validate_id(counter_id, self.max_counter_id)?;
        Ok(CountersReader::metadata_offset(counter_id)?)
    }
}
