// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

/// Fatal runner panics returned with the owned Agent.
pub struct AgentRunnerJoinError<A> {
    pub(crate) agent: A,
    pub(crate) primary_panic: Box<dyn Any + Send + 'static>,
    pub(crate) close_panic: Option<Box<dyn Any + Send + 'static>>,
}

impl<A> AgentRunnerJoinError<A> {
    /// Recovers the Agent and panic payloads.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        A,
        Box<dyn Any + Send + 'static>,
        Option<Box<dyn Any + Send + 'static>>,
    ) {
        (self.agent, self.primary_panic, self.close_panic)
    }
}

impl<A> Debug for AgentRunnerJoinError<A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentRunnerJoinError")
            .field("close_also_panicked", &self.close_panic.is_some())
            .finish_non_exhaustive()
    }
}

impl<A> Display for AgentRunnerJoinError<A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.close_panic.is_some() {
            formatter.write_str("Agent runner and cleanup panicked")
        } else {
            formatter.write_str("Agent runner panicked")
        }
    }
}

impl<A> Error for AgentRunnerJoinError<A> {}
