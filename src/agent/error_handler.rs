// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

/// Observes recoverable Agent failures.
pub trait ErrorHandler: Send + 'static {
    /// Handles one error. Implementations must not panic.
    fn on_error(&mut self, error: &(dyn Error + Send + Sync + 'static));
}

impl<F> ErrorHandler for F
where
    F: for<'a> FnMut(&'a (dyn Error + Send + Sync + 'static)) + Send + 'static,
{
    fn on_error(&mut self, error: &(dyn Error + Send + Sync + 'static)) {
        self(error);
    }
}
