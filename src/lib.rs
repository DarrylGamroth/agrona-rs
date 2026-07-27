// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Unofficial, idiomatic Rust port of selected
//! [Agrona](https://github.com/aeron-io/agrona) components.
//!
//! The Clock family and selected Agent family are implemented against the
//! repository's normative specifications.

pub mod agent;
pub mod clock;
