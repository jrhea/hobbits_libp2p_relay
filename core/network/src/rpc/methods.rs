//! Available RPC methods types and ids.

use crate::types::EnrBitfield;
use serde::Serialize;
use std::ops::Deref;

/// Maximum number of blocks in a single request.
pub const MAX_REQUEST_BLOCKS: u64 = 1024;

/// Wrapper over SSZ List to represent error message in rpc responses.
#[derive(Debug, Clone)]
pub struct ErrorType(Vec<u8>);

impl From<Vec<u8>> for ErrorType {
    fn from(s: Vec<u8>) -> Self {
        Self(s)
    }
}
impl From<String> for ErrorType {
    fn from(s: String) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl From<&str> for ErrorType {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl Deref for ErrorType {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToString for ErrorType {
    fn to_string(&self) -> String {
        match std::str::from_utf8(self.0.deref()) {
            Ok(s) => s.to_string(),
            Err(_) => format!("{:?}", self.0.deref()), // Display raw bytes if not a UTF-8 string
        }
    }
}

/* Request/Response data structures for RPC methods */

/* Requests */

/// Identifier of a request.
///
// NOTE: The handler stores the `RequestId` to inform back of responses and errors, but it's execution
// is independent of the contents on this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestId {
    Router,
    Sync(usize),
    Behaviour,
}

/// The STATUS request/response handshake message.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusMessage {
    /// The fork version of the chain we are broadcasting.
    pub fork_digest: [u8; 4],

    /// Latest finalized root.
    pub finalized_root: Vec<u8>,

    /// Latest finalized epoch.
    pub finalized_epoch: u64,

    /// The latest block root.
    pub head_root: Vec<u8>,

    /// The slot associated with the latest block root.
    pub head_slot: u64,
}

/// The PING request/response message.
#[derive(Clone, Debug, PartialEq)]
pub struct Ping {
    /// The metadata sequence number.
    pub data: u64,
}

/// The METADATA response structure.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MetaData {
    /// A sequential counter indicating when data gets modified.
    pub seq_number: u64,
    /// The persistent subnet bitfield.
    pub attnets: EnrBitfield,
}

/// The reason given for a `Goodbye` message.
///
/// Note: any unknown `u64::into(n)` will resolve to `Goodbye::Unknown` for any unknown `n`,
/// however `GoodbyeReason::Unknown.into()` will go into `0_u64`. Therefore de-serializing then
/// re-serializing may not return the same bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum GoodbyeReason {
    /// This node has shutdown.
    ClientShutdown = 1,

    /// Incompatible networks.
    IrrelevantNetwork = 2,

    /// Error/fault in the RPC.
    Fault = 3,

    /// Unknown reason.
    Unknown = 0,
}

impl From<u64> for GoodbyeReason {
    fn from(id: u64) -> GoodbyeReason {
        match id {
            1 => GoodbyeReason::ClientShutdown,
            2 => GoodbyeReason::IrrelevantNetwork,
            3 => GoodbyeReason::Fault,
            _ => GoodbyeReason::Unknown,
        }
    }
}

impl Into<u64> for GoodbyeReason {
    fn into(self) -> u64 {
        self as u64
    }
}

/* RPC Handling and Grouping */
// Collection of enums and structs used by the Codecs to encode/decode RPC messages

#[derive(Debug, Clone, PartialEq)]
pub enum RPCResponse {
    /// A HELLO message.
    Status(Vec<u8>),

    /// A PONG response to a PING request.
    Pong(Vec<u8>),

    /// A response to a META_DATA request.
    MetaData(Vec<u8>),
}

/// The structured response containing a result/code indicating success or failure
/// and the contents of the response
#[derive(Debug, Clone)]
pub enum RPCCodedResponse {
    /// The response is a successful.
    Success(RPCResponse),

    /// The response was invalid.
    InvalidRequest(ErrorType),

    /// The response indicates a server error.
    ServerError(ErrorType),

    /// There was an unknown response.
    Unknown(ErrorType),
}

/// The code assigned to an erroneous `RPCResponse`.
#[derive(Debug, Clone, Copy)]
pub enum RPCResponseErrorCode {
    InvalidRequest,
    ServerError,
    Unknown,
}

impl RPCCodedResponse {
    /// Used to encode the response in the codec.
    pub fn as_u8(&self) -> Option<u8> {
        match self {
            RPCCodedResponse::Success(_) => Some(0),
            RPCCodedResponse::InvalidRequest(_) => Some(1),
            RPCCodedResponse::ServerError(_) => Some(2),
            RPCCodedResponse::Unknown(_) => Some(255),
        }
    }

