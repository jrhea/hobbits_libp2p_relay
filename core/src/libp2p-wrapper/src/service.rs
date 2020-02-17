use crate::config::*;
use crate::behaviour::{Behaviour, BehaviourEvent, PubsubMessage};
use crate::error;
use crate::multiaddr::Protocol;
use crate::rpc::{RPCEvent};
use crate::NetworkConfig;
use crate::{Topic, TopicHash};
use futures::prelude::*;
use futures::Stream;
use libp2p::core::{
    identity::Keypair,
    multiaddr::Multiaddr,
    muxing::StreamMuxerBox,
    nodes::Substream,
    transport::boxed::Boxed,
    upgrade::{InboundUpgradeExt, OutboundUpgradeExt},
};
use std::sync::mpsc as sync;
use libp2p::{core, secio, PeerId, Swarm, Transport};
use slog::{debug, info, warn};
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::time::Duration;

type Libp2pStream = Boxed<(PeerId, StreamMuxerBox), Error>;
type Libp2pBehaviour = Behaviour<Substream<StreamMuxerBox>>;

const NETWORK_KEY_FILENAME: &str = "key";
pub const GOSSIP: &str = "GOSSIP";
pub const RPC: &str = "RPC";
pub const DISCOVERY: &str = "DISCOVERY";

pub struct Message {
    pub category: String,
    pub command: String,
    pub req_resp: u8,
    pub peer: String,
    pub value: Vec<u8>,
}

impl Message {
    pub fn new (category: String, command: String, req_resp: u8, peer: String, value: Vec<u8>) -> Message {
        Message {
            category: category,
            command: command,
            req_resp: req_resp,
            peer: peer,
            value: value
        }
    }
}

/// The configuration and state of the libp2p components for the beacon node.
pub struct Service{
    /// The libp2p Swarm handler.
    //TODO: Make this private
    pub swarm: Swarm<Libp2pStream, Libp2pBehaviour>,
    /// This node's PeerId.
    _local_peer_id: PeerId,
    tx: std::sync::Mutex<sync::Sender<Message>>,
    /// The libp2p logger handle.
    pub log: slog::Logger,
}

impl Service {

    pub fn new(config: NetworkConfig, tx: std::sync::Mutex<sync::Sender<Message>>, log: slog::Logger) -> error::Result<Self> {
        // load the private key from CLI flag, disk or generate a new one
        let local_private_key = load_private_key(&config, &log);

        let local_peer_id = PeerId::from(local_private_key.public());
        info!(log, "Local peer id: {:?}", local_peer_id);

        let mut swarm = {
            // Set up the transport - tcp/ws with secio and mplex/yamux
            let transport = build_transport(local_private_key.clone());
            // network behaviour
            let behaviour = Behaviour::new(&local_private_key, &config, &log)?;
            Swarm::new(transport, behaviour, local_peer_id.clone())
        };

        // listen on the specified address
        let listen_multiaddr = {
            let mut m = Multiaddr::from(config.listen_address);
            m.push(Protocol::Tcp(config.libp2p_port));
            m
        };

        match Swarm::listen_on(&mut swarm, listen_multiaddr.clone()) {
            Ok(_) => {
                let mut log_address = listen_multiaddr;
                log_address.push(Protocol::P2p(local_peer_id.clone().into()));
                info!(log, "Listening established"; "address" => format!("{}", log_address));
            }
            Err(err) => warn!(
                log,
                "Cannot listen on: {} because: {:?}", listen_multiaddr, err
            ),
        };

        // attempt to connect to user-input libp2p nodes
        for multiaddr in config.libp2p_nodes {
            match Swarm::dial_addr(&mut swarm, multiaddr.clone()) {
                Ok(()) => debug!(log, "Dialing libp2p peer"; "address" => format!("{}", multiaddr)),
                Err(err) => debug!(
                    log,
                    "Could not connect to peer"; "address" => format!("{}", multiaddr), "Error" => format!("{:?}", err)
                ),
            };
        }

        // subscribe to default gossipsub topics
        let mut topics = vec![];

        // Add any topics specified by the user
        topics.append(
            &mut config
                .topics
                .iter()
                .cloned()
                .map(|s| Topic::new(s))
                .collect(),
        );

        let mut subscribed_topics = vec![];
        for topic in topics {
            if swarm.subscribe(topic.clone()) {
                debug!(log, "Subscribed to topic: {:?}", topic);
                subscribed_topics.push(topic);
            } else {
                warn!(log, "Could not subscribe to topic: {:?}", topic)
            }
        }
        info!(log, "Subscribed to topics"; "topics" => format!("{:?}", subscribed_topics.iter().map(|t| format!("{}", t)).collect::<Vec<String>>()));
        Ok(Service {
            _local_peer_id: local_peer_id,
            swarm,
            tx,
            log,
        })
    }
}

