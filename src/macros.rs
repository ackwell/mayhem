
macro_rules! read_primitive {
	($type:ty, $fn_name:ident) => {
		pub fn $fn_name(&mut self) -> Result<$type> {
			let mut buffer = [0u8; std::mem::size_of::<$type>()];
			self.reader.read_exact(&mut buffer)?;
			Ok(<$type>::from_le_bytes(buffer))
		}
	};
}

pub(crate) use read_primitive;
