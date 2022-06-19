use std::io::Read;

use crate::error::{Error, Result};

use super::common::read_u64;

// TODO: return type
pub fn read(input: &mut impl Read) -> Result<()> {
	let magic = read_u64(input)?;
	if magic != 0xD011FACECAB00D1E {
		// TODO: macro for assets as errors.
		return Err(Error::Invalid(format!("Unexpected magic: {magic:#0x}")));
	}

	Ok(())
}
