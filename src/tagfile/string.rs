use std::io::Read;

use crate::error::{Error, Result};

use super::{i32::read_i32, tagfile::Tagfile};

impl<R: Read> Tagfile<R> {
	#[inline]
	pub fn read_string(&mut self) -> Result<String> {
		read_string(&mut self.reader, &mut self.strings)
	}
}

pub fn read_string(reader: &mut impl Read, strings: &mut Vec<Option<String>>) -> Result<String> {
	let length = read_i32(reader)?;

	// Negative lengths are interpreted as an index into the string cache.
	if length <= 0 {
		let index = usize::try_from(-length).unwrap();
		let string = strings[index]
			.as_ref()
			.unwrap_or_else(|| panic!("Unhandled None string at index {index}."));
		return Ok(string.clone());
	}

	// Otherwise, it's raw string data in the file - read it and cache.
	let mut buffer = vec![];
	reader
		.by_ref()
		.take(length.try_into().unwrap())
		.read_to_end(&mut buffer)?;
	let string = String::from_utf8(buffer)
		.map_err(|error| Error::Invalid(format!("Failed to parse string from buffer: {error}.")))?;

	strings.push(Some(string));
	Ok(strings.last().unwrap().as_ref().unwrap().clone())
}

#[cfg(test)]
mod test {
	use std::io::Cursor;

	use super::read_string;

	fn read(input: &[u8], mut cache: Vec<Option<String>>) -> (String, Vec<Option<String>>) {
		(
			read_string(&mut Cursor::new(input), &mut cache).unwrap(),
			cache,
		)
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
