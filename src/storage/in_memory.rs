use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
    time::SystemTime,
};

use super::info::Info;
use crate::{protocol::Cmd, Args};

#[derive(Default)]
pub struct Db {
    pub config: Config,
    info: Mutex<Info>,
    data: Mutex<HashMap<String, Value>>,
    pub connected_slaves: Mutex<Vec<Sender<Cmd>>>,
}

#[derive(Debug)]
enum Value {
    Data {
        data: String,
    },
    DataWithTTL {
        data: String,
        expiration: SystemTime,
    },
}

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub port: u16,
    pub replicaof: Option<String>,
    pub dir: Option<String>,
    pub dbfilename: Option<String>,
}

const DEFAULT_PORT: u16 = 6379;
impl Config {
    pub fn config_from_args(args: &Args) -> Self {
        Self {
            port: args.port.unwrap_or(DEFAULT_PORT),
            replicaof: args.replicaof.clone(),
            dir: args.dir.clone(),
            dbfilename: args.dbfilename.clone(),
        }
    }
    pub fn db_path(&self) -> Option<PathBuf> {
        self.dir
            .as_ref()
            .zip(self.dbfilename.as_ref())
            .map(|(dir, filename)| Path::new(dir.as_str()).join(filename.as_str()).clone())
    }
}
impl Db {
    pub fn new(config: Config) -> Self {
        Self {
            connected_slaves: Mutex::new(Vec::new()),
            info: Mutex::new(Info::from(&config)),
            config,
            data: Mutex::new(HashMap::default()),
        }
    }
    pub fn set(&self, key: &str, value: &str, expiration_time: Option<SystemTime>) {
        let mut data: std::sync::MutexGuard<'_, HashMap<String, Value>> = self.data.lock().unwrap();
        tracing::debug!("set {key} expiration {expiration_time:?}");
        let value = expiration_time.map_or_else(
            || Value::Data {
                data: value.to_owned(),
            },
            |expiration| Value::DataWithTTL {
                data: value.to_owned(),
                expiration,
            },
        );

        data.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut hash_map = self.data.lock().unwrap();

        let value = hash_map.get(key)?;

        match value {
            Value::Data { data } => Some(data.to_owned()),
            Value::DataWithTTL { data, expiration } => {
                get_with_ttl(key, data, expiration).or_else(|| {
                    delete(&mut hash_map, key);
                    None
                })
            }
        }
    }

    pub fn keys(&self, _: &str) -> Vec<String> {
        let hash_map = self.data.lock().unwrap();
        hash_map.keys().map(String::to_owned).collect()
    }

    pub fn info(&self) -> Info {
        let info = self.info.lock().unwrap();
        info.clone()
    }

    pub fn register_slave(&self, writer_to_slave: Sender<Cmd>) {
        let mut slaves = self.connected_slaves.lock().unwrap();
        slaves.push(writer_to_slave);
    }
}

fn get_with_ttl(key: &str, data: &str, expiration: &SystemTime) -> Option<String> {
    let now = SystemTime::now();

    //let elapsed = now.duration_since(*expiration);
    let Some(elapsed) = expiration.duration_since(now).ok() else {
        tracing::debug!("key {key} expired");
        return None;
    };

    tracing::debug!(
        "key {key} will expire in {}ms now:{now:?}, expiration:{expiration:?}",
        elapsed.as_millis()
    );
    Some(data.to_owned())
}

fn delete<T>(data: &mut std::sync::MutexGuard<'_, HashMap<String, T>>, key: &str) {
    data.remove(key);
}
