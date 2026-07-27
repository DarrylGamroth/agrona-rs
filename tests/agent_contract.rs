//! Agent trait contract tests.
use agrona::agent::{Agent, AgentResult};

struct Minimal;
impl Agent for Minimal {
    fn role_name(&self) -> &str {
        "minimal"
    }
    fn do_work(&mut self) -> AgentResult {
        Ok(7)
    }
}

#[test]
fn lifecycle_defaults_and_object_safety() {
    let mut agent: Box<dyn Agent> = Box::new(Minimal);
    assert_eq!("minimal", agent.role_name());
    assert!(agent.on_start().is_ok());
    assert_eq!(7, agent.do_work().unwrap());
    assert!(agent.on_close().is_ok());
}
