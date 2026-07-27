//! SleepingIdleStrategy tests.
use agrona::agent::{IdleStrategy, SleepingIdleStrategy};
use std::time::Duration;
#[test]
fn alias_default_and_zero_duration() {
    assert_eq!(
        Duration::from_nanos(1_000),
        SleepingIdleStrategy::default().duration()
    );
    let mut s = SleepingIdleStrategy::new(Duration::ZERO);
    assert_eq!("sleep-ns", s.alias());
    s.idle(1);
    s.idle(0);
}
