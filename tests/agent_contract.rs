//! Agent trait contract tests.
use agrona::agent::{Agent, AgentResult, IdleStrategy};

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

struct MinimalIdle {
    steps: usize,
}

impl IdleStrategy for MinimalIdle {
    fn idle_once(&mut self) {
        self.steps += 1;
    }
}

#[test]
fn idle_strategy_defaults_select_idle_and_expose_empty_alias() {
    let mut idle = MinimalIdle { steps: 0 };
    idle.idle(0);
    idle.idle(-1);
    idle.idle(1);
    idle.reset();
    assert_eq!(2, idle.steps);
    assert_eq!("", idle.alias());
}
