// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{Agent, AgentError, AgentErrorCounter, ErrorHandler};

/// A caller-driven, non-thread-safe owner of one Agent.
pub struct AgentInvoker<A, H> {
    agent: A,
    error_handler: H,
    error_counter: Option<AgentErrorCounter>,
    started: bool,
    running: bool,
    closed: bool,
}

impl<A: Agent, H: ErrorHandler> AgentInvoker<A, H> {
    /// Creates an invoker.
    #[must_use]
    pub fn new(agent: A, error_handler: H, error_counter: Option<AgentErrorCounter>) -> Self {
        Self {
            agent,
            error_handler,
            error_counter,
            started: false,
            running: false,
            closed: false,
        }
    }

    /// Attempts startup at most once.
    pub fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        match self.agent.on_start() {
            Ok(()) => self.running = true,
            Err(error) => {
                self.error_handler.on_error(error.as_ref());
                self.close();
            }
        }
    }

    /// Invokes one duty cycle, returning zero when no invocation completes.
    pub fn invoke(&mut self) -> i32 {
        if !self.running {
            return 0;
        }
        match self.agent.do_work() {
            Ok(work_count) => work_count,
            Err(AgentError::Failed(error)) => {
                if self.running {
                    if let Some(counter) = &self.error_counter {
                        counter.increment();
                    }
                }
                self.error_handler.on_error(error.as_ref());
                0
            }
            Err(AgentError::Terminated(termination)) => {
                self.running = false;
                if !termination.is_expected() {
                    self.error_handler.on_error(&termination);
                }
                self.close();
                0
            }
        }
    }

    /// Attempts cleanup at most once.
    pub fn close(&mut self) {
        if self.closed {
            return;
        }
        self.running = false;
        self.closed = true;
        if let Err(error) = self.agent.on_close() {
            self.error_handler.on_error(error.as_ref());
        }
    }

    /// Returns whether startup has been attempted.
    #[must_use]
    pub const fn is_started(&self) -> bool {
        self.started
    }

    /// Returns whether duty-cycle invocation is enabled.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// Returns whether cleanup has been attempted.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Borrows the owned Agent.
    #[must_use]
    pub const fn agent(&self) -> &A {
        &self.agent
    }

    /// Mutably borrows the owned Agent.
    pub fn agent_mut(&mut self) -> &mut A {
        &mut self.agent
    }

    /// Consumes the invoker and returns its Agent.
    pub fn into_agent(self) -> A {
        self.agent
    }
}
