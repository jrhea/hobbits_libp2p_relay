#![allow(clippy::type_complexity)]

use super::methods::*;
use crate::rpc::codec::{
    base::{BaseInboundCodec, BaseOutboundCodec},
    snappy::{SnappyInboundCodec, SnappyOutboundCodec},
    InboundCodec, OutboundCodec,
};
use futures::future::Ready;
use futures::prelude::*;
use futures::prelude::{AsyncRead, AsyncWrite};
use libp2p::core::{InboundUpgrade, OutboundUpgrade, ProtocolName, UpgradeInfo};
use std::io;
use std::marker::PhantomData;
use std::pin::Pin;
use std::time::Duration;
use tokio_io_timeout::TimeoutStream;
use tokio_util::{
    codec::Framed,
    compat::{Compat, FuturesAsyncReadCompatExt},
};

/// The maximum bytes that can be sent across the RPC.
const MAX_RPC_SIZE: usize = 1_048_576; // 1M
/// The protocol prefix the RPC protocol id.
const PROTOCOL_PREFIX: &str = "/eth2/beacon_chain/req";
/// Time allowed for the first byte of a request to arrive before we time out (Time To First Byte).
const TTFB_TIMEOUT: u64 = 5;
/// The number of seconds to wait for the first bytes of a request once a protocol has been
/// established before the stream is terminated.
const REQUEST_TIMEOUT: u64 = 15;

/// Protocol names to be used.
#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    /// The Status protocol name.
    Status,
    /// The Goodbye protocol name.
    Goodbye,
    /// The `Ping` protocol name.
    Ping,
    /// The `MetaData` protocol name.
    MetaData,
}

/// RPC Versions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    /// Version 1 of RPC
    V1,
}

/// RPC Encondings supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoding {
    Snappy,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Protocol::Status => "status",
            Protocol::Goodbye => "goodbye",
            Protocol::Ping => "ping",
            Protocol::MetaData => "metadata",
        };
        f.write_str(repr)
    }
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Encoding::Snappy => "ssz_snappy",
        };
        f.write_str(repr)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Version::V1 => "1",
        };
        f.write_str(repr)
    }
}

#[derive(Debug, Clone)]
pub struct RPCProtocol;

impl UpgradeInfo for RPCProtocol {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;

    /// The list of supported RPC protocols.
    fn protocol_info(&self) -> Self::InfoIter {
        vec![
            ProtocolId::new(Protocol::Status, Version::V1, Encoding::Snappy),
            ProtocolId::new(Protocol::Goodbye, Version::V1, Encoding::Snappy),
            ProtocolId::new(Protocol::Ping, Version::V1, Encoding::Snappy),
            ProtocolId::new(Protocol::MetaData, Version::V1, Encoding::Snappy),
        ]
    }
}

/// Tracks the types in a protocol id.
#[derive(Clone, Debug)]
pub struct ProtocolId {
    /// The RPC message type/name.
    pub message_name: Protocol,

    /// The version of the RPC.
    pub version: Version,

    /// The encoding of the RPC.
    pub encoding: Encoding,

    /// The protocol id that is formed from the above fields.
    protocol_id: String,
}

/// An RPC protocol ID.
impl ProtocolId {
    pub fn new(message_name: Protocol, version: Version, encoding: Encoding) -> Self {
        let protocol_id = format!(
            "{}/{}/{}/{}",
            PROTOCOL_PREFIX, message_name, version, encoding
        );

        ProtocolId {
            message_name,
            version,
            encoding,
            protocol_id,
        }
    }
}

impl ProtocolName for ProtocolId {
    fn protocol_name(&self) -> &[u8] {
        self.protocol_id.as_bytes()
    }
}

/* Inbound upgrade */

// The inbound protocol reads the request, decodes it and returns the stream to the protocol
// handler to respond to once ready.

