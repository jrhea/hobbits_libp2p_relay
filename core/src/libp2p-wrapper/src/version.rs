
/// Version information for Mothra

extern crate target_info;

use target_info::Target;

const TRACK: &str = "unstable";

/// Provides the current platform
pub fn platform() -> String {
    format!("{}-{}", Target::arch(), Target::os())
}

/// Version of the beacon node.
// TODO: Find the sha3 hash, date and rust version used to build the beacon_node binary
pub fn version() -> String {
    format!(
        "Mothra/v{}-{}/{}",
        env!("CARGO_PKG_VERSION"),
        TRACK,
        platform()
    )
}
