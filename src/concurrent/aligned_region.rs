// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Checked atomic access to a borrowed byte region.
//!
//! This module is the only library location that converts counter-buffer byte
//! addresses into atomic references. Its invariants are:
//!
//! - the source slice keeps every byte initialized, at a stable address, for
//!   the complete lifetime of this view;
//! - construction verifies an eight-byte-aligned non-empty base, which is
//!   sufficient for every counter field;
//! - each load first obtains a bounds-checked subslice and verifies the
//!   operand's natural alignment;
//! - the immutable slice prevents safe Rust code from creating a mutable
//!   alias; any same-process producer must use compatible atomic accesses for
//!   integral fields and must not mix them with racy non-atomic accesses;
//! - callers do not mutate key or label bytes while a returned borrowed view
//!   is live; and
//! - an external storage or mapping owner keeps the region valid and
//!   coordinates its producer lifetime.

use std::mem::align_of;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

use super::status::CountersReaderError;

pub(super) const REGION_ALIGNMENT: usize = align_of::<AtomicI64>();

#[derive(Clone, Copy, Debug)]
pub(super) struct AlignedRegion<'a> {
    bytes: &'a [u8],
}

impl<'a> AlignedRegion<'a> {
    pub(super) fn new(region: &'static str, bytes: &'a [u8]) -> Result<Self, CountersReaderError> {
        let address = bytes.as_ptr() as usize;
        if !bytes.is_empty() && address % REGION_ALIGNMENT != 0 {
            return Err(CountersReaderError::MisalignedRegion {
                region,
                address,
                required_alignment: REGION_ALIGNMENT,
            });
        }

        Ok(Self { bytes })
    }

    #[inline]
    pub(super) fn load_i32(&self, offset: usize, ordering: Ordering) -> i32 {
        let field = self
            .bytes
            .get(offset..offset + size_of::<i32>())
            .expect("validated counter metadata offset");
        let pointer = field.as_ptr().cast_mut().cast::<i32>();
        assert_eq!(
            0,
            pointer.addr() % align_of::<AtomicI32>(),
            "counter metadata field is not naturally aligned"
        );

        // SAFETY: construction and the checks above establish lifetime,
        // initialization, bounds, and natural alignment. The module-level
        // invariants require compatible atomic concurrent access.
        unsafe { AtomicI32::from_ptr(pointer).load(ordering) }
    }

    #[inline]
    pub(super) fn load_i64(&self, offset: usize, ordering: Ordering) -> i64 {
        let field = self
            .bytes
            .get(offset..offset + size_of::<i64>())
            .expect("validated counter values or metadata offset");
        let pointer = field.as_ptr().cast_mut().cast::<i64>();
        assert_eq!(
            0,
            pointer.addr() % align_of::<AtomicI64>(),
            "counter field is not naturally aligned"
        );

        // SAFETY: construction and the checks above establish lifetime,
        // initialization, bounds, and natural alignment. The module-level
        // invariants require compatible atomic concurrent access.
        unsafe { AtomicI64::from_ptr(pointer).load(ordering) }
    }

    #[inline]
    pub(super) fn bytes(&self, offset: usize, length: usize) -> &'a [u8] {
        self.bytes
            .get(offset..offset + length)
            .expect("validated counter byte range")
    }

    pub(super) fn as_slice(&self) -> &'a [u8] {
        self.bytes
    }
}
