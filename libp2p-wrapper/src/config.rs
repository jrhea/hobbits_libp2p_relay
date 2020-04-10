extern crate target_info;
use crate::{Enr, error};
use libp2p::discv5::{Discv5Config, Discv5ConfigBuilder};
use libp2p::gossipsub::{GossipsubConfig, GossipsubConfigBuilder, GossipsubMessage, MessageId};
use libp2p::Multiaddr;
use serde_derive::{Deserialize, Serialize};
use clap::{App, AppSettings, Arg, ArgMatches};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;
use std::process;
use target_info::Target;

pub const GOSSIP_MAX_SIZE: usize = 1_048_576;
pub const DEFAULT_NAME: &str = "mothra";
pub const DEFAULT_PROTOCOL_VERSION: &str = "mothra/libp2p";
pub const NETWORK_DIR: &str = "network";
pub const DEFAULT_DEBUG_LEVEL: &str = "info";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
/// Network configuration for mothra.
pub struct Config {
    /// Application name
    pub name: String,

    /// Provides client version 
    pub client_version: String,

    /// Provides the current platform
    pub platform: String,

    /// Provides protocol version 
    pub protocol_version: String,

    /// network directory for mothra
    pub network_dir: PathBuf,

    /// IP address to listen on.
    pub listen_address: std::net::IpAddr,

    /// The TCP port that libp2p listens on.
    pub libp2p_port: u16,

    /// UDP port that discovery listens on.
    pub discovery_port: u16,

    /// The address to broadcast to peers about which address we are listening on. None indicates
    /// that no discovery address has been set in the CLI args.
    pub enr_address: Option<std::net::IpAddr>,

    /// The udp port to broadcast to peers in order to reach back for discovery.
    pub enr_udp_port: Option<u16>,

    /// The tcp port to broadcast to peers in order to reach back for libp2p services.
    pub enr_tcp_port: Option<u16>,

    /// Target number of connected peers.
    pub max_peers: usize,

    /// A secp256k1 secret key, as bytes in ASCII-encoded hex.
    ///
    /// With or without `0x` prefix.
    #[serde(skip)]
    pub secret_key_hex: Option<String>,

    /// Gossipsub configuration parameters.
    #[serde(skip)]
    pub gs_config: GossipsubConfig,

    /// Discv5 configuration parameters.
    #[serde(skip)]
    pub discv5_config: Discv5Config,

    /// List of nodes to initially connect to.
    pub boot_nodes: Vec<Enr>,

    /// List of libp2p nodes to initially connect to.
    pub libp2p_nodes: Vec<Multiaddr>,

    /// List of extra topics to initially subscribe to as strings.
    pub topics: Vec<String>,

    /// Introduces randomization in network propagation of messages. This should only be set for
    /// testing purposes and will likely be removed in future versions.
    // TODO: Remove this functionality for mainnet
    pub propagation_percentage: Option<u8>,

    /// log debug level
    pub debug_level: String,
}

