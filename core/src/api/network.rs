use super::error;
use futures::prelude::*;
use futures::Stream;
use libp2p_wrapper::Service as LibP2PService;
use libp2p_wrapper::{Libp2pEvent, PeerId};
use libp2p_wrapper::{NetworkConfig, GossipTopic, RPCErrorResponse, RPCEvent, RPCRequest, RPCResponse};
use parking_lot::Mutex;
use slog::{debug, info, o, warn};
use std::sync::Arc;
use tokio::runtime::TaskExecutor;
use tokio::sync::{mpsc, oneshot};

pub const GOSSIP: &str = "GOSSIP";
pub const RPC: &str = "RPC";
pub const DISCOVERY: &str = "DISCOVERY";

type DiscoveredPeerType = fn(peer: String);
type ReceiveGossipType = fn(topic: String, data: Vec<u8>);
type ReceiveRpcType = fn(method: String, req_resp: u8, peer: String, data: Vec<u8>);

pub struct Network {
    libp2p_service: Arc<Mutex<LibP2PService>>,
    _libp2p_exit: oneshot::Sender<()>,
    network_send: mpsc::UnboundedSender<NetworkMessage>,
    log: slog::Logger,
}

impl Network {
    pub fn new(
        args: Vec<String>,
        executor: &TaskExecutor,
        discovered_peer: DiscoveredPeerType,
        receive_gossip: ReceiveGossipType,
        receive_rpc: ReceiveRpcType,
        log: slog::Logger,
    ) -> error::Result<Self> {
        let arg_matches = NetworkConfig::matches(args);
        let mut config = NetworkConfig::new();
        config.apply_cli_args(&arg_matches).unwrap();
        // build the network channel
        let (network_send, network_recv) = mpsc::unbounded_channel::<NetworkMessage>();
        // launch libp2p Network
        let libp2p_log = log.new(o!("Network" => "Libp2p"));
        let libp2p_service = Arc::new(Mutex::new(LibP2PService::new(&config, [0u8; 32].to_vec(), libp2p_log)?));
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
            network_send,
            log: log.clone(),
        };

        Ok(network_service)
    }

    pub fn gossip(&mut self, topic: String, data: Vec<u8>) {
        self.network_send
            .try_send(NetworkMessage::Publish {
                topics: vec![GossipTopic::new(topic)],
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
    discovered_peer: DiscoveredPeerType,
    receive_gossip: ReceiveGossipType,
    receive_rpc: ReceiveRpcType,
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
    discovered_peer: DiscoveredPeerType,
    receive_gossip: ReceiveGossipType,
    receive_rpc: ReceiveRpcType,
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
                    Libp2pEvent::RPC(peer_id, rpc_event) => {
                        debug!(log, "RPC Event: {:?}", rpc_event);
                        match rpc_event {
                            RPCEvent::Request(_, request) => match request {
                                RPCRequest::Message(data) => {
                                    debug!(log, "RPCRequest message received: {:?}", data);
                                    receive_rpc(
                                        "".to_string(),
                                        0,
                                        peer_id.to_string(),
                                        data.to_vec(),
                                    );
                                }
                            },
                            RPCEvent::Response(id, err_response) => match err_response {
                                RPCErrorResponse::InvalidRequest(error) => {
                                    warn!(log, "Peer indicated invalid request";"peer_id" => format!("{:?}", peer_id), "error" => error.as_string())
                                }
                                RPCErrorResponse::ServerError(error) => {
                                    warn!(log, "Peer internal server error";"peer_id" => format!("{:?}", peer_id), "error" => error.as_string())
                                }
                                RPCErrorResponse::Unknown(error) => {
                                    warn!(log, "Unknown peer error";"peer" => format!("{:?}", peer_id), "error" => error.as_string())
                                }
                                RPCErrorResponse::Success(response) => match response {
                                    RPCResponse::Message(data) => {
                                        debug!(log, "RPCResponse message received: {:?}", data);
                                        receive_rpc(
                                            "".to_string(),
                                            1,
                                            peer_id.to_string(),
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
                        id,
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
                    Libp2pEvent::PeerSubscribed(peer_id, topic) => {
                        debug!(log, "Peer {:?} subscribed to topic: {:?}", peer_id, topic);
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

/// Types of messages that the network Network can receive.
#[derive(Debug)]
pub enum NetworkMessage {
    /// Send a message to libp2p Network.
    //TODO: Define typing for messages across the wire
    Send(PeerId, OutgoingMessage),
    /// Publish a message to pubsub mechanism.
    Publish {
        topics: Vec<GossipTopic>,
        message: Vec<u8>,
    },
}

/// Type of outgoing messages that can be sent through the network Network.
#[derive(Debug)]
pub enum OutgoingMessage {
    /// Send an RPC request/response.
    RPC(RPCEvent),
}
