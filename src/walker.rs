use std::{fmt, rc::Rc};

use crate::node::{Field, Node, Value};

pub struct NodeWalker {
	pub(super) nodes: Rc<Vec<Node>>,
	pub(super) index: usize,
}

impl NodeWalker {
	pub fn node(&self, index: usize) -> NodeWalker {
		NodeWalker {
			nodes: self.nodes.clone(),
			index,
		}
	}

	fn current(&self) -> &Node {
		&self.nodes[self.index]
	}

	pub fn name(&self) -> &str {
		&self.current().definition.name
	}

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
