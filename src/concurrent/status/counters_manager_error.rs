// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{Display, Formatter};

use super::CountersReaderError;

/// Failure to construct or operate an Agrona-compatible counter manager.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CountersManagerError {
    /// Region construction or counter-ID validation failed.
    Reader(CountersReaderError),
    /// No unused or reusable counter record is available.
    Full {
        /// Greatest counter ID supported by the values region.
        max_counter_id: i32,
    },
    /// The selected counter record is not allocated.
    CounterNotAllocated {
        /// Rejected counter ID.
        counter_id: i32,
        /// Observed record state.
        state: i32,
    },
    /// A replacement key exceeds the fixed key field.
    KeyTooLong {
        /// Supplied key length.
        length: usize,
        /// Maximum accepted key length.
        maximum_length: usize,
    },
}

impl Display for CountersManagerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reader(error) => Display::fmt(error, formatter),
            Self::Full { max_counter_id } => write!(
                formatter,
                "unable to allocate counter: buffer is full at id {max_counter_id}"
            ),
            Self::CounterNotAllocated { counter_id, state } => {
                write!(
                    formatter,
                    "counter {counter_id} is not allocated: state={state}"
                )
            }
            Self::KeyTooLong {
                length,
                maximum_length,
            } => write!(
                formatter,
                "counter key length {length} exceeds maximum {maximum_length}"
            ),
        }
    }
}

impl Error for CountersManagerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Reader(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CountersReaderError> for CountersManagerError {
    fn from(value: CountersReaderError) -> Self {
        Self::Reader(value)
    }
}
