// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{AgentResult, BoxError};

/// A synchronous duty cycle with serialized lifecycle callbacks.
pub trait Agent: Send + 'static {
    /// Returns the role used for diagnostics and runner thread naming.
    fn role_name(&self) -> &str;

    /// Performs startup on the owning thread.
    fn on_start(&mut self) -> Result<(), BoxError> {
        Ok(())
    }

    /// Performs one duty cycle and returns its signed work count.
    fn do_work(&mut self) -> AgentResult;

    /// Performs cleanup on the owning thread.
    fn on_close(&mut self) -> Result<(), BoxError> {
        Ok(())
    }
}
