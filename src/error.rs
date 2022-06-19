use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
	#[error("{0}")]
	IO(#[from] io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
