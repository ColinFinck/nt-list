// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

// Required for deriving our traits when testing.
#[cfg(test)]
extern crate self as nt_list;

pub mod list;
mod private;
pub mod single_list;
mod traits;

pub use traits::*;
