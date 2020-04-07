use crate::{network, network::NetworkService, network::NetworkMessage};
use libp2p_wrapper::NetworkGlobals;
use slog::{debug, info, o, warn, Drain};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};
use cast::i16;
use env_logger::Env;
use std::sync::Arc;
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};

struct Context {
    runtime: Runtime,
    network_globals:  Arc<NetworkGlobals>,
    network_send: mpsc::UnboundedSender<NetworkMessage>,
    network_exit: oneshot::Sender<()>,
    log: slog::Logger,
}

static mut CONTEXT: Vec<Context> = Vec::new();

type DiscoveredPeerType = unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16);
type ReceiveGossipType = unsafe extern "C" fn(
    topic_c_uchar: *const c_uchar,
    topic_length: i16,
    data_c_uchar: *mut c_uchar,
    data_length: i16,
);
type ReceiveRpcType = unsafe extern "C" fn(
    method_c_uchar: *const c_uchar,
    method_length: i16,
    req_resp: i16,
    peer_c_uchar: *const c_uchar,
    peer_length: i16,
    data_c_uchar: *mut c_uchar,
    data_length: i16,
);
static mut DISCOVERED_PEER_PTR: Option<DiscoveredPeerType> = None;
static mut RECEIVE_GOSSIP_PTR: Option<ReceiveGossipType> = None;
static mut RECEIVE_RPC_PTR: Option<ReceiveRpcType> = None;

pub fn discovered_peer(peer: String) {
    let peer_length = i16(peer.len()).unwrap();
    unsafe { DISCOVERED_PEER_PTR.unwrap()(peer.as_ptr(), peer_length) };
}

pub fn receive_gossip(topic: String, mut data: Vec<u8>) {
    let topic_length = i16(topic.len()).unwrap();
    let data_length = i16(data.len()).unwrap();
    unsafe {
        RECEIVE_GOSSIP_PTR.unwrap()(topic.as_ptr(), topic_length, data.as_mut_ptr(), data_length)
    };
}

pub fn receive_rpc(method: String, req_resp: u8, peer: String, mut data: Vec<u8>) {
    let method_length = i16(method.len()).unwrap();
    let peer_length = i16(peer.len()).unwrap();
    let data_length = i16(data.len()).unwrap();
    unsafe {
        RECEIVE_RPC_PTR.unwrap()(
            method.as_ptr(),
            method_length,
            i16(req_resp),
            peer.as_ptr(),
            peer_length,
            data.as_mut_ptr(),
            data_length,
        )
    };
}

#[no_mangle]
pub unsafe extern "C" fn register_handlers(
    discovered_peer: DiscoveredPeerType,
    receive_gossip: ReceiveGossipType,
    receive_rpc: ReceiveRpcType,
) {
    DISCOVERED_PEER_PTR = Some(discovered_peer);
    RECEIVE_GOSSIP_PTR = Some(receive_gossip);
    RECEIVE_RPC_PTR = Some(receive_rpc);
}

#[no_mangle]
pub unsafe extern "C" fn network_start(args_c_char: *mut *mut c_char, length: isize) {
    env_logger::Builder::from_env(Env::default()).init();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let slog = slog::Logger::root(drain, o!());
    let log = slog.new(o!("Network" => "Network"));
    let mut args_vec = Vec::<String>::new();
    for idx in 0..length {
        let args_cstr = CStr::from_ptr(*args_c_char.offset(idx));
        match args_cstr.to_str() {
            Ok(s) => {
                args_vec.push(s.to_string());
            }
            Err(_) => warn!(log, "Invalid libp2p config provided! "),
        }
    }
    let runtime = Runtime::new()
        .map_err(|e| format!("Failed to start runtime: {:?}", e))
        .unwrap();
    let (network_globals, network_send, network_exit) = NetworkService::new(
        args_vec,
        &runtime.executor(),
        discovered_peer,
        receive_gossip,
        receive_rpc,
        log.clone(),
    ).unwrap();
    CONTEXT.push(Context {
        runtime,
        network_globals,
        network_send,
        network_exit,
        log: log.clone(),
    });
}

#[no_mangle]
pub unsafe extern "C" fn send_gossip(
    topic_c_uchar: *mut c_uchar,
    topic_length: usize,
    data_c_uchar: *mut c_uchar,
    data_length: usize,
) {
    let topic =
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(topic_c_uchar, topic_length))
            .to_string();
    let data = std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec();
    network::gossip(CONTEXT[0].network_send.clone(),topic, data, CONTEXT[0].log.clone());
}

#[no_mangle]
pub unsafe extern "C" fn send_rpc_request(
    method_c_uchar: *mut c_uchar,
    method_length: usize,
    peer_c_uchar: *mut c_uchar,
    peer_length: usize,
    data_c_uchar: *mut c_uchar,
    data_length: usize,
) {
    let method = std::str::from_utf8_unchecked(std::slice::from_raw_parts(
        method_c_uchar,
        method_length,
    ))
    .to_string();
    let peer =
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length))
            .to_string();
    let data = std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec();
    network::rpc_request(CONTEXT[0].network_send.clone(),method, peer, data,CONTEXT[0].log.clone());
}

#[no_mangle]
pub unsafe extern "C" fn send_rpc_response(
    method_c_uchar: *mut c_uchar,
    method_length: usize,
    peer_c_uchar: *mut c_uchar,
    peer_length: usize,
    data_c_uchar: *mut c_uchar,
    data_length: usize,
) {
    let method = std::str::from_utf8_unchecked(std::slice::from_raw_parts(
        method_c_uchar,
        method_length,
    ))
    .to_string();
    let peer =
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length))
            .to_string();
    let data = std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec();
    network::rpc_response(CONTEXT[0].network_send.clone(),method, peer, data, CONTEXT[0].log.clone());
}
