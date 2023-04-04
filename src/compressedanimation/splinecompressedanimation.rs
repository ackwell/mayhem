use std::cmp::min;
use std::io::{Cursor, Read, Seek, SeekFrom};
use crate::{
	compressedanimation::compressedanimation::{
		AnimationTrait,
		BaseCompressedAnimation,
	},
	error::{Error, Result},
	macros::{read_primitive},
	NodeWalker,
};

#[derive(Debug)]
pub struct SplineCompressedAnimation {
	pub base: BaseCompressedAnimation,
	pub block_duration: f32,
	pub frame_duration: f32,
	pub blocks: Vec<Block>,
}

impl SplineCompressedAnimation {
	pub fn new(node: &NodeWalker, base: BaseCompressedAnimation) -> Result<Self> {
		let num_frames = node.field_i32("numFrames", None)? as usize;
		let max_frames_per_block = node.field_i32("maxFramesPerBlock", None)? as usize;
		let block_duration = node.field_f32("blockDuration", None)?;
		let frame_duration = node.field_f32("frameDuration", None)?;
		let mut block_offsets = node.field_i32_vec("blockOffsets", None)?
			.iter().map(|x| *x as usize).collect::<Vec<usize>>();
		let data = node.field_u8_vec("data", None)?;
		block_offsets.push(data.len());

		let mut num_pending_frames = num_frames;

		let blocks = block_offsets.iter()
			.zip(block_offsets.iter().skip(1))
			.map(|(from, to)| {
				let num_block_frames = min(num_pending_frames, max_frames_per_block) as usize;
				num_pending_frames -= num_block_frames;
				Block::from_bytes(data.as_slice()[*from..*to].as_ref(), num_block_frames, base.num_tracks)
			})
			.collect::<Result<Vec<Block>>>()?;

		Ok(Self {
			base,
			block_duration,
			frame_duration,
			blocks,
		})
	}
}

impl AnimationTrait for SplineCompressedAnimation {
	fn duration(&self) -> f32 { return self.base.duration; }

	fn num_tracks(&self) -> usize { return self.base.num_tracks; }

	fn is_empty(&self, track_index: usize) -> bool {
		self.blocks.iter()
			.map(|b| &b.tracks[track_index])
			.all(|t| (true
				&& match t.translate { CompressedVec3Array::Empty => true, _ => false}
				&& match t.rotate { CompressedQuatArray::Empty => true, _ => false}
				&& match t.scale { CompressedVec3Array::Empty => true, _ => false}
			))
	}

	fn translate(&self, track_index: usize, mut time: f32) -> [f32; 3] {
		time %= self.base.duration;
		let block_index = (time / self.block_duration) as usize;
		let track = &self.blocks[block_index].tracks[track_index];
		match &track.translate {
			CompressedVec3Array::Spline(n) => n.index((time % self.block_duration) / self.frame_duration),
			CompressedVec3Array::Static(v) => *v,
			CompressedVec3Array::Empty => [0f32; 3],  // defaults to no translate
		}
	}

	fn rotate(&self, track_index: usize, mut time: f32) -> [f32; 4] {
		time %= self.base.duration;
		let block_index = (time / self.block_duration) as usize;
		let track = &self.blocks[block_index].tracks[track_index];
		match &track.rotate {
			CompressedQuatArray::Spline(n) => n.index((time % self.block_duration) / self.frame_duration),
			CompressedQuatArray::Static(v) => *v,
			CompressedQuatArray::Empty => [0f32, 0f32, 0f32, 1f32],  // unit quaternion
		}
	}

	fn scale(&self, track_index: usize, mut time: f32) -> [f32; 3] {
		time %= self.base.duration;
		let block_index = (time / self.block_duration) as usize;
		let track = &self.blocks[block_index].tracks[track_index];
		match &track.scale {
			CompressedVec3Array::Spline(n) => n.index((time % self.block_duration) / self.frame_duration),
			CompressedVec3Array::Static(v) => *v,
			CompressedVec3Array::Empty => [1f32; 3],  // defaults to scale 100%
		}
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
	read_primitive!(f32, read_f32);

	fn read_scaled_compressed_scalar(&mut self, t: &CompressedPrimitiveType) -> Result<f32> {
		match t {
			CompressedPrimitiveType::K8 => Ok(self.read_u8()? as f32 / u8::MAX as f32),
			CompressedPrimitiveType::K16 => Ok(self.read_u16()? as f32 / u16::MAX as f32),
			_ => Err(Error::Invalid("Unsupported compressed primitive type".into()))
		}
	}

	fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
		let mut res = vec![0u8; len];
		self.reader.read_exact(&mut res)?;
		Ok(res)
	}

