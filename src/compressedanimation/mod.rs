//! Utilities for decompressing compressed TRS animation.

mod compressedanimation;
mod splinecompressedanimation;
mod concatanimation;

pub use compressedanimation::{AnimationTrait, InterpolatableTimeToValueTrait, read_animation, new_from_root};
