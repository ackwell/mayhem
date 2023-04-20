use std::fmt::Debug;
use std::rc::Rc;
use crate::{
	compressedanimation::splinecompressedanimation::{SplineCompressedAnimation},
	error::{Error, Result},
	NodeWalker,
};

/// Represent values that may change over time.
pub trait InterpolatableTimeToValueTrait<const COUNT: usize> {
	/// Determine whether there are any values.
	fn is_empty(&self) -> bool;

	/// Determine whether there are more than one value.
	fn is_static(&self) -> bool;

	/// Get the duration stored.
	fn duration(&self) -> f32;

	/// Get the significant time points of frames, in seconds.
	fn frame_times(&self) -> Vec<f32>;

	/// Get the interpolated value over time in seconds.
	fn interpolate(&self, t: f32) -> [f32; COUNT];
}

impl<const COUNT: usize> Debug for dyn InterpolatableTimeToValueTrait<COUNT> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "duration={}", self.duration())
	}
}

/// Represent an animation consisting of TRS components.
pub trait AnimationTrait {
	/// Get the duration of the animation.
	fn duration(&self) -> f32;

	/// Get the number of tracks(bones) stored in this animation.
	fn num_tracks(&self) -> usize;

	/// Get the significant time points of frames, in seconds.
	fn frame_times(&self) -> Vec<f32>;

	/// Get the translation component of this animation of specified track(bone).
	fn translation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>>;

	/// Get the rotation component of this animation of specified track(bone).
	fn rotation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<4>>;

	/// Get the scale component of this animation of specified track(bone).
	fn scale(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>>;
}

impl Debug for dyn AnimationTrait {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "duration={}, num_tracks={}", self.duration(), self.num_tracks())
	}
}

#[derive(Debug)]
pub struct BaseCompressedAnimation {
	pub duration: f32,
	pub num_tracks: usize,
}

impl BaseCompressedAnimation {
	pub fn new(node: &NodeWalker) -> Result<Self> {
		if !node.is_or_inherited_from("hkaAnimation") {
			return Err(Error::Invalid(
				"Given node is not a valid animation.".into()
			));
		}

		let duration = node.field_f32("duration", None)?;
		let num_transform_tracks = node.field_i32("numberOfTransformTracks", None)? as usize;

		return Ok(Self {
			duration,
			num_tracks: num_transform_tracks,
		});
	}
}

/// Create a new animation from the given hkaAnimation node.
pub fn read_animation(animation_node: &NodeWalker) -> Result<Rc<dyn AnimationTrait>> {
	let base = BaseCompressedAnimation::new(animation_node)?;

	if animation_node.is_or_inherited_from("hkaSplineCompressedAnimation") {
		Ok(Rc::new(SplineCompressedAnimation::new(animation_node, base)?))
	} else {
		Err(Error::Invalid(format!("Unsupported animation type.")))
	}
}

/// Create a new vector of animations from the given root node of a tagfile.
pub fn new_from_root(root_node: &NodeWalker) -> Result<Vec<Rc<dyn AnimationTrait>>> {
	root_node
		.field_node_vec("namedVariants")?.first()
		.ok_or(Error::Invalid("namedVariants node contains no children.".into()))?
		.field_node("variant")?
		.field_node_vec("animations")?
		.iter()
		.map(|animation_node| read_animation(animation_node))
		.collect()
}