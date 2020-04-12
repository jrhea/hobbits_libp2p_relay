use clap::{App, AppSettings, Arg, ArgMatches};

pub fn cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new("mothra")
    .version(clap::crate_version!())
    .about("P2P networking component.")
    .arg(
        Arg::with_name("datadir")
            .long("datadir")
            .value_name("DIR")
            .help("The location of the data directory to use.")
            .takes_value(true)
    )
    .arg(
        Arg::with_name("auto-ports")
            .long("auto-ports")
            .short("a")
            .help("Allow the OS to select from available TCP/UDP ports.")
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
        Arg::with_name("disable-enr-auto-update")
            .long("disable-enr-auto-update")
            .short("-d")
            .help("This fixes the ENR's IP/PORT to whatever is specified at startup.")
            .takes_value(false),
    )
    .arg(
        Arg::with_name("topics")
            .long("topics")
            .value_name("STRING")
            .help("One or more comma-delimited gossipsub topics to subscribe to.")
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
            .help("Log filter.")
            .takes_value(true)
            .possible_values(&["info", "debug", "trace", "warn", "error", "crit"])
            .default_value("info"),
    )
}
