use std::{thread, time};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};
use std::sync::Arc;
use slog::{debug, info, o, warn, Drain};
use env_logger::{Env};
use mothra::{network, network::NetworkService, network::NetworkMessage, NetworkGlobals};



fn main() {
    let args = std::env::args().collect();
    let runtime = Runtime::new()
        .map_err(|e| format!("Failed to start runtime: {:?}", e))
        .unwrap();
    let executor = runtime.executor();
    env_logger::Builder::from_env(Env::default()).init();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let slog = slog::Logger::root(drain, o!());
    let network_logger = slog.new(o!("Example" => "Network"));
    let (network_globals, network_send, network_exit) = NetworkService::new(
            args,
            &executor,
            on_discovered_peer,
            on_receive_gossip,
            on_receive_rpc,
            network_logger.clone(),
    ).unwrap();

    let dur = time::Duration::from_secs(5);
    loop {
        thread::sleep(dur);
        let topic = "/eth2/beacon_block/ssz".to_string();
        let data = "Hello from Rust".as_bytes().to_vec();
        network::gossip(network_send.clone(),topic, data,network_logger.clone());
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
    println!("req_resp={:?}", req_resp);
    println!("peer={:?}", peer);
    println!("data={:?}", data);
}