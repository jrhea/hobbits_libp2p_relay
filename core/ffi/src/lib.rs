use cast::i16;
use env_logger::Env;
use mothra::{cli_app, gossip, rpc_request, rpc_response, Mothra, NetworkGlobals, NetworkMessage, Subscriber};
use slog::{debug, info, o, trace, warn, Drain, Level, Logger};
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};
use std::sync::Arc;
use std::{process, slice, str};
use tokio::sync::{mpsc, oneshot};
use tokio_compat::runtime::Runtime;

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
    message_id: *const c_uchar,
    message_id_length: i16,
    peer_id: *const c_uchar,
    peer_id_length: i16,
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

struct Client;

impl Client {
    pub fn new() -> Self {
        Client{}
    }
}

impl Subscriber for Client {
    fn discovered_peer(&self, peer: String) {
        let peer_length = i16(peer.len()).unwrap();
        unsafe { DISCOVERED_PEER_PTR.unwrap()(peer.as_ptr(), peer_length) };
    }

    fn receive_gossip(&self, message_id: String, peer_id: String, topic: String, mut data: Vec<u8>) {
        let message_id_length = i16(topic.len()).unwrap();
        let peer_id_length = i16(topic.len()).unwrap();
        let topic_length = i16(topic.len()).unwrap();
        let data_length = i16(data.len()).unwrap();
        unsafe {
            RECEIVE_GOSSIP_PTR.unwrap()(message_id.as_ptr(), message_id_length, peer_id.as_ptr(), peer_id_length, topic.as_ptr(), topic_length, data.as_mut_ptr(), data_length)
        };
    }

    fn receive_rpc(&self, method: String, req_resp: u8, peer: String, mut data: Vec<u8>) {
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
    let client_name_cstr = CStr::from_ptr(*client_constants.offset(0));
    let client_name: Option<String> = match client_name_cstr.to_str() {
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
    let matches = cli_app()
        .get_matches_from_safe(args_vec.iter())
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1);
        });

    let config = Mothra::get_config(client_name, client_version, protocol_version, &matches);
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
    let log = slog.new(o!("FFI" => "Mothra"));
    // build the current enr_fork_id for adding to our local ENR
    //TODO
    let enr_fork_id = [0u8; 32].to_vec();
    let client = Box::new(Client::new()) as Box<dyn Subscriber + Send>;
    let (network_globals, network_send, network_exit) = Mothra::new(
        config,
        enr_fork_id,
        &runtime.executor(),
        client,
        log.clone(),
    )
    .unwrap();
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
