use crate::{config::Config, error};
use futures::prelude::*;
use futures::Stream;
use network::Service as LibP2PService;
use network::{
    GossipTopic, Libp2pEvent, MessageId, NetworkConfig, NetworkGlobals, PeerId, RPCErrorResponse,
    RPCEvent, RPCRequest, RPCResponse, Swarm,
};

use clap::ArgMatches;
use slog::{debug, info, o, trace, warn, Drain, Level, Logger};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::timer::Delay;
use tokio_compat::runtime::TaskExecutor;

/// The time in seconds that a peer will be banned and prevented from reconnecting.
const BAN_PEER_TIMEOUT: u64 = 30;

pub type DiscoveredPeerType = fn(peer: String);
pub type ReceiveGossipType = fn(topic: String, data: Vec<u8>);
pub type ReceiveRpcType = fn(method: String, req_resp: u8, peer: String, data: Vec<u8>);

/// Handles communication between calling code and the `libp2p_p2p` service.
pub struct Mothra {
    /// The underlying libp2p service that drives all the network interactions.
    libp2p: LibP2PService,
    /// The network receiver channel
    network_recv: mpsc::UnboundedReceiver<NetworkMessage>,
    /// The network sender channel
    network_send: mpsc::UnboundedSender<NetworkMessage>,
    /// A collection of global variables, accessible outside of the network service.
    network_globals: Arc<NetworkGlobals>,
    /// An initial delay to update variables after the libp2p service has started.
    initial_delay: Delay,
    /// Probability of message propagation.
    propagation_percentage: Option<u8>,
    discovered_peer: DiscoveredPeerType,
    receive_gossip: ReceiveGossipType,
    receive_rpc: ReceiveRpcType,
    /// The logger for the network service.
    log: slog::Logger,
}

