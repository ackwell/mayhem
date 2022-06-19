use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_definition(&mut self) -> Result<Definition> {
		let name = self.read_string()?;
		let version = self.read_i32()?;

		Ok(Definition { name, version })
	}
}

// TODO: Definitions might make sense outside the immediate context of tagfiles, lift out?
#[derive(Debug)]
pub struct Definition {
	name: String,
	version: i32,
}
