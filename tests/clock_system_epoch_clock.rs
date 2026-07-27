// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral tests for `SystemEpochClock`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agrona::clock::{EpochClock, SystemEpochClock};

#[test]
fn tracks_system_epoch_milliseconds() {
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should follow the Unix epoch");
    let value = SystemEpochClock.time();
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should follow the Unix epoch");
    let tolerance = Duration::from_secs(1);

    assert!(value >= before.saturating_sub(tolerance).as_millis() as i64);
    assert!(value <= after.saturating_add(tolerance).as_millis() as i64);
}
