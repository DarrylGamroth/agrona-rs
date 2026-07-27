// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

/// Controls how an Agent owner waits when no work is available.
pub trait IdleStrategy: Send + 'static {
    /// Applies one idle decision for the previous work count.
    fn idle(&mut self, work_count: i32) {
        if work_count > 0 {
            self.reset();
        } else {
            self.idle_once();
        }
    }

    /// Performs one idle step.
    fn idle_once(&mut self);

    /// Resets state after useful work.
    fn reset(&mut self) {}

    /// Returns the stable Agrona strategy alias.
    fn alias(&self) -> &'static str {
        ""
    }
}
