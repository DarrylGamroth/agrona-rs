// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Direct counter and metadata read acceptance.

mod support;

use agrona::concurrent::status::CountersReader;
use support::{AlignedBuffer, put_i32, put_i64};

#[test]
fn reads_state_value_identities_and_complete_metadata() {
    let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);

    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::STATE_OFFSET,
        CountersReader::RECORD_ALLOCATED,
    );
    put_i32(metadata.as_bytes_mut(), CountersReader::TYPE_ID_OFFSET, 77);
    put_i64(
        metadata.as_bytes_mut(),
        CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
        123_456,
    );
    for (index, byte) in metadata.as_bytes_mut()
        [CountersReader::KEY_OFFSET..CountersReader::KEY_OFFSET + CountersReader::MAX_KEY_LENGTH]
        .iter_mut()
        .enumerate()
    {
        *byte = index as u8;
    }
    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::LABEL_LENGTH_OFFSET,
        CountersReader::MAX_LABEL_LENGTH as i32,
    );
    metadata.as_bytes_mut()[CountersReader::LABEL_VALUE_OFFSET
        ..CountersReader::LABEL_VALUE_OFFSET + CountersReader::MAX_LABEL_LENGTH]
        .fill(b'x');

    put_i64(
        values.as_bytes_mut(),
        CountersReader::COUNTER_VALUE_OFFSET,
        -9,
    );
    put_i64(
        values.as_bytes_mut(),
        CountersReader::REGISTRATION_ID_OFFSET,
        101,
    );
    put_i64(values.as_bytes_mut(), CountersReader::OWNER_ID_OFFSET, 202);
    put_i64(
        values.as_bytes_mut(),
        CountersReader::REFERENCE_ID_OFFSET,
        303,
    );

    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();
    assert_eq!(
        CountersReader::RECORD_ALLOCATED,
        reader.counter_state(0).unwrap()
    );
    assert_eq!(-9, reader.counter_value(0).unwrap());
    assert_eq!(101, reader.counter_registration_id(0).unwrap());
    assert_eq!(202, reader.counter_owner_id(0).unwrap());
    assert_eq!(303, reader.counter_reference_id(0).unwrap());
    assert_eq!(77, reader.counter_type_id(0).unwrap());
    assert_eq!(123_456, reader.free_for_reuse_deadline(0).unwrap());
    assert_eq!(
        CountersReader::MAX_KEY_LENGTH,
        reader.counter_key(0).unwrap().len()
    );
    assert_eq!(0, reader.counter_key(0).unwrap()[0]);
    assert_eq!(111, reader.counter_key(0).unwrap()[111]);
    assert_eq!(
        &[b'x'; CountersReader::MAX_LABEL_LENGTH],
        reader.counter_label(0).unwrap()
    );
}

#[test]
fn exposes_the_original_borrowed_regions() {
    let metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    assert!(std::ptr::eq(
        metadata.as_bytes().as_ptr(),
        reader.metadata_region().as_ptr()
    ));
    assert!(std::ptr::eq(
        values.as_bytes().as_ptr(),
        reader.values_region().as_ptr()
    ));
}
