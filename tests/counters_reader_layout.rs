// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Counter ABI layout acceptance for `CTR-LAYOUT-001`.

mod support;

use agrona::concurrent::status::CountersReader;
use support::AlignedBuffer;

#[test]
fn constants_match_the_agrona_and_aeron_abi() {
    assert_eq!(0, CountersReader::DEFAULT_TYPE_ID);
    assert_eq!(0, CountersReader::DEFAULT_REGISTRATION_ID);
    assert_eq!(0, CountersReader::DEFAULT_OWNER_ID);
    assert_eq!(0, CountersReader::DEFAULT_REFERENCE_ID);
    assert_eq!(-1, CountersReader::NULL_COUNTER_ID);
    assert_eq!(0, CountersReader::RECORD_UNUSED);
    assert_eq!(1, CountersReader::RECORD_ALLOCATED);
    assert_eq!(-1, CountersReader::RECORD_RECLAIMED);
    assert_eq!(i64::MAX, CountersReader::NOT_FREE_TO_REUSE);

    assert_eq!(64, CountersReader::CACHE_LINE_LENGTH);
    assert_eq!(0, CountersReader::COUNTER_VALUE_OFFSET);
    assert_eq!(8, CountersReader::REGISTRATION_ID_OFFSET);
    assert_eq!(16, CountersReader::OWNER_ID_OFFSET);
    assert_eq!(24, CountersReader::REFERENCE_ID_OFFSET);
    assert_eq!(128, CountersReader::COUNTER_LENGTH);

    assert_eq!(0, CountersReader::STATE_OFFSET);
    assert_eq!(4, CountersReader::TYPE_ID_OFFSET);
    assert_eq!(8, CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET);
    assert_eq!(16, CountersReader::KEY_OFFSET);
    assert_eq!(112, CountersReader::MAX_KEY_LENGTH);
    assert_eq!(128, CountersReader::LABEL_OFFSET);
    assert_eq!(128, CountersReader::LABEL_LENGTH_OFFSET);
    assert_eq!(132, CountersReader::LABEL_VALUE_OFFSET);
    assert_eq!(384, CountersReader::FULL_LABEL_LENGTH);
    assert_eq!(380, CountersReader::MAX_LABEL_LENGTH);
    assert_eq!(512, CountersReader::METADATA_LENGTH);
    assert_eq!(4, CountersReader::METADATA_TO_VALUES_RATIO);
}

#[test]
fn offsets_use_exact_record_strides() {
    assert_eq!(0, CountersReader::counter_offset(0).unwrap());
    assert_eq!(128, CountersReader::counter_offset(1).unwrap());
    assert_eq!(384, CountersReader::counter_offset(3).unwrap());

    assert_eq!(0, CountersReader::metadata_offset(0).unwrap());
    assert_eq!(512, CountersReader::metadata_offset(1).unwrap());
    assert_eq!(1536, CountersReader::metadata_offset(3).unwrap());
}

#[test]
fn zero_one_and_full_fixture_capacities_are_values_derived() {
    let empty = CountersReader::new(&[], &[]).unwrap();
    assert_eq!(-1, empty.max_counter_id());
    assert_eq!(0, empty.capacity());

    let metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let one = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();
    assert_eq!(0, one.max_counter_id());
    assert_eq!(1, one.capacity());

    const CAPACITY: usize = 32;
    let metadata = AlignedBuffer::new(CAPACITY * CountersReader::METADATA_LENGTH);
    let values = AlignedBuffer::new(CAPACITY * CountersReader::COUNTER_LENGTH);
    let full = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();
    assert_eq!(CAPACITY as i32 - 1, full.max_counter_id());
    assert_eq!(CAPACITY, full.capacity());
}
