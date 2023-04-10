use std::cmp::min;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;
use crate::{
	compressedanimation::compressedanimation::{
		AnimationTrait,
		BaseCompressedAnimation,
	},
	error::{Error, Result},
	macros::{read_primitive},
	NodeWalker,
};
use crate::compressedanimation::compressedanimation::InterpolatableTimeToValueTrait;
use crate::compressedanimation::concatanimation::ConcatAnimation;

#[derive(Debug)]
pub struct SplineCompressedAnimation {
	pub base: BaseCompressedAnimation,
	pub block_duration: f32,
	pub frame_duration: f32,
	pub blocks: Vec<Rc<Block>>,
	animation: ConcatAnimation,
}

impl SplineCompressedAnimation {
	pub fn new(node: &NodeWalker, base: BaseCompressedAnimation) -> Result<Self> {
		let max_frames_per_block = node.field_i32("maxFramesPerBlock", None)? as usize;
		let block_duration = node.field_f32("blockDuration", None)?;
		let frame_duration = node.field_f32("frameDuration", None)?;
		let mut block_offsets = node.field_i32_vec("blockOffsets", None)?
			.iter().map(|x| *x as usize).collect::<Vec<usize>>();
		let data = node.field_u8_vec("data", None)?;
		block_offsets.push(data.len());

		let mut num_pending_frames = node.field_i32("numFrames", None)? as usize;
		let mut pending_duration = node.field_f32("duration", None)?;

		let blocks = block_offsets.iter()
			.zip(block_offsets.iter().skip(1))
			.map(|(from, to)| {
				let num_frames = min(num_pending_frames, max_frames_per_block) as usize;
				num_pending_frames -= num_frames;

				let duration = if pending_duration > block_duration { block_duration } else { pending_duration };
				pending_duration -= duration;

				let block = Block::from_bytes(
					data.as_slice()[*from..*to].as_ref(),
					base.num_tracks,
					num_frames,
					frame_duration,
					duration);
				block.and_then(|x| Ok(Rc::new(x)))
			})
			.collect::<Result<Vec<Rc<Block>>>>()?;

		let animation = ConcatAnimation::new(blocks.iter().map(|x| x.to_owned() as Rc<dyn AnimationTrait>).collect())?;

		Ok(Self {
			base,
			block_duration,
			frame_duration,
			blocks,
			animation,
		})
	}
}

impl AnimationTrait for SplineCompressedAnimation {
	fn duration(&self) -> f32 { return self.base.duration; }

	fn num_tracks(&self) -> usize { return self.base.num_tracks; }

	fn frame_times(&self) -> Vec<f32> {
		self.animation.frame_times()
	}

	fn translation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>> {
		self.animation.translation(track_index)
	}

	fn rotation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<4>> {
		self.animation.rotation(track_index)
	}

	fn scale(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>> {
		self.animation.scale(track_index)
	}
}

struct BlockDataReader<'a> {
	reader: Cursor<&'a [u8]>,
}

impl<'a> BlockDataReader<'a> {
	fn new(data: &'a [u8]) -> Self {
		Self { reader: Cursor::new(data) }
	}

	fn align(&mut self, unit: usize) -> Result<()> {
		match self.reader.stream_position()? as usize % unit {
			0 => {}
			n => { self.reader.seek(SeekFrom::Current(unit as i64 - n as i64))?; }
		}

		Ok(())
	}

	read_primitive!(u8, read_u8);
	read_primitive!(u16, read_u16);
	read_primitive!(u32, read_u32);
	read_primitive!(f32, read_f32);

	fn read_scaled_compressed_scalar(&mut self, t: &CompressedScalarType) -> Result<f32> {
		match t {
			CompressedScalarType::K8 => Ok(self.read_u8()? as f32 / u8::MAX as f32),
			CompressedScalarType::K16 => Ok(self.read_u16()? as f32 / u16::MAX as f32),
		}
	}

	fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
		let mut res = vec![0u8; len];
		self.reader.read_exact(&mut res)?;
		Ok(res)
	}

	fn read_k32_quat(&mut self) -> Result<[f32; 4]> {

		let val = self.read_u32()?;

		let phi_theta = (val & 0x3FFFF) as f32;

		let r = 1f32 - (((val >> 18) & 0x3FF) as f32 / 0x3FF as f32).powi(2);

		let mut phi = phi_theta.sqrt().floor();
		let mut theta = 0f32;

		if phi != 0f32 {
			theta = std::f32::consts::PI / 4f32 * (phi_theta - phi * phi) / phi;
			phi = std::f32::consts::PI / 1022f32 * phi;
		}

		let magnitude = (1f32 - r.powi(2)).sqrt();
		let (s_phi, c_phi) = phi.sin_cos();
		let (s_theta, c_theta) = theta.sin_cos();

		Ok([
			s_phi * c_theta * magnitude * (if 0 == (val & 0x10000000) { 1f32 } else { -1f32 }),
			s_phi * s_theta * magnitude * (if 0 == (val & 0x20000000) { 1f32 } else { -1f32 }),
			c_phi * magnitude * (if 0 == (val & 0x40000000) { 1f32 } else { -1f32 }),
			r * (if 0 == (val & 0x80000000) { 1f32 } else { -1f32 }),
		])
	}

	fn read_k40_quat(&mut self) -> Result<[f32; 4]> {
		const MASK: u64 = (1 << 12) - 1;
		const DELTA: u64 = MASK >> 1;
		const DELTAF: f32 = DELTA as f32;

		let mut v = [0u8; 5];
		self.reader.read_exact(&mut v)?;
		let n = 0u64
			| ((v[4] as u64) << 32)
			| ((v[3] as u64) << 24)
			| ((v[2] as u64) << 16)
			| ((v[1] as u64) << 8)
			| ((v[0] as u64) << 0);

		let mut tmp: [f32; 4] = [
			(((n >> 0) & MASK) - DELTA) as f32 * std::f32::consts::FRAC_1_SQRT_2 / DELTAF,
			(((n >> 12) & MASK) - DELTA) as f32 * std::f32::consts::FRAC_1_SQRT_2 / DELTAF,
			(((n >> 24) & MASK) - DELTA) as f32 * std::f32::consts::FRAC_1_SQRT_2 / DELTAF,
			0f32,
		];
		let shift = ((n >> 36) & 0x3) as usize;
		let invert = 0 != ((n >> 38) & 0x1);

		tmp[3] = (1f32 - tmp[0] * tmp[0] - tmp[1] * tmp[1] - tmp[2] * tmp[2]).sqrt();
		if invert {
			tmp[3] = -tmp[3]
		}

		for i in 0..(3 - shift) {
			(tmp[3 - i], tmp[2 - i]) = (tmp[2 - i], tmp[3 - i])
		}

		Ok(tmp)
	}

	fn read_k48_quat(&mut self) -> Result<[f32; 4]> {
		const MASK: i32 = (1 << 15) - 1;
		const DELTA: i32 = MASK >> 1;
		const DELTAF: f32 = DELTA as f32;

		let x = self.read_u16()?;
		let y = self.read_u16()?;
		let z = self.read_u16()?;
		let shift = (((y >> 14) & 2) | (x >> 15)) as usize;
		let invert = 0 != (z & 0x8000);

		let mut tmp: [f32; 4] = [
			(((x as i32) & MASK) - DELTA) as f32 * std::f32::consts::FRAC_1_SQRT_2 / DELTAF,
			(((y as i32) & MASK) - DELTA) as f32 * std::f32::consts::FRAC_1_SQRT_2 / DELTAF,
			(((z as i32) & MASK) - DELTA) as f32 * std::f32::consts::FRAC_1_SQRT_2 / DELTAF,
			0f32,
		];

		tmp[3] = (1f32 - tmp[0] * tmp[0] - tmp[1] * tmp[1] - tmp[2] * tmp[2]).sqrt();
		if invert {
			tmp[3] = -tmp[3]
		}

		for i in 0..(3 - shift) {
			(tmp[3 - i], tmp[2 - i]) = (tmp[2 - i], tmp[3 - i])
		}

		Ok(tmp)
	}
}

#[derive(Debug)]
pub struct Block {
	pub num_frames: usize,
	pub frame_duration: f32,
	pub duration: f32,
	pub tracks: Vec<Track>,
}

