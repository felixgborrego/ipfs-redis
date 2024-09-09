use std::sync::Arc;

use crate::error::{Error, Result};
use crate::protocol::Data;
use crate::storage::Db;

/// Process the Ping command
pub fn ping_execute() -> Data {
    Data::SimpleString("PONG".to_string())
}

pub fn echo_execute(args: &[Data]) -> Result<Data> {
    let Some(Data::BulkString(data)) = args.first() else {
        return Err(Error::InvalidResp);
    };

    Ok(Data::BulkString(data.to_string()))
}
pub fn command_execute(_: &[Data]) -> Data {
    Data::BulkString(String::new())
}

pub fn info_execute(args: &[Data], state: &Arc<Db>) -> Result<Data> {
    let [_, ..] = args else {
        return Err(Error::ArgsMissing(
            "arg missing in info command".to_string(),
        ));
    };

    let info = state.info().replication.to_string();
    Ok(Data::BulkString(info))
}
pub fn config_execute(args: &[Data], state: &Arc<Db>) -> Result<Data> {
    let [Data::BulkString(sub_cmd), Data::BulkString(config_key), ..] = args else {
        return Err(Error::InvalidResp);
    };

    let config_value = match (sub_cmd.to_uppercase().as_str(), config_key.as_str()) {
        ("GET", "dir") => Ok(state.config.dir.clone()),
        ("GET", "dbfilename") => Ok(state.config.dbfilename.clone()),
        other => Err(Error::Unsupported(format!(
            "Unsupported Config sub command {other:?}"
        ))),
    }?;

    Ok(config_value.map_or(Data::NullBuilkString, |v| {
        Data::Array(vec![
            Data::BulkString(config_key.clone()),
            Data::BulkString(v),
        ])
    }))
}
