use std::{io::Read, rc::Rc};

use crate::error::Result;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_definition(&mut self) -> Result<Rc<Definition>> {
		let name = self.read_string()?;
		let version = self.read_i32()?;

		let parent_index = usize::try_from(self.read_i32()?).unwrap();
		let parent = self.definitions[parent_index].clone();

		let definition = Rc::new(Definition {
			name,
			version,
			parent,
		});

		self.definitions.push(Some(definition.clone()));
		Ok(definition)
	}
}

// TODO: Definitions might make sense outside the immediate context of tagfiles, lift out?
#[derive(Debug)]
pub struct Definition {
	name: String,
	version: i32,
	// TODO: Not super happy with the Rc here, though it's relatively ergonomic...
	parent: Option<Rc<Definition>>,
}
