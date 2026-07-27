//! CompositeAgent lifecycle and cursor tests.

use agrona::agent::{
    Agent, AgentError, AgentResult, BoxError, CompositeAgent, CompositeAgentError,
};
use std::collections::VecDeque;
use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};

struct Part {
    name: &'static str,
    work: VecDeque<AgentResult>,
}
impl Agent for Part {
    fn role_name(&self) -> &str {
        self.name
    }
    fn do_work(&mut self) -> AgentResult {
        self.work.pop_front().unwrap_or(Ok(0))
    }
}

#[test]
fn rejects_empty_and_builds_exact_role() {
    let empty = match CompositeAgent::new(vec![]) {
        Ok(_) => panic!("empty composite must be rejected"),
        Err(error) => error,
    };
    assert_eq!(
        "CompositeAgent requires at least one Agent",
        empty.to_string()
    );
    let composite = CompositeAgent::new(vec![
        Box::new(Part {
            name: "one",
            work: VecDeque::new(),
        }),
        Box::new(Part {
            name: "two",
            work: VecDeque::new(),
        }),
    ])
    .unwrap();
    assert_eq!("[one,two]", composite.role_name());
}

#[test]
fn resumes_after_a_failing_sub_agent() {
    let mut composite = CompositeAgent::new(vec![
        Box::new(Part {
            name: "one",
            work: VecDeque::from([
                Err(AgentError::Failed(Box::new(io::Error::other("x")))),
                Ok(1),
            ]),
        }),
        Box::new(Part {
            name: "two",
            work: VecDeque::from([Ok(2), Ok(2)]),
        }),
    ])
    .unwrap();
    assert!(composite.do_work().is_err());
    assert_eq!(2, composite.do_work().unwrap());
    assert_eq!(3, composite.do_work().unwrap());
}

struct LifecyclePart {
    name: &'static str,
    events: Arc<Mutex<Vec<String>>>,
    fail_start: bool,
    fail_close: bool,
}

impl Agent for LifecyclePart {
    fn role_name(&self) -> &str {
        self.name
    }

    fn on_start(&mut self) -> Result<(), BoxError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("start-{}", self.name));
        if self.fail_start {
            Err(Box::new(io::Error::other(self.name)))
        } else {
            Ok(())
        }
    }

    fn do_work(&mut self) -> AgentResult {
        Ok(0)
    }

    fn on_close(&mut self) -> Result<(), BoxError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("close-{}", self.name));
        if self.fail_close {
            Err(Box::new(io::Error::other(self.name)))
        } else {
            Ok(())
        }
    }
}

#[test]
fn lifecycle_attempts_every_agent_and_aggregates_errors_in_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut composite = CompositeAgent::new(vec![
        Box::new(LifecyclePart {
            name: "one",
            events: Arc::clone(&events),
            fail_start: true,
            fail_close: false,
        }),
        Box::new(LifecyclePart {
            name: "two",
            events: Arc::clone(&events),
            fail_start: false,
            fail_close: true,
        }),
        Box::new(LifecyclePart {
            name: "three",
            events: Arc::clone(&events),
            fail_start: true,
            fail_close: true,
        }),
    ])
    .unwrap();

    let start = composite.on_start().unwrap_err();
    let start = start.downcast_ref::<CompositeAgentError>().unwrap();
    assert_eq!("on_start", start.operation());
    assert_eq!(["one", "three"], error_messages(start).as_slice());
    assert_eq!(
        "CompositeAgent on_start failed for 2 Agent(s)",
        start.to_string()
    );
    assert_eq!("one", start.source().unwrap().to_string());
    assert!(format!("{start:?}").contains("CompositeAgentError"));

    let close = composite.on_close().unwrap_err();
    let close = close.downcast_ref::<CompositeAgentError>().unwrap();
    assert_eq!("on_close", close.operation());
    assert_eq!(["two", "three"], error_messages(close).as_slice());
    assert_eq!(
        [
            "start-one",
            "start-two",
            "start-three",
            "close-one",
            "close-two",
            "close-three"
        ],
        events.lock().unwrap().as_slice()
    );
}

#[test]
fn work_count_addition_wraps_like_java_i32() {
    let mut composite = CompositeAgent::new(vec![
        Box::new(Part {
            name: "max",
            work: VecDeque::from([Ok(i32::MAX)]),
        }),
        Box::new(Part {
            name: "one",
            work: VecDeque::from([Ok(1)]),
        }),
    ])
    .unwrap();
    assert_eq!(i32::MIN, composite.do_work().unwrap());
}

fn error_messages(error: &CompositeAgentError) -> Vec<String> {
    error.errors().iter().map(ToString::to_string).collect()
}
