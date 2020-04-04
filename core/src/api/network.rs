use super::error;
use clap::ArgMatches;
use clap::{App, AppSettings, Arg};
use futures::prelude::*;
use futures::Stream;
use libp2p_wrapper::Service as LibP2PService;
use libp2p_wrapper::Topic;
use libp2p_wrapper::{Libp2pEvent, PeerId};
use libp2p_wrapper::{NetworkConfig, RPCErrorResponse, RPCEvent, RPCRequest, RPCResponse};
use parking_lot::Mutex;
use slog::{debug, info, o, warn};
use std::sync::mpsc as sync;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{process, thread, time};
use tokio::runtime::TaskExecutor;
use tokio::sync::{mpsc, oneshot};

pub const GOSSIP: &str = "GOSSIP";
pub const RPC: &str = "RPC";
pub const DISCOVERY: &str = "DISCOVERY";

type discovered_peer_type = fn(peer: String);
type receive_gossip_type = fn(topic: String, data: Vec<u8>);
type receive_rpc_type = fn(method: String, req_resp: u8, peer: String, data: Vec<u8>);

pub struct Network {
    libp2p_service: Arc<Mutex<LibP2PService>>,
    _libp2p_exit: oneshot::Sender<()>,
    network_send: mpsc::UnboundedSender<NetworkMessage>,
    log: slog::Logger,
}

impl Network {
    pub fn new(
        args_vec: Vec<String>,
        executor: &TaskExecutor,
        discovered_peer: discovered_peer_type,
        receive_gossip: receive_gossip_type,
        receive_rpc: receive_rpc_type,
        log: slog::Logger,
    ) -> error::Result<(Self)> {
        let args = config(args_vec);
        let mut config = NetworkConfig::new();
        config.apply_cli_args(&args).unwrap();
        // build the network channel
        let (mut network_send, network_recv) = mpsc::unbounded_channel::<NetworkMessage>();
        // launch libp2p Network
        let libp2p_log = log.new(o!("Network" => "Libp2p"));
        let libp2p_service = Arc::new(Mutex::new(LibP2PService::new(config.clone(), libp2p_log)?));
        let libp2p_exit = spawn_service(
            libp2p_service.clone(),
            network_recv,
            executor,
            discovered_peer,
            receive_gossip,
            receive_rpc,
            log.clone(),
        )?;

        let network_service = Network {
            libp2p_service,
            _libp2p_exit: libp2p_exit,
            network_send: network_send.clone(),
            log: log.clone(),
        };

        Ok(network_service)
    }

    pub fn gossip(&mut self, topic: String, data: Vec<u8>) {
        self.network_send
            .try_send(NetworkMessage::Publish {
                topics: vec![Topic::new(topic)],
                message: data,
            })
            .unwrap_or_else(|_| warn!(self.log, "Could not send gossip message."));
    }

    pub fn rpc_request(&mut self, method: String, peer: String, data: Vec<u8>) {
        // use 0 as the default request id, when an ID is not required.
        let request_id: usize = 0;
        let rpc_request: RPCRequest = RPCRequest::Message(data);
        let rpc_event: RPCEvent = RPCEvent::Request(request_id, rpc_request);
        let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
        let peer_id = PeerId::from_bytes(bytes).map_err(|_| ()).unwrap();
        self.network_send
            .try_send(NetworkMessage::Send(
                peer_id,
                OutgoingMessage::RPC(rpc_event),
            ))
            .unwrap_or_else(|_| {
                warn!(
                    self.log,
                    "Could not send RPC message to the network service"
                )
            });
    }

    pub fn rpc_response(&mut self, method: String, peer: String, data: Vec<u8>) {
        // use 0 as the default request id, when an ID is not required.
        let request_id: usize = 0;
        let rpc_response: RPCResponse = RPCResponse::Message(data);
        let rpc_event: RPCEvent =
            RPCEvent::Response(request_id, RPCErrorResponse::Success(rpc_response));
        let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
        let peer_id = PeerId::from_bytes(bytes).map_err(|_| ()).unwrap();
        self.network_send
            .try_send(NetworkMessage::Send(
                peer_id,
                OutgoingMessage::RPC(rpc_event),
            ))
            .unwrap_or_else(|_| {
                warn!(
                    self.log,
                    "Could not send RPC message to the network service"
                )
            });
    }
}

fn spawn_service(
    libp2p_service: Arc<Mutex<LibP2PService>>,
    network_recv: mpsc::UnboundedReceiver<NetworkMessage>,
    executor: &TaskExecutor,
    discovered_peer: discovered_peer_type,
    receive_gossip: receive_gossip_type,
    receive_rpc: receive_rpc_type,
    log: slog::Logger,
) -> error::Result<tokio::sync::oneshot::Sender<()>> {
    let (network_exit, exit_rx) = tokio::sync::oneshot::channel();
    // spawn on the current executor
    executor.spawn(
        network_service(
            libp2p_service,
            network_recv,
            discovered_peer,
            receive_gossip,
            receive_rpc,
            log.clone(),
        )
        // allow for manual termination
        .select(exit_rx.then(|_| Ok(())))
        .then(move |_| {
            info!(log, "Network shutdown");
            Ok(())
        }),
    );
    Ok(network_exit)
}

