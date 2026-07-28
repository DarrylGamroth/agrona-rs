// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Atomic counter operation and concurrency tests.

mod support;

use std::sync::Arc;

use agrona::concurrent::status::{AtomicCounter, CountersManager, CountersReader};
use support::AlignedBuffer;

fn require_send_sync<T: Send + Sync>() {}

#[test]
fn counter_handles_are_send_and_sync() {
    require_send_sync::<AtomicCounter<'static>>();
}

#[test]
fn implements_every_atomic_and_single_writer_operation() {
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let counter = AtomicCounter::new(values.as_bytes_mut(), 0).unwrap();

    assert_eq!(0, counter.id());
    assert_eq!(0, counter.increment());
    assert_eq!(1, counter.increment_ordered());
    assert_eq!(2, counter.increment_opaque());
    assert_eq!(3, counter.increment_plain());
    assert_eq!(4, counter.decrement());
    assert_eq!(3, counter.decrement_ordered());
    assert_eq!(2, counter.decrement_opaque());
    assert_eq!(1, counter.decrement_plain());
    assert_eq!(0, counter.get());

    counter.set(10);
    assert_eq!(10, counter.get_and_add(5));
    assert_eq!(15, counter.get_and_add_ordered(2));
    assert_eq!(17, counter.get_and_add_opaque(2));
    assert_eq!(19, counter.get_and_add_plain(2));
    assert_eq!(21, counter.get_and_set(30));
    assert!(counter.compare_and_set(30, 31));
    assert!(!counter.compare_and_set(30, 32));
    assert_eq!(31, counter.get_acquire());

    counter.set_ordered(40);
    counter.set_opaque(41);
    counter.set_weak(42);
    counter.set_plain(43);
    assert_eq!(43, counter.get_opaque());
    assert_eq!(43, counter.get_weak());
    assert_eq!(43, counter.get_plain());

    assert!(!counter.propose_max(42));
    assert!(counter.propose_max(44));
    assert!(counter.propose_max_ordered(45));
    assert!(counter.propose_max_opaque(46));
    assert!(!counter.propose_max_release(46));

    counter.close();
    counter.close();
    assert!(counter.is_closed());
}

#[test]
fn arithmetic_wraps_like_java_long() {
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let counter = AtomicCounter::new(values.as_bytes_mut(), 0).unwrap();
    counter.set(i64::MAX);
    assert_eq!(i64::MAX, counter.increment());
    assert_eq!(i64::MIN, counter.get());
    assert_eq!(i64::MIN, counter.decrement_plain());
    assert_eq!(i64::MAX, counter.get());
}

#[test]
fn multi_writer_rmw_does_not_lose_updates() {
    let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let counter = Arc::new(manager.new_counter(b"shared").unwrap());
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for _ in 0..4 {
            let counter = Arc::clone(&counter);
            threads.push(scope.spawn(move || {
                for _ in 0..25_000 {
                    counter.increment();
                }
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
    });
    assert_eq!(100_000, counter.get());
}

#[test]
fn release_store_is_observed_by_acquire_load() {
    let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let counter = Arc::new(manager.new_counter(b"published").unwrap());
    let reader = Arc::clone(&counter);
    std::thread::scope(|scope| {
        let thread = scope.spawn(move || {
            while reader.get_acquire() != 77 {
                std::hint::spin_loop();
            }
        });
        counter.set_release(77);
        thread.join().unwrap();
    });
}

#[test]
fn close_is_local_and_stale_handles_make_explicit_free_visible() {
    let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut manager = CountersManager::new(metadata.as_bytes_mut(), values.as_bytes_mut()).unwrap();
    let counter = manager.new_counter(b"first").unwrap();
    counter.close();
    assert_eq!(
        CountersReader::RECORD_ALLOCATED,
        manager.reader().counter_state(counter.id()).unwrap()
    );

    manager.free(counter.id()).unwrap();
    let replacement = manager.new_counter_raw(9, Some(b"key"), b"second").unwrap();
    assert_eq!(counter.id(), replacement.id());
    replacement.set(55);
    assert_eq!(55, counter.get());
}
