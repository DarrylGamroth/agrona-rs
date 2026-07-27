// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

/// A cloneable process-local count of ordinary Agent work failures.
#[derive(Clone, Debug, Default)]
pub struct AgentErrorCounter {
    value: Arc<AtomicI64>,
}

impl AgentErrorCounter {
    /// Creates a counter with the supplied initial value.
    #[must_use]
    pub fn with_initial_value(value: i64) -> Self {
        Self {
            value: Arc::new(AtomicI64::new(value)),
        }
    }

    /// Returns the current diagnostic count.
    #[must_use]
    pub fn count(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    pub(crate) fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
}
