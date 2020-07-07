use crate::{config::Config, error};
use futures::prelude::*;
use futures::Stream;
use network::Service as LibP2PService;
use network::{
    rpc::{GoodbyeReason, RPCResponseErrorCode, RequestId, StatusMessage},
    types::GossipKind,
    BehaviourEvent, ConnectedPoint, GossipTopic, Libp2pEvent, MessageId, NetworkConfig,
    NetworkGlobals, PeerId, PeerRequestId, Request, Response, Swarm, TaskExecutor,
};

use clap::ArgMatches;
use slog::{debug, info, o, trace, warn, Drain, Level, Logger};
use std::sync::Arc;
use tokio::sync::mpsc;

/// The time in seconds that a peer will be banned and prevented from reconnecting.
const BAN_PEER_TIMEOUT: u64 = 30;

pub type DiscoveredPeerType = fn(peer: String);
pub type ReceiveGossipType = fn(message_id: String, peer_id: String, topic: String, data: Vec<u8>);
pub type ReceiveRpcType = fn(method: String, req_resp: u8, peer: String, data: Vec<u8>);

pub trait Subscriber {
    fn init(&mut self, send: mpsc::UnboundedSender<NetworkMessage>, fork_id: Vec<u8>);
    fn discovered_peer(&self, peer: String);
    fn receive_gossip(&self, message_id: String, peer_id: String, topic: String, data: Vec<u8>);
    fn receive_rpc(&self, method: String, req_resp: u8, peer: String, data: Vec<u8>);
}

/// Handles communication between calling code and the `libp2p_p2p` service.
pub struct Mothra {
    /// Reference to the client using Mothra
    client: Box<dyn Subscriber + Send>,
    /// The underlying libp2p service that drives all the network interactions.
    libp2p: LibP2PService,
    /// The network receiver channel
    network_recv: mpsc::UnboundedReceiver<NetworkMessage>,
    /// The network sender channel
    network_send: mpsc::UnboundedSender<NetworkMessage>,
    /// A collection of global variables, accessible outside of the network service.
    network_globals: Arc<NetworkGlobals>,
    /// Probability of message propagation.
    propagation_percentage: Option<u8>,
    // TODO: Make a struct that implements this functionality.
    // It should hold an array and a counter
    /// rpc requests
    requests: [Option<PeerRequestId>; 256],
    /// num active requests
    num_requests: u8,
    /// The logger for the network service.
    log: slog::Logger,
}

impl Mothra {
    pub fn new(
        mut config: Config,
        enr_fork_id: Vec<u8>,
        meta_data: Vec<u8>,
        ping_data: Vec<u8>,
        executor: &TaskExecutor,
        mut client: Box<dyn Subscriber + Send>,
        log: slog::Logger,
    ) -> error::Result<(Arc<NetworkGlobals>, mpsc::UnboundedSender<NetworkMessage>)> {
        // build the network channel
        let (network_send, network_recv) = mpsc::unbounded_channel::<NetworkMessage>();
        client.init(network_send.clone(), enr_fork_id.clone());
        // Inject the executor into the discv5 network config.
        config.network_config.discv5_config.executor = Some(Box::new(executor.clone()));
        // launch libp2p Network
        let (network_globals, mut libp2p) = LibP2PService::new(
            executor.clone(),
            &mut config.network_config,
            enr_fork_id,
            meta_data,
            ping_data,
            &log.clone(),
        )?;

        let mut subscribed_topics: Vec<GossipKind> = vec![];
        for topic_kind in &config.network_config.topics {
            if libp2p.swarm.subscribe_kind(topic_kind.clone()) {
                subscribed_topics.push(topic_kind.clone());
            } else {
                warn!(log, "Could not subscribe to topic"; "topic" => format!("{}",topic_kind));
            }
        }
        info!(log, "Subscribed to topics"; "topics" => format!("{:?}", subscribed_topics));

        // create & spawn the network service
        let network_service = Mothra {
            client,
            libp2p,
            network_recv,
            network_send: network_send.clone(),
            network_globals: network_globals.clone(),
            propagation_percentage: config.network_config.propagation_percentage,
            requests: [None; 256],
            num_requests: 0,
            log: log.clone(),
        };

        spawn_mothra(network_service, executor)?;

        Ok((network_globals, network_send))
    }

