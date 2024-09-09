use parser::read_string;

use crate::{
    error::{Error, Result},
    storage::{self, Config},
};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
mod parser;
pub struct Rdb {
    config: Config,
}
const MAGIC_HEADER: &str = "REDIS0011";
// Ops Codec https://rdb.fnordig.de/file_format.html#op-codes
const OP_CODEC_METADATA_SECTION_0XFA: u8 = 0xFA;
const OP_CODEC_SELECT_DB_0XFE: u8 = 0xFE;
const OP_CODEC_RESIZEDB_0XFB: u8 = 0xFB;

const OP_CODEC_EXPIRE_SEC_0XFD: u8 = 0xFD;
const OP_CODEC_EXPIRE_MS_0XFC: u8 = 0xFC;

//https://rdb.fnordig.de/file_format.html#value-type
const OP_CODEC_VALUE_TYPE_STRING_0X00: u8 = 0x00;
const OP_CODEC_END_OF_RDB_0XFF: u8 = 0xFF;

impl Rdb {
    pub fn from(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn load(&mut self) -> Result<storage::Db> {
        let Some(db_path) = self.config.db_path() else {
            tracing::warn!("File db not set, using empty db");
            return Ok(storage::Db::new(self.config.clone()));
        };

        let Ok(mut file) = File::open(&db_path) else {
            tracing::warn!("File not found, using empty db: {db_path:?}");
            return Ok(storage::Db::new(self.config.clone()));
        };

        let in_memory_db = storage::Db::new(self.config.clone());

        tracing::debug!("loading file {file:?}");
        load_from_reader(&mut file, &in_memory_db)?;
        tracing::debug!("Database successfully loaded from file!");

        Ok(in_memory_db)
    }
}
/// read the rdb as descreibed on the sepc <https://rdb.fnordig.de/file_format.html#redis-rdb-file-format>
pub fn load_from_reader<R>(reader: &mut R, in_memory_db: &storage::Db) -> Result<()>
where
    R: Read,
{
    let mut header = vec![0x00; 9];
    reader.read_exact(&mut header)?;
    let header = String::from_utf8(header)?;
    if header != MAGIC_HEADER {
        return Err(Error::InvalidRdb(format!(
            "Unsupported rdb header {header}"
        )));
    }

    let (metadata, has_data) = load_metadata(reader)?;
    tracing::debug!("metadata: {metadata:?}, has_data: {has_data}");
    if has_data {
        load_database_data(reader, in_memory_db)?;
    }

    // read hash An 8-byte CRC64 checksum of the entire file.
    let mut hash = vec![0x00; 8];
    reader.read_exact(&mut hash)?;
    tracing::debug!("rdb hash: {hash:?}");
    // TODO check hash

    Ok(())
}

fn load_metadata<R>(reader: &mut R) -> Result<(HashMap<String, String>, bool)>
where
    R: Read,
{
    let mut metadata: HashMap<_, _> = HashMap::new();

    let mut marker = vec![0x00; 1];
    reader.read_exact(&mut marker)?;

    while marker[0] != OP_CODEC_SELECT_DB_0XFE {
        if marker[0] == OP_CODEC_END_OF_RDB_0XFF {
            // there is no data after the metadata
            return Ok((metadata, false));
        }
        if marker[0] != OP_CODEC_METADATA_SECTION_0XFA {
            return Err(Error::InvalidRdb(format!(
                "unexpected header found {:x}",
                marker[0]
            )));
        }
        let key = parser::read_string(reader)?;
        let value = parser::read_string(reader)?;
        metadata.insert(key, value);
        reader.read_exact(&mut marker)?;
    }

    Ok((metadata, true))
}

fn load_database_data(reader: &mut impl Read, in_memory_db: &storage::Db) -> Result<()> {
    let mut db_index = vec![0x00; 1];
    reader.read_exact(&mut db_index)?;
    tracing::debug!("db number = {}", db_index[0]);

    let mut marker = vec![0x00; 1];
    reader.read_exact(&mut marker)?;
    if marker[0] != OP_CODEC_RESIZEDB_0XFB {
        return Err(Error::InvalidRdb(format!(
            "unexpected rdb format, expected OP_CODEC_RESIZEDB but found {:x}",
            marker[0]
        )));
    }

    let (hash_table_size, _) = parser::read_lenth_encoding(reader)?;
    let (expired_hash_table_size, _) = parser::read_lenth_encoding(reader)?;
    tracing::debug!(
        "hash_table_size: {hash_table_size}, expired_hash_table_size: {expired_hash_table_size}"
    );

    load_all_key_values(reader, in_memory_db)?;

    tracing::debug!("loaded all load_key_value");
    Ok(())
}

// Read key values until we hit the end of the db
fn load_all_key_values<R: Read>(reader: &mut R, in_memory_db: &storage::Db) -> Result<()> {
    let mut mask = vec![0x00; 1];
    reader.read_exact(&mut mask)?;
    let mut key_value_type = mask[0];

    while key_value_type != OP_CODEC_END_OF_RDB_0XFF {
        match key_value_type {
            OP_CODEC_VALUE_TYPE_STRING_0X00 => load_key_value_without_expire(reader, in_memory_db),
            OP_CODEC_EXPIRE_SEC_0XFD | OP_CODEC_EXPIRE_MS_0XFC => {
                load_key_value_expire(reader, in_memory_db, key_value_type)
            }
            other => Err(Error::Unsupported(format!(
                "Unsupported key value type {other:x}"
            ))),
        }?;

        reader.read_exact(&mut mask)?;
        key_value_type = mask[0];
    }

    tracing::debug!("End of rdb found");

    Ok(())
}

fn load_key_value_without_expire<R: Read>(
    reader: &mut R,
    in_memory_db: &storage::Db,
) -> Result<()> {
    let key = read_string(reader)?;
    let value = read_string(reader)?;
    tracing::debug!("rdb load {key}={value}");
    in_memory_db.set(&key, &value, None);
    Ok(())
}
fn load_key_value_expire<R: Read>(
    reader: &mut R,
    in_memory_db: &storage::Db,
    marker_expirity_type: u8,
) -> Result<()> {
    // read expiration
    let expiration_ms = match marker_expirity_type {
        OP_CODEC_EXPIRE_SEC_0XFD => {
            let mut n32: [u8; 4] = [0x0; 4];
            reader.read_exact(&mut n32)?;
            Ok(u64::from(u32::from_le_bytes(n32) * 1000))
        }
        OP_CODEC_EXPIRE_MS_0XFC => {
            let mut n64: [u8; 8] = [0x0; 8];
            reader.read_exact(&mut n64)?;
            let n = u64::from_le_bytes(n64);
            Ok(n)
        }
        other => Err(Error::Unsupported(format!(
            "unexpected expiration format {other:x}"
        ))),
    }?;

    let mut value_type = vec![0x00; 1];
    reader.read_exact(&mut value_type)?;
    if value_type[0] != OP_CODEC_VALUE_TYPE_STRING_0X00 {
        return Err(Error::InvalidRdb(format!(
            "load_key_value_expired - unexpected value type after reading expiration {:x}",
            value_type[0]
        )));
    }

    let key = read_string(reader)?;
    let value = read_string(reader)?;
    let expiration = Some(u64_to_instant(expiration_ms));
    tracing::debug!("rdb load {key}={value} {expiration:?}");
    in_memory_db.set(&key, &value, expiration);
    Ok(())
}

fn u64_to_instant(timestamp: u64) -> SystemTime {
    let duration = Duration::from_millis(timestamp);
    UNIX_EPOCH + duration
}
