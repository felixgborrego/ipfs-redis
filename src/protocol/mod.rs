use crate::error::Error;
use crate::error::Result;
mod parser;
mod write;

/// resp supprted as described here <https://redis.io/docs/latest/develop/reference/protocol-spec/#resp-protocol-description>
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Data {
    ConnectionClosed,
    SimpleString(String),
    BulkString(String),
    NullBuilkString,
    Array(Vec<Data>),
    FullResyncBinaryConent(Box<Data>, Vec<u8>),
}

impl Data {
    pub fn ok_response() -> Data {
        Data::SimpleString(String::from("OK"))
    }
}

#[derive(Debug, Clone)]
pub enum Cmd {
    ConnectionClosed, // Client close the connection
    Ping,
    Echo { args: Vec<Data> },
    Set { args: Vec<Data> },
    Get { args: Vec<Data> },
    Config { args: Vec<Data> },
    Command { args: Vec<Data> },
    Keys { args: Vec<Data> },
    Info { args: Vec<Data> },
    // Replication related commands
    Replconf { args: Vec<Data> },
    Psync { args: Vec<Data> },
}

impl<'a> TryFrom<&'a Data> for &'a str {
    type Error = Error;

    fn try_from(value: &'a Data) -> core::result::Result<Self, Self::Error> {
        match value {
            Data::BulkString(s) | Data::SimpleString(s) => Ok(s.as_str()),
            invalid => Err(Error::Unsupported(format!(
                "Unable to extract string from unsupported {invalid:?}"
            ))),
        }
    }
}

impl Cmd {
    pub fn from_str_args(cmd_str: &str, args: Vec<Data>) -> Result<Cmd> {
        match cmd_str.to_ascii_uppercase().as_str() {
            "ECHO" => Ok(Cmd::Echo { args }),
            "PING" => Ok(Cmd::Ping),
            "SET" => Ok(Cmd::Set { args }),
            "GET" => Ok(Cmd::Get { args }),
            "CONFIG" => Ok(Cmd::Config { args }),
            "COMMAND" => Ok(Cmd::Command { args }),
            "KEYS" => Ok(Cmd::Keys { args }),
            "INFO" => Ok(Cmd::Info { args }),
            "REPLCONF" => Ok(Cmd::Replconf { args }),
            "PSYNC" => Ok(Cmd::Psync { args }),
            c => Err(Error::Unsupported(c.to_string())),
        }
    }

    pub fn to_data(&self) -> Result<Data> {
        match self {
            Cmd::Ping => Ok(Data::Array(vec![Data::BulkString("PING".to_string())])),
            Cmd::Replconf { args } => {
                let mut data = vec![Data::BulkString("REPLCONF".to_string())];

                args.iter().for_each(|i| data.push(i.clone()));
                Ok(Data::Array(data))
            }

            Cmd::Psync { args } => {
                let mut data = vec![Data::BulkString("PSYNC".to_string())];

                args.iter().for_each(|a| data.push(a.clone()));
                Ok(Data::Array(data))
            }

            Cmd::Set { args } => {
                let mut data = vec![Data::BulkString("SET".to_string())];
                args.iter().for_each(|a| data.push(a.clone()));
                Ok(Data::Array(data))
            }
            other => Err(Error::Unsupported(format!(
                "Invalid command {other:?} to encode"
            ))),
        }
    }

    pub fn is_write(&self) -> bool {
        matches!(self, Cmd::Set { .. })
    }
}
