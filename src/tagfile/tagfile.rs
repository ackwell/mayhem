use std::{collections::HashMap, io::Read, rc::Rc};

use crate::{
	error::{Error, Result},
	node::{Definition, Node},
	walker::NodeWalker,
};

// TODO: return type
pub fn read(input: &mut impl Read) -> Result<NodeWalker> {
	let mut tagfile = Tagfile::new(input);
	let root_index = tagfile.read()?;
	let nodes = tagfile.nodes.into_iter().flatten().collect();
	Ok(NodeWalker {
		nodes: Rc::new(nodes),
		index: root_index,
	})
}

pub struct Tagfile<R> {
	pub version: i32,

	pub reader: R,

	pub nodes: Vec<Option<Node>>,

	// Caches
	// TODO: The Option<>s here are to support empty case values - but there's realistically very few of those, and it complicates consumption a reasonable amount. Consider alternatives.
	pub definitions: Vec<Option<Rc<Definition>>>,
	pub strings: Vec<Option<String>>,
	pub references: Vec<usize>,
	pub pending_references: HashMap<usize, usize>,
}

impl<R: Read> Tagfile<R> {
	pub fn new(reader: R) -> Self {
		Self {
			version: -1,
			reader,

			nodes: Vec::new(),

			definitions: Vec::from([None]),
			strings: Vec::from([Some("".into()), None]),
			references: Vec::from([usize::MAX]),
			pending_references: HashMap::new(),
		}
	}

	fn read(&mut self) -> Result<usize> {
		let magic = self.read_u64()?;
		if magic != 0xD011FACECAB00D1E {
			// TODO: macro for assets as errors.
			return Err(Error::Invalid(format!("Unexpected magic: {magic:#0x}.")));
		}

		loop {
			let tag = Tag::from(self.read_i32()?);
			match tag {
				Tag::Metadata => {
					self.version = self.read_i32()?;
					if self.version != 3 {
						todo!("Unhandled file version {}.", self.version)
					}
				}

				Tag::Definition => {
					// NOTE: Definitions are currently only referenced after reading via the cache.
					self.read_definition()?;
				}

				Tag::Node => {
					self.read_node(None, true)?;
				}

				Tag::EndOfFile => {
					break;
				}

				#[allow(unreachable_patterns)]
				other => todo!("Unhandled tag kind {other:?}."),
			}
		}

		// Ensure that there's no pending references that weren't fulfilled.
		if !self.pending_references.is_empty() {
			return Err(Error::Invalid(format!(
				"Dangling references remaining at end of file: {:?}.",
				self.pending_references.keys().collect::<Vec<_>>()
			)));
		}

		// Ensure that all reserved nodes were filled.
		if self.nodes.iter().any(|node| node.is_none()) {
			return Err(Error::Invalid(format!(
				"Reserved nodes with no content remaining at end of file: {:?}.",
				self.nodes
					.iter()
					.enumerate()
					.filter_map(|(index, entry)| entry.as_ref().map(|_| index))
					.collect::<Vec<_>>()
			)));
		}

		// The root node will be the first stored reference (ignoring the pre-supplied fake reference).
		let root_index = match self.references.get(1) {
			Some(index) => index,
			None => return Err(Error::Invalid("No root object found.".into())),
		};

		Ok(*root_index)
	}
}

#[derive(Debug)]
enum Tag {
	Metadata,
	Definition,
	Node,
	EndOfFile,
}

impl From<i32> for Tag {
	fn from(value: i32) -> Self {
		match value {
			1 => Self::Metadata,
			2 => Self::Definition,
			4 => Self::Node,
			7 => Self::EndOfFile,
			other => todo!("Unhandled tag kind ID {other}."),
		}
	}
}
