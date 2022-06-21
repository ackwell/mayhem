//! Utilities for reading with binary tagfile formats.

mod bitfield;
mod common;
mod definition;
mod i32;
mod node;
mod string;
mod tagfile;

pub use tagfile::read;
