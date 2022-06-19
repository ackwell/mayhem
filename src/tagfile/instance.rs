use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_instance(&mut self) -> Result<()> {
		todo!("instance")
	}
}