impl Block {
	pub fn from_bytes(data: &[u8], num_tracks: usize, num_frames: usize, frame_duration: f32, duration: f32) -> Result<Self> {
		let mut reader = BlockDataReader::new(data);
		let masks = (0..num_tracks)
			.map(|_| match TransformMask::new(&mut reader) {
				Ok(v) => Ok(v),
				Err(e) => Err(Error::Invalid(e.to_string())),
			})
			.collect::<Result<Vec<TransformMask>>>()?;
		let tracks = masks.iter()
			.map(|mask| Track::new(&mut reader, mask, num_frames, frame_duration, duration))
			.collect::<Result<Vec<Track>>>()?;

		Ok(Self {
			num_frames,
			frame_duration,
			duration,
			tracks,
		})
	}
}

impl AnimationTrait for Block {
	fn duration(&self) -> f32 {
		self.duration
	}

	fn num_tracks(&self) -> usize {
		self.tracks.len()
	}

	fn frame_times(&self) -> Vec<f32> {
		(0..(self.num_frames - 1)).map(|x| x as f32 * self.frame_duration).collect()
	}

	fn translation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>> {
		self.tracks[track_index].translate.clone()
	}

	fn rotation(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<4>> {
		self.tracks[track_index].rotate.clone()
	}

	fn scale(&self, track_index: usize) -> Rc<dyn InterpolatableTimeToValueTrait<3>> {
		self.tracks[track_index].scale.clone()
	}
}

#[derive(Debug)]
pub struct Track {
	pub frames: usize,
	pub translate: Rc<TimedCompressedFloatArray<3>>,
	pub rotate: Rc<TimedCompressedFloatArray<4>>,
	pub scale: Rc<TimedCompressedFloatArray<3>>,
}

impl Track {
	fn new(mut reader: &mut BlockDataReader, mask: &TransformMask, num_frames: usize, frame_duration: f32, duration: f32) -> Result<Self> {
		let translate = CompressedFloatArray::<3>::new(&mut reader, &mask.translate, &mask.translate_primitive_type()?)?;
		reader.align(4)?;
		let rotate = CompressedFloatArray::<4>::new(&mut reader, &mask.rotate, &mask.rotate_primitive_type()?)?;
		reader.align(4)?;
		let scale = CompressedFloatArray::<3>::new(&mut reader, &mask.scale, &mask.scale_primitive_type()?)?;
		reader.align(4)?;
		Ok(Self {
			frames: num_frames,
			translate: TimedCompressedFloatArray::new(translate, num_frames, frame_duration, duration).into(),
			rotate: TimedCompressedFloatArray::new(rotate, num_frames, frame_duration, duration).into(),
			scale: TimedCompressedFloatArray::new(scale, num_frames, frame_duration, duration).into(),
		})
	}
}

#[derive(Debug)]
pub enum CompressedFloatArray<const COUNT: usize> {
	Spline(Nurbs<COUNT>),
	Static([f32; COUNT]),
	Empty,
}

impl CompressedFloatArray<3> {
	fn new(reader: &mut BlockDataReader, mask: &VectorMask, primitive_type: &CompressedScalarType) -> Result<Self> {
		if mask.has_spline() {
			let num_items = reader.read_u16()? as usize;
			let degree = reader.read_u8()? as usize;
			let knots = reader.read_bytes(num_items + degree + 2)?;
			reader.align(4)?;

			let mut ranges = [[0f32; 2]; 3];
			for i in 0..3 {
				for j in 0..(mask.mask(i)? as usize) {
					ranges[i][j] = reader.read_f32()?
				}
			}

			let mut control_points = vec![[0f32; 3]; num_items + 1];
			for control_point in &mut control_points {
				for j in 0..3 {
					control_point[j] = match mask.mask(j)? {
						ValueMask::Spline =>
							ranges[j][0] + (ranges[j][1] - ranges[j][0]) * reader.read_scaled_compressed_scalar(primitive_type)?,
						_ => ranges[j][0],
					};
				}
			}

			Ok(Self::Spline(Nurbs::<3>::new(control_points, knots, degree)))
		} else if mask.has_static() {
			Ok(Self::Static([
				match mask.mask(0)? {
					ValueMask::Static => reader.read_f32()?,
					_ => 0f32
				},
				match mask.mask(1)? {
					ValueMask::Static => reader.read_f32()?,
					_ => 0f32
				},
				match mask.mask(2)? {
					ValueMask::Static => reader.read_f32()?,
					_ => 0f32
				},
			]))
		} else {
			Ok(Self::Empty)
		}
	}
}

impl CompressedFloatArray<4> {
	fn new(reader: &mut BlockDataReader, mask: &QuatMask, primitive_type: &CompressedQuaternionType) -> Result<Self> {
		if mask.has_spline() {
			let num_items = reader.read_u16()? as usize;
			let degree = reader.read_u8()? as usize;
			let knots = reader.read_bytes(num_items + degree + 2)?;

			let control_points = match primitive_type {
				CompressedQuaternionType::K32 => (0..num_items + 1)
					.map(|_| reader.read_k32_quat())
					.collect::<Result<Vec<[f32; 4]>>>()?,
				CompressedQuaternionType::K40 => (0..num_items + 1)
					.map(|_| reader.read_k40_quat())
					.collect::<Result<Vec<[f32; 4]>>>()?,
				CompressedQuaternionType::K48 => (0..num_items + 1)
					.map(|_| reader.read_k48_quat())
					.collect::<Result<Vec<[f32; 4]>>>()?,
				_ => return Err(Error::Invalid("Unsupported compressed primitive type.".into())),
			};

			Ok(Self::Spline(Nurbs::<4>::new(control_points, knots, degree)))
		} else if mask.has_static() {
			Ok(Self::Static(reader.read_k40_quat()?))
		} else {
			Ok(Self::Empty)
		}
	}
}

#[derive(Debug)]
pub struct TimedCompressedFloatArray<const COUNT: usize> {
	array: CompressedFloatArray<COUNT>,
	num_frames: usize,
	frame_duration: f32,
	duration: f32,
}

impl<const COUNT: usize> TimedCompressedFloatArray<COUNT> {
	fn new(array: CompressedFloatArray<COUNT>, num_frames: usize, frame_duration: f32, duration: f32) -> Self {
		Self {
			array,
			num_frames,
			frame_duration,
			duration,
		}
	}
}

impl<const COUNT: usize> InterpolatableTimeToValueTrait<COUNT> for TimedCompressedFloatArray<COUNT> {
	fn is_empty(&self) -> bool {
		match self.array {
			CompressedFloatArray::Empty => true,
			_ => false,
		}
	}

