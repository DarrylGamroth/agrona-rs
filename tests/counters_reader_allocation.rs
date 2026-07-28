// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Steady-state allocation acceptance for `CTR-ALLOC-001`.

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use agrona::concurrent::status::CountersReader;
use support::{AlignedBuffer, put_i32, put_i64};

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
fn repeated_read_and_scan_paths_allocate_zero_bytes() {
    let mut metadata = AlignedBuffer::new(CountersReader::METADATA_LENGTH);
    let mut values = AlignedBuffer::new(CountersReader::COUNTER_LENGTH);
    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::STATE_OFFSET,
        CountersReader::RECORD_ALLOCATED,
    );
    put_i32(metadata.as_bytes_mut(), CountersReader::TYPE_ID_OFFSET, 7);
    put_i64(
        metadata.as_bytes_mut(),
        CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
        19,
    );
    put_i32(
        metadata.as_bytes_mut(),
        CountersReader::LABEL_LENGTH_OFFSET,
        5,
    );
    metadata.as_bytes_mut()
        [CountersReader::LABEL_VALUE_OFFSET..CountersReader::LABEL_VALUE_OFFSET + 5]
        .copy_from_slice(b"label");
    put_i64(
        values.as_bytes_mut(),
        CountersReader::COUNTER_VALUE_OFFSET,
        11,
    );
    put_i64(
        values.as_bytes_mut(),
        CountersReader::REGISTRATION_ID_OFFSET,
        13,
    );
    put_i64(values.as_bytes_mut(), CountersReader::OWNER_ID_OFFSET, 17);
    put_i64(
        values.as_bytes_mut(),
        CountersReader::REFERENCE_ID_OFFSET,
        23,
    );
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    assert_eq!(
        0,
        allocation_delta(|| {
            for _ in 0..1_000 {
                black_box(reader.counter_value(0).unwrap());
                black_box(reader.counter_registration_id(0).unwrap());
                black_box(reader.counter_owner_id(0).unwrap());
                black_box(reader.counter_reference_id(0).unwrap());
                black_box(reader.counter_state(0).unwrap());
                black_box(reader.counter_type_id(0).unwrap());
                black_box(reader.free_for_reuse_deadline(0).unwrap());
                black_box(reader.counter_key(0).unwrap());
                black_box(reader.counter_label(0).unwrap());
                reader
                    .for_each_counter(|value, id, label| {
                        black_box((value, id, label));
                    })
                    .unwrap();
                reader
                    .for_each_metadata(|id, type_id, key, label| {
                        black_box((id, type_id, key, label));
                    })
                    .unwrap();
                black_box(reader.find_by_registration_id(13).unwrap());
                black_box(reader.find_by_type_id_and_registration_id(7, 13).unwrap());
            }
        })
    );
}
