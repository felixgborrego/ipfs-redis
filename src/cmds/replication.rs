use base64::prelude::*;
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{error::Result, protocol::Data, storage::Db};

pub fn replconf_execute(_: &[Data], _: &Arc<Db>) -> Data {
    Data::SimpleString("OK".to_string())
}

pub fn psync_execute(_: &[Data], database: &Arc<Db>) -> Result<Data> {
    let client_id = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let data = database
        .config
        .db_path()
        .map_or_else(|| Ok(empty_db()), read_db)?;

    Ok(Data::FullResyncBinaryConent(
        Box::new(Data::SimpleString(format!(
            "FULLRESYNC {} 0",
            client_id.as_millis()
        ))),
        data,
    ))
}

fn read_db(path: PathBuf) -> Result<Vec<u8>> {
    // Not a good idea to load the file in memory, but fine for this poc.
    // TODO do a on the fly writer
    let mut file_reader = File::open(path)?;
    let mut buf = Vec::<u8>::new();
    file_reader.read_to_end(&mut buf)?;
    Ok(buf)
}

const EMPTY_DB_BASE64:&str = "UkVESVMwMDEx+glyZWRpcy12ZXIFNy4yLjD6CnJlZGlzLWJpdHPAQPoFY3RpbWXCbQi8ZfoIdXNlZC1tZW3CsMQQAPoIYW9mLWJhc2XAAP/wbjv+wP9aog==";

fn empty_db() -> Vec<u8> {
    BASE64_STANDARD.decode(EMPTY_DB_BASE64).unwrap()
}
