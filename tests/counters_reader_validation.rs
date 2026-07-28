// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Counter region validation acceptance for `CTR-VALID-001`.

mod support;

use agrona::concurrent::status::{CountersReader, CountersReaderError};
use support::{AlignedBuffer, put_i32};

#[test]
fn rejects_partial_and_insufficient_regions() {
    let partial_metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH - 1);
    assert!(matches!(
        CountersReader::new(partial_metadata.as_bytes(), &[]),
        Err(CountersReaderError::PartialRecord {
            region: "metadata",
            ..
        })
    ));

    let partial_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH - 1);
    assert!(matches!(
        CountersReader::new(&[], partial_values.as_bytes()),
        Err(CountersReaderError::PartialRecord {
            region: "values",
            ..
        })
    ));

    let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    assert_eq!(
        CountersReaderError::MetadataTooSmall {
            actual: 0,
            required: CountersReader::METADATA_LENGTH,
        },
        CountersReader::new(&[], values.as_bytes()).unwrap_err()
    );
}

#[test]
fn rejects_misaligned_metadata_and_values_bases() {
    let metadata_backing = AlignedBuffer::new(CountersReader::METADATA_LENGTH + 8);
    let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let misaligned_metadata = &metadata_backing.as_bytes()[1..1 + CountersReader::METADATA_LENGTH];
    assert!(matches!(
        CountersReader::new(misaligned_metadata, values.as_bytes()),
        Err(CountersReaderError::MisalignedRegion {
            region: "metadata",
            ..
        })
    ));

    let metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let values_backing = AlignedBuffer::new(CountersReader::COUNTER_LENGTH + 8);
    let misaligned_values = &values_backing.as_bytes()[1..1 + CountersReader::COUNTER_LENGTH];
    assert!(matches!(
        CountersReader::new(metadata.as_bytes(), misaligned_values),
        Err(CountersReaderError::MisalignedRegion {
            region: "values",
            ..
        })
    ));
}

#[test]
fn rejects_negative_and_out_of_range_counter_ids_before_access() {
    let metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    for error in [
        reader.counter_value(-1).unwrap_err(),
        reader.counter_registration_id(1).unwrap_err(),
        reader.counter_owner_id(-2).unwrap_err(),
        reader.counter_reference_id(7).unwrap_err(),
        reader.counter_state(-1).unwrap_err(),
        reader.counter_type_id(1).unwrap_err(),
        reader.free_for_reuse_deadline(2).unwrap_err(),
        reader.counter_key(-1).unwrap_err(),
        reader.counter_label(1).unwrap_err(),
    ] {
        assert!(matches!(
            error,
            CountersReaderError::CounterIdOutOfRange { .. }
        ));
    }

    assert!(matches!(
        CountersReader::counter_offset(-1),
        Err(CountersReaderError::CounterIdOutOfRange { .. })
    ));
    assert!(matches!(
        CountersReader::metadata_offset(-1),
        Err(CountersReaderError::CounterIdOutOfRange { .. })
    ));
}

#[test]
fn rejects_negative_and_oversized_label_lengths() {
    for malformed in [-1, CountersReader::MAX_LABEL_LENGTH as i32 + 1] {
        let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
        let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
        put_i32(
            metadata.as_bytes_mut(),
            CountersReader::LABEL_LENGTH_OFFSET,
            malformed,
        );
        let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

        assert_eq!(
            Err(CountersReaderError::MalformedLabelLength {
                counter_id: 0,
                label_length: malformed,
                maximum_length: CountersReader::MAX_LABEL_LENGTH,
            }),
            reader.counter_label(0)
        );
    }
}

#[test]
fn errors_have_actionable_rust_display_messages() {
    let errors = [
        CountersReaderError::PartialRecord {
            region: "values",
            length: 127,
            record_length: 128,
        },
        CountersReaderError::MetadataTooSmall {
            actual: 511,
            required: 512,
        },
        CountersReaderError::MisalignedRegion {
            region: "metadata",
            address: 1,
            required_alignment: 8,
        },
        CountersReaderError::CapacityTooLarge {
            capacity: i32::MAX as usize + 2,
            maximum_capacity: i32::MAX as usize + 1,
        },
        CountersReaderError::CounterIdOutOfRange {
            counter_id: -1,
            max_counter_id: 0,
        },
        CountersReaderError::OffsetOverflow {
            counter_id: i32::MAX,
            record_length: usize::MAX,
        },
        CountersReaderError::MalformedLabelLength {
            counter_id: 0,
            label_length: 381,
            maximum_length: CountersReader::MAX_LABEL_LENGTH,
        },
        CountersReaderError::RegionSizeOverflow {
            values_length: usize::MAX,
        },
    ];

    for error in errors {
        let message = error.to_string();
        assert!(!message.is_empty());
        assert!(message.chars().any(char::is_numeric));
    }
}
