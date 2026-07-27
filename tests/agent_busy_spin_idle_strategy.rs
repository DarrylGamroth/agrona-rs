//! BusySpinIdleStrategy tests.
use agrona::agent::{BusySpinIdleStrategy, IdleStrategy};
#[test]
fn alias_and_work_counts() {
    let mut s = BusySpinIdleStrategy;
    assert_eq!("spin", s.alias());
    s.idle(1);
    s.idle(0);
    s.idle(-1);
}
