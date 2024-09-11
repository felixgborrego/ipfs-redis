use crate::error::{self, Result};
use futures::StreamExt;
use libp2p::{core::multiaddr::Multiaddr, identify, noise, swarm::SwarmEvent, tcp, yamux};
use std::{error::Error, time::Duration};
use tracing_subscriber::EnvFilter;

const PROTOCOL_VERSION: &str = "/ipfs/id/1.0.0";

async fn p2p_swam(remote_peer_addr: &str) -> Result<()> {
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

    // Tell the swarm to listen on all interfaces and a random, OS-assigned
    // port.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())?;

    let remote: Multiaddr = remote_peer_addr
        .parse()
        .map_err(|_| error::Error::ArgsMissing("Invalid address".to_string()))?;
    tracing::debug!("Connecting to peer at {remote}");
    swarm.dial(remote)?;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
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
