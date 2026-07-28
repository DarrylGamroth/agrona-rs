// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::Ordering;

use super::CountersReaderError;
use crate::concurrent::aligned_region::AlignedRegion;

/// Read-only view over Agrona-compatible counter metadata and values regions.
///
/// The reader borrows caller-owned storage and performs no allocation.
/// Construction validates record boundaries, the metadata-to-values size
/// ratio, capacity, and natural base alignment before any atomic access.
///
/// Integral fields that Agrona publishes with release semantics are loaded
/// with [`Ordering::Acquire`]. Upstream plain integral reads use
/// [`Ordering::Relaxed`] in Rust to avoid undefined racy non-atomic access
/// without adding a synchronizes-with edge.
#[derive(Clone, Copy, Debug)]
pub struct CountersReader<'a> {
    metadata: AlignedRegion<'a>,
    values: AlignedRegion<'a>,
    max_counter_id: i32,
}

impl<'a> CountersReader<'a> {
    /// Default type ID when none is supplied.
    pub const DEFAULT_TYPE_ID: i32 = 0;
    /// Default registration ID when none is supplied.
    pub const DEFAULT_REGISTRATION_ID: i64 = 0;
    /// Default owner ID when none is supplied.
    pub const DEFAULT_OWNER_ID: i64 = 0;
    /// Default reference ID when none is supplied.
    pub const DEFAULT_REFERENCE_ID: i64 = 0;
    /// Null counter ID used by the upstream APIs.
    pub const NULL_COUNTER_ID: i32 = -1;

    /// Record has not been used.
    pub const RECORD_UNUSED: i32 = 0;
    /// Record is currently allocated.
    pub const RECORD_ALLOCATED: i32 = 1;
    /// Record was active and has been reclaimed.
    pub const RECORD_RECLAIMED: i32 = -1;
    /// Deadline value indicating that an allocated counter cannot be reused.
    pub const NOT_FREE_TO_REUSE: i64 = i64::MAX;

    /// Cache-line length used by the counter ABI.
    pub const CACHE_LINE_LENGTH: usize = 64;

    /// Offset of the counter value within a values record.
    pub const COUNTER_VALUE_OFFSET: usize = 0;
    /// Offset of the registration ID within a values record.
    pub const REGISTRATION_ID_OFFSET: usize = 8;
    /// Offset of the owner ID within a values record.
    pub const OWNER_ID_OFFSET: usize = 16;
    /// Offset of the reference ID within a values record.
    pub const REFERENCE_ID_OFFSET: usize = 24;
    /// Length of a padded values record.
    pub const COUNTER_LENGTH: usize = Self::CACHE_LINE_LENGTH * 2;

    /// Offset of record state within a metadata record.
    pub const STATE_OFFSET: usize = 0;
    /// Offset of type ID within a metadata record.
    pub const TYPE_ID_OFFSET: usize = 4;
    /// Offset of the reuse deadline within a metadata record.
    pub const FREE_FOR_REUSE_DEADLINE_OFFSET: usize = 8;
    /// Offset of key bytes within a metadata record.
    pub const KEY_OFFSET: usize = 16;
    /// Maximum key length in bytes.
    pub const MAX_KEY_LENGTH: usize = 112;
    /// Agrona label-field offset, at which the length prefix begins.
    pub const LABEL_OFFSET: usize = Self::CACHE_LINE_LENGTH * 2;
    /// Offset of the label length prefix within a metadata record.
    pub const LABEL_LENGTH_OFFSET: usize = Self::LABEL_OFFSET;
    /// Offset of the first label byte within a metadata record.
    pub const LABEL_VALUE_OFFSET: usize = Self::LABEL_LENGTH_OFFSET + size_of::<i32>();
    /// Full label field length including its prefix.
    pub const FULL_LABEL_LENGTH: usize = Self::CACHE_LINE_LENGTH * 6;
    /// Maximum label length in bytes, excluding its prefix.
    pub const MAX_LABEL_LENGTH: usize = Self::FULL_LABEL_LENGTH - size_of::<i32>();
    /// Length of a metadata record.
    pub const METADATA_LENGTH: usize = Self::LABEL_OFFSET + Self::FULL_LABEL_LENGTH;
    /// Metadata bytes required per values byte.
    pub const METADATA_TO_VALUES_RATIO: usize = Self::METADATA_LENGTH / Self::COUNTER_LENGTH;

