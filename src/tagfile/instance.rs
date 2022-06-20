use std::io::Read;

use crate::error::{Error, Result};

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_instance(&mut self) -> Result<()> {
		// Read & resolve the definition for this instance.
		let definition_index = usize::try_from(self.read_i32()?).unwrap();
		let definition = self
			.definitions
			.get(definition_index)
			.and_then(|found| found.clone())
			.ok_or_else(|| {
				Error::Invalid(format!("Missing definition at index {definition_index}"))
			})?;

		let fields = definition.fields();
		let stored_fields = self.read_bitfield(fields.len())?;
		let values = fields
			.into_iter()
			.enumerate()
			.filter_map(|(index, field)| match stored_fields[index] {
				true => Some(self.read_value(field)),
				false => None,
			})
			.collect::<Result<Vec<_>>>()?;

		todo!("instance")
	}

	fn read_value(&mut self, field: &Field) -> Result<()> {
		println!("{field:#?}");
		Ok(())
	}
}
