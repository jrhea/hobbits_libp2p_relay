extern crate target_info;

use target_info::Target;
use std::{thread, time};
use tokio::runtime::Runtime;
use slog::{debug, info, o, warn, Drain};
use mothra::{Mothra, gossip};

fn main() {
    let start = time::Instant::now();
    let args = std::env::args().collect();
    let runtime = Runtime::new()
        .map_err(|e| format!("Failed to start runtime: {:?}", e))
        .unwrap();
    let executor = runtime.executor();
    let config = Mothra::get_config(
        Some("rust-example".into()),
        Some(format!("v{}-unstable",env!("CARGO_PKG_VERSION"))),
        Some("rust-example/libp2p".into()),
        args);
    let (network_globals, network_send, network_exit, network_logger) = Mothra::new(
            config,
            &executor,
            on_discovered_peer,
            on_receive_gossip,
            on_receive_rpc,
    ).unwrap();

    let dur = time::Duration::from_secs(5);
    loop {
        thread::sleep(dur);
        let topic = "/mothra/topic1".to_string();
        let data = format!("Hello from Rust.  Elapsed time: {:?}",start.elapsed()).as_bytes().to_vec();
        gossip(network_send.clone(),topic, data,network_logger.clone());
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