use std::io::Read;

use crate::error::{Error, Result};

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_instance(&mut self) -> Result<()> {
		let definition_index = usize::try_from(self.read_i32()?).unwrap();
		let definition = self
			.definitions
			.get(definition_index)
			.and_then(|found| found.clone())
			.ok_or_else(|| {
				Error::Invalid(format!("Missing definition at index {definition_index}"))
			})?;

		println!("definition {definition:#?}");

		todo!("instance")
	}
}
