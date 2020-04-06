pub mod behaviour;
mod config;
mod discovery;
pub mod rpc;
mod service;
pub mod types;
mod version;

// shift this type into discv5
pub type Enr = libp2p::discv5::enr::Enr<libp2p::discv5::enr::CombinedKey>;

pub use crate::types::{error, NetworkGlobals, PeerInfo, GossipTopic};
pub use config::Config as NetworkConfig;
pub use libp2p::gossipsub::{MessageId, Topic, TopicHash};
pub use libp2p::{multiaddr, Multiaddr};
pub use libp2p::{PeerId, Swarm};
pub use rpc::{RPCErrorResponse, RPCEvent, RPCRequest, RPCResponse};
pub use service::{Libp2pEvent, Service};
