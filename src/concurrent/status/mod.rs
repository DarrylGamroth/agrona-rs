// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Agrona-compatible counter status views.
//!
//! This increment provides a read-only [`CountersReader`] over caller-owned
//! metadata and values regions. It does not create mappings, allocate or free
//! counters, or mutate counter values.

mod counters_reader;
mod counters_reader_error;

pub use counters_reader::CountersReader;
pub use counters_reader_error::CountersReaderError;
