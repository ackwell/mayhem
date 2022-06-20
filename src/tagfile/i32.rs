use std::io::Read;

use crate::error::Result;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	pub fn read_i32(&mut self) -> Result<i32> {
		// Read first byte with sign bit.
		let mut byte = self.read_u8()?;
		let negative = byte & 1 == 1;
		let mut value = i32::from(byte >> 1) & 0x7FFFFFBF;

		// Continue reading bytes while the continuation bit is set.
		let mut shift = 6;
		while (byte & 0x80) != 0 {
			byte = self.read_u8()?;
			value |= i32::from(byte & 0x7F) << shift;
			shift += 7;
		}

		// Once read, negate if the bit was set.
		if negative {
			value = -value;
		}

		Ok(value)
	}
}

#[cfg(test)]
mod test {
	use std::io::Cursor;

	use crate::tagfile::tagfile::Tagfile;

	fn read(input: &[u8]) -> i32 {
		let mut tagfile = Tagfile::new(Cursor::new(input));
		tagfile.read_i32().unwrap()
	}

	#[test]
	fn zero() {
		assert_eq!(read(&[0]), 0);
	}

	#[test]
	fn one() {
		assert_eq!(read(&[2]), 1);
	}

	#[test]
	fn one_negative() {
		assert_eq!(read(&[3]), -1);
	}

	#[test]
	fn large() {
		assert_eq!(read(&[0xFE, 0xFF, 0x7F]), 1048575);
	}

	#[test]
	fn large_negative() {
		assert_eq!(read(&[0xFF, 0xFF, 0x7F]), -1048575);
	}
}