	fn is_static(&self) -> bool {
		match self.array {
			CompressedFloatArray::Empty => true,
			CompressedFloatArray::Static(_) => true,
			_ => false,
		}
	}

	fn duration(&self) -> f32 {
		self.duration
	}

	fn frame_times(&self) -> Vec<f32> {
		match &self.array {
			CompressedFloatArray::Spline(_) => (0..(self.num_frames - 1)).map(|x| x as f32 * self.frame_duration).collect(),
			CompressedFloatArray::Static(_) => vec!(0f32),
			CompressedFloatArray::Empty => vec!(0f32),
		}
	}

	fn interpolate(&self, t: f32) -> [f32; COUNT] {
		match &self.array {
			CompressedFloatArray::Spline(nurbs) => nurbs.interpolate(t / self.frame_duration),
			CompressedFloatArray::Static(v) => *v,
			CompressedFloatArray::Empty => [0f32; COUNT],
		}
	}
}

enum ValueMask {
	Spline = 2,
	Static = 1,
	Empty = 0,
}

#[derive(Debug)]
struct VectorMask {
	bits: u8,
}

impl VectorMask {
	fn new(bits: u8) -> Self { Self { bits } }

	fn has_static(&self) -> bool { 0 != (self.bits & 0x0F) }
	fn has_spline(&self) -> bool { 0 != (self.bits & 0xF0) }

	fn mask(&self, component_index: usize) -> Result<ValueMask> {
		if 3 <= component_index {
			panic!("Component index out of range.")
		}
		match (self.bits >> component_index) & 0x11 {
			0x00 => Ok(ValueMask::Empty),
			0x01 => Ok(ValueMask::Static),
			0x10 => Ok(ValueMask::Spline),
			_ => Err(Error::Invalid("Invalid mask".into())),
		}
	}
}

#[derive(Debug)]
struct QuatMask {
	bits: u8,
}

impl QuatMask {
	fn new(bits: u8) -> Self { Self { bits } }

