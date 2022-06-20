use std::io::Read;

use crate::error::{Error, Result};

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_bitfield(&mut self, count: usize) -> Result<Vec<bool>> {
		// Read enough bytes to cover the requested bitfield count.
		let bytes = (count + 7) / 8;
		let mut buffer = vec![];
		self.reader
			.by_ref()
			.take(bytes.try_into().unwrap())
			.read_to_end(&mut buffer)?;

		// Translate into boolean vector.
		let mut bitfield = buffer
			.into_iter()
			.flat_map(|byte| {
				(0..8).map(move |index| {
					let compare = 1 << index;
					byte & compare == compare
				})
			})
			.collect::<Vec<_>>();

		// Sanity check that there's no set bits outside the expected count.
		if bitfield.iter().skip(count).any(|value| *value) {
			return Err(Error::Invalid(
				"Found unexpected bit set after count in bitfield.".into(),
			));
		}

		// Limit the result to the requested count.
		bitfield.truncate(count);
		Ok(bitfield)
	}
}

#[cfg(test)]
mod test {
	use std::io::Cursor;

	use crate::tagfile::tagfile::Tagfile;

	fn read(input: &[u8], count: usize) -> Vec<bool> {
		let mut tagfile = Tagfile::new(Cursor::new(input));
		tagfile.read_bitfield(count).unwrap()
	}

	#[test]
	fn simple() {
		assert_eq!(
			read(&[1], 8),
			[true, false, false, false, false, false, false, false]
		);
	}

	#[test]
	fn mixed() {
		assert_eq!(
			read(&[170], 8),
			[false, true, false, true, false, true, false, true]
		);
	}

	#[test]
	fn multiple_bytes() {
		assert_eq!(
			read(&[1, 1], 16),
			[
				true, false, false, false, false, false, false, false, true, false, false, false,
				false, false, false, false
			]
		);
	}

	#[test]
	fn truncated() {
		assert_eq!(read(&[2], 2), [false, true]);
	}

	#[test]
	#[should_panic = "Found unexpected bit set after count in bitfield."]
	fn invalid() {
		read(&[2], 1);
	}
}