impl Stream for Service {
    type Item = Libp2pEvent;
    type Error = crate::error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.swarm.poll() {
                //Behaviour events
                Ok(Async::Ready(Some(event))) => match event {
                    BehaviourEvent::PubsubMessage {
                        source,
                        topics,
                        message,
                    } => {
                        //debug!(self.log, "Gossipsub message received"; "Message" => format!("{:?}", topics[0]));
                        self.tx.lock().unwrap().send(Message {
                            category: GOSSIP.to_string(),
                            command: topics[0].to_string(),
                            req_resp: Default::default(),
                            peer: Default::default(),
                            value: message.clone()
                        }).unwrap();
                        return Ok(Async::Ready(Some(Libp2pEvent::PubsubMessage {
                            source,
                            topics,
                            message,
                        })));
                    }
                    BehaviourEvent::RPC(peer_id, event) => {
                        //debug!(self.log,"Received RPC message from: {:?}", peer_id);
                        return Ok(Async::Ready(Some(Libp2pEvent::RPC(peer_id, event))));
                    }
                    BehaviourEvent::PeerDialed(peer_id) => {
                         return Ok(Async::Ready(Some(Libp2pEvent::PeerDialed(peer_id))));
                    }
                    BehaviourEvent::PeerDisconnected(peer_id) => {
                        return Ok(Async::Ready(Some(Libp2pEvent::PeerDisconnected(peer_id))));
                    }
                },
                Ok(Async::Ready(None)) => unreachable!("Swarm stream shouldn't end"),
                Ok(Async::NotReady) => break,
                _ => break,
            }
        }
        Ok(Async::NotReady)
    }
}

/// The implementation supports TCP/IP, WebSockets over TCP/IP, secio as the encryption layer, and
/// mplex or yamux as the multiplexing layer.
fn build_transport(local_private_key: Keypair) -> Boxed<(PeerId, StreamMuxerBox), Error> {
    // TODO: The Wire protocol currently doesn't specify encryption and this will need to be customised
    // in the future.
    let transport = libp2p::tcp::TcpConfig::new().nodelay(true);
    let transport = libp2p::dns::DnsConfig::new(transport);
    #[cfg(feature = "libp2p-websocket")]
    let transport = {
        let trans_clone = transport.clone();
        transport.or_transport(websocket::WsConfig::new(trans_clone))
    };
    transport
        .upgrade(core::upgrade::Version::V1)
        .authenticate(secio::SecioConfig::new(local_private_key))
        .multiplex(core::upgrade::SelectUpgrade::new(
            libp2p::yamux::Config::default(),
            libp2p::mplex::MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, core::muxing::StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(20))
        .map_err(|err| Error::new(ErrorKind::Other, err))
        .boxed()
}

/// Events that can be obtained from polling the Libp2p Service.
#[derive(Debug)]
pub enum Libp2pEvent {
    /// An RPC response request has been received on the swarm.
    RPC(PeerId, RPCEvent),
    /// Initiated the connection to a new peer.
    PeerDialed(PeerId),
    /// A peer has disconnected.
    PeerDisconnected(PeerId),
    /// Received pubsub message.
    PubsubMessage {
        source: PeerId,
        topics: Vec<TopicHash>,
        message:  Vec<u8>,
    },
}

/// Loads a private key from disk. If this fails, a new key is
/// generated and is then saved to disk.
///
/// Currently only secp256k1 keys are allowed, as these are the only keys supported by discv5.
fn load_private_key(config: &NetworkConfig, log: &slog::Logger) -> Keypair {
    // TODO: Currently using secp256k1 keypairs - currently required for discv5
    // check for key from disk
    let network_key_f = config.network_dir.join(NETWORK_KEY_FILENAME);
    if let Ok(mut network_key_file) = File::open(network_key_f.clone()) {
        let mut key_bytes: Vec<u8> = Vec::with_capacity(36);
        match network_key_file.read_to_end(&mut key_bytes) {
            Err(_) => debug!(log, "Could not read network key file"),
            Ok(_) => {
                // only accept secp256k1 keys for now
                if let Ok(secret_key) =
                    libp2p::core::identity::secp256k1::SecretKey::from_bytes(&mut key_bytes)
                {
                    let kp: libp2p::core::identity::secp256k1::Keypair = secret_key.into();
                    debug!(log, "Loaded network key from disk.");
                    return Keypair::Secp256k1(kp);
                } else {
                    debug!(log, "Network key file is not a valid secp256k1 key");
                }
            }
        }
    }

    // if a key could not be loaded from disk, generate a new one and save it
    let local_private_key = Keypair::generate_secp256k1();
    if let Keypair::Secp256k1(key) = local_private_key.clone() {
        let _ = std::fs::create_dir_all(&config.network_dir);
        match File::create(network_key_f.clone())
            .and_then(|mut f| f.write_all(&key.secret().to_bytes()))
        {
            Ok(_) => {
                debug!(log, "New network key generated and written to disk");
            }
            Err(e) => {
                warn!(
                    log,
                    "Could not write node key to file: {:?}. Error: {}", network_key_f, e
                );
            }
        }
    }
    local_private_key
}
