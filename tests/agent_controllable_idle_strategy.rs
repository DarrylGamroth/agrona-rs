//! ControllableIdleStrategy tests.
use agrona::agent::{ControllableIdleStrategy, ControllableIdleStrategyMode, IdleStrategy};
#[test]
fn publishes_typed_and_unknown_modes() {
    let (mut strategy, control) = ControllableIdleStrategy::new();
    assert_eq!("controllable", strategy.alias());
    control.set(ControllableIdleStrategyMode::NoOp);
    assert_eq!(1, control.raw());
    strategy.idle(0);
    control.set_raw(99);
    strategy.idle(1);
}
