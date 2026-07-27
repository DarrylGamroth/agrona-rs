//! YieldingIdleStrategy tests.
use agrona::agent::{IdleStrategy, YieldingIdleStrategy};
#[test]
fn alias_and_work_counts() {
    let mut s = YieldingIdleStrategy;
    assert_eq!("yield", s.alias());
    s.idle(1);
    s.idle(0);
    s.idle(-1);
}