impl Default for Config {
    /// Generate a default network configuration.
    fn default() -> Self {
        let client_version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let protocol_version = DEFAULT_PROTOCOL_VERSION.to_string();
        let platform = format!("{}-{}", Target::arch(), Target::os());
        let mut network_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        network_dir.push(format!(".{}",DEFAULT_NAME));
        network_dir.push(NETWORK_DIR);

        // The function used to generate a gossipsub message id
        // We use base64(SHA256(data)) for content addressing
        let gossip_message_id = |message: &GossipsubMessage| {
            MessageId(base64::encode_config(
                &Sha256::digest(&message.data),
                base64::URL_SAFE,
            ))
        };

        // gossipsub configuration
        // Note: The topics by default are sent as plain strings. Hashes are an optional
        // parameter.
        let gs_config = GossipsubConfigBuilder::new()
            .max_transmit_size(GOSSIP_MAX_SIZE)
            .heartbeat_interval(Duration::from_secs(20)) // TODO: Reduce for mainnet
            .manual_propagation() // require validation before propagation
            .no_source_id()
            .message_id_fn(gossip_message_id)
            .build();

        // discv5 configuration
        let discv5_config = Discv5ConfigBuilder::new()
            .request_timeout(Duration::from_secs(4))
            .request_retries(2)
            .enr_update(true) // update IP based on PONG responses
            .enr_peer_update_min(2) // prevents NAT's should be raised for mainnet
            .query_parallelism(5)
            .query_timeout(Duration::from_secs(2))
            .ip_limit(false) // limits /24 IP's in buckets. Enable for mainnet
            .ping_interval(Duration::from_secs(300))
            .build();

        Config {
            name: DEFAULT_NAME.to_string(),
            client_version,
            platform,
            protocol_version,
            network_dir,
            listen_address: "127.0.0.1".parse().expect("valid ip address"),
            libp2p_port: 9000,
            discovery_port: 9000,
            enr_address: None,
            enr_udp_port: None,
            enr_tcp_port: None,
            max_peers: 10,
            secret_key_hex: None,
            gs_config,
            discv5_config,
            boot_nodes: vec![],
            libp2p_nodes: vec![],
            topics: vec![],
            propagation_percentage: None,
            debug_level: DEFAULT_DEBUG_LEVEL.to_string(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn agent_version(&mut self) -> String {
        format!(
            "{}/{}/{}",
            self.name,
            self.client_version,
            self.platform
        )
    }

    pub fn apply_args(&mut self, name: Option<String>, client_version: Option<String>, protocol_version: Option<String>, cli_args: Vec<String>) -> Result<(), String> {
    
        // update self.name if name is not None
        if let Some(x) = name { self.name = x; }
        // update self.client_version if client_version is not None
        if let Some(x) = client_version { self.client_version = x; }
        // update self.protocol_version if protocol_version is not None
        if let Some(x) = protocol_version { self.protocol_version = x; }

          // collect cli argument matches
        let args = matches(self, cli_args);

        // If a `datadir` has been specified, set the network dir to be inside it.
        if let Some(dir) = args.value_of("datadir") {
            self.network_dir = PathBuf::from(dir).join("network");
        };

        if let Some(listen_address_str) = args.value_of("listen-address") {
            let listen_address = listen_address_str
                .parse()
                .map_err(|_| format!("Invalid listen address: {:?}", listen_address_str))?;
            self.listen_address = listen_address;
            self.enr_address = Some(listen_address);
        }

        if let Some(max_peers_str) = args.value_of("maxpeers") {
            self.max_peers = max_peers_str
                .parse::<usize>()
                .map_err(|_| format!("Invalid number of max peers: {}", max_peers_str))?;
        }

        if let Some(port_str) = args.value_of("port") {
            let port = port_str
                .parse::<u16>()
                .map_err(|_| format!("Invalid port: {}", port_str))?;
                self.libp2p_port = port;
                self.discovery_port = port;
                self.enr_tcp_port = Some(port);
                self.enr_udp_port = Some(port);
        }

        if let Some(disc_port_str) = args.value_of("discovery-port") {
            self.discovery_port = disc_port_str
                .parse::<u16>()
                .map_err(|_| format!("Invalid discovery port: {}", disc_port_str))?;
                self.enr_udp_port = Some(self.discovery_port);
        }

        if let Some(boot_enr_str) = args.value_of("boot-nodes") {
            self.boot_nodes = boot_enr_str
                .split(',')
                .map(|enr| enr.parse().map_err(|_| format!("Invalid ENR: {}", enr)))
                .collect::<Result<Vec<Enr>, _>>()?;
        }

        if let Some(libp2p_addresses_str) = args.value_of("libp2p-addresses") {
            self.libp2p_nodes = libp2p_addresses_str
                .split(',')
                .map(|multiaddr| {
                    multiaddr
                        .parse()
                        .map_err(|_| format!("Invalid Multiaddr: {}", multiaddr))
                })
                .collect::<Result<Vec<Multiaddr>, _>>()?;
        }

        if let Some(enr_address_str) = args.value_of("enr-address") {
            self.enr_address = Some(
                enr_address_str
                    .parse()
                    .map_err(|_| format!("Invalid discovery address: {:?}", enr_address_str))?,
            )
        }

        if let Some(enr_udp_port_str) = args.value_of("enr-udp-port") {
            self.enr_udp_port = Some(
                enr_udp_port_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid discovery port: {}", enr_udp_port_str))?,
            );
        }
    
        if let Some(enr_tcp_port_str) = args.value_of("enr-tcp-port") {
            self.enr_tcp_port = Some(
                enr_tcp_port_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid ENR TCP port: {}", enr_tcp_port_str))?,
            );
        }


        if args.is_present("disable_enr_auto_update") {
            self.discv5_config.enr_update = false;
        }

        if let Some(topics_str) = args.value_of("topics") {
            self.topics = topics_str.split(',').map(|s| s.into()).collect();
        }

        if let Some(debug_level_str) = args.value_of("debug-level") {
            self.debug_level = debug_level_str
                    .parse()
                    .map_err(|_| format!("Invalid debug-level: {:?}", debug_level_str))?;
        }

        /*
        * Zero-ports
        *
        * Replaces previously set flags.
        * Libp2p and discovery ports are set explicitly by selecting
        * a random free port so that we aren't needlessly updating ENR
        * Discovery address is set to localhost by default.
        */
        if args.is_present("zero-ports") {
            if self.enr_address == Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))) {
                self.enr_address = None
            }
            self.libp2p_port =
                unused_port("tcp").map_err(|e| format!("Failed to get port for libp2p: {}", e))?;
            self.discovery_port =
                unused_port("udp").map_err(|e| format!("Failed to get port for discovery: {}", e))?;
        }

        Ok(())
    }
}

