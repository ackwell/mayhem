use std::{fmt, rc::Rc};

use crate::{
	error::{Error, Result},
	node::{Definition, Node},
	value::Value,
};

/// View into a collection of nodes.
#[derive(Clone)]
pub struct NodeWalker {
	pub(super) nodes: Rc<Vec<Node>>,
	pub(super) index: usize,
}

macro_rules! nodewalker_field_typed_ref {
	($type:ty, $fn_name:ident, $enum_pattern:ident, $docstr:expr) => {
		#[doc="Read the value of the specified field as "]
		#[doc=$docstr]
		#[doc="."]
		pub fn $fn_name<'a>(&'a self, field_name: &str, default_value: Option<&'a $type>) -> Result<&'a $type> {
			match self.field(field_name) {
				Some(value) => match value {
					Value::$enum_pattern(x) => Ok(x),
					_ => Err(Error::Invalid(
						format!("Field {0} has an invalid type.", field_name).into()
					)),
				},
				None => match default_value {
					Some(value) => Ok(value),
					None => Err(Error::Invalid(
						format!("Field {0} is missing.", field_name).into()
					))
				},
			}
		}
	}
}

macro_rules! nodewalker_field_typed_copy {
	($type:ty, $fn_name:ident, $enum_pattern:ident, $docstr:expr) => {
		#[doc="Read the value of the specified field as "]
		#[doc=$docstr]
		#[doc="."]
		pub fn $fn_name(&self, field_name: &str, default_value: Option<$type>) -> Result<$type> {
			match self.field(field_name) {
				Some(value) => match value {
					Value::$enum_pattern(x) => Ok(*x),
					_ => Err(Error::Invalid(
						format!("Field {0} has an invalid type.", field_name).into()
					)),
				},
				None => match default_value {
					Some(value) => Ok(value),
					None => Err(Error::Invalid(
						format!("Field {0} is missing.", field_name).into()
					))
				},
			}
		}
	}
}

macro_rules! nodewalker_field_typed_vec_ref {
	($type:ty, $fn_name:ident, $enum_pattern:ident, $docstr:expr) => {
		#[doc="Read the value of the specified field as "]
		#[doc=$docstr]
		#[doc="."]
		pub fn $fn_name<'a>(&'a self, field_name: &str, default_value: Option<Vec<&'a $type>>) -> Result<Vec<&'a $type>> {
			match self.field(field_name) {
				Some(value) => match value {
					Value::Vector(x) => x.iter()
						.map(|x| match x {
							Value::$enum_pattern(y) => Ok(y),
							_ => Err(Error::Invalid(
								format!("Field {0} has an invalid type in array.", field_name).into()
							)),
						})
						.collect(),
					_ => Err(Error::Invalid(
						format!("Field {0} has an invalid type.", field_name).into()
					)),
				},
				None => match default_value {
					Some(value) => Ok(value),
					None => Err(Error::Invalid(
						format!("Field {0} is missing.", field_name).into()
					))
				},
			}
		}
	}
}

macro_rules! nodewalker_field_typed_vec_copy {
	($type:ty, $fn_name:ident, $enum_pattern:ident, $docstr:expr) => {
		#[doc="Read the value of the specified field as "]
		#[doc=$docstr]
		#[doc="."]
		pub fn $fn_name(&self, field_name: &str, default_value: Option<Vec<$type>>) -> Result<Vec<$type>> {
			match self.field(field_name) {
				Some(value) => match value {
					Value::Vector(x) => x.iter()
						.map(|x| match x {
							Value::$enum_pattern(y) => Ok(*y),
							_ => Err(Error::Invalid(
								format!("Field {0} has an invalid type in array.", field_name).into()
							)),
						})
						.collect(),
					_ => Err(Error::Invalid(
						format!("Field {0} has an invalid type.", field_name).into()
					)),
				},
				None => match default_value {
					Some(value) => Ok(value),
					None => Err(Error::Invalid(
						format!("Field {0} is missing.", field_name).into()
					))
				},
			}
		}
	};
}

impl NodeWalker {
	/// Get a walker instance for the requested node index.
	pub fn node(&self, index: usize) -> NodeWalker {
		// TODO: sanity check index.
		NodeWalker {
			nodes: self.nodes.clone(),
			index,
		}
	}

