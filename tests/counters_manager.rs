// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Counter manager allocation, reclamation, and mutation tests.

mod support;

use std::convert::Infallible;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use agrona::clock::EpochClock;
use agrona::concurrent::status::{
    CounterAllocationError, CountersManager, CountersManagerError, CountersReader,
};
use support::{AlignedBuffer, put_i32};

#[derive(Clone, Debug)]
struct TestClock(Arc<AtomicI64>);

impl TestClock {
    fn new(time_ms: i64) -> Self {
        Self(Arc::new(AtomicI64::new(time_ms)))
    }

    fn update(&self, time_ms: i64) {
        self.0.store(time_ms, Ordering::Release);
    }
}

impl EpochClock for TestClock {
    fn time(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

fn regions(capacity: usize) -> (AlignedBuffer, AlignedBuffer) {
    (
        AlignedBuffer::new(capacity * CountersReader::METADATA_LENGTH),
        AlignedBuffer::new(capacity * CountersReader::COUNTER_LENGTH),
    )
}

#[test]
fn validates_regions_and_reports_capacity_and_exhaustion() {
    let (mut metadata, mut values) = regions(2);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();

    assert_eq!(2, manager.capacity());
    assert_eq!(2, manager.available());
    assert_eq!(0, manager.allocate(b"zero").unwrap());
    assert_eq!(1, manager.available());
    assert_eq!(1, manager.allocate_with_type(b"one", 7).unwrap());
    assert_eq!(0, manager.available());
    assert_eq!(
        Err(CountersManagerError::Full { max_counter_id: 1 }),
        manager.allocate(b"full")
    );
}

#[test]
fn accepts_empty_regions_and_rejects_malformed_or_misaligned_regions() {
    let (mut empty_metadata, mut empty_values) = regions(0);
    let mut empty =
        CountersManager::new(empty_metadata.as_bytes_mut(), empty_values.as_bytes_mut()).unwrap();
    assert_eq!(0, empty.capacity());
    assert_eq!(0, empty.available());
    assert_eq!(
        Err(CountersManagerError::Full { max_counter_id: -1 }),
        empty.allocate(b"x")
    );

    let mut short_metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let mut two_values = AlignedBuffer::new(2 * CountersReader::COUNTER_LENGTH);
    assert!(
        CountersManager::new(short_metadata.as_bytes_mut(), two_values.as_bytes_mut()).is_err()
    );

    let mut shifted_metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH + 1);
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    assert!(
        CountersManager::new(
            &mut shifted_metadata.as_bytes_mut()[1..],
            values.as_bytes_mut()
        )
        .is_err()
    );
}

#[test]
fn publishes_exact_metadata_and_truncates_raw_fields() {
    let (mut metadata, mut values) = regions(1);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let key = vec![0x5a; CountersReader::MAX_KEY_LENGTH + 9];
    let label = vec![0x6b; CountersReader::MAX_LABEL_LENGTH + 11];

    let id = manager.allocate_raw(42, Some(&key), &label).unwrap();
    let reader = manager.reader();
    assert_eq!(
        CountersReader::RECORD_ALLOCATED,
        reader.counter_state(id).unwrap()
    );
    assert_eq!(42, reader.counter_type_id(id).unwrap());
    assert_eq!(
        CountersReader::NOT_FREE_TO_REUSE,
        reader.free_for_reuse_deadline(id).unwrap()
    );
    assert_eq!(
        &key[..CountersReader::MAX_KEY_LENGTH],
        reader.counter_key(id).unwrap()
    );
    assert_eq!(
        &label[..CountersReader::MAX_LABEL_LENGTH],
        reader.counter_label(id).unwrap()
    );
    assert_eq!(0, reader.counter_value(id).unwrap());
    assert_eq!(0, reader.counter_registration_id(id).unwrap());
    assert_eq!(0, reader.counter_owner_id(id).unwrap());
    assert_eq!(0, reader.counter_reference_id(id).unwrap());
}

#[test]
fn key_initializer_failure_returns_id_to_the_free_list() {
    let (mut metadata, mut values) = regions(1);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();

    let error = manager.allocate_with_key(b"failed", 1, |key| {
        key[0] = 9;
        Err("no key")
    });
    assert_eq!(Err(CounterAllocationError::KeyInitializer("no key")), error);
    assert_eq!(1, manager.available());
    assert_eq!(0, manager.allocate(b"recovered").unwrap());
}

#[test]
fn free_observes_delay_then_reuse_resets_value_and_identity() {
    let (mut metadata, mut values) = regions(2);
    let clock = TestClock::new(100);
    let mut manager = CountersManager::with_clock(
        metadata.as_bytes_mut(),
        values.as_bytes_mut(),
        clock.clone(),
        10,
    )
    .unwrap();
    let id = manager.allocate_raw(3, Some(&[7; 16]), b"first").unwrap();
    manager.set_counter_value(id, 91).unwrap();
    manager.set_counter_registration_id(id, 92).unwrap();
    manager.set_counter_owner_id(id, 93).unwrap();
    manager.set_counter_reference_id(id, 94).unwrap();

    manager.free(id).unwrap();
    {
        let reader = manager.reader();
        assert_eq!(
            CountersReader::RECORD_RECLAIMED,
            reader.counter_state(id).unwrap()
        );
        assert_eq!(
            &[0; CountersReader::MAX_KEY_LENGTH],
            reader.counter_key(id).unwrap()
        );
        assert_eq!(110, reader.free_for_reuse_deadline(id).unwrap());
    }
    assert_eq!(1, manager.available());
    assert_eq!(1, manager.allocate(b"second").unwrap());

    clock.update(109);
    assert_eq!(0, manager.available());
    clock.update(110);
    assert_eq!(1, manager.available());
    assert_eq!(id, manager.allocate_with_type(b"reused", 8).unwrap());

    let reader = manager.reader();
    assert_eq!(0, reader.counter_value(id).unwrap());
    assert_eq!(0, reader.counter_registration_id(id).unwrap());
    assert_eq!(0, reader.counter_owner_id(id).unwrap());
    assert_eq!(0, reader.counter_reference_id(id).unwrap());
    assert_eq!(8, reader.counter_type_id(id).unwrap());
}

#[test]
fn free_rejects_invalid_and_non_allocated_ids_and_wraps_deadline() {
    let (mut metadata, mut values) = regions(1);
    let clock = TestClock::new(i64::MAX);
    let mut manager =
        CountersManager::with_clock(metadata.as_bytes_mut(), values.as_bytes_mut(), clock, 1)
            .unwrap();

    assert!(matches!(
        manager.free(-1),
        Err(CountersManagerError::Reader(_))
    ));
    assert_eq!(
        Err(CountersManagerError::CounterNotAllocated {
            counter_id: 0,
            state: CountersReader::RECORD_UNUSED,
        }),
        manager.free(0)
    );
    let id = manager.allocate(b"x").unwrap();
    manager.free(id).unwrap();
    assert_eq!(
        Err(CountersManagerError::CounterNotAllocated {
            counter_id: id,
            state: CountersReader::RECORD_RECLAIMED,
        }),
        manager.free(id)
    );
    assert_eq!(
        i64::MIN,
        manager.reader().free_for_reuse_deadline(id).unwrap()
    );
}

#[test]
fn manager_mutations_preserve_order_and_field_boundaries() {
    let (mut metadata, mut values) = regions(1);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let id = manager
        .allocate_with_key(b"label", 1, |key| {
            key.fill(3);
            Ok::<_, Infallible>(())
        })
        .unwrap();

    manager.set_counter_value(id, 11).unwrap();
    manager.set_counter_registration_id(id, 12).unwrap();
    manager.set_counter_owner_id(id, 13).unwrap();
    manager.set_counter_reference_id(id, 14).unwrap();
    manager.set_counter_key(id, &[8, 9]).unwrap();
    manager.set_counter_label(id, b"a").unwrap();
    assert_eq!(2, manager.append_to_label(id, b"bc").unwrap());

    let reader = manager.reader();
    assert_eq!(11, reader.counter_value(id).unwrap());
    assert_eq!(12, reader.counter_registration_id(id).unwrap());
    assert_eq!(13, reader.counter_owner_id(id).unwrap());
    assert_eq!(14, reader.counter_reference_id(id).unwrap());
    assert_eq!(&[8, 9, 3], &reader.counter_key(id).unwrap()[..3]);
    assert_eq!(b"abc", reader.counter_label(id).unwrap());
}

#[test]
fn replacement_key_rejects_oversize_and_label_append_stops_at_capacity() {
    let (mut metadata, mut values) = regions(1);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let id = manager
        .allocate(&vec![b'a'; CountersReader::MAX_LABEL_LENGTH - 1])
        .unwrap();

    assert_eq!(1, manager.append_to_label(id, b"bc").unwrap());
    assert_eq!(0, manager.append_to_label(id, b"d").unwrap());
    assert_eq!(
        Err(CountersManagerError::KeyTooLong {
            length: CountersReader::MAX_KEY_LENGTH + 1,
            maximum_length: CountersReader::MAX_KEY_LENGTH,
        }),
        manager.set_counter_key(id, &[0; CountersReader::MAX_KEY_LENGTH + 1])
    );
    assert_eq!(
        CountersReader::MAX_LABEL_LENGTH,
        manager.reader().counter_label(id).unwrap().len()
    );
}

#[test]
fn append_rejects_a_malformed_published_label_length() {
    let (mut metadata, mut values) = regions(1);
    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::LABEL_LENGTH_OFFSET,
        -1,
    );
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    assert!(matches!(
        manager.append_to_label(0, b"x"),
        Err(CountersManagerError::Reader(
            agrona::concurrent::status::CountersReaderError::MalformedLabelLength { .. }
        ))
    ));
}

#[test]
fn typed_errors_have_actionable_display_and_sources() {
    let manager_error = CountersManagerError::Full { max_counter_id: 3 };
    assert!(manager_error.to_string().contains("full"));

    let allocation_error = CounterAllocationError::KeyInitializer(std::io::Error::other("key"));
    let error: &dyn Error = &allocation_error;
    assert!(error.to_string().contains("key initializer"));
    assert_eq!("key", error.source().unwrap().to_string());
}
