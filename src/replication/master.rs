use std::{
    io::{BufWriter, Write},
    sync::{mpsc::channel, Arc, Mutex},
};

use crate::{
    error::Result,
    protocol::{Cmd, Data},
    storage::Db,
};

/// If the command send to the client is a `FullResync` (end of the handshake)
/// it means the client is actualy a Redis slave! we register the slave and start the sync loop.
pub fn register_slave<W: Send + Write + 'static>(
    response: &Data,
    writer: &Arc<Mutex<BufWriter<W>>>,
    state: &Arc<Db>,
) {
    if !matches!(response, Data::FullResyncBinaryConent(_, _)) {
        return;
    }

    let writer = Arc::clone(writer);
    let state = Arc::clone(state);
    std::thread::spawn(move || {
        broadcast_to_slave_loop(&writer, &state).unwrap();
    });
}

// Used to broadcast master commands to the slaves
fn broadcast_to_slave_loop<W: Send + Write + 'static>(
    writer: &Arc<Mutex<BufWriter<W>>>,
    state: &Arc<Db>,
) -> Result<()> {
    tracing::debug!("Starting master to slave sync...");
    let (tx, rx) = channel::<Cmd>();

    state.register_slave(tx);

    for cmd in rx {
        tracing::debug!("new cmd to broadcast {cmd:?}");
        let data = cmd.to_data()?;

        let mut writer = writer.lock().unwrap();
        data.write_resp(&mut writer)?;
    }

    Ok(())
}

pub fn broadcast_cmd(cmd: &Cmd, state: &Arc<Db>) {
    let slaves = state.connected_slaves.lock().unwrap();
    for sender in slaves.iter() {
        tracing::debug!("broacasting cmd {cmd:?}");
        if let Err(e) = sender.send(cmd.clone()) {
            tracing::warn!("unabel to send {e}");
        };
    }
}
