pub mod error;
mod mothra;
pub use libp2p_wrapper::NetworkGlobals;
pub use mothra::{gossip, rpc_request, rpc_response, Mothra, NetworkMessage};
