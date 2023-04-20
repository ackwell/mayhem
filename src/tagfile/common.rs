use std::io::Read;

use crate::error::Result;
use crate::macros::read_primitive;

use super::tagfile::Tagfile;

impl<R: Read> Tagfile<R> {
	read_primitive!(u64, read_u64);
	read_primitive!(u8, read_u8);
	read_primitive!(f32, read_f32);
}
