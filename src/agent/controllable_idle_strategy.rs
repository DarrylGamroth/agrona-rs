// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{ControllableIdleStrategyControl, ControllableIdleStrategyMode, IdleStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

/// Selects its idle action from an atomically published mode.
#[derive(Debug)]
pub struct ControllableIdleStrategy {
    mode: Arc<AtomicI32>,
}

impl ControllableIdleStrategy {
    /// Java-compatible park duration.
    pub const PARK_DURATION: Duration = Duration::from_nanos(1_000);
    /// Creates a strategy and its controller.
    #[must_use]
    pub fn new() -> (Self, ControllableIdleStrategyControl) {
        let mode = Arc::new(AtomicI32::new(
            ControllableIdleStrategyMode::NotControlled as i32,
        ));
        (
            Self {
                mode: Arc::clone(&mode),
            },
            ControllableIdleStrategyControl { mode },
        )
    }
}

impl Default for ControllableIdleStrategy {
    fn default() -> Self {
        Self::new().0
    }
}

impl IdleStrategy for ControllableIdleStrategy {
    fn idle_once(&mut self) {
        match self.mode.load(Ordering::Acquire) {
            1 => {}
            2 => std::hint::spin_loop(),
            3 => std::thread::yield_now(),
            _ => std::thread::park_timeout(Self::PARK_DURATION),
        }
    }
    fn alias(&self) -> &'static str {
        "controllable"
    }
}
