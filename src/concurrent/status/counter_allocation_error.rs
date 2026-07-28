// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{Display, Formatter};

use super::CountersManagerError;

/// Counter allocation failure, including a caller key-initializer failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterAllocationError<E> {
    /// The manager could not select or initialize a record.
    Manager(CountersManagerError),
    /// The caller-provided key initializer failed.
    KeyInitializer(E),
}

impl<E: Display> Display for CounterAllocationError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manager(error) => Display::fmt(error, formatter),
            Self::KeyInitializer(error) => {
                write!(formatter, "counter key initializer failed: {error}")
            }
        }
    }
}

impl<E> Error for CounterAllocationError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manager(error) => Some(error),
            Self::KeyInitializer(error) => Some(error),
        }
    }
}

impl<E> From<CountersManagerError> for CounterAllocationError<E> {
    fn from(value: CountersManagerError) -> Self {
        Self::Manager(value)
    }
}
