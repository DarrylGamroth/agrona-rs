// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use super::AgentError;

/// Result returned by an Agent duty cycle.
pub type AgentResult<T = i32> = Result<T, AgentError>;
