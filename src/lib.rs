#![deny(clippy::pedantic)] // up front pain and suffering for the greater good :)
// Public API documentation lints, only relevant now that modules are exposed as a library.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]

use clap::Parser;

pub mod cmds;
pub mod error;
pub mod ipfs;
pub mod protocol;
pub mod rdb;
pub mod replication;
pub mod storage;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct Args {
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long)]
    pub dbfilename: Option<String>,
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long)]
    pub replicaof: Option<String>,
    #[arg(long)]
    pub remote_p2p_peer: Option<String>,
}
