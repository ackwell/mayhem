use std::rc::Rc;
use crate::{
	compressedanimation::AnimationTrait,
	compressedanimation::compressedanimation::InterpolatableTimeToValueTrait,
	error::{Error, Result},
};

#[derive(Debug)]
pub struct ConcatAnimation {
	parts: Vec<Rc<dyn AnimationTrait>>,
	translations: Vec<Rc<ConcatInterpolatableTimeToValue<3>>>,
	rotations: Vec<Rc<ConcatInterpolatableTimeToValue<4>>>,
	scales: Vec<Rc<ConcatInterpolatableTimeToValue<3>>>,
}

impl ConcatAnimation {
	pub fn new(parts: Vec<Rc<dyn AnimationTrait>>) -> Result<Self> {
		let mut translations = Vec::<Rc<ConcatInterpolatableTimeToValue<3>>>::new();
		let mut rotations = Vec::<Rc<ConcatInterpolatableTimeToValue<4>>>::new();
		let mut scales = Vec::<Rc<ConcatInterpolatableTimeToValue<3>>>::new();

		if !parts.is_empty() {
			if parts.iter().skip(1).any(|x| x.duration() != parts[0].duration()) {
				return Err(Error::Invalid("Durations of all parts must be equal.".into()))
			}

			let num_tracks = parts[0].num_tracks();
			if parts.iter().skip(1).any(|x| x.num_tracks() != num_tracks) {
				return Err(Error::Invalid("Number of tracks of all parts must be equal.".into()))
			}

			for track in 0..num_tracks {
				translations.push(ConcatInterpolatableTimeToValue::new(parts.iter().map(|x| x.translation(track) as Rc<dyn InterpolatableTimeToValueTrait<3>>).collect())?.into());
				rotations.push(ConcatInterpolatableTimeToValue::new(parts.iter().map(|x| x.rotation(track) as Rc<dyn InterpolatableTimeToValueTrait<4>>).collect())?.into());
				scales.push(ConcatInterpolatableTimeToValue::new(parts.iter().map(|x| x.scale(track) as Rc<dyn InterpolatableTimeToValueTrait<3>>).collect())?.into());
			}
		}

		Ok(Self {
			parts,
			translations,
			rotations,
			scales
		})
	}
}

impl AnimationTrait for ConcatAnimation {
	fn duration(&self) -> f32 {
		if self.parts.is_empty() { 0f32 } else { self.parts[0].duration() }
	}

	fn num_tracks(&self) -> usize {
		if self.parts.is_empty() { 0 } else { self.parts[0].num_tracks() }
	}

	fn frame_times(&self) -> Vec<f32> {
		let mut res = Vec::<f32>::new();
		let mut t = 0f32;
		for part in &self.parts {
			res.extend(part.frame_times().iter().map(|x| x + t));
			t += part.duration();
		}
		res
	}

	fn translation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>> {
		self.translations[track_index].clone()
	}

	fn rotation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<4>> {
		self.rotations[track_index].clone()
	}

	fn scale(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>> {
		self.scales[track_index].clone()
	}
}

#[derive(Debug)]
struct ConcatInterpolatableTimeToValue<const COUNT: usize> {
	parts: Vec<Rc<dyn InterpolatableTimeToValueTrait<COUNT>>>,
}

impl<const COUNT: usize> ConcatInterpolatableTimeToValue<COUNT> {
	fn new(parts: Vec<Rc<dyn InterpolatableTimeToValueTrait<COUNT>>>) -> Result<Self> {
		if parts.is_empty() {
			return Err(Error::Invalid("There must be at least one part.".into()))
		}

		if parts.iter().any(|x| x.duration() != parts[0].duration()) {
			return Err(Error::Invalid("All parts must have the same duration.".into()))
		}

		Ok(Self {
			parts,
		})
	}
}

impl<const COUNT: usize> InterpolatableTimeToValueTrait<COUNT> for ConcatInterpolatableTimeToValue<COUNT> {
	fn is_empty(&self) -> bool {
		self.parts.iter().all(|x| x.is_empty())
	}

	fn is_static(&self) -> bool {
		self.parts.iter().all(|x| x.is_static())
	}

	fn duration(&self) -> f32 {
		self.parts.iter().map(|x| x.duration()).sum()
	}

	fn frame_times(&self) -> Vec<f32> {
		let mut res = Vec::<f32>::new();

		let mut t = 0f32;
		for part in &self.parts {
			res.extend(part.frame_times().iter().map(|x| x + t));
			t += part.duration();
		}

		res
	}

	fn interpolate(&self, mut t: f32) -> [f32; COUNT] {
		loop {
			for part in &self.parts {
				if t < part.duration() {
					return part.interpolate(t)
				}

				t -= part.duration()
			}
		}
	}
}
