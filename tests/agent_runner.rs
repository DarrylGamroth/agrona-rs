//! AgentRunner lifecycle and ownership tests.

use agrona::agent::{
    Agent, AgentError, AgentErrorCounter, AgentResult, AgentRunner, AgentTermination, BoxError,
    NoOpIdleStrategy,
};
use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct RunsOnce {
    thread_name: Arc<Mutex<Option<String>>>,
    closed: Arc<Mutex<bool>>,
}
impl Agent for RunsOnce {
    fn role_name(&self) -> &str {
        "runner-role"
    }
    fn do_work(&mut self) -> AgentResult {
        *self.thread_name.lock().unwrap() = std::thread::current().name().map(str::to_owned);
        Err(AgentTermination::expected().into())
    }
    fn on_close(&mut self) -> Result<(), agrona::agent::BoxError> {
        *self.closed.lock().unwrap() = true;
        Ok(())
    }
}

#[test]
fn runs_on_named_thread_and_returns_agent_after_cleanup() {
    let name = Arc::new(Mutex::new(None));
    let closed = Arc::new(Mutex::new(false));
    let runner = AgentRunner::new(
        RunsOnce {
            thread_name: Arc::clone(&name),
            closed: Arc::clone(&closed),
        },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        Some(AgentErrorCounter::default()),
    );
    let agent = runner.start().unwrap().join().unwrap();
    assert_eq!(Some("runner-role".to_owned()), *name.lock().unwrap());
    assert!(*closed.lock().unwrap());
    assert_eq!("runner-role", agent.role_name());
}

#[test]
fn close_before_start_calls_cleanup_without_spawning() {
    let closed = Arc::new(Mutex::new(false));
    let runner = AgentRunner::new(
        RunsOnce {
            thread_name: Arc::new(Mutex::new(None)),
            closed: Arc::clone(&closed),
        },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    );
    let _ = runner.close();
    assert!(*closed.lock().unwrap());
}

struct ErrorThenStop {
    calls: usize,
}

impl Agent for ErrorThenStop {
    fn role_name(&self) -> &str {
        "error-then-stop"
    }

    fn do_work(&mut self) -> AgentResult {
        self.calls += 1;
        if self.calls == 1 {
            Err(AgentError::failed(io::Error::other("work")))
        } else {
            Err(AgentTermination::expected().into())
        }
    }
}

#[test]
fn ordinary_error_is_counted_reported_and_does_not_stop_the_loop() {
    let reports = Arc::new(AtomicUsize::new(0));
    let observer = Arc::clone(&reports);
    let counter = AgentErrorCounter::default();
    let runner = AgentRunner::new(
        ErrorThenStop { calls: 0 },
        NoOpIdleStrategy,
        move |_: &(dyn Error + Send + Sync + 'static)| {
            observer.fetch_add(1, Ordering::Relaxed);
        },
        Some(counter.clone()),
    );

    let agent = runner.start().unwrap().join().unwrap();
    assert_eq!(2, agent.calls);
    assert_eq!(1, counter.count());
    assert_eq!(1, reports.load(Ordering::Relaxed));
}

struct StartFails {
    closes: usize,
}

impl Agent for StartFails {
    fn role_name(&self) -> &str {
        "start-fails"
    }

    fn on_start(&mut self) -> Result<(), BoxError> {
        Err(Box::new(io::Error::other("start")))
    }

    fn do_work(&mut self) -> AgentResult {
        panic!("work must not run after startup failure")
    }

    fn on_close(&mut self) -> Result<(), BoxError> {
        self.closes += 1;
        Ok(())
    }
}

#[test]
fn startup_failure_is_reported_without_counting_and_cleanup_runs() {
    let reports = Arc::new(AtomicUsize::new(0));
    let observer = Arc::clone(&reports);
    let counter = AgentErrorCounter::default();
    let agent = AgentRunner::new(
        StartFails { closes: 0 },
        NoOpIdleStrategy,
        move |_: &(dyn Error + Send + Sync + 'static)| {
            observer.fetch_add(1, Ordering::Relaxed);
        },
        Some(counter.clone()),
    )
    .start()
    .unwrap()
    .join()
    .unwrap();

    assert_eq!(1, agent.closes);
    assert_eq!(0, counter.count());
    assert_eq!(1, reports.load(Ordering::Relaxed));
}

struct Panics {
    closes: usize,
}

impl Agent for Panics {
    fn role_name(&self) -> &str {
        "panics"
    }

    fn do_work(&mut self) -> AgentResult {
        panic!("fatal work panic")
    }

    fn on_close(&mut self) -> Result<(), BoxError> {
        self.closes += 1;
        Ok(())
    }
}

#[test]
fn panic_remains_fatal_but_join_retains_agent_and_cleanup_result() {
    let result = AgentRunner::new(
        Panics { closes: 0 },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    )
    .start()
    .unwrap()
    .join();
    let error = match result {
        Ok(_) => panic!("work panic must produce a structured join error"),
        Err(error) => error,
    };

    let (agent, panic, close_panic) = error.into_parts();
    assert_eq!(1, agent.closes);
    assert_eq!(
        Some(&"fatal work panic"),
        panic.downcast_ref::<&'static str>()
    );
    assert!(close_panic.is_none());
}

struct BlocksUntilReleased {
    entered: Arc<AtomicBool>,
    released: Arc<AtomicBool>,
    closes: usize,
}

impl Agent for BlocksUntilReleased {
    fn role_name(&self) -> &str {
        "blocked"
    }

    fn do_work(&mut self) -> AgentResult {
        self.entered.store(true, Ordering::Release);
        while !self.released.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        Ok(0)
    }

    fn on_close(&mut self) -> Result<(), BoxError> {
        self.closes += 1;
        Ok(())
    }
}

#[test]
fn close_retry_callback_can_diagnose_and_release_blocking_agent_code() {
    let entered = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));
    let handle = AgentRunner::new(
        BlocksUntilReleased {
            entered: Arc::clone(&entered),
            released: Arc::clone(&released),
            closes: 0,
        },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    )
    .start()
    .unwrap();

    while !entered.load(Ordering::Acquire) {
        std::thread::yield_now();
    }
    let stalls = AtomicUsize::new(0);
    let agent = handle
        .close_with_retry(Duration::from_millis(1), |_| {
            stalls.fetch_add(1, Ordering::Relaxed);
            released.store(true, Ordering::Release);
        })
        .unwrap();

    assert!(stalls.load(Ordering::Relaxed) >= 1);
    assert_eq!(1, agent.closes);
}

#[test]
fn spawn_failure_returns_the_complete_unstarted_runner() {
    let runner = AgentRunner::new(
        RunsOnce {
            thread_name: Arc::new(Mutex::new(None)),
            closed: Arc::new(Mutex::new(false)),
        },
        NoOpIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    );
    let error = match runner.start_with_builder(std::thread::Builder::new().stack_size(usize::MAX))
    {
        Ok(_) => panic!("an impossibly large stack must fail to spawn"),
        Err(error) => error,
    };
    let (_source, runner) = error.into_parts();
    let agent = runner.close();
    assert!(*agent.closed.lock().unwrap());
}
