use std::io::Read;

use crate::error::{Error, Result};

use super::{
	definition::{Field, FieldKind},
	tagfile::Tagfile,
};

impl<R: Read> Tagfile<R> {
	// TODO: what's the return type going to look like here? For consistency, it should probably act like a reference?
	pub fn read_node(&mut self) -> Result<()> {
		// Read & resolve the definition for this node.
		let definition_index = usize::try_from(self.read_i32()?).unwrap();
		let definition = self
			.definitions
			.get(definition_index)
			.and_then(|found| found.clone())
			.ok_or_else(|| {
				Error::Invalid(format!("Missing definition at index {definition_index}."))
			})?;

		// Get the next reference index for this node. If it's already been requested,
		// we use the pre-reserved node index rather than adding an additional one.
		let reference_index = self.references.len();
		let node_index = match self.pending_references.remove_entry(&reference_index) {
			Some((_key, value)) => value,
			None => {
				let next_index = self.nodes.len();
				self.nodes.push(None);
				next_index
			}
		};
		self.references.push(node_index);

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

		Ok(())
	}

	// TODO: does this need the full field, or just the field kind?
	// TODO: return type. probably needs a value enum.
	fn read_value(&mut self, field: &Field) -> Result<Value> {
		match &field.kind {
			FieldKind::String => Ok(Value::String(self.read_string()?)),
			FieldKind::Vector(inner_kind) => {
				let count = usize::try_from(self.read_i32()?).unwrap();
				let values = self.read_value_vector(&*inner_kind, count)?;
				Ok(Value::Vector(values))
			}
			other => todo!("Unhandled field kind {other:?}."),
		}
	}

	fn read_value_node(&mut self) -> Result<usize> {
		if self.version < 2 {
			todo!("Sub-v2 node values.")
		}

		let reference_index = usize::try_from(self.read_i32()?).unwrap();

		match self.references.get(reference_index) {
			Some(index) => Ok(*index),
			// A referenced node hasn't been read yet - reserve an entry in the node
			// array for it if one does not exist yet, and pre-emptively record that
			// index as a reference.
			None => {
				let reserved_index = self
					.pending_references
					.entry(reference_index)
					.or_insert_with(|| {
						let reserved_index = self.nodes.len();
						self.nodes.push(None);
						reserved_index
					});
				Ok(*reserved_index)
			}
		}
	}

	fn read_value_vector(&mut self, kind: &FieldKind, count: usize) -> Result<Vec<Value>> {
		match kind {
			FieldKind::Integer => {
				if self.version < 3 {
					todo!("Sub-v3 integer vector values.");
				}

				let unknown = self.read_i32()?;
				if unknown != 4 {
					todo!("Recieved unexpected integer vector marker {unknown}.");
				}

				(0..count)
					.map(|_| Ok(Value::Integer(self.read_i32()?)))
					.collect::<Result<Vec<_>>>()
			}

			FieldKind::String => (0..count)
				.map(|_| Ok(Value::String(self.read_string()?)))
				.collect::<Result<Vec<_>>>(),

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
				// TODO: This is similar to logic in read_node - deduplicate?
				let fields = definition.fields();
				let stored_fields = self.read_bitfield(fields.len())?;
				let values = fields
					.into_iter()
					.zip(stored_fields.into_iter())
					.filter(|(_, stored)| *stored)
					.map(|(field, _)| self.read_value_vector(&field.kind, count))
					.collect::<Vec<_>>();

				// TODO Push nodes onto the node array
				Ok((0..count).map(|_| Value::Node(usize::MAX)).collect())
			}

			FieldKind::Reference(..) => (0..count)
				.map(|_| Ok(Value::Node(self.read_value_node()?)))
				.collect::<Result<Vec<_>>>(),

			other => todo!("Unhandled array kind {kind:?}"),
		}
	}
}

#[derive(Debug)]
enum Value {
	Integer(i32),
	String(String),
	Node(usize),
	Vector(Vec<Value>),
}

#[derive(Debug)]
pub struct Node {
	// TODO: store fields
}
