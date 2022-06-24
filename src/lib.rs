//! Pure rust file format reader for a popular game middleware suite.

#![allow(clippy::module_inception)]
#![warn(missing_debug_implementations, missing_docs)]

mod error;
mod node;
mod value;
mod walker;

pub mod tagfile;

pub use {error::Error, value::Value, walker::NodeWalker};
