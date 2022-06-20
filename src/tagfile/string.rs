use std::io::Read;

use crate::error::{Error, Result};

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	#[inline]
	pub fn read_string(&mut self) -> Result<String> {
		let length = self.read_i32()?;

		// Negative lengths are interpreted as an index into the string cache.
		if length <= 0 {
			let index = usize::try_from(-length).unwrap();
			let string = self.strings[index]
				.as_ref()
				.unwrap_or_else(|| panic!("Unhandled None string at index {index}."));
			return Ok(string.clone());
		}

		// Otherwise, it's raw string data in the file - read it and cache.
		let mut buffer = vec![];
		self.reader
			.by_ref()
			.take(length.try_into().unwrap())
			.read_to_end(&mut buffer)?;
		let string = String::from_utf8(buffer).map_err(|error| {
			Error::Invalid(format!("Failed to parse string from buffer: {error}."))
		})?;

		self.strings.push(Some(string));
		Ok(self.strings.last().unwrap().as_ref().unwrap().clone())
	}
}

#[cfg(test)]
mod test {
	use std::io::Cursor;

	use crate::tagfile::tagfile::Tagfile;

	fn read(input: &[u8], cache: Vec<Option<String>>) -> (String, Vec<Option<String>>) {
		let mut tagfile = Tagfile::new(Cursor::new(input));
		tagfile.strings = cache;
		(tagfile.read_string().unwrap(), tagfile.strings)
	}

	#[test]
	fn hello() {
		assert_eq!(
			read(&[10, 104, 101, 108, 108, 111], vec![]),
			("hello".into(), vec![Some("hello".into())])
		)
	}

	#[test]
	fn from_cache() {
		assert_eq!(
			read(
				&[3],
				vec![Some("not this one".into()), Some("this one".into())]
			),
			(
				"this one".into(),
				vec![Some("not this one".into()), Some("this one".into())]
			)
		)
	}
}
