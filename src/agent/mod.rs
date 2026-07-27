// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Synchronous, single-owner Agent duty cycles and idle strategies.

#[cfg(not(target_has_atomic = "64"))]
compile_error!("the Agent module requires native 64-bit atomics");

// Keeping one Java component per correspondingly named Rust source file makes
// upstream behavioral comparison substantially easier.
#[allow(clippy::module_inception)]
mod agent;
mod agent_error;
mod agent_error_counter;
mod agent_invoker;
mod agent_result;
mod agent_runner;
mod agent_runner_handle;
mod agent_runner_join_error;
mod agent_runner_start_error;
mod agent_termination;
mod backoff_idle_strategy;
mod box_error;
mod busy_spin_idle_strategy;
mod composite_agent;
mod composite_agent_error;
mod controllable_idle_strategy;
mod controllable_idle_strategy_control;
mod controllable_idle_strategy_mode;
mod empty_composite_agent_error;
mod error_handler;
mod idle_strategy;
mod no_op_idle_strategy;
mod sleeping_idle_strategy;
mod sleeping_millis_idle_strategy;
mod yielding_idle_strategy;

pub use agent::Agent;
pub use agent_error::AgentError;
pub use agent_error_counter::AgentErrorCounter;
pub use agent_invoker::AgentInvoker;
pub use agent_result::AgentResult;
pub use agent_runner::AgentRunner;
pub use agent_runner_handle::AgentRunnerHandle;
pub use agent_runner_join_error::AgentRunnerJoinError;
pub use agent_runner_start_error::AgentRunnerStartError;
pub use agent_termination::AgentTermination;
pub use backoff_idle_strategy::BackoffIdleStrategy;
pub use box_error::BoxError;
pub use busy_spin_idle_strategy::BusySpinIdleStrategy;
pub use composite_agent::CompositeAgent;
pub use composite_agent_error::CompositeAgentError;
pub use controllable_idle_strategy::ControllableIdleStrategy;
pub use controllable_idle_strategy_control::ControllableIdleStrategyControl;
pub use controllable_idle_strategy_mode::ControllableIdleStrategyMode;
pub use empty_composite_agent_error::EmptyCompositeAgentError;
pub use error_handler::ErrorHandler;
pub use idle_strategy::IdleStrategy;
pub use no_op_idle_strategy::NoOpIdleStrategy;
pub use sleeping_idle_strategy::SleepingIdleStrategy;
pub use sleeping_millis_idle_strategy::SleepingMillisIdleStrategy;
pub use yielding_idle_strategy::YieldingIdleStrategy;
