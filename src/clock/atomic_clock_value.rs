// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from Agrona and substantially modified for Rust.

use std::sync::atomic::{AtomicI64, Ordering};

#[repr(align(128))]
#[derive(Debug)]
pub(super) struct AtomicClockValue(AtomicI64);

impl AtomicClockValue {
    pub(super) fn new(value: i64) -> Self {
        Self(AtomicI64::new(value))
    }

    #[inline]
    pub(super) fn load(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }

    #[inline]
    pub(super) fn store(&self, value: i64) {
        self.0.store(value, Ordering::Release);
    }

    #[inline]
    pub(super) fn advance(&self, delta: i64) -> i64 {
        let value = self.0.load(Ordering::Relaxed).wrapping_add(delta);
        self.0.store(value, Ordering::Release);
        value
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn value_occupies_its_alignment() {
        assert_eq!(128, align_of::<AtomicClockValue>());
        assert_eq!(128, size_of::<AtomicClockValue>());
    }
}
