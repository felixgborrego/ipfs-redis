use std::{
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::{
    cmds,
    error::{Error, Result},
    protocol::{self, Cmd, Data},
    rdb::{self},
    storage::{Config, Db},
};

/// The Slave Redis node starts the replication if configured
pub fn start_replication(state: Arc<Db>) -> Result<()> {
    let Some(replicaof) = &state.config.replicaof else {
        tracing::debug!("this is a master node - no replicaof set");
        return Ok(());
    };

    follow_master(replicaof.to_owned().as_str(), state)?;
    Ok(())
}

pub fn follow_master(replica_of: &str, state: Arc<Db>) -> Result<()> {
    let replica_of = replica_of.trim().replace(' ', ":");
    let tcp_stream = TcpStream::connect(replica_of)?;
    let mut writer = BufWriter::new(tcp_stream.try_clone().unwrap());
    let mut reader = BufReader::new(tcp_stream);

    handshake_step_1(&state.config, &mut writer, &mut reader)?;
    handshake_step_2(&mut writer, &mut reader, &state)?;

    // TODO read the rdb sent by the master

    // process commands
    std::thread::spawn(move || {
        if let Err(e) = follow_master_loop(&mut writer, &mut reader, &state) {
            tracing::warn!("Error processing master request {e:?}");
        }
    });
    Ok(())
}

fn follow_master_loop<W, R>(
    _: &mut BufWriter<W>,
    reader: &mut BufReader<R>,
    state: &Arc<Db>,
) -> Result<()>
where
    R: Read,
    W: Write,
{
    tracing::debug!("Starting slave loop...");
    loop {
        match Data::parse_cmd(reader) {
            Err(err) => println!("Unable to parse cmd: {err:?}"),
            Ok(cmd) => {
                tracing::debug!("cmd from master: {cmd:?}");
                let response = cmds::execute(cmd, state)?;
                //tracing::debug!("Response: {response:?}");
                if matches!(response, Data::ConnectionClosed) {
                    tracing::debug!("Client disconnected");
                }
            }
        }
    }
}

fn handshake_step_2<W, R>(
    writer: &mut BufWriter<W>,
    reader: &mut BufReader<R>,
    state: &Arc<Db>,
) -> Result<()>
where
    R: Read,
    W: Write,
{
    tracing::debug_span!("handshake2/2").in_scope(|| {
        let psync = Cmd::Psync {
            args: vec![
                Data::BulkString("?".to_string()),
                Data::BulkString("-1".to_string()),
            ],
        }
        .to_data()?;
        tracing::debug!(">> {psync:?}");
        psync.write_resp(writer)?;
        let response = protocol::Data::parse(reader)?;
        tracing::debug!(">> Handshake step 2/2 - response: {response:?}");

        let Data::SimpleString(response) = response else {
            return Err(Error::Unsupported(format!(
                "unexpected master cmd {response:?}"
            )));
        };
        if response.to_uppercase().starts_with("FULLRESYNC") {
            load_db_from_request(reader, state)?;
        }

        Ok(())
    })
}

fn handshake_step_1<W, R>(
    config: &Config,
    writer: &mut BufWriter<W>,
    reader: &mut BufReader<R>,
) -> Result<()>
where
    R: Read,
    W: Write,
{
    tracing::debug_span!("handshake1/1").in_scope(|| {
        let ping = Cmd::Ping.to_data()?;
        tracing::debug!("{ping:?}");
        ping.write_resp(writer)?;

        let response = protocol::Data::parse(reader)?;
        tracing::debug!("<< Handshake step 1/2 - ping response: {response:?}");

        let replconf_listening_port = Cmd::Replconf {
            args: vec![
                Data::BulkString("listening-port".to_string()),
                Data::BulkString(config.port.to_string()),
            ],
        }
        .to_data()?;
        tracing::debug!("{replconf_listening_port:?}");
        replconf_listening_port.write_resp(writer)?;
        let response = protocol::Data::parse(reader)?;
        tracing::debug!("<< Handshake step 1/2 replconf listening response: {response:?}");

        let replconf_capa = Cmd::Replconf {
            args: vec![
                Data::BulkString("capa".to_string()),
                Data::BulkString("psync2".to_string()),
            ],
        }
        .to_data()?;
        tracing::debug!("{replconf_capa:?}");
        replconf_capa.write_resp(writer)?;
        writer.flush()?;
        let response = protocol::Data::parse(reader)?;
        tracing::debug!("<< Handshake step 1/2 replconf capa response: {response:?}");

        Ok(())
    })
}

fn load_db_from_request<R>(reader: &mut BufReader<R>, state: &Arc<Db>) -> Result<()>
where
    R: Read,
{
    // read lenth
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    tracing::debug!("read rds from master: {buf}");
    rdb::load_from_reader(reader, state)
}