    /// Construct a checked reader over caller-owned metadata and values.
    ///
    /// The regions use native-endian Agrona/Aeron counter records. Empty
    /// regions are valid and produce a maximum counter ID of `-1`.
    pub fn new(metadata: &'a [u8], values: &'a [u8]) -> Result<Self, CountersReaderError> {
        Self::validate_record_boundary("metadata", metadata.len(), Self::METADATA_LENGTH)?;
        Self::validate_record_boundary("values", values.len(), Self::COUNTER_LENGTH)?;

        let required_metadata = values
            .len()
            .checked_mul(Self::METADATA_TO_VALUES_RATIO)
            .ok_or(CountersReaderError::RegionSizeOverflow {
                values_length: values.len(),
            })?;
        if metadata.len() < required_metadata {
            return Err(CountersReaderError::MetadataTooSmall {
                actual: metadata.len(),
                required: required_metadata,
            });
        }

        let capacity = values.len() / Self::COUNTER_LENGTH;
        let maximum_capacity = i32::MAX as usize + 1;
        if capacity > maximum_capacity {
            return Err(CountersReaderError::CapacityTooLarge {
                capacity,
                maximum_capacity,
            });
        }

        let metadata = AlignedRegion::new("metadata", metadata)?;
        let values = AlignedRegion::new("values", values)?;
        let max_counter_id = if capacity == 0 {
            Self::NULL_COUNTER_ID
        } else {
            i32::try_from(capacity - 1).expect("validated counter capacity")
        };

        Ok(Self {
            metadata,
            values,
            max_counter_id,
        })
    }

    pub(super) fn from_aligned(
        metadata: AlignedRegion<'a>,
        values: AlignedRegion<'a>,
        max_counter_id: i32,
    ) -> Self {
        Self {
            metadata,
            values,
            max_counter_id,
        }
    }

    pub(super) fn validate_values_region(values: &[u8]) -> Result<i32, CountersReaderError> {
        Self::validate_record_boundary("values", values.len(), Self::COUNTER_LENGTH)?;
        let capacity = values.len() / Self::COUNTER_LENGTH;
        let maximum_capacity = i32::MAX as usize + 1;
        if capacity > maximum_capacity {
            return Err(CountersReaderError::CapacityTooLarge {
                capacity,
                maximum_capacity,
            });
        }
        let max_counter_id = if capacity == 0 {
            Self::NULL_COUNTER_ID
        } else {
            i32::try_from(capacity - 1).expect("validated counter capacity")
        };
        AlignedRegion::new("values", values)?;
        Ok(max_counter_id)
    }

    pub(super) fn validate_id(
        counter_id: i32,
        max_counter_id: i32,
    ) -> Result<(), CountersReaderError> {
        if counter_id < 0 || counter_id > max_counter_id {
            return Err(CountersReaderError::CounterIdOutOfRange {
                counter_id,
                max_counter_id,
            });
        }
        Ok(())
    }

    /// Return the greatest counter ID supported by the values region.
    ///
    /// An empty values region returns `-1`, matching Agrona.
    pub fn max_counter_id(&self) -> i32 {
        self.max_counter_id
    }

    /// Return the number of complete values records.
    pub fn capacity(&self) -> usize {
        (self.max_counter_id as i64 + 1) as usize
    }

    /// Calculate the values-record offset for a non-negative counter ID.
    pub fn counter_offset(counter_id: i32) -> Result<usize, CountersReaderError> {
        Self::offset(counter_id, Self::COUNTER_LENGTH)
    }

    /// Calculate the metadata-record offset for a non-negative counter ID.
    pub fn metadata_offset(counter_id: i32) -> Result<usize, CountersReaderError> {
        Self::offset(counter_id, Self::METADATA_LENGTH)
    }

    /// Acquire-load the value for a counter.
    #[inline]
    pub fn counter_value(&self, counter_id: i32) -> Result<i64, CountersReaderError> {
        let offset = self.checked_counter_offset(counter_id)?;
        Ok(self
            .values
            .load_i64(offset + Self::COUNTER_VALUE_OFFSET, Ordering::Acquire))
    }

    /// Acquire-load the registration ID for a counter.
    #[inline]
    pub fn counter_registration_id(&self, counter_id: i32) -> Result<i64, CountersReaderError> {
        let offset = self.checked_counter_offset(counter_id)?;
        Ok(self
            .values
            .load_i64(offset + Self::REGISTRATION_ID_OFFSET, Ordering::Acquire))
    }

