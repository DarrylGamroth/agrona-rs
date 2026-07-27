// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::IdleStrategy;

/// Performs no idle action.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoOpIdleStrategy;

impl IdleStrategy for NoOpIdleStrategy {
    fn idle(&mut self, _work_count: i32) {}
    fn idle_once(&mut self) {}
    fn alias(&self) -> &'static str {
        "noop"
    }
}
