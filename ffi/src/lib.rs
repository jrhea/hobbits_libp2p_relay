use cast::i16;
use mothra::{
    network::gossip, network::rpc_request, network::rpc_response, network::NetworkMessage,
    network::NetworkService, NetworkGlobals,
};
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};
use std::sync::Arc;
use std::{slice, str};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};

struct Context {
    runtime: Runtime,
    network_globals: Arc<NetworkGlobals>,
    network_send: mpsc::UnboundedSender<NetworkMessage>,
    network_exit: oneshot::Sender<()>,
    log: slog::Logger,
}

static mut CONTEXT: Vec<Context> = Vec::new();

type DiscoveredPeerType = unsafe extern "C" fn(peer: *const c_uchar, peer_length: i16);
type ReceiveGossipType = unsafe extern "C" fn(
    topic: *const c_uchar,
    topic_length: i16,
    data: *mut c_uchar,
    data_length: i16,
);
type ReceiveRpcType = unsafe extern "C" fn(
    method: *const c_uchar,
    method_length: i16,
    req_resp: i16,
    peer: *const c_uchar,
    peer_length: i16,
    data: *mut c_uchar,
    data_length: i16,
);
static mut DISCOVERED_PEER_PTR: Option<DiscoveredPeerType> = None;
static mut RECEIVE_GOSSIP_PTR: Option<ReceiveGossipType> = None;
static mut RECEIVE_RPC_PTR: Option<ReceiveRpcType> = None;

fn discovered_peer(peer: String) {
    let peer_length = i16(peer.len()).unwrap();
    unsafe { DISCOVERED_PEER_PTR.unwrap()(peer.as_ptr(), peer_length) };
}

fn receive_gossip(topic: String, mut data: Vec<u8>) {
    let topic_length = i16(topic.len()).unwrap();
    let data_length = i16(data.len()).unwrap();
    unsafe {
        RECEIVE_GOSSIP_PTR.unwrap()(topic.as_ptr(), topic_length, data.as_mut_ptr(), data_length)
    };
}

fn receive_rpc(method: String, req_resp: u8, peer: String, mut data: Vec<u8>) {
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
pub unsafe extern "C" fn network_start(
    client_constants: *mut *mut c_char,
    num_client_constants: isize,
    args: *mut *mut c_char,
    num_args: isize,
) {
    let name_cstr = CStr::from_ptr(*client_constants.offset(0));
    let name: Option<String> = match name_cstr.to_str() {
        Ok(s) => {
            if !s.is_empty() {
                Some(s.to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let client_version_cstr = CStr::from_ptr(*client_constants.offset(1));
    let client_version: Option<String> = match client_version_cstr.to_str() {
        Ok(s) => {
            if !s.is_empty() {
                Some(s.to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let protocol_version_cstr = CStr::from_ptr(*client_constants.offset(2));
    let protocol_version: Option<String> = match protocol_version_cstr.to_str() {
        Ok(s) => {
            if !s.is_empty() {
                Some(s.to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let mut args_vec = Vec::<String>::new();
    for idx in 0..num_args {
        let args_cstr = CStr::from_ptr(*args.offset(idx));
        if let Ok(s) = args_cstr.to_str() {
            args_vec.push(s.to_string());
        }
    }

    let runtime = Runtime::new()
        .map_err(|e| format!("Failed to start runtime: {:?}", e))
        .unwrap();
    let (network_globals, network_send, network_exit, log) = NetworkService::new(
        name,
        client_version,
        protocol_version,
        args_vec,
        &runtime.executor(),
        discovered_peer,
        receive_gossip,
        receive_rpc,
    )
    .unwrap();
    CONTEXT.push(Context {
        runtime,
        network_globals,
        network_send,
        network_exit,
        log,
    });
}

#[no_mangle]
pub unsafe extern "C" fn send_gossip(
    topic: *mut c_uchar,
    topic_length: usize,
    data: *mut c_uchar,
    data_length: usize,
) {
    gossip(
        CONTEXT[0].network_send.clone(),
        str::from_utf8_unchecked(slice::from_raw_parts(topic, topic_length)).into(),
        slice::from_raw_parts_mut(data, data_length).to_vec(),
        CONTEXT[0].log.clone(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn send_rpc_request(
    method: *mut c_uchar,
    method_length: usize,
    peer: *mut c_uchar,
    peer_length: usize,
    data: *mut c_uchar,
    data_length: usize,
) {
    rpc_request(
        CONTEXT[0].network_send.clone(),
        str::from_utf8_unchecked(slice::from_raw_parts(method, method_length)).into(),
        str::from_utf8_unchecked(slice::from_raw_parts(peer, peer_length)).into(),
        slice::from_raw_parts_mut(data, data_length).to_vec(),
        CONTEXT[0].log.clone(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn send_rpc_response(
    method: *mut c_uchar,
    method_length: usize,
    peer: *mut c_uchar,
    peer_length: usize,
    data: *mut c_uchar,
    data_length: usize,
) {
    rpc_response(
        CONTEXT[0].network_send.clone(),
        str::from_utf8_unchecked(slice::from_raw_parts(method, method_length)).into(),
        str::from_utf8_unchecked(slice::from_raw_parts(peer, peer_length)).into(),
        slice::from_raw_parts_mut(data, data_length).to_vec(),
        CONTEXT[0].log.clone(),
    );
}
