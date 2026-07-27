//! Agent error, termination, and diagnostic-counter tests.

use std::error::Error;
use std::io;

use agrona::agent::{AgentError, AgentErrorCounter, AgentTermination};

#[test]
fn recoverable_and_termination_errors_retain_display_and_source() {
    let failed = AgentError::failed(io::Error::other("work failed"));
    assert_eq!("work failed", failed.to_string());
    assert_eq!("work failed", failed.source().unwrap().to_string());

    let expected = AgentError::from(AgentTermination::expected().with_message("complete"));
    assert_eq!("expected Agent termination: complete", expected.to_string());
    assert_eq!(
        "expected Agent termination: complete",
        expected.source().unwrap().to_string()
    );

    let unexpected = AgentTermination::unexpected();
    assert!(!unexpected.is_expected());
    assert_eq!("unexpected Agent termination", unexpected.to_string());
}

#[test]
fn diagnostic_counter_honors_initial_value_and_wraps() {
    let counter = AgentErrorCounter::with_initial_value(i64::MAX);
    assert_eq!(i64::MAX, counter.count());

    struct Fails;
    impl agrona::agent::Agent for Fails {
        fn role_name(&self) -> &str {
            "fails"
        }

        fn do_work(&mut self) -> agrona::agent::AgentResult {
            Err(AgentError::failed(io::Error::other("failure")))
        }
    }

    let mut invoker = agrona::agent::AgentInvoker::new(
        Fails,
        |_: &(dyn Error + Send + Sync + 'static)| {},
        Some(counter.clone()),
    );
    invoker.start();
    invoker.invoke();
    assert_eq!(i64::MIN, counter.count());
}
