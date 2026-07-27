// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Steady-state allocation acceptance for the Agent family.

use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use agrona::agent::{
    Agent, AgentInvoker, AgentResult, AgentRunner, AgentTermination, BackoffIdleStrategy,
    BusySpinIdleStrategy, CompositeAgent, ControllableIdleStrategy, IdleStrategy, NoOpIdleStrategy,
    SleepingIdleStrategy, SleepingMillisIdleStrategy, YieldingIdleStrategy,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every operation delegates to `System` with the original pointer and
// layout. The counter is observational and does not alter allocation
// semantics.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the caller-provided valid layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the caller-provided valid layout.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: delegated with the pointer and layout supplied by the
        // original caller.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegated with the caller-provided pointer, layout, and new
        // size.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

struct WorkAgent {
    value: i32,
}

impl Agent for WorkAgent {
    fn role_name(&self) -> &str {
        "allocation-work"
    }

    fn do_work(&mut self) -> AgentResult {
        Ok(self.value)
    }
}

struct RunnerAllocationAgent {
    calls: usize,
    baseline: usize,
    allocated_in_loop: bool,
}

impl Agent for RunnerAllocationAgent {
    fn role_name(&self) -> &str {
        "allocation-runner"
    }

    fn do_work(&mut self) -> AgentResult {
        self.calls += 1;
        if self.calls == 1 {
            self.baseline = ALLOCATIONS.load(Ordering::SeqCst);
        } else if self.calls <= 1_001 {
            self.allocated_in_loop |= ALLOCATIONS.load(Ordering::SeqCst) != self.baseline;
        } else {
            return Err(AgentTermination::expected().into());
        }
        Ok(1)
    }
}

fn ignore_error(_: &(dyn Error + Send + Sync + 'static)) {}

fn allocation_delta(operation: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    operation();
    ALLOCATIONS.load(Ordering::SeqCst) - before
}

#[test]
fn successful_agent_paths_allocate_nothing_after_construction() {
    let mut invoker = AgentInvoker::new(WorkAgent { value: 1 }, ignore_error, None);
    invoker.start();
    assert_eq!(
        0,
        allocation_delta(|| {
            for _ in 0..1_000 {
                black_box(invoker.invoke());
            }
        })
    );

    let mut composite = CompositeAgent::new(vec![
        Box::new(WorkAgent { value: 1 }),
        Box::new(WorkAgent { value: 2 }),
    ])
    .unwrap();
    assert_eq!(
        0,
        allocation_delta(|| {
            for _ in 0..1_000 {
                black_box(composite.do_work().unwrap());
            }
        })
    );

    let mut backoff = BackoffIdleStrategy::new(0, 0, Duration::ZERO, Duration::ZERO);
    let mut spin = BusySpinIdleStrategy;
    let mut noop = NoOpIdleStrategy;
    let mut sleeping = SleepingIdleStrategy::new(Duration::ZERO);
    let mut sleeping_ms = SleepingMillisIdleStrategy::new(Duration::ZERO);
    let mut yielding = YieldingIdleStrategy;
    let (mut controllable, control) = ControllableIdleStrategy::new();
    control.set_raw(1);
    for _ in 0..4 {
        backoff.idle(0);
    }
    sleeping.idle(0);
    sleeping_ms.idle(0);
    assert_eq!(
        0,
        allocation_delta(|| {
            for _ in 0..1_000 {
                backoff.idle(0);
                spin.idle(0);
                noop.idle(0);
                sleeping.idle(0);
                sleeping_ms.idle(0);
                yielding.idle(0);
                controllable.idle(0);
            }
        })
    );

    let agent = AgentRunner::new(
        RunnerAllocationAgent {
            calls: 0,
            baseline: 0,
            allocated_in_loop: false,
        },
        NoOpIdleStrategy,
        ignore_error,
        None,
    )
    .start()
    .unwrap()
    .join()
    .unwrap();
    assert_eq!(1_002, agent.calls);
    assert!(!agent.allocated_in_loop);
}
