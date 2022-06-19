use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
