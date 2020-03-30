use clap::ArgMatches;
use futures::prelude::*;
use std::sync::mpsc as sync;
use std::time::{Duration, Instant};
use std::env;
use std::process;
use std::{thread, time};
use slog::{debug, info, o, warn, Drain};
use tokio::runtime::TaskExecutor;
use tokio::runtime::Builder;
use tokio::timer::Interval;
use tokio_timer::clock::Clock;
use futures::Future;
use clap::{App, Arg, AppSettings};
use libp2p_wrapper::{NetworkConfig,Topic,Message,GOSSIP,DISCOVERY,RPC,RPCRequest,RPCResponse,RPCErrorResponse,RPCEvent,PeerId};
use tokio::sync::mpsc;
use network::{Network,NetworkMessage,OutgoingMessage};

use std::cell::RefCell;
use std::cell::RefMut;
use std::io::Cursor;
use std::convert::TryFrom;
use std::mem;
use std::ops::Range;
use cast::i16;
use cast::i8;
use env_logger::{Env};

pub mod network;
pub mod error;

/// The interval between heartbeat events.
pub const HEARTBEAT_INTERVAL_SECONDS: u64 = 10;

/// Create a warning log whenever the peer count is at or below this value.
pub const WARN_PEER_COUNT: usize = 1;

#[derive(Debug)]
pub struct Global {
    tx: RefCell<Option<sync::Sender<Message>>>
}

pub static mut GLOBAL: Global = Global{ tx:  RefCell::new(None) };

impl Global {
    pub fn set(&mut self, value: &RefCell<Option<sync::Sender<Message>>>) {
        self.tx.swap(value);
    }

    pub fn get(&self) -> RefMut<Option<sync::Sender<Message>>> {
        self.tx.borrow_mut()
    }
}

#[macro_export]
macro_rules! set_tx {
    ($fmt:expr) => (
        unsafe {
            GLOBAL.set($fmt)
        }
    );
}

#[macro_export]
macro_rules! get_tx {
    () => (
        unsafe {
            GLOBAL.get()
        }
    );
}

#[cfg(feature = "api")]
pub mod api {
    use super::*;
    type discovered_peer_type = fn(peer: String);
    type receive_gossip_type = fn(topic: String, data: Vec<u8>);
    type receive_rpc_type =  fn(method: String, req_resp: u8, peer: String, data: Vec<u8>);
    pub static mut s_discovered_peer_ptr: Option<discovered_peer_type> = None;
    pub static mut s_receive_gossip_ptr: Option<receive_gossip_type> = None;
    pub static mut s_receive_rpc_ptr: Option<receive_rpc_type> = None;

    pub unsafe fn discovered_peer (peer: String){
        s_discovered_peer_ptr.unwrap()(peer);
    }
    
    pub unsafe fn receive_gossip (topic: String, data: Vec<u8>) {
        s_receive_gossip_ptr.unwrap()(topic, data);
    }
    
    pub unsafe fn receive_rpc (method: String, req_resp: u8, peer: String, data: Vec<u8>) {
        s_receive_rpc_ptr.unwrap()(method, req_resp, peer, data);
    }

    #[no_mangle]
    pub unsafe fn register_handlers(
            discovered_peer_ptr: fn(peer: String),
            receive_gossip_ptr: fn(topic: String, data: Vec<u8>),
            receive_rpc_ptr: fn(method: String, req_resp: u8, peer: String, data: Vec<u8>)
    ) {
        s_discovered_peer_ptr = Some(discovered_peer_ptr);
        s_receive_gossip_ptr = Some(receive_gossip_ptr);
        s_receive_rpc_ptr = Some(receive_rpc_ptr);
    }

    #[no_mangle]
    pub unsafe fn network_start(args_vec: Vec<String>) {
        env_logger::Builder::from_env(Env::default()).init();
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let slog = slog::Logger::root(drain, o!());
        let log = slog.new(o!("API" => "init()"));
        let args = config(args_vec);
        let (mut tx1, rx1) = sync::channel();

        thread::spawn(move || {
            super::start(args, &rx1, log.new(o!("API" => "start()")));
        });

        set_tx!(&RefCell::new(Some(tx1)));
    }

