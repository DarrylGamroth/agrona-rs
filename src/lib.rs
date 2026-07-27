// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Unofficial, idiomatic Rust port of selected
//! [Agrona](https://github.com/aeron-io/agrona) components.
//!
//! The Clock family is implemented. The complete Agent family is selected as
//! the next delivery increment. Their design and acceptance gates are
//! recorded in the repository's porting plan.

pub mod clock;
