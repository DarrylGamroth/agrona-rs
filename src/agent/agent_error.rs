// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use super::{AgentTermination, BoxError};

/// A recoverable Agent duty-cycle failure or explicit termination.
#[derive(Debug)]
pub enum AgentError {
    /// A recoverable failure after which the duty cycle can continue.
    Failed(BoxError),
    /// A request to terminate the Agent.
    Terminated(AgentTermination),
}

impl AgentError {
    /// Boxes a concrete recoverable error.
    pub fn failed(error: impl Error + Send + Sync + 'static) -> Self {
        Self::Failed(Box::new(error))
    }
}

impl From<AgentTermination> for AgentError {
    fn from(termination: AgentTermination) -> Self {
        Self::Terminated(termination)
    }
}

impl Display for AgentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(error) => Display::fmt(error, formatter),
            Self::Terminated(termination) => Display::fmt(termination, formatter),
        }
    }
}

impl Error for AgentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Failed(error) => Some(error.as_ref()),
            Self::Terminated(termination) => Some(termination),
        }
    }
}
