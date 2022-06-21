use std::{io::Read, rc::Rc};

use crate::error::{Error, Result};

use super::{
	definition::{Definition, Field, FieldKind},
	tagfile::Tagfile,
};

impl<R: Read> Tagfile<R> {
	// TODO: what's the return type going to look like here? For consistency, it should probably act like a reference?
	pub fn read_node(
		&mut self,
		definition: Option<Rc<Definition>>,
		store_reference: bool,
	) -> Result<usize> {
		// Default to storing the node at the end of the node array.
		let mut node_index = self.nodes.len();

		// If storing a reference, check if it's already been requested. If it has,
		// we can use the pre-reserved node index rather than adding a new one.
		if store_reference {
			let reference_index = self.references.len();
			if let Some((_key, value)) = self.pending_references.remove_entry(&reference_index) {
				node_index = value;
			}
			self.references.push(node_index);
		}

		// If the node is still intended to be placed at the end, reserve a position for it.
		if node_index == self.nodes.len() {
			self.nodes.push(None);
		}

		// Read & resolve the definition for this node, if one has not been provided.
		let definition = match definition {
			Some(definition) => definition,
			None => {
				let definition_index = usize::try_from(self.read_i32()?).unwrap();
				self.definitions
					.get(definition_index)
					.and_then(|found| found.clone())
					.ok_or_else(|| {
						Error::Invalid(format!("Missing definition at index {definition_index}."))
					})?
			}
		};

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

		self.nodes[node_index] = Some(Node { definition, values });

		Ok(node_index)
	}

	// TODO: does this need the full field, or just the field kind?
	fn read_value(&mut self, field: &Field) -> Result<Value> {
		match &field.kind {
			FieldKind::Byte => Ok(Value::U8(self.read_u8()?)),

			FieldKind::Integer => Ok(Value::I32(self.read_i32()?)),

			FieldKind::String => Ok(Value::String(self.read_string()?)),

			FieldKind::Struct(name) => {
				if self.version < 2 {
					todo!("Sub-v2 struct values.");
				}

				// Look up the definition for the struct by name.
				let definition = self
					.definitions
					.iter()
					.flatten()
					.find(|definition| &definition.name == name)
					.ok_or_else(|| Error::Invalid(format!("Missing definition for {name}")))?
					.clone();

				Ok(Value::Node(self.read_node(Some(definition), false)?))
			}

			FieldKind::Reference(..) => Ok(Value::Node(self.read_value_node()?)),

			FieldKind::Vector(inner_kind) => {
				let count = usize::try_from(self.read_i32()?).unwrap();
				let values = self.read_value_vector(&*inner_kind, count)?;
				Ok(Value::Vector(values))
			}

			kind @ FieldKind::Array(inner, count) => {
				if !matches!(**inner, FieldKind::Float) || !matches!(count, 4 | 8 | 12 | 16) {
					return Err(Error::Invalid(format!("Unexpected array kind {kind:?}.")));
				}

				let values = (0..*count)
					.map(|_| Ok(Value::F32(self.read_f32()?)))
					.collect::<Result<Vec<_>>>()?;

				Ok(Value::Vector(values))
			}

			other => todo!("Unhandled field kind {other:?}."),
		}
	}

	fn read_value_node(&mut self) -> Result<usize> {
		if self.version < 2 {
			todo!("Sub-v2 node values.");
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
					.map(|_| Ok(Value::I32(self.read_i32()?)))
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
					.collect::<Result<Vec<_>>>()?;

				// Collate the read values into the final vector of nodes.
				let nodes = (0..count)
					.map(|index| {
						let node_index = self.nodes.len();
						self.nodes.push(Some(Node {
							// TODO: Not keen on the clone here but the structure makes it a bit hard. Other options?
							definition: definition.clone(),
							values: values
								.iter()
								.map(|field_values| field_values[index].clone())
								.collect::<Vec<_>>(),
						}));
						Value::Node(node_index)
					})
					.collect::<Vec<_>>();

				Ok(nodes)
			}

			// TODO: Can I use the definition name in the type to sanity check?
			FieldKind::Reference(..) => (0..count)
				.map(|_| Ok(Value::Node(self.read_value_node()?)))
				.collect::<Result<Vec<_>>>(),

			kind @ FieldKind::Array(inner, array_count) => {
				// Vector<Array< only support specific floating point arrays.
				// TODO: Pull this logic out as a helper?
				if !matches!(**inner, FieldKind::Float) || !matches!(array_count, 4 | 8 | 12 | 16) {
					return Err(Error::Invalid(format!(
						"Unexpected vector of array kind {kind:?}."
					)));
				}

				// 4-element arrays can be represented as 3 elements in data.
				let mut final_count = *array_count;
				if final_count == 4 {
					final_count = usize::try_from(self.read_i32()?).unwrap();
					if !matches!(final_count, 3 | 4) {
						return Err(Error::Invalid(format!(
							"Unexpected array length {final_count}."
						)));
					}
				}

				(0..count)
					.map(|_| {
						let array = (0..final_count)
							.map(|_| Ok(Value::F32(self.read_f32()?)))
							.collect::<Result<Vec<_>>>()?;
						Ok(Value::Vector(array))
					})
					.collect::<Result<Vec<_>>>()
			}

			other => todo!("Unhandled vector kind {kind:?}"),
		}
	}
}

#[derive(Clone, Debug)]
enum Value {
	U8(u8),
	I32(i32),
	F32(f32),
	String(String),
	Node(usize),
	Vector(Vec<Value>),
}

#[derive(Debug)]
pub struct Node {
	definition: Rc<Definition>,
	values: Vec<Value>,
}
