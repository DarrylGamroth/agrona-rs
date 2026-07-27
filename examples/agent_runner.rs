// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runs a bounded counter Agent to expected termination on a dedicated thread.

use std::error::Error;

use agrona::agent::{
    Agent, AgentErrorCounter, AgentResult, AgentRunner, AgentTermination, BackoffIdleStrategy,
};

#[derive(Debug)]
struct CounterAgent {
    remaining: u64,
}

impl Agent for CounterAgent {
    fn role_name(&self) -> &str {
        "counter"
    }

    fn on_start(&mut self) -> Result<(), agrona::agent::BoxError> {
        println!("{} started", self.role_name());
        Ok(())
    }

    fn do_work(&mut self) -> AgentResult {
        if self.remaining == 0 {
            return Err(AgentTermination::expected().into());
        }

        println!("remaining work: {}", self.remaining);
        self.remaining -= 1;
        Ok(1)
    }

    fn on_close(&mut self) -> Result<(), agrona::agent::BoxError> {
        println!("{} closed", self.role_name());
        Ok(())
    }
}

fn report_error(error: &(dyn Error + Send + Sync + 'static)) {
    eprintln!("counter Agent error: {error}");
}

fn main() -> Result<(), Box<dyn Error>> {
    let errors = AgentErrorCounter::default();
    let runner = AgentRunner::new(
        CounterAgent { remaining: 3 },
        BackoffIdleStrategy::default(),
        report_error,
        Some(errors.clone()),
    );

    let handle = runner.start()?;
    println!(
        "running {} on thread {:?}",
        handle.is_running(),
        handle.worker_thread().name()
    );

    let agent = handle.join()?;
    println!("completed with {} recoverable errors", errors.count());
    assert_eq!(0, agent.remaining);

    Ok(())
}
