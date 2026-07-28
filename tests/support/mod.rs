// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use std::cell::UnsafeCell;
use std::slice;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

pub struct AlignedBuffer {
    words: Vec<u64>,
    length: usize,
}

impl AlignedBuffer {
    pub fn new(length: usize) -> Self {
        Self {
            words: vec![0; length.div_ceil(size_of::<u64>())],
            length,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: `words` is initialized and stable for the returned borrow.
        // `length` never exceeds its byte capacity.
        unsafe { slice::from_raw_parts(self.words.as_ptr().cast::<u8>(), self.length) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: the unique borrow prevents aliases, `words` is initialized,
        // and `length` never exceeds its byte capacity.
        unsafe { slice::from_raw_parts_mut(self.words.as_mut_ptr().cast::<u8>(), self.length) }
    }
}

pub fn put_i32(buffer: &mut [u8], offset: usize, value: i32) {
    buffer[offset..offset + size_of::<i32>()].copy_from_slice(&value.to_ne_bytes());
}

pub fn put_i64(buffer: &mut [u8], offset: usize, value: i64) {
    buffer[offset..offset + size_of::<i64>()].copy_from_slice(&value.to_ne_bytes());
}

pub struct SharedAlignedBuffer {
    words: Box<[UnsafeCell<u64>]>,
    length: usize,
}

// SAFETY: mutation helpers use atomics for integral fields. `write_bytes`
// models a single external producer and its caller must publish with release
// ordering before a reader accesses those bytes.
unsafe impl Sync for SharedAlignedBuffer {}

impl SharedAlignedBuffer {
    pub fn new(length: usize) -> Self {
        let words = (0..length.div_ceil(size_of::<u64>()))
            .map(|_| UnsafeCell::new(0))
            .collect();
        Self { words, length }
    }

    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: the boxed storage is initialized and address-stable. This
        // fixture deliberately models a region that an external producer can
        // update behind a read-only mapping.
        unsafe {
            slice::from_raw_parts(
                self.words.as_ptr().cast::<UnsafeCell<u64>>().cast::<u8>(),
                self.length,
            )
        }
    }

    pub fn store_i32(&self, offset: usize, value: i32, ordering: Ordering) {
        assert!(offset + size_of::<i32>() <= self.length);
        let pointer = self
            .words
            .as_ptr()
            .cast::<UnsafeCell<u64>>()
            .cast::<u8>()
            .wrapping_add(offset)
            .cast_mut()
            .cast::<i32>();
        assert_eq!(0, pointer.addr() % align_of::<AtomicI32>());

        // SAFETY: the fixture owns initialized, aligned storage and all
        // concurrent integral access at this location is atomic.
        unsafe { AtomicI32::from_ptr(pointer).store(value, ordering) };
    }

    pub fn store_i64(&self, offset: usize, value: i64, ordering: Ordering) {
        assert!(offset + size_of::<i64>() <= self.length);
        let pointer = self
            .words
            .as_ptr()
            .cast::<UnsafeCell<u64>>()
            .cast::<u8>()
            .wrapping_add(offset)
            .cast_mut()
            .cast::<i64>();
        assert_eq!(0, pointer.addr() % align_of::<AtomicI64>());

        // SAFETY: the fixture owns initialized, aligned storage and all
        // concurrent integral access at this location is atomic.
        unsafe { AtomicI64::from_ptr(pointer).store(value, ordering) };
    }

    pub fn write_bytes(&self, offset: usize, bytes: &[u8]) {
        assert!(offset + bytes.len() <= self.length);
        let pointer = self
            .words
            .as_ptr()
            .cast::<UnsafeCell<u64>>()
            .cast::<u8>()
            .wrapping_add(offset)
            .cast_mut();

        // SAFETY: each record is written exactly once by the fixture's single
        // producer before release publication. The reader waits for that
        // publication and the producer never writes the record again.
        unsafe { pointer.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len()) };
    }
}
