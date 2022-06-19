use std::io::Read;

use crate::error::{Error, Result};

// TODO: return type
pub fn read(input: &mut impl Read) -> Result<()> {
	let mut tagfile = Tagfile::new(input);
	tagfile.read()?;

	Ok(())
}

pub struct Tagfile<R> {
	pub version: i32,

	pub reader: R,
}

impl<R: Read> Tagfile<R> {
	fn new(reader: R) -> Self {
		Self {
			version: -1,
			reader,
		}
	}

	fn read(&mut self) -> Result<()> {
		let magic = self.read_u64()?;
		if magic != 0xD011FACECAB00D1E {
			// TODO: macro for assets as errors.
			return Err(Error::Invalid(format!("Unexpected magic: {magic:#0x}.")));
		}

		loop {
			let tag = Tag::from(self.read_i32()?);
			match tag {
				Tag::Metadata => {
					self.version = self.read_i32()?;
					if self.version != 3 {
						todo!("Unhandled file version {}.", self.version)
					}
				}

				Tag::Definition => {
					let definition = self.read_definition()?;
					todo!("{definition:#?}")
				}

				other => todo!("Unhandled tag kind {other:?}."),
			}
		}

		Ok(())
	}
}

#[derive(Debug)]
enum Tag {
	Metadata,
	Definition,
}

impl From<i32> for Tag {
	fn from(value: i32) -> Self {
		match value {
			1 => Self::Metadata,
			2 => Self::Definition,
			other => todo!("Unhandled tag kind ID {other}."),
		}
	}
}