pub type InboundOutput<TSocket> = (RPCRequest, InboundFramed<TSocket>);
pub type InboundFramed<TSocket> = Framed<TimeoutStream<Compat<TSocket>>, InboundCodec>;
type FnAndThen<TSocket> = fn(
    (Option<Result<RPCRequest, RPCError>>, InboundFramed<TSocket>),
) -> Ready<Result<InboundOutput<TSocket>, RPCError>>;
type FnMapErr = fn(tokio::time::Elapsed) -> RPCError;

impl<TSocket> InboundUpgrade<TSocket> for RPCProtocol
where
    TSocket: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = InboundOutput<TSocket>;
    type Error = RPCError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: TSocket, protocol: ProtocolId) -> Self::Future {
        let protocol_name = protocol.message_name;
        // convert the socket to tokio compatible socket
        let socket = socket.compat();
        let codec = match protocol.encoding {
            Encoding::Snappy => {
                let ssz_snappy_codec =
                    BaseInboundCodec::new(SnappyInboundCodec::new(protocol, MAX_RPC_SIZE));
                InboundCodec::Snappy(ssz_snappy_codec)
            }
        };
        let mut timed_socket = TimeoutStream::new(socket);
        timed_socket.set_read_timeout(Some(Duration::from_secs(TTFB_TIMEOUT)));

        let socket = Framed::new(timed_socket, codec);

        // MetaData requests should be empty, return the stream
        Box::pin(match protocol_name {
            Protocol::MetaData => future::Either::Left(future::ok((RPCRequest::MetaData, socket))),

            _ => future::Either::Right(
                tokio::time::timeout(Duration::from_secs(REQUEST_TIMEOUT), socket.into_future())
                    .map_err(RPCError::from as FnMapErr)
                    .and_then({
                        |(req, stream)| match req {
                            Some(Ok(request)) => future::ok((request, stream)),
                            Some(Err(_)) | None => future::err(RPCError::IncompleteStream),
                        }
                    } as FnAndThen<TSocket>),
            ),
        })
    }
}

/* Outbound request */

// Combines all the RPC requests into a single enum to implement `UpgradeInfo` and
// `OutboundUpgrade`

#[derive(Debug, Clone, PartialEq)]
pub enum RPCRequest {
    Status(Vec<u8>),
    Goodbye(Vec<u8>),
    Ping(Vec<u8>),
    MetaData,
}

impl UpgradeInfo for RPCRequest {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;

    // add further protocols as we support more encodings/versions
    fn protocol_info(&self) -> Self::InfoIter {
        self.supported_protocols()
    }
}

/// Implements the encoding per supported protocol for `RPCRequest`.
impl RPCRequest {
    pub fn supported_protocols(&self) -> Vec<ProtocolId> {
        match self {
            // add more protocols when versions/encodings are supported
            RPCRequest::Status(_) => vec![ProtocolId::new(
                Protocol::Status,
                Version::V1,
                Encoding::Snappy,
            )],
            RPCRequest::Goodbye(_) => vec![ProtocolId::new(
                Protocol::Goodbye,
                Version::V1,
                Encoding::Snappy,
            )],
            RPCRequest::Ping(_) => vec![ProtocolId::new(
                Protocol::Ping,
                Version::V1,
                Encoding::Snappy,
            )],
            RPCRequest::MetaData => vec![ProtocolId::new(
                Protocol::MetaData,
                Version::V1,
                Encoding::Snappy,
            )],
        }
    }

    /* These functions are used in the handler for stream management */

    /// Number of responses expected for this request.
    pub fn expected_responses(&self) -> usize {
        match self {
            RPCRequest::Status(_) => 1,
            RPCRequest::Goodbye(_) => 0,
            RPCRequest::Ping(_) => 1,
            RPCRequest::MetaData => 1,
        }
    }

    /// Gives the corresponding `Protocol` to this request.
    pub fn protocol(&self) -> Protocol {
        match self {
            RPCRequest::Status(_) => Protocol::Status,
            RPCRequest::Goodbye(_) => Protocol::Goodbye,
            RPCRequest::Ping(_) => Protocol::Ping,
            RPCRequest::MetaData => Protocol::MetaData,
        }
    }
}

/* RPC Response type - used for outbound upgrades */

