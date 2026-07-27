// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Error returned when constructing an empty static composite.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptyCompositeAgentError;

impl Display for EmptyCompositeAgentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("CompositeAgent requires at least one Agent")
    }
}
impl Error for EmptyCompositeAgentError {}
