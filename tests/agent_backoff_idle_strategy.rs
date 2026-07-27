//! BackoffIdleStrategy tests.
use agrona::agent::{BackoffIdleStrategy, IdleStrategy};

#[test]
fn defaults_alias_and_reset_are_usable() {
    let mut strategy = BackoffIdleStrategy::default();
    assert_eq!("backoff", strategy.alias());
    strategy.idle(0);
    strategy.idle(1);
}