fn matches(config: &Config, args: Vec<String>) -> ArgMatches<'static> {
    App::new(config.name.clone())
    .version(&*config.client_version)
    .setting(AppSettings::ColoredHelp)
    .setting(AppSettings::NextLineHelp)
    .arg(
        Arg::with_name("datadir")
            .long("datadir")
            .value_name("DIR")
            .help("Data directory for mothra to use.")
            .takes_value(true)
    )
    // network related arguments
    .arg(
        Arg::with_name("zero-ports")
            .long("zero-ports")
            .short("z")
            .help("Sets all listening TCP/UDP ports to 0, allowing the OS to choose some \
                   arbitrary free ports.")
            .takes_value(false),
    )
    .arg(
        Arg::with_name("listen-address")
            .long("listen-address")
            .value_name("ADDRESS")
            .help("The address the client will listen for UDP and TCP connections.")
            .default_value("127.0.0.1")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("port")
            .long("port")
            .value_name("PORT")
            .help("The TCP/UDP port to listen on.")
            .default_value("9000")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("discovery-port")
            .long("discovery-port")
            .value_name("PORT")
            .help("The discovery UDP port.")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("maxpeers")
            .long("maxpeers")
            .help("The maximum number of peers.")
            .default_value("10")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("boot-nodes")
            .long("boot-nodes")
            .allow_hyphen_values(true)
            .value_name("ENR-LIST")
            .help("One or more comma-delimited base64-encoded ENR's to bootstrap the p2p network.")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("enr-udp-port")
            .long("enr-udp-port")
            .value_name("PORT")
            .help("The UDP port of the local ENR.")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("enr-tcp-port")
            .long("enr-tcp-port")
            .value_name("PORT")
            .help("The TCP port of the local ENR.")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("enr-address")
            .long("enr-address")
            .value_name("ADDRESS")
            .help("The IP address to broadcast to other peers on how to reach this node. \
            Set this only if you are sure other nodes can connect to your local node on this address. \
            Discovery will automatically find your external address,if possible.\n")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("disable-enr-auto-update")
            .long("disable-enr-auto-update")
            .short("-d")
            .help("Discovery automatically updates the nodes local ENR with an external IP address and port as seen by other peers on the network. \
            This disables this feature, fixing the ENR's IP/PORT to those specified on boot.\n")
            .takes_value(false),
    )
    .arg(
        Arg::with_name("topics")
            .long("topics")
            .value_name("STRING")
            .help("One or more comma-delimited gossipsub topic strings to subscribe to.")
            .takes_value(true),
    )
        .arg(
        Arg::with_name("libp2p-addresses")
            .long("libp2p-addresses")
            .value_name("MULTIADDR")
            .help("One or more comma-delimited multiaddrs to manually connect to a libp2p peer without an ENR.")
            .takes_value(true),
        )
    .arg(
        Arg::with_name("debug-level")
            .long("debug-level")
            .value_name("LEVEL")
            .help("Specifies what log records are displayed.")
            .takes_value(true)
            .possible_values(&["info", "debug", "trace", "warn", "error", "crit"])
    )
   .get_matches_from_safe(args.iter())
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1);
        })
}

pub fn unused_port(transport: &str) -> error::Result<u16> {
    let local_addr = match transport {
        "tcp" => {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| {
                format!("Failed to create TCP listener to find unused port: {:?}", e)
            })?;
            listener.local_addr().map_err(|e| {
                format!(
                    "Failed to read TCP listener local_addr to find unused port: {:?}",
                    e
                )
            })?
        }
        "udp" => {
            let socket = std::net::UdpSocket::bind("127.0.0.1:0") 
                .map_err(|e| format!("Failed to create UDP socket to find unused port: {:?}", e))?;
            socket.local_addr().map_err(|e| {
                format!(
                    "Failed to read UDP socket local_addr to find unused port: {:?}",
                    e
                )
            })?
        }
        _ => return Err("Invalid transport to find unused port".into()),
    };
    Ok(local_addr.port())
}
