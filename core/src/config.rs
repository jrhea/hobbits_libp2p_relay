use clap::ArgMatches;
use network::{unused_port, CombinedKey, Enr, Multiaddr, NetworkConfig, DEFAULT_CLIENT_NAME};
use std::path::PathBuf;

pub const DEFAULT_DEBUG_LEVEL: &str = "info";

/// Mothra configuration
pub struct Config {
    /// The client name
    pub client_name: String,

    /// The client version
    pub client_version: String,

    /// The log debug level
    pub debug_level: String,

    /// Network configuration
    pub network_config: NetworkConfig,
}

impl Default for Config {
    /// Generate a default mothra configuration.
    fn default() -> Self {
        let network_config = NetworkConfig::new();
        Config {
            client_name: DEFAULT_CLIENT_NAME.into(),
            client_version: format!("v{}", env!("CARGO_PKG_VERSION")),
            debug_level: DEFAULT_DEBUG_LEVEL.into(),
            network_config,
        }
    }
}

impl Config {
    pub fn new(
        client_name: Option<String>,
        client_version: Option<String>,
        protocol_version: Option<String>,
    ) -> Self {
        let mut config = Config::default();
        // update self.client_name if client_name is not None
        if let Some(x) = client_name {
            config.client_name = x;
        }
        // update self.client_version if client_version is not None
        if let Some(x) = client_version {
            config.client_version = x;
        }
        // update self.network_config.protocol_version if protocol_version is not None
        if let Some(x) = protocol_version {
            config.network_config.protocol_version = x;
        }
        config
    }
    pub fn apply_cli_args(&mut self, args: &ArgMatches) -> Result<(), String> {
        // If a `datadir` has been specified, set the network dir to be inside it.
        if let Some(dir) = args.value_of("datadir") {
            self.network_config.network_dir = PathBuf::from(dir).join("network");
        };

        if let Some(listen_address_str) = args.value_of("listen-address") {
            let listen_address = listen_address_str
                .parse()
                .map_err(|_| format!("Invalid listen address: {:?}", listen_address_str))?;
            self.network_config.listen_address = listen_address;
            self.network_config.enr_address = Some(listen_address);
        }

        if let Some(max_peers_str) = args.value_of("maxpeers") {
            self.network_config.max_peers = max_peers_str
                .parse::<usize>()
                .map_err(|_| format!("Invalid number of max peers: {}", max_peers_str))?;
        }

        if let Some(port_str) = args.value_of("port") {
            let port = port_str
                .parse::<u16>()
                .map_err(|_| format!("Invalid port: {}", port_str))?;
            self.network_config.libp2p_port = port;
            self.network_config.discovery_port = port;
            self.network_config.enr_tcp_port = Some(port);
            self.network_config.enr_udp_port = Some(port);
        }

        if let Some(disc_port_str) = args.value_of("discovery-port") {
            self.network_config.discovery_port = disc_port_str
                .parse::<u16>()
                .map_err(|_| format!("Invalid discovery port: {}", disc_port_str))?;
            self.network_config.enr_udp_port = Some(self.network_config.discovery_port);
        }

        if let Some(boot_enr_str) = args.value_of("boot-nodes") {
            self.network_config.boot_nodes = boot_enr_str
                .split(',')
                .map(|enr| enr.parse().map_err(|_| format!("Invalid ENR: {}", enr)))
                .collect::<Result<Vec<Enr<CombinedKey>>, _>>()?;
        }

        if let Some(libp2p_addresses_str) = args.value_of("libp2p-addresses") {
            self.network_config.libp2p_nodes = libp2p_addresses_str
                .split(',')
                .map(|multiaddr| {
                    multiaddr
                        .parse()
                        .map_err(|_| format!("Invalid Multiaddr: {}", multiaddr))
                })
                .collect::<Result<Vec<Multiaddr>, _>>()?;
        }

        if let Some(enr_address_str) = args.value_of("enr-address") {
            self.network_config.enr_address = Some(
                enr_address_str
                    .parse()
                    .map_err(|_| format!("Invalid discovery address: {:?}", enr_address_str))?,
            )
        }

        if let Some(enr_udp_port_str) = args.value_of("enr-udp-port") {
            self.network_config.enr_udp_port = Some(
                enr_udp_port_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid discovery port: {}", enr_udp_port_str))?,
            );
        }

        if let Some(enr_tcp_port_str) = args.value_of("enr-tcp-port") {
            self.network_config.enr_tcp_port = Some(
                enr_tcp_port_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid ENR TCP port: {}", enr_tcp_port_str))?,
            );
        }

        if args.is_present("disable_enr_auto_update") {
            self.network_config.discv5_config.enr_update = false;
        }

        if let Some(topics_str) = args.value_of("topics") {
            self.network_config.topics = topics_str.split(',').map(|s| s.into()).collect();
        }

        if let Some(debug_level_str) = args.value_of("debug-level") {
            self.debug_level = debug_level_str
                .parse()
                .map_err(|_| format!("Invalid debug-level: {:?}", debug_level_str))?;
        }

        if args.is_present("auto-ports") {
            if self.network_config.enr_address
                == Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)))
            {
                self.network_config.enr_address = None
            }
            self.network_config.libp2p_port =
                unused_port("tcp").map_err(|e| format!("Failed to get port for libp2p: {}", e))?;
            self.network_config.discovery_port = unused_port("udp")
                .map_err(|e| format!("Failed to get port for discovery: {}", e))?;

            self.network_config.enr_tcp_port = Some(self.network_config.libp2p_port);
            self.network_config.enr_udp_port = Some(self.network_config.discovery_port);
        }
        Ok(())
    }
}
