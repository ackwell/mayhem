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
	pub name: String,
	version: i32,
	// TODO: Not super happy with the Rc here, though it's relatively ergonomic...
	parent: Option<Rc<Definition>>,
	fields: Vec<Field>,
}

impl Definition {
	// TODO: this generates a bunch of intermediate Vecs, which would be good to avoid.
	pub fn fields(&self) -> Vec<&Field> {
		self.parent
			.iter()
			.flat_map(|definition| definition.fields())
			.chain(self.fields.iter())
			.collect()
	}
}

// TODO: maybe move fields to seperate module?
#[derive(Debug)]
pub struct Field {
	name: String,
	pub kind: FieldKind,
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

#[cfg(test)]
mod test {
	use std::io::Cursor;

	use crate::tagfile::tagfile::Tagfile;

	use super::FieldKind;

	fn read(input: &[u8]) -> FieldKind {
		let mut tagfile = Tagfile::new(Cursor::new(input));
		tagfile.read_kind().unwrap()
	}

	#[test]
	fn field_float() {
		let value = read(&[6]);
		assert!(
			matches!(value, FieldKind::Float),
			"Expected Float, got {value:?}."
		)
	}

	#[test]
	fn field_vector() {
		let value = read(&[12]);
		assert!(
			matches!(value, FieldKind::Tuple(ref inner, 12) if matches!(**inner, FieldKind::Float)),
			"Expected Tuple(Float, 12), got {value:?}."
		)
	}

	#[test]
	fn field_reference() {
		let value = read(&[16, 10, 104, 101, 108, 108, 111]);
		assert!(
			matches!(value, FieldKind::Reference(ref string) if string == "hello"),
			"Expected Reference(\"hello\"), got {value:?}."
		)
	}

	#[test]
	fn field_byte_array() {
		let value = read(&[34]);
		assert!(
			matches!(value, FieldKind::Array(ref inner) if matches!(**inner, FieldKind::Byte)),
			"Expected Array(Byte), got {value:?}."
		)
	}

	#[test]
	fn field_float_tuple() {
		let value = read(&[70, 8]);
		assert!(
			matches!(value, FieldKind::Tuple(ref inner, 4) if matches!(**inner, FieldKind::Float)),
			"Expected Tuple(Float, 4), got {value:?}."
		)
	}
}
