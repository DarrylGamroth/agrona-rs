// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Agrona-compatible counter readers, managers, values, positions, and status
//! indicators over caller-owned memory regions.

mod atomic_counter;
mod atomic_long_position;
mod counter_allocation_error;
mod counters_manager;
mod counters_manager_error;
mod counters_reader;
mod counters_reader_error;
mod position;
mod readable_position;
mod status_indicator;
mod status_indicator_reader;
mod unsafe_buffer_position;
mod unsafe_buffer_status_indicator;

pub use atomic_counter::AtomicCounter;
pub use atomic_long_position::AtomicLongPosition;
pub use counter_allocation_error::CounterAllocationError;
pub use counters_manager::CountersManager;
pub use counters_manager_error::CountersManagerError;
pub use counters_reader::CountersReader;
pub use counters_reader_error::CountersReaderError;
pub use position::Position;
pub use readable_position::ReadablePosition;
pub use status_indicator::StatusIndicator;
pub use status_indicator_reader::StatusIndicatorReader;
pub use unsafe_buffer_position::UnsafeBufferPosition;
pub use unsafe_buffer_status_indicator::UnsafeBufferStatusIndicator;
