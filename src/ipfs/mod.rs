use libp2p::{gossipsub, mdns, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux};

#[derive(NetworkBehaviour)]
struct P2pBehaviour {
    gossipsub: gossipsub::Behaviour,
}


https://github.com/libp2p/rust-libp2p/blob/master/examples/chat/src/main.rs