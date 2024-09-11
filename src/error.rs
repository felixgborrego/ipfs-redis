use derive_more::{derive::Display, From};
use std::{convert::Infallible, string::FromUtf8Error, time::SystemTimeError};

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
    #[from]
    P2pError(libp2p::noise::Error),
    #[from]
    P2pTransportError(libp2p::TransportError<std::io::Error>),
    #[from]
    P2pSwarmError(libp2p::swarm::DialError),
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

// Sinte this not happen it's safe to just implement this way
impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl std::error::Error for Error {}
