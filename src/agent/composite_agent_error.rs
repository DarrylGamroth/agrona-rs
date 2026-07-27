// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::BoxError;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Ordered recoverable lifecycle failures from a static composite.
#[derive(Debug)]
pub struct CompositeAgentError {
    operation: &'static str,
    errors: Vec<BoxError>,
}

impl CompositeAgentError {
    pub(crate) fn new(operation: &'static str, errors: Vec<BoxError>) -> Self {
        Self { operation, errors }
    }
    /// Returns the failed lifecycle operation.
    #[must_use]
    pub const fn operation(&self) -> &'static str {
        self.operation
    }
    /// Returns failures in sub-agent encounter order.
    #[must_use]
    pub fn errors(&self) -> &[BoxError] {
        &self.errors
    }
}
impl Display for CompositeAgentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompositeAgent {} failed for {} Agent(s)",
            self.operation,
            self.errors.len()
        )
    }
}
impl Error for CompositeAgentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.errors.first().map(|e| e.as_ref() as _)
    }
}
