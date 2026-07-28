// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Position and status-indicator contract tests.

mod support;

use std::sync::Arc;

use agrona::concurrent::status::{
    AtomicLongPosition, CountersManager, CountersReader, Position, ReadablePosition,
    StatusIndicator, StatusIndicatorReader, UnsafeBufferPosition, UnsafeBufferStatusIndicator,
};
use support::AlignedBuffer;

fn exercise_position(position: &dyn Position) {
    position.set(1);
    assert_eq!(1, position.get());
    position.set_opaque(2);
    assert_eq!(2, position.get_opaque());
    position.set_ordered(3);
    assert_eq!(3, position.get_acquire());
    position.set_volatile(4);
    assert_eq!(4, position.get_volatile());
    assert!(!position.propose_max(4));
    assert!(position.propose_max_ordered(5));
    assert!(position.propose_max_opaque(6));
    position.close();
    assert!(position.is_closed());
}

#[test]
fn atomic_long_position_matches_position_contract() {
    let position = AtomicLongPosition::with_id_and_value(7, -1);
    assert_eq!(7, position.id());
    assert_eq!(-1, position.get());
    exercise_position(&position);

    let default = AtomicLongPosition::default();
    assert_eq!(0, default.id());
    assert_eq!(0, default.get());
}

#[test]
fn buffer_position_uses_exact_counter_stride() {
    let mut metadata = AlignedBuffer::new(2 * CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(2 * CountersReader::COUNTER_LENGTH);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    manager.allocate(b"first").unwrap();
    manager.allocate(b"second").unwrap();
    let first = UnsafeBufferPosition::from_counter(manager.counter_handle(0).unwrap());
    let second = UnsafeBufferPosition::from_counter(manager.counter_handle(1).unwrap());
    first.set_volatile(11);
    second.set_volatile(22);
    assert_eq!(11, first.get_volatile());
    assert_eq!(22, second.get_volatile());
    exercise_position(&second);
}

#[test]
fn status_indicator_matches_ordering_contract_and_stride() {
    let mut metadata = AlignedBuffer::new(2 * CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(2 * CountersReader::COUNTER_LENGTH);
    let manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let first = UnsafeBufferStatusIndicator::from_counter(manager.counter_handle(0).unwrap());
    let second = UnsafeBufferStatusIndicator::from_counter(manager.counter_handle(1).unwrap());

    first.set_volatile(8);
    second.set_opaque(9);
    assert_eq!(8, first.get_volatile());
    assert_eq!(9, second.get_opaque());
    second.set_ordered(10);
    assert_eq!(10, second.get_acquire());
}

#[test]
fn buffer_wrappers_reject_invalid_ids_and_partial_regions() {
    let mut first_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut second_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut partial_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    assert!(UnsafeBufferPosition::new(first_values.as_bytes_mut(), -1).is_err());
    assert!(UnsafeBufferStatusIndicator::new(second_values.as_bytes_mut(), 1).is_err());
    assert!(
        UnsafeBufferPosition::new(
            &mut partial_values.as_bytes_mut()[..CountersReader::COUNTER_LENGTH - 1],
            0
        )
        .is_err()
    );
}

#[test]
fn release_writes_publish_to_acquire_readers() {
    let local = Arc::new(AtomicLongPosition::new());
    let mut metadata = AlignedBuffer::new(2 * CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(2 * CountersReader::COUNTER_LENGTH);
    let manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let buffer = Arc::new(UnsafeBufferPosition::from_counter(
        manager.counter_handle(0).unwrap(),
    ));
    let status = Arc::new(UnsafeBufferStatusIndicator::from_counter(
        manager.counter_handle(1).unwrap(),
    ));

    std::thread::scope(|scope| {
        let local_reader = Arc::clone(&local);
        let buffer_reader = Arc::clone(&buffer);
        let status_reader = Arc::clone(&status);
        let reader = scope.spawn(move || {
            while local_reader.get_acquire() != 71
                || buffer_reader.get_acquire() != 72
                || status_reader.get_acquire() != 73
            {
                std::hint::spin_loop();
            }
        });

        local.set_release(71);
        buffer.set_release(72);
        status.set_release(73);
        reader.join().unwrap();
    });
}
