// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Steady-state allocation acceptance for `CLK-ALLOC-001`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::time::Duration;

use agrona::clock::{
    CachedEpochClock, CachedNanoClock, EpochClock, EpochMicroClock, EpochNanoClock, NanoClock,
    OffsetEpochNanoClock, OffsetEpochNanoClockConfig, SystemEpochClock, SystemEpochMicroClock,
    SystemEpochNanoClock, SystemNanoClock,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every operation delegates to `System` with the original layout and
// pointer. The counter is observational and does not alter allocation
// semantics.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the caller-provided valid layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the caller-provided valid layout.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: delegated with the pointer and layout supplied by the
        // original caller.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the caller-provided pointer, layout, and new
        // size.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Debug)]
struct FixedEpochClock(i64);

impl EpochClock for FixedEpochClock {
    fn time(&self) -> i64 {
        self.0
    }
}

#[derive(Debug)]
struct IncrementingNanoClock(AtomicI64);

impl NanoClock for IncrementingNanoClock {
    fn nano_time(&self) -> i64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

fn allocation_delta(operation: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    operation();
    ALLOCATIONS.load(Ordering::SeqCst) - before
}

#[test]
fn steady_state_clock_paths_do_not_allocate() {
    let system_epoch = SystemEpochClock;
    let system_micro = SystemEpochMicroClock;
    let system_epoch_nano = SystemEpochNanoClock;
    let system_nano = SystemNanoClock;
    black_box(system_epoch.time());
    black_box(system_micro.micro_time());
    black_box(system_epoch_nano.nano_time());
    black_box(system_nano.nano_time());

    assert_eq!(
        0,
        allocation_delta(|| {
            for _ in 0..1_000 {
                black_box(system_epoch.time());
                black_box(system_micro.micro_time());
                black_box(system_epoch_nano.nano_time());
                black_box(system_nano.nano_time());
            }
        })
    );

    let mut cached_epoch = CachedEpochClock::new();
    let epoch_reader = cached_epoch.reader();
    let mut cached_nano = CachedNanoClock::new();
    let nano_reader = cached_nano.reader();
    black_box(epoch_reader.time());
    black_box(nano_reader.nano_time());

    assert_eq!(
        0,
        allocation_delta(|| {
            for value in 0..1_000 {
                cached_epoch.update(value);
                black_box(cached_epoch.advance(1));
                black_box(epoch_reader.time());
                cached_nano.update(value);
                black_box(cached_nano.advance(1));
                black_box(nano_reader.nano_time());
            }
        })
    );

    let offset = OffsetEpochNanoClock::with_sources(
        FixedEpochClock(1_000),
        IncrementingNanoClock(AtomicI64::new(0)),
        OffsetEpochNanoClockConfig::new(1, Duration::from_nanos(10), Duration::from_secs(60))
            .expect("configuration should be valid"),
    )
    .expect("initial sample should succeed");
    black_box(offset.nano_time());
    black_box(offset.is_within_threshold());

    assert_eq!(
        0,
        allocation_delta(|| {
            for _ in 0..1_000 {
                black_box(offset.nano_time());
                black_box(offset.is_within_threshold());
            }
        })
    );
}
