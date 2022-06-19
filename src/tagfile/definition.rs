use std::{io::Read, rc::Rc};

use crate::error::{Error, Result};

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_definition(&mut self) -> Result<Rc<Definition>> {
		let name = self.read_string()?;
		let version = self.read_i32()?;

		let parent_index = usize::try_from(self.read_i32()?).unwrap();
		let parent = self.definitions[parent_index].clone();

		let field_count = self.read_i32()?;
		let fields = (0..field_count)
			.map(|_index| self.read_field())
			.collect::<Result<Vec<_>>>()?;

		let definition = Rc::new(Definition {
			name,
			version,
			parent,
			fields,
		});

		self.definitions.push(Some(definition.clone()));
		Ok(definition)
	}

	fn read_field(&mut self) -> Result<Field> {
		let name = self.read_string()?;
		let kind = self.read_kind()?;

		Ok(Field { name, kind })
	}

	fn read_kind(&mut self) -> Result<FieldKind> {
		// First value contains the base field kind as well as some metadata.
		let kind_data = self.read_i32()?;
		let base_kind = kind_data & 0xF;
		let is_array = (kind_data & 0x10) != 0;
		let is_tuple = (kind_data & 0x20) != 0;

		// Tuples unhelpfully have their size before anything else.
		let tuple_size = match is_tuple {
			true => usize::try_from(self.read_i32()?).unwrap(),
			false => 0,
		};

		// Map to the base field kind.
		let mut field_kind = match base_kind {
			0x0 => FieldKind::Void,
			0x1 => FieldKind::Byte,
			0x2 => FieldKind::Integer,
			0x3 => FieldKind::Float,
			0x4 => FieldKind::Tuple(FieldKind::Float.into(), 4),
			0x5 => FieldKind::Tuple(FieldKind::Float.into(), 8),
			0x6 => FieldKind::Tuple(FieldKind::Float.into(), 12),
			0x7 => FieldKind::Tuple(FieldKind::Float.into(), 16),
			0x8 => FieldKind::Reference(self.read_string()?),
			0x9 => FieldKind::Struct(self.read_string()?),
			0xA => FieldKind::String,
			other => {
				return Err(Error::Invalid(format!(
					"Unexpected base field kind {other}"
				)))
			}
		};

		// Wrap the field kind in container kinds if appropriate.
		if is_tuple {
			field_kind = FieldKind::Tuple(field_kind.into(), tuple_size);
		} else if is_array {
			field_kind = FieldKind::Array(field_kind.into());
		}

		Ok(field_kind)
	}
}

// TODO: These structs might make sense outside the immediate context of tagfiles, lift out?
#[derive(Debug)]
pub struct Definition {
	name: String,
	version: i32,
	// TODO: Not super happy with the Rc here, though it's relatively ergonomic...
	parent: Option<Rc<Definition>>,
	fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Field {
	name: String,
	kind: FieldKind,
}

#[derive(Debug)]
pub enum FieldKind {
	Void,
	Byte,
	Float,
	Integer,
	String,
	Struct(String),
	Reference(String),
	Array(Box<FieldKind>),
	Tuple(Box<FieldKind>, usize),
}