	fn current(&self) -> &Node {
		&self.nodes[self.index]
	}

	/// Get the current node's struct name.
	pub fn name(&self) -> &str {
		&self.current().definition.name
	}

	/// Get the current nodes's struct version.
	pub fn version(&self) -> i32 {
		self.current().definition.version
	}

	/// Check if current node is an instance of specified type (definition).
	pub fn is_or_inherited_from(&self, definition_name: &str) -> bool {
		let mut d = &self.current().definition;
		loop {
			if d.name == definition_name {
				return true;
			}

			match &d.parent {
				Some(x) => d = x,
				None => return false
			}
		}
	}

	fn field_impl(
		&self,
		field_name: &str,
		definition: &Rc<Definition>,
		field_index: &mut usize,
		value_index: &mut usize,
	) -> Option<&Value> {
		if definition.parent.is_some() {
			let r = self.field_impl(
				field_name,
				definition.parent.as_ref().unwrap(),
				field_index,
				value_index);
			if r.is_some() {
				return r;
			}
		}

		let node = self.current();
		for i in 0..definition.fields.len() {
			if definition.fields[i].name == field_name {
				return match node.field_mask[*field_index] {
					true => Some(&node.values[*value_index]),
					_ => None,
				};
			} else {
				if node.field_mask[*field_index] {
					*value_index += 1;
				}
			}

			*field_index += 1;
		}

		None
	}

	/// Get the value of the specified field.
	pub fn field(&self, field_name: &str) -> Option<&Value> {
		let mut field_index: usize = 0;
		let mut value_index: usize = 0;
		self.field_impl(field_name, &self.current().definition, &mut field_index, &mut value_index)
	}

	nodewalker_field_typed_copy!(u8, field_u8, U8, "a u8");
	nodewalker_field_typed_vec_copy!(u8, field_u8_vec, U8, "a Vec<u8>");
	nodewalker_field_typed_copy!(i32, field_i32, I32, "an i32");
	nodewalker_field_typed_vec_copy!(i32, field_i32_vec, I32, "a Vec<i32>");
	nodewalker_field_typed_copy!(f32, field_f32, F32, "a f32");
	nodewalker_field_typed_vec_copy!(f32, field_f32_vec, F32, "a Vec<f32>");
	nodewalker_field_typed_ref!(String, field_string, String, "a &String");
	nodewalker_field_typed_vec_ref!(String, field_string_vec, String, "a Vec<&String>");
	nodewalker_field_typed_ref!(Vec<Value>, field_vec, Vector, "a &Vec<Value>");
	nodewalker_field_typed_vec_ref!(Vec<Value>, field_vec_vec, Vector, "a Vec<&Vec<Value>>");

	/// Read the value of the specified field as a NodeWalker.
	pub fn field_node(&self, field_name: &str) -> Result<NodeWalker> {
		match self.field(field_name) {
			Some(value) => match value {
				Value::Node(x) => Ok(self.node(*x)),
				_ => Err(Error::Invalid(
					format!("Field {0} has an invalid type.", field_name).into()
				)),
			},
			None => Err(Error::Invalid(
				format!("Field {0} is missing.", field_name).into()
			)),
		}
	}

	/// Read the value of the specified field as a Vec<NodeWalker>.
	pub fn field_node_vec(&self, field_name: &str) -> Result<Vec<NodeWalker>> {
		match self.field(field_name) {
			Some(value) => match value {
				Value::Vector(x) => x.iter()
					.map(|x| match x {
						Value::Node(y) => Ok(self.node(*y)),
						_ => Err(Error::Invalid(
							format!("Field {0} has an invalid type in array.", field_name).into()
						)),
					})
					.collect(),
				_ => Err(Error::Invalid(
					format!("Field {0} has an invalid type.", field_name).into()
				)),
			},
			None => Err(Error::Invalid(
				format!("Field {0} is missing.", field_name).into()
			)),
		}
	}
}

impl fmt::Debug for NodeWalker {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("NodeWalker")
			.field("index", &self.index)
			.field("node", &self.nodes[self.index])
			.finish()
	}
}
