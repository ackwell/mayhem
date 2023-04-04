use crate::{
	compressedanimation::splinecompressedanimation::{SplineCompressedAnimation},
	error::{Error, Result},
	NodeWalker,
};

/// Trait for retrieving TRS information from an animation.
pub trait AnimationTrait {
	/// Get the duration of the animation.
	fn duration(&self) -> f32;

	/// Get the number of tracks(bones) in the animation.
	fn num_tracks(&self) -> usize;

	/// Determine whether the track of given index is empty.
	fn is_empty(&self, track_index: usize) -> bool;

	/// Get the translation(Vector3) in the animation of the specified track at given time point.
	fn translate(&self, track_index: usize, time: f32) -> [f32; 3];

	/// Get the rotation(Quaternion) in the animation of the specified track at given time point.
	fn rotate(&self, track_index: usize, time: f32) -> [f32; 4];

	/// Get the scale(Vector3) in the animation of the specified track at given time point.
	fn scale(&self, track_index: usize, time: f32) -> [f32; 3];
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

/// Represent a compressed animation.
#[derive(Debug)]
pub enum CompressedAnimation {
	/// This animation is compressed using spline compression.
	SplineCompressedAnimation(SplineCompressedAnimation),
}

impl CompressedAnimation {
	/// Create a new CompressedAnimation from the given animation node.
	pub fn new(animation_node: &NodeWalker) -> Result<Self> {
		let base = BaseCompressedAnimation::new(animation_node)?;

		if animation_node.is_or_inherited_from("hkaSplineCompressedAnimation") {
			Ok(CompressedAnimation::SplineCompressedAnimation(SplineCompressedAnimation::new(animation_node, base)?))
		} else {
			Err(Error::Invalid(format!("Unsupported animation type.")))
		}
	}

	/// Create a new vector of CompressedAnimation from the given root node of a tagfile.
	pub fn new_from_root(root_node: &NodeWalker) -> Result<Vec<Self>> {
		root_node
			.field_node_vec("namedVariants")?.first()
			.ok_or(Error::Invalid("namedVariants node contains no children.".into()))?
			.field_node("variant")?
			.field_node_vec("animations")?
			.iter()
			.map(|animation_node| Self::new(animation_node))
			.collect()
	}
}

impl AnimationTrait for CompressedAnimation {
	fn duration(&self) -> f32 {
		match self {
			CompressedAnimation::SplineCompressedAnimation(a) => a.duration()
		}
	}

	fn num_tracks(&self) -> usize {
		match self {
			CompressedAnimation::SplineCompressedAnimation(a) => a.num_tracks()
		}
	}

	fn is_empty(&self, track_index: usize) -> bool {
		match self {
			CompressedAnimation::SplineCompressedAnimation(a) => a.is_empty(track_index)
		}
	}

	fn translate(&self, track_index: usize, time: f32) -> [f32; 3] {
		match self {
			CompressedAnimation::SplineCompressedAnimation(a) => a.translate(track_index, time)
		}
	}

	fn rotate(&self, track_index: usize, time: f32) -> [f32; 4] {
		match self {
			CompressedAnimation::SplineCompressedAnimation(a) => a.rotate(track_index, time)
		}
	}

	fn scale(&self, track_index: usize, time: f32) -> [f32; 3] {
		match self {
			CompressedAnimation::SplineCompressedAnimation(a) => a.scale(track_index, time)
		}
	}
}
