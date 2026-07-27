// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, Thread};
use std::time::Duration;

use super::AgentRunnerJoinError;
use super::agent_runner::RunnerOutcome;

/// Control and join handle for a running Agent.
pub struct AgentRunnerHandle<A> {
    pub(crate) running: Arc<AtomicBool>,
    pub(crate) closed: Arc<AtomicBool>,
    pub(crate) worker_thread: Thread,
    pub(crate) join_handle: Option<JoinHandle<RunnerOutcome<A>>>,
}

impl<A> AgentRunnerHandle<A> {
    /// Cooperatively requests stop and unparks a parked worker.
    pub fn request_stop(&self) {
        self.running.store(false, Ordering::Release);
        self.worker_thread.unpark();
    }

    /// Returns whether the worker should continue running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Returns whether cleanup has completed.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Returns whether the worker thread has finished.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.join_handle
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
    }

    /// Returns the worker thread for diagnostics or application wakeups.
    #[must_use]
    pub const fn worker_thread(&self) -> &Thread {
        &self.worker_thread
    }

    /// Waits for natural completion without first requesting stop.
    pub fn join(mut self) -> Result<A, AgentRunnerJoinError<A>> {
        let outcome = self
            .join_handle
            .take()
            .expect("Agent runner joined more than once")
            .join()
            .expect("Agent runner bootstrap panicked outside its protected lifecycle");
        match outcome.primary_panic {
            Some(primary_panic) => Err(AgentRunnerJoinError {
                agent: outcome.agent,
                primary_panic,
                close_panic: outcome.close_panic,
            }),
            None => match outcome.close_panic {
                Some(primary_panic) => Err(AgentRunnerJoinError {
                    agent: outcome.agent,
                    primary_panic,
                    close_panic: None,
                }),
                None => Ok(outcome.agent),
            },
        }
    }

    /// Requests stop and waits for cleanup.
    pub fn close(self) -> Result<A, AgentRunnerJoinError<A>> {
        self.request_stop();
        self.join()
    }

    /// Requests stop and reports repeated stalls while waiting for cleanup.
    ///
    /// A zero retry interval waits without diagnostics. The callback can
    /// inspect the worker or activate an application-owned cancellation
    /// mechanism, but this method cannot forcibly cancel blocking Agent code.
    pub fn close_with_retry<F>(
        self,
        retry_interval: Duration,
        mut on_stall: F,
    ) -> Result<A, AgentRunnerJoinError<A>>
    where
        F: FnMut(&Thread),
    {
        self.request_stop();
        if !retry_interval.is_zero() {
            while !self.is_finished() {
                std::thread::sleep(retry_interval);
                if !self.is_finished() {
                    on_stall(&self.worker_thread);
                    self.worker_thread.unpark();
                }
            }
        }
        self.join()
    }
}
