// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral tests for `SystemEpochNanoClock`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agrona::clock::{EpochNanoClock, SystemEpochNanoClock};

#[test]
fn tracks_system_epoch_nanoseconds() {
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should follow the Unix epoch");
    let value = SystemEpochNanoClock.nano_time();
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should follow the Unix epoch");
    let tolerance = Duration::from_secs(1);

    assert!(value >= before.saturating_sub(tolerance).as_nanos() as i64);
    assert!(value <= after.saturating_add(tolerance).as_nanos() as i64);
}
