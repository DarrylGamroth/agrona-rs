// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::thread;

use super::{
    Agent, AgentError, AgentErrorCounter, AgentRunnerHandle, AgentRunnerStartError, ErrorHandler,
    IdleStrategy,
};

/// An unstarted dedicated-thread Agent owner.
pub struct AgentRunner<A, I, H> {
    pub(crate) agent: A,
    pub(crate) idle_strategy: I,
    pub(crate) error_handler: H,
    pub(crate) error_counter: Option<AgentErrorCounter>,
}

impl<A, I, H> std::fmt::Debug for AgentRunner<A, I, H> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentRunner")
            .finish_non_exhaustive()
    }
}

impl<A: Agent, I: IdleStrategy, H: ErrorHandler> AgentRunner<A, I, H> {
    /// Creates an unstarted runner.
    #[must_use]
    pub fn new(
        agent: A,
        idle_strategy: I,
        error_handler: H,
        error_counter: Option<AgentErrorCounter>,
    ) -> Self {
        Self {
            agent,
            idle_strategy,
            error_handler,
            error_counter,
        }
    }

    /// Starts the runner while preserving ownership on OS spawn failure.
    pub fn start(
        self,
    ) -> Result<AgentRunnerHandle<A>, AgentRunnerStartError<AgentRunner<A, I, H>>> {
        self.start_with_builder(thread::Builder::new())
    }

    /// Starts the runner with a caller-configured thread builder.
    ///
    /// The Agent role always becomes the thread name. Stack-size and other
    /// builder configuration is retained.
    pub fn start_with_builder(
        self,
        builder: thread::Builder,
    ) -> Result<AgentRunnerHandle<A>, AgentRunnerStartError<AgentRunner<A, I, H>>> {
        let role_name = self.agent.role_name().to_owned();
        let running = Arc::new(AtomicBool::new(true));
        let closed = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = sync_channel::<Self>(0);
        let worker_running = Arc::clone(&running);
        let worker_closed = Arc::clone(&closed);

        let join_handle = match builder.name(role_name).spawn(move || {
            let runner = receiver
                .recv()
                .expect("Agent runner bootstrap sender dropped unexpectedly");
            run(runner, &worker_running, &worker_closed)
        }) {
            Ok(handle) => handle,
            Err(error) => {
                return Err(AgentRunnerStartError {
                    error,
                    runner: self,
                });
            }
        };

        if let Err(send_error) = sender.send(self) {
            let error = io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Agent worker exited during startup",
            );
            let _ = join_handle.join();
            return Err(AgentRunnerStartError {
                error,
                runner: send_error.0,
            });
        }

        let worker_thread = join_handle.thread().clone();
        Ok(AgentRunnerHandle {
            running,
            closed,
            worker_thread,
            join_handle: Some(join_handle),
        })
    }

    /// Closes without spawning and returns the Agent.
    pub fn close(mut self) -> A {
        if let Err(error) = self.agent.on_close() {
            self.error_handler.on_error(error.as_ref());
        }
        self.agent
    }
}

pub(crate) struct RunnerOutcome<A> {
    pub(crate) agent: A,
    pub(crate) primary_panic: Option<Box<dyn std::any::Any + Send>>,
    pub(crate) close_panic: Option<Box<dyn std::any::Any + Send>>,
}

fn run<A: Agent, I: IdleStrategy, H: ErrorHandler>(
    runner: AgentRunner<A, I, H>,
    running: &AtomicBool,
    closed: &AtomicBool,
) -> RunnerOutcome<A> {
    let AgentRunner {
        mut agent,
        mut idle_strategy,
        mut error_handler,
        error_counter,
    } = runner;

    let primary_panic = catch_unwind(AssertUnwindSafe(|| {
        if let Err(error) = agent.on_start() {
            running.store(false, Ordering::Release);
            error_handler.on_error(error.as_ref());
        }

        while running.load(Ordering::Acquire) {
            match agent.do_work() {
                Ok(work_count) => idle_strategy.idle(work_count),
                Err(AgentError::Failed(error)) => {
                    if running.load(Ordering::Acquire) {
                        if let Some(counter) = &error_counter {
                            counter.increment();
                        }
                    }
                    error_handler.on_error(error.as_ref());
                }
                Err(AgentError::Terminated(termination)) => {
                    running.store(false, Ordering::Release);
                    if !termination.is_expected() {
                        error_handler.on_error(&termination);
                    }
                }
            }
        }
    }))
    .err();

    running.store(false, Ordering::Release);
    let close_panic = catch_unwind(AssertUnwindSafe(|| {
        if let Err(error) = agent.on_close() {
            error_handler.on_error(error.as_ref());
        }
    }))
    .err();
    closed.store(true, Ordering::Release);

    RunnerOutcome {
        agent,
        primary_panic,
        close_panic,
    }
}
