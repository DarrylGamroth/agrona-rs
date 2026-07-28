// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Failure to construct or access an Agrona-compatible counter reader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CountersReaderError {
    /// A region length does not end on a complete record boundary.
    PartialRecord {
        /// Name of the rejected region.
        region: &'static str,
        /// Supplied region length in bytes.
        length: usize,
        /// Required record length in bytes.
        record_length: usize,
    },
    /// The metadata region cannot describe every values record.
    MetadataTooSmall {
        /// Supplied metadata length in bytes.
        actual: usize,
        /// Minimum required metadata length in bytes.
        required: usize,
    },
    /// A region base is not naturally aligned for atomic access.
    MisalignedRegion {
        /// Name of the rejected region.
        region: &'static str,
        /// Supplied base address.
        address: usize,
        /// Required byte alignment.
        required_alignment: usize,
    },
    /// The values-derived capacity cannot be represented by Agrona counter IDs.
    CapacityTooLarge {
        /// Number of complete values records.
        capacity: usize,
        /// Largest supported number of records.
        maximum_capacity: usize,
    },
    /// A counter ID is outside the values-derived reader capacity.
    CounterIdOutOfRange {
        /// Rejected counter ID.
        counter_id: i32,
        /// Greatest valid ID, or `-1` for an empty reader.
        max_counter_id: i32,
    },
    /// An offset calculation overflowed the target address space.
    OffsetOverflow {
        /// Counter ID whose offset could not be represented.
        counter_id: i32,
        /// Record stride used for the calculation.
        record_length: usize,
    },
    /// A published label length is outside the metadata record.
    MalformedLabelLength {
        /// Counter containing the malformed length.
        counter_id: i32,
        /// Published signed label length.
        label_length: i32,
        /// Greatest valid label length.
        maximum_length: usize,
    },
    /// The values length cannot be multiplied by the metadata ratio.
    RegionSizeOverflow {
        /// Supplied values-region length.
        values_length: usize,
    },
}

impl Display for CountersReaderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PartialRecord {
                region,
                length,
                record_length,
            } => write!(
                formatter,
                "{region} region length {length} is not a multiple of record length {record_length}"
            ),
            Self::MetadataTooSmall { actual, required } => write!(
                formatter,
                "metadata region is too small: {actual} bytes supplied, {required} required"
            ),
            Self::MisalignedRegion {
                region,
                address,
                required_alignment,
            } => write!(
                formatter,
                "{region} region address {address:#x} is not aligned to {required_alignment} bytes"
            ),
            Self::CapacityTooLarge {
                capacity,
                maximum_capacity,
            } => write!(
                formatter,
                "counter capacity {capacity} exceeds maximum {maximum_capacity}"
            ),
            Self::CounterIdOutOfRange {
                counter_id,
                max_counter_id,
            } => write!(
                formatter,
                "counter id {counter_id} is out of range: 0..={max_counter_id}"
            ),
            Self::OffsetOverflow {
                counter_id,
                record_length,
            } => write!(
                formatter,
                "offset for counter id {counter_id} and record length {record_length} overflowed"
            ),
            Self::MalformedLabelLength {
                counter_id,
                label_length,
                maximum_length,
            } => write!(
                formatter,
                "counter {counter_id} label length {label_length} is outside 0..={maximum_length}"
            ),
            Self::RegionSizeOverflow { values_length } => write!(
                formatter,
                "metadata size calculation overflowed for values length {values_length}"
            ),
        }
    }
}

impl Error for CountersReaderError {}