	fn read_k40_quat(&mut self) -> Result<[f32; 4]> {
		const DELTA: i32 = 0x801;
		const FRACTAL: f32 = 0.000345436;
		let mut v = [0u8; 5];
		self.reader.read_exact(&mut v)?;
		let n = 0u64
			| ((v[4] as u64) << 32)
			| ((v[3] as u64) << 24)
			| ((v[2] as u64) << 16)
			| ((v[1] as u64) << 8)
			| ((v[0] as u64) << 0);

		let mut tmp = [0f32; 4];
		tmp[0] = (((n >> 0) & 0xFFF) as i32 - DELTA) as f32 * FRACTAL;
		tmp[1] = (((n >> 12) & 0xFFF) as i32 - DELTA) as f32 * FRACTAL;
		tmp[2] = (((n >> 24) & 0xFFF) as i32 - DELTA) as f32 * FRACTAL;
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
}

#[derive(Debug)]
pub struct Block {
	pub num_frames: usize,
	pub tracks: Vec<Track>,
}

impl Block {
	pub fn from_bytes(data: &[u8], num_frames: usize, num_tracks: usize) -> Result<Self> {
		let mut reader = BlockDataReader::new(data);
		let masks = (0..num_tracks)
			.map(|_| match TransformMask::new(&mut reader) {
				Ok(v) => Ok(v),
				Err(e) => Err(Error::Invalid(e.to_string())),
			})
			.collect::<Result<Vec<TransformMask>>>()?;
		let tracks = masks.iter()
			.map(|mask| Track::new(&mut reader, mask, num_frames))
			.collect::<Result<Vec<Track>>>()?;

		Ok(Self {
			num_frames,
			tracks,
		})
	}
}

#[derive(Debug)]
pub struct Track {
	pub frames: usize,
	pub translate: CompressedVec3Array,
	pub rotate: CompressedQuatArray,
	pub scale: CompressedVec3Array,
}

impl Track {
	fn new(mut reader: &mut BlockDataReader, mask: &TransformMask, frames: usize) -> Result<Self> {
		let translate = CompressedVec3Array::new(&mut reader, &mask.translate, &mask.translate_primitive_type()?)?;
		reader.align(4)?;
		let rotate = CompressedQuatArray::new(&mut reader, &mask.rotate, &mask.rotate_primitive_type()?)?;
		reader.align(4)?;
		let scale = CompressedVec3Array::new(&mut reader, &mask.scale, &mask.scale_primitive_type()?)?;
		reader.align(4)?;
		Ok(Self {
			frames,
			translate,
			rotate,
			scale,
		})
	}
}

#[derive(Debug)]
pub enum CompressedVec3Array {
	Spline(Nurbs<3>),
	Static([f32; 3]),
	Empty,
}

impl CompressedVec3Array {
	fn new(reader: &mut BlockDataReader, mask: &VectorMask, primitive_type: &CompressedPrimitiveType) -> Result<Self> {
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

#[derive(Debug)]
pub enum CompressedQuatArray {
	Spline(Nurbs<4>),
	Static([f32; 4]),
	Empty,
}

impl CompressedQuatArray {
	fn new(reader: &mut BlockDataReader, mask: &QuatMask, primitive_type: &CompressedPrimitiveType) -> Result<Self> {
		if mask.has_spline() {
			let num_items = reader.read_u16()? as usize;
			let degree = reader.read_u8()? as usize;
			let knots = reader.read_bytes(num_items + degree + 2)?;

			let control_points = match primitive_type {
				CompressedPrimitiveType::K40 => (0..num_items + 1)
					.map(|_| reader.read_k40_quat())
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

	fn translate_primitive_type(&self) -> Result<CompressedPrimitiveType> {
		CompressedPrimitiveType::from(self.compression & 0x3)
	}

	fn rotate_primitive_type(&self) -> Result<CompressedPrimitiveType> {
		CompressedPrimitiveType::from(((self.compression >> 2) & 0xF) + 2)
	}

	fn scale_primitive_type(&self) -> Result<CompressedPrimitiveType> {
		CompressedPrimitiveType::from(self.compression >> 6)
	}
}

enum CompressedPrimitiveType {
	K8,
	K16,
	K32,
	K40,
	K48,
}

impl CompressedPrimitiveType {
	fn from(value: u8) -> Result<Self> {
		match value {
			0 => Ok(CompressedPrimitiveType::K8),
			1 => Ok(CompressedPrimitiveType::K16),
			2 => Ok(CompressedPrimitiveType::K32),
			3 => Ok(CompressedPrimitiveType::K40),
			4 => Ok(CompressedPrimitiveType::K48),
			_ => Err(Error::Invalid(format!("{0} is not a valid CompressedPrimitiveType.", value).into()))
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

	pub fn index(&self, t: f32) -> [f32; N] {
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
