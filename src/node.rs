use std::rc::Rc;

use crate::value::Value;

#[derive(Debug)]
pub struct Node {
	pub definition: Rc<Definition>,
	pub field_mask: Vec<bool>,
	pub values: Vec<Value>,
}

#[derive(Debug)]
pub struct Definition {
	pub name: String,
	pub version: i32,
	// TODO: Not super happy with the Rc here, though it's relatively ergonomic...
	pub parent: Option<Rc<Definition>>,
	pub fields: Vec<Field>,
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
	pub name: String,
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
	Vector(Box<FieldKind>),
	Array(Box<FieldKind>, usize),
}
