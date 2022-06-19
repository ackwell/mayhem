use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_definition(&mut self) -> Result<Definition> {
		Ok(Definition {})
	}
}

// TODO: Definitions might make sense outside the immediate context of tagfiles, lift out?
#[derive(Debug)]
pub struct Definition {}
