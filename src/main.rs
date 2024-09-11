#![deny(clippy::pedantic)] // up front pain and suffering for the greater good :)

use std::error::Error;
use std::io::{BufReader, BufWriter, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use clap::Parser;
use protocol::Data;
use replication::{master, slave};
use storage::{Config, Db};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod cmds;
mod error;
mod ipfs;
mod protocol;
mod rdb;
mod replication;
mod storage;

use rdb::Rdb;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(long)]
    dir: Option<String>,
    #[arg(long)]
    dbfilename: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    replicaof: Option<String>,
    #[arg(long)]
    remote_p2p_peer: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // setup logs
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse();
    tracing::debug!("args: {args:?}");
    let config = Config::config_from_args(&args);

    ipfs::start_swam_loop(&args.remote_p2p_peer);

    tracing::info!("Redis server starting at {}!", config.port);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port));

    match listener {
        Ok(mut tcp_listener) => start_loop(&config, &mut tcp_listener),
        Err(e) => Err(error::Error::IO(e)),
    }?;

    Ok(())
}

/// main Redis loop, listening for incomming command request
fn start_loop(conf: &Config, tcp_listener: &mut TcpListener) -> error::Result<()> {
    let mut db = Rdb::from(conf);
    let database = db.load()?;
    let state = Arc::new(database);

    // Starts replication if the node is a slave
    slave::start_replication(Arc::clone(&state))?;

    loop {
        let state: Arc<Db> = Arc::clone(&state);
        if let Ok((client_stream, _)) = tcp_listener.accept() {
            tracing::debug!("New client connected!");
            std::thread::spawn(move || {
                if let Err(e) = process_client_requets(client_stream, &state) {
                    tracing::warn!("Error processing request {e:?}");
                }
                tracing::debug!("Thread completed");
            });
        }
    }
}

/// Process the incoming request from a single Redis client.
fn process_client_requets(stream: TcpStream, state: &Arc<Db>) -> error::Result<()> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let writer = Arc::new(Mutex::new(BufWriter::new(stream)));

    loop {
        match Data::parse_cmd(&mut reader) {
            Err(err) => println!("Unable to parse cmd: {err:?}"),
            Ok(cmd) => {
                let response = cmds::execute(cmd, state)?;
                tracing::debug!("process_stream response: {response:?}");
                if matches!(response, Data::ConnectionClosed) {
                    tracing::debug!("Client disconnected");
                    return Ok(());
                }

                // if the client is doing a handshake
                master::register_slave(&response, &writer, state);

                let mut writer = writer.lock().unwrap();
                response.write_resp(&mut writer)?;
                writer.flush()?;
            }
        }
    }
}
