use clap::ArgMatches;
use futures::prelude::*;
use std::sync::mpsc as sync;
use std::time::{Duration, Instant};
use std::env;
use std::process;
use std::sync::Arc;
use std::{thread, time};
use slog::{debug, info, o, warn, Drain};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime, TaskExecutor};
use tokio::timer::Interval;
use tokio_timer::clock::Clock;
use futures::Future;
use clap::{App, Arg, AppSettings};
use libp2p_wrapper::{NetworkConfig,Topic,RPCRequest,RPCResponse,RPCErrorResponse,RPCEvent,PeerId};
use tokio::sync::mpsc;
use crate::network::{Network,NetworkMessage,OutgoingMessage,GOSSIP,DISCOVERY,RPC};

use std::cell::RefCell;
use std::cell::RefMut;
use std::io::Cursor;
use std::convert::TryFrom;
use std::mem;
use std::ops::Range;
use cast::i16;
use cast::i8;
use env_logger::{Env};
use std::ffi::{CStr,CString};
use std::os::raw::{c_uchar,c_char,c_int};

struct Context {
    runtime: Runtime,
    network_service: Network,
}

static mut context: Vec<Context> = Vec::new();

type discovered_peer_type = unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16);
type receive_gossip_type = unsafe extern "C" fn(topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16);
type receive_rpc_type =  unsafe extern "C" fn(method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16);
static mut discovered_peer_ptr: Option<discovered_peer_type> = None;
static mut receive_gossip_ptr: Option<receive_gossip_type> = None;
static mut receive_rpc_ptr: Option<receive_rpc_type> = None;

pub fn discovered_peer(peer: String){
    let peer_length = i16(peer.len()).unwrap();
    unsafe { discovered_peer_ptr.unwrap()(peer.as_ptr(), peer_length) };
}

pub fn receive_gossip (topic: String, mut data: Vec<u8>) {
    let topic_length = i16(topic.len()).unwrap();
    let data_length = i16(data.len()).unwrap();
    unsafe { receive_gossip_ptr.unwrap()(topic.as_ptr(), topic_length, data.as_mut_ptr(), data_length) };
}

pub fn receive_rpc (method: String, req_resp: u8, peer: String, mut data: Vec<u8>) {
    let method_length = i16(method.len()).unwrap();
    let peer_length = i16(peer.len()).unwrap();
    let data_length = i16(data.len()).unwrap();
    unsafe { receive_rpc_ptr.unwrap()(method.as_ptr(), method_length, i16(req_resp), peer.as_ptr(), peer_length, data.as_mut_ptr(), data_length) };
}

#[no_mangle]
pub unsafe extern "C" fn register_handlers(
    discovered_peer: unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16),
    receive_gossip: unsafe extern "C" fn(topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16), 
    receive_rpc: unsafe extern "C" fn(method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16)
) {
    discovered_peer_ptr = Some(discovered_peer);
    receive_gossip_ptr = Some(receive_gossip);
    receive_rpc_ptr = Some(receive_rpc);
}

#[no_mangle]
pub extern "C" fn network_start(args_c_char: *mut *mut c_char, length: isize) {
    env_logger::Builder::from_env(Env::default()).init();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let slog = slog::Logger::root(drain, o!());
    let log = slog.new(o!("Network" => "Network"));
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
    let runtime = Runtime::new().map_err(|e| format!("Failed to start runtime: {:?}", e)).unwrap();
    let network_service = Network::new(
            args_vec,
            &runtime.executor(),
            discovered_peer,
            receive_gossip,
            receive_rpc,
            log.clone(),
    ).unwrap();
    unsafe {
        context.push(Context {
            runtime: runtime,
            network_service: network_service,
        })
    };
}

#[no_mangle]
pub extern "C" fn send_gossip(topic_c_uchar: *mut c_uchar, topic_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
    unsafe {
        let topic = std::str::from_utf8_unchecked(std::slice::from_raw_parts(topic_c_uchar, topic_length)).to_string();
        let mut data = std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec();
        context[0].network_service.gossip(topic,data);
    }
}

#[no_mangle]
pub extern "C" fn send_rpc_request(method_c_uchar: *mut c_uchar, method_length: usize, peer_c_uchar: *mut c_uchar, peer_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
    unsafe {
        let method = std::str::from_utf8_unchecked(std::slice::from_raw_parts(method_c_uchar, method_length)).to_string();
        let peer = std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length)).to_string();
        let mut data = std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec();
        context[0].network_service.rpc_request(method, peer, data);
    }
}

#[no_mangle]
pub extern "C" fn send_rpc_response(method_c_uchar: *mut c_uchar, method_length: usize, peer_c_uchar: *mut c_uchar, peer_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
    unsafe {
        let method = std::str::from_utf8_unchecked(std::slice::from_raw_parts(method_c_uchar, method_length)).to_string();
        let peer = std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length)).to_string();
        let mut data = std::slice::from_raw_parts_mut(data_c_uchar, data_length).to_vec();
        context[0].network_service.rpc_response(method, peer, data);
    }
}    
