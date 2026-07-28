// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Checked atomic access to borrowed byte regions.
//!
//! This is the only library module that converts counter-buffer addresses to
//! atomic references. Its invariants are:
//!
//! - backing storage is initialized, address-stable, and valid for the view's
//!   complete lifetime;
//! - construction validates eight-byte base alignment;
//! - every field access checks bounds and natural alignment;
//! - integral locations that can be observed concurrently are accessed only
//!   through compatible atomics;
//! - mutable metadata bytes remain behind a unique manager borrow;
//! - key and label bytes are not mutated while a borrowed byte view is live;
//! - a mapping owner, when added, retains the mapping and coordinates external
//!   producer lifetime.

use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::align_of;
use std::ptr::NonNull;
use std::slice;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

use super::status::CountersReaderError;

pub(super) const REGION_ALIGNMENT: usize = align_of::<AtomicI64>();

#[derive(Clone, Copy, Debug)]
pub(super) struct AlignedRegion<'a> {
    pointer: NonNull<u8>,
    length: usize,
    lifetime: PhantomData<&'a [u8]>,
}

// SAFETY: the view exposes only atomic integral access and immutable byte
// views. Its documented invariants prohibit conflicting non-atomic mutation.
unsafe impl Send for AlignedRegion<'_> {}
// SAFETY: concurrent integral access is atomic and byte views cannot be
// mutated through this type.
unsafe impl Sync for AlignedRegion<'_> {}

impl<'a> AlignedRegion<'a> {
    pub(super) fn new(region: &'static str, bytes: &'a [u8]) -> Result<Self, CountersReaderError> {
        Self::from_pointer(region, bytes.as_ptr().cast_mut(), bytes.len())
    }

    fn from_pointer(
        region: &'static str,
        pointer: *mut u8,
        length: usize,
    ) -> Result<Self, CountersReaderError> {
        let address = pointer as usize;
        if length != 0 && address % REGION_ALIGNMENT != 0 {
            return Err(CountersReaderError::MisalignedRegion {
                region,
                address,
                required_alignment: REGION_ALIGNMENT,
            });
        }

        Ok(Self {
            pointer: NonNull::new(pointer).unwrap_or_else(NonNull::dangling),
            length,
            lifetime: PhantomData,
        })
    }

    #[inline]
    pub(super) fn load_i32(&self, offset: usize, ordering: Ordering) -> i32 {
        self.atomic_i32(offset).load(ordering)
    }

    #[inline]
    pub(super) fn store_i32(&self, offset: usize, value: i32, ordering: Ordering) {
        self.atomic_i32(offset).store(value, ordering);
    }

    #[inline]
    pub(super) fn load_i64(&self, offset: usize, ordering: Ordering) -> i64 {
        self.atomic_i64(offset).load(ordering)
    }

    #[inline]
    pub(super) fn store_i64(&self, offset: usize, value: i64, ordering: Ordering) {
        self.atomic_i64(offset).store(value, ordering);
    }

    #[inline]
    pub(super) fn fetch_add_i64(&self, offset: usize, value: i64, ordering: Ordering) -> i64 {
        self.atomic_i64(offset).fetch_add(value, ordering)
    }

    #[inline]
    pub(super) fn swap_i64(&self, offset: usize, value: i64, ordering: Ordering) -> i64 {
        self.atomic_i64(offset).swap(value, ordering)
    }

    #[inline]
    pub(super) fn compare_exchange_i64(
        &self,
        offset: usize,
        expected: i64,
        update: i64,
        success: Ordering,
        failure: Ordering,
    ) -> Result<i64, i64> {
        self.atomic_i64(offset)
            .compare_exchange(expected, update, success, failure)
    }

    #[inline]
    pub(super) fn bytes(&self, offset: usize, length: usize) -> &[u8] {
        self.range(offset, length);

        // SAFETY: construction establishes validity and lifetime, and the
        // range check above establishes bounds. The returned borrow is tied to
        // `self`, preventing manager metadata mutation through safe Rust.
        unsafe { slice::from_raw_parts(self.pointer.as_ptr().add(offset), length) }
    }

    fn atomic_i32(&self, offset: usize) -> &AtomicI32 {
        self.range(offset, size_of::<i32>());
        let pointer = self.pointer.as_ptr().wrapping_add(offset).cast::<i32>();
        assert_eq!(
            0,
            pointer.addr() % align_of::<AtomicI32>(),
            "counter metadata field is not naturally aligned"
        );

        // SAFETY: the module invariants establish validity, initialization,
        // alignment, lifetime, and compatible atomic access.
        unsafe { AtomicI32::from_ptr(pointer) }
    }

    fn atomic_i64(&self, offset: usize) -> &AtomicI64 {
        self.range(offset, size_of::<i64>());
        let pointer = self.pointer.as_ptr().wrapping_add(offset).cast::<i64>();
        assert_eq!(
            0,
            pointer.addr() % align_of::<AtomicI64>(),
            "counter field is not naturally aligned"
        );

        // SAFETY: the module invariants establish validity, initialization,
        // alignment, lifetime, and compatible atomic access.
        unsafe { AtomicI64::from_ptr(pointer) }
    }

    fn range(&self, offset: usize, length: usize) {
        let end = offset
            .checked_add(length)
            .expect("counter byte range overflow");
        assert!(end <= self.length, "counter byte range is out of bounds");
    }
}

#[derive(Debug)]
pub(super) struct MutableAlignedRegion<'a> {
    pointer: NonNull<u8>,
    length: usize,
    lifetime: PhantomData<&'a mut [u8]>,
    not_sync: PhantomData<Cell<()>>,
}

// SAFETY: moving the unique manager-owned view to another thread preserves
// exclusive metadata mutation. The type is intentionally not `Sync`.
unsafe impl Send for MutableAlignedRegion<'_> {}

impl<'a> MutableAlignedRegion<'a> {
    pub(super) fn new(
        region: &'static str,
        bytes: &'a mut [u8],
    ) -> Result<Self, CountersReaderError> {
        let checked = AlignedRegion::from_pointer(region, bytes.as_mut_ptr(), bytes.len())?;
        Ok(Self {
            pointer: checked.pointer,
            length: checked.length,
            lifetime: PhantomData,
            not_sync: PhantomData,
        })
    }

    pub(super) fn view(&self) -> AlignedRegion<'_> {
        AlignedRegion {
            pointer: self.pointer,
            length: self.length,
            lifetime: PhantomData,
        }
    }

    pub(super) fn atomic_view(&self) -> AlignedRegion<'a> {
        AlignedRegion {
            pointer: self.pointer,
            length: self.length,
            lifetime: PhantomData,
        }
    }

    pub(super) fn bytes_mut(&mut self, offset: usize, length: usize) -> &mut [u8] {
        let end = offset
            .checked_add(length)
            .expect("counter byte range overflow");
        assert!(end <= self.length, "counter byte range is out of bounds");

        // SAFETY: construction establishes validity and lifetime, the range
        // check establishes bounds, and `&mut self` provides unique metadata
        // access for the returned borrow.
        unsafe { slice::from_raw_parts_mut(self.pointer.as_ptr().add(offset), length) }
    }
}
