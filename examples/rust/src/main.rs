use std::{thread, time};
use mothra::{register_handlers,network_start,send_gossip};

fn main() {
    let args = std::env::args().collect();
    unsafe{
        register_handlers(
            on_discovered_peer,
            on_receive_gossip,
            on_receive_rpc
        );
    }
    println!("foo");
    unsafe {
        network_start(args);
    }
    println!("foo2");
    let dur = time::Duration::from_secs(5);
    loop{
        thread::sleep(dur);
        let topic = "/eth2/beacon_block/ssz".to_string();
        let data = "Hello from Rust".as_bytes().to_vec();
        send_gossip(topic, data);
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