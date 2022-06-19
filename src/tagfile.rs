use std::io::Read;

use crate::error::Result;

fn read_i32(input: &mut impl Read) -> Result<i32> {
	// Read first byte with sign bit.
	let mut byte = read_u8(input)?;
	let negative = byte & 1 == 1;
	let mut value = i32::from(byte >> 1) & 0x7FFFFFBF;

	// Continue reading bytes while the continuation bit is set.
	let mut shift = 6;
	while (byte & 0x80) != 0 {
		byte = read_u8(input)?;
		value |= i32::from(byte & 0x7F) << shift;
		shift += 7;
	}

	// Once read, negate if the bit was set.
	if negative {
		value = -value;
	}

	Ok(value)
}

fn read_u8(input: &mut impl Read) -> Result<u8> {
	let mut buffer = [0u8; 1];
	input.read_exact(&mut buffer)?;
	let value = u8::from_le_bytes(buffer);
	Ok(value)
}

#[cfg(test)]
mod test {
	use std::io::Cursor;

	use super::read_i32;

	fn read(input: &[u8]) -> i32 {
		read_i32(&mut Cursor::new(input)).unwrap()
	}

	#[test]
	fn zero() {
		let value = read(&[0]);
		assert_eq!(value, 0);
	}

	#[test]
	fn one() {
		let value = read(&[2]);
		assert_eq!(value, 1);
	}

	#[test]
	fn one_negative() {
		let value = read(&[3]);
		assert_eq!(value, -1);
	}

	#[test]
	fn large() {
		let value = read(&[0xFE, 0xFF, 0x7F]);
		assert_eq!(value, 1048575);
	}

	#[test]
	fn large_negative() {
		let value = read(&[0xFF, 0xFF, 0x7F]);
		assert_eq!(value, -1048575);
	}
}
