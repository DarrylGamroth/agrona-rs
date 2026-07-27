// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// A request to stop an Agent, classified as expected or unexpected.
#[derive(Debug)]
pub struct AgentTermination {
    expected: bool,
    message: Option<String>,
}

impl AgentTermination {
    /// Creates an expected, quiet termination.
    #[must_use]
    pub const fn expected() -> Self {
        Self {
            expected: true,
            message: None,
        }
    }

    /// Creates an unexpected termination that is reported.
    #[must_use]
    pub const fn unexpected() -> Self {
        Self {
            expected: false,
            message: None,
        }
    }

    /// Attaches an explanatory message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Returns whether this is an expected termination.
    #[must_use]
    pub const fn is_expected(&self) -> bool {
        self.expected
    }
}

impl Display for AgentTermination {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let kind = if self.expected {
            "expected"
        } else {
            "unexpected"
        };
        match &self.message {
            Some(message) => write!(formatter, "{kind} Agent termination: {message}"),
            None => write!(formatter, "{kind} Agent termination"),
        }
    }
}

impl Error for AgentTermination {}