    pub fn get_config(
        client_name: Option<String>,
        client_version: Option<String>,
        protocol_version: Option<String>,
        args: &ArgMatches,
    ) -> Config {
        // build NetworkConfig from args
        let mut config = Config::new(client_name, client_version, protocol_version);
        config.apply_cli_args(args).unwrap();
        config
    }
}

fn spawn_mothra(mut mothra: Mothra, executor: &TaskExecutor) -> error::Result<()> {
    let mut exit_rx = executor.exit();

    // spawn on the current executor
    executor.spawn_without_exit(async move {
        loop {
            // build the futures to check simultaneously
            tokio::select! {
                // handle network shutdown
                _ = (&mut exit_rx) => {
                    // network thread is terminating
                    info!(mothra.log, "Mothra shutdown");
                    return;
                }
                // handle a message sent to the network
                Some(message) = mothra.network_recv.recv() => {
                    match message {
                        NetworkMessage::SendRequest{ peer_id, request, request_id } => {
                            debug!(mothra.log, "SendRequest to peer: {:?} request type: {:?}", peer_id, request);
                            mothra.libp2p.send_request(peer_id, request_id, request);
                        }
                        NetworkMessage::SendResponse{ peer_id, response, index } => {
                            debug!(mothra.log, "SendResponse to peer: {:?} response type: {:?}", peer_id, response);
                            // Find the PeerRequestId
                            let id = mothra.requests[index as usize].unwrap();
                            // send response to libp2p
                            mothra.libp2p.send_response(peer_id, id, response);
                            // zero out the old PeerRequestId
                            mothra.requests[index as usize] = None;
                            // decrement the numrequests
                            mothra.num_requests -= 1;
                        }
                        NetworkMessage::Propagate {
                            propagation_source,
                            message_id,
                        } => {
                                trace!(mothra.log, "Propagating gossipsub message";
                                    "propagation_peer" => format!("{:?}", propagation_source),
                                    "message_id" => message_id.to_string(),
                                );
                                mothra
                                    .libp2p
                                    .swarm
                                    .propagate_message(&propagation_source, message_id);
                        }
                        NetworkMessage::Publish { topic, message } => {
                                debug!(
                                    mothra.log,
                                    "Sending pubsub message";
                                    "topic" => format!("{:?}", topic)
                                );
                                mothra.libp2p.swarm.publish(topic, message);
                        }
                        NetworkMessage::Disconnect { peer_id } => {
                            mothra.libp2p.disconnect_and_ban_peer(
                                peer_id,
                                std::time::Duration::from_secs(BAN_PEER_TIMEOUT),
                            );
                        }
                        NetworkMessage::Subscribe { subscriptions } => {
                            let mut subscribed_topics: Vec<GossipKind> = vec![];
                            for topic_kind in subscriptions {
                                if mothra.libp2p.swarm.subscribe_kind(topic_kind.clone().into()) {
                                    subscribed_topics.push(topic_kind.clone().into());
                                } else {
                                    warn!(mothra.log, "Could not subscribe to topic"; "topic" => format!("{:?}",topic_kind));
                                }
                            }
                            info!(mothra.log, "Subscribed to topics"; "topics" => format!("{:?}", subscribed_topics));
                        }
                    }
                } // end mothra.network_recv.recv()
                libp2p_event = mothra.libp2p.next_event() => {
                    // poll the swarm
                    match libp2p_event {
                        Libp2pEvent::Behaviour(event) => match event {
                            BehaviourEvent::RequestReceived{peer_id, id, request} => {
                                debug!(mothra.log, "Mothra {:?} received from: {:?} id: {:?}", request, peer_id, id);
                                // Save the PeerRequestId
                                mothra.requests[mothra.num_requests as usize] = Some(id);
                                // Call out to bindings to encode and store the index of the PeerRequestId
                                mothra.client.receive_rpc("Status".to_string(), 1, peer_id.to_string(), vec![mothra.num_requests]);
                                // incrememnt the variable that indicates the total number of indexes
                                mothra.num_requests += 1;
                            }
                            BehaviourEvent::ResponseReceived{peer_id, id, response} => {
                                debug!(mothra.log, "{:?} received from: {:?}", peer_id, response);
                            }
                            BehaviourEvent::RPCFailed{id, peer_id, error} => {
                                debug!(mothra.log, "RPC request to: {:?} failed. error: {:?}", peer_id, error);
                            }
                            BehaviourEvent::StatusPeer(peer_id) => {
                                debug!(mothra.log, "Status request received from: {:?}", peer_id);
                                mothra.client.receive_rpc("Status".to_string(), 0, peer_id.to_string(), vec![])
                            }
                            BehaviourEvent::PubsubMessage {
                                id,
                                source,
                                topics,
                                message
                            } => {
                                debug!(mothra.log, "Gossip message received from: {:?}", source);
                                mothra.client.receive_gossip(id.to_string(), source.to_string(), topics[0].to_string(), message.clone());
                            }
                            BehaviourEvent::PeerSubscribed(peer_id, topic) => {
                                //debug!(mothra.log, "Subscribed to: {:?} for topic: {:?}", peer_id, topic);
                            },
                        }
                        Libp2pEvent::NewListenAddr(multiaddr) => {
                            mothra.network_globals.listen_multiaddrs.write().push(multiaddr);
                        }
                        Libp2pEvent::PeerConnected{ peer_id, endpoint,} => {
                            debug!(mothra.log, "Peer Connected"; "peer_id" => peer_id.to_string(), "endpoint" => format!("{:?}", endpoint));
                            mothra.client.receive_rpc("Status".to_string(), 0, peer_id.to_string(), vec![])

                        }
                        Libp2pEvent::PeerDisconnected{ peer_id, endpoint,} => {
                            debug!(mothra.log, "Peer Disconnected";  "peer_id" => peer_id.to_string(), "endpoint" => format!("{:?}", endpoint));
                        }
                    }
                }

            } //end select
        } //end loop

    }, "mothra");
    Ok(())
}

