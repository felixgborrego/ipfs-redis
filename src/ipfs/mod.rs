use libp2p::{gossipsub, mdns, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux};

#[derive(NetworkBehaviour)]
struct P2pBehaviour {
    gossipsub: gossipsub::Behaviour,
}

pub fn connect_swam() {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) /
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the ki
                .message_id_fn(message_id_fn) 
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; 

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            Ok(MyBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("irfs-redis-broadcast");
    // subscribes to our topic
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
}
