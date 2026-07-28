// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Concurrent data structures and shared-memory protocol views.

#[cfg(not(target_has_atomic = "64"))]
compile_error!("the agrona-rs concurrent module requires native 64-bit atomics");

mod aligned_region;
pub mod status;
