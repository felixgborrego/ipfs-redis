use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::error::{Error, Result};
use crate::protocol::Data;
use crate::storage::Db;

/// Implmement set command as descibed here <https://redis.io/docs/latest/commands/set/>
pub fn set_execute(args: &[Data], state: &Arc<Db>) -> Result<Data> {
    let [key, value, ..] = args else {
        return Err(Error::Unsupported(format!("Unexpected set args {args:?}")));
    };

    let key: &str = key.try_into()?;
    let value: &str = value.try_into()?;

    let set_arts = args.get(2).zip(args.get(3));
    let expiration = set_arts.map_or(Ok(None), parse_set_args)?;

    state.set(key, value, expiration);
    Ok(Data::ok_response())
}

/// return the valu stored in the key
/// if the key is missing, GET command should return "null build string"
pub fn get_execute(args: &[Data], state: &Arc<Db>) -> Result<Data> {
    let [key, ..] = args else {
        return Err(Error::Unsupported(format!("Unexpected get args {args:?}")));
    };

    let key: &str = key.try_into()?;
    Ok(state
        .get(key)
        .map_or(Data::NullBuilkString, Data::BulkString))
}

pub fn keys_execute(args: &[Data], state: &Arc<Db>) -> Result<Data> {
    let [patern, ..] = args else {
        return Err(Error::Unsupported(format!("Unexpected get args {args:?}")));
    };
    let patern: &str = patern.try_into()?;
    tracing::debug!("executing: keys {patern}");
    let keys = state.keys(patern);
    Ok(Data::Array(
        keys.into_iter().map(Data::BulkString).collect(),
    ))
}

fn parse_set_args(args: (&Data, &Data)) -> Result<Option<SystemTime>> {
    match args {
        (Data::BulkString(ex_cmd), Data::BulkString(ms)) => {
            if ex_cmd.to_ascii_uppercase() == "PX" {
                let ms: u64 = ms.parse()?;
                let expiration = SystemTime::now() + Duration::from_millis(ms);
                Ok(Some(expiration))
            } else {
                Err(Error::Unsupported(format!(
                    "unsupported set arts {ex_cmd:?}"
                )))
            }
        }
        other => Err(Error::Unsupported(format!(
            "unsupported set arts {other:?}"
        ))),
    }
}
