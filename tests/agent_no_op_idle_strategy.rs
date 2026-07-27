//! NoOpIdleStrategy tests.
use agrona::agent::{IdleStrategy, NoOpIdleStrategy};
#[test]
fn alias_and_work_counts() {
    let mut s = NoOpIdleStrategy;
    assert_eq!("noop", s.alias());
    s.idle(1);
    s.idle(0);
    s.idle(-1);
}
