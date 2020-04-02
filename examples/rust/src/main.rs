use std::{thread, time};
use tokio::runtime::Builder;
use tokio::runtime::TaskExecutor;
use tokio_timer::clock::Clock;
use slog::{debug, info, o, warn, Drain};
use env_logger::{Env};
use mothra::network::Network;


fn main() {
    let args = std::env::args().collect();
    let runtime = Builder::new()
    .name_prefix("Example-")
    .clock(Clock::system())
    .build()
    .map_err(|e| format!("{:?}", e)).unwrap();
    let executor = runtime.executor();
    env_logger::Builder::from_env(Env::default()).init();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let slog = slog::Logger::root(drain, o!());
    let network_logger = slog.new(o!("Network" => "Network"));
    let mut network = Network::new(
            args,
            &executor,
            on_discovered_peer,
            on_receive_gossip,
            on_receive_rpc,
            network_logger,
    ).unwrap();

    let dur = time::Duration::from_secs(5);
    loop {
        let topic = "/eth2/beacon_block/ssz".to_string();
        let data = "Hello from Rust".as_bytes().to_vec();
        network.gossip(topic, data);
        thread::sleep(dur);
    }
}


fn on_discovered_peer (peer: String){
    println!("Rust: discovered peer");
    println!("peer={:?}", peer);
}

fn on_receive_gossip (topic: String, data: Vec<u8>){
    println!("Rust: received gossip");
    println!("topic={:?}", topic);
    println!("data={:?}", data);
}

fn on_receive_rpc (method: String, req_resp: u8, peer: String, data: Vec<u8>) { 
    println!("Rust: received rpc");
    println!("method={:?}", method);
    println!("peer={:?}", peer);
    println!("data={:?}", data);
}