// TODO: Consider removing these helper methods and simply using the network_send channel
pub fn gossip(
    mut network_send: mpsc::UnboundedSender<NetworkMessage>,
    topic: String,
    data: Vec<u8>,
    log: slog::Logger,
) {
    network_send
        .send(NetworkMessage::Publish {
            topic: GossipTopic::new(topic),
            message: data,
        })
        .unwrap_or_else(|_| warn!(log, "Could not send gossip message."));
}

pub fn rpc_request(
    mut network_send: mpsc::UnboundedSender<NetworkMessage>,
    method: String,
    peer: String,
    data: Vec<u8>,
    log: slog::Logger,
) {
    let request_id: RequestId = RequestId::Behaviour;
    let request: Request = Request::Status(data);
    let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
    let peer_id = PeerId::from_bytes(bytes).map_err(|_| ()).unwrap();
    network_send
        .send(NetworkMessage::SendRequest {
            peer_id,
            request,
            request_id,
        })
        .unwrap_or_else(|_| warn!(log, "Could not send RPC request to the network service"));
}

pub fn rpc_response(
    mut network_send: mpsc::UnboundedSender<NetworkMessage>,
    method: String,
    peer: String,
    data: Vec<u8>,
    log: slog::Logger,
) {
    let index = 0;
    let response: Response = Response::Status(data);
    let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
    let peer_id = PeerId::from_bytes(bytes).map_err(|_| ()).unwrap();
    network_send
        .send(NetworkMessage::SendResponse {
            peer_id,
            response,
            index,
        })
        .unwrap_or_else(|_| warn!(log, "Could not send RPC response to the network service"));
}

/// Types of messages that the network service can receive.
#[derive(Debug)]
pub enum NetworkMessage {
    /// Subscribe to a list of topics.
    Subscribe { subscriptions: Vec<GossipTopic> },
    /// Send an RPC request to the libp2p service.
    SendRequest {
        peer_id: PeerId,
        request: Request,
        request_id: RequestId,
    },
    /// Send a successful Response to the libp2p service.
    SendResponse {
        peer_id: PeerId,
        response: Response,
        index: u8,
    },
    /// Publish a message.
    Publish {
        topic: GossipTopic,
        message: Vec<u8>,
    },
    /// Propagate a received gossipsub message.
    Propagate {
        propagation_source: PeerId,
        message_id: MessageId,
    },
    /// Disconnect and bans a peer id.
    Disconnect { peer_id: PeerId },
}
