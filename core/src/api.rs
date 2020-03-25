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
use crate::mothra_api::{config,start};

#[derive(Debug)]
pub struct Global {
    tx: RefCell<Option<Sender<Message>>>
}

pub static mut GLOBAL: Global = Global{ tx:  RefCell::new(None) };

impl Global {
    pub fn set(&mut self, value: &RefCell<Option<Sender<Message>>>) {
        self.tx.swap(value);
    }

    pub fn get(&self) -> RefMut<Option<Sender<Message>>> {
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

type discovered_peer_type = fn(peer: String);
type receive_gossip_type = fn(topic: String, data: Vec<u8>);
type receive_rpc_type =  fn(method: String, req_resp: u8, peer: String, data: Vec<u8>);
static mut s_discovered_peer_ptr: Option<discovered_peer_type> = None;
static mut s_receive_gossip_ptr: Option<receive_gossip_type> = None;
static mut s_receive_rpc_ptr: Option<receive_rpc_type> = None;

pub unsafe fn libp2p_register_handlers(
        discovered_peer_ptr: fn(peer: String),
        receive_gossip_ptr: fn(topic: String, data: Vec<u8>),
        receive_rpc_ptr: fn(method: String, req_resp: u8, peer: String, data: Vec<u8>)
    ) {
        s_discovered_peer_ptr = Some(discovered_peer_ptr);
        s_receive_gossip_ptr = Some(receive_gossip_ptr);
        s_receive_rpc_ptr = Some(receive_rpc_ptr);
}

pub unsafe fn discovered_peer (peer: String){
    s_discovered_peer_ptr.unwrap()(peer);
}

pub unsafe fn receive_gossip (topic: String, data: Vec<u8>) {
    s_receive_gossip_ptr.unwrap()(topic, data);
}

pub unsafe fn receive_rpc (method: String, req_resp: u8, peer: String, data: Vec<u8>) {
    s_receive_rpc_ptr.unwrap()(method, req_resp, peer, data);
}

pub unsafe fn network_start(args_vec: Vec<String>) {
    Builder::from_env(Env::default()).init();
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let slog = slog::Logger::root(drain, o!());
    let log = slog.new(o!("API" => "init()"));
    let args = config(args_vec);
    let (mut tx1, rx1) = channel();
    let (tx2, rx2) = channel();

    let nlog = log.clone();
    thread::spawn(move || {
        start(args, &tx2, &rx1, nlog.new(o!("API" => "start()")));
    });

    set_tx!(&RefCell::new(Some(tx1)));

    ///Listen for messages rcvd from the network
    thread::spawn(move || {
        loop{
            match rx2.recv(){
                Ok(mut network_message) => {
                    if network_message.category == GOSSIP.to_string(){
                        debug!(log, "received GOSSIP from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
                        let topic = network_message.command;
                        let data = network_message.value.to_vec();
                        receive_gossip(topic, data);
                    } else if network_message.category == RPC.to_string(){
                        //debug!(log, "received RPC from peer: {:?} method: {:?} req/resp: {:?}", network_message.peer,network_message.command,network_message.req_resp);
                        let method =  network_message.command;
                        let req_resp = network_message.req_resp;
                        let peer = network_message.peer;
                        let data = network_message.value.to_vec();
                        receive_rpc(method, req_resp, peer, data);
                    } else if network_message.category == DISCOVERY.to_string(){
                        //debug!(log, "discovered peer: {:?}", network_message.peer);
                        let peer = network_message.peer;
                        discovered_peer(peer);
                    }
                }
                Err(_) => {
                    std::println!("Rcv Thread Error: rx2.recv().unwrap()");
                }
            }
        }     
    });

}

pub fn send_gossip(topic: String, data: Vec<u8>) {
    let gossip_data = Message::new(GOSSIP.to_string(),topic,Default::default(),Default::default(),data);
    get_tx!().as_mut().unwrap().send(gossip_data);
}

pub fn send_rpc_request(method: String, peer: String, data: Vec<u8>) {
    send_rpc(method,0,peer,data);
}

pub fn send_rpc_response(method: String, peer: String, data: Vec<u8>) {
    send_rpc(method,1,peer,data);
}

fn send_rpc(method: String, req_resp: u8, peer: String, data: Vec<u8>){
    let rpc_data = Message::new(RPC.to_string(),method,req_resp,peer,data);
    get_tx!().as_mut().unwrap().send(rpc_data);
}