// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Interoperability acceptance against buffers produced by Agrona Java.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use agrona::concurrent::status::CountersReader;
use support::AlignedBuffer;

fn fixture_directory() -> PathBuf {
    std::env::var_os("AGRONA_COUNTER_FIXTURE_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/counters"),
        PathBuf::from,
    )
}

fn aligned_fixture(name: &str) -> AlignedBuffer {
    let bytes = fs::read(fixture_directory().join(name)).unwrap();
    let mut aligned = AlignedBuffer::new(bytes.len());
    aligned.as_bytes_mut().copy_from_slice(&bytes);
    aligned
}

#[test]
fn reads_regions_generated_by_the_pinned_agrona_java_implementation() {
    let metadata = aligned_fixture("metadata.bin");
    let values = aligned_fixture("values.bin");
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    assert_eq!(4, reader.capacity());
    assert_eq!(3, reader.max_counter_id());

    assert_eq!(
        CountersReader::RECORD_ALLOCATED,
        reader.counter_state(0).unwrap()
    );
    assert_eq!(7, reader.counter_type_id(0).unwrap());
    assert_eq!(
        CountersReader::NOT_FREE_TO_REUSE,
        reader.free_for_reuse_deadline(0).unwrap()
    );
    assert_eq!(42, reader.counter_value(0).unwrap());
    assert_eq!(1_001, reader.counter_registration_id(0).unwrap());
    assert_eq!(2_002, reader.counter_owner_id(0).unwrap());
    assert_eq!(3_003, reader.counter_reference_id(0).unwrap());
    assert_eq!(b"alpha", reader.counter_label(0).unwrap());
    assert_eq!(
        (0..CountersReader::MAX_KEY_LENGTH as u8).collect::<Vec<_>>(),
        reader.counter_key(0).unwrap()
    );

    assert_eq!(
        CountersReader::RECORD_ALLOCATED,
        reader.counter_state(1).unwrap()
    );
    assert_eq!(-5, reader.counter_type_id(1).unwrap());
    assert_eq!(-42, reader.counter_value(1).unwrap());
    assert_eq!(-1_001, reader.counter_registration_id(1).unwrap());
    assert_eq!(-2_002, reader.counter_owner_id(1).unwrap());
    assert_eq!(-3_003, reader.counter_reference_id(1).unwrap());
    assert_eq!(
        &[b'x'; CountersReader::MAX_LABEL_LENGTH],
        reader.counter_label(1).unwrap()
    );
    assert!(
        reader
            .counter_key(1)
            .unwrap()
            .iter()
            .all(|byte| *byte == 0xA5)
    );

    assert_eq!(
        CountersReader::RECORD_RECLAIMED,
        reader.counter_state(2).unwrap()
    );
    assert_eq!(99, reader.counter_type_id(2).unwrap());
    assert_eq!(1_289, reader.free_for_reuse_deadline(2).unwrap());
    assert_eq!(77, reader.counter_value(2).unwrap());
    assert_eq!(88, reader.counter_registration_id(2).unwrap());
    assert!(reader.counter_key(2).unwrap().iter().all(|byte| *byte == 0));

    assert_eq!(
        CountersReader::RECORD_UNUSED,
        reader.counter_state(3).unwrap()
    );

    let mut enumerated = Vec::new();
    reader
        .for_each_counter(|value, counter_id, label| {
            enumerated.push((value, counter_id, label.len()));
        })
        .unwrap();
    assert_eq!(
        vec![(42, 0, 5), (-42, 1, CountersReader::MAX_LABEL_LENGTH)],
        enumerated
    );

    assert_eq!(Some(0), reader.find_by_registration_id(1_001).unwrap());
    assert_eq!(
        Some(1),
        reader
            .find_by_type_id_and_registration_id(-5, -1_001)
            .unwrap()
    );
    assert_eq!(None, reader.find_by_registration_id(88).unwrap());
}
