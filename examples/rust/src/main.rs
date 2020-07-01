extern crate target_info;
use clap::{App, AppSettings, Arg, ArgMatches};
use env_logger::Env;
use mothra::{cli_app, gossip, Mothra, Subscriber};
use slog::{debug, info, o, trace, warn, Drain, Level, Logger};
use std::{thread, time};
use tokio_compat::runtime::Runtime;

struct Client;

impl Client {
    pub fn new() -> Self {
        Client {}
    }
}

impl Subscriber for Client {
    fn discovered_peer(&self, peer: String) {
        println!("Rust: discovered peer");
        println!("peer={:?}", peer);
    }

    fn receive_gossip(&self, message_id: String, peer_id: String, topic: String, data: Vec<u8>) {
        println!("Rust: received gossip");
        println!("message id={:?}", message_id);
        println!("peer id={:?}", peer_id);
        println!("topic={:?}", topic);
        println!("data={:?}", String::from_utf8_lossy(&data));
    }

    fn receive_rpc(&self, method: String, req_resp: u8, peer: String, data: Vec<u8>) {
        println!("Rust: received rpc");
        println!("method={:?}", method);
        println!("req_resp={:?}", req_resp);
        println!("peer={:?}", peer);
        println!("data={:?}", String::from_utf8_lossy(&data));
    }
}

fn main() {
    let start = time::Instant::now();
    // Parse the CLI parameters.
    let matches = App::new("rust-example")
        .version(clap::crate_version!())
        .author("Jonny Rhea")
        .about("Mothra example app")
        .arg(
            Arg::with_name("foo")
                .long("foo")
                .short("f")
                .value_name("FOO")
                .help("This is a dummy option.")
                .takes_value(false),
        )
        .subcommand(cli_app())
        .get_matches();

    if matches.is_present("foo") {
        println!("Foo flag found");
    }

    let runtime = Runtime::new()
        .map_err(|e| format!("Failed to start runtime: {:?}", e))
        .unwrap();
    let executor = runtime.executor();
    let config = Mothra::get_config(
        Some("rust-example".into()),
        Some(format!("v{}-unstable", env!("CARGO_PKG_VERSION"))),
        Some("rust-example/libp2p".into()),
        &matches.subcommand_matches("mothra").unwrap(),
    );
    // configure logging
    env_logger::Builder::from_env(Env::default()).init();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build();
    let drain = match config.debug_level.as_str() {
        "info" => drain.filter_level(Level::Info),
        "debug" => drain.filter_level(Level::Debug),
        "trace" => drain.filter_level(Level::Trace),
        "warn" => drain.filter_level(Level::Warning),
        "error" => drain.filter_level(Level::Error),
        "crit" => drain.filter_level(Level::Critical),
        _ => drain.filter_level(Level::Info),
    };
    let slog = Logger::root(drain.fuse(), o!());
    let log = slog.new(o!("Rust-Example" => "Mothra"));
    let enr_fork_id = [0u8; 32].to_vec();
    let client = Box::new(Client::new()) as Box<dyn Subscriber + Send>;
    let (network_globals, network_send, network_exit) =
        Mothra::new(config, enr_fork_id, &executor, client, log.clone()).unwrap();

    let dur = time::Duration::from_secs(5);
    loop {
        thread::sleep(dur);
        let topic = "/mothra/topic1".to_string();
        let data = format!("Hello from Rust.  Elapsed time: {:?}", start.elapsed())
            .as_bytes()
            .to_vec();
        gossip(network_send.clone(), topic, data, log.clone());
    }
}
