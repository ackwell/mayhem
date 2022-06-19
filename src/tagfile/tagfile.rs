use std::io::Read;

use crate::error::{Error, Result};

use super::{common::read_u64, i32::read_i32};

// TODO: return type
pub fn read(input: &mut impl Read) -> Result<()> {
	let magic = read_u64(input)?;
	if magic != 0xD011FACECAB00D1E {
		// TODO: macro for assets as errors.
		return Err(Error::Invalid(format!("Unexpected magic: {magic:#0x}")));
	}

	loop {
		let tag = Tag::from(read_i32(input)?);
		match tag {
			Tag::Metadata => {
				let version = read_i32(input)?;
				println!("file version {version}");
			}
			other => todo!("Unhandled tag kind {other:?}"),
		}
	}

	Ok(())
}

#[derive(Debug)]
enum Tag {
	Metadata,
}

impl From<i32> for Tag {
	fn from(value: i32) -> Self {
		match value {
			1 => Self::Metadata,
			other => todo!("Unhandled tag kind ID {other}."),
		}
	}
}
