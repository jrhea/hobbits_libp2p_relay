pub mod behaviour;
mod config;
mod discovery;
pub mod rpc;
mod service;
pub mod types;

pub use crate::types::{
    error, EnrBitfield, EnrForkId, GossipTopic, NetworkGlobals, PeerInfo
};
pub use config::unused_port;
pub use config::Config as NetworkConfig;
pub use libp2p::gossipsub::{MessageId, Topic, TopicHash};
pub use libp2p::{multiaddr, Multiaddr, PeerId, Swarm};
pub use libp2p::discv5::enr::{Enr, CombinedKey};
pub use rpc::{RPCErrorResponse, RPCEvent, RPCRequest, RPCResponse};
pub use service::{Libp2pEvent, Service};

pub const DEFAULT_CLIENT_NAME: &str = "mothra";
