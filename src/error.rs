use std::io;

use thiserror::Error;

/// An error that occured.
#[derive(Error, Debug)]
pub enum Error {
	/// Invalid data or behavior was encountered.
	#[error("Invalid: {0}")]
	Invalid(String),
}

impl From<io::Error> for Error {
	fn from(error: io::Error) -> Self {
		Self::Invalid(error.to_string())
	}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
