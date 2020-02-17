/// This crate contains the main link for artemis to rust-libp2p. It therefore re-exports
/// all required libp2p functionality.
///
/// This crate builds and manages the libp2p services required by the beacon node.
pub mod behaviour;
mod config;
mod discovery;
pub mod error;
pub mod rpc;
mod service;

pub use behaviour::PubsubMessage;
pub use config::{
    Config as NetworkConfig};
pub use libp2p::gossipsub::{Topic, TopicHash};
pub use libp2p::multiaddr;
pub use libp2p::Multiaddr;
pub use libp2p::{
    gossipsub::{GossipsubConfig, GossipsubConfigBuilder},
    PeerId,
};
pub use rpc::{RPCEvent,RPCRequest,RPCResponse,RPCErrorResponse,RPCProtocol,RPC};
pub use service::Libp2pEvent;
pub use service::Service;
pub use service::Message;
pub use service::DISCOVERY;
pub use service::GOSSIP;
pub use service::RPC;
