// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral and publication tests for `CachedNanoClock`.

use std::sync::{Arc, Barrier};
use std::thread;

use agrona::clock::{CachedNanoClock, NanoClock};

#[test]
fn update_and_advance_use_wrapping_arithmetic() {
    let mut clock = CachedNanoClock::with_initial_time(i64::MAX);
    let reader = clock.reader();

    assert_eq!(i64::MAX, clock.nano_time());
    assert_eq!(i64::MIN, clock.advance(1));
    assert_eq!(i64::MIN, reader.nano_time());
    clock.update(777);
    assert_eq!(777, reader.nano_time());
}

#[test]
fn publishes_to_multiple_readers() {
    let mut writer = CachedNanoClock::new();
    let final_reader = writer.reader();
    let barrier = Arc::new(Barrier::new(4));
    let mut readers = Vec::new();

    for _ in 0..3 {
        let reader = writer.reader();
        let barrier = Arc::clone(&barrier);
        readers.push(thread::spawn(move || {
            barrier.wait();
            let mut previous = i64::MIN;
            for _ in 0..100_000 {
                let current = reader.nano_time();
                assert!(current >= previous);
                previous = current;
            }
        }));
    }

    barrier.wait();
    for value in 1..=100_000 {
        writer.update(value);
    }

    for reader in readers {
        reader.join().expect("reader should not panic");
    }
    assert_eq!(100_000, final_reader.nano_time());
}
