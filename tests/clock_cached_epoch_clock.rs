// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral tests for `CachedEpochClock`.

use agrona::clock::{CachedEpochClock, EpochClock};

#[test]
fn updates_advances_and_publishes_to_reader() {
    let mut clock = CachedEpochClock::new();
    let reader = clock.reader();

    assert_eq!(0, clock.time());
    assert_eq!(0, reader.time());
    clock.update(333);
    assert_eq!(333, reader.time());
    assert_eq!(340, clock.advance(7));
    assert_eq!(340, reader.time());
}
