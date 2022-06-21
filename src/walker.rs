use std::{fmt, rc::Rc};

use crate::node::{Field, Node, Value};

/// View into a collection of nodes.
pub struct NodeWalker {
	pub(super) nodes: Rc<Vec<Node>>,
	pub(super) index: usize,
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

	fn iter_fields(&self) -> impl Iterator<Item = (&Field, Option<usize>)> {
		let current = self.current();
		let mask_indexes = current.field_mask.iter().scan(0usize, |index, mask| {
			Some(match mask {
				true => {
					let id = *index;
					*index += 1;
					Some(id)
				}
				false => None,
			})
		});

		std::iter::zip(current.definition.fields(), mask_indexes)
	}

	/// Get the value of the specified field.
	pub fn field(&self, name: &str) -> Option<&Value> {
		self.iter_fields()
			.find(|(field, _index)| field.name == name)
			.and_then(|(_field, index)| index.map(|index| &self.current().values[index]))
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
