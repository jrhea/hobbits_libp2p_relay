use super::ENR_FILENAME;
use crate::Enr;
use crate::NetworkConfig;
use libp2p::core::identity::Keypair;
use libp2p::discv5::enr::{CombinedKey, EnrBuilder};
use slog::{debug, warn};
use ssz::Encode;
use ssz_types::BitVector;
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::str::FromStr;
use types::{EnrForkId, EthSpec};

/// The ENR field specifying the fork id.
pub const ETH2_ENR_KEY: &str = "eth2";
/// The ENR field specifying the subnet bitfield.
pub const BITFIELD_ENR_KEY: &str = "attnets";

/// Loads an ENR from file if it exists and matches the current NodeId and sequence number. If none
/// exists, generates a new one.
///
/// If an ENR exists, with the same NodeId, this function checks to see if the loaded ENR from
/// disk is suitable to use, otherwise we increment our newly generated ENR's sequence number.
pub fn build_or_load_enr<T: EthSpec>(
    local_key: Keypair,
    config: &NetworkConfig,
    enr_fork_id: Vec<u8>,
    log: &slog::Logger,
) -> Result<Enr, String> {
    // Build the local ENR.
    // Note: Discovery should update the ENR record's IP to the external IP as seen by the
    // majority of our peers, if the CLI doesn't expressly forbid it.
    let enr_key: CombinedKey = local_key
        .try_into()
        .map_err(|_| "Invalid key type for ENR records")?;

    let mut local_enr = build_enr::<T>(&enr_key, config, enr_fork_id)?;

    let enr_f = config.network_dir.join(ENR_FILENAME);
    if let Ok(mut enr_file) = File::open(enr_f.clone()) {
        let mut enr_string = String::new();
        match enr_file.read_to_string(&mut enr_string) {
            Err(_) => debug!(log, "Could not read ENR from file"),
            Ok(_) => {
                match Enr::from_str(&enr_string) {
                    Ok(disk_enr) => {
                        // if the same node id, then we may need to update our sequence number
                        if local_enr.node_id() == disk_enr.node_id() {
                            if compare_enr(&local_enr, &disk_enr) {
                                debug!(log, "ENR loaded from disk"; "file" => format!("{:?}", enr_f));
                                // the stored ENR has the same configuration, use it
                                return Ok(disk_enr);
                            }

                            // same node id, different configuration - update the sequence number
                            let new_seq_no = disk_enr.seq().checked_add(1).ok_or_else(|| "ENR sequence number on file is too large. Remove it to generate a new NodeId")?;
                            local_enr.set_seq(new_seq_no, &enr_key).map_err(|e| {
                                format!("Could not update ENR sequence number: {:?}", e)
                            })?;
                            debug!(log, "ENR sequence number increased"; "seq" =>  new_seq_no);
                        }
                    }
                    Err(e) => {
                        warn!(log, "ENR from file could not be decoded"; "error" => format!("{:?}", e));
                    }
                }
            }
        }
    }

    save_enr_to_disk(&config.network_dir, &local_enr, log);

    Ok(local_enr)
}

/// Builds a lighthouse ENR given a `NetworkConfig`.
fn build_enr<T: EthSpec>(
    enr_key: &CombinedKey,
    config: &NetworkConfig,
    enr_fork_id: Vec<u8>,
) -> Result<Enr, String> {
    let mut builder = EnrBuilder::new("v4");
    if let Some(enr_address) = config.enr_address {
        builder.ip(enr_address);
    }
    if let Some(udp_port) = config.enr_udp_port {
        builder.udp(udp_port);
    }
    // we always give it our listening tcp port
    // TODO: Add uPnP support to map udp and tcp ports
    let tcp_port = config.enr_tcp_port.unwrap_or_else(|| config.libp2p_port);
    builder.tcp(tcp_port);

    // set the `eth2` field on our ENR

    // builder.add_value(ETH2_ENR_KEY.into(), enr_fork_id.as_ssz_bytes());
    // TODO: fix this
    builder.add_value(ETH2_ENR_KEY.into(), enr_fork_id);

    // set the "attnets" field on our ENR
    let bitfield = BitVector::<T::SubnetBitfieldLength>::new();

    builder.add_value(BITFIELD_ENR_KEY.into(), bitfield.as_ssz_bytes());

    builder
        .tcp(config.libp2p_port)
        .build(enr_key)
        .map_err(|e| format!("Could not build Local ENR: {:?}", e))
}

/// Defines the conditions under which we use the locally built ENR or the one stored on disk.
/// If this function returns true, we use the `disk_enr`.
fn compare_enr(local_enr: &Enr, disk_enr: &Enr) -> bool {
    // take preference over disk_enr address if one is not specified
    (local_enr.ip().is_none() || local_enr.ip() == disk_enr.ip())
        // tcp ports must match
        && local_enr.tcp() == disk_enr.tcp()
        // must match on the same fork
        && local_enr.get(ETH2_ENR_KEY) == disk_enr.get(ETH2_ENR_KEY)
        // take preference over disk udp port if one is not specified
        && (local_enr.udp().is_none() || local_enr.udp() == disk_enr.udp())
        // we need the BITFIELD_ENR_KEY key to match, otherwise we use a new ENR. This will likely only
        // be true for non-validating nodes
        && local_enr.get(BITFIELD_ENR_KEY) == disk_enr.get(BITFIELD_ENR_KEY)
}

/// Saves an ENR to disk
pub fn save_enr_to_disk(dir: &Path, enr: &Enr, log: &slog::Logger) {
    let _ = std::fs::create_dir_all(dir);
    match File::create(dir.join(Path::new(ENR_FILENAME)))
        .and_then(|mut f| f.write_all(&enr.to_base64().as_bytes()))
    {
        Ok(_) => {
            debug!(log, "ENR written to disk");
        }
        Err(e) => {
            warn!(
                log,
                "Could not write ENR to file"; "file" => format!("{:?}{:?}",dir, ENR_FILENAME),  "error" => format!("{}", e)
            );
        }
    }
}
