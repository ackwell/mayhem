#![allow(clippy::module_inception)]

mod error;
mod node;
mod walker;

pub mod tagfile;

pub use {error::Error, walker::NodeWalker};
