use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

macro_rules! read_primitive {
	($type:ty, $fn_name:ident) => {
		impl<R: Read> Tagfile<R> {
			#[inline]
			pub fn $fn_name(&mut self) -> Result<$type> {
				$fn_name(&mut self.reader)
			}
		}

		pub fn $fn_name(input: &mut impl Read) -> Result<$type> {
			let mut buffer = [0u8; std::mem::size_of::<$type>()];
			input.read_exact(&mut buffer)?;
			Ok(<$type>::from_le_bytes(buffer))
		}
	};
}

read_primitive!(u64, read_u64);
read_primitive!(u8, read_u8);
