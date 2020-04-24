//NOTE: This should be removed in favour of the PeerManager PeerInfo, once built.
use crate::{EnrBitfield};

/// Information about a given connected peer.
#[derive(Default, Debug, Clone)]
pub struct PeerInfo {
    /// The current syncing state of the peer. The state may be determined after it's initial
    /// connection.
    pub syncing_state: Option<PeerSyncingState>,
    /// The ENR subnet bitfield of the peer. This may be determined after it's initial
    /// connection.
    pub enr_bitfield: Option<EnrBitfield>,
}

#[derive(Debug, Clone)]
pub enum PeerSyncingState {
    /// At the current state as our node.
    Synced,
    /// The peer is further ahead than our node and useful for block downloads.
    Ahead,
    /// Is behind our current head and not useful for block downloads.
    Behind,
}

impl PeerInfo {
    /// Creates a new PeerInfo, specifying it's
    pub fn new() -> Self {
        PeerInfo {
            syncing_state: None,
            enr_bitfield: None,
        }
    }
}
