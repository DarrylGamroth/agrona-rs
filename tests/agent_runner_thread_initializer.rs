//! AgentRunner worker initialization tests.

use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};

use agrona::agent::{
    Agent, AgentResult, AgentRunner, AgentTermination, BoxError, NoOpIdleStrategy,
};

#[derive(Debug)]
struct RecordsLifecycle {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Agent for RecordsLifecycle {
    fn role_name(&self) -> &str {
        "initialized-runner"
    }

    fn on_start(&mut self) -> Result<(), BoxError> {
        self.events.lock().unwrap().push("on_start");
        Ok(())
    }

    fn do_work(&mut self) -> AgentResult {
        self.events.lock().unwrap().push("do_work");
        Err(AgentTermination::expected().into())
    }

    fn on_close(&mut self) -> Result<(), BoxError> {
        self.events.lock().unwrap().push("on_close");
        Ok(())
    }
}

#[test]
fn initializer_runs_on_named_worker_before_agent_start() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let initializer_events = Arc::clone(&events);
    let runner = AgentRunner::new(
        RecordsLifecycle {
            events: Arc::clone(&events),
        },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    );

    let agent = runner
        .start_with_builder_and_thread_initializer(
            std::thread::Builder::new().stack_size(2 * 1024 * 1024),
            move || {
                assert_eq!(Some("initialized-runner"), std::thread::current().name());
                initializer_events.lock().unwrap().push("initializer");
                Ok(())
            },
        )
        .unwrap()
        .join()
        .unwrap();

    assert_eq!("initialized-runner", agent.role_name());
    assert_eq!(
        &["initializer", "on_start", "do_work", "on_close"],
        events.lock().unwrap().as_slice()
    );
}

#[test]
fn initializer_failure_is_reported_and_prevents_agent_start_and_work() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let reports = Arc::new(Mutex::new(Vec::new()));
    let report_observer = Arc::clone(&reports);
    let runner = AgentRunner::new(
        RecordsLifecycle {
            events: Arc::clone(&events),
        },
        NoOpIdleStrategy,
        move |error: &(dyn Error + Send + Sync + 'static)| {
            report_observer.lock().unwrap().push(error.to_string());
        },
        None,
    );

    let _agent = runner
        .start_with_thread_initializer(|| {
            Err(Box::new(io::Error::other("worker initialization failed")) as BoxError)
        })
        .unwrap()
        .join()
        .unwrap();

    assert_eq!(&["on_close"], events.lock().unwrap().as_slice());
    assert_eq!(
        &["worker initialization failed"],
        reports.lock().unwrap().as_slice()
    );
}

#[test]
fn initializer_panic_is_fatal_and_cleanup_still_runs() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let result = AgentRunner::new(
        RecordsLifecycle {
            events: Arc::clone(&events),
        },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    )
    .start_with_thread_initializer(|| -> Result<(), BoxError> {
        panic!("initializer panic");
    })
    .unwrap()
    .join();

    let error = result.unwrap_err();
    let (_agent, panic, close_panic) = error.into_parts();
    assert_eq!(
        Some(&"initializer panic"),
        panic.downcast_ref::<&'static str>()
    );
    assert!(close_panic.is_none());
    assert_eq!(&["on_close"], events.lock().unwrap().as_slice());
}
