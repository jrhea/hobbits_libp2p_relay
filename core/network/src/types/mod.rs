pub mod error;
mod globals;
mod peer_info;
mod topics;

pub use globals::NetworkGlobals;
pub use peer_info::PeerInfo;
pub use topics::GossipTopic;

#[allow(type_alias_bounds)]
pub type EnrBitfield = Vec<u8>;
pub type EnrForkId = Vec<u8>;

pub type Enr = discv5::enr::Enr<discv5::enr::CombinedKey>;