impl Mothra {
    pub fn new(
        mut config: Config,
        enr_fork_id: Vec<u8>,
        executor: &TaskExecutor,
        discovered_peer: DiscoveredPeerType,
        receive_gossip: ReceiveGossipType,
        receive_rpc: ReceiveRpcType,
        log: slog::Logger,
    ) -> error::Result<(
        Arc<NetworkGlobals>,
        mpsc::UnboundedSender<NetworkMessage>,
        oneshot::Sender<()>,
    )> {
        // build the network channel
        let (network_send, network_recv) = mpsc::unbounded_channel::<NetworkMessage>();

        // launch libp2p Network
        let (network_globals, libp2p) =
            LibP2PService::new(&mut config.network_config, enr_fork_id, log.clone())?;

        //TODO
        // for enr in load_dht::<T::Store, T::EthSpec>(store.clone()) {
        //     libp2p.swarm.add_enr(enr);
        // }

        // A delay used to initialise code after the network has started
        // This is currently used to obtain the listening addresses from the libp2p service.
        let initial_delay = Delay::new(Instant::now() + Duration::from_secs(1));

        // create & spawn the network service
        let network_service = Mothra {
            libp2p,
            network_recv,
            network_send: network_send.clone(),
            network_globals: network_globals.clone(),
            initial_delay,
            propagation_percentage: config.network_config.propagation_percentage,
            discovered_peer,
            receive_gossip,
            receive_rpc,
            log: log.clone(),
        };

        let network_exit = spawn_mothra(network_service, &executor)?;

        Ok((network_globals, network_send, network_exit))
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

fn spawn_mothra(
    mut mothra: Mothra,
    executor: &TaskExecutor,
) -> error::Result<tokio::sync::oneshot::Sender<()>> {
    let (network_exit, mut exit_rx) = tokio::sync::oneshot::channel();

    // spawn on the current executor
    executor.spawn(
    futures::future::poll_fn(move || -> Result<_, ()> {

        let log = &mothra.log;

        // handles any logic which requires an initial delay
        if !mothra.initial_delay.is_elapsed() {
            if let Ok(Async::Ready(_)) = mothra.initial_delay.poll() {
                        let multi_addrs = Swarm::listeners(&mothra.libp2p.swarm).cloned().collect();
                        *mothra.network_globals.listen_multiaddrs.write() = multi_addrs;
            }
        }

        // perform termination tasks when the network is being shutdown
        if let Ok(Async::Ready(_)) | Err(_) = exit_rx.poll() {
                    // network thread is terminating TODO
                    // let enrs: Vec<Enr> = service.libp2p.swarm.enr_entries().cloned().collect();
                    // debug!(
                    //     log,
                    //     "Persisting DHT to store";
                    //     "Number of peers" => format!("{}", enrs.len()),
                    // );
                    // match persist_dht::<T::Store, T::EthSpec>(service.store.clone(), enrs) {
                    //     Err(e) => error!(
                    //         log,
                    //         "Failed to persist DHT on drop";
                    //         "error" => format!("{:?}", e)
                    //     ),
                    //     Ok(_) => info!(
                    //         log,
                    //         "Saved DHT state";
                    //     ),
                    // }

                    info!(log.clone(), "Network service shutdown");
                    return Ok(Async::Ready(()));
        }

        // processes the network channel before processing the libp2p swarm
        loop {
            // poll the network channel
            match mothra.network_recv.poll() {
                Ok(Async::Ready(Some(message))) => match message {
                    NetworkMessage::RPC(peer_id, rpc_event) => {
                        trace!(log, "Sending RPC"; "rpc" => format!("{:?}", rpc_event));
                        mothra.libp2p.swarm.send_rpc(peer_id, rpc_event);
                    }
                    NetworkMessage::Propagate {
                        propagation_source,
                        message_id,
                    } => {
                        // TODO: Remove this for mainnet
                        // randomly prevents propagation
                        let mut should_send = true;
                        if let Some(percentage) = mothra.propagation_percentage {
                            // not exact percentage but close enough
                            let rand = rand::random::<u8>() % 100;
                            if rand > percentage {
                                // don't propagate
                                should_send = false;
                            }
                        }
                        if !should_send {
                            info!(log, "Random filter did not propagate message");
                        } else {
                            trace!(log, "Propagating gossipsub message";
                            "propagation_peer" => format!("{:?}", propagation_source),
                            "message_id" => message_id.to_string(),
                            );
                            mothra.libp2p
                                .swarm
                                .propagate_message(&propagation_source, message_id);
                        }
                    }
                    NetworkMessage::Publish { topics, message } => {
                        // TODO: Remove this for mainnet
                        // randomly prevents propagation
                        let mut should_send = true;
                        if let Some(percentage) = mothra.propagation_percentage {
                            // not exact percentage but close enough
                            let rand = rand::random::<u8>() % 100;
                            if rand > percentage {
                                // don't propagate
                                should_send = false;
                            }
                        }
                        if !should_send {
                            info!(log, "Random filter did not publish messages");
                        } else {
                            debug!(log, "Sending pubsub message"; "topics" => format!("{:?}",topics));
                            mothra.libp2p.swarm.publish(topics, message);
                        }
                    }
                    NetworkMessage::Disconnect { peer_id } => {
                        mothra.libp2p.disconnect_and_ban_peer(
                            peer_id,
                            std::time::Duration::from_secs(BAN_PEER_TIMEOUT),
                        );
                    }
                },
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(None)) => {
                    debug!(log, "Network channel closed");
                    return Err(());
                }
                Err(e) => {
                    debug!(log, "Network channel error"; "error" => format!("{}", e));
                    return Err(());
                }
            }
        }

        let mut peers_to_ban = Vec::<PeerId>::new();
        // poll the swarm
        loop {
            match mothra.libp2p.poll() {
                Ok(Async::Ready(Some(event))) => match event {
                    Libp2pEvent::RPC(peer_id, rpc_event) => {
                        debug!(log, "RPC Event: {:?}", rpc_event);
                        match rpc_event {
                            RPCEvent::Request(_, request) => match request {
                                RPCRequest::Message(data) => {
                                    debug!(log, "RPCRequest message received: {:?}", data);
                                    (mothra.receive_rpc)(
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
                                        (mothra.receive_rpc)(
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
                    Libp2pEvent::PeerDialed(peer_id) => {
                        debug!(log, "Peer Dialed: {:?}", peer_id);
                        (mothra.discovered_peer)(peer_id.to_string());
                    }
                    Libp2pEvent::PeerDisconnected(peer_id) => {
                        debug!(log, "Peer Disconnected: {:?}", peer_id);
                    }
                    Libp2pEvent::PubsubMessage {
                        id,
                        source,
                        topics,
                        message,
                    } => {
                        debug!(log, "Gossip message received from: {:?}", source);
                        (mothra.receive_gossip)(topics[0].to_string(), message.clone());
                    }
                    Libp2pEvent::PeerSubscribed(peer_id, topic) => {
                        debug!(log, "Peer {:?} subscribed to topic: {:?}", peer_id, topic);
                    }
                },
                Ok(Async::Ready(None)) => unreachable!("Stream never ends"),
                Ok(Async::NotReady) => break,
                Err(_) => break,
            }
        }

        // ban and disconnect any peers that sent Goodbye requests
        while let Some(peer_id) = peers_to_ban.pop() {
            mothra.libp2p.disconnect_and_ban_peer(
                peer_id.clone(),
                std::time::Duration::from_secs(BAN_PEER_TIMEOUT),
            );
        }

        // if we have just forked, update inform the libp2p layer TODO
        // if let Some(mut update_fork_delay) =  service.next_fork_update.take() {
        //     if !update_fork_delay.is_elapsed() {
        //         if let Ok(Async::Ready(_)) = update_fork_delay.poll() {
        //                 service.libp2p.swarm.update_fork_version(service.beacon_chain.enr_fork_id());
        //                 service.next_fork_update = next_fork_delay(&service.beacon_chain);
        //         }
        //     }
        // }

        Ok(Async::NotReady)
    })

    );

    Ok(network_exit)
}

pub fn gossip(
    mut network_send: mpsc::UnboundedSender<NetworkMessage>,
    topic: String,
    data: Vec<u8>,
    log: slog::Logger,
) {
    network_send
        .try_send(NetworkMessage::Publish {
            topics: vec![GossipTopic::new(topic)],
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
    // use 0 as the default request id, when an ID is not required.
    let request_id: usize = 0;
    let rpc_request: RPCRequest = RPCRequest::Message(data);
    let rpc_event: RPCEvent = RPCEvent::Request(request_id, rpc_request);
    let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
    let peer_id = PeerId::from_bytes(bytes).map_err(|_| ()).unwrap();
    network_send
        .try_send(NetworkMessage::RPC(peer_id, rpc_event))
        .unwrap_or_else(|_| warn!(log, "Could not send RPC message to the network service"));
}

pub fn rpc_response(
    mut network_send: mpsc::UnboundedSender<NetworkMessage>,
    method: String,
    peer: String,
    data: Vec<u8>,
    log: slog::Logger,
) {
    // use 0 as the default request id, when an ID is not required.
    let request_id: usize = 0;
    let rpc_response: RPCResponse = RPCResponse::Message(data);
    let rpc_event: RPCEvent =
        RPCEvent::Response(request_id, RPCErrorResponse::Success(rpc_response));
    let bytes = bs58::decode(peer.as_str()).into_vec().unwrap();
    let peer_id = PeerId::from_bytes(bytes).map_err(|_| ()).unwrap();
    network_send
        .try_send(NetworkMessage::RPC(peer_id, rpc_event))
        .unwrap_or_else(|_| warn!(log, "Could not send RPC message to the network service"));
}

/// Types of messages that the network service can receive.
#[derive(Debug)]
pub enum NetworkMessage {
    /// Send an RPC message to the libp2p service.
    RPC(PeerId, RPCEvent),
    /// Publish a list of messages to the gossipsub protocol.
    Publish {
        topics: Vec<GossipTopic>,
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