    /// Tells the codec whether to decode as an RPCResponse or an error.
    pub fn is_response(response_code: u8) -> bool {
        match response_code {
            0 => true,
            _ => false,
        }
    }

    /// Builds an RPCCodedResponse from a response code and an ErrorMessage
    pub fn from_error(response_code: u8, err: String) -> Self {
        match response_code {
            1 => RPCCodedResponse::InvalidRequest(err.into()),
            2 => RPCCodedResponse::ServerError(err.into()),
            _ => RPCCodedResponse::Unknown(err.into()),
        }
    }

    /// Builds an RPCCodedResponse from a response code and an ErrorMessage
    pub fn from_error_code(response_code: RPCResponseErrorCode, err: String) -> Self {
        match response_code {
            RPCResponseErrorCode::InvalidRequest => RPCCodedResponse::InvalidRequest(err.into()),
            RPCResponseErrorCode::ServerError => RPCCodedResponse::ServerError(err.into()),
            RPCResponseErrorCode::Unknown => RPCCodedResponse::Unknown(err.into()),
        }
    }

    /// Specifies which response allows for multiple chunks for the stream handler.
    pub fn multiple_responses(&self) -> bool {
        match self {
            RPCCodedResponse::Success(resp) => match resp {
                RPCResponse::Status(_) => false,
                RPCResponse::Pong(_) => false,
                RPCResponse::MetaData(_) => false,
            },
            RPCCodedResponse::InvalidRequest(_) => true,
            RPCCodedResponse::ServerError(_) => true,
            RPCCodedResponse::Unknown(_) => true,
        }
    }

    /// Returns true if this response is an error. Used to terminate the stream after an error is
    /// sent.
    pub fn is_error(&self) -> bool {
        match self {
            RPCCodedResponse::Success(_) => false,
            _ => true,
        }
    }

    pub fn error_code(&self) -> Option<RPCResponseErrorCode> {
        match self {
            RPCCodedResponse::Success(_) => None,
            RPCCodedResponse::InvalidRequest(_) => Some(RPCResponseErrorCode::InvalidRequest),
            RPCCodedResponse::ServerError(_) => Some(RPCResponseErrorCode::ServerError),
            RPCCodedResponse::Unknown(_) => Some(RPCResponseErrorCode::Unknown),
        }
    }
}

impl std::fmt::Display for RPCResponseErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            RPCResponseErrorCode::InvalidRequest => "The request was invalid",
            RPCResponseErrorCode::ServerError => "Server error occurred",
            RPCResponseErrorCode::Unknown => "Unknown error occurred",
        };
        f.write_str(repr)
    }
}

impl std::fmt::Display for StatusMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status Message: Fork Digest: {:?}, Finalized Root: {:?}, Finalized Epoch: {}, Head Root: {:?}, Head Slot: {}", self.fork_digest, self.finalized_root, self.finalized_epoch, self.head_root, self.head_slot)
    }
}

impl std::fmt::Display for RPCResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RPCResponse::Status(status) => write!(f, "{:?}", status),
            RPCResponse::Pong(ping) => write!(f, "Pong: {:?}", ping),
            RPCResponse::MetaData(metadata) => write!(f, "Metadata: {:?}", metadata),
        }
    }
}

impl std::fmt::Display for RPCCodedResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RPCCodedResponse::Success(res) => write!(f, "{}", res),
            RPCCodedResponse::InvalidRequest(err) => write!(f, "Invalid Request: {:?}", err),
            RPCCodedResponse::ServerError(err) => write!(f, "Server Error: {:?}", err),
            RPCCodedResponse::Unknown(err) => write!(f, "Unknown Error: {:?}", err),
        }
    }
}

impl std::fmt::Display for GoodbyeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoodbyeReason::ClientShutdown => write!(f, "Client Shutdown"),
            GoodbyeReason::IrrelevantNetwork => write!(f, "Irrelevant Network"),
            GoodbyeReason::Fault => write!(f, "Fault"),
            GoodbyeReason::Unknown => write!(f, "Unknown Reason"),
        }
    }
}

impl slog::Value for RequestId {
    fn serialize(
        &self,
        record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        match self {
            RequestId::Behaviour => slog::Value::serialize("Behaviour", record, key, serializer),
            RequestId::Router => slog::Value::serialize("Router", record, key, serializer),
            RequestId::Sync(ref id) => slog::Value::serialize(id, record, key, serializer),
        }
    }
}
