use std::sync::Arc;

use crate::error::Result;
use crate::protocol::{Cmd, Data};
use crate::replication::master;
use crate::storage::Db;
mod basic;
mod replication;
mod set_get;

pub fn execute(cmd: Cmd, state: &Arc<Db>) -> Result<Data> {
    if cmd.is_write() && state.info().is_master() {
        master::broadcast_cmd(&cmd, state);
    }
    tracing::debug_span!("cmd_execute", cmd = ?cmd).in_scope(|| match cmd {
        Cmd::ConnectionClosed => Ok(Data::ConnectionClosed),
        Cmd::Ping => Ok(basic::ping_execute()),
        Cmd::Echo { args } => basic::echo_execute(&args),
        Cmd::Get { args } => set_get::get_execute(&args, state),
        Cmd::Set { args } => set_get::set_execute(&args, state),
        Cmd::Config { args } => basic::config_execute(&args, state),
        Cmd::Command { args } => Ok(basic::command_execute(&args)),
        Cmd::Keys { args } => set_get::keys_execute(&args, state),
        Cmd::Info { args } => basic::info_execute(&args, state),
        Cmd::Replconf { args } => Ok(replication::replconf_execute(&args, state)),
        Cmd::Psync { args } => replication::psync_execute(&args, state),
    })
}
