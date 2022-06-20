use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

macro_rules! read_primitive {
	($type:ty, $fn_name:ident) => {
		pub fn $fn_name(&mut self) -> Result<$type> {
			let mut buffer = [0u8; std::mem::size_of::<$type>()];
			self.reader.read_exact(&mut buffer)?;
			Ok(<$type>::from_le_bytes(buffer))
		}
	};
}

impl<R: Read> Tagfile<R> {
	read_primitive!(u64, read_u64);
	read_primitive!(u8, read_u8);
}