    pub fn network_receive(mut network_message: Message, log: slog::Logger){
        if network_message.category == GOSSIP.to_string(){
            debug!(log, "received GOSSIP from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
            let topic = network_message.command;
            let data = network_message.value.to_vec();
            unsafe{ api::receive_gossip(topic, data); }
        } else if network_message.category == RPC.to_string(){
            debug!(log, "received RPC from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
            let method =  network_message.command;
            let req_resp = network_message.req_resp;
            let peer = network_message.peer;
            let data = network_message.value.to_vec();
            unsafe{ api::receive_rpc(method, req_resp, peer, data); }
        } else if network_message.category == DISCOVERY.to_string(){
            debug!(log, "discovered peer: {:?}", network_message.peer);
            let peer = network_message.peer;
            unsafe { api::discovered_peer(peer); }
        }
    }

    #[no_mangle]
    pub fn send_gossip(topic: String, data: Vec<u8>) {
        let gossip_data = Message::new(GOSSIP.to_string(),topic,Default::default(),Default::default(),data);
        get_tx!().as_mut().unwrap().send(gossip_data);
    }

    #[no_mangle]
    pub fn send_rpc_request(method: String, peer: String, data: Vec<u8>) {
        send_rpc(method,0,peer,data);
    }

    #[no_mangle]
    pub fn send_rpc_response(method: String, peer: String, data: Vec<u8>) {
        send_rpc(method,1,peer,data);
    }

    #[no_mangle]
    fn send_rpc(method: String, req_resp: u8, peer: String, data: Vec<u8>){
        let rpc_data = Message::new(RPC.to_string(),method,req_resp,peer,data);
        get_tx!().as_mut().unwrap().send(rpc_data);
    }
}

#[cfg(feature = "capi")]
pub mod api {
    use super::*;
    use std::ffi::{CStr,CString};
    use std::os::raw::{c_uchar,c_char,c_int};

    type discovered_peer_type = unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16);
    type receive_gossip_type = unsafe extern "C" fn(topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16);
    type receive_rpc_type =  unsafe extern "C" fn(method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16);
    static mut s_discovered_peer_ptr: Option<discovered_peer_type> = None;
    static mut s_receive_gossip_ptr: Option<receive_gossip_type> = None;
    static mut s_receive_rpc_ptr: Option<receive_rpc_type> = None;

    pub unsafe extern "C" fn discovered_peer (peer_c_uchar: *const c_uchar, peer_length: i16) {
        s_discovered_peer_ptr.unwrap()(peer_c_uchar, peer_length);
    }

    pub unsafe extern "C" fn receive_gossip (topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16) {
        s_receive_gossip_ptr.unwrap()(topic_c_uchar, topic_length, data_c_uchar, data_length);
    }

    pub unsafe extern "C" fn receive_rpc (method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16) {
        s_receive_rpc_ptr.unwrap()(method_c_uchar, method_length, req_resp, peer_c_uchar, peer_length, data_c_uchar, data_length);
    }

    #[no_mangle]
    pub unsafe extern "C" fn register_handlers(
        discovered_peer_ptr: unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16),
        receive_gossip_ptr: unsafe extern "C" fn(topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16), 
        receive_rpc_ptr: unsafe extern "C" fn(method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16)
    ) {
        s_discovered_peer_ptr = Some(discovered_peer_ptr);
        s_receive_gossip_ptr = Some(receive_gossip_ptr);
        s_receive_rpc_ptr = Some(receive_rpc_ptr);
    }

    #[no_mangle]
    pub extern "C" fn network_start(args_c_char: *mut *mut c_char, length: isize) {
        env_logger::Builder::from_env(Env::default()).init();

        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let slog = slog::Logger::root(drain, o!());
        let log = slog.new(o!("API" => "init()"));
        let mut args_vec = Vec::<String>::new();
        for idx in 0..length {
            let args_cstr = unsafe { CStr::from_ptr(*args_c_char.offset(idx)) };
            match args_cstr.to_str() {
                Ok(s) => {
                args_vec.push(s.to_string());
                }
                Err(_) => {
                    warn!(log,"Invalid libp2p config provided! ")
                }
            }
        }
        let args = config(args_vec);
        let (mut tx1, rx1) = sync::channel();

        thread::spawn(move || {
            start(args, &rx1, log.new(o!("API" => "start()")));
        });

        set_tx!(&RefCell::new(Some(tx1)));
    }