/* Outbound upgrades */

pub type OutboundFramed<TSocket> = Framed<Compat<TSocket>, OutboundCodec>;

impl<TSocket> OutboundUpgrade<TSocket> for RPCRequest
where
    TSocket: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = OutboundFramed<TSocket>;
    type Error = RPCError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, socket: TSocket, protocol: Self::Info) -> Self::Future {
        // convert to a tokio compatible socket
        let socket = socket.compat();
        let codec = match protocol.encoding {
            Encoding::Snappy => {
                let ssz_snappy_codec =
                    BaseOutboundCodec::new(SnappyOutboundCodec::new(protocol, MAX_RPC_SIZE));
                OutboundCodec::Snappy(ssz_snappy_codec)
            }
        };

        let mut socket = Framed::new(socket, codec);

        let future = async { socket.send(self).await.map(|_| socket) };
        Box::pin(future)
    }
}

/// Error in RPC Encoding/Decoding.
#[derive(Debug, Clone)]
pub enum RPCError {
    /// Error when decoding the raw buffer from ssz.
    // NOTE: in the future a ssz::DecodeError should map to an InvalidData error
    DecodeError,
    /// IO Error.
    IoError(String),
    /// The peer returned a valid response but the response indicated an error.
    ErrorResponse(RPCResponseErrorCode, String),
    /// Timed out waiting for a response.
    StreamTimeout,
    /// Peer does not support the protocol.
    UnsupportedProtocol,
    /// Stream ended unexpectedly.
    IncompleteStream,
    /// Peer sent invalid data.
    InvalidData,
    /// An error occurred due to internal reasons. Ex: timer failure.
    InternalError(&'static str),
    /// Negotiation with this peer timed out.
    NegotiationTimeout,
    /// Handler rejected this request.
    HandlerRejected,
}

impl From<tokio::time::Elapsed> for RPCError {
    fn from(_: tokio::time::Elapsed) -> Self {
        RPCError::StreamTimeout
    }
}

impl From<io::Error> for RPCError {
    fn from(err: io::Error) -> Self {
        RPCError::IoError(err.to_string())
    }
}

// Error trait is required for `ProtocolsHandler`
impl std::fmt::Display for RPCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RPCError::DecodeError => write!(f, "Error while decoding RPC"),
            RPCError::InvalidData => write!(f, "Peer sent unexpected data"),
            RPCError::IoError(ref err) => write!(f, "IO Error: {}", err),
            RPCError::ErrorResponse(ref code, ref reason) => write!(
                f,
                "RPC response was an error: {} with reason: {}",
                code, reason
            ),
            RPCError::StreamTimeout => write!(f, "Stream Timeout"),
            RPCError::UnsupportedProtocol => write!(f, "Peer does not support the protocol"),
            RPCError::IncompleteStream => write!(f, "Stream ended unexpectedly"),
            RPCError::InternalError(ref err) => write!(f, "Internal error: {}", err),
            RPCError::NegotiationTimeout => write!(f, "Negotiation timeout"),
            RPCError::HandlerRejected => write!(f, "Handler rejected the request"),
        }
    }
}

impl std::error::Error for RPCError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            // NOTE: this does have a source
            RPCError::DecodeError => None,
            RPCError::IoError(_) => None,
            RPCError::StreamTimeout => None,
            RPCError::UnsupportedProtocol => None,
            RPCError::IncompleteStream => None,
            RPCError::InvalidData => None,
            RPCError::InternalError(_) => None,
            RPCError::ErrorResponse(_, _) => None,
            RPCError::NegotiationTimeout => None,
            RPCError::HandlerRejected => None,
        }
    }
}

impl std::fmt::Display for RPCRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RPCRequest::Status(status) => write!(f, "Status Message: {:?}", status),
            RPCRequest::Goodbye(reason) => write!(f, "Goodbye: {:?}", reason),
            RPCRequest::Ping(ping) => write!(f, "Ping: {:?}", ping),
            RPCRequest::MetaData => write!(f, "MetaData request"),
        }
    }
}