	fn has_static(&self) -> bool { 0 != (self.bits & 0x0F) }
	fn has_spline(&self) -> bool { 0 != (self.bits & 0xF0) }
}

#[derive(Debug)]
struct TransformMask {
	compression: u8,
	translate: VectorMask,
	rotate: QuatMask,
	scale: VectorMask,
}

impl TransformMask {
	fn new(reader: &mut BlockDataReader) -> Result<Self> {
		let mut buf = [0u8; 4];
		reader.reader.read_exact(&mut buf)?;
		Ok(Self {
			compression: buf[0],
			translate: VectorMask::new(buf[1]),
			rotate: QuatMask::new(buf[2]),
			scale: VectorMask::new(buf[3]),
		})
	}

	fn translate_primitive_type(&self) -> Result<CompressedScalarType> {
		CompressedScalarType::from(self.compression & 0x3)
	}

	fn rotate_primitive_type(&self) -> Result<CompressedQuaternionType> {
		CompressedQuaternionType::from((self.compression >> 2) & 0xF)
	}

	fn scale_primitive_type(&self) -> Result<CompressedScalarType> {
		CompressedScalarType::from(self.compression >> 6)
	}
}

enum CompressedScalarType {
	K8,
	K16,
}

impl CompressedScalarType {
	fn from(value: u8) -> Result<Self> {
		match value {
			0 => Ok(Self::K8),
			1 => Ok(Self::K16),
			_ => Err(Error::Invalid(format!("{0} is not a valid CompressedScalarType.", value).into()))
		}
	}
}

enum CompressedQuaternionType {
	K32,
	K40,
	K48,
	K24,
	K16,
	K128,
}

impl CompressedQuaternionType {
	fn from(value: u8) -> Result<Self> {
		match value {
			0 => Ok(Self::K32),
			1 => Ok(Self::K40),
			2 => Ok(Self::K48),
			3 => Ok(Self::K24),
			4 => Ok(Self::K16),
			5 => Ok(Self::K128),
			_ => Err(Error::Invalid(format!("{0} is not a valid CompressedQuaternionType.", value).into()))
		}
	}
}

#[derive(Debug)]
pub struct Nurbs<const N: usize> {
	control_points: Vec<[f32; N]>,
	knots: Vec<u8>,
	degree: usize,
}

impl<const N: usize> Nurbs<N> {
	pub fn new(control_points: Vec<[f32; N]>, knots: Vec<u8>, degree: usize) -> Self {
		Self {
			control_points,
			knots,
			degree,
		}
	}

	pub fn interpolate(&self, t: f32) -> [f32; N] {
		let span = self.find_span(t);
		let basis = self.bspline_basis(span, t);

		let mut value = [0f32; N];
		for i in 0..(self.degree + 1) {
			for j in 0..N {
				value[j] += self.control_points[span - i][j] * basis[i]
			}
		}

		value
	}

	/*
	 * bsplineBasis and findSpan are based on the implementations of
	 * https://github.com/PredatorCZ/HavokLib
	 */
	fn bspline_basis(&self, span: usize, t: f32) -> [f32; N] {
		let mut res = [0f32; N];
		res[0] = 1f32;

		for i in 0..self.degree {
			for j in (0..(i + 1)).rev() {
				let mut tmp = res[j];
				tmp *= t - self.knots[span - j] as f32;
				tmp /= (self.knots[span + i + 1 - j] - self.knots[span - j]) as f32;
				res[j + 1] += res[j] - tmp;
				res[j] = tmp;
			}
		}

		res
	}

	fn find_span(&self, t: f32) -> usize {
		if t >= self.knots[self.control_points.len()].into() {
			self.control_points.len() - 1
		} else {
			let mut low = self.degree;
			let mut high = self.control_points.len();
			let mut mid = (low + high) / 2;

			while t < self.knots[mid].into() || t >= self.knots[mid + 1].into() {
				if t < self.knots[mid].into() {
					high = mid;
				} else {
					low = mid;
				}

				mid = (low + high) / 2;
			}

			mid
		}
	}
}