    /// Load the owner ID with the Rust adaptation of upstream plain ordering.
    #[inline]
    pub fn counter_owner_id(&self, counter_id: i32) -> Result<i64, CountersReaderError> {
        let offset = self.checked_counter_offset(counter_id)?;
        Ok(self
            .values
            .load_i64(offset + Self::OWNER_ID_OFFSET, Ordering::Relaxed))
    }

    /// Load the reference ID with the Rust adaptation of upstream plain ordering.
    #[inline]
    pub fn counter_reference_id(&self, counter_id: i32) -> Result<i64, CountersReaderError> {
        let offset = self.checked_counter_offset(counter_id)?;
        Ok(self
            .values
            .load_i64(offset + Self::REFERENCE_ID_OFFSET, Ordering::Relaxed))
    }

    /// Acquire-load the metadata state for a counter.
    #[inline]
    pub fn counter_state(&self, counter_id: i32) -> Result<i32, CountersReaderError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        Ok(self
            .metadata
            .load_i32(offset + Self::STATE_OFFSET, Ordering::Acquire))
    }

    /// Read the type ID after acquiring the record state.
    pub fn counter_type_id(&self, counter_id: i32) -> Result<i32, CountersReaderError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        self.acquire_record(offset);
        Ok(self
            .metadata
            .load_i32(offset + Self::TYPE_ID_OFFSET, Ordering::Relaxed))
    }

    /// Read the free-for-reuse deadline after acquiring the record state.
    pub fn free_for_reuse_deadline(&self, counter_id: i32) -> Result<i64, CountersReaderError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        self.acquire_record(offset);
        Ok(self.metadata.load_i64(
            offset + Self::FREE_FOR_REUSE_DEADLINE_OFFSET,
            Ordering::Relaxed,
        ))
    }

    /// Borrow the complete 112-byte key after acquiring the record state.
    pub fn counter_key(&self, counter_id: i32) -> Result<&[u8], CountersReaderError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        self.acquire_record(offset);
        Ok(self
            .metadata
            .bytes(offset + Self::KEY_OFFSET, Self::MAX_KEY_LENGTH))
    }

    /// Borrow the published label bytes after acquiring state and label length.
    pub fn counter_label(&self, counter_id: i32) -> Result<&[u8], CountersReaderError> {
        let offset = self.checked_metadata_offset(counter_id)?;
        self.acquire_record(offset);
        self.label_at(counter_id, offset)
    }

    /// Enumerate allocated counters as value, ID, and borrowed label bytes.
    ///
    /// Reclaimed records are skipped and the scan stops at the first unused
    /// record. Callback allocation, if any, is owned by the caller.
    pub fn for_each_counter(
        &self,
        mut consumer: impl FnMut(i64, i32, &[u8]),
    ) -> Result<(), CountersReaderError> {
        for counter_id in 0..self.capacity() {
            let counter_id = counter_id as i32;
            let metadata_offset = Self::metadata_offset(counter_id)?;
            match self
                .metadata
                .load_i32(metadata_offset + Self::STATE_OFFSET, Ordering::Acquire)
            {
                Self::RECORD_ALLOCATED => {
                    let label = self.label_at(counter_id, metadata_offset)?;
                    let value = self.counter_value(counter_id)?;
                    consumer(value, counter_id, label);
                }
                Self::RECORD_UNUSED => break,
                _ => {}
            }
        }

        Ok(())
    }

    /// Enumerate allocated metadata as ID, type ID, key, and label bytes.
    ///
    /// Reclaimed records are skipped and the scan stops at the first unused
    /// record. Callback allocation, if any, is owned by the caller.
    pub fn for_each_metadata(
        &self,
        mut consumer: impl FnMut(i32, i32, &[u8], &[u8]),
    ) -> Result<(), CountersReaderError> {
        for counter_id in 0..self.capacity() {
            let counter_id = counter_id as i32;
            let metadata_offset = Self::metadata_offset(counter_id)?;
            match self
                .metadata
                .load_i32(metadata_offset + Self::STATE_OFFSET, Ordering::Acquire)
            {
                Self::RECORD_ALLOCATED => {
                    let type_id = self
                        .metadata
                        .load_i32(metadata_offset + Self::TYPE_ID_OFFSET, Ordering::Relaxed);
                    let key = self
                        .metadata
                        .bytes(metadata_offset + Self::KEY_OFFSET, Self::MAX_KEY_LENGTH);
                    let label = self.label_at(counter_id, metadata_offset)?;
                    consumer(counter_id, type_id, key, label);
                }
                Self::RECORD_UNUSED => break,
                _ => {}
            }
        }

        Ok(())
    }

    /// Find the first allocated counter with a registration ID.
    pub fn find_by_registration_id(
        &self,
        registration_id: i64,
    ) -> Result<Option<i32>, CountersReaderError> {
        for counter_id in 0..self.capacity() {
            let counter_id = counter_id as i32;
            match self.counter_state(counter_id)? {
                Self::RECORD_ALLOCATED => {
                    if registration_id == self.counter_registration_id(counter_id)? {
                        return Ok(Some(counter_id));
                    }
                }
                Self::RECORD_UNUSED => break,
                _ => {}
            }
        }

        Ok(None)
    }

    /// Find the first allocated counter with a type and registration ID.
    pub fn find_by_type_id_and_registration_id(
        &self,
        type_id: i32,
        registration_id: i64,
    ) -> Result<Option<i32>, CountersReaderError> {
        for counter_id in 0..self.capacity() {
            let counter_id = counter_id as i32;
            match self.counter_state(counter_id)? {
                Self::RECORD_ALLOCATED => {
                    if type_id == self.counter_type_id(counter_id)?
                        && registration_id == self.counter_registration_id(counter_id)?
                    {
                        return Ok(Some(counter_id));
                    }
                }
                Self::RECORD_UNUSED => break,
                _ => {}
            }
        }

        Ok(None)
    }

    fn validate_record_boundary(
        region: &'static str,
        length: usize,
        record_length: usize,
    ) -> Result<(), CountersReaderError> {
        if length % record_length != 0 {
            return Err(CountersReaderError::PartialRecord {
                region,
                length,
                record_length,
            });
        }

        Ok(())
    }

    fn offset(counter_id: i32, record_length: usize) -> Result<usize, CountersReaderError> {
        if counter_id < 0 {
            return Err(CountersReaderError::CounterIdOutOfRange {
                counter_id,
                max_counter_id: Self::NULL_COUNTER_ID,
            });
        }

        (counter_id as usize).checked_mul(record_length).ok_or(
            CountersReaderError::OffsetOverflow {
                counter_id,
                record_length,
            },
        )
    }

    #[inline]
    fn validate_counter_id(&self, counter_id: i32) -> Result<(), CountersReaderError> {
        Self::validate_id(counter_id, self.max_counter_id)
    }

    #[inline]
    fn checked_counter_offset(&self, counter_id: i32) -> Result<usize, CountersReaderError> {
        self.validate_counter_id(counter_id)?;
        Self::counter_offset(counter_id)
    }

    #[inline]
    fn checked_metadata_offset(&self, counter_id: i32) -> Result<usize, CountersReaderError> {
        self.validate_counter_id(counter_id)?;
        Self::metadata_offset(counter_id)
    }

    #[inline]
    fn acquire_record(&self, metadata_offset: usize) {
        self.metadata
            .load_i32(metadata_offset + Self::STATE_OFFSET, Ordering::Acquire);
    }

    fn label_at(
        &self,
        counter_id: i32,
        metadata_offset: usize,
    ) -> Result<&[u8], CountersReaderError> {
        let label_length = self.metadata.load_i32(
            metadata_offset + Self::LABEL_LENGTH_OFFSET,
            Ordering::Acquire,
        );
        let label_length = usize::try_from(label_length).map_err(|_| {
            CountersReaderError::MalformedLabelLength {
                counter_id,
                label_length,
                maximum_length: Self::MAX_LABEL_LENGTH,
            }
        })?;
        if label_length > Self::MAX_LABEL_LENGTH {
            return Err(CountersReaderError::MalformedLabelLength {
                counter_id,
                label_length: i32::try_from(label_length).unwrap_or(i32::MAX),
                maximum_length: Self::MAX_LABEL_LENGTH,
            });
        }

        Ok(self
            .metadata
            .bytes(metadata_offset + Self::LABEL_VALUE_OFFSET, label_length))
    }
}
