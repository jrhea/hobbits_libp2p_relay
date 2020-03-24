pub mod mothra_api;
extern crate getopts;
use std::sync::mpsc::{channel,Sender,Receiver};
use std::{thread, time};
use std::ffi::{CStr,CString};
use std::os::raw::{c_uchar,c_char,c_int};
use std::io::Cursor;
use std::convert::TryFrom;
use std::mem;
use std::ops::Range;
use std::cell::RefCell;
use std::cell::RefMut;
use cast::i16;
use cast::i8;
use slog::{info, debug, warn, o, Drain};
use env_logger::{Builder, Env};
use libp2p_wrapper::{Message,GOSSIP,RPC,DISCOVERY};
use mothra_api::api;

#[derive(Debug)]
struct Global {
    tx: RefCell<Option<Sender<Message>>>
}

static mut GLOBAL: Global = Global{ tx:  RefCell::new(None) };

impl Global {
    fn set(&mut self, value: &RefCell<Option<Sender<Message>>>) {
        self.tx.swap(value);
    }

    fn get(&self) -> RefMut<Option<Sender<Message>>> {
        self.tx.borrow_mut()
    }
}

macro_rules! set_tx {
    ($fmt:expr) => (
        unsafe {
            GLOBAL.set($fmt)
        }
    );
}

macro_rules! get_tx {
    () => (
        unsafe {
            GLOBAL.get()
        }
    );
}

type discovered_peer_type = unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16);
type receive_gossip_type = unsafe extern "C" fn(topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16);
type receive_rpc_type =  unsafe extern "C" fn(method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16);
static mut s_discovered_peer_ptr: Option<discovered_peer_type> = None;
static mut s_receive_gossip_ptr: Option<receive_gossip_type> = None;
static mut s_receive_rpc_ptr: Option<receive_rpc_type> = None;

#[no_mangle]
pub unsafe extern "C" fn libp2p_register_handlers(
        discovered_peer_ptr: unsafe extern "C" fn(peer_c_uchar: *const c_uchar, peer_length: i16),
        receive_gossip_ptr: unsafe extern "C" fn(topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16), 
        receive_rpc_ptr: unsafe extern "C" fn(method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16)
    ) {
        s_discovered_peer_ptr = Some(discovered_peer_ptr);
        s_receive_gossip_ptr = Some(receive_gossip_ptr);
        s_receive_rpc_ptr = Some(receive_rpc_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn discovered_peer (peer_c_uchar: *const c_uchar, peer_length: i16) {
    s_discovered_peer_ptr.unwrap()(peer_c_uchar, peer_length);
}

#[no_mangle]
pub unsafe extern "C" fn receive_gossip (topic_c_uchar: *const c_uchar, topic_length: i16, data_c_uchar: *mut c_uchar, data_length: i16) {
    s_receive_gossip_ptr.unwrap()(topic_c_uchar, topic_length, data_c_uchar, data_length);
}

#[no_mangle]
pub unsafe extern "C" fn receive_rpc (method_c_uchar: *const c_uchar, method_length: i16, req_resp: i16, peer_c_uchar: *const c_uchar, peer_length: i16, data_c_uchar: *mut c_uchar, data_length: i16) {
    s_receive_rpc_ptr.unwrap()(method_c_uchar, method_length, req_resp, peer_c_uchar, peer_length, data_c_uchar, data_length);
}

#[no_mangle]
pub extern "C" fn libp2p_start(args_c_char: *mut *mut c_char, length: isize) {
    Builder::from_env(Env::default()).init();

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
    let args = api::config(args_vec);
    let (mut tx1, rx1) = channel();
    let (tx2, rx2) = channel();

    let nlog = log.clone();
    thread::spawn(move || {
        api::start(args, &tx2, &rx1, nlog.new(o!("API" => "start()")));
    });

    set_tx!(&RefCell::new(Some(tx1)));

    ///Listen for messages rcvd from the network
    thread::spawn(move || {
        loop{
            match rx2.recv(){
                Ok(mut network_message) => {
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
                        //debug!(log, "received RPC from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
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
                        //debug!(log, "discovered peer: {:?}", network_message.peer);
                        let peer_length = i16(network_message.peer.len()).unwrap();
                        let peer = network_message.peer.as_ptr();
                        unsafe {
                            discovered_peer(peer, peer_length);
                        }
                    }
                }
                Err(_) => {
                    std::println!("Rcv Thread Error: rx2.recv().unwrap()");
                }
            }
        }     
    });

}

#[no_mangle]
pub extern "C" fn libp2p_send_gossip(topic_c_uchar: *mut c_uchar, topic_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
    let topic = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(topic_c_uchar, topic_length)) };
    let mut data = unsafe { std::slice::from_raw_parts_mut(data_c_uchar, data_length) };
    let gossip_data = Message::new(GOSSIP.to_string(),topic.to_string(),Default::default(),Default::default(),data.to_vec());
    get_tx!().as_mut().unwrap().send(gossip_data);
}

#[no_mangle]
pub extern "C" fn libp2p_send_rpc_request(method_c_uchar: *mut c_uchar, method_length: usize, peer_c_uchar: *mut c_uchar, peer_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
    //std::println!("In libp2p_send_rpc_request");
    let method = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(method_c_uchar, method_length)) };
    let peer = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length)) };
    let mut data = unsafe { std::slice::from_raw_parts_mut(data_c_uchar, data_length) };
    let rpc_data = Message::new(RPC.to_string(),method.to_string(),0,peer.to_string(),data.to_vec());
    get_tx!().as_mut().unwrap().send(rpc_data);
}

#[no_mangle]
pub extern "C" fn libp2p_send_rpc_response(method_c_uchar: *mut c_uchar, method_length: usize, peer_c_uchar: *mut c_uchar, peer_length: usize, data_c_uchar: *mut c_uchar, data_length: usize) {
    //std::println!("In libp2p_send_rpc_response");
    let method = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(method_c_uchar, method_length)) };
    let peer = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(peer_c_uchar, peer_length)) };
    let mut data = unsafe { std::slice::from_raw_parts_mut(data_c_uchar, data_length) };
    let rpc_data = Message::new(RPC.to_string(),method.to_string(),1,peer.to_string(),data.to_vec());
    get_tx!().as_mut().unwrap().send(rpc_data);
}