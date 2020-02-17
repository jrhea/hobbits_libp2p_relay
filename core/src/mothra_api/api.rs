use clap::ArgMatches;
use futures::prelude::*;
use std::sync::mpsc as sync;
use std::time::{Duration, Instant};
use std::env;
use std::process;
use std::{thread, time};
use slog::{debug, info, o, warn};
use tokio::runtime::TaskExecutor;
use tokio::runtime::Builder;
use tokio::timer::Interval;
use tokio_timer::clock::Clock;
use futures::Future;
use clap::{App, Arg, AppSettings};
use libp2p_wrapper::{NetworkConfig,Topic,Message,GOSSIP,RPC,RPCRequest,RPCResponse,RPCErrorResponse,RPCEvent,PeerId};
use tokio::sync::mpsc;
use super::network::{Network,NetworkMessage,OutgoingMessage};

/// The interval between heartbeat events.
pub const HEARTBEAT_INTERVAL_SECONDS: u64 = 10;

/// Create a warning log whenever the peer count is at or below this value.
pub const WARN_PEER_COUNT: usize = 1;

pub fn start(args: ArgMatches, local_tx: &sync::Sender<Message>,local_rx: &sync::Receiver<Message>, log: slog::Logger) {
    info!(log,"Initializing libP2P....");
    let runtime = Builder::new()
        .name_prefix("api-")
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
                    //debug!(log,  "in api.rs: sending gossip with topic {:?}",local_message.command);
                    gossip(network_send.clone(),local_message.command,local_message.value.to_vec(),log.new(o!("API" => "gossip()")));
                }
                else if local_message.category == RPC.to_string(){
                    if local_message.req_resp == 0 {
                        //debug!(log,  "in api.rs: sending request rpc_method of type {:?}",local_message.command);
                        rpc_request(network_send.clone(),local_message.command,local_message.peer,local_message.value.to_vec(),log.new(o!("API" => "rpc()")));
                    } else {
                        //debug!(log,  "in api.rs: sending response rpc_method of type {:?}",local_message.command);
                        rpc_response(network_send.clone(),local_message.command,local_message.peer,local_message.value.to_vec(),log.new(o!("API" => "rpc()")));
                    }
                }
            }
            Err(_) => {
                
            }
        }
        match network_rx.try_recv(){
            Ok(network_message) => {
                //debug!(log,  "in api.rs: receiving message {:?} {:?}",network_message.category,network_message.command);
                local_tx.send(network_message).unwrap();
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