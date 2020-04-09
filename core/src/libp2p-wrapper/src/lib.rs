pub mod behaviour;
mod config;
mod discovery;
pub mod rpc;
mod service;
pub mod types;
mod version;

pub use crate::types::{error, NetworkGlobals, PeerInfo, GossipTopic, Enr, EnrBitfield, EnrForkId, SubnetId};
pub use config::Config as NetworkConfig;
pub use libp2p::gossipsub::{MessageId, Topic, TopicHash};
pub use libp2p::{PeerId, Swarm, multiaddr, Multiaddr};
pub use rpc::{RPCErrorResponse, RPCEvent, RPCRequest, RPCResponse};
pub use service::{Libp2pEvent, Service};
