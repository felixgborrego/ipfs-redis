use std::fmt::Display;

use uuid::Uuid;

use super::Config;

#[derive(Debug, Clone, Default)]
pub struct Info {
    pub replication: Replication,
}

#[derive(Debug, Clone, Default)]
pub struct Replication {
    pub connected_slaves: u32,
    pub role: String,
    pub master_replid: String,
    pub master_repl_offset: u32,
}

impl Info {
    pub fn from(config: &Config) -> Self {
        let my_uuid = Uuid::now_v7();
        Self {
            replication: Replication {
                master_replid: my_uuid.to_string(),
                master_repl_offset: 0,
                connected_slaves: 0,
                role: config
                    .replicaof
                    .clone()
                    .map_or("master".to_string(), |_| "slave".to_string()),
            },
        }
    }

    pub fn is_master(&self) -> bool {
        matches!(self.replication.role.as_str(), "master")
    }
}

impl Display for Replication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# Replication")?;
        writeln!(f, "role:{}", self.role)?;
        writeln!(f, "master_replid:{}", self.master_replid)?;
        writeln!(f, "connected_slaves:{}", self.connected_slaves)?;
        writeln!(f, "master_repl_offset:{}", self.master_repl_offset)?;
        Ok(())
    }
}
