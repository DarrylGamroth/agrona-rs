//! AgentInvoker lifecycle and error tests.

use agrona::agent::{
    Agent, AgentError, AgentErrorCounter, AgentInvoker, AgentResult, AgentTermination,
};
use std::error::Error;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

struct Scripted {
    starts: usize,
    closes: usize,
    work: Vec<AgentResult>,
}
impl Agent for Scripted {
    fn role_name(&self) -> &str {
        "scripted"
    }
    fn on_start(&mut self) -> Result<(), agrona::agent::BoxError> {
        self.starts += 1;
        Ok(())
    }
    fn do_work(&mut self) -> AgentResult {
        self.work.remove(0)
    }
    fn on_close(&mut self) -> Result<(), agrona::agent::BoxError> {
        self.closes += 1;
        Ok(())
    }
}

#[test]
fn ordinary_errors_count_and_continue_then_expected_termination_closes() {
    let seen = Arc::new(Mutex::new(0usize));
    let observer = Arc::clone(&seen);
    let counter = AgentErrorCounter::default();
    let agent = Scripted {
        starts: 0,
        closes: 0,
        work: vec![
            Err(AgentError::Failed(Box::new(io::Error::other("work")))),
            Ok(3),
            Err(AgentTermination::expected().into()),
        ],
    };
    let mut invoker = AgentInvoker::new(
        agent,
        move |_: &(dyn Error + Send + Sync + 'static)| *observer.lock().unwrap() += 1,
        Some(counter.clone()),
    );
    invoker.start();
    assert_eq!(0, invoker.invoke());
    assert_eq!(3, invoker.invoke());
    assert_eq!(0, invoker.invoke());
    assert_eq!(1, counter.count());
    assert_eq!(1, *seen.lock().unwrap());
    assert!(invoker.is_closed());
    assert_eq!(1, invoker.agent().starts);
    assert_eq!(1, invoker.agent().closes);
}

#[test]
fn unexpected_termination_reports_without_counting() {
    let seen = Arc::new(Mutex::new(0usize));
    let observer = Arc::clone(&seen);
    let counter = AgentErrorCounter::default();
    let agent = Scripted {
        starts: 0,
        closes: 0,
        work: vec![Err(AgentTermination::unexpected().into())],
    };
    let mut invoker = AgentInvoker::new(
        agent,
        move |_: &(dyn Error + Send + Sync + 'static)| *observer.lock().unwrap() += 1,
        Some(counter.clone()),
    );
    invoker.start();
    assert_eq!(0, invoker.invoke());
    assert_eq!(0, counter.count());
    assert_eq!(1, *seen.lock().unwrap());
}

struct LifecycleFailure {
    fail_start: bool,
    fail_close: bool,
    starts: usize,
    closes: usize,
}

impl Agent for LifecycleFailure {
    fn role_name(&self) -> &str {
        "lifecycle-failure"
    }

    fn on_start(&mut self) -> Result<(), agrona::agent::BoxError> {
        self.starts += 1;
        if self.fail_start {
            Err(Box::new(io::Error::other("start")))
        } else {
            Ok(())
        }
    }

    fn do_work(&mut self) -> AgentResult {
        Ok(0)
    }

    fn on_close(&mut self) -> Result<(), agrona::agent::BoxError> {
        self.closes += 1;
        if self.fail_close {
            Err(Box::new(io::Error::other("close")))
        } else {
            Ok(())
        }
    }
}

#[test]
fn startup_failure_closes_once_and_lifecycle_errors_are_not_counted() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let observer = Arc::clone(&reports);
    let counter = AgentErrorCounter::default();
    let mut invoker = AgentInvoker::new(
        LifecycleFailure {
            fail_start: true,
            fail_close: true,
            starts: 0,
            closes: 0,
        },
        move |error: &(dyn Error + Send + Sync + 'static)| {
            observer.lock().unwrap().push(error.to_string());
        },
        Some(counter.clone()),
    );

    invoker.start();
    invoker.start();
    invoker.close();
    assert!(invoker.is_started());
    assert!(!invoker.is_running());
    assert!(invoker.is_closed());
    assert_eq!(1, invoker.agent().starts);
    assert_eq!(1, invoker.agent().closes);
    assert_eq!(&["start", "close"], reports.lock().unwrap().as_slice());
    assert_eq!(0, counter.count());
}

#[test]
fn error_handler_panic_is_not_converted_to_an_agent_error() {
    let mut invoker = AgentInvoker::new(
        Scripted {
            starts: 0,
            closes: 0,
            work: vec![Err(AgentError::failed(io::Error::other("work")))],
        },
        |_: &(dyn Error + Send + Sync + 'static)| panic!("handler panic"),
        None,
    );
    invoker.start();

    let panic = catch_unwind(AssertUnwindSafe(|| invoker.invoke()));
    assert!(panic.is_err());
    assert!(invoker.is_running());
    invoker.close();
    assert_eq!(1, invoker.agent().closes);
}
