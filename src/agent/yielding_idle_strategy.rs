// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::IdleStrategy;

/// Yields the OS thread when no work was done.
#[derive(Clone, Copy, Debug, Default)]
pub struct YieldingIdleStrategy;

impl IdleStrategy for YieldingIdleStrategy {
    fn idle_once(&mut self) {
        std::thread::yield_now();
    }
    fn alias(&self) -> &'static str {
        "yield"
    }
}
