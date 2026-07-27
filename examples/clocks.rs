// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates system, cached, monotonic, and offset clock usage.

use agrona::clock::{
    CachedEpochClock, EpochClock, EpochNanoClock, NanoClock, OffsetEpochNanoClock,
    SystemEpochClock, SystemNanoClock,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let epoch_ms = SystemEpochClock.time();
    let start_ns = SystemNanoClock.nano_time();

    let mut cached_clock = CachedEpochClock::with_initial_time(epoch_ms);
    let cached_reader = cached_clock.reader();
    cached_clock.advance(1);

    let offset_clock = OffsetEpochNanoClock::new()?;

    println!("system epoch milliseconds: {epoch_ms}");
    println!("cached epoch milliseconds: {}", cached_reader.time());
    println!("offset epoch nanoseconds: {}", offset_clock.nano_time());
    println!(
        "elapsed monotonic nanoseconds: {}",
        SystemNanoClock.nano_time().wrapping_sub(start_ns)
    );

    Ok(())
}