fn network_service(
    libp2p_service: Arc<Mutex<LibP2PService>>,
    mut network_recv: mpsc::UnboundedReceiver<NetworkMessage>,
    discovered_peer: discovered_peer_type,
    receive_gossip: receive_gossip_type,
    receive_rpc: receive_rpc_type,
    log: slog::Logger,
) -> impl futures::Future<Item = (), Error = libp2p_wrapper::error::Error> {
    futures::future::poll_fn(move || -> Result<_, libp2p_wrapper::error::Error> {
        loop {
            // poll the network channel
            match network_recv.poll() {
                Ok(Async::Ready(Some(message))) => match message {
                    NetworkMessage::Send(peer_id, outgoing_message) => match outgoing_message {
                        OutgoingMessage::RPC(rpc_event) => {
                            debug!(log, "Sending RPC Event: {:?}", rpc_event);
                            libp2p_service.lock().swarm.send_rpc(peer_id, rpc_event);
                        }
                    },
                    NetworkMessage::Publish { topics, message } => {
                        debug!(log, "Sending pubsub message"; "topics" => format!("{:?}",topics));
                        libp2p_service.lock().swarm.publish(topics, message);
                    }
                },
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(None)) => {
                    return Err(libp2p_wrapper::error::Error::from("Network channel closed"));
                }
                Err(_) => {
                    return Err(libp2p_wrapper::error::Error::from("Network channel error"));
                }
            }
        }
        loop {
            // poll the swarm
            match libp2p_service.lock().poll() {
                Ok(Async::Ready(Some(event))) => match event {
                    Libp2pEvent::RPC(_peer_id, rpc_event) => {
                        debug!(log, "RPC Event: {:?}", rpc_event);
                        match rpc_event {
                            RPCEvent::Request(_, request) => match request {
                                RPCRequest::Message(data) => {
                                    debug!(log, "RPCRequest message received: {:?}", data);
                                    receive_rpc(
                                        "".to_string(),
                                        0,
                                        _peer_id.to_string(),
                                        data.to_vec(),
                                    );
                                }
                            },
                            RPCEvent::Response(id, err_response) => match err_response {
                                RPCErrorResponse::InvalidRequest(error) => {
                                    warn!(log, "Peer indicated invalid request";"peer_id" => format!("{:?}", _peer_id), "error" => error.as_string())
                                }
                                RPCErrorResponse::ServerError(error) => {
                                    warn!(log, "Peer internal server error";"peer_id" => format!("{:?}", _peer_id), "error" => error.as_string())
                                }
                                RPCErrorResponse::Unknown(error) => {
                                    warn!(log, "Unknown peer error";"peer" => format!("{:?}", _peer_id), "error" => error.as_string())
                                }
                                RPCErrorResponse::Success(response) => match response {
                                    RPCResponse::Message(data) => {
                                        debug!(log, "RPCResponse message received: {:?}", data);
                                        receive_rpc(
                                            "".to_string(),
                                            1,
                                            _peer_id.to_string(),
                                            data.to_vec(),
                                        );
                                    }
                                },
                            },
                            RPCEvent::Error(_, _) => {
                                warn!(log, "RPCEvent Error");
                            }
                        }
                    }
                    Libp2pEvent::PubsubMessage {
                        source,
                        topics,
                        message,
                    } => {
                        debug!(log, "Gossip message received: {:?}", message);
                        receive_gossip(topics[0].to_string(), message.clone());
                    }
                    Libp2pEvent::PeerDialed(peer_id) => {
                        debug!(log, "Peer Dialed: {:?}", peer_id);
                        discovered_peer(peer_id.to_string());
                    }
                    Libp2pEvent::PeerDisconnected(peer_id) => {
                        debug!(log, "Peer Disconnected: {:?}", peer_id);
                    }
                },
                Ok(Async::Ready(None)) => unreachable!("Stream never ends"),
                Ok(Async::NotReady) => break,
                Err(_) => break,
            }
        }
        Ok(Async::NotReady)
    })
}

fn config(args: Vec<String>) -> ArgMatches<'static> {
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

/// Types of messages that the network Network can receive.
#[derive(Debug)]
pub enum NetworkMessage {
    /// Send a message to libp2p Network.
    //TODO: Define typing for messages across the wire
    Send(PeerId, OutgoingMessage),
    /// Publish a message to pubsub mechanism.
    Publish {
        topics: Vec<Topic>,
        message: Vec<u8>,
    },
}

/// Type of outgoing messages that can be sent through the network Network.
#[derive(Debug)]
pub enum OutgoingMessage {
    /// Send an RPC request/response.
    RPC(RPCEvent),
}