    pub fn network_receive(mut network_message: Message, log: slog::Logger){
        if network_message.category == GOSSIP.to_string(){
            debug!(log, "received GOSSIP from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
            let topic_length = i16(network_message.command.len()).unwrap();
            let topic = network_message.command.as_ptr();
            let data_length = i16(network_message.value.len()).unwrap();
            let data = network_message.value.as_mut_ptr();
            unsafe {
                receive_gossip(topic, topic_length, data, data_length);
            }
        } else if network_message.category == RPC.to_string(){
            debug!(log, "received RPC from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
            let method_length = i16(network_message.command.len()).unwrap();
            let method =  network_message.command.as_ptr();
            let req_resp = i16(network_message.req_resp);
            let peer_length = i16(network_message.peer.len()).unwrap();
            let peer = network_message.peer.as_ptr();
            let data_length = i16(network_message.value.len()).unwrap();
            let data = network_message.value.as_mut_ptr();
            unsafe {
                receive_rpc(method, method_length, req_resp, peer, peer_length, data, data_length);
            }
        } else if network_message.category == DISCOVERY.to_string(){
            debug!(log, "discovered peer: {:?}", network_message.peer);
            let peer_length = i16(network_message.peer.len()).unwrap();
            let peer = network_message.peer.as_ptr();
            unsafe {
                discovered_peer(peer, peer_length);
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn send_gossip(topic_c_uchar: *mut c_uchar, topic_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
        let topic = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(topic_c_uchar, topic_length)).to_string() };
        let mut data = unsafe { std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec() };
        let gossip_data = Message::new(GOSSIP.to_string(),topic,Default::default(),Default::default(),data);
        get_tx!().as_mut().unwrap().send(gossip_data);
    }

    #[no_mangle]
    pub extern "C" fn send_rpc_request(method_c_uchar: *mut c_uchar, method_length: usize, peer_c_uchar: *mut c_uchar, peer_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
        let method = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(method_c_uchar, method_length)).to_string() };
        let peer = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length)).to_string() };
        let mut data = unsafe { std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec() };
        let rpc_data = Message::new(RPC.to_string(),method,0,peer,data);
        get_tx!().as_mut().unwrap().send(rpc_data);
    }

    #[no_mangle]
    pub extern "C" fn send_rpc_response(method_c_uchar: *mut c_uchar, method_length: usize, peer_c_uchar: *mut c_uchar, peer_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
        let method = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(method_c_uchar, method_length)).to_string() };
        let peer = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length)).to_string() };
        let mut data = unsafe { std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec() };
        let rpc_data = Message::new(RPC.to_string(),method,1,peer,data);
        get_tx!().as_mut().unwrap().send(rpc_data);
    }
}

pub fn start(args: ArgMatches, local_rx: &sync::Receiver<Message>, log: slog::Logger) {
    info!(log,"Initializing libP2P....");
    let runtime = Builder::new()
        .name_prefix("API-")
        .clock(Clock::system())
        .build()
        .map_err(|e| format!("{:?}", e)).unwrap();
    let executor = runtime.executor();
    let mut network_config = NetworkConfig::new();
    network_config.apply_cli_args(&args).unwrap();
    let network_logger = log.new(o!("Network" => "Network"));
    let (network_tx, network_rx) = sync::channel();
    let (network, network_send) = Network::new(
            network_tx,
            &network_config,
            &executor.clone(),
            network_logger,
    ).unwrap();
    
    monitor(&network, executor, log.clone());
    let dur = time::Duration::from_millis(50);
    loop {
        match local_rx.try_recv(){
            Ok(local_message) => {
                if local_message.category == GOSSIP.to_string(){
                    //debug!(log,  "in mod.rs: sending gossip with topic {:?}",local_message.command);
                    gossip(network_send.clone(),local_message.command,local_message.value.to_vec(),log.new(o!("API" => "gossip()")));
                }
                else if local_message.category == RPC.to_string(){
                    if local_message.req_resp == 0 {
                        //debug!(log,  "in mod.rs: sending request rpc_method of type {:?}",local_message.command);
                        rpc_request(network_send.clone(),local_message.command,local_message.peer,local_message.value.to_vec(),log.new(o!("API" => "rpc()")));
                    } else {
                        //debug!(log,  "in mod.rs: sending response rpc_method of type {:?}",local_message.command);
                        rpc_response(network_send.clone(),local_message.command,local_message.peer,local_message.value.to_vec(),log.new(o!("API" => "rpc()")));
                    }
                }
            }
            Err(_) => {
                
            }
        }
        match network_rx.try_recv(){
            Ok(mut network_message) => {
                api::network_receive(network_message, log.new(o!("API" => "network_receive()")));
            }
            Err(_) => {
    
            }
        }
        thread::sleep(dur);
    }
}

