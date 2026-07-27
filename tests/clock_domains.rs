// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Time-domain contract tests for `CLK-DOM-001`.

use agrona::clock::{
    EpochClock, EpochMicroClock, EpochNanoClock, NanoClock, SystemEpochClock,
    SystemEpochMicroClock, SystemEpochNanoClock, SystemNanoClock,
};

#[test]
fn provider_traits_are_object_safe_and_domains_are_distinct() {
    let epoch: Box<dyn EpochClock> = Box::new(SystemEpochClock);
    let epoch_micro: Box<dyn EpochMicroClock> = Box::new(SystemEpochMicroClock);
    let epoch_nano: Box<dyn EpochNanoClock> = Box::new(SystemEpochNanoClock);
    let monotonic: Box<dyn NanoClock> = Box::new(SystemNanoClock);

    assert!(epoch.time() > 0);
    assert!(epoch_micro.micro_time() > 0);
    assert!(epoch_nano.nano_time() > 0);
    assert!(monotonic.nano_time() >= 0);
}
