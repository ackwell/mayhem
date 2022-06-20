use std::io::Read;

use crate::error::{Error, Result};

use super::{
	definition::{Field, FieldKind},
	tagfile::Tagfile,
};

impl<R: Read> Tagfile<R> {
	pub fn read_instance(&mut self) -> Result<()> {
		// Read & resolve the definition for this instance.
		let definition_index = usize::try_from(self.read_i32()?).unwrap();
		let definition = self
			.definitions
			.get(definition_index)
			.and_then(|found| found.clone())
			.ok_or_else(|| {
				Error::Invalid(format!("Missing definition at index {definition_index}."))
			})?;

		// Read fields. Order is guaranteed to follow definition fields, however
		// values may be sparse, as defined by the bitfield.
		let fields = definition.fields();
		let stored_fields = self.read_bitfield(fields.len())?;
		let values = fields
			.into_iter()
			.zip(stored_fields.into_iter())
			.filter(|(_, stored)| *stored)
			.map(|(field, _)| self.read_value(field))
			.collect::<Result<Vec<_>>>()?;

		todo!("instance")
	}

	// TODO: does this need the full field, or just the field kind?
	// TODO: return type. probably needs a value enum.
	fn read_value(&mut self, field: &Field) -> Result<()> {
		match &field.kind {
			FieldKind::Array(inner_kind) => {
				let count = usize::try_from(self.read_i32()?).unwrap();
				println!("{count}");
				self.read_value_array(&*inner_kind, count)?
			}
			other => todo!("Unhandled field kind {other:?}."),
		};
		Ok(())
	}

	// TODO: return type. Not sure how to resolve what the inner type will be... Vec<Value>? but that then means that every entry in the vec could technically be a different type, which... isn't nice, to say the least.
	fn read_value_array(&mut self, kind: &FieldKind, count: usize) -> Result<()> {
		match kind {
			FieldKind::String => {
				let strings = (0..count)
					.map(|_| self.read_string())
					.collect::<Result<Vec<_>>>()?;
			}
			// TODO: this is probably complicated enough to warrant its own function.
			FieldKind::Struct(definition_name) => {
				// Read in the definition for the requested struct and its fields.
				let definition = self
					.definitions
					.iter()
					.find_map(|option| {
						option
							.as_ref()
							.filter(|definition| &definition.name == definition_name)
							.cloned()
					})
					.ok_or_else(|| {
						Error::Invalid(format!("Missing requested definition {definition_name}."))
					})?;

				// Read values for the fields of the struct. Of note, fields are flattened
				// - all of the first field for the entire array will be read before any
				// of the second, and so on.
				// TODO: This is similar to logic in read_instance - deduplicate?
				let fields = definition.fields();
				let stored_fields = self.read_bitfield(fields.len())?;
				let values = fields
					.into_iter()
					.zip(stored_fields.into_iter())
					.filter(|(_, stored)| *stored)
					.map(|(field, _)| self.read_value_array(&field.kind, count));
			}
			other => todo!("Unhandled array kind {kind:?}"),
		};

		Ok(())
	}
}
