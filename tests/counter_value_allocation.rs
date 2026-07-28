// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Steady-state allocation acceptance for counter value APIs.

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use agrona::concurrent::status::{
    AtomicCounter, AtomicLongPosition, CountersReader, Position, ReadablePosition, StatusIndicator,
    StatusIndicatorReader, UnsafeBufferPosition, UnsafeBufferStatusIndicator,
};
use support::AlignedBuffer;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every operation delegates to `System` with the original arguments.
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
        // SAFETY: delegated with the original pointer and layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the original pointer, layout, and new size.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocation_delta(operation: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    operation();
    ALLOCATIONS.load(Ordering::SeqCst) - before
}

#[test]
fn repeated_counter_position_and_status_paths_allocate_zero_bytes() {
    let mut counter_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut position_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let mut status_values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    let counter = AtomicCounter::new(counter_values.as_bytes_mut(), 0).unwrap();
    let buffer_position = UnsafeBufferPosition::new(position_values.as_bytes_mut(), 0).unwrap();
    let status = UnsafeBufferStatusIndicator::new(status_values.as_bytes_mut(), 0).unwrap();
    let local_position = AtomicLongPosition::new();

    assert_eq!(
        0,
        allocation_delta(|| {
            for value in 0..1_000 {
                black_box(counter.get_and_add(1));
                counter.set_release(value);
                black_box(counter.get_acquire());
                black_box(counter.compare_and_set(value, value + 1));
                black_box(counter.propose_max_opaque(value + 2));

                buffer_position.set_release(value);
                black_box(buffer_position.get_acquire());
                black_box(buffer_position.propose_max_opaque(value + 1));

                local_position.set_volatile(value);
                black_box(local_position.get_volatile());
                black_box(local_position.propose_max_release(value + 1));

                status.set_release(value);
                black_box(status.get_acquire());
            }
        })
    );
}
