//! SleepingMillisIdleStrategy tests.
use agrona::agent::{IdleStrategy, SleepingMillisIdleStrategy};
use std::time::Duration;
#[test]
fn alias_default_and_zero_duration() {
    assert_eq!(
        Duration::from_millis(1),
        SleepingMillisIdleStrategy::default().duration()
    );
    let mut s = SleepingMillisIdleStrategy::new(Duration::ZERO);
    assert_eq!("sleep-ms", s.alias());
    s.idle(1);
    s.idle(0);
}
