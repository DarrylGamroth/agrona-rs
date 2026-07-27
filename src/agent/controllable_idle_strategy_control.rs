// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ControllableIdleStrategyMode;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

/// A cloneable publisher for controllable idle mode.
#[derive(Clone, Debug)]
pub struct ControllableIdleStrategyControl {
    pub(crate) mode: Arc<AtomicI32>,
}

impl ControllableIdleStrategyControl {
    /// Release-publishes a typed mode.
    pub fn set(&self, mode: ControllableIdleStrategyMode) {
        self.set_raw(mode as i32);
    }
    /// Release-publishes a raw mode; unknown values intentionally park.
    pub fn set_raw(&self, mode: i32) {
        self.mode.store(mode, Ordering::Release);
    }
    /// Acquire-loads the current raw mode.
    #[must_use]
    pub fn raw(&self) -> i32 {
        self.mode.load(Ordering::Acquire)
    }
}
