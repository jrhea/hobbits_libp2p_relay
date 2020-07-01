pub mod error;
mod globals;
mod topics;

pub use globals::NetworkGlobals;
pub use topics::GossipTopic;

#[allow(type_alias_bounds)]
pub type EnrBitfield = Vec<u8>;
pub type EnrForkId = Vec<u8>;
pub type SubnetId = u64;
pub type GossipKind = String;
pub type Enr = discv5::enr::Enr<discv5::enr::CombinedKey>;
