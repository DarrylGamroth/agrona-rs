// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Interoperability acceptance against buffers produced by Agrona Java.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use agrona::clock::EpochClock;
use agrona::concurrent::status::{CountersManager, CountersReader};
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

    assert_eq!(5, reader.capacity());
    assert_eq!(4, reader.max_counter_id());

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
        CountersReader::RECORD_ALLOCATED,
        reader.counter_state(2).unwrap()
    );
    assert_eq!(55, reader.counter_type_id(2).unwrap());
    assert_eq!(0, reader.counter_value(2).unwrap());
    assert_eq!(0, reader.counter_registration_id(2).unwrap());
    assert_eq!(0, reader.counter_owner_id(2).unwrap());
    assert_eq!(0, reader.counter_reference_id(2).unwrap());
    assert_eq!(b"reused", reader.counter_label(2).unwrap());

    assert_eq!(
        CountersReader::RECORD_RECLAIMED,
        reader.counter_state(3).unwrap()
    );
    assert_eq!(99, reader.counter_type_id(3).unwrap());
    assert_eq!(1_344, reader.free_for_reuse_deadline(3).unwrap());
    assert_eq!(177, reader.counter_value(3).unwrap());
    assert_eq!(188, reader.counter_registration_id(3).unwrap());
    assert!(reader.counter_key(3).unwrap().iter().all(|byte| *byte == 0));

    assert_eq!(
        CountersReader::RECORD_UNUSED,
        reader.counter_state(4).unwrap()
    );

    let mut enumerated = Vec::new();
    reader
        .for_each_counter(|value, counter_id, label| {
            enumerated.push((value, counter_id, label.len()));
        })
        .unwrap();
    assert_eq!(
        vec![
            (42, 0, 5),
            (-42, 1, CountersReader::MAX_LABEL_LENGTH),
            (0, 2, 6),
        ],
        enumerated
    );

    assert_eq!(Some(0), reader.find_by_registration_id(1_001).unwrap());
    assert_eq!(
        Some(1),
        reader
            .find_by_type_id_and_registration_id(-5, -1_001)
            .unwrap()
    );
    assert_eq!(None, reader.find_by_registration_id(188).unwrap());
}

#[derive(Clone, Copy)]
struct FixedClock(i64);

impl EpochClock for FixedClock {
    fn time(&self) -> i64 {
        self.0
    }
}

#[test]
fn produces_regions_for_the_pinned_agrona_java_reader() {
    let mut metadata = AlignedBuffer::new(4 * CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(4 * CountersReader::COUNTER_LENGTH);
    let mut manager = CountersManager::with_clock(
        metadata.as_bytes_mut(),
        values.as_bytes_mut(),
        FixedClock(2_000),
        0,
    )
    .unwrap();

    let first = manager
        .allocate_raw(17, Some(b"rust-key"), b"rust-alpha")
        .unwrap();
    manager.set_counter_value(first, 142).unwrap();
    manager.set_counter_registration_id(first, 1_101).unwrap();
    manager.set_counter_owner_id(first, 2_202).unwrap();
    manager.set_counter_reference_id(first, 3_303).unwrap();

    let maximum_key = vec![0xa6; CountersReader::MAX_KEY_LENGTH];
    let maximum_label = vec![b'z'; CountersReader::MAX_LABEL_LENGTH];
    let maximum = manager
        .allocate_raw(-15, Some(&maximum_key), &maximum_label)
        .unwrap();
    manager.set_counter_value(maximum, -142).unwrap();

    let reused = manager.allocate(b"old").unwrap();
    manager.set_counter_value(reused, 900).unwrap();
    manager.set_counter_registration_id(reused, 901).unwrap();
    manager.set_counter_owner_id(reused, 902).unwrap();
    manager.set_counter_reference_id(reused, 903).unwrap();
    manager.free(reused).unwrap();
    assert_eq!(
        reused,
        manager
            .allocate_raw(23, Some(b"reused-key"), b"reused")
            .unwrap()
    );

    let reclaimed = manager.allocate(b"reclaimed").unwrap();
    manager.set_counter_value(reclaimed, 77).unwrap();
    manager.free(reclaimed).unwrap();

    {
        let reader = manager.reader();
        assert_eq!(b"rust-alpha", reader.counter_label(first).unwrap());
        assert_eq!(
            CountersReader::MAX_KEY_LENGTH,
            reader.counter_key(maximum).unwrap().len()
        );
        assert_eq!(0, reader.counter_value(reused).unwrap());
        assert_eq!(0, reader.counter_registration_id(reused).unwrap());
        assert_eq!(0, reader.counter_owner_id(reused).unwrap());
        assert_eq!(0, reader.counter_reference_id(reused).unwrap());
        assert_eq!(
            CountersReader::RECORD_RECLAIMED,
            reader.counter_state(reclaimed).unwrap()
        );
    }
    drop(manager);

    if let Some(output) = std::env::var_os("AGRONA_RUST_COUNTER_FIXTURE_DIR") {
        let output = PathBuf::from(output);
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("metadata.bin"), metadata.as_bytes()).unwrap();
        fs::write(output.join("values.bin"), values.as_bytes()).unwrap();
    }
}
