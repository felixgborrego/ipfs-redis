use derive_more::{derive::Display, From};
use std::{string::FromUtf8Error, time::SystemTimeError};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From, Display)]
pub enum Error {
    InvalidResp,
    ArgsMissing(String),
    InvalidRdb(String),

    // Externals
    #[from]
    IO(std::io::Error),
    #[from]
    Parser(std::num::ParseIntError),
    #[from]
    Unsupported(String),
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Error::InvalidRdb(format!("Invalid utf8 - {e}"))
    }
}

impl From<SystemTimeError> for Error {
    fn from(err: SystemTimeError) -> Error {
        Error::Unsupported(format!("Invalid - {err}"))
    }
}

impl std::error::Error for Error {}
