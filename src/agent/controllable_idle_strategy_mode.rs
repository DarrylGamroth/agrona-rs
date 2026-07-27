// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

/// Modes understood by `ControllableIdleStrategy`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ControllableIdleStrategyMode {
    /// Parks, matching Java's not-controlled default.
    NotControlled = 0,
    /// Does nothing.
    NoOp = 1,
    /// Spins.
    BusySpin = 2,
    /// Yields.
    Yield = 3,
    /// Parks.
    Park = 4,
}
