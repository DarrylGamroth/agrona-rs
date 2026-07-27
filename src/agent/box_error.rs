// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

/// A thread-safe heterogeneous Agent error.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;