fn monitor(
    network: &Network,
    executor: TaskExecutor,
    log: slog::Logger
) {
    let err_log = log.clone();
    let (_exit_signal, exit) = exit_future::signal();
    // notification heartbeat
    let interval = Interval::new(
        Instant::now(),
        Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS),
    );

    let libp2p = network.libp2p_service();

    let heartbeat = move |_| {

        let connected_peer_count = libp2p.lock().swarm.num_connected_peers();

        debug!(log, "libp2p"; "peer_count" => connected_peer_count);

        if connected_peer_count <= WARN_PEER_COUNT {
            warn!(log, "Low libp2p peer count"; "peer_count" => connected_peer_count);
        }

        Ok(())
    };

    // map error and spawn
    let heartbeat_interval = interval
        .map_err(move |e| debug!(err_log, "Timer error {}", e))
        .for_each(heartbeat);
    executor.spawn(exit.until(heartbeat_interval).map(|_| ()));

}

fn gossip( mut network_send: mpsc::UnboundedSender<NetworkMessage>, topic: String, data: Vec<u8>, log: slog::Logger){
    network_send.try_send(NetworkMessage::Publish {
                topics: vec![Topic::new(topic)],
                message: data,})
                .unwrap_or_else(|_| {
                    warn!(
                        log,
                        "Could not send gossip message."
                    )
                });
}

fn rpc_request( mut network_send: mpsc::UnboundedSender<NetworkMessage>, method: String, peer: String, data: Vec<u8>, log: slog::Logger){
    // use 0 as the default request id, when an ID is not required.
    let request_id: usize = 0;
    let rpc_request: RPCRequest =  RPCRequest::Message(data);
    let rpc_event: RPCEvent = RPCEvent::Request(request_id, rpc_request);
    let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
    let peer_id = PeerId::from_bytes(bytes).map_err(|_|()).unwrap();
    network_send.try_send(NetworkMessage::Send(peer_id,OutgoingMessage::RPC(rpc_event)))
                .unwrap_or_else(|_| {
                    warn!(
                        log,
                        "Could not send RPC message to the network service"
                    )
                });
}

fn rpc_response( mut network_send: mpsc::UnboundedSender<NetworkMessage>, method: String, peer: String, data: Vec<u8>, log: slog::Logger){
    // use 0 as the default request id, when an ID is not required.
    let request_id: usize = 0;
    let rpc_response: RPCResponse =  RPCResponse::Message(data);
    let rpc_event: RPCEvent = RPCEvent::Response(request_id,RPCErrorResponse::Success(rpc_response));
    let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
    let peer_id = PeerId::from_bytes(bytes).map_err(|_|()).unwrap();
    network_send.try_send(NetworkMessage::Send(peer_id,OutgoingMessage::RPC(rpc_event)))
                .unwrap_or_else(|_| {
                    warn!(
                        log,
                        "Could not send RPC message to the network service"
                    )
                });
}

pub fn config(args: Vec<String>) -> ArgMatches<'static> {
    
    App::new("Mothra")
    .version("0.0.1")
    .author("Your Mom")
    .about("LibP2P for Dummies")
    .setting(AppSettings::TrailingVarArg)
    .setting(AppSettings::DontDelimitTrailingValues)
    .arg(
        Arg::with_name("datadir")
            .long("datadir")
            .value_name("DIR")
            .help("Data directory for keys and databases.")
            .takes_value(true)
    )
    // network related arguments
    .arg(
        Arg::with_name("listen-address")
            .long("listen-address")
            .value_name("ADDRESS")
            .help("The address the client will listen for UDP and TCP connections. (default 127.0.0.1).")
            .default_value("127.0.0.1")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("port")
            .long("port")
            .value_name("PORT")
            .help("The TCP/UDP port to listen on. The UDP port can be modified by the --discovery-port flag.")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("maxpeers")
            .long("maxpeers")
            .help("The maximum number of peers (default 10).")
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
        Arg::with_name("discovery-port")
            .long("disc-port")
            .value_name("PORT")
            .help("The discovery UDP port.")
            .default_value("9000")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("discovery-address")
            .long("discovery-address")
            .value_name("ADDRESS")
            .help("The IP address to broadcast to other peers on how to reach this node.")
            .takes_value(true),
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
            .help("Possible values: info, debug, trace, warn, error, crit")
            .takes_value(true)
            .possible_values(&["info", "debug", "trace", "warn", "error", "crit"])
            .default_value("info"),
    )
    .arg(
        Arg::with_name("verbosity")
            .short("v")
            .multiple(true)
            .help("Sets the verbosity level")
            .takes_value(true),
    )
   .get_matches_from_safe(args.iter())
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1);
        })
}