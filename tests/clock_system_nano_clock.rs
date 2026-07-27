// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Behavioral tests for `SystemNanoClock`.

use std::thread;
use std::time::Duration;

use agrona::clock::{NanoClock, SystemNanoClock};

#[test]
fn is_nondecreasing_and_measures_elapsed_time() {
    let clock = SystemNanoClock;
    let samples: Vec<_> = (0..1_000).map(|_| clock.nano_time()).collect();
    assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));

    let before = clock.nano_time();
    thread::sleep(Duration::from_millis(2));
    assert!(clock.nano_time() > before);
}
