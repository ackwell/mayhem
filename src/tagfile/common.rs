use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	#[inline]
	pub fn read_u64(&mut self) -> Result<u64> {
		read_u64(&mut self.reader)
	}

	#[inline]
	pub fn read_u8(&mut self) -> Result<u8> {
		read_u8(&mut self.reader)
	}
}

pub fn read_u64(input: &mut impl Read) -> Result<u64> {
	let mut buffer = [0u8; 8];
	input.read_exact(&mut buffer)?;
	let value = u64::from_le_bytes(buffer);
	Ok(value)
}

pub fn read_u8(input: &mut impl Read) -> Result<u8> {
	let mut buffer = [0u8; 1];
	input.read_exact(&mut buffer)?;
	let value = u8::from_le_bytes(buffer);
	Ok(value)
}
