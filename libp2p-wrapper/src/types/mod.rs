pub mod error;
mod globals;
mod peer_info;
mod topics;

pub use globals::NetworkGlobals;
pub use peer_info::PeerInfo;
pub use topics::GossipTopic;

use types::{BitVector, EthSpec, MainnetEthSpec};

#[allow(type_alias_bounds)]
pub type EnrBitfield = BitVector<<MainnetEthSpec as EthSpec>::SubnetBitfieldLength>;
pub type SubnetId = u64;
pub type EnrForkId = Vec<u8>;
// shift this type into discv5
pub type Enr = libp2p::discv5::enr::Enr<libp2p::discv5::enr::CombinedKey>;