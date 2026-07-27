//! Agent atomic-publication and cooperative-stop tests.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use agrona::agent::{Agent, AgentResult, AgentRunner, BusySpinIdleStrategy};

struct ProgressAgent {
    calls: Arc<AtomicUsize>,
}

impl Agent for ProgressAgent {
    fn role_name(&self) -> &str {
        "stop-publication"
    }

    fn do_work(&mut self) -> AgentResult {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(0)
    }
}

#[test]
fn release_published_stop_is_observed_by_busy_worker() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handle = AgentRunner::new(
        ProgressAgent {
            calls: Arc::clone(&calls),
        },
        BusySpinIdleStrategy,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        None,
    )
    .start()
    .unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline, "worker did not make progress");
        std::thread::yield_now();
    }

    handle.request_stop();
    assert!(!handle.is_running());
    let agent = handle.join().unwrap();
    assert!(agent.calls.load(Ordering::Relaxed) > 0);
}
