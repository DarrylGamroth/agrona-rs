// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Closed-loop Agent microbenchmark for `DEC-AGENT-001`.

use std::error::Error;
use std::hint::black_box;
use std::time::Instant;

use agrona::agent::{
    Agent, AgentInvoker, AgentResult, AgentRunner, AgentTermination, BusySpinIdleStrategy,
    CompositeAgent, IdleStrategy, NoOpIdleStrategy,
};

const DEFAULT_ITERATIONS: u64 = 10_000_000;

struct WorkAgent;

impl Agent for WorkAgent {
    fn role_name(&self) -> &str {
        "benchmark-work"
    }

    fn do_work(&mut self) -> AgentResult {
        Ok(1)
    }
}

struct BoundedRunnerAgent {
    remaining: u64,
}

impl Agent for BoundedRunnerAgent {
    fn role_name(&self) -> &str {
        "benchmark-runner"
    }

    fn do_work(&mut self) -> AgentResult {
        if self.remaining == 0 {
            Err(AgentTermination::expected().into())
        } else {
            self.remaining -= 1;
            Ok(1)
        }
    }
}

fn ignore_error(_: &(dyn Error + Send + Sync + 'static)) {}

fn measure<T>(name: &str, iterations: u64, mut operation: impl FnMut() -> T) {
    for _ in 0..10_000 {
        black_box(operation());
    }

    let started = Instant::now();
    for _ in 0..iterations {
        black_box(operation());
    }
    report(name, iterations, started);
}

fn report(name: &str, iterations: u64, started: Instant) {
    let nanos_per_operation = started.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;
    println!("{name:28} {nanos_per_operation:10.3} ns/op");
}

fn main() {
    let iterations = std::env::var("AGRONA_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_ITERATIONS);

    println!("agrona-rs Agent benchmark");
    println!("Agrona baseline: d4a47c67258f85b39910c4999da346ead655b736");
    println!(
        "target: {}-{}; iterations: {iterations}",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!("closed-loop; no affinity or dedicated-core claim");

    let mut invoker = AgentInvoker::new(WorkAgent, ignore_error, None);
    invoker.start();
    measure("AgentInvoker::invoke", iterations, || {
        black_box(invoker.invoke());
    });

    let mut composite =
        CompositeAgent::new(vec![Box::new(WorkAgent), Box::new(WorkAgent)]).unwrap();
    measure("CompositeAgent::do_work/2", iterations, || {
        black_box(composite.do_work().unwrap());
    });

    let mut noop = NoOpIdleStrategy;
    measure("NoOpIdleStrategy::idle", iterations, || noop.idle(0));

    let mut spin = BusySpinIdleStrategy;
    measure("BusySpinIdleStrategy::idle", iterations, || spin.idle(0));

    let started = Instant::now();
    let agent = AgentRunner::new(
        BoundedRunnerAgent {
            remaining: iterations,
        },
        NoOpIdleStrategy,
        ignore_error,
        None,
    )
    .start()
    .unwrap()
    .join()
    .unwrap();
    black_box(agent);
    report("AgentRunner::do_work", iterations, started);
}
