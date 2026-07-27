// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::IdleStrategy;

/// Spins when no work was done.
#[derive(Clone, Copy, Debug, Default)]
pub struct BusySpinIdleStrategy;

impl IdleStrategy for BusySpinIdleStrategy {
    #[inline]
    fn idle_once(&mut self) {
        std::hint::spin_loop();
    }
    fn alias(&self) -> &'static str {
        "spin"
    }
}
