// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

/// An OS-thread startup failure that retains the unstarted runner.
#[derive(Debug)]
pub struct AgentRunnerStartError<R> {
    pub(crate) error: io::Error,
    pub(crate) runner: R,
}

impl<R> AgentRunnerStartError<R> {
    /// Returns the OS error.
    #[must_use]
    pub const fn error(&self) -> &io::Error {
        &self.error
    }

    /// Recovers both the OS error and unstarted runner.
    #[must_use]
    pub fn into_parts(self) -> (io::Error, R) {
        (self.error, self.runner)
    }
}

impl<R> Display for AgentRunnerStartError<R> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "failed to start Agent runner: {}", self.error)
    }
}

impl<R: fmt::Debug> Error for AgentRunnerStartError<R> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}
