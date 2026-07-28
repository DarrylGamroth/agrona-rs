// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Dense counter enumeration and search acceptance for `CTR-ITER-001`.

mod support;

use agrona::concurrent::status::{CountersReader, CountersReaderError};
use support::{AlignedBuffer, put_i32, put_i64};

const CAPACITY: usize = 5;

fn populated_regions() -> (AlignedBuffer, AlignedBuffer) {
    let mut metadata = AlignedBuffer::new(CAPACITY * CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(CAPACITY * CountersReader::COUNTER_LENGTH);
    let states = [
        CountersReader::RECORD_ALLOCATED,
        CountersReader::RECORD_RECLAIMED,
        CountersReader::RECORD_ALLOCATED,
        CountersReader::RECORD_UNUSED,
        CountersReader::RECORD_ALLOCATED,
    ];

    for (counter_id, state) in states.into_iter().enumerate() {
        let metadata_offset = counter_id * CountersReader::METADATA_LENGTH;
        let values_offset = counter_id * CountersReader::COUNTER_LENGTH;
        put_i32(
            metadata.as_bytes_mut(),
            metadata_offset + CountersReader::STATE_OFFSET,
            state,
        );
        put_i32(
            metadata.as_bytes_mut(),
            metadata_offset + CountersReader::TYPE_ID_OFFSET,
            10 + counter_id as i32,
        );
        let label = format!("counter-{counter_id}");
        put_i32(
            metadata.as_bytes_mut(),
            metadata_offset + CountersReader::LABEL_LENGTH_OFFSET,
            label.len() as i32,
        );
        metadata.as_bytes_mut()[metadata_offset + CountersReader::LABEL_VALUE_OFFSET
            ..metadata_offset + CountersReader::LABEL_VALUE_OFFSET + label.len()]
            .copy_from_slice(label.as_bytes());
        metadata.as_bytes_mut()[metadata_offset + CountersReader::KEY_OFFSET] = counter_id as u8;

        put_i64(
            values.as_bytes_mut(),
            values_offset + CountersReader::COUNTER_VALUE_OFFSET,
            100 + counter_id as i64,
        );
        put_i64(
            values.as_bytes_mut(),
            values_offset + CountersReader::REGISTRATION_ID_OFFSET,
            1_000 + counter_id as i64,
        );
    }

    (metadata, values)
}

#[test]
fn enumeration_skips_reclaimed_and_stops_at_first_unused() {
    let (metadata, values) = populated_regions();
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    let mut counters = Vec::new();
    reader
        .for_each_counter(|value, counter_id, label| {
            counters.push((value, counter_id, label.to_vec()));
        })
        .unwrap();
    assert_eq!(
        vec![
            (100, 0, b"counter-0".to_vec()),
            (102, 2, b"counter-2".to_vec())
        ],
        counters
    );

    let mut records = Vec::new();
    reader
        .for_each_metadata(|counter_id, type_id, key, label| {
            records.push((counter_id, type_id, key[0], label.to_vec()));
        })
        .unwrap();
    assert_eq!(
        vec![
            (0, 10, 0, b"counter-0".to_vec()),
            (2, 12, 2, b"counter-2".to_vec())
        ],
        records
    );
}

#[test]
fn searches_only_allocated_records_before_first_unused() {
    let (metadata, values) = populated_regions();
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    assert_eq!(Some(0), reader.find_by_registration_id(1_000).unwrap());
    assert_eq!(Some(2), reader.find_by_registration_id(1_002).unwrap());
    assert_eq!(None, reader.find_by_registration_id(1_001).unwrap());
    assert_eq!(None, reader.find_by_registration_id(1_004).unwrap());

    assert_eq!(
        Some(2),
        reader
            .find_by_type_id_and_registration_id(12, 1_002)
            .unwrap()
    );
    assert_eq!(
        None,
        reader
            .find_by_type_id_and_registration_id(10, 1_002)
            .unwrap()
    );
}

#[test]
fn enumeration_reports_a_malformed_published_label() {
    let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::STATE_OFFSET,
        CountersReader::RECORD_ALLOCATED,
    );
    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::LABEL_LENGTH_OFFSET,
        381,
    );
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    assert!(matches!(
        reader.for_each_counter(|_, _, _| {}),
        Err(CountersReaderError::MalformedLabelLength { .. })
    ));
    assert!(matches!(
        reader.for_each_metadata(|_, _, _, _| {}),
        Err(CountersReaderError::MalformedLabelLength { .. })
    ));
}
