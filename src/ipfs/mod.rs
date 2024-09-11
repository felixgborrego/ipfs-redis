use crate::error::{self, Result};
use futures::StreamExt;
use libp2p::{core::multiaddr::Multiaddr, identify, noise, swarm::SwarmEvent, tcp, yamux};
use std::time::Duration;
use tokio::task;

const PROTOCOL_VERSION: &str = "/ipfs/id/1.0.0";

pub fn start_swam_loop(remote_peer_addr: &Option<String>) {
    let addr = remote_peer_addr.clone();
    task::spawn(async move {
        if let Err(e) = connect_p2p_swam(addr).await {
            tracing::warn!("Error processing request {e:?}");
        }
        tracing::debug!("Async task completed");
    });
}

async fn connect_p2p_swam(remote_peer_addr: Option<String>) -> Result<()> {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            identify::Behaviour::new(identify::Config::new(PROTOCOL_VERSION.into(), key.public()))
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())?;

    if let Some(arr) = remote_peer_addr {
        let remote: Multiaddr = arr
            .parse()
            .map_err(|_| error::Error::ArgsMissing("Invalid address".to_string()))?;
        tracing::debug!("Connecting to peer at {remote}");

        swarm.dial(remote)?;
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!(" 📡 Listening on {address:?} for peers!");
            }
            // Prints peer id identify info is being sent to.
            SwarmEvent::Behaviour(identify::Event::Sent { peer_id, .. }) => {
                tracing::debug!("Sent identify info to {peer_id:?}");
            }
            // Prints out the info received via the identify event
            SwarmEvent::Behaviour(identify::Event::Received { info, .. }) => {
                tracing::debug!("Received {info:?}");
            }
            _ => {}
        }
    }
}
